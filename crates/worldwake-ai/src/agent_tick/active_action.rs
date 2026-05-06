use worldwake_core::{
    ActionInterruptReasonTag, BlockerMemory, CauseRef, CognitiveProfile, ContentionIntents,
    DiscrepancyMemory, EntityId, FrameState, IntentionFrame, Permille, PlanInvalidationReason,
    ReplanReason, Tick,
};
use worldwake_sim::{
    AbortReason, ActionHandlerRegistry, InterruptReason, Interruptibility, PerAgentBeliefView,
    RuntimeBeliefView, SchedulerActionRuntime, TickInputError,
};

use super::frame::progress_op_kinds;
use super::observation::{InFlightReconciliation, reconcile_in_flight_state};
use super::{
    AgentTickContext, AssumptionRefContext, FrameSwitchMarginSource, build_candidate_plans,
    persist_blocked_memory, persist_discrepancy_memory,
};
use crate::DirtySet;
use crate::failure_handling::{ExecutionFailure, FailureClassification};
use crate::plan_step_expectations::{
    expire_plan_step_expectations, persist_expectation_store_update,
};
use crate::{
    AgendaEntry, AgentDecisionRuntime, DecisionContext, InterruptDecision, PendingRepairContext,
    PlanFailureContext, PlanTerminalKind, PlannedStep, classify_frame_plan_relation,
    evaluate_interrupt, handle_plan_failure, has_frame, ranking::OrderedRanked,
};

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

fn should_build_interrupt_plans(
    interruptibility: Interruptibility,
    runtime: &AgentDecisionRuntime,
    active_goal: Option<&AgendaEntry>,
    jc: Option<&IntentionFrame>,
) -> bool {
    if interruptibility != Interruptibility::FreelyInterruptible {
        return false;
    }

    has_frame(jc) || active_goal.is_some() || runtime.current_plan.is_some()
}

#[allow(clippy::too_many_arguments)]
pub(super) fn handle_active_action_phase(
    ctx: &mut AgentTickContext<'_>,
    runtime: &mut AgentDecisionRuntime,
    active_goal: &mut Option<AgendaEntry>,
    jc: &mut Option<IntentionFrame>,
    facility_intents: &mut worldwake_core::ContentionIntents,
    blocked_memory: &mut BlockerMemory,
    discrepancy_memory: &mut DiscrepancyMemory,
    agent: EntityId,
    ranked_candidates: &OrderedRanked<'_>,
    active_action: &worldwake_sim::ActionInstance,
    default_switch_margin: Permille,
    frame_switch_margin: Permille,
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
    // Only compute candidate plans when the interrupt pathway actually uses them.
    // `planned_candidates` is consumed only by `interrupt_freely`, so we can skip
    // the expensive GOAP search for NonInterruptible and InterruptibleWithPenalty
    // actions.
    let needs_plans =
        should_build_interrupt_plans(interruptibility, runtime, active_goal.as_ref(), jc.as_ref());
    let planned_candidates = needs_plans.then(|| {
        build_candidate_plans(
            ctx.world,
            ctx.scheduler,
            agent,
            ranked_candidates,
            None,
            discrepancy_memory,
            blocked_memory,
            tick,
            ctx.cognitive,
            ctx.execution_budget,
            ctx.semantics_table,
            action_defs,
            action_handlers,
            ctx.recipe_registry,
            false,
            false,
            &runtime.exhaustion_cache,
        )
    });
    let selection_plans = planned_candidates
        .as_ref()
        .map(super::planning::CandidatePlanningPass::selection_plans);
    let active_goal_key = active_goal.as_ref().map(|ag| ag.key.goal_key);
    let decision = evaluate_interrupt(
        runtime,
        active_goal_key,
        jc.as_ref(),
        interruptibility,
        ranked_candidates,
        selection_plans.as_deref(),
        plan_valid,
        default_switch_margin,
        frame_switch_margin,
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
                    recipe_registry: ctx.recipe_registry,
                    action_defs,
                },
                worldwake_sim::InterruptReason::Reprioritized,
            )
            .map_err(|error| TickInputError::new(format!("{error:?}")))?;
        // Pass the agent's real discrepancy_memory through. A throwaway
        // `DiscrepancyMemory::default()` here would silently lose any
        // discrepancy `reconcile_in_flight_state` records during interrupt
        // reconciliation (e.g. handling the replan signal that just fired),
        // because the throwaway is dropped at the end of this scope and
        // never persisted to the agent's component.
        let _ = reconcile_in_flight_state(
            ctx,
            runtime,
            active_goal,
            jc,
            facility_intents,
            blocked_memory,
            discrepancy_memory,
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
    jc: Option<&IntentionFrame>,
    cognitive: &CognitiveProfile,
) -> Permille {
    goal_switch_margin_details(view, agent, jc, cognitive).0
}

