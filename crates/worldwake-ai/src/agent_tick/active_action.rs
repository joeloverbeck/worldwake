use worldwake_core::{
    ActiveGoal, BlockedIntentMemory, CauseRef, EntityId, JourneyCommitment,
    JourneyCommitmentState, Permille, Tick,
};
use worldwake_sim::{
    ActionHandlerRegistry, PerAgentBeliefView, RuntimeBeliefView, SchedulerActionRuntime,
    TickInputError,
};

use crate::failure_handling::ExecutionFailure;
use crate::{
    classify_journey_plan_relation, evaluate_interrupt, handle_plan_failure, has_journey_commitment,
    AgentDecisionRuntime, DecisionContext, InterruptDecision, JourneyClearReason,
    PlanFailureContext, PlanTerminalKind, PlannedStep, PlanningBudget, RankedGoal,
};

use super::{
    build_candidate_plans, persist_blocked_memory, plans_as_options, AgentTickContext,
    JourneySwitchMarginSource,
};
use super::observation::{reconcile_in_flight_state, InFlightReconciliation};

pub(super) fn active_action_for_agent(
    ctx: &AgentTickContext<'_>,
    agent: EntityId,
) -> Option<worldwake_sim::ActionInstance> {
    ctx.scheduler
        .active_actions()
        .values()
        .find(|instance| instance.actor == agent)
        .cloned()
}

#[allow(clippy::too_many_arguments)]
pub(super) fn handle_active_action_phase(
    ctx: &mut AgentTickContext<'_>,
    runtime: &mut AgentDecisionRuntime,
    active_goal: &mut Option<ActiveGoal>,
    jc: &mut Option<JourneyCommitment>,
    facility_intents: &mut worldwake_core::FacilityQueueIntents,
    blocked_memory: &mut BlockedIntentMemory,
    agent: EntityId,
    ranked_candidates: &[RankedGoal],
    active_action: &worldwake_sim::ActionInstance,
    default_switch_margin: Permille,
    journey_switch_margin: Permille,
    tick: Tick,
    action_defs: &worldwake_sim::ActionDefRegistry,
    action_handlers: &ActionHandlerRegistry,
    decision_context: DecisionContext,
) -> Result<InterruptDecision, TickInputError> {
    let interruptibility = action_defs
        .get(active_action.def_id)
        .map_or(worldwake_sim::Interruptibility::NonInterruptible, |def| {
            def.interruptibility
        });
    let plan_valid = runtime
        .current_plan
        .as_ref()
        .is_some_and(|plan| runtime.current_step_index < plan.steps.len());
    let planned_candidates = has_journey_commitment(jc.as_ref()).then(|| {
        build_candidate_plans(
            ctx.world,
            ctx.scheduler,
            agent,
            ranked_candidates,
            blocked_memory,
            tick,
            ctx.budget,
            ctx.semantics_table,
            action_defs,
            action_handlers,
            ctx.recipe_registry,
            false,
            false,
        )
    });
    let planned_as_options = planned_candidates.as_ref().map(|p| plans_as_options(p));
    let active_goal_key = active_goal.map(|ag| ag.goal_key);
    let decision = evaluate_interrupt(
        runtime,
        active_goal_key,
        jc.as_ref(),
        interruptibility,
        ranked_candidates,
        planned_as_options.as_deref(),
        plan_valid,
        default_switch_margin,
        journey_switch_margin,
        &decision_context,
    );
    if let InterruptDecision::InterruptForReplan { trigger: _ } = decision {
        let replan = ctx
            .scheduler
            .interrupt_active_action(
                active_action.instance_id,
                SchedulerActionRuntime {
                    action_defs,
                    action_handlers,
                    world: ctx.world,
                    event_log: ctx.event_log,
                    rng: ctx.rng,
                },
                worldwake_sim::ActionExecutionContext {
                    cause: CauseRef::SystemTick(tick),
                    tick,
                },
                worldwake_sim::InterruptReason::Reprioritized,
            )
            .map_err(|error| TickInputError::new(format!("{error:?}")))?;
        reconcile_in_flight_state(
            ctx,
            runtime,
            active_goal,
            jc,
            facility_intents,
            blocked_memory,
            None,
            agent,
            InFlightReconciliation {
                replan_signals: &[&replan],
                start_failures: &[],
                committed_actions: &[],
            },
        )?;
    }

    Ok(decision)
}

