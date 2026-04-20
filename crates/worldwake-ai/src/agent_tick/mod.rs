mod active_action;
mod candidates;
mod execution;
mod frame;
mod observation;
mod planning;
use active_action::{
    active_action_for_agent, advance_completed_step, effective_goal_switch_margin,
    goal_switch_margin_details, handle_active_action_phase, handle_current_step_failure,
};
use candidates::abandon_expired_facility_queues;
use execution::{
    apply_step_materialization_bindings, committed_action_for_step, current_step,
    enqueue_valid_step_or_handle_failure, finalize_agent_tick, persist_active_goal,
    persist_blocked_memory, persist_discrepancy_memory, persist_facility_queue_intents,
    persist_intention_frame, plan_finished,
};
use frame::{
    AssumptionEvalResult, apply_assumption_result, check_patience_exhaustion, evaluate_assumptions,
    handle_recoverable_travel_step_blockage, populate_assumptions, record_assumption_failure,
    update_frame_for_adopted_plan,
};
pub use frame::{FrameDebugSnapshot, FrameSwitchMarginSource};
use observation::{
    CompletedPlanSummary, InFlightReconciliation, ReadPhaseContext, reconcile_in_flight_state,
    refresh_runtime_for_read_phase_with_memories, update_runtime_observation_snapshot,
};
use planning::{
    build_candidate_plans, plan_and_validate_next_step_traced, selection_candidates,
    summarize_ranked_goal, summarize_step,
};

use crate::decision_trace::{
    ActionStartFailureSummary, AffordanceSummary, AffordanceTrace, AgentDecisionTrace,
    CandidateTrace, DecisionOutcome, DecisionTraceSink, DiscrepancyTrace, ExecutionFailureReason,
    ExecutionTrace, ExhaustionTraceEntry, FrameTransitionKind, FrameTransitionTrace,
    InterruptTrace, PatrolRouteSnapshotTrace, PlanSearchTrace, PlanningPipelineTrace,
    SelectionTrace,
};
use crate::{
    AgentDecisionRuntime, PlannerOpSemantics, authoritative_target, build_semantics_table,
    frame_runtime_snapshot,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use worldwake_core::FrameClearReason;
use worldwake_core::{
    ActionDefId, ActiveGoal, BlockerMemory, CauseRef, CognitiveProfile, ContentionIntents,
    ControlSource, DecisionEventPayload, EntityId, EventPayload, EventTag, ExecutionBudget,
    GoalAbandonReason, GoalAbandonedPayload, GoalOfferedPayload, GoalSuspendedPayload,
    GoalSuppressedPayload, GoalSwitchReason, IntentionFrame, LastProactiveExplorationTick,
    LearnedOpportunityMemory, OpportunityAnchor, OpportunityEntry, PendingEvent,
    PlanInvalidatedPayload, PlanInvalidationReason, PursuitInvalidationReasonTag, RepairEntry,
    RepairKey, RepairMemory, ReplanReason, ReplanTriggeredPayload, Tick, VisibilitySpec,
    WitnessData, WorldTxn,
};
use worldwake_sim::{
    ActionHandlerRegistry, AutonomousController, AutonomousControllerContext, CommittedAction,
    EntityBeliefView, PerAgentBeliefRuntime, PerAgentBeliefView, RecipeRegistry, ReplanNeeded,
    RuntimeBeliefView, SaveError, SaveableRuntime, Scheduler, SpatialBeliefView, TickInputError,
};

pub struct AgentTickDriver {
    runtime_by_agent: BTreeMap<EntityId, AgentDecisionRuntime>,
    semantics_cache: Option<(usize, BTreeMap<ActionDefId, PlannerOpSemantics>)>,
    /// Optional trace collector. When `Some`, decision traces are recorded.
    trace_sink: Option<DecisionTraceSink>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
struct AgentTickDriverState {
    runtime_by_agent: BTreeMap<EntityId, AgentDecisionRuntime>,
}

impl AgentTickDriver {
    #[must_use]
    pub fn new() -> Self {
        Self {
            runtime_by_agent: BTreeMap::new(),
            semantics_cache: None,
            trace_sink: None,
        }
    }

    pub fn from_saved_runtime(
        bytes: &[u8],
        world: &worldwake_core::World,
    ) -> Result<Self, SaveError> {
        let mut driver = Self::new();
        driver.restore_runtime_state(bytes)?;
        driver.post_load_validate(world);
        Ok(driver)
    }

    /// Enable decision tracing. Must be called before stepping.
    pub fn enable_tracing(&mut self) {
        self.trace_sink = Some(DecisionTraceSink::new());
    }

    /// Read access to the trace sink.
    #[must_use]
    pub fn trace_sink(&self) -> Option<&DecisionTraceSink> {
        self.trace_sink.as_ref()
    }

    /// Mutable access to the trace sink (for tests).
    pub fn trace_sink_mut(&mut self) -> Option<&mut DecisionTraceSink> {
        self.trace_sink.as_mut()
    }

    fn semantics_table(
        &mut self,
        action_defs: &worldwake_sim::ActionDefRegistry,
    ) -> &BTreeMap<ActionDefId, PlannerOpSemantics> {
        let action_count = action_defs.len();
        let rebuild = self
            .semantics_cache
            .as_ref()
            .is_none_or(|(cached_len, _)| *cached_len != action_count);
        if rebuild {
            self.semantics_cache = Some((action_count, build_semantics_table(action_defs)));
        }

        &self
            .semantics_cache
            .as_ref()
            .expect("semantics cache must exist after rebuild")
            .1
    }

    #[must_use]
    pub fn frame_snapshot(
        &self,
        world: &worldwake_core::World,
        agent: EntityId,
    ) -> Option<FrameDebugSnapshot> {
        let runtime = self.runtime_by_agent.get(&agent)?;
        let frame = world.get_component_intention_frame(agent);
        let view = PerAgentBeliefView::from_world(agent, world);
        let cognitive = world
            .get_component_cognitive_profile(agent)
            .cloned()
            .unwrap_or_else(|| panic!("AI agent {agent} lacks CognitiveProfile"));
        let (effective_switch_margin, switch_margin_source) =
            goal_switch_margin_details(&view, agent, frame, &cognitive);
        Some(FrameDebugSnapshot {
            runtime: frame_runtime_snapshot(frame, runtime),
            effective_switch_margin,
            switch_margin_source,
            patrol_route: patrol_route_snapshot(&view, agent),
        })
    }

    fn post_load_validate(&mut self, _world: &worldwake_core::World) {
        // All runtime fields are now serialized, so save/load is lossless.
        // No post-load fixups needed — the deserialized state is identical
        // to the pre-save state, preserving replay determinism.
        self.semantics_cache = None;
    }

    fn restore_runtime_state(&mut self, bytes: &[u8]) -> Result<(), SaveError> {
        let state: AgentTickDriverState = bincode::deserialize(bytes)
            .map_err(|error| SaveError::RuntimeDeserialization(error.to_string()))?;
        self.runtime_by_agent = state.runtime_by_agent;
        self.semantics_cache = None;
        self.trace_sink = None;
        Ok(())
    }
}

impl Default for AgentTickDriver {
    fn default() -> Self {
        Self::new()
    }
}

fn patrol_route_snapshot(
    view: &dyn RuntimeBeliefView,
    agent: EntityId,
) -> PatrolRouteSnapshotTrace {
    let route = view.patrol_route(agent);
    let current_waypoint = route
        .as_ref()
        .and_then(|route| route.assigned_places.get(route.current_index).copied());
    PatrolRouteSnapshotTrace {
        route,
        current_waypoint,
    }
}

impl SaveableRuntime for AgentTickDriver {
    fn save_runtime_state(&self) -> Result<Vec<u8>, SaveError> {
        bincode::serialize(&AgentTickDriverState {
            runtime_by_agent: self.runtime_by_agent.clone(),
        })
        .map_err(|error| SaveError::RuntimeSerialization(error.to_string()))
    }
}

pub(super) fn runtime_belief_view<'a>(
    agent: EntityId,
    world: &'a worldwake_core::World,
    scheduler: &'a Scheduler,
    action_defs: &'a worldwake_sim::ActionDefRegistry,
    recipe_registry: &'a RecipeRegistry,
) -> PerAgentBeliefView<'a> {
    PerAgentBeliefView::with_runtime_from_world_at_tick_with_recipes(
        agent,
        scheduler.current_tick(),
        world,
        Some(recipe_registry),
        PerAgentBeliefRuntime::new(scheduler.active_actions(), action_defs),
    )
}

