use super::{SearchCandidate, SearchNode, TacticalGoal};
use crate::goal_model::{
    GoalPayloadOverrideError, grounded_goal_epistemic_subjects,
    grounded_goal_matches_epistemic_barrier,
};
use crate::planner_duration_contract::PlannerDurationDependency;
use crate::{
    GoalKindPlannerExt, GroundedGoal, PlanTerminalKind, PlannedStep, PlannerOpKind,
    PlannerOpSemantics, PlanningEntityRef, apply_hypothetical_transition,
};
use heuristic::{
    combined_relevant_places_for_tactical, compute_heuristic, compute_landmark_heuristic,
};
use std::collections::BTreeMap;
use worldwake_core::{ActionDefId, ExecutionBudget};
use worldwake_sim::{ActionDefRegistry, RecipeRegistry, TemporalBeliefView};

use super::heuristic;
use super::landmarks::{LandmarkSet, planning_facts_from_state};

#[cfg(test)]
#[allow(clippy::too_many_arguments)]
#[allow(clippy::trivially_copy_pass_by_ref)]
pub(super) fn build_successor<'snapshot>(
    goal: &GroundedGoal,
    semantics_table: &BTreeMap<ActionDefId, PlannerOpSemantics>,
    registry: &ActionDefRegistry,
    node: &SearchNode<'snapshot>,
    candidate: &SearchCandidate,
    recipes: &RecipeRegistry,
    execution_budget: &ExecutionBudget,
    tactical_goal: Option<&TacticalGoal>,
    landmark_set: &LandmarkSet,
) -> Option<(Option<PlanTerminalKind>, SearchNode<'snapshot>)> {
    build_successor_detailed(
        goal,
        semantics_table,
        registry,
        node,
        candidate,
        recipes,
        execution_budget,
        tactical_goal,
        landmark_set,
    )
    .ok()
}

