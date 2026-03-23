mod active_action;
mod candidates;
mod execution;
mod journey;
mod observation;
mod planning;
pub use journey::{JourneyDebugSnapshot, JourneySwitchMarginSource};
use journey::{handle_recoverable_travel_step_blockage, update_journey_for_adopted_plan};
use active_action::{
    active_action_for_agent, advance_completed_step, effective_goal_switch_margin,
    goal_switch_margin_details, handle_active_action_phase, handle_current_step_failure,
};
use observation::{
    reconcile_in_flight_state, refresh_runtime_for_read_phase,
    update_runtime_observation_snapshot, InFlightReconciliation, ReadPhaseContext,
};
use execution::{
    apply_step_materialization_bindings, committed_action_for_step, current_step,
    enqueue_valid_step_or_handle_failure, finalize_agent_tick, persist_blocked_memory,
    persist_journey_commitment, plan_finished,
};
use candidates::abandon_expired_facility_queues;
use planning::{
    build_candidate_plans, plan_and_validate_next_step_traced, plans_as_options,
    summarize_ranked_goal, summarize_step,
};

use crate::decision_trace::{
    ActionStartFailureSummary, AgentDecisionTrace, CandidateTrace, DecisionOutcome,
    DecisionTraceSink, ExecutionFailureReason, ExecutionTrace, InterruptTrace, PlanSearchTrace,
    PlanningPipelineTrace, SelectionTrace,
};
use crate::{
    build_semantics_table, journey_runtime_snapshot, AgentDecisionRuntime, JourneyClearReason,
    PlannerOpSemantics, PlanningBudget,
};
use std::collections::BTreeMap;
use worldwake_core::{ActionDefId, ControlSource, EntityId, Tick};
use worldwake_sim::{
    ActionHandlerRegistry, AutonomousController, AutonomousControllerContext, CommittedAction,
    PerAgentBeliefRuntime, PerAgentBeliefView, RecipeRegistry, ReplanNeeded, RuntimeBeliefView,
    Scheduler, TickInputError,
};

pub struct AgentTickDriver {
    runtime_by_agent: BTreeMap<EntityId, AgentDecisionRuntime>,
    budget: PlanningBudget,
    semantics_cache: Option<(usize, BTreeMap<ActionDefId, PlannerOpSemantics>)>,
    /// Optional trace collector. When `Some`, decision traces are recorded.
    trace_sink: Option<DecisionTraceSink>,
}