pub(super) struct AgentTickContext<'a> {
    pub(super) world: &'a mut worldwake_core::World,
    pub(super) event_log: &'a mut worldwake_core::EventLog,
    pub(super) scheduler: &'a mut Scheduler,
    pub(super) rng: &'a mut worldwake_sim::DeterministicRng,
    pub(super) action_defs: &'a worldwake_sim::ActionDefRegistry,
    pub(super) action_handlers: &'a ActionHandlerRegistry,
    pub(super) recipe_registry: &'a RecipeRegistry,
    pub(super) semantics_table: &'a BTreeMap<ActionDefId, PlannerOpSemantics>,
    pub(super) cognitive: &'a CognitiveProfile,
    pub(super) execution_budget: &'a ExecutionBudget,
    pub(super) tick: Tick,
}

pub(super) fn emit_decision_event(
    event_log: &mut worldwake_core::EventLog,
    tick: Tick,
    agent: EntityId,
    tag: EventTag,
    decision_payload: DecisionEventPayload,
) {
    let _ = event_log.emit(PendingEvent::from_payload(EventPayload {
        tick,
        cause: CauseRef::SystemTick(tick),
        actor_id: Some(agent),
        action_name: None,
        target_ids: Vec::new(),
        evidence: Vec::new(),
        place_id: None,
        state_deltas: Vec::new(),
        observed_entities: BTreeMap::new(),
        visibility: VisibilitySpec::Hidden,
        witness_data: WitnessData::default(),
        tags: BTreeSet::from([tag]),
        decision_payload: Some(decision_payload),
    }));
}

fn emit_plan_invalidated(
    event_log: &mut worldwake_core::EventLog,
    tick: Tick,
    agent: EntityId,
    goal_key: worldwake_core::GoalKey,
    reason: PlanInvalidationReason,
) {
    emit_decision_event(
        event_log,
        tick,
        agent,
        EventTag::PlanInvalidated,
        DecisionEventPayload::PlanInvalidated(PlanInvalidatedPayload {
            agent,
            goal_key,
            reason,
        }),
    );
}

fn emit_replan_triggered(
    event_log: &mut worldwake_core::EventLog,
    tick: Tick,
    agent: EntityId,
    goal_key: worldwake_core::GoalKey,
    reason: ReplanReason,
) {
    emit_decision_event(
        event_log,
        tick,
        agent,
        EventTag::ReplanTriggered,
        DecisionEventPayload::ReplanTriggered(ReplanTriggeredPayload {
            agent,
            goal_key,
            reason,
        }),
    );
}

fn map_pursuit_invalidation_reason(
    reason: crate::PursuitInvalidationReason,
) -> PursuitInvalidationReasonTag {
    match reason {
        crate::PursuitInvalidationReason::NoProfile => PursuitInvalidationReasonTag::NoProfile,
        crate::PursuitInvalidationReason::NoBelief => PursuitInvalidationReasonTag::NoBelief,
        crate::PursuitInvalidationReason::TargetDead => PursuitInvalidationReasonTag::TargetDead,
        crate::PursuitInvalidationReason::PlaceUnknown => {
            PursuitInvalidationReasonTag::PlaceUnknown
        }
        crate::PursuitInvalidationReason::CoLocated => PursuitInvalidationReasonTag::CoLocated,
        crate::PursuitInvalidationReason::PlaceChanged => {
            PursuitInvalidationReasonTag::PlaceChanged
        }
        crate::PursuitInvalidationReason::ConfidenceDecayed => {
            PursuitInvalidationReasonTag::ConfidenceDecayed
        }
    }
}

fn infer_goal_switch_reason(
    previous_goal: worldwake_core::GoalKey,
    new_goal: worldwake_core::GoalKey,
    ranked_candidates: &[crate::RankedGoal],
) -> GoalSwitchReason {
    let previous = ranked_candidates
        .iter()
        .find(|candidate| candidate.grounded.key == previous_goal);
    let new = ranked_candidates
        .iter()
        .find(|candidate| candidate.grounded.key == new_goal);
    match (previous, new) {
        (Some(previous), Some(new)) if new.priority_class > previous.priority_class => {
            GoalSwitchReason::HigherPriorityGoal
        }
        _ => GoalSwitchReason::SameClassMargin,
    }
}

fn emit_candidate_decision_events(
    event_log: &mut worldwake_core::EventLog,
    tick: Tick,
    agent: EntityId,
    offers: &[crate::candidate_generation::CandidateOfferDiagnostic],
    suppressions: &[crate::candidate_generation::CandidateSuppressionDiagnostic],
) {
    for offer in offers {
        emit_decision_event(
            event_log,
            tick,
            agent,
            EventTag::GoalOffered,
            DecisionEventPayload::GoalOffered(GoalOfferedPayload {
                agent,
                goal_key: offer.opportunity.goal_key,
                emitter: offer.emitter,
                source_evidence: offer.source_evidence.clone(),
            }),
        );
    }
    for suppression in suppressions {
        emit_decision_event(
            event_log,
            tick,
            agent,
            EventTag::GoalSuppressed,
            DecisionEventPayload::GoalSuppressed(GoalSuppressedPayload {
                agent,
                goal_key: suppression.opportunity.goal_key,
                reason: suppression.reason,
            }),
        );
    }
}

