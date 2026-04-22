use std::collections::{BTreeMap, BTreeSet, VecDeque};

use worldwake_core::{CommodityKind, EntityId, GoalKind, HomeostaticNeedId, Quantity};
use worldwake_sim::{InventoryBeliefView, ProfileBeliefView, RecipeRegistry};

use crate::{GoalOffer, PlanningEntityRef, PlanningState};

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

/// Result of building an RPG and extracting a relaxed plan.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(super) struct RelaxedPlanResult {
    /// Number of operators in the extracted relaxed plan. This is the
    /// FF relaxed-plan heuristic value.
    pub(super) h_ff: u32,
    /// Indices into the input operator slice for selected layer-0
    /// operators in the relaxed plan.
    pub(super) helpful_action_indices: BTreeSet<usize>,
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
    goal: &GoalOffer,
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

pub(super) fn compute_ff_heuristic(
    initial_facts: &BTreeSet<PlanningFact>,
    goal_facts: &BTreeSet<PlanningFact>,
    operators: &[PlanningOperator],
) -> Option<RelaxedPlanResult> {
    if goal_facts.is_empty() || goal_facts.is_subset(initial_facts) {
        return Some(RelaxedPlanResult::default());
    }

    let mut reachable_facts = initial_facts.clone();
    let mut first_achiever = BTreeMap::<PlanningFact, (usize, usize)>::new();
    let max_layers = operators.len();

    for layer in 0..max_layers {
        let mut new_facts = BTreeSet::new();

        for (operator_index, operator) in operators.iter().enumerate() {
            if !operator.preconditions.is_subset(&reachable_facts) {
                continue;
            }

            for effect in &operator.add_effects {
                if reachable_facts.contains(effect) || new_facts.contains(effect) {
                    continue;
                }
                new_facts.insert(effect.clone());
                first_achiever.insert(effect.clone(), (layer, operator_index));
            }
        }

        if new_facts.is_empty() {
            return None;
        }

        reachable_facts.extend(new_facts);
        if goal_facts.is_subset(&reachable_facts) {
            break;
        }
    }

    if !goal_facts.is_subset(&reachable_facts) {
        return None;
    }

    let mut selected_operator_indices = BTreeSet::new();
    let mut helpful_action_indices = BTreeSet::new();

    for goal_fact in goal_facts {
        select_fact(
            goal_fact,
            initial_facts,
            &first_achiever,
            operators,
            &mut selected_operator_indices,
            &mut helpful_action_indices,
        );
    }

    Some(RelaxedPlanResult {
        h_ff: selected_operator_indices.len() as u32,
        helpful_action_indices,
    })
}

