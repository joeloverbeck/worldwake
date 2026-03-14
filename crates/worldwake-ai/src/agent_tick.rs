use crate::candidate_generation::generate_candidates_with_travel_horizon;
use crate::{
    authoritative_target, build_planning_snapshot_with_blocked_facility_uses,
    build_semantics_table, clear_resolved_blockers, evaluate_interrupt, handle_plan_failure,
    rank_candidates, resolve_planning_targets_with, revalidate_next_step, search_plan,
    select_best_plan, AgentDecisionRuntime, GoalKindPlannerExt, InterruptDecision,
    JourneyClearReason, JourneyCommitmentState, JourneyRuntimeSnapshot, PlanFailureContext,
    PlanTerminalKind, PlannedPlan, PlannedStep, PlannerOpSemantics, PlanningBudget,
    QueuedFacilityIntent, RankedGoal,
};
use std::collections::BTreeMap;
use worldwake_core::{
    ActionDefId, BlockedIntent, BlockedIntentMemory, BlockingFact, CauseRef, CommodityKind,
    ControlSource, EntityId, Permille, Quantity, Tick, UniqueItemKind, VisibilitySpec, WitnessData,
    WorldTxn,
};
use worldwake_sim::{
    ActionHandlerRegistry, AutonomousController, AutonomousControllerContext, CommitOutcome,
    CommittedAction, InputKind, PerAgentBeliefRuntime, PerAgentBeliefView, RecipeRegistry,
    ReplanNeeded, RuntimeBeliefView, Scheduler, SchedulerActionRuntime, TickInputError,
};

pub struct AgentTickDriver {
    runtime_by_agent: BTreeMap<EntityId, AgentDecisionRuntime>,
    budget: PlanningBudget,
    semantics_cache: Option<(usize, BTreeMap<ActionDefId, PlannerOpSemantics>)>,
}

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

impl AgentTickDriver {
    #[must_use]
    pub fn new(budget: PlanningBudget) -> Self {
        Self {
            runtime_by_agent: BTreeMap::new(),
            budget,
            semantics_cache: None,
        }
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
        let view = PerAgentBeliefView::from_world(agent, world);
        let (effective_switch_margin, switch_margin_source) =
            goal_switch_margin_details(&view, agent, runtime, &self.budget);
        Some(JourneyDebugSnapshot {
            runtime: runtime.journey_runtime_snapshot(),
            effective_switch_margin,
            switch_margin_source,
        })
    }
}

fn runtime_belief_view<'a>(
    agent: EntityId,
    world: &'a worldwake_core::World,
    scheduler: &'a Scheduler,
    action_defs: &'a worldwake_sim::ActionDefRegistry,
) -> PerAgentBeliefView<'a> {
    PerAgentBeliefView::with_runtime_from_world(
        agent,
        world,
        PerAgentBeliefRuntime::new(scheduler.active_actions(), action_defs),
    )
}

struct AgentTickContext<'a> {
    world: &'a mut worldwake_core::World,
    event_log: &'a mut worldwake_core::EventLog,
    scheduler: &'a mut Scheduler,
    rng: &'a mut worldwake_sim::DeterministicRng,
    action_defs: &'a worldwake_sim::ActionDefRegistry,
    action_handlers: &'a ActionHandlerRegistry,
    recipe_registry: &'a RecipeRegistry,
    semantics_table: &'a BTreeMap<ActionDefId, PlannerOpSemantics>,
    budget: &'a PlanningBudget,
    tick: Tick,
}

#[derive(Clone, Copy)]
struct ReadPhaseContext<'a> {
    recipe_registry: &'a RecipeRegistry,
    utility: &'a worldwake_core::UtilityProfile,
    tick: Tick,
    travel_horizon: u8,
    structural_block_ticks: u32,
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
        process_agent(
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
        )
    }
}

#[allow(clippy::too_many_lines)]
fn process_agent(
    ctx: &mut AgentTickContext<'_>,
    runtime_by_agent: &mut BTreeMap<EntityId, AgentDecisionRuntime>,
    agent: EntityId,
    replan_signals: &[&ReplanNeeded],
    committed_actions: &[CommittedAction],
) -> Result<(), TickInputError> {
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
    let runtime = runtime_by_agent.entry(agent).or_default();
    let active_action = active_action_for_agent(ctx, agent);

    {
        let view = runtime_belief_view(agent, ctx.world, ctx.scheduler, action_defs);
        if view.is_dead(agent) || !view.is_alive(agent) {
            runtime.clear_journey_commitment_with_reason(JourneyClearReason::Death);
            runtime.current_goal = None;
            runtime.current_plan = None;
            runtime.current_step_index = 0;
            runtime.step_in_flight = false;
            runtime.dirty = false;
            runtime.materialization_bindings.clear();
            update_runtime_observation_snapshot(&view, agent, runtime);
            return Ok(());
        }
    }

    reconcile_in_flight_state(
        ctx,
        runtime,
        &mut blocked_memory,
        active_action.as_ref(),
        agent,
        replan_signals,
        committed_actions,
    )?;

    let _ = abandon_expired_facility_queues(ctx.world, ctx.event_log, agent, tick)?;

    let ranked_candidates = refresh_runtime_for_read_phase(
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
    );
    let active_action = active_action_for_agent(ctx, agent);
    let journey_switch_margin = {
        let view = runtime_belief_view(agent, ctx.world, ctx.scheduler, action_defs);
        effective_goal_switch_margin(&view, agent, runtime, budget)
    };
    let default_switch_margin = budget.switch_margin_permille;

    if let Some(active_action) = active_action {
        return handle_active_action_phase(
            ctx,
            runtime,
            &mut blocked_memory,
            &original_blocked,
            agent,
            &ranked_candidates,
            &active_action,
            default_switch_margin,
            journey_switch_margin,
            tick,
            action_defs,
            action_handlers,
        );
    }

    let (next_step, next_step_valid) = plan_and_validate_next_step(
        ctx.world,
        ctx.scheduler,
        runtime,
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
    );

    if let Some(step) = next_step {
        let valid = next_step_valid.expect("validation result must exist for current step");
        enqueue_valid_step_or_handle_failure(
            ctx,
            runtime,
            &mut blocked_memory,
            agent,
            tick,
            &original_blocked,
            &step,
            valid,
        )?;
    }

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
    )
}

#[allow(clippy::too_many_arguments)]
fn enqueue_valid_step_or_handle_failure(
    ctx: &mut AgentTickContext<'_>,
    runtime: &mut AgentDecisionRuntime,
    blocked_memory: &mut BlockedIntentMemory,
    agent: EntityId,
    tick: Tick,
    original_blocked: &BlockedIntentMemory,
    step: &PlannedStep,
    valid: bool,
) -> Result<(), TickInputError> {
    if !valid {
        let view = runtime_belief_view(agent, ctx.world, ctx.scheduler, ctx.action_defs);
        if handle_recoverable_travel_step_blockage(
            &view,
            runtime,
            blocked_memory,
            agent,
            step,
            tick,
            ctx.budget,
        ) {
            return Ok(());
        }
        return handle_current_step_failure(ctx, runtime, blocked_memory, agent, step, None);
    }

    let Some(targets) = resolve_step_targets(runtime, step) else {
        let view = runtime_belief_view(agent, ctx.world, ctx.scheduler, ctx.action_defs);
        if handle_recoverable_travel_step_blockage(
            &view,
            runtime,
            blocked_memory,
            agent,
            step,
            tick,
            ctx.budget,
        ) {
            return finalize_agent_tick(
                ctx.world,
                ctx.event_log,
                ctx.scheduler,
                ctx.action_defs,
                agent,
                tick,
                original_blocked,
                blocked_memory,
                runtime,
            );
        }
        handle_current_step_failure(ctx, runtime, blocked_memory, agent, step, None)?;
        return finalize_agent_tick(
            ctx.world,
            ctx.event_log,
            ctx.scheduler,
            ctx.action_defs,
            agent,
            tick,
            original_blocked,
            blocked_memory,
            runtime,
        );
    };

    let _ = ctx.scheduler.input_queue_mut().enqueue(
        tick,
        InputKind::RequestAction {
            actor: agent,
            def_id: step.def_id,
            targets,
            payload_override: step.payload_override.clone(),
            mode: worldwake_sim::ActionRequestMode::BestEffort,
        },
    );
    runtime.step_in_flight = true;
    Ok(())
}

fn active_action_for_agent(
    ctx: &AgentTickContext<'_>,
    agent: EntityId,
) -> Option<worldwake_sim::ActionInstance> {
    ctx.scheduler
        .active_actions()
        .values()
        .find(|instance| instance.actor == agent)
        .cloned()
}

fn abandon_expired_facility_queues(
    world: &mut worldwake_core::World,
    event_log: &mut worldwake_core::EventLog,
    agent: EntityId,
    tick: Tick,
) -> Result<bool, TickInputError> {
    let limit = {
        let view = PerAgentBeliefView::from_world(agent, world);
        let Some(limit) = view.facility_queue_patience_ticks(agent) else {
            return Ok(false);
        };
        limit
    };

    abandon_expired_facility_queues_with_limit(world, event_log, agent, tick, limit)
}

fn abandon_expired_facility_queues_with_limit(
    world: &mut worldwake_core::World,
    event_log: &mut worldwake_core::EventLog,
    agent: EntityId,
    tick: Tick,
    limit: std::num::NonZeroU32,
) -> Result<bool, TickInputError> {
    let expired_facilities = {
        let view = PerAgentBeliefView::from_world(agent, world);
        let Some(place) = view.effective_place(agent) else {
            return Ok(false);
        };

        view.entities_at(place)
            .into_iter()
            .filter(|facility| view.has_exclusive_facility_policy(*facility))
            .filter(|facility| {
                view.facility_grant(*facility)
                    .is_none_or(|grant| grant.actor != agent)
            })
            .filter(|facility| {
                view.facility_queue_join_tick(*facility, agent)
                    .is_some_and(|queued_at| tick >= queued_at + u64::from(limit.get()))
            })
            .collect::<Vec<_>>()
    };

    let mut changed = false;
    for facility in expired_facilities {
        let Some(mut queue) = world.get_component_facility_use_queue(facility).cloned() else {
            continue;
        };
        if !queue.remove_actor(agent) {
            continue;
        }

        let mut txn = WorldTxn::new(
            world,
            tick,
            CauseRef::SystemTick(tick),
            None,
            world.effective_place(facility),
            VisibilitySpec::SamePlace,
            WitnessData::default(),
        );
        txn.set_component_facility_use_queue(facility, queue)
            .map_err(|error| TickInputError::new(error.to_string()))?;
        let _ = txn.commit(event_log);
        changed = true;
    }

    Ok(changed)
}

#[allow(clippy::too_many_arguments)]
fn refresh_runtime_for_read_phase(
    world: &worldwake_core::World,
    scheduler: &Scheduler,
    action_defs: &worldwake_sim::ActionDefRegistry,
    runtime: &mut AgentDecisionRuntime,
    blocked_memory: &mut BlockedIntentMemory,
    agent: EntityId,
    replan_signals: &[&ReplanNeeded],
    phase: ReadPhaseContext<'_>,
) -> Vec<RankedGoal> {
    // One authoritative read view covers blocker cleanup, snapshot dirtiness, and ranking.
    let view = runtime_belief_view(agent, world, scheduler, action_defs);
    let before = blocked_memory.clone();
    let queue_transition_changed =
        handle_facility_queue_transitions(&view, runtime, blocked_memory, agent, phase.tick, phase);
    clear_resolved_blockers(&view, agent, blocked_memory, phase.tick);
    let blocked_changed_from_cleanup = *blocked_memory != before;
    let snapshot_changed =
        observation_snapshot_changed(&view, agent, runtime, phase.recipe_registry);
    let queue_patience_exhausted = facility_queue_patience_exhausted(&view, agent, phase.tick);

    runtime.dirty = runtime.dirty
        || runtime.current_plan.is_none()
        || plan_finished(runtime)
        || !replan_signals.is_empty()
        || queue_transition_changed
        || blocked_changed_from_cleanup
        || snapshot_changed
        || queue_patience_exhausted;

    let candidates = generate_candidates_with_travel_horizon(
        &view,
        agent,
        blocked_memory,
        phase.recipe_registry,
        phase.tick,
        phase.travel_horizon,
    );
    rank_candidates(
        &candidates,
        &view,
        agent,
        phase.utility,
        phase.recipe_registry,
    )
}

fn handle_facility_queue_transitions(
    view: &dyn RuntimeBeliefView,
    runtime: &mut AgentDecisionRuntime,
    blocked_memory: &mut BlockedIntentMemory,
    agent: EntityId,
    tick: Tick,
    phase: ReadPhaseContext<'_>,
) -> bool {
    let previous_place = runtime.last_effective_place;
    let current_place = view.effective_place(agent);
    let current_signature = facility_access_signature(view, agent);
    let current_by_facility = current_signature
        .iter()
        .copied()
        .map(|(facility, queued, grant)| (facility, (queued, grant)))
        .collect::<BTreeMap<_, _>>();
    let mut changed = false;

    for (facility, was_queued, previous_grant) in runtime.last_facility_access_signature.clone() {
        let current = current_by_facility.get(&facility).copied();
        let now_queued = current.is_some_and(|(queued, _)| queued);
        let now_granted = current.and_then(|(_, grant)| grant);

        if was_queued && !now_queued && now_granted.is_none() {
            if previous_place == current_place {
                if let Some(intent) = runtime.queued_facility_intents.remove(&facility) {
                    blocked_memory.record(BlockedIntent {
                        goal_key: intent.goal_key,
                        blocking_fact: BlockingFact::ExclusiveFacilityUnavailable,
                        related_entity: Some(facility),
                        related_place: current_place,
                        related_action: Some(intent.intended_action),
                        observed_tick: tick,
                        expires_tick: tick + u64::from(phase.structural_block_ticks),
                    });
                    changed = true;
                }
            } else if runtime.queued_facility_intents.remove(&facility).is_some() {
                changed = true;
            }
        }

        if previous_grant.is_some() && now_granted.is_none() {
            changed |= runtime.queued_facility_intents.remove(&facility).is_some();
        }
    }

    changed
}

#[allow(clippy::too_many_arguments)]
fn handle_active_action_phase(
    ctx: &mut AgentTickContext<'_>,
    runtime: &mut AgentDecisionRuntime,
    blocked_memory: &mut BlockedIntentMemory,
    original_blocked: &BlockedIntentMemory,
    agent: EntityId,
    ranked_candidates: &[RankedGoal],
    active_action: &worldwake_sim::ActionInstance,
    default_switch_margin: Permille,
    journey_switch_margin: Permille,
    tick: Tick,
    action_defs: &worldwake_sim::ActionDefRegistry,
    action_handlers: &ActionHandlerRegistry,
) -> Result<(), TickInputError> {
    let interruptibility = action_defs
        .get(active_action.def_id)
        .map_or(worldwake_sim::Interruptibility::NonInterruptible, |def| {
            def.interruptibility
        });
    let plan_valid = runtime
        .current_plan
        .as_ref()
        .is_some_and(|plan| runtime.current_step_index < plan.steps.len());
    let planned_candidates = runtime.has_journey_commitment().then(|| {
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
        )
    });
    if let InterruptDecision::InterruptForReplan { trigger: _ } = evaluate_interrupt(
        runtime,
        interruptibility,
        ranked_candidates,
        planned_candidates.as_deref(),
        plan_valid,
        default_switch_margin,
        journey_switch_margin,
    ) {
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
        reconcile_in_flight_state(ctx, runtime, blocked_memory, None, agent, &[&replan], &[])?;
    }

    finalize_agent_tick(
        ctx.world,
        ctx.event_log,
        ctx.scheduler,
        action_defs,
        agent,
        tick,
        original_blocked,
        blocked_memory,
        runtime,
    )
}

fn effective_goal_switch_margin(
    view: &dyn RuntimeBeliefView,
    agent: EntityId,
    runtime: &AgentDecisionRuntime,
    budget: &PlanningBudget,
) -> Permille {
    goal_switch_margin_details(view, agent, runtime, budget).0
}