impl AutonomousController for AgentTickDriver {
    fn name(&self) -> &'static str {
        "agent_tick_driver"
    }

    fn claims_agent(
        &self,
        _world: &worldwake_core::World,
        _agent: EntityId,
        control_source: ControlSource,
    ) -> bool {
        control_source == ControlSource::Ai
    }

    fn produce_agent_input(
        &mut self,
        ctx: AutonomousControllerContext<'_>,
        agent: EntityId,
        replan_signals: &[&ReplanNeeded],
        committed_actions: &[CommittedAction],
    ) -> Result<(), TickInputError> {
        // Fast path: skip dead agents whose cleanup has already been performed.
        // After the first death tick processes frame/goal/plan clearing and
        // component persistence, subsequent ticks have no work to do.
        if let Some(runtime) = self.runtime_by_agent.get(&agent)
            && runtime.dead_cleanup_done
            && ctx.world.get_component_dead_at(agent).is_some()
        {
            return Ok(());
        }

        // Ensure semantics cache is populated, then split-borrow fields to
        // avoid cloning the entire BTreeMap on every agent tick.
        let _ = self.semantics_table(ctx.action_defs);
        let semantics_table = &self.semantics_cache.as_ref().unwrap().1;
        let tracing = self.trace_sink.is_some();
        let cognitive = ctx
            .world
            .get_component_cognitive_profile(agent)
            .cloned()
            .unwrap_or_else(|| panic!("AI agent {agent} lacks CognitiveProfile"));
        let execution_budget = ctx
            .world
            .get_component_execution_budget(agent)
            .cloned()
            .unwrap_or_else(|| panic!("AI agent {agent} lacks ExecutionBudget"));
        let trace = process_agent(
            &mut AgentTickContext {
                world: ctx.world,
                event_log: ctx.event_log,
                scheduler: ctx.scheduler,
                rng: ctx.rng,
                action_defs: ctx.action_defs,
                action_handlers: ctx.action_handlers,
                recipe_registry: ctx.recipe_registry,
                semantics_table,
                cognitive: &cognitive,
                execution_budget: &execution_budget,
                tick: ctx.tick,
            },
            &mut self.runtime_by_agent,
            agent,
            replan_signals,
            committed_actions,
            tracing,
        )?;
        if let (Some(sink), Some(trace)) = (self.trace_sink.as_mut(), trace) {
            sink.record(trace);
        }
        Ok(())
    }
}