pub(super) fn goal_switch_margin_details(
    view: &dyn RuntimeBeliefView,
    agent: EntityId,
    jc: Option<&IntentionFrame>,
    cognitive: &CognitiveProfile,
) -> (Permille, FrameSwitchMarginSource) {
    if has_frame(jc) {
        let profile = view
            .intention_disposition_profile(agent)
            .unwrap_or_else(|| panic!("agent {agent} lacks IntentionDispositionProfile"));
        return (
            profile.commitment_switch_margin,
            FrameSwitchMarginSource::FrameProfile,
        );
    }

    (
        cognitive.switch_margin,
        FrameSwitchMarginSource::CognitiveProfile,
    )
}

/// Advance the step index after a completed step. Returns the updated
/// intention frame (or `None` if it was cleared).
pub(super) fn advance_completed_step(
    runtime: &mut AgentDecisionRuntime,
    active_goal: &mut Option<AgendaEntry>,
    facility_intents: &mut ContentionIntents,
    jc: Option<&IntentionFrame>,
    completed_op_kind: crate::PlannerOpKind,
    tick: Tick,
) -> Option<IntentionFrame> {
    let completed_plan_relation = runtime
        .current_plan
        .as_ref()
        .map(|plan| classify_frame_plan_relation(jc, plan));

    let mut updated_jc = jc.cloned();

    if let Some(ref mut c) = updated_jc
        && progress_op_kinds(&c.domain).contains(&completed_op_kind)
    {
        c.last_progress_tick = Some(tick);
        c.stalled_ticks = 0;
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
            runtime.current_plan = None;
            runtime.current_step_index = 0;
            runtime.dirty.insert(DirtySet::PLAN_FINISHED);
            runtime.materialization_bindings.clear();
            facility_intents.intents.clear();
        }
        PlanTerminalKind::GoalSatisfied | PlanTerminalKind::CombatCommitment => {
            if completed_plan_relation == Some(crate::FramePlanRelation::SuspendsFrame) {
                if let Some(ref mut c) = updated_jc {
                    c.state = FrameState::Active;
                }
            } else {
                if updated_jc.is_some() {
                    runtime.last_frame_clear_reason =
                        Some(worldwake_core::FrameClearReason::GoalSatisfied);
                }
                updated_jc = None;
            }
            *active_goal = None;
            runtime.current_plan = None;
            runtime.current_step_index = 0;
            runtime.dirty.insert(DirtySet::PLAN_FINISHED);
            runtime.materialization_bindings.clear();
            facility_intents.intents.clear();
        }
    }

    updated_jc
}

#[allow(clippy::too_many_arguments)]
pub(super) fn handle_current_step_failure(
    ctx: &mut AgentTickContext<'_>,
    runtime: &mut AgentDecisionRuntime,
    active_goal: Option<worldwake_core::GoalKey>,
    jc: &mut Option<IntentionFrame>,
    blocked_memory: &mut BlockerMemory,
    discrepancy_memory: &mut DiscrepancyMemory,
    facility_intents: &mut ContentionIntents,
    agent: EntityId,
    step: &PlannedStep,
    execution_failure: Option<ExecutionFailure<'_>>,
    belief_discrepancy: Option<worldwake_core::Discrepancy>,
    plan_invalidation_reason: Option<PlanInvalidationReason>,
) -> Result<ReplanReason, TickInputError> {
    let tick = ctx.tick;
    let _ = persist_expectation_store_update(ctx.world, ctx.event_log, agent, tick, |store| {
        expire_plan_step_expectations(store)
    })?;
    let world = &mut *ctx.world;
    let event_log = &mut *ctx.event_log;
    let cognitive = ctx.cognitive;
    let view = PerAgentBeliefView::from_world(agent, world);
    let active_assumptions = jc
        .as_ref()
        .map_or_else(Vec::new, |frame| frame.assumptions.clone());
    let active_plan = runtime.current_plan.clone();
    let goal_key = active_goal.unwrap_or_else(|| {
        runtime
            .current_plan
            .as_ref()
            .map(|plan| plan.goal)
            .expect("failed step must have a current goal")
    });
    if let Some(failed_plan) = runtime.current_plan.clone() {
        runtime.pending_repair_context = Some(PendingRepairContext {
            failed_plan,
            failed_step_index: runtime
                .current_step_index
                .try_into()
                .expect("failed repair step index exceeds u16"),
        });
    }
    runtime.accepted_repair = None;
    let classification = handle_plan_failure(
        &PlanFailureContext {
            view: &view,
            agent,
            goal_key,
            failed_step: step,
            execution_failure,
            belief_discrepancy,
            current_tick: tick,
        },
        runtime,
        jc,
        blocked_memory,
        discrepancy_memory,
        facility_intents,
        cognitive,
    );
    let replan_reason =
        resolve_replan_reason(plan_invalidation_reason, execution_failure, classification);
    runtime.step_in_flight = false;
    runtime.current_step_index = 0;
    persist_blocked_memory(
        world,
        event_log,
        agent,
        tick,
        &BlockerMemory::default(),
        blocked_memory,
        AssumptionRefContext::new(&active_assumptions, cognitive.decision_history_alternatives)
            .with_plan(active_plan.as_ref()),
    )?;
    persist_discrepancy_memory(
        world,
        event_log,
        agent,
        tick,
        &DiscrepancyMemory::default(),
        discrepancy_memory,
        AssumptionRefContext::new(&active_assumptions, cognitive.decision_history_alternatives)
            .with_plan(active_plan.as_ref()),
    )?;
    Ok(replan_reason)
}

