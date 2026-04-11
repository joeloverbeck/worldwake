use crate::{
    GoalKindPlannerExt, GroundedGoal, PlannerOpKind, PlannerOpSemantics, PlanningEntityRef,
    PlanningSnapshot, PlanningState, goal_model::trace_prerequisite_guidance,
    shared_collections::SharedVec,
};
use std::collections::{BTreeMap, BTreeSet};
use worldwake_core::{ActionDefId, EntityId, ExecutionBudget};
use worldwake_sim::RecipeRegistry;

use super::{
    SearchCandidate, SearchNode, TacticalGoal,
    landmarks::{LandmarkSet, PlanningFact, actionable_landmarks},
};

/// Compute the A* heuristic: minimum perceived travel cost from the actor's
/// current simulated position to the nearest goal-relevant place. Returns 0
/// when the actor is already at a goal-relevant place, when no spatial
/// guidance is available (empty `goal_relevant_places`), or when the actor's
/// place cannot be resolved.
pub(super) fn compute_heuristic(
    snapshot: &PlanningSnapshot,
    state: &PlanningState<'_>,
    goal_relevant_places: &[EntityId],
) -> u32 {
    if goal_relevant_places.is_empty() {
        return 0;
    }
    let actor = state.snapshot().actor();
    let actor_place = state.effective_place_ref(PlanningEntityRef::Authoritative(actor));
    actor_place
        .and_then(|place| snapshot.min_perceived_travel_cost_to_any(place, goal_relevant_places))
        .unwrap_or(0)
}

#[allow(dead_code)]
pub(super) fn compute_landmark_heuristic(
    landmarks: &LandmarkSet,
    current_facts: &BTreeSet<PlanningFact>,
) -> u32 {
    actionable_landmarks(landmarks, current_facts).len() as u32
}

pub(super) struct CombinedRelevantPlaces {
    pub(super) places: Vec<EntityId>,
    pub(super) prerequisite_places_count: u16,
    pub(super) guidance_trace: Option<crate::decision_trace::PrerequisiteGuidanceTrace>,
}

#[allow(clippy::trivially_copy_pass_by_ref)]
pub(super) fn combined_relevant_places(
    goal: &GroundedGoal,
    state: &PlanningState<'_>,
    recipes: &RecipeRegistry,
    execution_budget: &ExecutionBudget,
) -> CombinedRelevantPlaces {
    combined_relevant_places_internal(goal, state, recipes, execution_budget, false)
}

#[allow(clippy::trivially_copy_pass_by_ref)]
pub(super) fn combined_relevant_places_for_tactical(
    goal: &GroundedGoal,
    state: &PlanningState<'_>,
    recipes: &RecipeRegistry,
    execution_budget: &ExecutionBudget,
    tactical_goal: Option<&TacticalGoal>,
) -> CombinedRelevantPlaces {
    tactical_goal.and_then(tactical_goal_places).map_or_else(
        || combined_relevant_places(goal, state, recipes, execution_budget),
        |destination| CombinedRelevantPlaces {
            places: vec![destination],
            prerequisite_places_count: 0,
            guidance_trace: None,
        },
    )
}

#[allow(clippy::trivially_copy_pass_by_ref)]
pub(super) fn combined_relevant_places_with_guidance_for_tactical(
    goal: &GroundedGoal,
    state: &PlanningState<'_>,
    recipes: &RecipeRegistry,
    execution_budget: &ExecutionBudget,
    tactical_goal: Option<&TacticalGoal>,
) -> CombinedRelevantPlaces {
    tactical_goal.and_then(tactical_goal_places).map_or_else(
        || combined_relevant_places_with_guidance(goal, state, recipes, execution_budget),
        |destination| CombinedRelevantPlaces {
            places: vec![destination],
            prerequisite_places_count: 0,
            guidance_trace: None,
        },
    )
}

#[allow(clippy::trivially_copy_pass_by_ref)]
pub(super) fn combined_relevant_places_with_guidance(
    goal: &GroundedGoal,
    state: &PlanningState<'_>,
    recipes: &RecipeRegistry,
    execution_budget: &ExecutionBudget,
) -> CombinedRelevantPlaces {
    combined_relevant_places_internal(goal, state, recipes, execution_budget, true)
}