#[allow(clippy::too_many_lines)]
fn process_agent(
    ctx: &mut AgentTickContext<'_>,
    runtime_by_agent: &mut BTreeMap<EntityId, AgentDecisionRuntime>,
    agent: EntityId,
    replan_signals: &[&ReplanNeeded],
    committed_actions: &[CommittedAction],
    tracing: bool,
) -> Result<Option<AgentDecisionTrace>, TickInputError> {
    let action_defs = ctx.action_defs;
    let action_handlers = ctx.action_handlers;
    let recipe_registry = ctx.recipe_registry;
    let semantics_table = ctx.semantics_table;
    let cognitive = ctx.cognitive;
    let execution_budget = ctx.execution_budget;
    let tick = ctx.tick;

    let mut blocked_memory = ctx
        .world
        .get_component_blocker_memory(agent)
        .cloned()
        .unwrap_or_default();
    let original_blocked = blocked_memory.clone();
    let mut discrepancy_memory = ctx
        .world
        .get_component_discrepancy_memory(agent)
        .cloned()
        .unwrap_or_default();
    let original_discrepancy_memory = discrepancy_memory.clone();
    let mut violation_memory = ctx
        .world
        .get_component_violation_memory(agent)
        .cloned()
        .unwrap_or_default();
    let original_violation_memory = violation_memory.clone();
    let mut repair_memory = ctx
        .world
        .get_component_repair_memory(agent)
        .cloned()
        .unwrap_or_default();
    let original_repair_memory = repair_memory.clone();
    let mut learned_opportunity_memory = ctx
        .world
        .get_component_learned_opportunity_memory(agent)
        .cloned()
        .unwrap_or_default();
    let original_learned_opportunity_memory = learned_opportunity_memory.clone();
    let memory_capacity = ctx
        .world
        .get_component_memory_capacity_profile(agent)
        .cloned()
        .unwrap_or_default();
    repair_memory.expire(tick);
    learned_opportunity_memory.expire(tick);
    let utility = ctx
        .world
        .get_component_utility_profile(agent)
        .cloned()
        .unwrap_or_default();
    // Read intention frame from authoritative component.
    let original_frame = ctx.world.get_component_intention_frame(agent).cloned();
    let mut current_frame = original_frame.clone();
    // Read active goal from authoritative component.
    let original_active_goal = ctx.world.get_component_active_goal(agent).copied();
    let mut current_active_goal = original_active_goal;
    // Read facility queue intents from authoritative component.
    let original_facility_intents = ctx
        .world
        .get_component_contention_intents(agent)
        .cloned()
        .unwrap_or_default();
    let mut current_facility_intents = original_facility_intents.clone();
    let runtime = runtime_by_agent.entry(agent).or_default();
    let original_plan_goal = runtime.current_plan.as_ref().map(|plan| plan.goal);
    let active_action = active_action_for_agent(ctx, agent);
    let start_failures = ctx.scheduler.take_action_start_failures_for(agent);
    let mut frame_transitions: Option<Vec<FrameTransitionKind>> =
        if tracing { Some(Vec::new()) } else { None };

    // ── Dead-agent early return ──
    {
        let view = runtime_belief_view(
            agent,
            ctx.world,
            ctx.scheduler,
            action_defs,
            recipe_registry,
        );
        if view.is_dead(agent) || !view.is_alive(agent) {
            if let Some(goal_key) = original_active_goal.map(|goal| goal.goal_key) {
                emit_decision_event(
                    ctx.event_log,
                    tick,
                    agent,
                    EventTag::GoalAbandoned,
                    DecisionEventPayload::GoalAbandoned(GoalAbandonedPayload {
                        agent,
                        goal_key,
                        reason: GoalAbandonReason::FrameCleared {
                            reason: FrameClearReason::Death,
                        },
                    }),
                );
            }
            if current_frame.is_some() {
                runtime.last_frame_clear_reason = Some(FrameClearReason::Death);
                current_frame = None;
                if let Some(ref mut ft) = frame_transitions {
                    ft.push(FrameTransitionKind::Cleared {
                        reason: FrameClearReason::Death,
                        failed_assumption: None,
                    });
                }
            }
            current_active_goal = None;
            current_facility_intents = ContentionIntents::default();
            runtime.current_plan = None;
            runtime.current_step_index = 0;
            runtime.step_in_flight = false;
            runtime.dirty = crate::DirtySet::default();
            runtime.materialization_bindings.clear();
            runtime.dead_cleanup_done = true;
            update_runtime_observation_snapshot(&view, agent, runtime);
            persist_intention_frame(
                ctx.world,
                ctx.event_log,
                agent,
                tick,
                original_frame.as_ref(),
                current_frame.as_ref(),
            )?;
            persist_active_goal(
                ctx.world,
                ctx.event_log,
                agent,
                tick,
                original_active_goal.as_ref(),
                current_active_goal.as_ref(),
            )?;
            persist_facility_queue_intents(
                ctx.world,
                ctx.event_log,
                agent,
                tick,
                &original_facility_intents,
                &current_facility_intents,
            )?;
            return Ok(tracing.then_some(AgentDecisionTrace {
                agent,
                tick,
                outcome: DecisionOutcome::Dead,
            }));
        }
    }

    let reconciliation = reconcile_in_flight_state(
        ctx,
        runtime,
        &mut current_active_goal,
        &mut current_frame,
        &mut current_facility_intents,
        &mut blocked_memory,
        &mut discrepancy_memory,
        active_action.as_ref(),
        agent,
        InFlightReconciliation {
            replan_signals,
            start_failures: &start_failures,
            committed_actions,
        },
    )?;
    if let Some((goal_key, reason)) = reconciliation.plan_invalidation {
        emit_plan_invalidated(ctx.event_log, tick, agent, goal_key, reason);
    }
    if let Some((goal_key, reason)) = reconciliation.replan_trigger.clone() {
        emit_replan_triggered(ctx.event_log, tick, agent, goal_key, reason);
    }
    if let Some(summary) = reconciliation.completed_plan {
        record_repair_memory_from_completed_plan(
            &mut repair_memory,
            &blocked_memory,
            &summary,
            tick,
            cognitive.repair_memory_ticks,
            memory_capacity,
        );
    }

    // Detect progress recorded during reconciliation (advance_completed_step).
    if let Some(ref mut ft) = frame_transitions {
        let new_progress = current_frame
            .as_ref()
            .and_then(|f| f.last_progress_tick)
            .is_some_and(|lpt| {
                lpt == tick
                    && original_frame.as_ref().and_then(|f| f.last_progress_tick) != Some(tick)
            });
        if new_progress {
            ft.push(FrameTransitionKind::Progressed { tick });
        }
    }

    let _ = abandon_expired_facility_queues(
        ctx.world,
        ctx.event_log,
        agent,
        tick,
        cognitive.structural_block_ticks,
    )?;

    // ── Pre-planning assumption evaluation ──
    // Evaluate frame assumptions (except NoCriticalThreat, which needs ranked
    // candidates and is deferred to after ranking).
    {
        let should_eval = current_frame
            .as_ref()
            .is_some_and(|f| !matches!(f.state, worldwake_core::FrameState::Exhausted));
        if should_eval {
            let view = runtime_belief_view(
                agent,
                ctx.world,
                ctx.scheduler,
                action_defs,
                recipe_registry,
            );
            let frame = current_frame.as_mut().unwrap();
            frame.assumptions = populate_assumptions(frame, agent, &view);
            let eval = evaluate_assumptions(&frame.assumptions, &view, agent, None);
            if !matches!(eval, AssumptionEvalResult::Deferred) {
                let pre_state = current_frame.as_ref().unwrap().state;
                current_frame = Some(apply_assumption_result(
                    current_frame.as_ref().unwrap(),
                    &eval,
                    tick,
                    runtime,
                ));
                emit_assumption_transitions(&pre_state, &eval, tick, &mut frame_transitions);
                if matches!(eval, AssumptionEvalResult::CriticalFailure(_)) {
                    let AssumptionEvalResult::CriticalFailure(assumption) = eval else {
                        unreachable!()
                    };
                    // Create blocked intent so the agent doesn't immediately
                    // re-adopt the same goal after assumption failure. The
                    // structural block-ticks TTL preserves the pre-S109
                    // suppression duration of `BlockingFact::AssumptionFailed`
                    // — a failed plan-level assumption is structural, not a
                    // transient drift.
                    record_assumption_failure(
                        current_frame.as_ref().unwrap(),
                        view.effective_place(agent),
                        current_step(runtime)
                            .and_then(|step| step.targets.first().copied())
                            .and_then(authoritative_target),
                        &mut discrepancy_memory,
                        tick,
                        cognitive.structural_block_ticks,
                    );
                    runtime.current_plan = None;
                    runtime.current_step_index = 0;
                    runtime.materialization_bindings.clear();
                    current_facility_intents.intents.clear();
                    runtime.dirty.insert(crate::DirtySet::ASSUMPTION_FAILED);
                    if let Some(goal_key) = original_plan_goal
                        .or(current_active_goal.as_ref().map(|goal| goal.goal_key))
                    {
                        let reason = PlanInvalidationReason::AssumptionFailed { assumption };
                        emit_plan_invalidated(ctx.event_log, tick, agent, goal_key, reason);
                        emit_replan_triggered(
                            ctx.event_log,
                            tick,
                            agent,
                            goal_key,
                            ReplanReason::PlanInvalidated { reason },
                        );
                    }
                }
            }
        }
    }

    // ── Read phase: candidate generation + ranking ──
    let active_goal_key = current_active_goal.as_ref().map(|ag| ag.goal_key);
    let read_result = refresh_runtime_for_read_phase_with_memories(
        ctx.world,
        ctx.scheduler,
        action_defs,
        runtime,
        active_goal_key,
        &mut current_facility_intents,
        &mut blocked_memory,
        &mut discrepancy_memory,
        &mut violation_memory,
        &repair_memory,
        &learned_opportunity_memory,
        agent,
        replan_signals,
        ReadPhaseContext {
            recipe_registry,
            utility: &utility,
            tick,
            travel_horizon: cognitive.snapshot_travel_horizon,
            structural_block_ticks: cognitive.structural_block_ticks,
        },
        tracing,
    );
    let pursuit_invalidation = read_result.pursuit_invalidation;
    if let Some(pursuit_invalidation) = pursuit_invalidation
        && let Some(goal_key) = original_plan_goal.or(active_goal_key)
    {
        let reason = PlanInvalidationReason::PursuitInvalidated {
            reason: map_pursuit_invalidation_reason(pursuit_invalidation),
        };
        emit_plan_invalidated(ctx.event_log, tick, agent, goal_key, reason);
        emit_replan_triggered(
            ctx.event_log,
            tick,
            agent,
            goal_key,
            ReplanReason::PlanInvalidated { reason },
        );
    }
    emit_candidate_decision_events(
        ctx.event_log,
        tick,
        agent,
        &read_result.offered,
        &read_result.suppressed,
    );
    record_learned_opportunities_from_read_phase(
        &runtime_belief_view(
            agent,
            ctx.world,
            ctx.scheduler,
            action_defs,
            recipe_registry,
        ),
        agent,
        active_goal_key,
        &read_result.generated_keys,
        &mut learned_opportunity_memory,
        tick,
        cognitive.learned_opportunity_memory_ticks,
        memory_capacity,
    );
    if !read_result.pending_acquisition_exhaustion_resets.is_empty() {
        apply_acquisition_exhaustion_tracker_resets(
            ctx.world,
            ctx.event_log,
            agent,
            tick,
            &read_result.pending_acquisition_exhaustion_resets,
        )?;
    }
    let ranked_candidates = read_result.ranked;

    // ── Deferred NoCriticalThreat evaluation ──
    // Now that ranked candidates are available, evaluate NoCriticalThreat
    // assumptions that were deferred in the pre-planning stage.
    if let Some(frame) = current_frame.as_ref()
        && !matches!(frame.state, worldwake_core::FrameState::Exhausted)
    {
        let has_no_critical_threat = frame
            .assumptions
            .iter()
            .any(|a| matches!(a, worldwake_core::FrameAssumption::NoCriticalThreat));
        if has_no_critical_threat {
            let deferred_eval = evaluate_assumptions(
                &[worldwake_core::FrameAssumption::NoCriticalThreat],
                &runtime_belief_view(
                    agent,
                    ctx.world,
                    ctx.scheduler,
                    action_defs,
                    recipe_registry,
                ),
                agent,
                Some(&ranked_candidates),
            );
            if matches!(
                deferred_eval,
                AssumptionEvalResult::RecoverableFailure(_) | AssumptionEvalResult::AllPass
            ) {
                let pre_state = frame.state;
                current_frame = Some(apply_assumption_result(
                    frame,
                    &deferred_eval,
                    tick,
                    runtime,
                ));
                emit_assumption_transitions(
                    &pre_state,
                    &deferred_eval,
                    tick,
                    &mut frame_transitions,
                );
            }
        }
    }

    // ── Feasibility annotation and re-sort ──
    let mut ranked_candidates = ranked_candidates;
    {
        let view = runtime_belief_view(
            agent,
            ctx.world,
            ctx.scheduler,
            action_defs,
            recipe_registry,
        );
        for ranked in &mut ranked_candidates {
            ranked.feasibility = crate::feasibility::feasibility_hint(
                &view,
                agent,
                ranked,
                &blocked_memory,
                current_frame.as_ref(),
                tick,
            );
        }
        ranked_candidates.sort_by(crate::ranking::compare_ranked_goals);
    }

    let active_action = active_action_for_agent(ctx, agent);
    let frame_switch_margin = {
        let jc = ctx.world.get_component_intention_frame(agent);
        let view = runtime_belief_view(
            agent,
            ctx.world,
            ctx.scheduler,
            action_defs,
            recipe_registry,
        );
        effective_goal_switch_margin(&view, agent, jc, cognitive)
    };
    let default_switch_margin = cognitive.switch_margin;

    // ── Active-action path: interrupt evaluation ──
    let outcome_trace = if let Some(active_action) = active_action {
        let active_goal_before_interrupt = current_active_goal.as_ref().map(|goal| goal.goal_key);
        let interrupt_decision = handle_active_action_phase(
            ctx,
            runtime,
            &mut current_active_goal,
            &mut current_frame,
            &mut current_facility_intents,
            &mut blocked_memory,
            &mut discrepancy_memory,
            agent,
            &ranked_candidates,
            &active_action,
            default_switch_margin,
            frame_switch_margin,
            tick,
            action_defs,
            action_handlers,
            read_result.decision_context,
        )?;
        if matches!(
            interrupt_decision,
            crate::InterruptDecision::InterruptForReplan { .. }
        ) && let Some(goal_key) = active_goal_before_interrupt
            .or_else(|| original_plan_goal)
            .or_else(|| runtime.current_plan.as_ref().map(|plan| plan.goal))
        {
            emit_replan_triggered(
                ctx.event_log,
                tick,
                agent,
                goal_key,
                ReplanReason::ActionInterrupted {
                    reason: worldwake_core::ActionInterruptReasonTag::Reprioritized,
                },
            );
        }

        tracing.then(|| {
            let action_name = action_defs
                .get(active_action.def_id)
                .map_or_else(|| "unknown".to_owned(), |def| def.name.clone());
            let top_challenger = ranked_candidates.first().map(summarize_ranked_goal);
            let top_challenger_comparison = active_goal_before_interrupt.and_then(|current_goal| {
                let challenger = ranked_candidates.first()?;
                let current = ranked_candidates
                    .iter()
                    .find(|candidate| candidate.grounded.key == current_goal)?;
                crate::ranking::explain_ranked_goal_order(challenger, current)
            });
            DecisionOutcome::ActiveAction {
                action_def_id: active_action.def_id,
                action_name,
                interrupt: InterruptTrace {
                    decision: interrupt_decision,
                    top_challenger,
                    top_challenger_comparison,
                },
                frame_transition: build_frame_transition_trace(&mut frame_transitions),
            }
        })
    } else {
        // ── Planning path ──
        let previous_goal = current_active_goal.as_ref().map(|ag| ag.goal_key);

        // Drain action start failures for this agent from the scheduler.
        let agent_failures: Vec<ActionStartFailureSummary> = start_failures
            .iter()
            .map(|f| ActionStartFailureSummary {
                tick: f.tick,
                def_id: f.def_id,
                request: f.request,
                reason: f.reason.clone(),
            })
            .collect();

        let (
            next_step,
            next_step_valid,
            plan_continued,
            plan_search_trace,
            selection_trace,
            pending_tracker_increments,
        ) = plan_and_validate_next_step_traced(
            ctx.world,
            ctx.event_log,
            ctx.scheduler,
            runtime,
            &mut current_active_goal,
            &mut current_frame,
            &mut current_facility_intents,
            agent,
            &ranked_candidates,
            &blocked_memory,
            default_switch_margin,
            frame_switch_margin,
            utility.side_benefit_weight,
            tick,
            cognitive,
            execution_budget,
            semantics_table,
            action_defs,
            action_handlers,
            tracing,
            previous_goal,
            ctx.recipe_registry,
        );
        if !pending_tracker_increments.is_empty() {
            apply_acquisition_exhaustion_tracker_increments(
                ctx.world,
                ctx.event_log,
                agent,
                tick,
                &pending_tracker_increments,
            )?;
        }

        // ── Execution ──
        let mut execution_trace = if tracing {
            Some(ExecutionTrace {
                enqueued_step: None,
                revalidation_passed: None,
                failure: None,
            })
        } else {
            None
        };

        if let Some(step) = next_step {
            let valid = next_step_valid.expect("validation result must exist for current step");

            if tracing {
                let et = execution_trace.as_mut().unwrap();
                et.revalidation_passed = Some(valid);
                et.enqueued_step = Some(summarize_step(&step, action_defs));
            }

            let active_goal_key = current_active_goal.as_ref().map(|ag| ag.goal_key);
            let exec_result = enqueue_valid_step_or_handle_failure(
                ctx,
                runtime,
                active_goal_key,
                &mut current_frame,
                &mut blocked_memory,
                &mut discrepancy_memory,
                &mut current_facility_intents,
                agent,
                tick,
                &original_blocked,
                &original_discrepancy_memory,
                &original_violation_memory,
                &violation_memory,
                &original_repair_memory,
                &repair_memory,
                &original_learned_opportunity_memory,
                &learned_opportunity_memory,
                &step,
                valid,
            );

            if let Err(ref _e) = exec_result
                && let Some(et) = execution_trace.as_mut()
                && !valid
            {
                et.failure = Some(ExecutionFailureReason::RevalidationFailed);
            }
            exec_result?;
        }

        tracing.then(|| {
            let (patrol_route, affordance_trace) = {
                let view = runtime_belief_view(
                    agent,
                    ctx.world,
                    ctx.scheduler,
                    action_defs,
                    recipe_registry,
                );
                let patrol = patrol_route_snapshot(&view, agent);
                let place = view.effective_place(agent);
                let affordances =
                    worldwake_sim::get_affordances(&view, agent, action_defs, action_handlers);
                let trace = AffordanceTrace {
                    available: affordances
                        .iter()
                        .map(|a| AffordanceSummary {
                            def_id: a.def_id,
                            action_name: action_defs
                                .get(a.def_id)
                                .map_or_else(|| "unknown".to_owned(), |d| d.name.clone()),
                            target_count: a.bound_targets.len(),
                        })
                        .collect(),
                    place,
                };
                (patrol, trace)
            };
            let candidate_trace = CandidateTrace {
                generated: read_result.generated_keys,
                evidence: read_result.candidate_evidence,
                fully_blocked_desires: read_result.fully_blocked_desires,
                places_reachable: read_result.places_reachable,
                places_after_belief_filter: read_result.places_after_belief_filter,
                ranked: ranked_candidates
                    .iter()
                    .map(summarize_ranked_goal)
                    .collect(),
                top_ranked_comparison: ranked_candidates
                    .first()
                    .zip(ranked_candidates.get(1))
                    .and_then(|(winner, runner_up)| {
                        crate::ranking::explain_ranked_goal_order(winner, runner_up)
                    }),
                suppressed: read_result
                    .suppressed
                    .iter()
                    .map(|suppression| suppression.opportunity.goal_key)
                    .collect(),
                zero_motive: read_result.zero_motive,
                omitted_political: read_result.omitted_political,
                omitted_bandit: read_result.omitted_bandit,
                omitted_social: read_result.omitted_social,
                omitted_violation_detection: read_result.omitted_violation_detection,
            };

            let selection = selection_trace.unwrap_or(SelectionTrace {
                selected_opportunity: None,
                selected_plan: None,
                selected_plan_source: None,
                goal_switch: None,
                previous_goal: None,
                plan_replacement: None,
                snapshot_continuation: None,
            });
            let selected_patrol_anchor = selection.selected_opportunity.and_then(|opportunity| {
                matches!(
                    opportunity.goal_key.kind,
                    worldwake_core::GoalKind::Patrol { .. }
                )
                .then_some(opportunity.anchor)
            });

            DecisionOutcome::Planning(Box::new(PlanningPipelineTrace {
                affordances: Some(affordance_trace),
                dirty: runtime.dirty,
                plan_continued,
                candidates: candidate_trace,
                planning: plan_search_trace.unwrap_or(PlanSearchTrace {
                    attempts: Vec::new(),
                    same_goal_trace: None,
                }),
                selection,
                execution: execution_trace.unwrap_or(ExecutionTrace {
                    enqueued_step: None,
                    revalidation_passed: None,
                    failure: None,
                }),
                action_start_failures: agent_failures,
                discrepancy_trace: discrepancy_memory
                    .entries
                    .values()
                    .filter(|entry| entry.expires_tick > tick)
                    .map(|entry| DiscrepancyTrace {
                        discrepancy: entry.discrepancy,
                        blocker_key: entry.blocker_key,
                        expires_tick: entry.expires_tick,
                    })
                    .collect(),
                exhaustion_snapshot: runtime
                    .exhaustion_cache
                    .iter()
                    .map(|(opportunity, entry)| ExhaustionTraceEntry {
                        opportunity: *opportunity,
                        retry_state: entry.retry_state.clone(),
                        consecutive_failures: entry.consecutive_failures,
                        next_retry_tick: entry.next_retry_tick,
                        retry_eligible: entry.is_retry_eligible(tick),
                    })
                    .collect(),
                frame_transition: build_frame_transition_trace(&mut frame_transitions),
                patrol_route,
                selected_patrol_anchor,
                pursuit_invalidation,
            }))
        })
    };

    // ── Per-tick stall increment ──
    // If the frame is Active and no progress was recorded this tick, increment
    // stalled_ticks. Progress resets happen inside advance_completed_step via
    // progress_op_kinds().
    let patience_exhausted = if let Some(ref mut frame) = current_frame {
        if matches!(frame.state, worldwake_core::FrameState::Active)
            && frame.last_progress_tick != Some(tick)
        {
            frame.stalled_ticks = frame
                .stalled_ticks
                .checked_add(1)
                .expect("stalled ticks overflowed");
            frame.stalled_ticks >= frame.patience_limit
        } else {
            false
        }
    } else {
        false
    };

    // ── Patience exhaustion → Blocker + Exhausted state ──
    if patience_exhausted {
        let view = runtime_belief_view(
            agent,
            ctx.world,
            ctx.scheduler,
            action_defs,
            recipe_registry,
        );
        let exhausted = check_patience_exhaustion(
            current_frame.as_ref().unwrap(),
            view.effective_place(agent),
            &mut blocked_memory,
            &mut current_facility_intents,
            runtime,
            tick,
            cognitive.structural_block_ticks,
        );
        if exhausted {
            let frame_ref = current_frame.as_ref().unwrap();
            if let Some(ref mut ft) = frame_transitions {
                ft.push(FrameTransitionKind::Exhausted {
                    stalled_ticks: frame_ref.stalled_ticks,
                    patience_limit: frame_ref.patience_limit,
                    blocked_intent_recorded: true,
                });
            }
            current_frame = Some(IntentionFrame {
                state: worldwake_core::FrameState::Exhausted,
                ..current_frame.take().unwrap()
            });
        }
    }

    // ── Detect frame creation (new frame this tick) ──
    if let Some(ref mut ft) = frame_transitions {
        let was_none = original_frame.is_none();
        let is_some = current_frame.is_some();
        let established_changed = current_frame.as_ref().is_some_and(|f| {
            original_frame
                .as_ref()
                .is_none_or(|orig| orig.established_at != f.established_at)
        });
        if is_some && (was_none || established_changed) {
            let frame = current_frame.as_ref().unwrap();
            ft.push(FrameTransitionKind::Created {
                goal: frame.goal,
                domain_tag: frame.domain.domain_tag(),
                patience_limit: frame.patience_limit,
                assumptions_count: frame.assumptions.len(),
            });
        }
        // Detect frame clearing (had a frame, now gone, not already emitted).
        if original_frame.is_some()
            && current_frame.is_none()
            && !ft
                .iter()
                .any(|t| matches!(t, FrameTransitionKind::Cleared { .. }))
            && let Some(reason) = runtime.last_frame_clear_reason
        {
            ft.push(FrameTransitionKind::Cleared {
                reason,
                failed_assumption: None,
            });
        }
    }

    update_exploration_counter_for_adopted_goal(
        ctx.world,
        ctx.event_log,
        agent,
        current_active_goal.as_ref(),
        tick,
    )?;

    if let Some(original_frame) = original_frame.as_ref()
        && let Some(IntentionFrame {
            goal,
            state:
                worldwake_core::FrameState::Suspended {
                    reason,
                    suspended_at: _,
                },
            ..
        }) = current_frame.as_ref()
        && *goal == original_frame.goal
        && !matches!(
            original_frame.state,
            worldwake_core::FrameState::Suspended {
                reason: existing_reason,
                ..
            } if existing_reason == *reason
        )
    {
        emit_decision_event(
            ctx.event_log,
            tick,
            agent,
            EventTag::GoalSuspended,
            DecisionEventPayload::GoalSuspended(GoalSuspendedPayload {
                agent,
                goal_key: original_frame.goal,
                reason: *reason,
            }),
        );
    }

    if let Some(previous_goal) = original_active_goal.map(|goal| goal.goal_key) {
        let preserved_as_suspended = current_frame.as_ref().is_some_and(|frame| {
            frame.goal == previous_goal
                && matches!(frame.state, worldwake_core::FrameState::Suspended { .. })
        });
        let still_active = current_active_goal
            .as_ref()
            .is_some_and(|goal| goal.goal_key == previous_goal);
        if !preserved_as_suspended && !still_active {
            let abandon_reason = if let Some(new_goal) = current_active_goal.map(|goal| goal.goal_key)
            {
                GoalAbandonReason::GoalSwitched {
                    new_goal,
                    switch_kind: infer_goal_switch_reason(
                        previous_goal,
                        new_goal,
                        &ranked_candidates,
                    ),
                }
            } else if let Some(reason) = runtime.last_frame_clear_reason {
                GoalAbandonReason::FrameCleared { reason }
            } else {
                GoalAbandonReason::FrameCleared {
                    reason: FrameClearReason::LostPlan,
                }
            };
            emit_decision_event(
                ctx.event_log,
                tick,
                agent,
                EventTag::GoalAbandoned,
                DecisionEventPayload::GoalAbandoned(GoalAbandonedPayload {
                    agent,
                    goal_key: previous_goal,
                    reason: abandon_reason,
                }),
            );
        }
    }

    // ── Finalize (runs for both paths) ──
    persist_intention_frame(
        ctx.world,
        ctx.event_log,
        agent,
        tick,
        original_frame.as_ref(),
        current_frame.as_ref(),
    )?;
    persist_active_goal(
        ctx.world,
        ctx.event_log,
        agent,
        tick,
        original_active_goal.as_ref(),
        current_active_goal.as_ref(),
    )?;
    persist_facility_queue_intents(
        ctx.world,
        ctx.event_log,
        agent,
        tick,
        &original_facility_intents,
        &current_facility_intents,
    )?;
    finalize_agent_tick(
        ctx.world,
        ctx.event_log,
        ctx.scheduler,
        action_defs,
        recipe_registry,
        agent,
        tick,
        &original_blocked,
        &blocked_memory,
        &original_discrepancy_memory,
        &discrepancy_memory,
        &original_violation_memory,
        &violation_memory,
        &original_repair_memory,
        &repair_memory,
        &original_learned_opportunity_memory,
        &learned_opportunity_memory,
        runtime,
    )?;

    Ok(outcome_trace.map(|outcome| AgentDecisionTrace {
        agent,
        tick,
        outcome,
    }))
}