fn resolve_replan_reason(
    plan_invalidation_reason: Option<PlanInvalidationReason>,
    execution_failure: Option<ExecutionFailure<'_>>,
    classification: FailureClassification,
) -> ReplanReason {
    plan_invalidation_reason.map_or_else(
        || map_replan_reason(execution_failure, classification),
        |reason| ReplanReason::PlanInvalidated { reason },
    )
}

fn map_replan_reason(
    execution_failure: Option<ExecutionFailure<'_>>,
    classification: FailureClassification,
) -> ReplanReason {
    match execution_failure {
        Some(ExecutionFailure::Replan(signal)) => match &signal.reason {
            AbortReason::Interrupted { kind, .. } => ReplanReason::ActionInterrupted {
                reason: map_interrupt_reason(*kind),
            },
            AbortReason::CommitConditionFailed { .. } | AbortReason::ExternalAbort { .. } => {
                map_failure_classification(classification)
            }
        },
        Some(ExecutionFailure::Start(_)) => ReplanReason::ActionStartFailed,
        None => map_failure_classification(classification),
    }
}

fn map_failure_classification(classification: FailureClassification) -> ReplanReason {
    match classification {
        FailureClassification::Blocker(blocking_fact) => {
            ReplanReason::BlockingFactRecorded { blocking_fact }
        }
        FailureClassification::Discrepancy(discrepancy) => {
            ReplanReason::DiscrepancyRecorded { discrepancy }
        }
    }
}

fn map_interrupt_reason(reason: InterruptReason) -> ActionInterruptReasonTag {
    match reason {
        InterruptReason::DangerNearby => ActionInterruptReasonTag::DangerNearby,
        InterruptReason::Reprioritized => ActionInterruptReasonTag::Reprioritized,
        InterruptReason::Other => ActionInterruptReasonTag::Other,
    }
}

#[cfg(test)]
mod tests {
    use super::{map_replan_reason, resolve_replan_reason, should_build_interrupt_plans};
    use crate::failure_handling::{ExecutionFailure, FailureClassification};
    use crate::{
        AgendaEntry, AgendaEntryKey, AgendaOrigin, AgendaPhase, AgentDecisionRuntime,
        FeasibilityHint, GoalOffer, KillCondition, PlannedPlan, PlannedStep,
    };
    use std::collections::BTreeSet;
    use worldwake_core::{
        AcquisitionQuantity, ActionDefId, EntityId, GoalKey, GoalKind, IntentionFrame,
        OpportunityAnchor, PlanInvalidationReason, ReplanReason, Tick,
    };
    use worldwake_sim::{
        AbortReason, ActionInstanceId, ActionStartFailure, ActionStartFailureReason,
        InterruptReason, Interruptibility, ReplanNeeded, RequestAttemptTrace, RequestBindingKind,
        RequestProvenance, ResolvedRequestTrace,
    };

    fn entity(slot: u32) -> EntityId {
        EntityId {
            slot,
            generation: 0,
        }
    }

    fn committed_goal(goal_key: GoalKey, tick: Tick) -> AgendaEntry {
        AgendaEntry {
            key: AgendaEntryKey {
                goal_key,
                anchor: OpportunityAnchor::None,
            },
            offer: GoalOffer {
                key: goal_key,
                anchor: OpportunityAnchor::None,
                evidence_entities: BTreeSet::new(),
                evidence_places: BTreeSet::new(),
                obligation_source: None,
                commitment_impact_if_ignored: worldwake_core::Permille::ZERO,
                required_information_gaps: Vec::new(),
                invalidators: Vec::new(),
                learned_expectation_refs: Vec::new(),
                acquisition_quantity: None,
            },
            phase: AgendaPhase::Committed,
            origin: AgendaOrigin::NeedDrive,
            introduced_tick: tick,
            last_reconsidered_tick: tick,
            revival_trigger: None,
            kill_condition: KillCondition::External,
            priority_class: crate::GoalPriorityClass::Background,
            motive_score: 0,
            provenance: None,
            source_reliability_discount: None,
            competition_discount: None,
            source_composite: None,
            feasibility: FeasibilityHint::Uncertain,
        }
    }

