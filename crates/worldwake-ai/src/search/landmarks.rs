use std::collections::{BTreeSet, VecDeque};

use worldwake_core::{CommodityKind, EntityId, GoalKind, HomeostaticNeedId, Quantity};
use worldwake_sim::{InventoryBeliefView, ProfileBeliefView, RecipeRegistry};

use crate::{GroundedGoal, PlanningEntityRef, PlanningState};

use super::candidates::SearchCandidate;

#[allow(dead_code)]
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(super) enum PlanningFact {
    AtPlace(EntityId),
    HasCommodity(CommodityKind),
    HasEntity(EntityId),
    FacilityAvailable(EntityId),
    EntityPresent(EntityId),
    NeedSatisfied(HomeostaticNeedId),
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(super) struct PlanningOperator {
    pub(super) preconditions: BTreeSet<PlanningFact>,
    pub(super) add_effects: BTreeSet<PlanningFact>,
    pub(super) del_effects: BTreeSet<PlanningFact>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(super) struct LandmarkSet {
    pub(super) landmarks: BTreeSet<PlanningFact>,
    pub(super) orderings: Vec<(PlanningFact, PlanningFact)>,
}

impl LandmarkSet {
    pub(super) fn empty() -> Self {
        Self::default()
    }
}

pub(super) fn planning_facts_from_state(state: &PlanningState<'_>) -> BTreeSet<PlanningFact> {
    let actor = state.snapshot().actor();
    let mut facts = BTreeSet::new();
    if let Some(place) = state.effective_place_ref(PlanningEntityRef::Authoritative(actor)) {
        facts.insert(PlanningFact::AtPlace(place));
    }
    for commodity in CommodityKind::ALL {
        if state.commodity_quantity(actor, commodity) > Quantity(0) {
            facts.insert(PlanningFact::HasCommodity(commodity));
        }
    }
    for entity in state.direct_possessions(actor) {
        facts.insert(PlanningFact::HasEntity(entity));
    }
    if let Some(needs) = state.homeostatic_needs(actor)
        && let Some(thresholds) = state.drive_thresholds(actor)
    {
        if needs.hunger < thresholds.hunger.low() {
            facts.insert(PlanningFact::NeedSatisfied(HomeostaticNeedId::Hunger));
        }
        if needs.thirst < thresholds.thirst.low() {
            facts.insert(PlanningFact::NeedSatisfied(HomeostaticNeedId::Thirst));
        }
        if needs.fatigue < thresholds.fatigue.low() {
            facts.insert(PlanningFact::NeedSatisfied(HomeostaticNeedId::Fatigue));
        }
        if needs.bladder < thresholds.bladder.low() {
            facts.insert(PlanningFact::NeedSatisfied(HomeostaticNeedId::Bladder));
        }
        if needs.dirtiness < thresholds.dirtiness.low() {
            facts.insert(PlanningFact::NeedSatisfied(HomeostaticNeedId::Dirtiness));
        }
    }
    facts
}

pub(super) fn planning_operator_from_transition(
    before: &PlanningState<'_>,
    after: &PlanningState<'_>,
) -> PlanningOperator {
    let before_facts = planning_facts_from_state(before);
    let after_facts = planning_facts_from_state(after);
    let mut preconditions = BTreeSet::new();
    if let Some(place) =
        before.effective_place_ref(PlanningEntityRef::Authoritative(before.snapshot().actor()))
    {
        preconditions.insert(PlanningFact::AtPlace(place));
    }
    preconditions.extend(before_facts.difference(&after_facts).cloned());
    PlanningOperator {
        preconditions,
        add_effects: after_facts.difference(&before_facts).cloned().collect(),
        del_effects: before_facts.difference(&after_facts).cloned().collect(),
    }
}

pub(super) fn goal_facts_from_goal(
    goal: &GroundedGoal,
    state: &PlanningState<'_>,
    recipes: &RecipeRegistry,
) -> BTreeSet<PlanningFact> {
    match goal.key.kind {
        GoalKind::AcquireCommodity { commodity, .. }
        | GoalKind::RestockCommodity { commodity }
        | GoalKind::ConsumeOwnedCommodity { commodity } => {
            BTreeSet::from([PlanningFact::HasCommodity(commodity)])
        }
        GoalKind::ProduceCommodity { recipe_id } => recipes
            .get(recipe_id)
            .map(|recipe| {
                recipe
                    .outputs
                    .iter()
                    .filter(|(_, quantity)| *quantity > Quantity(0))
                    .map(|(commodity, _)| PlanningFact::HasCommodity(*commodity))
                    .collect()
            })
            .unwrap_or_default(),
        GoalKind::ExploreLocation { target_place, .. } => {
            BTreeSet::from([PlanningFact::AtPlace(target_place)])
        }
        GoalKind::TreatWounds { .. } => state
            .homeostatic_needs(state.snapshot().actor())
            .map(|_| BTreeSet::from([PlanningFact::HasCommodity(CommodityKind::Medicine)]))
            .unwrap_or_default(),
        _ => BTreeSet::new(),
    }
}

pub(super) fn extract_landmarks(
    initial_facts: &BTreeSet<PlanningFact>,
    goal_facts: &BTreeSet<PlanningFact>,
    operators: &[PlanningOperator],
    max_depth: u8,
) -> LandmarkSet {
    let mut landmarks = goal_facts.clone();
    let mut orderings = BTreeSet::new();
    let mut queue = goal_facts
        .iter()
        .cloned()
        .map(|fact| (fact, 0u8))
        .collect::<VecDeque<_>>();

    while let Some((fact, depth)) = queue.pop_front() {
        if depth >= max_depth || initial_facts.contains(&fact) {
            continue;
        }

        let achievers = operators
            .iter()
            .filter(|operator| operator.add_effects.contains(&fact))
            .collect::<Vec<_>>();
        if achievers.is_empty() {
            continue;
        }

        let mut shared_preconditions = achievers[0].preconditions.clone();
        for achiever in achievers.iter().skip(1) {
            shared_preconditions.retain(|prec| achiever.preconditions.contains(prec));
        }

        for predecessor in shared_preconditions {
            if predecessor == fact {
                continue;
            }
            orderings.insert((predecessor.clone(), fact.clone()));
            if landmarks.insert(predecessor.clone()) {
                queue.push_back((predecessor, depth.saturating_add(1)));
            }
        }
    }

    LandmarkSet {
        landmarks,
        orderings: orderings.into_iter().collect(),
    }
}

pub(super) fn preferred_operators(
    landmarks: &LandmarkSet,
    current_facts: &BTreeSet<PlanningFact>,
    candidates: &[SearchCandidate],
    operators: &[PlanningOperator],
) -> BTreeSet<usize> {
    let actionable_landmarks = actionable_landmarks(landmarks, current_facts);
    if actionable_landmarks.is_empty() {
        return BTreeSet::new();
    }

    candidates
        .iter()
        .zip(operators.iter())
        .enumerate()
        .filter_map(|(index, (_candidate, operator))| {
            operator
                .add_effects
                .iter()
                .any(|effect| actionable_landmarks.contains(effect))
                .then_some(index)
        })
        .collect()
}

pub(super) fn actionable_landmarks(
    landmarks: &LandmarkSet,
    current_facts: &BTreeSet<PlanningFact>,
) -> BTreeSet<PlanningFact> {
    landmarks
        .landmarks
        .iter()
        .filter(|landmark| !current_facts.contains(*landmark))
        .filter(|landmark| {
            landmarks
                .orderings
                .iter()
                .filter(|(_, successor)| successor == *landmark)
                .all(|(predecessor, _)| current_facts.contains(predecessor))
        })
        .cloned()
        .collect()
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use worldwake_core::{ActionDefId, CommodityKind, EntityId, HomeostaticNeedId};
    use worldwake_sim::ActionPayload;

    use super::{LandmarkSet, PlanningFact, PlanningOperator, extract_landmarks, preferred_operators};
    use crate::PlanningEntityRef;
    use crate::search::candidates::SearchCandidate;

    fn entity(slot: u32) -> EntityId {
        EntityId {
            slot,
            generation: 1,
        }
    }

    fn fact_set(facts: impl IntoIterator<Item = PlanningFact>) -> BTreeSet<PlanningFact> {
        facts.into_iter().collect()
    }

    fn operator(
        preconditions: impl IntoIterator<Item = PlanningFact>,
        add_effects: impl IntoIterator<Item = PlanningFact>,
    ) -> PlanningOperator {
        PlanningOperator {
            preconditions: fact_set(preconditions),
            add_effects: fact_set(add_effects),
            del_effects: BTreeSet::new(),
        }
    }

    fn candidate(id: u32) -> SearchCandidate {
        SearchCandidate {
            def_id: ActionDefId(id),
            authoritative_targets: Vec::new(),
            planning_targets: Vec::<PlanningEntityRef>::new(),
            payload_override: Option::<ActionPayload>::None,
            planner_only: false,
            trace_index: None,
        }
    }

    #[test]
    fn goal_facts_are_landmarks() {
        let goal = PlanningFact::HasCommodity(CommodityKind::Water);
        let landmarks = extract_landmarks(&BTreeSet::new(), &fact_set([goal.clone()]), &[], 4);

        assert!(landmarks.landmarks.contains(&goal));
    }

    #[test]
    fn shared_precondition_discovery() {
        let place = PlanningFact::AtPlace(entity(1));
        let goal = PlanningFact::HasCommodity(CommodityKind::Water);
        let landmarks = extract_landmarks(
            &BTreeSet::new(),
            &fact_set([goal.clone()]),
            &[
                operator([place.clone()], [goal.clone()]),
                operator([place.clone()], [goal.clone()]),
            ],
            4,
        );

        assert!(landmarks.landmarks.contains(&place));
        assert!(landmarks.orderings.contains(&(place, goal)));
    }

    #[test]
    fn no_achievers_marks_unachievable_without_extra_predecessors() {
        let goal = PlanningFact::NeedSatisfied(HomeostaticNeedId::Hunger);
        let landmarks = extract_landmarks(&BTreeSet::new(), &fact_set([goal.clone()]), &[], 4);

        assert_eq!(landmarks.landmarks, fact_set([goal]));
        assert!(landmarks.orderings.is_empty());
    }

    #[test]
    fn initial_facts_skipped() {
        let place = PlanningFact::AtPlace(entity(1));
        let goal = PlanningFact::HasCommodity(CommodityKind::Water);
        let landmarks = extract_landmarks(
            &fact_set([goal.clone()]),
            &fact_set([goal.clone()]),
            &[operator([place.clone()], [goal.clone()])],
            4,
        );

        assert_eq!(landmarks.landmarks, fact_set([goal]));
        assert!(landmarks.orderings.is_empty());
    }

    #[test]
    fn max_depth_limits_chain() {
        let at_well = PlanningFact::AtPlace(entity(2));
        let has_bucket = PlanningFact::HasEntity(entity(3));
        let has_water = PlanningFact::HasCommodity(CommodityKind::Water);
        let landmarks = extract_landmarks(
            &BTreeSet::new(),
            &fact_set([has_water.clone()]),
            &[
                operator([has_bucket.clone()], [has_water.clone()]),
                operator([at_well.clone()], [has_bucket.clone()]),
            ],
            1,
        );

        assert!(landmarks.landmarks.contains(&has_water));
        assert!(landmarks.landmarks.contains(&has_bucket));
        assert!(!landmarks.landmarks.contains(&at_well));
    }

    #[test]
    fn empty_operators_returns_goal_landmarks() {
        let goal_a = PlanningFact::NeedSatisfied(HomeostaticNeedId::Fatigue);
        let goal_b = PlanningFact::FacilityAvailable(entity(4));
        let landmarks = extract_landmarks(
            &BTreeSet::new(),
            &fact_set([goal_a.clone(), goal_b.clone()]),
            &[],
            4,
        );

        assert_eq!(landmarks.landmarks, fact_set([goal_a, goal_b]));
        assert!(landmarks.orderings.is_empty());
    }

    #[test]
    fn preferred_operators_selects_landmark_achievers() {
        let place = PlanningFact::AtPlace(entity(1));
        let water = PlanningFact::HasCommodity(CommodityKind::Water);
        let preferred = preferred_operators(
            &LandmarkSet {
                landmarks: fact_set([place.clone(), water.clone()]),
                orderings: vec![(place.clone(), water.clone())],
            },
            &fact_set([place]),
            &[candidate(1), candidate(2)],
            &[
                operator([], [water.clone()]),
                operator([], [PlanningFact::EntityPresent(entity(9))]),
            ],
        );

        assert_eq!(preferred, BTreeSet::from([0]));
    }

    #[test]
    fn preferred_operators_empty_when_no_landmarks() {
        let preferred = preferred_operators(
            &LandmarkSet::empty(),
            &BTreeSet::new(),
            &[candidate(1)],
            &[operator([], [PlanningFact::HasCommodity(CommodityKind::Water)])],
        );

        assert!(preferred.is_empty());
    }
}