fn record_repair_memory_from_completed_plan(
    repair_memory: &mut RepairMemory,
    blocked_memory: &BlockerMemory,
    summary: &CompletedPlanSummary,
    current_tick: Tick,
    ttl_ticks: u32,
    memory_capacity: worldwake_core::MemoryCapacityProfile,
) {
    let alternate_target = match summary.opportunity.anchor {
        OpportunityAnchor::Place(place) | OpportunityAnchor::Entity(place) => place,
        OpportunityAnchor::None => return,
    };
    if !matches!(
        summary.terminal_kind,
        crate::PlanTerminalKind::GoalSatisfied | crate::PlanTerminalKind::CombatCommitment
    ) {
        return;
    }
    let has_prior_alternate_context = blocked_memory.intents.values().any(|blocker| {
        blocker.expires_tick > current_tick
            && blocker.blocker_key.goal_key == summary.goal_key
            && blocker
                .blocker_key
                .target
                .or(blocker.blocker_key.place)
                .is_some_and(|blocked| blocked != alternate_target)
    });
    if !has_prior_alternate_context {
        return;
    }

    let repair_key = RepairKey {
        goal_key: summary.goal_key,
        alternate_target,
    };
    let success_count = repair_memory
        .repairs
        .get(&repair_key)
        .filter(|entry| entry.expires_tick > current_tick)
        .map_or(1, |entry| entry.success_count.saturating_add(1));
    repair_memory.record(RepairEntry {
        repair_key,
        observed_tick: current_tick,
        expires_tick: Tick(current_tick.0 + u64::from(ttl_ticks)),
        success_count,
    });
    repair_memory.enforce_capacity(&memory_capacity);
}