#[allow(clippy::trivially_copy_pass_by_ref)]
fn combined_relevant_places_internal(
    goal: &GroundedGoal,
    state: &PlanningState<'_>,
    recipes: &RecipeRegistry,
    execution_budget: &ExecutionBudget,
    include_guidance_trace: bool,
) -> CombinedRelevantPlaces {
    let mut places = goal.key.kind.goal_relevant_places(state, recipes);
    if places.is_empty()
        && matches!(
            goal.key.kind,
            worldwake_core::GoalKind::SearchForMissing { .. }
        )
    {
        places.extend(goal.evidence_places.iter().copied());
    }
    let goal_relevant_places_for_trace = include_guidance_trace.then(|| places.clone());
    let base_len = places.len();
    let prerequisite_places = goal
        .key
        .kind
        .prerequisite_places(state, recipes, execution_budget);
    let prerequisite_places_for_trace = include_guidance_trace.then(|| prerequisite_places.clone());
    for place in prerequisite_places {
        if !places.contains(&place) {
            places.push(place);
        }
    }
    let prerequisite_places_count = (places.len() - base_len) as u16;
    let guidance_trace = if include_guidance_trace {
        trace_prerequisite_guidance(
            goal_relevant_places_for_trace
                .expect("guidance trace should preserve goal-relevant places"),
            prerequisite_places_for_trace
                .expect("guidance trace should preserve prerequisite places"),
            crate::goal_model::prerequisite_depleted_source_exclusions(
                &goal.key.kind,
                state,
                recipes,
            ),
        )
    } else {
        None
    };
    CombinedRelevantPlaces {
        places,
        prerequisite_places_count,
        guidance_trace,
    }
}

#[allow(clippy::trivially_copy_pass_by_ref)]
#[cfg(test)]
pub(super) fn root_node<'snapshot>(
    snapshot: &'snapshot PlanningSnapshot,
    goal: &GroundedGoal,
    recipes: &RecipeRegistry,
    execution_budget: &ExecutionBudget,
) -> SearchNode<'snapshot> {
    root_node_for_tactical(snapshot, goal, recipes, execution_budget, None)
}

#[allow(clippy::trivially_copy_pass_by_ref)]
pub(super) fn root_node_for_tactical<'snapshot>(
    snapshot: &'snapshot PlanningSnapshot,
    goal: &GroundedGoal,
    recipes: &RecipeRegistry,
    execution_budget: &ExecutionBudget,
    tactical_goal: Option<&TacticalGoal>,
) -> SearchNode<'snapshot> {
    let state = PlanningState::new(snapshot);
    let combined_places = combined_relevant_places_for_tactical(
        goal,
        &state,
        recipes,
        execution_budget,
        tactical_goal,
    );
    let heuristic_ticks = compute_heuristic(snapshot, &state, &combined_places.places);
    SearchNode {
        state,
        steps: SharedVec::new(),
        total_estimated_ticks: 0,
        search_cost: 0,
        tactical_barrier_reached: false,
        heuristic_ticks,
    }
}

fn tactical_goal_places(tactical_goal: &TacticalGoal) -> Option<EntityId> {
    match tactical_goal {
        TacticalGoal::AcquirePrerequisite { destination, .. }
        | TacticalGoal::Explore { destination }
        | TacticalGoal::TravelToGoal { destination } => Some(*destination),
        TacticalGoal::SocialQuery { .. } => None,
    }
}