fn goal_switch_margin_details(
    view: &dyn RuntimeBeliefView,
    agent: EntityId,
    runtime: &AgentDecisionRuntime,
    budget: &PlanningBudget,
) -> (Permille, JourneySwitchMarginSource) {
    if runtime.has_journey_commitment() {
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

#[allow(clippy::too_many_arguments)]
fn build_candidate_plans(
    world: &worldwake_core::World,
    scheduler: &Scheduler,
    agent: EntityId,
    ranked_candidates: &[RankedGoal],
    blocked_memory: &BlockedIntentMemory,
    current_tick: Tick,
    budget: &PlanningBudget,
    semantics_table: &BTreeMap<ActionDefId, PlannerOpSemantics>,
    action_defs: &worldwake_sim::ActionDefRegistry,
    action_handlers: &ActionHandlerRegistry,
) -> Vec<(crate::GoalKey, Option<PlannedPlan>)> {
    let view = runtime_belief_view(agent, world, scheduler, action_defs);
    ranked_candidates
        .iter()
        .take(usize::from(budget.max_candidates_to_plan))
        .map(|ranked| {
            let snapshot = build_planning_snapshot_with_blocked_facility_uses(
                &view,
                agent,
                &ranked.grounded.evidence_entities,
                &ranked.grounded.evidence_places,
                budget.snapshot_travel_horizon,
                blocked_memory,
                current_tick,
            );
            let plan = search_plan(
                &snapshot,
                &ranked.grounded,
                semantics_table,
                action_defs,
                action_handlers,
                budget,
            );
            (ranked.grounded.key, plan)
        })
        .collect()
}

#[allow(clippy::too_many_arguments)]
fn plan_and_validate_next_step(
    world: &worldwake_core::World,
    scheduler: &Scheduler,
    runtime: &mut AgentDecisionRuntime,
    agent: EntityId,
    ranked_candidates: &[RankedGoal],
    blocked_memory: &BlockedIntentMemory,
    default_switch_margin: Permille,
    journey_switch_margin: Permille,
    tick: Tick,
    budget: &PlanningBudget,
    semantics_table: &BTreeMap<ActionDefId, PlannerOpSemantics>,
    action_defs: &worldwake_sim::ActionDefRegistry,
    action_handlers: &ActionHandlerRegistry,
) -> (Option<PlannedStep>, Option<bool>) {
    // A second read view covers plan selection and step validation after the active-action fork.
    let view = runtime_belief_view(agent, world, scheduler, action_defs);
    if runtime.dirty {
        let plans = build_candidate_plans(
            world,
            scheduler,
            agent,
            ranked_candidates,
            blocked_memory,
            tick,
            budget,
            semantics_table,
            action_defs,
            action_handlers,
        );

        if let Some(selected_plan) = select_best_plan(
            ranked_candidates,
            &plans,
            runtime,
            default_switch_margin,
            journey_switch_margin,
        ) {
            runtime.materialization_bindings.clear();
            runtime.current_goal = Some(selected_plan.goal);
            update_journey_fields_for_adopted_plan(runtime, &selected_plan, tick);
            runtime.current_plan = Some(selected_plan);
            runtime.current_step_index = 0;
            runtime.step_in_flight = false;
            runtime.last_priority_class = ranked_candidates
                .iter()
                .find(|candidate| Some(candidate.grounded.key) == runtime.current_goal)
                .map(|candidate| candidate.priority_class);
        } else {
            runtime.clear_journey_commitment_with_reason(JourneyClearReason::LostTravelPlan);
            runtime.materialization_bindings.clear();
            runtime.current_goal = None;
            runtime.current_plan = None;
            runtime.current_step_index = 0;
            runtime.step_in_flight = false;
            runtime.last_priority_class = ranked_candidates
                .first()
                .map(|candidate| candidate.priority_class);
        }
        runtime.dirty = false;
    }

    let next_step = current_step(runtime).cloned();
    let next_step_valid = next_step.as_ref().map(|step| {
        revalidate_next_step(
            &view,
            agent,
            step,
            &runtime.materialization_bindings,
            action_defs,
            action_handlers,
        )
    });
    (next_step, next_step_valid)
}

#[allow(clippy::too_many_arguments)]
fn finalize_agent_tick(
    world: &mut worldwake_core::World,
    event_log: &mut worldwake_core::EventLog,
    scheduler: &Scheduler,
    action_defs: &worldwake_sim::ActionDefRegistry,
    agent: EntityId,
    tick: Tick,
    original_blocked: &BlockedIntentMemory,
    blocked_memory: &BlockedIntentMemory,
    runtime: &mut AgentDecisionRuntime,
) -> Result<(), TickInputError> {
    persist_blocked_memory(
        world,
        event_log,
        agent,
        tick,
        original_blocked,
        blocked_memory,
    )?;
    {
        // Snapshot the post-mutation world state before ending the tick.
        let view = runtime_belief_view(agent, world, scheduler, action_defs);
        update_runtime_observation_snapshot(&view, agent, runtime);
    }
    Ok(())
}

fn reconcile_in_flight_state(
    ctx: &mut AgentTickContext<'_>,
    runtime: &mut AgentDecisionRuntime,
    blocked_memory: &mut BlockedIntentMemory,
    active_action: Option<&worldwake_sim::ActionInstance>,
    agent: EntityId,
    replan_signals: &[&ReplanNeeded],
    committed_actions: &[CommittedAction],
) -> Result<(), TickInputError> {
    if !runtime.step_in_flight {
        return Ok(());
    }
    if active_action.is_some() {
        return Ok(());
    }

    let failed_signal = replan_signals.first().copied();
    let Some(step) = current_step(runtime).cloned() else {
        runtime.step_in_flight = false;
        return Ok(());
    };

    if let Some(signal) = failed_signal {
        let _ = ctx.action_defs.get(signal.failed_action_def);
        handle_current_step_failure(ctx, runtime, blocked_memory, agent, &step, Some(signal))?;
        return Ok(());
    }

    let Some(committed_action) = committed_action_for_step(&step, committed_actions) else {
        handle_current_step_failure(ctx, runtime, blocked_memory, agent, &step, None)?;
        return Ok(());
    };
    reconcile_committed_facility_queue_intents(runtime, &step);
    if apply_step_materialization_bindings(runtime, &step, &committed_action.outcome).is_err() {
        handle_current_step_failure(ctx, runtime, blocked_memory, agent, &step, None)?;
        return Ok(());
    }

    runtime.step_in_flight = false;
    advance_completed_step(runtime, step.op_kind, ctx.tick);
    Ok(())
}

fn reconcile_committed_facility_queue_intents(
    runtime: &mut AgentDecisionRuntime,
    step: &PlannedStep,
) {
    let Some(facility) = step.targets.first().copied().and_then(authoritative_target) else {
        return;
    };

    match step.op_kind {
        crate::PlannerOpKind::QueueForFacilityUse => {
            let Some(goal_key) = runtime
                .current_goal
                .or_else(|| runtime.current_plan.as_ref().map(|plan| plan.goal))
            else {
                return;
            };
            let Some(payload) = step
                .payload_override
                .as_ref()
                .and_then(worldwake_sim::ActionPayload::as_queue_for_facility_use)
            else {
                return;
            };
            runtime.queued_facility_intents.insert(
                facility,
                QueuedFacilityIntent {
                    goal_key,
                    intended_action: payload.intended_action,
                },
            );
        }
        crate::PlannerOpKind::Harvest | crate::PlannerOpKind::Craft => {
            runtime.queued_facility_intents.remove(&facility);
        }
        crate::PlannerOpKind::Travel
        | crate::PlannerOpKind::Sleep
        | crate::PlannerOpKind::Relieve
        | crate::PlannerOpKind::Trade
        | crate::PlannerOpKind::Consume
        | crate::PlannerOpKind::Wash
        | crate::PlannerOpKind::Heal
        | crate::PlannerOpKind::MoveCargo
        | crate::PlannerOpKind::Loot
        | crate::PlannerOpKind::Bury
        | crate::PlannerOpKind::Attack
        | crate::PlannerOpKind::Defend => {}
    }
}

fn advance_completed_step(
    runtime: &mut AgentDecisionRuntime,
    completed_op_kind: crate::PlannerOpKind,
    tick: Tick,
) {
    let completed_plan_relation = runtime
        .current_plan
        .as_ref()
        .map(|plan| runtime.classify_journey_plan_relation(plan));

    if completed_op_kind == crate::PlannerOpKind::Travel {
        runtime.journey_last_progress_tick = Some(tick);
        runtime.consecutive_blocked_leg_ticks = 0;
    }

    runtime.current_step_index = runtime
        .current_step_index
        .checked_add(1)
        .expect("agent decision runtime step index overflowed");

    let Some(plan) = runtime.current_plan.as_ref() else {
        return;
    };
    if runtime.current_step_index < plan.steps.len() {
        return;
    }

    match plan.terminal_kind {
        PlanTerminalKind::ProgressBarrier => {
            runtime.current_goal = Some(plan.goal);
            runtime.current_plan = None;
            runtime.current_step_index = 0;
            runtime.dirty = true;
            runtime.materialization_bindings.clear();
        }
        PlanTerminalKind::GoalSatisfied | PlanTerminalKind::CombatCommitment => {
            if completed_plan_relation == Some(crate::JourneyPlanRelation::SuspendsCommitment) {
                runtime.journey_commitment_state = JourneyCommitmentState::Active;
            } else {
                runtime.clear_journey_commitment_with_reason(JourneyClearReason::GoalSatisfied);
            }
            runtime.current_goal = None;
            runtime.current_plan = None;
            runtime.current_step_index = 0;
            runtime.dirty = true;
            runtime.materialization_bindings.clear();
        }
    }
}

fn handle_current_step_failure(
    ctx: &mut AgentTickContext<'_>,
    runtime: &mut AgentDecisionRuntime,
    blocked_memory: &mut BlockedIntentMemory,
    agent: EntityId,
    step: &PlannedStep,
    replan_signal: Option<&ReplanNeeded>,
) -> Result<(), TickInputError> {
    let world = &mut *ctx.world;
    let event_log = &mut *ctx.event_log;
    let budget = ctx.budget;
    let tick = ctx.tick;
    let view = PerAgentBeliefView::from_world(agent, world);
    let goal_key = runtime.current_goal.unwrap_or_else(|| {
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
            replan_signal,
            current_tick: tick,
        },
        runtime,
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

fn update_journey_fields_for_adopted_plan(
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

fn handle_recoverable_travel_step_blockage(
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

fn resolve_step_targets(
    runtime: &AgentDecisionRuntime,
    step: &PlannedStep,
) -> Option<Vec<EntityId>> {
    resolve_planning_targets_with(&step.targets, |id| {
        runtime.materialization_bindings.resolve(id)
    })
}

fn committed_action_for_step<'a>(
    step: &PlannedStep,
    committed_actions: &'a [CommittedAction],
) -> Option<&'a CommittedAction> {
    if committed_actions.len() != 1 {
        return None;
    }
    let committed = &committed_actions[0];
    (committed.def_id == step.def_id).then_some(committed)
}

fn apply_step_materialization_bindings(
    runtime: &mut AgentDecisionRuntime,
    step: &PlannedStep,
    outcome: &CommitOutcome,
) -> Result<(), ()> {
    use std::collections::BTreeSet;

    let tags = step
        .expected_materializations
        .iter()
        .map(|expected| expected.tag)
        .chain(outcome.materializations.iter().map(|actual| actual.tag))
        .collect::<BTreeSet<_>>();
    let mut newly_bound_entities = BTreeSet::new();

    for tag in tags {
        let expected = step
            .expected_materializations
            .iter()
            .filter(|expected| expected.tag == tag)
            .collect::<Vec<_>>();
        let actual = outcome
            .materializations
            .iter()
            .filter(|materialization| materialization.tag == tag)
            .collect::<Vec<_>>();
        if expected.len() != actual.len() {
            return Err(());
        }

        for (expected, actual) in expected.into_iter().zip(actual.into_iter()) {
            if !newly_bound_entities.insert(actual.entity) {
                return Err(());
            }
            if let Some(existing) = runtime
                .materialization_bindings
                .resolve(expected.hypothetical_id)
            {
                if existing != actual.entity {
                    return Err(());
                }
                continue;
            }
            runtime
                .materialization_bindings
                .bind(expected.hypothetical_id, actual.entity);
        }
    }

    Ok(())
}

fn persist_blocked_memory(
    world: &mut worldwake_core::World,
    event_log: &mut worldwake_core::EventLog,
    agent: EntityId,
    tick: Tick,
    before: &BlockedIntentMemory,
    after: &BlockedIntentMemory,
) -> Result<(), TickInputError> {
    let existing = world.get_component_blocked_intent_memory(agent);
    if existing == Some(after)
        || (existing.is_none() && before == after && after.intents.is_empty())
    {
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
    txn.set_component_blocked_intent_memory(agent, after.clone())
        .map_err(|error| TickInputError::new(error.to_string()))?;
    let _ = txn.commit(event_log);
    Ok(())
}

fn current_step(runtime: &AgentDecisionRuntime) -> Option<&PlannedStep> {
    runtime
        .current_plan
        .as_ref()
        .and_then(|plan| plan.steps.get(runtime.current_step_index))
}

fn plan_finished(runtime: &AgentDecisionRuntime) -> bool {
    runtime.current_plan.as_ref().is_some_and(|plan| {
        runtime.current_step_index >= plan.steps.len() && !runtime.step_in_flight
    })
}

fn observation_snapshot_changed(
    view: &dyn RuntimeBeliefView,
    agent: EntityId,
    runtime: &AgentDecisionRuntime,
    recipe_registry: &RecipeRegistry,
) -> bool {
    let current_commodity_signature = commodity_signature(view, agent);
    let commodity_filter = runtime
        .current_goal
        .map(|goal| goal.kind.relevant_observed_commodities(recipe_registry))
        .or_else(|| {
            runtime.current_plan.as_ref().map(|plan| {
                plan.goal
                    .kind
                    .relevant_observed_commodities(recipe_registry)
            })
        });
    runtime.last_effective_place != view.effective_place(agent)
        || runtime.last_needs != view.homeostatic_needs(agent)
        || runtime.last_wounds != view.wounds(agent)
        || filtered_commodity_signature(
            &runtime.last_commodity_signature,
            commodity_filter.as_ref(),
        ) != filtered_commodity_signature(
            &current_commodity_signature,
            commodity_filter.as_ref(),
        )
        || runtime.last_unique_item_signature != unique_item_signature(view, agent)
        || runtime.last_facility_access_signature != facility_access_signature(view, agent)
}

fn update_runtime_observation_snapshot(
    view: &dyn RuntimeBeliefView,
    agent: EntityId,
    runtime: &mut AgentDecisionRuntime,
) {
    runtime.last_effective_place = view.effective_place(agent);
    runtime.last_needs = view.homeostatic_needs(agent);
    runtime.last_wounds = view.wounds(agent);
    runtime.last_commodity_signature = commodity_signature(view, agent);
    runtime.last_unique_item_signature = unique_item_signature(view, agent);
    runtime.last_facility_access_signature = facility_access_signature(view, agent);
}

fn facility_access_signature(
    view: &dyn RuntimeBeliefView,
    agent: EntityId,
) -> Vec<(EntityId, bool, Option<ActionDefId>)> {
    let Some(place) = view.effective_place(agent) else {
        return Vec::new();
    };

    view.entities_at(place)
        .into_iter()
        .filter(|entity| view.has_exclusive_facility_policy(*entity))
        .filter_map(|facility| {
            let queued = view.facility_queue_position(facility, agent).is_some();
            let matching_grant = view
                .facility_grant(facility)
                .and_then(|grant| (grant.actor == agent).then_some(grant.intended_action));
            (queued || matching_grant.is_some()).then_some((facility, queued, matching_grant))
        })
        .collect()
}

fn facility_queue_patience_exhausted(
    view: &dyn RuntimeBeliefView,
    agent: EntityId,
    tick: Tick,
) -> bool {
    let Some(limit) = view.facility_queue_patience_ticks(agent) else {
        return false;
    };
    let Some(place) = view.effective_place(agent) else {
        return false;
    };

    view.entities_at(place).into_iter().any(|facility| {
        if !view.has_exclusive_facility_policy(facility) {
            return false;
        }
        if view
            .facility_grant(facility)
            .is_some_and(|grant| grant.actor == agent)
        {
            return false;
        }
        view.facility_queue_join_tick(facility, agent)
            .is_some_and(|queued_at| tick >= queued_at + u64::from(limit.get()))
    })
}

fn commodity_signature(
    view: &dyn RuntimeBeliefView,
    agent: EntityId,
) -> Vec<(CommodityKind, Quantity)> {
    CommodityKind::ALL
        .into_iter()
        .filter_map(|commodity| {
            let quantity = view.commodity_quantity(agent, commodity);
            (quantity > Quantity(0)).then_some((commodity, quantity))
        })
        .collect()
}

fn filtered_commodity_signature(
    signature: &[(CommodityKind, Quantity)],
    relevant: Option<&Option<std::collections::BTreeSet<CommodityKind>>>,
) -> Vec<(CommodityKind, Quantity)> {
    match relevant {
        Some(Some(relevant)) => signature
            .iter()
            .copied()
            .filter(|(commodity, _)| relevant.contains(commodity))
            .collect(),
        Some(None) | None => signature.to_vec(),
    }
}

fn unique_item_signature(
    view: &dyn RuntimeBeliefView,
    agent: EntityId,
) -> Vec<(UniqueItemKind, u32)> {
    UniqueItemKind::ALL
        .into_iter()
        .filter_map(|kind| {
            let count = view.unique_item_count(agent, kind);
            (count > 0).then_some((kind, count))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{
        abandon_expired_facility_queues_with_limit, advance_completed_step,
        apply_step_materialization_bindings, committed_action_for_step,
        effective_goal_switch_margin, facility_queue_patience_exhausted,
        handle_recoverable_travel_step_blockage, persist_blocked_memory,
        plan_and_validate_next_step, refresh_runtime_for_read_phase, resolve_step_targets,
        update_journey_fields_for_adopted_plan, update_runtime_observation_snapshot,
        AgentTickDriver, ReadPhaseContext,
    };
    use crate::PlanningBudget;
    use crate::{
        build_semantics_table, CommodityPurpose, ExpectedMaterialization, GoalKey, GoalKind,
        JourneyCommitmentState, JourneySwitchMarginSource, PlanTerminalKind, PlannedPlan,
        PlannedStep, PlannerOpKind, PlanningEntityRef, QueuedFacilityIntent, RankedGoal,
    };
    use std::collections::BTreeSet;
    use std::fs;
    use std::num::NonZeroU32;
    use std::path::PathBuf;
    use worldwake_core::{
        build_believed_entity_state, build_prototype_world, ActionDefId, BeliefConfidencePolicy,
        BlockedIntent, BlockedIntentMemory, BlockingFact, BodyCostPerTick, CarryCapacity,
        CauseRef, CommodityKind, ControlSource, DeadAt, DemandMemory, DemandObservation,
        DemandObservationReason, DeprivationExposure, DriveThresholds, EntityId, EntityKind,
        EventLog, ExclusiveFacilityPolicy, FacilityUseQueue, GrantedFacilityUse,
        HomeostaticNeeds, KnownRecipes, LoadUnits, MerchandiseProfile, MetabolismProfile,
        PendingEvent, PerceptionProfile, PerceptionSource, Permille, Place, Quantity, RecipeId,
        ResourceSource, Seed, Tick, Topology, TravelDispositionProfile, TravelEdge, TravelEdgeId,
        UtilityProfile, VisibilitySpec, WitnessData, WorkstationMarker, WorkstationTag, World,
        WorldTxn,
    };
    use worldwake_sim::{
        step_tick, ActionDefRegistry, ActionDuration, ActionHandlerRegistry,
        AutonomousControllerRuntime, CommitOutcome, CommittedAction, ControllerState,
        DeterministicRng, DurationExpr, Materialization, MaterializationTag, PerAgentBeliefView,
        RecipeDefinition, RecipeRegistry, RuntimeBeliefView, Scheduler, SystemDispatchTable,
        SystemExecutionContext, SystemId, SystemManifest, TickStepServices,
    };
    use worldwake_systems::{
        build_full_action_registries, perception_system, register_needs_actions,
    };

    struct Harness {
        world: World,
        event_log: EventLog,
        scheduler: Scheduler,
        controller: ControllerState,
        rng: DeterministicRng,
        recipes: RecipeRegistry,
        defs: ActionDefRegistry,
        handlers: ActionHandlerRegistry,
        driver: AgentTickDriver,
        actor: worldwake_core::EntityId,
    }

    impl Harness {
        fn new(control_source: ControlSource) -> Self {
            let mut world = World::new(build_prototype_world()).unwrap();
            let place = world.topology().place_ids().next().unwrap();
            let actor = {
                let mut txn = new_txn(&mut world, 1);
                let actor = txn.create_agent("Aster", control_source).unwrap();
                let bread = txn
                    .create_item_lot(CommodityKind::Bread, Quantity(1))
                    .unwrap();
                txn.set_ground_location(actor, place).unwrap();
                txn.set_ground_location(bread, place).unwrap();
                txn.set_possessor(bread, actor).unwrap();
                txn.set_component_homeostatic_needs(
                    actor,
                    HomeostaticNeeds::new(
                        worldwake_core::Permille::new(800).unwrap(),
                        worldwake_core::Permille::new(0).unwrap(),
                        worldwake_core::Permille::new(0).unwrap(),
                        worldwake_core::Permille::new(0).unwrap(),
                        worldwake_core::Permille::new(0).unwrap(),
                    ),
                )
                .unwrap();
                txn.set_component_deprivation_exposure(actor, DeprivationExposure::default())
                    .unwrap();
                txn.set_component_drive_thresholds(actor, DriveThresholds::default())
                    .unwrap();
                txn.set_component_metabolism_profile(actor, MetabolismProfile::default())
                    .unwrap();
                commit_txn(txn);
                actor
            };

            let mut defs = ActionDefRegistry::new();
            let mut handlers = ActionHandlerRegistry::new();
            register_needs_actions(&mut defs, &mut handlers);

            sync_all_beliefs(&mut world, actor, Tick(1));

            Self {
                world,
                event_log: EventLog::new(),
                scheduler: Scheduler::new(SystemManifest::canonical()),
                controller: ControllerState::with_entity(actor),
                rng: DeterministicRng::new(Seed([3; 32])),
                recipes: RecipeRegistry::new(),
                defs,
                handlers,
                driver: AgentTickDriver::new(PlanningBudget::default()),
                actor,
            }
        }

        fn step_once(&mut self) -> worldwake_sim::TickStepResult {
            let mut controllers = AutonomousControllerRuntime::new(vec![&mut self.driver]);
            step_tick(
                &mut self.world,
                &mut self.event_log,
                &mut self.scheduler,
                &mut self.controller,
                &mut self.rng,
                TickStepServices {
                    action_defs: &self.defs,
                    action_handlers: &self.handlers,
                    recipe_registry: &self.recipes,
                    systems: &SystemDispatchTable::canonical_noop(),
                    input_producer: Some(&mut controllers),
                },
            )
            .unwrap()
        }

        fn active_action_name(&self) -> Option<&str> {
            self.scheduler
                .active_actions()
                .values()
                .next()
                .and_then(|action| self.defs.get(action.def_id))
                .map(|def| def.name.as_str())
        }

        fn runtime(&self) -> Option<&crate::AgentDecisionRuntime> {
            self.driver.runtime_by_agent.get(&self.actor)
        }
    }

    fn cargo_topology(origin: EntityId, destination: EntityId) -> Topology {
        let mut topology = Topology::new();
        topology
            .add_place(
                origin,
                Place {
                    name: "Origin".to_string(),
                    capacity: None,
                    tags: BTreeSet::default(),
                },
            )
            .unwrap();
        topology
            .add_place(
                destination,
                Place {
                    name: "Destination".to_string(),
                    capacity: None,
                    tags: BTreeSet::default(),
                },
            )
            .unwrap();
        topology
            .add_edge(TravelEdge::new(TravelEdgeId(1), origin, destination, 2, None).unwrap())
            .unwrap();
        topology
            .add_edge(TravelEdge::new(TravelEdgeId(2), destination, origin, 2, None).unwrap())
            .unwrap();
        topology
    }

    fn seed_cargo_harness_actor(
        world: &mut World,
        origin: EntityId,
        destination: EntityId,
        possessed: bool,
    ) -> (EntityId, EntityId) {
        let mut txn = new_txn(world, 1);
        let actor = txn.create_agent("Mira", ControlSource::Ai).unwrap();
        let water = txn
            .create_item_lot(CommodityKind::Bread, Quantity(3))
            .unwrap();
        txn.set_ground_location(actor, origin).unwrap();
        txn.set_ground_location(water, origin).unwrap();
        if possessed {
            txn.set_possessor(water, actor).unwrap();
        } else {
            txn.set_owner(water, actor).unwrap();
        }
        txn.set_component_homeostatic_needs(actor, HomeostaticNeeds::default())
            .unwrap();
        txn.set_component_deprivation_exposure(actor, DeprivationExposure::default())
            .unwrap();
        txn.set_component_drive_thresholds(actor, DriveThresholds::default())
            .unwrap();
        txn.set_component_metabolism_profile(actor, MetabolismProfile::default())
            .unwrap();
        txn.set_component_carry_capacity(actor, CarryCapacity(LoadUnits(3)))
            .unwrap();
        txn.set_component_merchandise_profile(
            actor,
            MerchandiseProfile {
                sale_kinds: [CommodityKind::Bread].into_iter().collect(),
                home_market: Some(destination),
            },
        )
        .unwrap();
        txn.set_component_demand_memory(
            actor,
            DemandMemory {
                observations: vec![DemandObservation {
                    commodity: CommodityKind::Bread,
                    quantity: Quantity(2),
                    place: destination,
                    tick: Tick(1),
                    counterparty: None,
                    reason: DemandObservationReason::WantedToBuyButNoSeller,
                }],
            },
        )
        .unwrap();
        commit_txn(txn);
        (actor, water)
    }

    fn cargo_harness(possessed: bool) -> (Harness, EntityId, EntityId, EntityId) {
        let origin = entity(1);
        let destination = entity(2);
        let mut world = World::new(cargo_topology(origin, destination)).unwrap();
        let actor = seed_cargo_harness_actor(&mut world, origin, destination, possessed);
        let recipes = RecipeRegistry::new();
        let registries = build_full_action_registries(&recipes).unwrap();

        sync_all_beliefs(&mut world, actor.0, Tick(1));

        (
            Harness {
                world,
                event_log: EventLog::new(),
                scheduler: Scheduler::new(SystemManifest::canonical()),
                controller: ControllerState::with_entity(actor.0),
                rng: DeterministicRng::new(Seed([9; 32])),
                recipes,
                defs: registries.defs,
                handlers: registries.handlers,
                driver: AgentTickDriver::new(PlanningBudget {
                    max_plan_depth: 2,
                    ..PlanningBudget::default()
                }),
                actor: actor.0,
            },
            actor.1,
            origin,
            destination,
        )
    }

    fn step_until(harness: &mut Harness, max_ticks: usize, predicate: impl Fn(&Harness) -> bool) {
        for _ in 0..max_ticks {
            if predicate(harness) {
                return;
            }
            let _ = harness.step_once();
        }
        assert!(
            predicate(harness),
            "condition not met within {max_ticks} ticks"
        );
    }

    fn new_txn(world: &mut World, tick: u64) -> WorldTxn<'_> {
        WorldTxn::new(
            world,
            Tick(tick),
            CauseRef::Bootstrap,
            None,
            None,
            VisibilitySpec::SamePlace,
            WitnessData::default(),
        )
    }

    fn commit_txn(txn: WorldTxn<'_>) {
        let mut event_log = EventLog::new();
        let _ = txn.commit(&mut event_log);
    }

    fn sync_all_beliefs(world: &mut World, observer: EntityId, observed_tick: Tick) {
        let snapshots = world
            .entities()
            .filter(|entity| *entity != observer)
            .filter_map(|entity| {
                build_believed_entity_state(
                    world,
                    entity,
                    observed_tick,
                    PerceptionSource::DirectObservation,
                )
                .map(|state| (entity, state))
            })
            .collect::<Vec<_>>();
        let mut store = world
            .get_component_agent_belief_store(observer)
            .cloned()
            .expect("observer must have AgentBeliefStore");
        store.known_entities.clear();
        for (entity, state) in snapshots {
            store.update_entity(entity, state);
        }
        let mut txn = WorldTxn::new(
            world,
            observed_tick,
            CauseRef::Bootstrap,
            None,
            None,
            VisibilitySpec::SamePlace,
            WitnessData::default(),
        );
        txn.set_component_agent_belief_store(observer, store)
            .expect("observer belief store should remain writable");
        commit_txn(txn);
    }

    fn sync_selected_beliefs(
        world: &mut World,
        observer: EntityId,
        entities: &[EntityId],
        observed_tick: Tick,
        source: PerceptionSource,
    ) {
        let mut store = world
            .get_component_agent_belief_store(observer)
            .cloned()
            .expect("observer must have AgentBeliefStore");
        store.known_entities.clear();
        for entity in entities {
            if let Some(state) = build_believed_entity_state(world, *entity, observed_tick, source) {
                store.update_entity(*entity, state);
            }
        }
        let mut txn = WorldTxn::new(
            world,
            observed_tick,
            CauseRef::Bootstrap,
            None,
            None,
            VisibilitySpec::SamePlace,
            WitnessData::default(),
        );
        txn.set_component_agent_belief_store(observer, store)
            .expect("observer belief store should remain writable");
        commit_txn(txn);
    }

    fn hungry_acquisition_harness() -> (Harness, EntityId, EntityId, EntityId) {
        let origin = entity(11);
        let destination = entity(12);
        let mut world = World::new(cargo_topology(origin, destination)).unwrap();
        let (actor, seller) = {
            let mut txn = new_txn(&mut world, 1);
            let actor = txn.create_agent("Hungry", ControlSource::Ai).unwrap();
            let seller = txn.create_agent("Seller", ControlSource::Ai).unwrap();
            let bread = txn
                .create_item_lot(CommodityKind::Bread, Quantity(3))
                .unwrap();
            txn.set_ground_location(actor, origin).unwrap();
            txn.set_ground_location(seller, origin).unwrap();
            txn.set_ground_location(bread, origin).unwrap();
            txn.set_possessor(bread, seller).unwrap();
            txn.set_component_homeostatic_needs(
                actor,
                HomeostaticNeeds::new(
                    Permille::new(800).unwrap(),
                    Permille::new(0).unwrap(),
                    Permille::new(0).unwrap(),
                    Permille::new(0).unwrap(),
                    Permille::new(0).unwrap(),
                ),
            )
            .unwrap();
            txn.set_component_deprivation_exposure(actor, DeprivationExposure::default())
                .unwrap();
            txn.set_component_drive_thresholds(actor, DriveThresholds::default())
                .unwrap();
            txn.set_component_metabolism_profile(actor, MetabolismProfile::default())
                .unwrap();
            txn.set_component_perception_profile(
                actor,
                PerceptionProfile {
                    memory_capacity: 12,
                    memory_retention_ticks: 64,
                    observation_fidelity: Permille::new(1000).unwrap(),
                    confidence_policy: BeliefConfidencePolicy::default(),
                },
            )
            .unwrap();
            txn.set_component_merchandise_profile(
                seller,
                MerchandiseProfile {
                    sale_kinds: [CommodityKind::Bread].into_iter().collect(),
                    home_market: Some(origin),
                },
            )
            .unwrap();
            commit_txn(txn);
            (actor, seller)
        };

        let mut defs = ActionDefRegistry::new();
        let mut handlers = ActionHandlerRegistry::new();
        register_needs_actions(&mut defs, &mut handlers);

        (
            Harness {
                world,
                event_log: EventLog::new(),
                scheduler: Scheduler::new(SystemManifest::canonical()),
                controller: ControllerState::with_entity(actor),
                rng: DeterministicRng::new(Seed([5; 32])),
                recipes: RecipeRegistry::new(),
                defs,
                handlers,
                driver: AgentTickDriver::new(PlanningBudget::default()),
                actor,
            },
            seller,
            origin,
            destination,
        )
    }

    fn stale_remote_acquisition_harness() -> (Harness, EntityId, EntityId, EntityId, EntityId) {
        let origin = entity(21);
        let destination = entity(22);
        let mut world = World::new(cargo_topology(origin, destination)).unwrap();
        let (actor, seller, local_witness) = {
            let mut txn = new_txn(&mut world, 0);
            let actor = txn.create_agent("Hungry", ControlSource::Ai).unwrap();
            let seller = txn.create_agent("RemoteSeller", ControlSource::Ai).unwrap();
            let local_witness = txn.create_agent("Witness", ControlSource::Ai).unwrap();
            let bread = txn
                .create_item_lot(CommodityKind::Bread, Quantity(3))
                .unwrap();
            txn.set_ground_location(actor, origin).unwrap();
            txn.set_ground_location(local_witness, origin).unwrap();
            txn.set_ground_location(seller, destination).unwrap();
            txn.set_ground_location(bread, destination).unwrap();
            txn.set_possessor(bread, seller).unwrap();
            txn.set_component_homeostatic_needs(
                actor,
                HomeostaticNeeds::new(
                    Permille::new(800).unwrap(),
                    Permille::new(0).unwrap(),
                    Permille::new(0).unwrap(),
                    Permille::new(0).unwrap(),
                    Permille::new(0).unwrap(),
                ),
            )
            .unwrap();
            txn.set_component_deprivation_exposure(actor, DeprivationExposure::default())
                .unwrap();
            txn.set_component_drive_thresholds(actor, DriveThresholds::default())
                .unwrap();
            txn.set_component_metabolism_profile(actor, MetabolismProfile::default())
                .unwrap();
            txn.set_component_perception_profile(
                actor,
                PerceptionProfile {
                    memory_capacity: 12,
                    memory_retention_ticks: 4,
                    observation_fidelity: Permille::new(1000).unwrap(),
                    confidence_policy: BeliefConfidencePolicy::default(),
                },
            )
            .unwrap();
            txn.set_component_merchandise_profile(
                seller,
                MerchandiseProfile {
                    sale_kinds: [CommodityKind::Bread].into_iter().collect(),
                    home_market: Some(destination),
                },
            )
            .unwrap();
            commit_txn(txn);
            (actor, seller, local_witness)
        };

        let mut defs = ActionDefRegistry::new();
        let mut handlers = ActionHandlerRegistry::new();
        register_needs_actions(&mut defs, &mut handlers);

        sync_selected_beliefs(
            &mut world,
            actor,
            &[seller],
            Tick(0),
            PerceptionSource::Inference,
        );

        (
            Harness {
                world,
                event_log: EventLog::new(),
                scheduler: Scheduler::new(SystemManifest::canonical()),
                controller: ControllerState::with_entity(actor),
                rng: DeterministicRng::new(Seed([7; 32])),
                recipes: RecipeRegistry::new(),
                defs,
                handlers,
                driver: AgentTickDriver::new(PlanningBudget::default()),
                actor,
            },
            seller,
            local_witness,
            origin,
            destination,
        )
    }

    fn ranked_goals_at(harness: &mut Harness, tick: Tick) -> Vec<RankedGoal> {
        let utility = harness
            .world
            .get_component_utility_profile(harness.actor)
            .cloned()
            .unwrap_or_default();
        let runtime = harness
            .driver
            .runtime_by_agent
            .entry(harness.actor)
            .or_default();
        let mut blocked = BlockedIntentMemory::default();
        refresh_runtime_for_read_phase(
            &harness.world,
            &harness.scheduler,
            &harness.defs,
            runtime,
            &mut blocked,
            harness.actor,
            &[],
            ReadPhaseContext {
                recipe_registry: &harness.recipes,
                utility: &utility,
                tick,
                travel_horizon: PlanningBudget::default().snapshot_travel_horizon,
                structural_block_ticks: PlanningBudget::default().structural_block_ticks,
            },
        )
    }

    fn has_goal(ranked: &[RankedGoal], goal: GoalKind) -> bool {
        let key = GoalKey::from(goal);
        ranked.iter().any(|candidate| candidate.grounded.key == key)
    }

    fn run_same_place_observation(
        harness: &mut Harness,
        tick: Tick,
        place: EntityId,
        observed_actor: EntityId,
    ) {
        let _ = harness.event_log.emit(PendingEvent::new(
            tick,
            CauseRef::Bootstrap,
            Some(observed_actor),
            vec![observed_actor],
            Some(place),
            Vec::new(),
            VisibilitySpec::SamePlace,
            WitnessData::default(),
            BTreeSet::new(),
        ));
        let active_actions = std::collections::BTreeMap::new();
        perception_system(SystemExecutionContext {
            world: &mut harness.world,
            event_log: &mut harness.event_log,
            rng: &mut harness.rng,
            active_actions: &active_actions,
            action_defs: &harness.defs,
            tick,
            system_id: SystemId::Perception,
        })
        .unwrap();
    }

    fn run_perception_tick(harness: &mut Harness, tick: Tick) {
        let active_actions = std::collections::BTreeMap::new();
        perception_system(SystemExecutionContext {
            world: &mut harness.world,
            event_log: &mut harness.event_log,
            rng: &mut harness.rng,
            active_actions: &active_actions,
            action_defs: &harness.defs,
            tick,
            system_id: SystemId::Perception,
        })
        .unwrap();
    }

    fn relocate_entity(world: &mut World, entity: EntityId, destination: EntityId, tick: Tick) {
        let mut txn = new_txn(world, tick.0);
        txn.set_ground_location(entity, destination).unwrap();
        commit_txn(txn);
    }

    fn kill_entity(world: &mut World, entity: EntityId, tick: Tick) {
        let mut txn = new_txn(world, tick.0);
        txn.set_component_dead_at(entity, DeadAt(tick)).unwrap();
        commit_txn(txn);
    }

    fn entity(slot: u32) -> EntityId {
        EntityId {
            slot,
            generation: 1,
        }
    }

    fn pm(value: u16) -> Permille {
        Permille::new(value).unwrap()
    }

    fn harvest_apple_recipe() -> RecipeDefinition {
        RecipeDefinition {
            name: "Harvest Apples".to_string(),
            inputs: vec![],
            outputs: vec![(CommodityKind::Apple, Quantity(2))],
            work_ticks: NonZeroU32::new(3).unwrap(),
            required_workstation_tag: Some(WorkstationTag::OrchardRow),
            required_tool_kinds: vec![],
            body_cost_per_tick: BodyCostPerTick::new(pm(3), pm(2), pm(5), pm(1)),
        }
    }

    struct ExclusiveQueueHarness {
        world: World,
        recipes: RecipeRegistry,
        defs: ActionDefRegistry,
        handlers: ActionHandlerRegistry,
        scheduler: Scheduler,
        actor: EntityId,
        orchard_farm: EntityId,
        orchard_row: EntityId,
    }

    fn build_exclusive_queue_harness() -> ExclusiveQueueHarness {
        let orchard_farm =
            worldwake_core::prototype_place_entity(worldwake_core::PrototypePlace::OrchardFarm);
        let mut recipes = RecipeRegistry::new();
        recipes.register(harvest_apple_recipe());
        let registries = build_full_action_registries(&recipes).unwrap();
        let mut world = World::new(build_prototype_world()).unwrap();
        let (actor, orchard_row) = {
            let mut txn = new_txn(&mut world, 1);
            let actor = txn.create_agent("Merchant", ControlSource::Ai).unwrap();
            let orchard_row = txn.create_entity(EntityKind::Facility);
            txn.set_ground_location(actor, orchard_farm).unwrap();
            txn.set_ground_location(orchard_row, orchard_farm).unwrap();
            txn.set_component_homeostatic_needs(actor, HomeostaticNeeds::default())
                .unwrap();
            txn.set_component_deprivation_exposure(actor, DeprivationExposure::default())
                .unwrap();
            txn.set_component_drive_thresholds(actor, DriveThresholds::default())
                .unwrap();
            txn.set_component_metabolism_profile(actor, MetabolismProfile::default())
                .unwrap();
            txn.set_component_carry_capacity(actor, CarryCapacity(LoadUnits(50)))
                .unwrap();
            txn.set_component_known_recipes(actor, KnownRecipes::with([RecipeId(0)]))
                .unwrap();
            txn.set_component_workstation_marker(
                orchard_row,
                WorkstationMarker(WorkstationTag::OrchardRow),
            )
            .unwrap();
            txn.set_component_resource_source(
                orchard_row,
                ResourceSource {
                    commodity: CommodityKind::Apple,
                    available_quantity: Quantity(10),
                    max_quantity: Quantity(10),
                    regeneration_ticks_per_unit: None,
                    last_regeneration_tick: None,
                },
            )
            .unwrap();
            txn.set_component_exclusive_facility_policy(
                orchard_row,
                ExclusiveFacilityPolicy {
                    grant_hold_ticks: NonZeroU32::new(3).unwrap(),
                },
            )
            .unwrap();
            txn.set_component_facility_use_queue(orchard_row, FacilityUseQueue::default())
                .unwrap();
            commit_txn(txn);
            (actor, orchard_row)
        };

        sync_all_beliefs(&mut world, actor, Tick(1));

        ExclusiveQueueHarness {
            world,
            recipes,
            defs: registries.defs,
            handlers: registries.handlers,
            scheduler: Scheduler::new(SystemManifest::canonical()),
            actor,
            orchard_farm,
            orchard_row,
        }
    }

    fn set_local_queue_state(
        world: &mut World,
        actor: EntityId,
        facility: EntityId,
        queued_at: u64,
        grant_action: Option<ActionDefId>,
    ) {
        let mut txn = new_txn(world, queued_at.max(1));
        let mut queue = txn
            .get_component_facility_use_queue(facility)
            .cloned()
            .unwrap_or_default();
        queue.waiting.clear();
        queue.granted = None;
        if let Some(action_def) = grant_action {
            queue.granted = Some(GrantedFacilityUse {
                actor,
                intended_action: action_def,
                granted_at: Tick(queued_at),
                expires_at: Tick(queued_at + 3),
            });
        } else {
            queue
                .enqueue(actor, ActionDefId(77), Tick(queued_at))
                .unwrap();
        }
        txn.set_component_facility_use_queue(facility, queue)
            .unwrap();
        commit_txn(txn);
        sync_all_beliefs(world, actor, Tick(queued_at.max(1)));
    }

    fn clear_local_queue_state(world: &mut World, actor: EntityId, facility: EntityId, tick: u64) {
        let mut txn = new_txn(world, tick.max(1));
        let mut queue = txn
            .get_component_facility_use_queue(facility)
            .cloned()
            .unwrap_or_default();
        queue.waiting.clear();
        queue.granted = None;
        txn.set_component_facility_use_queue(facility, queue)
            .unwrap();
        commit_txn(txn);
        sync_all_beliefs(world, actor, Tick(tick.max(1)));
    }

    fn add_local_queued_facility(world: &mut World, actor: EntityId, queued_at: u64) -> EntityId {
        let place = world.effective_place(actor).unwrap();
        let facility = {
            let mut txn = new_txn(world, queued_at.max(1));
            let facility = txn.create_entity(EntityKind::Facility);
            txn.set_ground_location(facility, place).unwrap();
            txn.set_component_exclusive_facility_policy(
                facility,
                ExclusiveFacilityPolicy {
                    grant_hold_ticks: NonZeroU32::new(3).unwrap(),
                },
            )
            .unwrap();
            txn.set_component_facility_use_queue(facility, FacilityUseQueue::default())
                .unwrap();
            commit_txn(txn);
            facility
        };
        set_local_queue_state(world, actor, facility, queued_at, None);
        facility
    }

    fn barrier_step() -> PlannedStep {
        PlannedStep {
            def_id: ActionDefId(8),
            targets: vec![PlanningEntityRef::Authoritative(entity(11))],
            payload_override: None,
            op_kind: PlannerOpKind::Trade,
            estimated_ticks: 3,
            is_materialization_barrier: true,
            expected_materializations: Vec::new(),
        }
    }

    fn travel_step(def_id: u32, target: EntityId) -> PlannedStep {
        PlannedStep {
            def_id: ActionDefId(def_id),
            targets: vec![PlanningEntityRef::Authoritative(target)],
            payload_override: None,
            op_kind: PlannerOpKind::Travel,
            estimated_ticks: 1,
            is_materialization_barrier: false,
            expected_materializations: Vec::new(),
        }
    }

    fn hypothetical_step(def_id: u32, hypothetical: u32) -> PlannedStep {
        PlannedStep {
            def_id: ActionDefId(def_id),
            targets: vec![PlanningEntityRef::Hypothetical(
                crate::HypotheticalEntityId(hypothetical),
            )],
            payload_override: None,
            op_kind: PlannerOpKind::MoveCargo,
            estimated_ticks: 1,
            is_materialization_barrier: false,
            expected_materializations: vec![ExpectedMaterialization {
                tag: MaterializationTag::SplitOffLot,
                hypothetical_id: crate::HypotheticalEntityId(hypothetical),
            }],
        }
    }

    fn active_runtime(goal: GoalKind) -> crate::AgentDecisionRuntime {
        let goal = GoalKey::from(goal);
        crate::AgentDecisionRuntime {
            current_goal: Some(goal),
            current_plan: Some(PlannedPlan::new(
                goal,
                vec![barrier_step()],
                PlanTerminalKind::GoalSatisfied,
            )),
            current_step_index: 0,
            step_in_flight: false,
            dirty: false,
            ..crate::AgentDecisionRuntime::default()
        }
    }

    fn ranked_goal(
        goal: GoalKind,
        evidence_entities: impl IntoIterator<Item = EntityId>,
        evidence_places: impl IntoIterator<Item = EntityId>,
    ) -> RankedGoal {
        RankedGoal {
            grounded: crate::GroundedGoal {
                key: GoalKey::from(goal),
                evidence_entities: evidence_entities.into_iter().collect(),
                evidence_places: evidence_places.into_iter().collect(),
            },
            priority_class: crate::GoalPriorityClass::Medium,
            motive_score: 500,
        }
    }

    #[derive(Default)]
    struct QueuePatienceBeliefView {
        place: Option<EntityId>,
        facilities_at_place: Vec<EntityId>,
        queue_join_ticks: std::collections::BTreeMap<EntityId, Tick>,
        grants: std::collections::BTreeMap<EntityId, GrantedFacilityUse>,
        patience_ticks: Option<NonZeroU32>,
    }

    impl RuntimeBeliefView for QueuePatienceBeliefView {
        fn is_alive(&self, _entity: EntityId) -> bool {
            true
        }
        fn entity_kind(&self, _entity: EntityId) -> Option<EntityKind> {
            None
        }
        fn effective_place(&self, _entity: EntityId) -> Option<EntityId> {
            self.place
        }
        fn is_in_transit(&self, _entity: EntityId) -> bool {
            false
        }
        fn entities_at(&self, _place: EntityId) -> Vec<EntityId> {
            self.facilities_at_place.clone()
        }
        fn direct_possessions(&self, _holder: EntityId) -> Vec<EntityId> {
            Vec::new()
        }
        fn adjacent_places(&self, _place: EntityId) -> Vec<EntityId> {
            Vec::new()
        }
        fn knows_recipe(&self, _actor: EntityId, _recipe: RecipeId) -> bool {
            false
        }
        fn unique_item_count(
            &self,
            _holder: EntityId,
            _kind: worldwake_core::UniqueItemKind,
        ) -> u32 {
            0
        }
        fn commodity_quantity(&self, _holder: EntityId, _kind: CommodityKind) -> Quantity {
            Quantity(0)
        }
        fn controlled_commodity_quantity_at_place(
            &self,
            _agent: EntityId,
            _place: EntityId,
            _commodity: CommodityKind,
        ) -> Quantity {
            Quantity(0)
        }
        fn local_controlled_lots_for(
            &self,
            _agent: EntityId,
            _place: EntityId,
            _commodity: CommodityKind,
        ) -> Vec<EntityId> {
            Vec::new()
        }
        fn item_lot_commodity(&self, _entity: EntityId) -> Option<CommodityKind> {
            None
        }
        fn item_lot_consumable_profile(
            &self,
            _entity: EntityId,
        ) -> Option<worldwake_core::CommodityConsumableProfile> {
            None
        }
        fn direct_container(&self, _entity: EntityId) -> Option<EntityId> {
            None
        }
        fn direct_possessor(&self, _entity: EntityId) -> Option<EntityId> {
            None
        }
        fn workstation_tag(&self, _entity: EntityId) -> Option<WorkstationTag> {
            None
        }
        fn has_exclusive_facility_policy(&self, entity: EntityId) -> bool {
            self.facilities_at_place.contains(&entity)
        }
        fn facility_queue_position(&self, facility: EntityId, _actor: EntityId) -> Option<u32> {
            self.queue_join_ticks.contains_key(&facility).then_some(0)
        }
        fn facility_grant(&self, facility: EntityId) -> Option<&GrantedFacilityUse> {
            self.grants.get(&facility)
        }
        fn facility_queue_join_tick(&self, facility: EntityId, _actor: EntityId) -> Option<Tick> {
            self.queue_join_ticks.get(&facility).copied()
        }
        fn facility_queue_patience_ticks(&self, _agent: EntityId) -> Option<NonZeroU32> {
            self.patience_ticks
        }
        fn place_has_tag(&self, _place: EntityId, _tag: worldwake_core::PlaceTag) -> bool {
            false
        }
        fn resource_source(&self, _entity: EntityId) -> Option<ResourceSource> {
            None
        }
        fn has_production_job(&self, _entity: EntityId) -> bool {
            false
        }
        fn can_control(&self, _actor: EntityId, _entity: EntityId) -> bool {
            false
        }
        fn has_control(&self, _entity: EntityId) -> bool {
            false
        }
        fn carry_capacity(&self, _entity: EntityId) -> Option<LoadUnits> {
            None
        }
        fn load_of_entity(&self, _entity: EntityId) -> Option<LoadUnits> {
            None
        }
        fn reservation_conflicts(
            &self,
            _entity: EntityId,
            _range: worldwake_core::TickRange,
        ) -> bool {
            false
        }
        fn reservation_ranges(&self, _entity: EntityId) -> Vec<worldwake_core::TickRange> {
            Vec::new()
        }
        fn is_dead(&self, _entity: EntityId) -> bool {
            false
        }
        fn is_incapacitated(&self, _entity: EntityId) -> bool {
            false
        }
        fn has_wounds(&self, _entity: EntityId) -> bool {
            false
        }
        fn homeostatic_needs(&self, _agent: EntityId) -> Option<HomeostaticNeeds> {
            None
        }
        fn drive_thresholds(&self, _agent: EntityId) -> Option<DriveThresholds> {
            None
        }
        fn metabolism_profile(&self, _agent: EntityId) -> Option<MetabolismProfile> {
            None
        }
        fn trade_disposition_profile(
            &self,
            _agent: EntityId,
        ) -> Option<worldwake_core::TradeDispositionProfile> {
            None
        }
        fn travel_disposition_profile(&self, _agent: EntityId) -> Option<TravelDispositionProfile> {
            None
        }
        fn combat_profile(&self, _agent: EntityId) -> Option<worldwake_core::CombatProfile> {
            None
        }
        fn wounds(&self, _agent: EntityId) -> Vec<worldwake_core::Wound> {
            Vec::new()
        }
        fn visible_hostiles_for(&self, _agent: EntityId) -> Vec<EntityId> {
            Vec::new()
        }
        fn current_attackers_of(&self, _agent: EntityId) -> Vec<EntityId> {
            Vec::new()
        }
        fn agents_selling_at(&self, _place: EntityId, _commodity: CommodityKind) -> Vec<EntityId> {
            Vec::new()
        }
        fn known_recipes(&self, _agent: EntityId) -> Vec<RecipeId> {
            Vec::new()
        }
        fn matching_workstations_at(
            &self,
            _place: EntityId,
            _tag: WorkstationTag,
        ) -> Vec<EntityId> {
            Vec::new()
        }
        fn resource_sources_at(
            &self,
            _place: EntityId,
            _commodity: CommodityKind,
        ) -> Vec<EntityId> {
            Vec::new()
        }
        fn demand_memory(&self, _agent: EntityId) -> Vec<DemandObservation> {
            Vec::new()
        }
        fn merchandise_profile(&self, _agent: EntityId) -> Option<MerchandiseProfile> {
            None
        }
        fn corpse_entities_at(&self, _place: EntityId) -> Vec<EntityId> {
            Vec::new()
        }
        fn in_transit_state(&self, _entity: EntityId) -> Option<worldwake_core::InTransitOnEdge> {
            None
        }
        fn adjacent_places_with_travel_ticks(
            &self,
            _place: EntityId,
        ) -> Vec<(EntityId, NonZeroU32)> {
            Vec::new()
        }
        fn estimate_duration(
            &self,
            _actor: EntityId,
            _duration: &DurationExpr,
            _targets: &[EntityId],
            _payload: &worldwake_sim::ActionPayload,
        ) -> Option<ActionDuration> {
            None
        }
    }

    #[test]
    fn effective_goal_switch_margin_uses_route_margin_for_any_journey_commitment() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let place = world.topology().place_ids().next().unwrap();
        let actor = {
            let mut txn = new_txn(&mut world, 1);
            let actor = txn.create_agent("Aster", ControlSource::Ai).unwrap();
            txn.set_ground_location(actor, place).unwrap();
            txn.set_component_travel_disposition_profile(
                actor,
                TravelDispositionProfile {
                    route_replan_margin: Permille::new(300).unwrap(),
                    blocked_leg_patience_ticks: std::num::NonZeroU32::new(4).unwrap(),
                },
            )
            .unwrap();
            commit_txn(txn);
            actor
        };
        let budget = PlanningBudget::default();
        let view = PerAgentBeliefView::from_world(actor, &world);
        let active_journey = crate::AgentDecisionRuntime {
            current_plan: Some(PlannedPlan::new(
                GoalKey::from(GoalKind::Sleep),
                vec![travel_step(1, place)],
                PlanTerminalKind::GoalSatisfied,
            )),
            journey_committed_goal: Some(GoalKey::from(GoalKind::Sleep)),
            journey_committed_destination: Some(place),
            journey_established_at: Some(Tick(7)),
            ..crate::AgentDecisionRuntime::default()
        };
        let planless_commitment = crate::AgentDecisionRuntime {
            journey_committed_goal: Some(GoalKey::from(GoalKind::Sleep)),
            journey_committed_destination: Some(place),
            journey_established_at: Some(Tick(7)),
            ..crate::AgentDecisionRuntime::default()
        };
        let not_a_journey = crate::AgentDecisionRuntime {
            current_plan: Some(PlannedPlan::new(
                GoalKey::from(GoalKind::Sleep),
                vec![barrier_step()],
                PlanTerminalKind::GoalSatisfied,
            )),
            journey_established_at: Some(Tick(7)),
            ..crate::AgentDecisionRuntime::default()
        };

        assert_eq!(
            effective_goal_switch_margin(&view, actor, &active_journey, &budget),
            Permille::new(300).unwrap()
        );
        assert_eq!(
            effective_goal_switch_margin(&view, actor, &planless_commitment, &budget),
            Permille::new(300).unwrap()
        );
        assert_eq!(
            effective_goal_switch_margin(&view, actor, &not_a_journey, &budget),
            budget.switch_margin_permille
        );
        assert_eq!(
            effective_goal_switch_margin(&view, entity(999), &active_journey, &budget,),
            budget.switch_margin_permille
        );
    }

    #[test]
    fn grant_arrival_marks_runtime_dirty_from_facility_access_snapshot() {
        let mut harness = Harness::new(ControlSource::Ai);
        let facility = add_local_queued_facility(&mut harness.world, harness.actor, 1);
        let mut runtime = active_runtime(GoalKind::Sleep);
        let view = PerAgentBeliefView::from_world(harness.actor, &harness.world);
        update_runtime_observation_snapshot(&view, harness.actor, &mut runtime);

        set_local_queue_state(
            &mut harness.world,
            harness.actor,
            facility,
            2,
            Some(ActionDefId(77)),
        );

        let mut blocked = BlockedIntentMemory::default();
        let _ = refresh_runtime_for_read_phase(
            &harness.world,
            &harness.scheduler,
            &harness.defs,
            &mut runtime,
            &mut blocked,
            harness.actor,
            &[],
            ReadPhaseContext {
                recipe_registry: &harness.recipes,
                utility: &UtilityProfile::default(),
                tick: Tick(2),
                travel_horizon: PlanningBudget::default().snapshot_travel_horizon,
                structural_block_ticks: PlanningBudget::default().structural_block_ticks,
            },
        );

        assert!(runtime.dirty);
    }

    #[test]
    fn queue_patience_exhaustion_marks_runtime_dirty() {
        let agent = entity(1);
        let place = entity(2);
        let facility = entity(3);
        let view = QueuePatienceBeliefView {
            place: Some(place),
            facilities_at_place: vec![facility],
            queue_join_ticks: [(facility, Tick(1))].into_iter().collect(),
            patience_ticks: NonZeroU32::new(3),
            ..QueuePatienceBeliefView::default()
        };

        assert!(facility_queue_patience_exhausted(&view, agent, Tick(4)));
    }

    #[test]
    fn abandon_expired_facility_queues_removes_actor_from_authoritative_queue() {
        let mut harness = Harness::new(ControlSource::Ai);
        let facility = add_local_queued_facility(&mut harness.world, harness.actor, 1);

        assert!(abandon_expired_facility_queues_with_limit(
            &mut harness.world,
            &mut harness.event_log,
            harness.actor,
            Tick(4),
            NonZeroU32::new(3).unwrap(),
        )
        .unwrap());

        let queue = harness
            .world
            .get_component_facility_use_queue(facility)
            .expect("facility queue should remain attached");
        assert_eq!(
            queue.position_of(harness.actor),
            None,
            "Patience expiry should remove the actor from authoritative queue state"
        );
    }

    #[test]
    fn abandoned_queue_then_records_standard_exclusive_facility_blocker() {
        let mut harness = Harness::new(ControlSource::Ai);
        let facility = add_local_queued_facility(&mut harness.world, harness.actor, 1);
        let goal = GoalKey::from(GoalKind::RestockCommodity {
            commodity: CommodityKind::Apple,
        });
        let mut runtime = crate::AgentDecisionRuntime {
            current_goal: Some(goal),
            ..crate::AgentDecisionRuntime::default()
        };
        runtime.queued_facility_intents.insert(
            facility,
            QueuedFacilityIntent {
                goal_key: goal,
                intended_action: ActionDefId(77),
            },
        );
        let initial_view = PerAgentBeliefView::from_world(harness.actor, &harness.world);
        update_runtime_observation_snapshot(&initial_view, harness.actor, &mut runtime);

        assert!(abandon_expired_facility_queues_with_limit(
            &mut harness.world,
            &mut harness.event_log,
            harness.actor,
            Tick(4),
            NonZeroU32::new(3).unwrap(),
        )
        .unwrap());

        let mut blocked = BlockedIntentMemory::default();
        let _ = refresh_runtime_for_read_phase(
            &harness.world,
            &harness.scheduler,
            &harness.defs,
            &mut runtime,
            &mut blocked,
            harness.actor,
            &[],
            ReadPhaseContext {
                recipe_registry: &harness.recipes,
                utility: &UtilityProfile::default(),
                tick: Tick(4),
                travel_horizon: PlanningBudget::default().snapshot_travel_horizon,
                structural_block_ticks: PlanningBudget::default().structural_block_ticks,
            },
        );

        assert_eq!(blocked.intents.len(), 1);
        assert_eq!(
            blocked.intents[0].blocking_fact,
            BlockingFact::ExclusiveFacilityUnavailable
        );
        assert_eq!(blocked.intents[0].related_entity, Some(facility));
        assert_eq!(blocked.intents[0].related_action, Some(ActionDefId(77)));
        assert!(runtime.queued_facility_intents.is_empty());
    }

    #[test]
    fn missing_queue_patience_profile_does_not_mark_runtime_dirty() {
        let agent = entity(1);
        let place = entity(2);
        let facility = entity(3);
        let view = QueuePatienceBeliefView {
            place: Some(place),
            facilities_at_place: vec![facility],
            queue_join_ticks: [(facility, Tick(1))].into_iter().collect(),
            patience_ticks: None,
            ..QueuePatienceBeliefView::default()
        };

        assert!(!facility_queue_patience_exhausted(&view, agent, Tick(10)));
    }

    #[test]
    fn grant_arrival_replan_can_select_direct_harvest_step() {
        let mut harness = build_exclusive_queue_harness();
        let harvest_action = harness
            .defs
            .iter()
            .find(|def| def.name == "harvest:Harvest Apples")
            .map(|def| def.id)
            .expect("harvest action should be registered");
        let mut txn = new_txn(&mut harness.world, 1);
        let mut queue = txn
            .get_component_facility_use_queue(harness.orchard_row)
            .cloned()
            .expect("exclusive orchard should have queue state");
        queue
            .enqueue(harness.actor, harvest_action, Tick(1))
            .unwrap();
        txn.set_component_facility_use_queue(harness.orchard_row, queue)
            .unwrap();
        commit_txn(txn);

        let mut runtime = active_runtime(GoalKind::Sleep);
        let initial_view = PerAgentBeliefView::from_world(harness.actor, &harness.world);
        update_runtime_observation_snapshot(&initial_view, harness.actor, &mut runtime);

        set_local_queue_state(
            &mut harness.world,
            harness.actor,
            harness.orchard_row,
            2,
            Some(harvest_action),
        );

        let mut blocked = BlockedIntentMemory::default();
        let _ = refresh_runtime_for_read_phase(
            &harness.world,
            &harness.scheduler,
            &harness.defs,
            &mut runtime,
            &mut blocked,
            harness.actor,
            &[],
            ReadPhaseContext {
                recipe_registry: &harness.recipes,
                utility: &UtilityProfile::default(),
                tick: Tick(2),
                travel_horizon: PlanningBudget::default().snapshot_travel_horizon,
                structural_block_ticks: PlanningBudget::default().structural_block_ticks,
            },
        );
        assert!(runtime.dirty);

        let goal = ranked_goal(
            GoalKind::RestockCommodity {
                commodity: CommodityKind::Apple,
            },
            [harness.orchard_row],
            [harness.orchard_farm],
        );
        let semantics = build_semantics_table(&harness.defs);
        let (next_step, next_step_valid) = plan_and_validate_next_step(
            &harness.world,
            &harness.scheduler,
            &mut runtime,
            harness.actor,
            std::slice::from_ref(&goal),
            &blocked,
            PlanningBudget::default().switch_margin_permille,
            PlanningBudget::default().switch_margin_permille,
            Tick(2),
            &PlanningBudget::default(),
            &semantics,
            &harness.defs,
            &harness.handlers,
        );

        assert_eq!(runtime.current_goal, Some(goal.grounded.key));
        assert_eq!(next_step_valid, Some(true));
        assert_eq!(
            next_step
                .expect("grant arrival should yield an executable exclusive step")
                .op_kind,
            PlannerOpKind::Harvest
        );
    }

    #[test]
    fn same_place_queue_invalidation_records_exclusive_facility_blocker() {
        let mut harness = Harness::new(ControlSource::Ai);
        let facility = add_local_queued_facility(&mut harness.world, harness.actor, 1);
        let goal = GoalKey::from(GoalKind::RestockCommodity {
            commodity: CommodityKind::Apple,
        });
        let mut runtime = crate::AgentDecisionRuntime {
            current_goal: Some(goal),
            ..crate::AgentDecisionRuntime::default()
        };
        runtime.queued_facility_intents.insert(
            facility,
            QueuedFacilityIntent {
                goal_key: goal,
                intended_action: ActionDefId(77),
            },
        );
        let initial_view = PerAgentBeliefView::from_world(harness.actor, &harness.world);
        update_runtime_observation_snapshot(&initial_view, harness.actor, &mut runtime);

        clear_local_queue_state(&mut harness.world, harness.actor, facility, 2);

        let mut blocked = BlockedIntentMemory::default();
        let _ = refresh_runtime_for_read_phase(
            &harness.world,
            &harness.scheduler,
            &harness.defs,
            &mut runtime,
            &mut blocked,
            harness.actor,
            &[],
            ReadPhaseContext {
                recipe_registry: &harness.recipes,
                utility: &UtilityProfile::default(),
                tick: Tick(2),
                travel_horizon: PlanningBudget::default().snapshot_travel_horizon,
                structural_block_ticks: PlanningBudget::default().structural_block_ticks,
            },
        );

        assert_eq!(blocked.intents.len(), 1);
        assert_eq!(
            blocked.intents[0].blocking_fact,
            BlockingFact::ExclusiveFacilityUnavailable
        );
        assert_eq!(blocked.intents[0].related_entity, Some(facility));
        assert_eq!(blocked.intents[0].related_action, Some(ActionDefId(77)));
        assert!(runtime.queued_facility_intents.is_empty());
    }

    #[test]
    fn grant_loss_does_not_record_hard_blocker() {
        let mut harness = Harness::new(ControlSource::Ai);
        let facility = add_local_queued_facility(&mut harness.world, harness.actor, 1);
        let goal = GoalKey::from(GoalKind::RestockCommodity {
            commodity: CommodityKind::Apple,
        });
        set_local_queue_state(
            &mut harness.world,
            harness.actor,
            facility,
            1,
            Some(ActionDefId(77)),
        );

        let mut runtime = crate::AgentDecisionRuntime {
            current_goal: Some(goal),
            ..crate::AgentDecisionRuntime::default()
        };
        runtime.queued_facility_intents.insert(
            facility,
            QueuedFacilityIntent {
                goal_key: goal,
                intended_action: ActionDefId(77),
            },
        );
        let initial_view = PerAgentBeliefView::from_world(harness.actor, &harness.world);
        update_runtime_observation_snapshot(&initial_view, harness.actor, &mut runtime);

        clear_local_queue_state(&mut harness.world, harness.actor, facility, 2);

        let mut blocked = BlockedIntentMemory::default();
        let _ = refresh_runtime_for_read_phase(
            &harness.world,
            &harness.scheduler,
            &harness.defs,
            &mut runtime,
            &mut blocked,
            harness.actor,
            &[],
            ReadPhaseContext {
                recipe_registry: &harness.recipes,
                utility: &UtilityProfile::default(),
                tick: Tick(2),
                travel_horizon: PlanningBudget::default().snapshot_travel_horizon,
                structural_block_ticks: PlanningBudget::default().structural_block_ticks,
            },
        );

        assert!(blocked.intents.is_empty());
        assert!(runtime.queued_facility_intents.is_empty());
    }

    #[test]
    fn queued_actor_can_eat_without_losing_queue_membership() {
        let mut harness = Harness::new(ControlSource::Ai);
        let facility = add_local_queued_facility(&mut harness.world, harness.actor, 1);

        let result = harness.step_once();

        assert_eq!(result.actions_started, 1);
        assert_eq!(harness.active_action_name(), Some("eat"));
        let queue = harness
            .world
            .get_component_facility_use_queue(facility)
            .expect("queued facility should still exist");
        assert!(queue
            .waiting
            .values()
            .any(|queued| queued.actor == harness.actor));
    }

    #[test]
    fn journey_snapshot_reports_profile_margin_source_for_active_journey() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let place = world.topology().place_ids().next().unwrap();
        let actor = {
            let mut txn = new_txn(&mut world, 1);
            let actor = txn.create_agent("Aster", ControlSource::Ai).unwrap();
            txn.set_ground_location(actor, place).unwrap();
            txn.set_component_travel_disposition_profile(
                actor,
                TravelDispositionProfile {
                    route_replan_margin: Permille::new(300).unwrap(),
                    blocked_leg_patience_ticks: std::num::NonZeroU32::new(4).unwrap(),
                },
            )
            .unwrap();
            commit_txn(txn);
            actor
        };
        let mut driver = AgentTickDriver::new(PlanningBudget::default());
        driver.runtime_by_agent.insert(
            actor,
            crate::AgentDecisionRuntime {
                current_plan: Some(PlannedPlan::new(
                    GoalKey::from(GoalKind::Sleep),
                    vec![travel_step(1, place)],
                    PlanTerminalKind::GoalSatisfied,
                )),
                journey_committed_goal: Some(GoalKey::from(GoalKind::Sleep)),
                journey_committed_destination: Some(place),
                journey_established_at: Some(Tick(7)),
                ..crate::AgentDecisionRuntime::default()
            },
        );

        let snapshot = driver.journey_snapshot(&world, actor).unwrap();

        assert_eq!(
            snapshot.switch_margin_source,
            JourneySwitchMarginSource::JourneyProfile
        );
        assert_eq!(
            snapshot.effective_switch_margin,
            Permille::new(300).unwrap()
        );
        assert_eq!(snapshot.runtime.committed_destination, Some(place));
        assert_eq!(snapshot.runtime.active_plan_destination, Some(place));
        assert!(snapshot.runtime.has_active_journey_travel);
    }

    #[test]
    fn journey_snapshot_reports_budget_margin_when_no_profile_override_applies() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let place = world.topology().place_ids().next().unwrap();
        let actor = {
            let mut txn = new_txn(&mut world, 1);
            let actor = txn.create_agent("Aster", ControlSource::Ai).unwrap();
            txn.set_ground_location(actor, place).unwrap();
            commit_txn(txn);
            actor
        };
        let budget = PlanningBudget::default();
        let mut driver = AgentTickDriver::new(budget.clone());
        driver.runtime_by_agent.insert(
            actor,
            crate::AgentDecisionRuntime {
                current_plan: Some(PlannedPlan::new(
                    GoalKey::from(GoalKind::Sleep),
                    vec![barrier_step()],
                    PlanTerminalKind::GoalSatisfied,
                )),
                ..crate::AgentDecisionRuntime::default()
            },
        );

        let snapshot = driver.journey_snapshot(&world, actor).unwrap();

        assert_eq!(
            snapshot.switch_margin_source,
            JourneySwitchMarginSource::BudgetDefault
        );
        assert_eq!(
            snapshot.effective_switch_margin,
            budget.switch_margin_permille
        );
        assert_eq!(snapshot.runtime.committed_destination, None);
        assert_eq!(snapshot.runtime.active_plan_destination, None);
        assert!(!snapshot.runtime.has_active_journey_travel);
    }

    #[test]
    fn travel_led_plan_adoption_sets_journey_commitment_anchor() {
        let goal = GoalKey::from(GoalKind::Sleep);
        let destination = entity(11);
        let plan = PlannedPlan::new(
            goal,
            vec![travel_step(1, destination), barrier_step()],
            PlanTerminalKind::GoalSatisfied,
        );
        let mut runtime = crate::AgentDecisionRuntime::default();

        update_journey_fields_for_adopted_plan(&mut runtime, &plan, Tick(9));

        assert_eq!(runtime.journey_committed_goal, Some(goal));
        assert_eq!(runtime.journey_committed_destination, Some(destination));
        assert_eq!(runtime.journey_established_at, Some(Tick(9)));
        assert_eq!(runtime.journey_last_progress_tick, None);
        assert_eq!(runtime.consecutive_blocked_leg_ticks, 0);
    }

    #[test]
    fn non_travel_plan_adoption_suspends_journey_commitment() {
        let goal = GoalKey::from(GoalKind::Sleep);
        let plan = PlannedPlan::new(goal, vec![barrier_step()], PlanTerminalKind::GoalSatisfied);
        let mut runtime = crate::AgentDecisionRuntime {
            journey_committed_goal: Some(goal),
            journey_committed_destination: Some(entity(12)),
            journey_established_at: Some(Tick(3)),
            journey_last_progress_tick: Some(Tick(7)),
            consecutive_blocked_leg_ticks: 2,
            ..crate::AgentDecisionRuntime::default()
        };

        update_journey_fields_for_adopted_plan(&mut runtime, &plan, Tick(9));

        assert_eq!(runtime.journey_committed_goal, Some(goal));
        assert_eq!(runtime.journey_committed_destination, Some(entity(12)));
        assert_eq!(
            runtime.journey_commitment_state,
            JourneyCommitmentState::Suspended
        );
        assert_eq!(runtime.journey_established_at, Some(Tick(3)));
        assert_eq!(runtime.journey_last_progress_tick, Some(Tick(7)));
        assert_eq!(runtime.consecutive_blocked_leg_ticks, 2);
        assert_eq!(runtime.last_journey_clear_reason, None);
    }

    #[test]
    fn same_goal_same_destination_replan_preserves_journey_commitment() {
        let goal = GoalKey::from(GoalKind::Sleep);
        let destination = entity(11);
        let plan = PlannedPlan::new(
            goal,
            vec![travel_step(1, destination), barrier_step()],
            PlanTerminalKind::GoalSatisfied,
        );
        let mut runtime = crate::AgentDecisionRuntime {
            current_goal: Some(goal),
            journey_committed_goal: Some(goal),
            journey_committed_destination: Some(destination),
            journey_established_at: Some(Tick(4)),
            journey_last_progress_tick: Some(Tick(6)),
            consecutive_blocked_leg_ticks: 3,
            ..crate::AgentDecisionRuntime::default()
        };

        update_journey_fields_for_adopted_plan(&mut runtime, &plan, Tick(9));

        assert_eq!(runtime.journey_committed_goal, Some(goal));
        assert_eq!(runtime.journey_committed_destination, Some(destination));
        assert_eq!(
            runtime.journey_commitment_state,
            JourneyCommitmentState::Active
        );
        assert_eq!(runtime.journey_established_at, Some(Tick(4)));
        assert_eq!(runtime.journey_last_progress_tick, Some(Tick(6)));
        assert_eq!(runtime.consecutive_blocked_leg_ticks, 3);
    }

    #[test]
    fn same_goal_different_destination_replan_restarts_journey_commitment() {
        let goal = GoalKey::from(GoalKind::Sleep);
        let original_destination = entity(11);
        let new_destination = entity(22);
        let plan = PlannedPlan::new(
            goal,
            vec![travel_step(1, new_destination), barrier_step()],
            PlanTerminalKind::GoalSatisfied,
        );
        let mut runtime = crate::AgentDecisionRuntime {
            current_goal: Some(goal),
            journey_committed_goal: Some(goal),
            journey_committed_destination: Some(original_destination),
            journey_established_at: Some(Tick(4)),
            journey_last_progress_tick: Some(Tick(6)),
            consecutive_blocked_leg_ticks: 3,
            ..crate::AgentDecisionRuntime::default()
        };

        update_journey_fields_for_adopted_plan(&mut runtime, &plan, Tick(9));

        assert_eq!(runtime.journey_committed_goal, Some(goal));
        assert_eq!(runtime.journey_committed_destination, Some(new_destination));
        assert_eq!(
            runtime.journey_commitment_state,
            JourneyCommitmentState::Active
        );
        assert_eq!(runtime.journey_established_at, Some(Tick(9)));
        assert_eq!(runtime.journey_last_progress_tick, None);
        assert_eq!(runtime.consecutive_blocked_leg_ticks, 0);
    }

    #[test]
    fn travel_leg_completion_updates_progress_tick_and_resets_blocked_counter() {
        let goal = GoalKey::from(GoalKind::Sleep);
        let mut runtime = crate::AgentDecisionRuntime {
            current_goal: Some(goal),
            current_plan: Some(PlannedPlan::new(
                goal,
                vec![travel_step(1, entity(11)), barrier_step()],
                PlanTerminalKind::GoalSatisfied,
            )),
            current_step_index: 0,
            journey_committed_goal: Some(goal),
            journey_committed_destination: Some(entity(11)),
            journey_established_at: Some(Tick(1)),
            consecutive_blocked_leg_ticks: 5,
            ..crate::AgentDecisionRuntime::default()
        };

        advance_completed_step(&mut runtime, PlannerOpKind::Travel, Tick(9));

        assert_eq!(runtime.current_step_index, 1);
        assert_eq!(runtime.journey_last_progress_tick, Some(Tick(9)));
        assert_eq!(runtime.consecutive_blocked_leg_ticks, 0);
    }

    #[test]
    fn recoverable_blocked_travel_step_increments_consecutive_blocked_ticks_and_forces_replan() {
        let goal = GoalKey::from(GoalKind::Sleep);
        let plan = PlannedPlan::new(
            goal,
            vec![travel_step(1, entity(11)), barrier_step()],
            PlanTerminalKind::GoalSatisfied,
        );
        let step = plan.steps[0].clone();
        let mut world = World::new(build_prototype_world()).unwrap();
        let place = world.topology().place_ids().next().unwrap();
        let actor = {
            let mut txn = new_txn(&mut world, 1);
            let actor = txn.create_agent("Aster", ControlSource::Ai).unwrap();
            txn.set_ground_location(actor, place).unwrap();
            txn.set_component_travel_disposition_profile(
                actor,
                TravelDispositionProfile {
                    route_replan_margin: Permille::new(300).unwrap(),
                    blocked_leg_patience_ticks: std::num::NonZeroU32::new(4).unwrap(),
                },
            )
            .unwrap();
            commit_txn(txn);
            actor
        };
        let view = PerAgentBeliefView::from_world(actor, &world);
        let mut runtime = crate::AgentDecisionRuntime {
            current_goal: Some(goal),
            current_plan: Some(plan.clone()),
            current_step_index: 0,
            journey_committed_goal: Some(goal),
            journey_committed_destination: Some(entity(11)),
            journey_established_at: Some(Tick(2)),
            consecutive_blocked_leg_ticks: 1,
            dirty: false,
            ..crate::AgentDecisionRuntime::default()
        };
        let mut blocked_memory = BlockedIntentMemory::default();

        assert!(handle_recoverable_travel_step_blockage(
            &view,
            &mut runtime,
            &mut blocked_memory,
            actor,
            &step,
            Tick(9),
            &PlanningBudget::default(),
        ));
        assert_eq!(runtime.consecutive_blocked_leg_ticks, 2);
        assert!(runtime.dirty);
        assert_eq!(runtime.current_goal, Some(goal));
        assert_eq!(runtime.journey_committed_goal, Some(goal));
        assert_eq!(runtime.journey_committed_destination, Some(entity(11)));
        assert_eq!(runtime.current_plan, None);
        assert_eq!(runtime.current_step_index, 0);
        assert!(blocked_memory.intents.is_empty());
        assert!(runtime
            .materialization_bindings
            .hypothetical_to_authoritative
            .is_empty());
    }

    #[test]
    fn blocked_leg_patience_exhaustion_clears_commitment_and_records_blocker() {
        let goal = GoalKey::from(GoalKind::Sleep);
        let destination = entity(11);
        let plan = PlannedPlan::new(
            goal,
            vec![travel_step(1, destination), barrier_step()],
            PlanTerminalKind::GoalSatisfied,
        );
        let step = plan.steps[0].clone();
        let mut world = World::new(build_prototype_world()).unwrap();
        let place = world.topology().place_ids().next().unwrap();
        let actor = {
            let mut txn = new_txn(&mut world, 1);
            let actor = txn.create_agent("Aster", ControlSource::Ai).unwrap();
            txn.set_ground_location(actor, place).unwrap();
            txn.set_component_travel_disposition_profile(
                actor,
                TravelDispositionProfile {
                    route_replan_margin: Permille::new(300).unwrap(),
                    blocked_leg_patience_ticks: std::num::NonZeroU32::new(2).unwrap(),
                },
            )
            .unwrap();
            commit_txn(txn);
            actor
        };
        let view = PerAgentBeliefView::from_world(actor, &world);
        let mut runtime = crate::AgentDecisionRuntime {
            current_goal: Some(goal),
            current_plan: Some(plan),
            current_step_index: 0,
            journey_committed_goal: Some(goal),
            journey_committed_destination: Some(destination),
            journey_established_at: Some(Tick(2)),
            journey_last_progress_tick: Some(Tick(4)),
            consecutive_blocked_leg_ticks: 1,
            dirty: false,
            ..crate::AgentDecisionRuntime::default()
        };
        let mut blocked_memory = BlockedIntentMemory::default();
        let budget = PlanningBudget::default();

        assert!(handle_recoverable_travel_step_blockage(
            &view,
            &mut runtime,
            &mut blocked_memory,
            actor,
            &step,
            Tick(9),
            &budget,
        ));
        assert_eq!(runtime.current_goal, Some(goal));
        assert_eq!(runtime.current_plan, None);
        assert_eq!(runtime.current_step_index, 0);
        assert!(runtime.dirty);
        assert_eq!(runtime.journey_committed_goal, None);
        assert_eq!(runtime.journey_committed_destination, None);
        assert_eq!(runtime.journey_established_at, None);
        assert_eq!(runtime.journey_last_progress_tick, None);
        assert_eq!(runtime.consecutive_blocked_leg_ticks, 0);
        assert_eq!(
            runtime.last_journey_clear_reason,
            Some(crate::JourneyClearReason::PatienceExhausted)
        );
        assert_eq!(blocked_memory.intents.len(), 1);
        assert_eq!(blocked_memory.intents[0].goal_key, goal);
        assert_eq!(
            blocked_memory.intents[0].blocking_fact,
            BlockingFact::NoKnownPath
        );
        assert_eq!(blocked_memory.intents[0].related_entity, None);
        assert_eq!(blocked_memory.intents[0].related_place, Some(destination));
        assert_eq!(blocked_memory.intents[0].observed_tick, Tick(9));
        assert_eq!(
            blocked_memory.intents[0].expires_tick,
            Tick(9 + u64::from(budget.structural_block_ticks))
        );
    }

    #[test]
    fn hungry_ai_agent_emits_request_and_starts_consume_action() {
        let mut harness = Harness::new(ControlSource::Ai);

        let result = harness.step_once();

        assert_eq!(result.inputs_processed, 1);
        assert_eq!(result.actions_started, 1);
        assert_eq!(harness.scheduler.active_actions().len(), 1);
        assert_eq!(
            harness
                .world
                .controlled_commodity_quantity(harness.actor, CommodityKind::Bread),
            Quantity(1)
        );
    }

    #[test]
    fn hungry_ai_agent_completes_consume_action_over_subsequent_ticks() {
        let mut harness = Harness::new(ControlSource::Ai);

        for _ in 0..8 {
            let _ = harness.step_once();
            if harness
                .world
                .controlled_commodity_quantity(harness.actor, CommodityKind::Bread)
                == Quantity(0)
            {
                break;
            }
        }

        assert_eq!(
            harness
                .world
                .controlled_commodity_quantity(harness.actor, CommodityKind::Bread),
            Quantity(0)
        );
    }

    #[test]
    fn human_controlled_agent_is_skipped_by_ai_driver() {
        let mut harness = Harness::new(ControlSource::Human);

        let result = harness.step_once();

        assert_eq!(result.inputs_processed, 0);
        assert_eq!(result.actions_started, 0);
        assert_eq!(
            harness
                .world
                .controlled_commodity_quantity(harness.actor, CommodityKind::Bread),
            Quantity(1)
        );
    }

    #[test]
    fn dead_ai_agent_is_skipped_by_ai_driver() {
        let mut harness = Harness::new(ControlSource::Ai);
        harness.driver.runtime_by_agent.insert(
            harness.actor,
            crate::AgentDecisionRuntime {
                journey_committed_goal: Some(GoalKey::from(GoalKind::Sleep)),
                journey_committed_destination: Some(entity(11)),
                journey_established_at: Some(Tick(1)),
                ..crate::AgentDecisionRuntime::default()
            },
        );
        {
            let mut txn = new_txn(&mut harness.world, 2);
            txn.set_component_dead_at(harness.actor, worldwake_core::DeadAt(Tick(2)))
                .unwrap();
            let _ = txn.commit(&mut harness.event_log);
        }

        let result = harness.step_once();

        assert_eq!(result.inputs_processed, 0);
        assert_eq!(result.actions_started, 0);
        assert_eq!(
            harness
                .world
                .controlled_commodity_quantity(harness.actor, CommodityKind::Bread),
            Quantity(1)
        );
        assert_eq!(
            harness.runtime().unwrap().last_journey_clear_reason,
            Some(crate::JourneyClearReason::Death)
        );
    }

    #[test]
    fn progress_barrier_completion_preserves_goal_and_forces_replan() {
        let goal = GoalKey::from(GoalKind::AcquireCommodity {
            commodity: CommodityKind::Bread,
            purpose: CommodityPurpose::SelfConsume,
        });
        let destination = entity(11);
        let mut runtime = crate::AgentDecisionRuntime {
            current_goal: Some(goal),
            current_plan: Some(PlannedPlan::new(
                goal,
                vec![travel_step(1, destination)],
                PlanTerminalKind::ProgressBarrier,
            )),
            current_step_index: 0,
            journey_committed_goal: Some(goal),
            journey_committed_destination: Some(destination),
            journey_established_at: Some(Tick(1)),
            step_in_flight: false,
            dirty: false,
            ..crate::AgentDecisionRuntime::default()
        };

        advance_completed_step(&mut runtime, PlannerOpKind::Travel, Tick(4));

        assert_eq!(runtime.current_goal, Some(goal));
        assert_eq!(runtime.current_plan, None);
        assert_eq!(runtime.current_step_index, 0);
        assert_eq!(runtime.journey_committed_goal, Some(goal));
        assert_eq!(runtime.journey_committed_destination, Some(destination));
        assert_eq!(runtime.journey_last_progress_tick, Some(Tick(4)));
        assert!(runtime.dirty);
        assert!(runtime
            .materialization_bindings
            .hypothetical_to_authoritative
            .is_empty());
    }

    #[test]
    fn suspended_detour_completion_preserves_commitment_and_reactivates_it() {
        let committed_goal = GoalKey::from(GoalKind::AcquireCommodity {
            commodity: CommodityKind::Bread,
            purpose: CommodityPurpose::SelfConsume,
        });
        let detour_goal = GoalKey::from(GoalKind::ConsumeOwnedCommodity {
            commodity: CommodityKind::Water,
        });
        let destination = entity(11);
        let mut runtime = crate::AgentDecisionRuntime {
            current_goal: Some(detour_goal),
            current_plan: Some(PlannedPlan::new(
                detour_goal,
                vec![PlannedStep {
                    def_id: ActionDefId(9),
                    targets: vec![PlanningEntityRef::Authoritative(entity(12))],
                    payload_override: None,
                    op_kind: PlannerOpKind::Consume,
                    estimated_ticks: 1,
                    is_materialization_barrier: false,
                    expected_materializations: Vec::new(),
                }],
                PlanTerminalKind::GoalSatisfied,
            )),
            current_step_index: 0,
            journey_committed_goal: Some(committed_goal),
            journey_committed_destination: Some(destination),
            journey_commitment_state: JourneyCommitmentState::Suspended,
            journey_established_at: Some(Tick(1)),
            journey_last_progress_tick: Some(Tick(3)),
            step_in_flight: false,
            dirty: false,
            ..crate::AgentDecisionRuntime::default()
        };

        advance_completed_step(&mut runtime, PlannerOpKind::Consume, Tick(4));

        assert_eq!(runtime.current_goal, None);
        assert_eq!(runtime.current_plan, None);
        assert_eq!(runtime.current_step_index, 0);
        assert_eq!(runtime.journey_committed_goal, Some(committed_goal));
        assert_eq!(runtime.journey_committed_destination, Some(destination));
        assert_eq!(
            runtime.journey_commitment_state,
            JourneyCommitmentState::Active
        );
        assert_eq!(runtime.journey_established_at, Some(Tick(1)));
        assert_eq!(runtime.journey_last_progress_tick, Some(Tick(3)));
        assert_eq!(runtime.last_journey_clear_reason, None);
        assert!(runtime.dirty);
    }

    #[test]
    fn goal_completion_records_goal_satisfied_clear_reason() {
        let goal = GoalKey::from(GoalKind::Sleep);
        let destination = entity(11);
        let mut runtime = crate::AgentDecisionRuntime {
            current_goal: Some(goal),
            current_plan: Some(PlannedPlan::new(
                goal,
                vec![travel_step(1, destination)],
                PlanTerminalKind::GoalSatisfied,
            )),
            current_step_index: 0,
            journey_committed_goal: Some(goal),
            journey_committed_destination: Some(destination),
            journey_established_at: Some(Tick(1)),
            ..crate::AgentDecisionRuntime::default()
        };

        advance_completed_step(&mut runtime, PlannerOpKind::Travel, Tick(4));

        assert_eq!(
            runtime.last_journey_clear_reason,
            Some(crate::JourneyClearReason::GoalSatisfied)
        );
        assert_eq!(runtime.journey_committed_goal, None);
        assert_eq!(runtime.journey_committed_destination, None);
    }

    #[test]
    fn apply_step_materialization_bindings_binds_expected_outputs() {
        let mut runtime = crate::AgentDecisionRuntime::default();
        let step = hypothetical_step(4, 7);
        let created = entity(21);
        let outcome = CommitOutcome {
            materializations: vec![Materialization {
                tag: MaterializationTag::SplitOffLot,
                entity: created,
            }],
        };

        apply_step_materialization_bindings(&mut runtime, &step, &outcome).unwrap();

        assert_eq!(
            runtime
                .materialization_bindings
                .resolve(crate::HypotheticalEntityId(7)),
            Some(created)
        );
    }

    #[test]
    fn apply_step_materialization_bindings_rejects_mismatched_counts() {
        let mut runtime = crate::AgentDecisionRuntime::default();
        let step = hypothetical_step(4, 7);

        assert!(
            apply_step_materialization_bindings(&mut runtime, &step, &CommitOutcome::empty())
                .is_err()
        );
    }

    #[test]
    fn resolve_step_targets_uses_materialization_bindings_for_hypothetical_refs() {
        let mut runtime = crate::AgentDecisionRuntime::default();
        let step = hypothetical_step(4, 7);
        let created = entity(21);
        runtime
            .materialization_bindings
            .bind(crate::HypotheticalEntityId(7), created);

        assert_eq!(resolve_step_targets(&runtime, &step), Some(vec![created]));
    }

    #[test]
    fn committed_action_for_step_requires_single_matching_def() {
        let step = barrier_step();
        let matching = CommittedAction {
            actor: entity(1),
            def_id: step.def_id,
            instance_id: worldwake_sim::ActionInstanceId(4),
            tick: Tick(9),
            outcome: CommitOutcome::empty(),
        };
        let mismatched = CommittedAction {
            def_id: ActionDefId(99),
            ..matching.clone()
        };

        assert_eq!(
            committed_action_for_step(&step, std::slice::from_ref(&matching)),
            Some(&matching)
        );
        assert_eq!(committed_action_for_step(&step, &[]), None);
        assert_eq!(
            committed_action_for_step(&step, &[matching.clone(), mismatched.clone()]),
            None
        );
        assert_eq!(
            committed_action_for_step(&step, std::slice::from_ref(&mismatched)),
            None
        );
    }

    #[test]
    fn materialized_pickup_binding_survives_intervening_travel_until_put_down_resolution() {
        let hypothetical_id = crate::HypotheticalEntityId(0);
        let created = entity(42);
        let goal = GoalKey::from(GoalKind::MoveCargo {
            commodity: CommodityKind::Bread,
            destination: entity(22),
        });
        let plan = PlannedPlan::new(
            goal,
            vec![
                PlannedStep {
                    def_id: ActionDefId(4),
                    targets: vec![PlanningEntityRef::Authoritative(entity(11))],
                    payload_override: None,
                    op_kind: PlannerOpKind::MoveCargo,
                    estimated_ticks: 1,
                    is_materialization_barrier: false,
                    expected_materializations: vec![ExpectedMaterialization {
                        tag: MaterializationTag::SplitOffLot,
                        hypothetical_id,
                    }],
                },
                PlannedStep {
                    def_id: ActionDefId(5),
                    targets: vec![PlanningEntityRef::Authoritative(entity(22))],
                    payload_override: None,
                    op_kind: PlannerOpKind::Travel,
                    estimated_ticks: 2,
                    is_materialization_barrier: false,
                    expected_materializations: Vec::new(),
                },
                PlannedStep {
                    def_id: ActionDefId(6),
                    targets: vec![PlanningEntityRef::Hypothetical(hypothetical_id)],
                    payload_override: None,
                    op_kind: PlannerOpKind::MoveCargo,
                    estimated_ticks: 1,
                    is_materialization_barrier: false,
                    expected_materializations: Vec::new(),
                },
            ],
            PlanTerminalKind::GoalSatisfied,
        );
        let mut runtime = crate::AgentDecisionRuntime {
            current_goal: Some(goal),
            current_plan: Some(plan.clone()),
            current_step_index: 0,
            step_in_flight: true,
            dirty: false,
            ..crate::AgentDecisionRuntime::default()
        };

        apply_step_materialization_bindings(
            &mut runtime,
            &plan.steps[0],
            &CommitOutcome {
                materializations: vec![Materialization {
                    tag: MaterializationTag::SplitOffLot,
                    entity: created,
                }],
            },
        )
        .unwrap();
        runtime.step_in_flight = false;
        advance_completed_step(&mut runtime, PlannerOpKind::MoveCargo, Tick(3));

        assert_eq!(runtime.current_step_index, 1);
        assert_eq!(
            runtime.materialization_bindings.resolve(hypothetical_id),
            Some(created)
        );

        runtime.step_in_flight = true;
        apply_step_materialization_bindings(&mut runtime, &plan.steps[1], &CommitOutcome::empty())
            .unwrap();
        runtime.step_in_flight = false;
        advance_completed_step(&mut runtime, PlannerOpKind::Travel, Tick(4));

        assert_eq!(runtime.current_step_index, 2);
        assert_eq!(
            resolve_step_targets(&runtime, &plan.steps[2]),
            Some(vec![created])
        );

        runtime.step_in_flight = true;
        apply_step_materialization_bindings(&mut runtime, &plan.steps[2], &CommitOutcome::empty())
            .unwrap();
        runtime.step_in_flight = false;
        advance_completed_step(&mut runtime, PlannerOpKind::MoveCargo, Tick(5));

        assert!(runtime.current_plan.is_none());
        assert!(!runtime.step_in_flight);
        assert!(runtime
            .materialization_bindings
            .hypothetical_to_authoritative
            .is_empty());
    }

    #[allow(clippy::too_many_lines)]
    #[test]
    fn goal_stability_across_cargo_replan_after_materialization() {
        let (mut harness, original_lot, origin, destination) = cargo_harness(false);
        let expected_goal = GoalKey::from(GoalKind::MoveCargo {
            commodity: CommodityKind::Bread,
            destination,
        });
        let budget = PlanningBudget {
            max_plan_depth: 2,
            ..PlanningBudget::default()
        };
        let semantics = crate::build_semantics_table(&harness.defs);
        let view = PerAgentBeliefView::from_world(harness.actor, &harness.world);
        let grounded = crate::generate_candidates(
            &view,
            harness.actor,
            &BlockedIntentMemory::default(),
            &harness.recipes,
            Tick(0),
        )
        .into_iter()
        .find(|candidate| candidate.key == expected_goal)
        .expect("owned ground lot with home-market demand should emit MoveCargo");
        assert_eq!(
            grounded.evidence_entities,
            [original_lot].into_iter().collect()
        );
        assert_eq!(
            grounded.evidence_places,
            [origin, destination].into_iter().collect()
        );
        let snapshot = crate::build_planning_snapshot(
            &view,
            harness.actor,
            &grounded.evidence_entities,
            &grounded.evidence_places,
            1,
        );
        let planning_state = crate::PlanningState::new(&snapshot);
        let planning_affordances = worldwake_sim::get_affordances(
            &planning_state,
            harness.actor,
            &harness.defs,
            &harness.handlers,
        );
        assert!(
            planning_affordances.iter().any(|affordance| {
                harness
                    .defs
                    .get(affordance.def_id)
                    .is_some_and(|def| def.name == "pick_up")
            }),
            "planning state should expose pick_up affordance for owned ground cargo"
        );
        let plan = crate::search_plan(
            &snapshot,
            &grounded,
            &semantics,
            &harness.defs,
            &harness.handlers,
            &budget,
        );
        assert!(
            plan.is_some(),
            "partial cargo pickup should be plannable before runtime continuity is asserted"
        );

        let mut blocked = BlockedIntentMemory::default();
        let utility = harness
            .world
            .get_component_utility_profile(harness.actor)
            .cloned()
            .unwrap_or_default();
        let runtime = harness
            .driver
            .runtime_by_agent
            .entry(harness.actor)
            .or_default();
        let ranked = refresh_runtime_for_read_phase(
            &harness.world,
            &harness.scheduler,
            &harness.defs,
            runtime,
            &mut blocked,
            harness.actor,
            &[],
            ReadPhaseContext {
                recipe_registry: &harness.recipes,
                utility: &utility,
                tick: Tick(1),
                travel_horizon: budget.snapshot_travel_horizon,
                structural_block_ticks: budget.structural_block_ticks,
            },
        );
        let (next_step, next_step_valid) = plan_and_validate_next_step(
            &harness.world,
            &harness.scheduler,
            runtime,
            harness.actor,
            &ranked,
            &blocked,
            budget.switch_margin_permille,
            budget.switch_margin_permille,
            Tick(1),
            &budget,
            &semantics,
            &harness.defs,
            &harness.handlers,
        );
        let pick_up = next_step.expect("cargo runtime should choose an initial pick_up step");
        assert_eq!(runtime.current_goal, Some(expected_goal));
        assert_eq!(pick_up.op_kind, PlannerOpKind::MoveCargo);
        assert_eq!(
            pick_up.targets,
            vec![PlanningEntityRef::Authoritative(original_lot)]
        );
        assert_eq!(next_step_valid, Some(true));

        update_runtime_observation_snapshot(&view, harness.actor, runtime);

        let carried_water = {
            let mut txn = new_txn(&mut harness.world, 2);
            let (_, split_off) = txn.split_lot(original_lot, Quantity(2)).unwrap();
            txn.set_ground_location(split_off, origin).unwrap();
            txn.set_possessor(split_off, harness.actor).unwrap();
            commit_txn(txn);
            split_off
        };
        assert_eq!(
            harness
                .world
                .get_component_item_lot(original_lot)
                .unwrap()
                .quantity,
            Quantity(1)
        );
        assert_eq!(
            harness.world.possessor_of(carried_water),
            Some(harness.actor)
        );
        assert_eq!(harness.world.effective_place(carried_water), Some(origin));
        assert_eq!(
            harness
                .world
                .get_component_item_lot(carried_water)
                .unwrap()
                .quantity,
            Quantity(2)
        );
        sync_all_beliefs(&mut harness.world, harness.actor, Tick(2));

        runtime.step_in_flight = true;
        apply_step_materialization_bindings(
            runtime,
            &pick_up,
            &CommitOutcome {
                materializations: vec![Materialization {
                    tag: MaterializationTag::SplitOffLot,
                    entity: carried_water,
                }],
            },
        )
        .unwrap();
        runtime.step_in_flight = false;
        advance_completed_step(runtime, PlannerOpKind::MoveCargo, Tick(2));
        assert_eq!(runtime.current_goal, Some(expected_goal));

        let ranked_after_pickup = refresh_runtime_for_read_phase(
            &harness.world,
            &harness.scheduler,
            &harness.defs,
            runtime,
            &mut blocked,
            harness.actor,
            &[],
            ReadPhaseContext {
                recipe_registry: &harness.recipes,
                utility: &utility,
                tick: Tick(2),
                travel_horizon: budget.snapshot_travel_horizon,
                structural_block_ticks: budget.structural_block_ticks,
            },
        );
        assert!(runtime.dirty);
        let (next_step, next_step_valid) = plan_and_validate_next_step(
            &harness.world,
            &harness.scheduler,
            runtime,
            harness.actor,
            &ranked_after_pickup,
            &blocked,
            budget.switch_margin_permille,
            budget.switch_margin_permille,
            Tick(2),
            &budget,
            &semantics,
            &harness.defs,
            &harness.handlers,
        );
        let travel = next_step.expect("dirty cargo runtime should continue planning the same goal");
        assert_eq!(runtime.current_goal, Some(expected_goal));
        assert!(matches!(
            travel.op_kind,
            PlannerOpKind::Travel | PlannerOpKind::MoveCargo
        ));
        assert_eq!(next_step_valid, Some(true));
    }

    #[test]
    fn irrelevant_commodity_change_does_not_trigger_replan_for_sleep_goal() {
        let mut harness = Harness::new(ControlSource::Ai);
        let utility = harness
            .world
            .get_component_utility_profile(harness.actor)
            .cloned()
            .unwrap_or_default();
        let runtime = harness
            .driver
            .runtime_by_agent
            .entry(harness.actor)
            .or_insert_with(|| active_runtime(GoalKind::Sleep));
        let view = PerAgentBeliefView::from_world(harness.actor, &harness.world);
        update_runtime_observation_snapshot(&view, harness.actor, runtime);

        {
            let place = harness.world.effective_place(harness.actor).unwrap();
            let mut txn = new_txn(&mut harness.world, 2);
            let coin = txn
                .create_item_lot(CommodityKind::Coin, Quantity(1))
                .unwrap();
            txn.set_ground_location(coin, place).unwrap();
            txn.set_possessor(coin, harness.actor).unwrap();
            commit_txn(txn);
        }

        let mut blocked = BlockedIntentMemory::default();
        let _ = refresh_runtime_for_read_phase(
            &harness.world,
            &harness.scheduler,
            &harness.defs,
            runtime,
            &mut blocked,
            harness.actor,
            &[],
            ReadPhaseContext {
                recipe_registry: &harness.recipes,
                utility: &utility,
                tick: Tick(2),
                travel_horizon: PlanningBudget::default().snapshot_travel_horizon,
                structural_block_ticks: PlanningBudget::default().structural_block_ticks,
            },
        );

        assert!(!runtime.dirty);
    }

    #[test]
    fn relevant_commodity_change_triggers_replan_for_consume_goal() {
        let mut harness = Harness::new(ControlSource::Ai);
        let utility = harness
            .world
            .get_component_utility_profile(harness.actor)
            .cloned()
            .unwrap_or_default();
        let runtime = harness
            .driver
            .runtime_by_agent
            .entry(harness.actor)
            .or_insert_with(|| {
                active_runtime(GoalKind::ConsumeOwnedCommodity {
                    commodity: CommodityKind::Bread,
                })
            });
        let view = PerAgentBeliefView::from_world(harness.actor, &harness.world);
        update_runtime_observation_snapshot(&view, harness.actor, runtime);

        {
            let place = harness.world.effective_place(harness.actor).unwrap();
            let mut txn = new_txn(&mut harness.world, 2);
            let bread = txn
                .create_item_lot(CommodityKind::Bread, Quantity(1))
                .unwrap();
            txn.set_ground_location(bread, place).unwrap();
            txn.set_possessor(bread, harness.actor).unwrap();
            commit_txn(txn);
        }

        let mut blocked = BlockedIntentMemory::default();
        let _ = refresh_runtime_for_read_phase(
            &harness.world,
            &harness.scheduler,
            &harness.defs,
            runtime,
            &mut blocked,
            harness.actor,
            &[],
            ReadPhaseContext {
                recipe_registry: &harness.recipes,
                utility: &utility,
                tick: Tick(2),
                travel_horizon: PlanningBudget::default().snapshot_travel_horizon,
                structural_block_ticks: PlanningBudget::default().structural_block_ticks,
            },
        );

        assert!(runtime.dirty);
    }

    #[test]
    fn no_plan_always_marks_runtime_dirty() {
        let harness = Harness::new(ControlSource::Ai);
        let utility = harness
            .world
            .get_component_utility_profile(harness.actor)
            .cloned()
            .unwrap_or_default();
        let mut runtime = crate::AgentDecisionRuntime::default();
        let view = PerAgentBeliefView::from_world(harness.actor, &harness.world);
        update_runtime_observation_snapshot(&view, harness.actor, &mut runtime);
        let mut blocked = BlockedIntentMemory::default();

        let _ = refresh_runtime_for_read_phase(
            &harness.world,
            &harness.scheduler,
            &harness.defs,
            &mut runtime,
            &mut blocked,
            harness.actor,
            &[],
            ReadPhaseContext {
                recipe_registry: &harness.recipes,
                utility: &utility,
                tick: Tick(1),
                travel_horizon: PlanningBudget::default().snapshot_travel_horizon,
                structural_block_ticks: PlanningBudget::default().structural_block_ticks,
            },
        );

        assert!(runtime.dirty);
    }

    #[test]
    fn same_place_perception_seeds_seller_belief_for_runtime_candidates() {
        let (mut harness, seller, origin, _destination) = hungry_acquisition_harness();

        let before = ranked_goals_at(&mut harness, Tick(1));
        assert!(!has_goal(
            &before,
            GoalKind::AcquireCommodity {
                commodity: CommodityKind::Bread,
                purpose: CommodityPurpose::SelfConsume,
            }
        ));
        assert!(harness
            .world
            .get_component_agent_belief_store(harness.actor)
            .unwrap()
            .get_entity(&seller)
            .is_none());

        run_same_place_observation(&mut harness, Tick(2), origin, seller);

        let belief = harness
            .world
            .get_component_agent_belief_store(harness.actor)
            .unwrap()
            .get_entity(&seller)
            .cloned()
            .expect("perception should seed a direct observation for the seller");
        assert_eq!(belief.last_known_place, Some(origin));
        assert!(belief.alive);
        assert_eq!(belief.source, PerceptionSource::DirectObservation);

        let after = ranked_goals_at(&mut harness, Tick(2));
        assert!(has_goal(
            &after,
            GoalKind::AcquireCommodity {
                commodity: CommodityKind::Bread,
                purpose: CommodityPurpose::SelfConsume,
            }
        ));
    }

    #[test]
    fn unseen_seller_relocation_preserves_stale_acquisition_belief() {
        let (mut harness, seller, origin, destination) = hungry_acquisition_harness();
        run_same_place_observation(&mut harness, Tick(2), origin, seller);

        relocate_entity(&mut harness.world, seller, destination, Tick(3));

        let view = PerAgentBeliefView::from_world(harness.actor, &harness.world);
        assert_eq!(harness.world.effective_place(seller), Some(destination));
        assert_eq!(view.effective_place(seller), Some(origin));

        let ranked = ranked_goals_at(&mut harness, Tick(3));
        assert!(has_goal(
            &ranked,
            GoalKind::AcquireCommodity {
                commodity: CommodityKind::Bread,
                purpose: CommodityPurpose::SelfConsume,
            }
        ));
    }

    #[test]
    fn unseen_death_does_not_create_corpse_reaction_without_reobservation() {
        let (mut harness, seller, origin, destination) = hungry_acquisition_harness();
        run_same_place_observation(&mut harness, Tick(2), origin, seller);

        relocate_entity(&mut harness.world, seller, destination, Tick(3));
        kill_entity(&mut harness.world, seller, Tick(3));

        let view = PerAgentBeliefView::from_world(harness.actor, &harness.world);
        assert!(harness.world.get_component_dead_at(seller).is_some());
        assert!(!view.is_dead(seller));
        assert!(view.is_alive(seller));
        assert!(view.corpse_entities_at(origin).is_empty());

        let ranked = ranked_goals_at(&mut harness, Tick(3));
        assert!(!ranked.iter().any(|candidate| {
            matches!(
                candidate.grounded.key.kind,
                GoalKind::LootCorpse { corpse } if corpse == seller
            )
        }));
        assert!(!ranked.iter().any(|candidate| {
            matches!(
                candidate.grounded.key.kind,
                GoalKind::BuryCorpse { corpse, .. } if corpse == seller
            )
        }));
    }

    #[test]
    fn expired_remote_acquisition_belief_remains_until_perception_refresh() {
        let (mut harness, seller, _local_witness, _origin, destination) =
            stale_remote_acquisition_harness();

        let before = ranked_goals_at(&mut harness, Tick(1));
        assert!(has_goal(
            &before,
            GoalKind::AcquireCommodity {
                commodity: CommodityKind::Bread,
                purpose: CommodityPurpose::SelfConsume,
            }
        ));
        assert_eq!(
            harness
                .world
                .get_component_agent_belief_store(harness.actor)
                .unwrap()
                .get_entity(&seller)
                .and_then(|belief| belief.last_known_place),
            Some(destination)
        );

        let after_retention_without_refresh = ranked_goals_at(&mut harness, Tick(10));
        assert!(has_goal(
            &after_retention_without_refresh,
            GoalKind::AcquireCommodity {
                commodity: CommodityKind::Bread,
                purpose: CommodityPurpose::SelfConsume,
            }
        ));
        assert!(
            harness
                .world
                .get_component_agent_belief_store(harness.actor)
                .unwrap()
                .get_entity(&seller)
                .is_some(),
            "belief retention is enforced during perception refresh, not by ranked_goals_at alone"
        );
    }

    #[test]
    fn perception_refresh_evicts_expired_remote_acquisition_belief_and_removes_goal() {
        let (mut harness, seller, local_witness, origin, destination) =
            stale_remote_acquisition_harness();

        let before = ranked_goals_at(&mut harness, Tick(1));
        assert!(has_goal(
            &before,
            GoalKind::AcquireCommodity {
                commodity: CommodityKind::Bread,
                purpose: CommodityPurpose::SelfConsume,
            }
        ));
        assert_eq!(
            harness
                .world
                .get_component_agent_belief_store(harness.actor)
                .unwrap()
                .get_entity(&seller)
                .and_then(|belief| belief.last_known_place),
            Some(destination)
        );

        run_perception_tick(&mut harness, Tick(10));

        let store = harness
            .world
            .get_component_agent_belief_store(harness.actor)
            .unwrap();
        assert!(
            store.get_entity(&seller).is_none(),
            "expired remote seller belief should be evicted on a later perception refresh"
        );
        let local_belief = store
            .get_entity(&local_witness)
            .expect("same-place witness should be observed during refresh");
        assert_eq!(local_belief.last_known_place, Some(origin));

        let after = ranked_goals_at(&mut harness, Tick(10));
        assert!(
            !has_goal(
                &after,
                GoalKind::AcquireCommodity {
                    commodity: CommodityKind::Bread,
                    purpose: CommodityPurpose::SelfConsume,
                }
            ),
            "once retention enforcement prunes the stale remote seller, the acquire goal must disappear"
        );
    }

    #[test]
    fn cargo_satisfaction_at_destination_while_carrying() {
        let (mut harness, remote_lot, _origin, destination) = cargo_harness(true);

        let _ = harness.step_once();
        assert_eq!(
            harness.runtime().unwrap().current_goal,
            Some(GoalKey::from(GoalKind::MoveCargo {
                commodity: CommodityKind::Bread,
                destination,
            }))
        );

        step_until(&mut harness, 8, |state| {
            state.world.effective_place(state.actor) == Some(destination)
                && state.scheduler.active_actions().is_empty()
        });

        let result = harness.step_once();

        assert_eq!(result.actions_started, 0);
        assert_eq!(harness.world.possessor_of(remote_lot), Some(harness.actor));
        assert_eq!(harness.world.effective_place(remote_lot), Some(destination));
        assert_eq!(harness.runtime().unwrap().current_goal, None);
        assert!(harness.runtime().unwrap().current_plan.is_none());
        assert_eq!(harness.active_action_name(), None);
    }

    #[test]
    fn merchant_restock_requires_delivery_to_home_market() {
        let (mut harness, remote_lot, origin, destination) = cargo_harness(true);

        assert_eq!(harness.world.possessor_of(remote_lot), Some(harness.actor));
        assert_eq!(harness.world.effective_place(remote_lot), Some(origin));
        assert_ne!(origin, destination);

        let result = harness.step_once();
        assert_eq!(result.actions_started, 1);

        assert_eq!(
            harness.runtime().unwrap().current_goal,
            Some(GoalKey::from(GoalKind::MoveCargo {
                commodity: CommodityKind::Bread,
                destination,
            }))
        );
        assert!(
            harness.world.is_in_transit(harness.actor)
                || harness.world.effective_place(remote_lot) == Some(destination)
        );
    }

    #[test]
    fn persist_blocked_memory_skips_empty_unchanged_state() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let mut event_log = EventLog::new();
        let place = world.topology().place_ids().next().unwrap();
        let agent = {
            let mut txn = new_txn(&mut world, 1);
            let agent = txn.create_agent("Aster", ControlSource::Ai).unwrap();
            txn.set_ground_location(agent, place).unwrap();
            let _ = txn.commit(&mut event_log);
            agent
        };

        persist_blocked_memory(
            &mut world,
            &mut event_log,
            agent,
            Tick(2),
            &BlockedIntentMemory::default(),
            &BlockedIntentMemory::default(),
        )
        .unwrap();

        assert_eq!(world.get_component_blocked_intent_memory(agent), None);
        assert_eq!(event_log.len(), 1);
    }

    #[test]
    fn persist_blocked_memory_commits_changed_component() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let mut event_log = EventLog::new();
        let place = world.topology().place_ids().next().unwrap();
        let agent = {
            let mut txn = new_txn(&mut world, 1);
            let agent = txn.create_agent("Aster", ControlSource::Ai).unwrap();
            txn.set_ground_location(agent, place).unwrap();
            let _ = txn.commit(&mut event_log);
            agent
        };
        let blocked = BlockedIntentMemory {
            intents: vec![BlockedIntent {
                goal_key: GoalKey::from(GoalKind::Sleep),
                blocking_fact: BlockingFact::Unknown,
                related_entity: None,
                related_place: None,
                related_action: None,
                observed_tick: Tick(2),
                expires_tick: Tick(7),
            }],
        };

        persist_blocked_memory(
            &mut world,
            &mut event_log,
            agent,
            Tick(2),
            &BlockedIntentMemory::default(),
            &blocked,
        )
        .unwrap();

        assert_eq!(
            world.get_component_blocked_intent_memory(agent),
            Some(&blocked)
        );
        assert_eq!(event_log.len(), 2);
    }

    #[test]
    fn belief_read_modules_do_not_depend_on_world_directly() {
        let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(|path| path.parent())
            .expect("workspace layout should place crate under crates/")
            .to_path_buf();
        let modules = [
            "crates/worldwake-ai/src/candidate_generation.rs",
            "crates/worldwake-ai/src/enterprise.rs",
            "crates/worldwake-ai/src/failure_handling.rs",
            "crates/worldwake-ai/src/plan_revalidation.rs",
            "crates/worldwake-ai/src/planning_snapshot.rs",
            "crates/worldwake-ai/src/planning_state.rs",
            "crates/worldwake-ai/src/pressure.rs",
            "crates/worldwake-ai/src/ranking.rs",
            "crates/worldwake-ai/src/search.rs",
        ];

        for relative in modules {
            let source = fs::read_to_string(repo_root.join(relative))
                .unwrap_or_else(|error| panic!("failed to read {relative}: {error}"));
            let production_source = source
                .split("\n#[cfg(test)]")
                .next()
                .expect("split always returns at least one segment");
            assert!(
                !production_source.contains("worldwake_core::World"),
                "{relative} should read through RuntimeBeliefView instead of depending on World"
            );
            assert!(
                !production_source.contains("&World"),
                "{relative} should not take &World directly"
            );
            assert!(
                !production_source.contains("WorldTxn"),
                "{relative} should not mutate authoritative state directly"
            );
        }
    }

    #[test]
    fn goal_read_modules_use_goal_belief_view_boundary() {
        let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(|path| path.parent())
            .expect("workspace layout should place crate under crates/")
            .to_path_buf();
        let modules = [
            "crates/worldwake-ai/src/candidate_generation.rs",
            "crates/worldwake-ai/src/enterprise.rs",
            "crates/worldwake-ai/src/goal_explanation.rs",
            "crates/worldwake-ai/src/pressure.rs",
            "crates/worldwake-ai/src/ranking.rs",
        ];

        for relative in modules {
            let source = fs::read_to_string(repo_root.join(relative))
                .unwrap_or_else(|error| panic!("failed to read {relative}: {error}"));
            let production_source = source
                .split("\n#[cfg(test)]")
                .next()
                .expect("split always returns at least one segment");
            assert!(
                production_source.contains("GoalBeliefView"),
                "{relative} should compile against GoalBeliefView"
            );
            assert!(
                !production_source.contains("&dyn RuntimeBeliefView"),
                "{relative} should not depend on the broad RuntimeBeliefView boundary"
            );
        }
    }
}