impl AgentTickDriver {
    #[must_use]
    pub fn new(budget: PlanningBudget) -> Self {
        Self {
            runtime_by_agent: BTreeMap::new(),
            budget,
            semantics_cache: None,
            trace_sink: None,
        }
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
    pub fn journey_snapshot(
        &self,
        world: &worldwake_core::World,
        agent: EntityId,
    ) -> Option<JourneyDebugSnapshot> {
        let runtime = self.runtime_by_agent.get(&agent)?;
        let jc = world.get_component_journey_commitment(agent);
        let view = PerAgentBeliefView::from_world(agent, world);
        let (effective_switch_margin, switch_margin_source) =
            goal_switch_margin_details(&view, agent, jc, &self.budget);
        Some(JourneyDebugSnapshot {
            runtime: journey_runtime_snapshot(jc, runtime),
            effective_switch_margin,
            switch_margin_source,
        })
    }
}

pub(super) fn runtime_belief_view<'a>(
    agent: EntityId,
    world: &'a worldwake_core::World,
    scheduler: &'a Scheduler,
    action_defs: &'a worldwake_sim::ActionDefRegistry,
) -> PerAgentBeliefView<'a> {
    PerAgentBeliefView::with_runtime_from_world_at_tick(
        agent,
        scheduler.current_tick(),
        world,
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
    pub(super) budget: &'a PlanningBudget,
    pub(super) tick: Tick,
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
        let semantics_table = self.semantics_table(ctx.action_defs).clone();
        let tracing = self.trace_sink.is_some();
        let trace = process_agent(
            &mut AgentTickContext {
                world: ctx.world,
                event_log: ctx.event_log,
                scheduler: ctx.scheduler,
                rng: ctx.rng,
                action_defs: ctx.action_defs,
                action_handlers: ctx.action_handlers,
                recipe_registry: ctx.recipe_registry,
                semantics_table: &semantics_table,
                budget: &self.budget,
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
    let budget = ctx.budget;
    let tick = ctx.tick;

    let mut blocked_memory = ctx
        .world
        .get_component_blocked_intent_memory(agent)
        .cloned()
        .unwrap_or_default();
    let original_blocked = blocked_memory.clone();
    let utility = ctx
        .world
        .get_component_utility_profile(agent)
        .cloned()
        .unwrap_or_default();
    // Read journey commitment from authoritative component.
    let original_jc = ctx.world.get_component_journey_commitment(agent).copied();
    let mut current_jc = original_jc;
    let runtime = runtime_by_agent.entry(agent).or_default();
    let active_action = active_action_for_agent(ctx, agent);
    let start_failures = ctx.scheduler.take_action_start_failures_for(agent);

    // ── Dead-agent early return ──
    {
        let view = runtime_belief_view(agent, ctx.world, ctx.scheduler, action_defs);
        if view.is_dead(agent) || !view.is_alive(agent) {
            if current_jc.is_some() {
                runtime.last_journey_clear_reason = Some(JourneyClearReason::Death);
                current_jc = None;
            }
            runtime.current_goal = None;
            runtime.current_plan = None;
            runtime.current_step_index = 0;
            runtime.step_in_flight = false;
            runtime.dirty = false;
            runtime.materialization_bindings.clear();
            update_runtime_observation_snapshot(&view, agent, runtime);
            persist_journey_commitment(
                ctx.world,
                ctx.event_log,
                agent,
                tick,
                original_jc.as_ref(),
                current_jc.as_ref(),
            )?;
            return Ok(tracing.then_some(AgentDecisionTrace {
                agent,
                tick,
                outcome: DecisionOutcome::Dead,
            }));
        }
    }

    reconcile_in_flight_state(
        ctx,
        runtime,
        &mut current_jc,
        &mut blocked_memory,
        active_action.as_ref(),
        agent,
        InFlightReconciliation {
            replan_signals,
            start_failures: &start_failures,
            committed_actions,
        },
    )?;

    let _ = abandon_expired_facility_queues(ctx.world, ctx.event_log, agent, tick)?;

    // ── Read phase: candidate generation + ranking ──
    let read_result = refresh_runtime_for_read_phase(
        ctx.world,
        ctx.scheduler,
        action_defs,
        runtime,
        &mut blocked_memory,
        agent,
        replan_signals,
        ReadPhaseContext {
            recipe_registry,
            utility: &utility,
            tick,
            travel_horizon: budget.snapshot_travel_horizon,
            structural_block_ticks: budget.structural_block_ticks,
        },
        tracing,
    );
    let ranked_candidates = read_result.ranked;
    let active_action = active_action_for_agent(ctx, agent);
    let journey_switch_margin = {
        let jc = ctx.world.get_component_journey_commitment(agent);
        let view = runtime_belief_view(agent, ctx.world, ctx.scheduler, action_defs);
        effective_goal_switch_margin(&view, agent, jc, budget)
    };
    let default_switch_margin = budget.switch_margin_permille;

    // ── Active-action path: interrupt evaluation ──
    let outcome_trace = if let Some(active_action) = active_action {
        let interrupt_decision = handle_active_action_phase(
            ctx,
            runtime,
            &mut current_jc,
            &mut blocked_memory,
            agent,
            &ranked_candidates,
            &active_action,
            default_switch_margin,
            journey_switch_margin,
            tick,
            action_defs,
            action_handlers,
            read_result.decision_context,
        )?;

        tracing.then(|| {
            let action_name = action_defs
                .get(active_action.def_id)
                .map_or_else(|| "unknown".to_owned(), |def| def.name.clone());
            let top_challenger = ranked_candidates.first().map(summarize_ranked_goal);
            DecisionOutcome::ActiveAction {
                action_def_id: active_action.def_id,
                action_name,
                interrupt: InterruptTrace {
                    decision: interrupt_decision,
                    top_challenger,
                },
            }
        })
    } else {
        // ── Planning path ──
        let previous_goal = runtime.current_goal;

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

        let (next_step, next_step_valid, plan_continued, plan_search_trace, selection_trace) =
            plan_and_validate_next_step_traced(
                ctx.world,
                ctx.scheduler,
                runtime,
                &mut current_jc,
                agent,
                &ranked_candidates,
                &blocked_memory,
                default_switch_margin,
                journey_switch_margin,
                tick,
                budget,
                semantics_table,
                action_defs,
                action_handlers,
                tracing,
                previous_goal,
                &read_result.dirty_reasons,
                ctx.recipe_registry,
            );

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

            let exec_result = enqueue_valid_step_or_handle_failure(
                ctx,
                runtime,
                &mut current_jc,
                &mut blocked_memory,
                agent,
                tick,
                &original_blocked,
                &step,
                valid,
            );

            if let Err(ref _e) = exec_result {
                if let Some(et) = execution_trace.as_mut() {
                    if !valid {
                        et.failure = Some(ExecutionFailureReason::RevalidationFailed);
                    }
                }
            }
            exec_result?;
        }

        tracing.then(|| {
            let candidate_trace = CandidateTrace {
                generated: read_result.generated_keys,
                evidence: read_result.candidate_evidence,
                ranked: ranked_candidates
                    .iter()
                    .map(summarize_ranked_goal)
                    .collect(),
                suppressed: read_result.suppressed,
                zero_motive: read_result.zero_motive,
                omitted_political: read_result.omitted_political,
                omitted_social: read_result.omitted_social,
            };

            DecisionOutcome::Planning(Box::new(PlanningPipelineTrace {
                dirty_reasons: read_result.dirty_reasons,
                plan_continued,
                candidates: candidate_trace,
                planning: plan_search_trace.unwrap_or(PlanSearchTrace {
                    attempts: Vec::new(),
                }),
                selection: selection_trace.unwrap_or(SelectionTrace {
                    selected: None,
                    selected_plan: None,
                    selected_plan_source: None,
                    goal_switch: None,
                    previous_goal: None,
                    plan_replacement: None,
                }),
                execution: execution_trace.unwrap_or(ExecutionTrace {
                    enqueued_step: None,
                    revalidation_passed: None,
                    failure: None,
                }),
                action_start_failures: agent_failures,
            }))
        })
    };

    // ── Finalize (runs for both paths) ──
    persist_journey_commitment(
        ctx.world,
        ctx.event_log,
        agent,
        tick,
        original_jc.as_ref(),
        current_jc.as_ref(),
    )?;
    finalize_agent_tick(
        ctx.world,
        ctx.event_log,
        ctx.scheduler,
        action_defs,
        agent,
        tick,
        &original_blocked,
        &blocked_memory,
        runtime,
    )?;

    Ok(outcome_trace.map(|outcome| AgentDecisionTrace {
        agent,
        tick,
        outcome,
    }))
}


#[cfg(test)]
mod tests;