#[allow(clippy::too_many_arguments)]
fn record_learned_opportunities_from_read_phase(
    view: &dyn RuntimeBeliefView,
    agent: EntityId,
    active_goal: Option<worldwake_core::GoalKey>,
    generated_keys: &[worldwake_core::OpportunityKey],
    learned_opportunity_memory: &mut LearnedOpportunityMemory,
    current_tick: Tick,
    ttl_ticks: u32,
    memory_capacity: worldwake_core::MemoryCapacityProfile,
) {
    let Some(active_goal) = active_goal else {
        return;
    };
    let Some(in_transit) = view.in_transit_state(agent) else {
        return;
    };
    for opportunity in generated_keys {
        if opportunity.goal_key == active_goal {
            continue;
        }
        if matches!(opportunity.anchor, OpportunityAnchor::None) {
            continue;
        }
        learned_opportunity_memory.record(OpportunityEntry {
            opportunity: *opportunity,
            observed_tick: current_tick,
            expires_tick: Tick(current_tick.0 + u64::from(ttl_ticks)),
            observed_at: in_transit.destination,
        });
    }
    learned_opportunity_memory.enforce_capacity(&memory_capacity);
}

pub(super) fn update_exploration_counter_for_adopted_goal(
    world: &mut worldwake_core::World,
    event_log: &mut worldwake_core::EventLog,
    agent: EntityId,
    active_goal: Option<&ActiveGoal>,
    tick: Tick,
) -> Result<(), TickInputError> {
    let Some(active_goal) = active_goal.filter(|active_goal| active_goal.adopted_at == tick) else {
        return Ok(());
    };
    let Some(mut profile) = world.get_component_exploration_profile(agent).copied() else {
        return Ok(());
    };

    let mut proactive_commit_tick = None;
    match active_goal.goal_key.kind {
        worldwake_core::GoalKind::ExploreLocation {
            motivating_need, ..
        } => {
            profile.consecutive_exploration_count =
                profile.consecutive_exploration_count.saturating_add(1);
            if motivating_need == worldwake_core::ExplorationMotivation::Proactive {
                proactive_commit_tick = Some(tick);
            }
        }
        _ => {
            profile.consecutive_exploration_count = 0;
        }
    }

    let mut txn = WorldTxn::new(
        world,
        tick,
        CauseRef::SystemTick(tick),
        Some(agent),
        None,
        VisibilitySpec::Hidden,
        WitnessData::default(),
    );
    txn.set_component_exploration_profile(agent, profile)
        .map_err(|error| TickInputError::new(error.to_string()))?;
    if let Some(proactive_commit_tick) = proactive_commit_tick {
        txn.set_component_last_proactive_exploration_tick(
            agent,
            LastProactiveExplorationTick(Some(proactive_commit_tick)),
        )
        .map_err(|error| TickInputError::new(error.to_string()))?;
    }
    let _ = txn.commit(event_log);
    Ok(())
}