#[allow(clippy::too_many_arguments)]
#[allow(clippy::trivially_copy_pass_by_ref)]
pub(super) fn build_successor_detailed<'snapshot>(
    goal: &GroundedGoal,
    semantics_table: &BTreeMap<ActionDefId, PlannerOpSemantics>,
    registry: &ActionDefRegistry,
    node: &SearchNode<'snapshot>,
    candidate: &SearchCandidate,
    recipes: &RecipeRegistry,
    execution_budget: &ExecutionBudget,
    tactical_goal: Option<&TacticalGoal>,
    landmark_set: &LandmarkSet,
) -> Result<
    (Option<PlanTerminalKind>, SearchNode<'snapshot>),
    crate::decision_trace::RootCandidateSkipReason,
> {
    let def = registry
        .get(candidate.def_id)
        .ok_or(crate::decision_trace::RootCandidateSkipReason::MissingActionDef)?;
    let semantics = semantics_table
        .get(&candidate.def_id)
        .ok_or(crate::decision_trace::RootCandidateSkipReason::MissingSemantics)?;
    let epistemic_subjects = grounded_goal_epistemic_subjects(goal, &node.state);
    let epistemic_barrier_active = !epistemic_subjects.is_empty();
    let is_goal_relevant = if tactical_goal.is_some() {
        true
    } else if epistemic_barrier_active {
        semantics.op_kind == PlannerOpKind::Travel
            || grounded_goal_matches_epistemic_barrier(
                &epistemic_subjects,
                semantics.op_kind,
                &candidate.authoritative_targets,
                candidate.payload_override.as_ref(),
            )
    } else {
        goal.key
            .kind
            .relevant_op_kinds()
            .contains(&semantics.op_kind)
    };
    if !is_goal_relevant {
        return Err(crate::decision_trace::RootCandidateSkipReason::IrrelevantGoalOp);
    }

    let actor = node.state.snapshot().actor();
    let payload_override = goal
        .key
        .kind
        .build_payload_override(
            candidate.payload_override.as_ref(),
            &node.state,
            &candidate.authoritative_targets,
            def,
            semantics,
        )
        .map_err(root_candidate_payload_error)?;
    let effective_payload = payload_override.as_ref().unwrap_or(&def.payload);
    let duration = node
        .state
        .estimate_duration(
            actor,
            &def.duration,
            &candidate.authoritative_targets,
            effective_payload,
        )
        .ok_or_else(
            || crate::decision_trace::RootCandidateSkipReason::DurationEstimateFailed {
                dependency: PlannerDurationDependency::from_duration_expr(def.duration)
                    .expect("planner search should not use fixed-duration failure diagnostics"),
            },
        )?;
    let estimated_ticks = duration.ticks();
    let transition = apply_hypothetical_transition(
        goal,
        *semantics,
        node.state.clone(),
        &candidate.planning_targets,
        payload_override.as_ref(),
    )
    .ok_or(crate::decision_trace::RootCandidateSkipReason::HypotheticalTransitionFailed)?;
    let step = PlannedStep {
        def_id: candidate.def_id,
        targets: transition.targets,
        payload_override,
        op_kind: semantics.op_kind,
        estimated_ticks,
        is_materialization_barrier: semantics.is_materialization_barrier,
        expected_materializations: transition.expected_materializations,
    };
    let terminal = terminal_kind(goal, &transition.state, &step, tactical_goal);
    if !semantics.may_appear_mid_plan && terminal.is_none() {
        return Err(crate::decision_trace::RootCandidateSkipReason::NonTerminalLeafOnly);
    }
    let total_estimated_ticks = node
        .total_estimated_ticks
        .checked_add(estimated_ticks)
        .ok_or(crate::decision_trace::RootCandidateSkipReason::TotalDurationOverflow)?;
    let search_step_cost = if semantics.op_kind == PlannerOpKind::Travel {
        let current_place = node
            .state
            .effective_place_ref(PlanningEntityRef::Authoritative(actor))
            .ok_or(crate::decision_trace::RootCandidateSkipReason::HypotheticalTransitionFailed)?;
        let destination = candidate
            .authoritative_targets
            .first()
            .copied()
            .ok_or(crate::decision_trace::RootCandidateSkipReason::MissingActionDef)?;
        node.state
            .snapshot()
            .direct_perceived_travel_cost(current_place, destination)
            .unwrap_or(estimated_ticks)
    } else {
        estimated_ticks
    };
    let search_cost = node
        .search_cost
        .checked_add(search_step_cost)
        .ok_or(crate::decision_trace::RootCandidateSkipReason::TotalDurationOverflow)?;
    let combined_places = combined_relevant_places_for_tactical(
        goal,
        &transition.state,
        recipes,
        execution_budget,
        tactical_goal,
    );
    let spatial_heuristic = compute_heuristic(
        node.state.snapshot(),
        &transition.state,
        &combined_places.places,
    );
    let landmark_heuristic =
        compute_landmark_heuristic(landmark_set, &planning_facts_from_state(&transition.state));
    let heuristic_ticks = spatial_heuristic.max(landmark_heuristic);
    let tactical_barrier_reached = node.tactical_barrier_reached
        || tactical_goal.is_some_and(|goal| goal.progress_barrier_satisfied(&transition.state));
    let mut steps = node.steps.clone();
    steps.push(step);

    Ok((
        terminal,
        SearchNode {
            state: transition.state,
            steps,
            total_estimated_ticks,
            search_cost,
            tactical_barrier_reached,
            heuristic_ticks,
        },
    ))
}

fn root_candidate_payload_error(
    error: GoalPayloadOverrideError,
) -> crate::decision_trace::RootCandidateSkipReason {
    use crate::decision_trace::PayloadOverrideFailureReason as TracePayloadError;

    let reason = match error {
        GoalPayloadOverrideError::MissingTarget => TracePayloadError::MissingTarget,
        GoalPayloadOverrideError::UnsupportedGoal => TracePayloadError::UnsupportedGoal,
        GoalPayloadOverrideError::MissingActorPlace => TracePayloadError::MissingActorPlace,
        GoalPayloadOverrideError::SellerUnavailable => TracePayloadError::SellerUnavailable,
        GoalPayloadOverrideError::SellerOutOfStock => TracePayloadError::SellerOutOfStock,
        GoalPayloadOverrideError::ActorCannotPay => TracePayloadError::ActorCannotPay,
    };
    crate::decision_trace::RootCandidateSkipReason::PayloadOverride(reason)
}

pub(super) fn terminal_kind(
    goal: &GroundedGoal,
    state: &crate::PlanningState<'_>,
    step: &PlannedStep,
    tactical_goal: Option<&TacticalGoal>,
) -> Option<PlanTerminalKind> {
    if matches!(step.op_kind, PlannerOpKind::Attack | PlannerOpKind::Defend) {
        return Some(PlanTerminalKind::CombatCommitment);
    }
    if goal.key.kind.is_satisfied(state) {
        return Some(PlanTerminalKind::GoalSatisfied);
    }
    if let Some(tactical_goal) = tactical_goal {
        match tactical_goal {
            TacticalGoal::Explore { .. } if tactical_goal.progress_barrier_satisfied(state) => {
                return Some(PlanTerminalKind::ProgressBarrier);
            }
            TacticalGoal::SocialQuery { .. } if step.op_kind == PlannerOpKind::AskWitness => {
                return Some(PlanTerminalKind::ProgressBarrier);
            }
            _ => {}
        }
    }
    let epistemic_subjects = grounded_goal_epistemic_subjects(goal, state);
    if grounded_goal_matches_epistemic_barrier(
        &epistemic_subjects,
        step.op_kind,
        &step
            .targets
            .iter()
            .filter_map(|target| match target {
                PlanningEntityRef::Authoritative(entity) => Some(*entity),
                PlanningEntityRef::Hypothetical(_) => None,
            })
            .collect::<Vec<_>>(),
        step.payload_override.as_ref(),
    ) {
        return Some(PlanTerminalKind::ProgressBarrier);
    }
    goal.key
        .kind
        .is_progress_barrier(step)
        .then_some(PlanTerminalKind::ProgressBarrier)
}