    #[test]
    fn frame_less_current_plan_still_builds_interrupt_plans() {
        let current_goal = GoalKey::from(GoalKind::AcquireCommodity {
            commodity: worldwake_core::CommodityKind::Water,
            purpose: worldwake_core::CommodityPurpose::SelfConsume,
            quantity: AcquisitionQuantity::single(),
        });
        let runtime = AgentDecisionRuntime {
            current_plan: Some(PlannedPlan::new(
                worldwake_core::OpportunityKey {
                    goal_key: current_goal,
                    anchor: OpportunityAnchor::None,
                },
                current_goal,
                vec![PlannedStep {
                    def_id: ActionDefId(1),
                    targets: vec![crate::PlanningEntityRef::Authoritative(entity(7))],
                    target_place: None,
                    payload_override: None,
                    op_kind: crate::PlannerOpKind::Travel,
                    estimated_ticks: 3,
                    is_materialization_barrier: false,
                    expected_materializations: Vec::new(),
                    guard: None,
                    expectations: Vec::new(),
                }],
                crate::PlanTerminalKind::ProgressBarrier,
            )),
            ..AgentDecisionRuntime::default()
        };

        assert!(should_build_interrupt_plans(
            Interruptibility::FreelyInterruptible,
            &runtime,
            Some(&committed_goal(current_goal, Tick(5))),
            Option::<&IntentionFrame>::None,
        ));
    }

    #[test]
    fn non_interruptible_actions_skip_interrupt_planning_without_frame() {
        let runtime = AgentDecisionRuntime::default();

        assert!(!should_build_interrupt_plans(
            Interruptibility::NonInterruptible,
            &runtime,
            None,
            Option::<&IntentionFrame>::None,
        ));
    }

    fn sample_replan_needed(reason: AbortReason) -> ReplanNeeded {
        ReplanNeeded {
            agent: entity(1),
            failed_action_def: ActionDefId(7),
            failed_instance: ActionInstanceId(9),
            reason,
            tick: Tick(3),
        }
    }

    fn sample_start_failure() -> ActionStartFailure {
        ActionStartFailure {
            tick: Tick(3),
            actor: entity(1),
            def_id: ActionDefId(7),
            request: ResolvedRequestTrace {
                attempt: RequestAttemptTrace {
                    input_sequence_no: 1,
                    provenance: RequestProvenance::AiPlan,
                },
                binding: RequestBindingKind::BestEffortFallback,
            },
            reason: ActionStartFailureReason::PreconditionFailed("blocked".to_string()),
        }
    }

    #[test]
    fn interrupt_replan_reason_preserves_interrupt_cause() {
        let signal = sample_replan_needed(AbortReason::interrupted(InterruptReason::Reprioritized));

        let reason = map_replan_reason(
            Some(ExecutionFailure::Replan(&signal)),
            FailureClassification::Discrepancy(worldwake_core::Discrepancy::ImproperPlanningState),
        );

        assert_eq!(
            reason,
            ReplanReason::ActionInterrupted {
                reason: worldwake_core::ActionInterruptReasonTag::Reprioritized,
            }
        );
    }

    #[test]
    fn start_failure_replan_reason_stays_distinct_from_failure_classification() {
        let start_failure = sample_start_failure();

        let reason = map_replan_reason(
            Some(ExecutionFailure::Start(&start_failure)),
            FailureClassification::Blocker(worldwake_core::BlockingFact::NoKnownPath),
        );

        assert_eq!(reason, ReplanReason::ActionStartFailed);
    }

    #[test]
    fn explicit_plan_invalidation_reason_overrides_failure_classification() {
        let start_failure = sample_start_failure();

        let reason = resolve_replan_reason(
            Some(PlanInvalidationReason::ExpectationMismatch { step_index: 2 }),
            Some(ExecutionFailure::Start(&start_failure)),
            FailureClassification::Blocker(worldwake_core::BlockingFact::NoKnownPath),
        );

        assert_eq!(
            reason,
            ReplanReason::PlanInvalidated {
                reason: PlanInvalidationReason::ExpectationMismatch { step_index: 2 },
            }
        );
    }
}
