use crate::{
    authoritative_target, classify_journey_plan_relation, has_active_journey_travel,
    AgentDecisionRuntime, JourneyClearReason, JourneyRuntimeSnapshot, PlannedStep, PlanningBudget,
};
use worldwake_core::JourneyCommitmentState;
use worldwake_core::{
    BlockedIntent, BlockedIntentMemory, BlockerKey, EntityId, JourneyCommitment, Permille, Tick,
};
use worldwake_sim::RuntimeBeliefView;

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum JourneySwitchMarginSource {
    BudgetDefault,
    JourneyProfile,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct JourneyDebugSnapshot {
    pub runtime: JourneyRuntimeSnapshot,
    pub effective_switch_margin: Permille,
    pub switch_margin_source: JourneySwitchMarginSource,
}

/// Updates the journey commitment for a newly adopted plan.
///
/// Returns the updated commitment (or `None` if it was cleared).
pub(super) fn update_journey_for_adopted_plan(
    jc: Option<&JourneyCommitment>,
    selected_plan: &crate::PlannedPlan,
    tick: Tick,
    runtime: &mut AgentDecisionRuntime,
) -> Option<JourneyCommitment> {
    let relation = classify_journey_plan_relation(jc, selected_plan);

    if relation == crate::JourneyPlanRelation::SuspendsCommitment {
        return jc.map(|c| JourneyCommitment {
            state: JourneyCommitmentState::Suspended,
            ..*c
        });
    }

    let Some(destination) = selected_plan.terminal_travel_destination() else {
        if jc.is_some() {
            runtime.last_journey_clear_reason = Some(JourneyClearReason::LostTravelPlan);
        }
        return None;
    };

    let same_commitment = relation == crate::JourneyPlanRelation::RefreshesCommitment;

    if same_commitment {
        if let Some(existing) = jc {
            return Some(JourneyCommitment {
                committed_goal: selected_plan.goal,
                destination,
                state: JourneyCommitmentState::Active,
                ..*existing
            });
        }
    }

    Some(JourneyCommitment {
        committed_goal: selected_plan.goal,
        destination,
        state: JourneyCommitmentState::Active,
        established_at: tick,
        last_progress_tick: None,
        consecutive_blocked_leg_ticks: 0,
    })
}

/// Handles a blocked travel step during an active journey. Returns `true`
/// if the blockage was handled (caller should not fall through to generic
/// failure handling).
///
/// Returns `(handled, updated_commitment)`. When `handled` is true, the
/// caller should use `updated_commitment` as the new journey state.
#[allow(clippy::too_many_arguments)]
pub(super) fn handle_recoverable_travel_step_blockage(
    view: &dyn RuntimeBeliefView,
    jc: Option<&JourneyCommitment>,
    runtime: &mut AgentDecisionRuntime,
    active_goal: Option<worldwake_core::GoalKey>,
    blocked_memory: &mut BlockedIntentMemory,
    agent: EntityId,
    step: &PlannedStep,
    tick: Tick,
    budget: &PlanningBudget,
) -> (bool, Option<JourneyCommitment>) {
    if step.op_kind != crate::PlannerOpKind::Travel
        || !has_active_journey_travel(
            jc,
            runtime.current_plan.as_ref(),
            runtime.current_step_index,
        )
    {
        return (false, jc.copied());
    }

    let commitment = jc.expect("active journey travel requires a commitment");
    let new_blocked = commitment
        .consecutive_blocked_leg_ticks
        .checked_add(1)
        .expect("consecutive blocked leg ticks overflowed");

    let patience_exhausted = view
        .travel_disposition_profile(agent)
        .is_some_and(|profile| new_blocked >= profile.blocked_leg_patience_ticks.get());

    let updated_commitment = if patience_exhausted {
        let goal_key = active_goal.unwrap_or_else(|| {
            runtime
                .current_plan
                .as_ref()
                .map(|plan| plan.goal)
                .expect("active journey travel must retain a current goal")
        });
        blocked_memory.record(BlockedIntent {
            blocker_key: BlockerKey {
                goal_key,
                place: blocked_leg_target(step),
                target: None,
                action_def: Some(step.def_id),
            },
            blocking_fact: worldwake_core::BlockingFact::NoKnownPath,
            diagnostic_context: None,
            observed_tick: tick,
            expires_tick: tick + u64::from(budget.structural_block_ticks),
        });
        runtime.last_journey_clear_reason = Some(JourneyClearReason::PatienceExhausted);
        None
    } else {
        Some(JourneyCommitment {
            consecutive_blocked_leg_ticks: new_blocked,
            ..*commitment
        })
    };

    runtime.current_plan = None;
    runtime.current_step_index = 0;
    runtime.materialization_bindings.clear();
    runtime.dirty = true;
    (true, updated_commitment)
}

fn blocked_leg_target(step: &PlannedStep) -> Option<EntityId> {
    step.targets.first().copied().and_then(authoritative_target)
}