/// Removes travel candidates that move the actor farther from every
/// goal-relevant place.  Non-travel candidates are never pruned.
/// When `goal_places` is empty, the function is a no-op.
///
/// When the actor is already at one goal-relevant place, pruning continues
/// against the remaining relevant places so search can leave the current place
/// without broadening into arbitrary detours.
pub(super) fn prune_travel_away_from_goal(
    candidates: &mut Vec<SearchCandidate>,
    current_place: EntityId,
    goal_places: &[EntityId],
    snapshot: &PlanningSnapshot,
    semantics_table: &BTreeMap<ActionDefId, PlannerOpSemantics>,
) -> Option<crate::decision_trace::TravelPruningTrace> {
    if goal_places.is_empty() {
        return None;
    }
    let current_min = snapshot
        .min_travel_ticks_to_any(current_place, goal_places)
        .unwrap_or(u32::MAX);
    let effective_goal_places = if current_min == 0 {
        let alternatives = goal_places
            .iter()
            .copied()
            .filter(|place| *place != current_place)
            .collect::<Vec<_>>();
        if alternatives.is_empty() {
            return None;
        }
        alternatives
    } else {
        goal_places.to_vec()
    };
    let current_min = snapshot
        .min_perceived_travel_cost_to_any(current_place, &effective_goal_places)
        .unwrap_or(u32::MAX);
    let mut retained = Vec::new();
    let mut pruned = Vec::new();
    let mut kept_candidates = Vec::with_capacity(candidates.len());

    for candidate in candidates.drain(..) {
        let Some(semantics) = semantics_table.get(&candidate.def_id) else {
            kept_candidates.push(candidate);
            continue;
        };
        if semantics.op_kind != PlannerOpKind::Travel {
            kept_candidates.push(candidate);
            continue;
        }
        let Some(destination) = candidate.authoritative_targets.first().copied() else {
            kept_candidates.push(candidate);
            continue;
        };

        let remaining_travel_ticks = snapshot
            .min_perceived_travel_cost_to_any(destination, &effective_goal_places)
            .unwrap_or(u32::MAX);
        let direct_cost = snapshot
            .direct_perceived_travel_breakdown(current_place, destination)
            .unwrap_or(
                crate::planning_snapshot::DirectPerceivedTravelCostBreakdown {
                    base_ticks: 0,
                    threat: worldwake_core::Permille::new(0)
                        .expect("zero permille should remain valid"),
                    penalty_ticks: 0,
                    perceived_cost: 0,
                },
            );
        let successor = crate::decision_trace::TravelSuccessorTrace {
            destination,
            base_ticks: direct_cost.base_ticks,
            threat_permille: direct_cost.threat,
            penalty_ticks: direct_cost.penalty_ticks,
            direct_perceived_cost: direct_cost.perceived_cost,
            remaining_travel_ticks,
            projected_total_cost: direct_cost
                .perceived_cost
                .saturating_add(remaining_travel_ticks),
        };
        if remaining_travel_ticks <= current_min {
            retained.push(successor);
            kept_candidates.push(candidate);
        } else {
            pruned.push(successor);
        }
    }

    *candidates = kept_candidates;
    if retained.is_empty() && pruned.is_empty() {
        return None;
    }
    retained.sort_by_key(|successor| successor.destination);
    pruned.sort_by_key(|successor| successor.destination);
    Some(crate::decision_trace::TravelPruningTrace {
        current_place,
        current_remaining_travel_ticks: current_min,
        retained,
        pruned,
    })
}

#[cfg(test)]
mod tests {
    use super::compute_landmark_heuristic;
    use crate::search::landmarks::{LandmarkSet, PlanningFact};
    use std::collections::BTreeSet;
    use worldwake_core::{CommodityKind, EntityId};

    fn entity(slot: u32) -> EntityId {
        EntityId {
            slot,
            generation: 1,
        }
    }

    fn fact_set(facts: impl IntoIterator<Item = PlanningFact>) -> BTreeSet<PlanningFact> {
        facts.into_iter().collect()
    }

    #[test]
    fn landmark_heuristic_all_achieved() {
        let well = PlanningFact::AtPlace(entity(1));
        let water = PlanningFact::HasCommodity(CommodityKind::Water);
        let current_facts = fact_set([well.clone(), water.clone()]);
        let landmarks = LandmarkSet {
            landmarks: current_facts.clone(),
            orderings: vec![(well, water)],
        };

        assert_eq!(compute_landmark_heuristic(&landmarks, &current_facts), 0);
    }

    #[test]
    fn landmark_heuristic_counts_actionable() {
        let well = PlanningFact::AtPlace(entity(1));
        let water = PlanningFact::HasCommodity(CommodityKind::Water);
        let landmarks = LandmarkSet {
            landmarks: fact_set([well.clone(), water.clone()]),
            orderings: vec![(well.clone(), water.clone())],
        };

        assert_eq!(compute_landmark_heuristic(&landmarks, &fact_set([well])), 1);
    }

    #[test]
    fn landmark_heuristic_ignores_blocked() {
        let well = PlanningFact::AtPlace(entity(1));
        let bucket = PlanningFact::HasEntity(entity(2));
        let water = PlanningFact::HasCommodity(CommodityKind::Water);
        let landmarks = LandmarkSet {
            landmarks: fact_set([well.clone(), bucket.clone(), water.clone()]),
            orderings: vec![(well, bucket.clone()), (bucket, water)],
        };

        assert_eq!(compute_landmark_heuristic(&landmarks, &BTreeSet::new()), 1);
    }

    #[test]
    fn landmark_heuristic_empty_landmarks() {
        assert_eq!(
            compute_landmark_heuristic(&LandmarkSet::empty(), &BTreeSet::new()),
            0
        );
    }
}
