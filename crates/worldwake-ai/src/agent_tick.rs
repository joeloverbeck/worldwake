use crate::{
    build_planning_snapshot, build_semantics_table, clear_resolved_blockers, evaluate_interrupt,
    generate_candidates, handle_plan_failure, rank_candidates, resolve_planning_targets_with,
    revalidate_next_step, search_plan, select_best_plan, AgentDecisionRuntime, GoalKindPlannerExt,
    InterruptDecision, PlanFailureContext, PlanTerminalKind, PlannedStep, PlannerOpSemantics,
    PlanningBudget, RankedGoal,
};
use std::collections::BTreeMap;
use worldwake_core::{
    BlockedIntentMemory, CauseRef, CommodityKind, ControlSource, EntityId, Quantity, Tick,
    UniqueItemKind, VisibilitySpec, WitnessData, WorldTxn,
};
use worldwake_sim::{
    ActionDefId, ActionHandlerRegistry, AutonomousController, AutonomousControllerContext,
    BeliefView, CommitOutcome, CommittedAction, InputKind, OmniscientBeliefView, RecipeRegistry,
    ReplanNeeded, Scheduler, SchedulerActionRuntime, TickInputError,
};

pub struct AgentTickDriver {
    runtime_by_agent: BTreeMap<EntityId, AgentDecisionRuntime>,
    budget: PlanningBudget,
    semantics_cache: Option<(usize, BTreeMap<ActionDefId, PlannerOpSemantics>)>,
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
        let view = OmniscientBeliefView::new(ctx.world);
        if view.is_dead(agent) || !view.is_alive(agent) {
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

    let ranked_candidates = refresh_runtime_for_read_phase(
        ctx.world,
        runtime,
        &mut blocked_memory,
        agent,
        replan_signals,
        ReadPhaseContext {
            recipe_registry,
            utility: &utility,
            tick,
        },
    );
    let active_action = active_action_for_agent(ctx, agent);

    if let Some(active_action) = active_action {
        return handle_active_action_phase(
            ctx,
            runtime,
            &mut blocked_memory,
            &original_blocked,
            agent,
            &ranked_candidates,
            &active_action,
            budget,
            tick,
            action_defs,
            action_handlers,
        );
    }

    let (next_step, next_step_valid) = plan_and_validate_next_step(
        ctx.world,
        runtime,
        agent,
        &ranked_candidates,
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
        return handle_current_step_failure(ctx, runtime, blocked_memory, agent, step, None);
    }

    let Some(targets) = resolve_step_targets(runtime, step) else {
        handle_current_step_failure(ctx, runtime, blocked_memory, agent, step, None)?;
        return finalize_agent_tick(
            ctx.world,
            ctx.event_log,
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

fn refresh_runtime_for_read_phase(
    world: &worldwake_core::World,
    runtime: &mut AgentDecisionRuntime,
    blocked_memory: &mut BlockedIntentMemory,
    agent: EntityId,
    replan_signals: &[&ReplanNeeded],
    phase: ReadPhaseContext<'_>,
) -> Vec<RankedGoal> {
    // One authoritative read view covers blocker cleanup, snapshot dirtiness, and ranking.
    let view = OmniscientBeliefView::new(world);
    let before = blocked_memory.clone();
    clear_resolved_blockers(&view, agent, blocked_memory, phase.tick);
    let blocked_changed_from_cleanup = *blocked_memory != before;
    let snapshot_changed =
        observation_snapshot_changed(&view, agent, runtime, phase.recipe_registry);

    runtime.dirty = runtime.dirty
        || runtime.current_plan.is_none()
        || plan_finished(runtime)
        || !replan_signals.is_empty()
        || blocked_changed_from_cleanup
        || snapshot_changed;

    let candidates = generate_candidates(
        &view,
        agent,
        blocked_memory,
        phase.recipe_registry,
        phase.tick,
    );
    rank_candidates(
        &candidates,
        &view,
        agent,
        phase.utility,
        phase.recipe_registry,
    )
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
    budget: &PlanningBudget,
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
    if let InterruptDecision::InterruptForReplan { trigger: _ } = evaluate_interrupt(
        runtime,
        interruptibility,
        ranked_candidates,
        plan_valid,
        budget,
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
        agent,
        tick,
        original_blocked,
        blocked_memory,
        runtime,
    )
}

#[allow(clippy::too_many_arguments)]
fn plan_and_validate_next_step(
    world: &worldwake_core::World,
    runtime: &mut AgentDecisionRuntime,
    agent: EntityId,
    ranked_candidates: &[RankedGoal],
    budget: &PlanningBudget,
    semantics_table: &BTreeMap<ActionDefId, PlannerOpSemantics>,
    action_defs: &worldwake_sim::ActionDefRegistry,
    action_handlers: &ActionHandlerRegistry,
) -> (Option<PlannedStep>, Option<bool>) {
    // A second read view covers plan selection and step validation after the active-action fork.
    let view = OmniscientBeliefView::new(world);
    if runtime.dirty {
        let plans = ranked_candidates
            .iter()
            .take(usize::from(budget.max_candidates_to_plan))
            .map(|ranked| {
                let snapshot = build_planning_snapshot(
                    &view,
                    agent,
                    &ranked.grounded.evidence_entities,
                    &ranked.grounded.evidence_places,
                    budget.snapshot_travel_horizon,
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
            .collect::<Vec<_>>();

        if let Some(selected_plan) = select_best_plan(ranked_candidates, &plans, runtime, budget) {
            runtime.materialization_bindings.clear();
            runtime.current_goal = Some(selected_plan.goal);
            runtime.current_plan = Some(selected_plan);
            runtime.current_step_index = 0;
            runtime.step_in_flight = false;
            runtime.last_priority_class = ranked_candidates
                .iter()
                .find(|candidate| Some(candidate.grounded.key) == runtime.current_goal)
                .map(|candidate| candidate.priority_class);
        } else {
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

fn finalize_agent_tick(
    world: &mut worldwake_core::World,
    event_log: &mut worldwake_core::EventLog,
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
        let view = OmniscientBeliefView::new(world);
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
    if apply_step_materialization_bindings(runtime, &step, &committed_action.outcome).is_err() {
        handle_current_step_failure(ctx, runtime, blocked_memory, agent, &step, None)?;
        return Ok(());
    }

    runtime.step_in_flight = false;
    advance_completed_step(runtime);
    Ok(())
}

fn advance_completed_step(runtime: &mut AgentDecisionRuntime) {
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
    let view = OmniscientBeliefView::new(world);
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
    view: &dyn BeliefView,
    agent: EntityId,
    runtime: &AgentDecisionRuntime,
    recipe_registry: &RecipeRegistry,
) -> bool {
    let current_commodity_signature = commodity_signature(view, agent);
    let commodity_filter = runtime
        .current_goal
        .map(|goal| goal.kind.relevant_observed_commodities(recipe_registry))
        .or_else(|| {
            runtime
                .current_plan
                .as_ref()
                .map(|plan| plan.goal.kind.relevant_observed_commodities(recipe_registry))
        });
    runtime.last_effective_place != view.effective_place(agent)
        || runtime.last_needs != view.homeostatic_needs(agent)
        || runtime.last_wounds != view.wounds(agent)
        || filtered_commodity_signature(&runtime.last_commodity_signature, commodity_filter.as_ref())
            != filtered_commodity_signature(&current_commodity_signature, commodity_filter.as_ref())
        || runtime.last_unique_item_signature != unique_item_signature(view, agent)
}

fn update_runtime_observation_snapshot(
    view: &dyn BeliefView,
    agent: EntityId,
    runtime: &mut AgentDecisionRuntime,
) {
    runtime.last_effective_place = view.effective_place(agent);
    runtime.last_needs = view.homeostatic_needs(agent);
    runtime.last_wounds = view.wounds(agent);
    runtime.last_commodity_signature = commodity_signature(view, agent);
    runtime.last_unique_item_signature = unique_item_signature(view, agent);
}

fn commodity_signature(view: &dyn BeliefView, agent: EntityId) -> Vec<(CommodityKind, Quantity)> {
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

fn unique_item_signature(view: &dyn BeliefView, agent: EntityId) -> Vec<(UniqueItemKind, u32)> {
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
        advance_completed_step, apply_step_materialization_bindings, committed_action_for_step,
        persist_blocked_memory, plan_and_validate_next_step, refresh_runtime_for_read_phase,
        resolve_step_targets, update_runtime_observation_snapshot, AgentTickDriver,
        ReadPhaseContext,
    };
    use crate::PlanningBudget;
    use crate::{
        CommodityPurpose, ExpectedMaterialization, GoalKey, GoalKind, PlanTerminalKind,
        PlannedPlan, PlannedStep, PlannerOpKind, PlanningEntityRef,
    };
    use std::fs;
    use std::path::PathBuf;
    use worldwake_core::{
        build_prototype_world, BlockedIntent, BlockedIntentMemory, BlockingFact, CarryCapacity,
        CauseRef, CommodityKind, ControlSource, DemandMemory, DemandObservation,
        DemandObservationReason, DeprivationExposure, DriveThresholds, EntityId, EventLog,
        HomeostaticNeeds, LoadUnits, MerchandiseProfile, MetabolismProfile, Place, Quantity,
        Seed, Tick, Topology, TravelEdge, TravelEdgeId, VisibilitySpec, WitnessData, World,
        WorldTxn,
    };
    use worldwake_sim::{
        step_tick, ActionDefId, ActionDefRegistry, ActionHandlerRegistry,
        AutonomousControllerRuntime, CommitOutcome, CommittedAction, ControllerState,
        DeterministicRng, Materialization, MaterializationTag, OmniscientBeliefView,
        RecipeRegistry, Scheduler, SystemDispatchTable, SystemManifest, TickStepServices,
    };
    use worldwake_systems::{build_full_action_registries, register_needs_actions};

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

    fn cargo_harness(possessed: bool) -> (Harness, EntityId, EntityId, EntityId) {
        let origin = entity(1);
        let destination = entity(2);
        let mut topology = Topology::new();
        topology
            .add_place(
                origin,
                Place {
                    name: "Origin".to_string(),
                    capacity: None,
                    tags: Default::default(),
                },
            )
            .unwrap();
        topology
            .add_place(
                destination,
                Place {
                    name: "Destination".to_string(),
                    capacity: None,
                    tags: Default::default(),
                },
            )
            .unwrap();
        topology
            .add_edge(TravelEdge::new(TravelEdgeId(1), origin, destination, 2, None).unwrap())
            .unwrap();
        topology
            .add_edge(TravelEdge::new(TravelEdgeId(2), destination, origin, 2, None).unwrap())
            .unwrap();
        let mut world = World::new(topology).unwrap();
        let actor = {
            let mut txn = new_txn(&mut world, 1);
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
        };
        let recipes = RecipeRegistry::new();
        let registries = build_full_action_registries(&recipes).unwrap();

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

    fn step_until(
        harness: &mut Harness,
        max_ticks: usize,
        predicate: impl Fn(&Harness) -> bool,
    ) {
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

    fn entity(slot: u32) -> EntityId {
        EntityId {
            slot,
            generation: 1,
        }
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
    }

    #[test]
    fn progress_barrier_completion_preserves_goal_and_forces_replan() {
        let goal = GoalKey::from(GoalKind::AcquireCommodity {
            commodity: CommodityKind::Bread,
            purpose: CommodityPurpose::SelfConsume,
        });
        let mut runtime = crate::AgentDecisionRuntime {
            current_goal: Some(goal),
            current_plan: Some(PlannedPlan::new(
                goal,
                vec![barrier_step()],
                PlanTerminalKind::ProgressBarrier,
            )),
            current_step_index: 0,
            step_in_flight: false,
            dirty: false,
            ..crate::AgentDecisionRuntime::default()
        };

        advance_completed_step(&mut runtime);

        assert_eq!(runtime.current_goal, Some(goal));
        assert_eq!(runtime.current_plan, None);
        assert_eq!(runtime.current_step_index, 0);
        assert!(runtime.dirty);
        assert!(runtime
            .materialization_bindings
            .hypothetical_to_authoritative
            .is_empty());
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
        advance_completed_step(&mut runtime);

        assert_eq!(runtime.current_step_index, 1);
        assert_eq!(
            runtime.materialization_bindings.resolve(hypothetical_id),
            Some(created)
        );

        runtime.step_in_flight = true;
        apply_step_materialization_bindings(&mut runtime, &plan.steps[1], &CommitOutcome::empty())
            .unwrap();
        runtime.step_in_flight = false;
        advance_completed_step(&mut runtime);

        assert_eq!(runtime.current_step_index, 2);
        assert_eq!(
            resolve_step_targets(&runtime, &plan.steps[2]),
            Some(vec![created])
        );

        runtime.step_in_flight = true;
        apply_step_materialization_bindings(&mut runtime, &plan.steps[2], &CommitOutcome::empty())
            .unwrap();
        runtime.step_in_flight = false;
        advance_completed_step(&mut runtime);

        assert!(runtime.current_plan.is_none());
        assert!(!runtime.step_in_flight);
        assert!(runtime
            .materialization_bindings
            .hypothetical_to_authoritative
            .is_empty());
    }

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
        let view = OmniscientBeliefView::new(&harness.world);
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
        assert_eq!(grounded.evidence_entities, [original_lot].into_iter().collect());
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
            runtime,
            &mut blocked,
            harness.actor,
            &[],
            ReadPhaseContext {
                recipe_registry: &harness.recipes,
                utility: &utility,
                tick: Tick(1),
            },
        );
        let (next_step, next_step_valid) = plan_and_validate_next_step(
            &harness.world,
            runtime,
            harness.actor,
            &ranked,
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
        assert_eq!(harness.world.possessor_of(carried_water), Some(harness.actor));
        assert_eq!(harness.world.effective_place(carried_water), Some(origin));
        assert_eq!(
            harness
                .world
                .get_component_item_lot(carried_water)
                .unwrap()
                .quantity,
            Quantity(2)
        );

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
        advance_completed_step(runtime);
        assert_eq!(runtime.current_goal, Some(expected_goal));

        let ranked_after_pickup = refresh_runtime_for_read_phase(
            &harness.world,
            runtime,
            &mut blocked,
            harness.actor,
            &[],
            ReadPhaseContext {
                recipe_registry: &harness.recipes,
                utility: &utility,
                tick: Tick(2),
            },
        );
        assert!(runtime.dirty);
        let (next_step, next_step_valid) = plan_and_validate_next_step(
            &harness.world,
            runtime,
            harness.actor,
            &ranked_after_pickup,
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
        assert_eq!(next_step_valid, Some(false));
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
        let view = OmniscientBeliefView::new(&harness.world);
        update_runtime_observation_snapshot(&view, harness.actor, runtime);

        {
            let place = harness.world.effective_place(harness.actor).unwrap();
            let mut txn = new_txn(&mut harness.world, 2);
            let coin = txn.create_item_lot(CommodityKind::Coin, Quantity(1)).unwrap();
            txn.set_ground_location(coin, place).unwrap();
            txn.set_possessor(coin, harness.actor).unwrap();
            commit_txn(txn);
        }

        let mut blocked = BlockedIntentMemory::default();
        let _ = refresh_runtime_for_read_phase(
            &harness.world,
            runtime,
            &mut blocked,
            harness.actor,
            &[],
            ReadPhaseContext {
                recipe_registry: &harness.recipes,
                utility: &utility,
                tick: Tick(2),
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
        let runtime = harness.driver.runtime_by_agent.entry(harness.actor).or_insert_with(|| {
            active_runtime(GoalKind::ConsumeOwnedCommodity {
                commodity: CommodityKind::Bread,
            })
        });
        let view = OmniscientBeliefView::new(&harness.world);
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
            runtime,
            &mut blocked,
            harness.actor,
            &[],
            ReadPhaseContext {
                recipe_registry: &harness.recipes,
                utility: &utility,
                tick: Tick(2),
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
        let view = OmniscientBeliefView::new(&harness.world);
        update_runtime_observation_snapshot(&view, harness.actor, &mut runtime);
        let mut blocked = BlockedIntentMemory::default();

        let _ = refresh_runtime_for_read_phase(
            &harness.world,
            &mut runtime,
            &mut blocked,
            harness.actor,
            &[],
            ReadPhaseContext {
                recipe_registry: &harness.recipes,
                utility: &utility,
                tick: Tick(1),
            },
        );

        assert!(runtime.dirty);
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
                "{relative} should read through BeliefView instead of depending on World"
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
}