fn apply_acquisition_exhaustion_tracker_resets(
    world: &mut worldwake_core::World,
    event_log: &mut worldwake_core::EventLog,
    agent: EntityId,
    tick: Tick,
    needs: &BTreeSet<worldwake_core::HomeostaticNeedId>,
) -> Result<(), TickInputError> {
    let Some(mut tracker) = world
        .get_component_acquisition_exhaustion_tracker(agent)
        .copied()
    else {
        return Ok(());
    };
    let mut changed = false;
    for need in needs {
        if tracker.count(*need) > 0 {
            tracker.reset(*need);
            changed = true;
        }
    }
    if !changed {
        return Ok(());
    }

    let mut txn = WorldTxn::new(
        world,
        tick,
        CauseRef::SystemTick(tick),
        Some(agent),
        None,
        VisibilitySpec::Hidden,
        WitnessData::default(),
    );
    txn.set_component_acquisition_exhaustion_tracker(agent, tracker)
        .map_err(|error| TickInputError::new(error.to_string()))?;
    let _ = txn.commit(event_log);
    Ok(())
}

fn apply_acquisition_exhaustion_tracker_increments(
    world: &mut worldwake_core::World,
    event_log: &mut worldwake_core::EventLog,
    agent: EntityId,
    tick: Tick,
    needs: &BTreeSet<worldwake_core::HomeostaticNeedId>,
) -> Result<(), TickInputError> {
    let Some(mut tracker) = world
        .get_component_acquisition_exhaustion_tracker(agent)
        .copied()
    else {
        return Ok(());
    };

    for need in needs {
        tracker.increment(*need);
    }

    let mut txn = WorldTxn::new(
        world,
        tick,
        CauseRef::SystemTick(tick),
        Some(agent),
        None,
        VisibilitySpec::Hidden,
        WitnessData::default(),
    );
    txn.set_component_acquisition_exhaustion_tracker(agent, tracker)
        .map_err(|error| TickInputError::new(error.to_string()))?;
    let _ = txn.commit(event_log);
    Ok(())
}