pub(super) fn effective_goal_switch_margin(
    view: &dyn RuntimeBeliefView,
    agent: EntityId,
    jc: Option<&JourneyCommitment>,
    budget: &PlanningBudget,
) -> Permille {
    goal_switch_margin_details(view, agent, jc, budget).0
}

pub(super) fn goal_switch_margin_details(
    view: &dyn RuntimeBeliefView,
    agent: EntityId,
    jc: Option<&JourneyCommitment>,
    budget: &PlanningBudget,
) -> (Permille, JourneySwitchMarginSource) {
    if has_journey_commitment(jc) {
        if let Some(profile) = view.travel_disposition_profile(agent) {
            return (
                profile.route_replan_margin,
                JourneySwitchMarginSource::JourneyProfile,
            );
        }
    }

    (
        budget.switch_margin_permille,
        JourneySwitchMarginSource::BudgetDefault,
    )
}

/// Advance the step index after a completed step. Returns the updated
/// journey commitment (or `None` if it was cleared).
pub(super) fn advance_completed_step(
    runtime: &mut AgentDecisionRuntime,
    active_goal: &mut Option<ActiveGoal>,
    jc: Option<&JourneyCommitment>,
    completed_op_kind: crate::PlannerOpKind,
    tick: Tick,
) -> Option<JourneyCommitment> {
    let completed_plan_relation = runtime
        .current_plan
        .as_ref()
        .map(|plan| classify_journey_plan_relation(jc, plan));

    let mut updated_jc = jc.copied();

    if completed_op_kind == crate::PlannerOpKind::Travel {
        if let Some(ref mut c) = updated_jc {
            c.last_progress_tick = Some(tick);
            c.consecutive_blocked_leg_ticks = 0;
        }
    }

    runtime.current_step_index = runtime
        .current_step_index
        .checked_add(1)
        .expect("agent decision runtime step index overflowed");

    let Some(plan) = runtime.current_plan.as_ref() else {
        return updated_jc;
    };
    if runtime.current_step_index < plan.steps.len() {
        return updated_jc;
    }

    match plan.terminal_kind {
        PlanTerminalKind::ProgressBarrier => {
            *active_goal = Some(ActiveGoal {
                goal_key: plan.goal,
                adopted_at: tick,
            });
            runtime.current_plan = None;
            runtime.current_step_index = 0;
            runtime.dirty = true;
            runtime.materialization_bindings.clear();
        }
        PlanTerminalKind::GoalSatisfied | PlanTerminalKind::CombatCommitment => {
            if completed_plan_relation == Some(crate::JourneyPlanRelation::SuspendsCommitment) {
                if let Some(ref mut c) = updated_jc {
                    c.state = JourneyCommitmentState::Active;
                }
            } else {
                if updated_jc.is_some() {
                    runtime.last_journey_clear_reason =
                        Some(JourneyClearReason::GoalSatisfied);
                }
                updated_jc = None;
            }
            *active_goal = None;
            runtime.current_plan = None;
            runtime.current_step_index = 0;
            runtime.dirty = true;
            runtime.materialization_bindings.clear();
        }
    }

    updated_jc
}

#[allow(clippy::too_many_arguments)]
pub(super) fn handle_current_step_failure(
    ctx: &mut AgentTickContext<'_>,
    runtime: &mut AgentDecisionRuntime,
    active_goal: Option<worldwake_core::GoalKey>,
    jc: &mut Option<JourneyCommitment>,
    blocked_memory: &mut BlockedIntentMemory,
    agent: EntityId,
    step: &PlannedStep,
    execution_failure: Option<ExecutionFailure<'_>>,
) -> Result<(), TickInputError> {
    let world = &mut *ctx.world;
    let event_log = &mut *ctx.event_log;
    let budget = ctx.budget;
    let tick = ctx.tick;
    let view = PerAgentBeliefView::from_world(agent, world);
    let goal_key = active_goal.unwrap_or_else(|| {
        runtime
            .current_plan
            .as_ref()
            .map(|plan| plan.goal)
            .expect("failed step must have a current goal")
    });
    handle_plan_failure(
        &PlanFailureContext {
            view: &view,
            agent,
            goal_key,
            failed_step: step,
            execution_failure,
            current_tick: tick,
        },
        runtime,
        jc,
        blocked_memory,
        budget,
    );
    runtime.step_in_flight = false;
    runtime.current_step_index = 0;
    persist_blocked_memory(
        world,
        event_log,
        agent,
        tick,
        &BlockedIntentMemory::default(),
        blocked_memory,
    )
}
