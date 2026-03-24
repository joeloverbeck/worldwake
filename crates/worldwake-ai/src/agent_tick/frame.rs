use crate::{
    authoritative_target, classify_frame_plan_relation, has_active_frame_travel,
    AgentDecisionRuntime, FrameRuntimeSnapshot, PlannedStep, PlanningBudget,
};
use worldwake_core::{
    BlockedIntent, BlockedIntentMemory, BlockerKey, EntityId, FrameClearReason, FrameState,
    IntentionDomain, IntentionFrame, Permille, SuspensionReason, Tick,
};
use worldwake_sim::RuntimeBeliefView;

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum FrameSwitchMarginSource {
    BudgetDefault,
    FrameProfile,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FrameDebugSnapshot {
    pub runtime: FrameRuntimeSnapshot,
    pub effective_switch_margin: Permille,
    pub switch_margin_source: FrameSwitchMarginSource,
}

/// Updates the intention frame for a newly adopted plan.
///
/// Returns the updated frame (or `None` if it was cleared).
pub(super) fn update_frame_for_adopted_plan(
    frame: Option<&IntentionFrame>,
    selected_plan: &crate::PlannedPlan,
    tick: Tick,
    runtime: &mut AgentDecisionRuntime,
) -> Option<IntentionFrame> {
    let relation = classify_frame_plan_relation(frame, selected_plan);

    if relation == crate::FramePlanRelation::SuspendsFrame {
        return frame.map(|f| IntentionFrame {
            state: FrameState::Suspended {
                reason: SuspensionReason::PriorityInterrupt,
                suspended_at: tick,
            },
            ..f.clone()
        });
    }

    let Some(destination) = selected_plan.terminal_travel_destination() else {
        if frame.is_some() {
            runtime.last_frame_clear_reason = Some(FrameClearReason::LostPlan);
        }
        return None;
    };

    let same_frame = relation == crate::FramePlanRelation::RefreshesFrame;

    if same_frame {
        if let Some(existing) = frame {
            return Some(IntentionFrame {
                goal: selected_plan.goal,
                domain: IntentionDomain::Travel { destination },
                state: FrameState::Active,
                ..existing.clone()
            });
        }
    }

    Some(IntentionFrame {
        goal: selected_plan.goal,
        domain: IntentionDomain::Travel { destination },
        assumptions: Vec::new(),
        state: FrameState::Active,
        established_at: tick,
        last_progress_tick: None,
        stalled_ticks: 0,
        patience_limit: 30, // default; caller may override from profile
    })
}

/// Handles a blocked travel step during an active frame. Returns `true`
/// if the blockage was handled (caller should not fall through to generic
/// failure handling).
///
/// Returns `(handled, updated_frame)`. When `handled` is true, the
/// caller should use `updated_frame` as the new frame state.
#[allow(clippy::too_many_arguments)]
pub(super) fn handle_recoverable_travel_step_blockage(
    view: &dyn RuntimeBeliefView,
    frame: Option<&IntentionFrame>,
    runtime: &mut AgentDecisionRuntime,
    active_goal: Option<worldwake_core::GoalKey>,
    blocked_memory: &mut BlockedIntentMemory,
    agent: EntityId,
    step: &PlannedStep,
    tick: Tick,
    budget: &PlanningBudget,
) -> (bool, Option<IntentionFrame>) {
    if step.op_kind != crate::PlannerOpKind::Travel
        || !has_active_frame_travel(
            frame,
            runtime.current_plan.as_ref(),
            runtime.current_step_index,
        )
    {
        return (false, frame.cloned());
    }

    let f = frame.expect("active frame travel requires a frame");
    let new_stalled = f
        .stalled_ticks
        .checked_add(1)
        .expect("stalled ticks overflowed");

    let patience_exhausted = view
        .intention_disposition_profile(agent)
        .is_some_and(|profile| {
            new_stalled >= profile.patience_for(f.domain.domain_tag())
        });

    let updated_frame = if patience_exhausted {
        let goal_key = active_goal.unwrap_or_else(|| {
            runtime
                .current_plan
                .as_ref()
                .map(|plan| plan.goal)
                .expect("active frame travel must retain a current goal")
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
        runtime.last_frame_clear_reason = Some(FrameClearReason::PatienceExhausted);
        None
    } else {
        Some(IntentionFrame {
            stalled_ticks: new_stalled,
            ..f.clone()
        })
    };

    runtime.current_plan = None;
    runtime.current_step_index = 0;
    runtime.materialization_bindings.clear();
    runtime.dirty = true;
    (true, updated_frame)
}

fn blocked_leg_target(step: &PlannedStep) -> Option<EntityId> {
    step.targets.first().copied().and_then(authoritative_target)
}
