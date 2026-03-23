use crate::{
    authoritative_target, AgentDecisionRuntime, JourneyClearReason, JourneyCommitmentState,
    JourneyRuntimeSnapshot, PlannedStep, PlanningBudget,
};
use worldwake_core::{BlockedIntent, BlockedIntentMemory, EntityId, Permille, Tick};
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

pub(super) fn update_journey_fields_for_adopted_plan(
    runtime: &mut AgentDecisionRuntime,
    selected_plan: &crate::PlannedPlan,
    tick: Tick,
) {
    let relation = runtime.classify_journey_plan_relation(selected_plan);

    if relation == crate::JourneyPlanRelation::SuspendsCommitment {
        runtime.journey_commitment_state = JourneyCommitmentState::Suspended;
        return;
    }

    let Some(destination) = selected_plan.terminal_travel_destination() else {
        runtime.clear_journey_commitment_with_reason(JourneyClearReason::LostTravelPlan);
        return;
    };

    let same_commitment = relation == crate::JourneyPlanRelation::RefreshesCommitment;
    runtime.journey_committed_goal = Some(selected_plan.goal);
    runtime.journey_committed_destination = Some(destination);
    runtime.journey_commitment_state = JourneyCommitmentState::Active;
    if runtime.journey_established_at.is_some() && same_commitment {
        return;
    }

    runtime.journey_established_at = Some(tick);
    runtime.journey_last_progress_tick = None;
    runtime.consecutive_blocked_leg_ticks = 0;
}

pub(super) fn handle_recoverable_travel_step_blockage(
    view: &dyn RuntimeBeliefView,
    runtime: &mut AgentDecisionRuntime,
    blocked_memory: &mut BlockedIntentMemory,
    agent: EntityId,
    step: &PlannedStep,
    tick: Tick,
    budget: &PlanningBudget,
) -> bool {
    if step.op_kind != crate::PlannerOpKind::Travel || !runtime.has_active_journey_travel() {
        return false;
    }

    runtime.consecutive_blocked_leg_ticks = runtime
        .consecutive_blocked_leg_ticks
        .checked_add(1)
        .expect("consecutive blocked leg ticks overflowed");

    let patience_exhausted = view
        .travel_disposition_profile(agent)
        .is_some_and(|profile| {
            runtime.consecutive_blocked_leg_ticks >= profile.blocked_leg_patience_ticks.get()
        });

    if patience_exhausted {
        let goal_key = runtime.current_goal.unwrap_or_else(|| {
            runtime
                .current_plan
                .as_ref()
                .map(|plan| plan.goal)
                .expect("active journey travel must retain a current goal")
        });
        blocked_memory.record(BlockedIntent {
            goal_key,
            blocking_fact: worldwake_core::BlockingFact::NoKnownPath,
            related_entity: None,
            related_place: blocked_leg_target(step),
            related_action: None,
            observed_tick: tick,
            expires_tick: tick + u64::from(budget.structural_block_ticks),
        });
        runtime.clear_journey_commitment_with_reason(JourneyClearReason::PatienceExhausted);
    }

    runtime.current_plan = None;
    runtime.current_step_index = 0;
    runtime.materialization_bindings.clear();
    runtime.dirty = true;
    true
}

fn blocked_leg_target(step: &PlannedStep) -> Option<EntityId> {
    step.targets.first().copied().and_then(authoritative_target)
}