/// Convert collected frame transitions into a trace, consuming the option.
/// Returns `None` when tracing is disabled or no transitions were recorded.
fn build_frame_transition_trace(
    transitions: &mut Option<Vec<FrameTransitionKind>>,
) -> Option<FrameTransitionTrace> {
    let ts = transitions.as_mut()?;
    if ts.is_empty() {
        return None;
    }
    Some(FrameTransitionTrace {
        transitions: std::mem::take(ts),
    })
}

/// Emit assumption-evaluation-driven frame transitions (suspend, resume, exhaust).
fn emit_assumption_transitions(
    pre_state: &worldwake_core::FrameState,
    eval: &AssumptionEvalResult,
    tick: Tick,
    frame_transitions: &mut Option<Vec<FrameTransitionKind>>,
) {
    let Some(ref mut ft) = *frame_transitions else {
        return;
    };
    match eval {
        AssumptionEvalResult::RecoverableFailure(reason) => {
            ft.push(FrameTransitionKind::Suspended {
                reason: *reason,
                tick,
            });
        }
        AssumptionEvalResult::CriticalFailure(failed) => {
            ft.push(FrameTransitionKind::Cleared {
                reason: FrameClearReason::AssumptionFailed,
                failed_assumption: Some(*failed),
            });
        }
        AssumptionEvalResult::AllPass => {
            if matches!(pre_state, worldwake_core::FrameState::Suspended { .. }) {
                ft.push(FrameTransitionKind::Resumed { tick });
            }
        }
        AssumptionEvalResult::Deferred => {}
    }
}

#[cfg(test)]
mod tests;