fn select_fact(
    fact: &PlanningFact,
    initial_facts: &BTreeSet<PlanningFact>,
    first_achiever: &BTreeMap<PlanningFact, (usize, usize)>,
    operators: &[PlanningOperator],
    selected_operator_indices: &mut BTreeSet<usize>,
    helpful_action_indices: &mut BTreeSet<usize>,
) {
    if initial_facts.contains(fact) {
        return;
    }

    let Some(&(layer, operator_index)) = first_achiever.get(fact) else {
        return;
    };

    if layer == 0 {
        helpful_action_indices.insert(operator_index);
    }

    if !selected_operator_indices.insert(operator_index) {
        return;
    }

    for precondition in &operators[operator_index].preconditions {
        select_fact(
            precondition,
            initial_facts,
            first_achiever,
            operators,
            selected_operator_indices,
            helpful_action_indices,
        );
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

    use super::{
        LandmarkSet, PlanningFact, PlanningOperator, RelaxedPlanResult, compute_ff_heuristic,
        extract_landmarks, preferred_operators,
    };
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

    fn operator_with_delete(
        preconditions: impl IntoIterator<Item = PlanningFact>,
        add_effects: impl IntoIterator<Item = PlanningFact>,
        del_effects: impl IntoIterator<Item = PlanningFact>,
    ) -> PlanningOperator {
        PlanningOperator {
            preconditions: fact_set(preconditions),
            add_effects: fact_set(add_effects),
            del_effects: fact_set(del_effects),
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
            expansion_trace_index: None,
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
            &[operator(
                [],
                [PlanningFact::HasCommodity(CommodityKind::Water)],
            )],
        );

        assert!(preferred.is_empty());
    }

    #[test]
    fn ff_heuristic_returns_none_for_unreachable_goals() {
        let goal = PlanningFact::HasCommodity(CommodityKind::Water);

        assert_eq!(
            compute_ff_heuristic(&BTreeSet::new(), &fact_set([goal]), &[]),
            None
        );
    }

    #[test]
    fn ff_heuristic_returns_zero_when_goal_already_satisfied() {
        let goal = PlanningFact::HasCommodity(CommodityKind::Water);

        assert_eq!(
            compute_ff_heuristic(&fact_set([goal.clone()]), &fact_set([goal]), &[]),
            Some(RelaxedPlanResult::default())
        );
    }

    #[test]
    fn ff_heuristic_counts_linear_relaxed_plan() {
        let has_bucket = PlanningFact::HasEntity(entity(1));
        let has_water = PlanningFact::HasCommodity(CommodityKind::Water);
        let thirst_satisfied = PlanningFact::NeedSatisfied(HomeostaticNeedId::Thirst);

        let result = compute_ff_heuristic(
            &fact_set([has_bucket.clone()]),
            &fact_set([thirst_satisfied.clone()]),
            &[
                operator([has_bucket], [has_water.clone()]),
                operator([has_water], [thirst_satisfied]),
            ],
        )
        .expect("linear chain should be reachable");

        assert_eq!(result.h_ff, 2);
        assert_eq!(result.helpful_action_indices, BTreeSet::from([0]));
    }

    #[test]
    fn ff_heuristic_ignores_delete_effects_under_relaxation() {
        let has_hands = PlanningFact::HasEntity(entity(1));
        let has_water = PlanningFact::HasCommodity(CommodityKind::Water);
        let has_medicine = PlanningFact::HasCommodity(CommodityKind::Medicine);

        let result = compute_ff_heuristic(
            &fact_set([has_hands.clone()]),
            &fact_set([has_water.clone(), has_medicine.clone()]),
            &[
                operator_with_delete(
                    [has_hands.clone()],
                    [has_water.clone()],
                    [has_hands.clone()],
                ),
                operator_with_delete([has_hands], [has_medicine.clone()], []),
                operator([], [PlanningFact::FacilityAvailable(entity(99))]),
            ],
        )
        .expect("delete relaxation should keep both goals reachable");

        assert_eq!(result.h_ff, 2);
        assert_eq!(result.helpful_action_indices, BTreeSet::from([0, 1]));
    }

    #[test]
    fn ff_heuristic_helpful_actions_only_include_selected_layer_zero_ops() {
        let seed_fact = PlanningFact::HasEntity(entity(1));
        let intermediate = PlanningFact::FacilityAvailable(entity(2));
        let distractor = PlanningFact::EntityPresent(entity(3));
        let goal = PlanningFact::HasCommodity(CommodityKind::Water);

        let result = compute_ff_heuristic(
            &fact_set([seed_fact.clone()]),
            &fact_set([goal.clone()]),
            &[
                operator([seed_fact.clone()], [intermediate.clone()]),
                operator([seed_fact], [distractor]),
                operator([intermediate], [goal]),
            ],
        )
        .expect("goal should be reachable");

        assert_eq!(result.h_ff, 2);
        assert_eq!(result.helpful_action_indices, BTreeSet::from([0]));
    }

    #[test]
    fn ff_heuristic_is_deterministic() {
        let place = PlanningFact::AtPlace(entity(1));
        let has_bucket = PlanningFact::HasEntity(entity(2));
        let has_water = PlanningFact::HasCommodity(CommodityKind::Water);
        let thirst_satisfied = PlanningFact::NeedSatisfied(HomeostaticNeedId::Thirst);
        let operators = vec![
            operator([place.clone()], [has_bucket.clone()]),
            operator([has_bucket], [has_water.clone()]),
            operator([has_water], [thirst_satisfied.clone()]),
        ];
        let initial_facts = fact_set([place]);
        let goal_facts = fact_set([thirst_satisfied]);

        let first = compute_ff_heuristic(&initial_facts, &goal_facts, &operators);
        let second = compute_ff_heuristic(&initial_facts, &goal_facts, &operators);
        let third = compute_ff_heuristic(&initial_facts, &goal_facts, &operators);

        assert_eq!(first, second);
        assert_eq!(second, third);
    }
}
