use crate::{
    build_planning_snapshot, build_semantics_table, clear_resolved_blockers, evaluate_interrupt,
    generate_candidates, handle_plan_failure, rank_candidates, revalidate_next_step,
    search_plan, select_best_plan, AgentDecisionRuntime, InterruptDecision, PlanFailureContext,
    PlannedStep, PlannerOpSemantics, PlanningBudget,
};
use std::collections::BTreeMap;
use worldwake_core::{
    BlockedIntentMemory, CauseRef, CommodityKind, ControlSource, EntityId, Quantity, Tick,
    UniqueItemKind, VisibilitySpec, WitnessData, WorldTxn,
};
use worldwake_sim::{
    ActionDefId, ActionHandlerRegistry, AutonomousController, AutonomousControllerContext,
    BeliefView, OmniscientBeliefView, RecipeRegistry, ReplanNeeded, Scheduler,
    SchedulerActionRuntime, TickInputError,
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

impl AutonomousController for AgentTickDriver {
    fn name(&self) -> &'static str {
        "agent_tick_driver"
    }

    fn claims_agent(&self, _world: &worldwake_core::World, _agent: EntityId, control_source: ControlSource) -> bool {
        control_source == ControlSource::Ai
    }

    fn produce_agent_input(
        &mut self,
        ctx: AutonomousControllerContext<'_>,
        agent: EntityId,
        replan_signals: &[&ReplanNeeded],
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
        )
    }
}

#[allow(clippy::too_many_lines)]
fn process_agent(
    ctx: &mut AgentTickContext<'_>,
    runtime_by_agent: &mut BTreeMap<EntityId, AgentDecisionRuntime>,
    agent: EntityId,
    replan_signals: &[&ReplanNeeded],
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
    let active_action = ctx
        .scheduler
        .active_actions()
        .values()
        .find(|instance| instance.actor == agent)
        .cloned();

    {
        let view = OmniscientBeliefView::new(ctx.world);
        if view.is_dead(agent) || !view.is_alive(agent) {
            runtime.current_goal = None;
            runtime.current_plan = None;
            runtime.current_step_index = 0;
            runtime.step_in_flight = false;
            runtime.dirty = false;
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
    )?;

    let blocked_changed_from_cleanup = {
        let view = OmniscientBeliefView::new(ctx.world);
        let before = blocked_memory.clone();
        clear_resolved_blockers(&view, agent, &mut blocked_memory, tick);
        blocked_memory != before
    };

    let active_action = ctx
        .scheduler
        .active_actions()
        .values()
        .find(|instance| instance.actor == agent)
        .cloned();
    let snapshot_changed = {
        let view = OmniscientBeliefView::new(ctx.world);
        observation_snapshot_changed(&view, agent, runtime)
    };

    runtime.dirty = runtime.dirty
        || runtime.current_plan.is_none()
        || plan_finished(runtime)
        || !replan_signals.is_empty()
        || blocked_changed_from_cleanup
        || snapshot_changed;

    let ranked_candidates = {
        let view = OmniscientBeliefView::new(ctx.world);
        let candidates = generate_candidates(&view, agent, &blocked_memory, recipe_registry, tick);
        rank_candidates(&candidates, &view, agent, &utility, recipe_registry)
    };

    if let Some(active_action) = active_action {
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
            &ranked_candidates,
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
            reconcile_in_flight_state(
                ctx,
                runtime,
                &mut blocked_memory,
                None,
                agent,
                &[&replan],
            )?;
        }

        persist_blocked_memory(
            ctx.world,
            ctx.event_log,
            agent,
            tick,
            &original_blocked,
            &blocked_memory,
        )?;
        {
            let view = OmniscientBeliefView::new(ctx.world);
            update_runtime_observation_snapshot(&view, agent, runtime);
        }
        return Ok(());
    }

    if runtime.dirty {
        let plans = {
            let view = OmniscientBeliefView::new(ctx.world);
            ranked_candidates
                .iter()
                .take(usize::from(budget.max_candidates_to_plan))
                .map(|ranked| {
                    let snapshot = build_planning_snapshot(
                        &view,
                        agent,
                        &ranked.grounded.evidence_entities,
                        &ranked.grounded.evidence_places,
                        budget.max_plan_depth,
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
                .collect::<Vec<_>>()
        };

        if let Some(selected_plan) = select_best_plan(&ranked_candidates, &plans, runtime, budget) {
            runtime.current_goal = Some(selected_plan.goal);
            runtime.current_plan = Some(selected_plan);
            runtime.current_step_index = 0;
            runtime.step_in_flight = false;
            runtime.last_priority_class = ranked_candidates
                .iter()
                .find(|candidate| Some(candidate.grounded.key) == runtime.current_goal)
                .map(|candidate| candidate.priority_class);
        } else {
            runtime.current_goal = None;
            runtime.current_plan = None;
            runtime.current_step_index = 0;
            runtime.step_in_flight = false;
            runtime.last_priority_class = ranked_candidates.first().map(|candidate| candidate.priority_class);
        }
        runtime.dirty = false;
    }

    if let Some(step) = current_step(runtime).cloned() {
        let valid = {
            let view = OmniscientBeliefView::new(ctx.world);
            revalidate_next_step(&view, agent, &step, action_defs, action_handlers)
        };
        if valid {
            let _ = ctx
                .scheduler
                .input_queue_mut()
                .enqueue(tick, step.to_request_action(agent));
            runtime.step_in_flight = true;
        } else {
            handle_current_step_failure(ctx, runtime, &mut blocked_memory, agent, &step, None)?;
        }
    }

    persist_blocked_memory(
        ctx.world,
        ctx.event_log,
        agent,
        tick,
        &original_blocked,
        &blocked_memory,
    )?;
    {
        let view = OmniscientBeliefView::new(ctx.world);
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

    runtime.step_in_flight = false;
    runtime.current_step_index = runtime
        .current_step_index
        .checked_add(1)
        .expect("agent decision runtime step index overflowed");
    if plan_finished(runtime) {
        runtime.current_goal = None;
        runtime.current_plan = None;
        runtime.current_step_index = 0;
        runtime.dirty = true;
    }
    Ok(())
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

fn persist_blocked_memory(
    world: &mut worldwake_core::World,
    event_log: &mut worldwake_core::EventLog,
    agent: EntityId,
    tick: Tick,
    before: &BlockedIntentMemory,
    after: &BlockedIntentMemory,
) -> Result<(), TickInputError> {
    let existing = world.get_component_blocked_intent_memory(agent);
    if existing == Some(after) || (existing.is_none() && before == after && after.intents.is_empty()) {
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
    runtime
        .current_plan
        .as_ref()
        .is_some_and(|plan| runtime.current_step_index >= plan.steps.len() && !runtime.step_in_flight)
}

fn observation_snapshot_changed(view: &dyn BeliefView, agent: EntityId, runtime: &AgentDecisionRuntime) -> bool {
    runtime.last_effective_place != view.effective_place(agent)
        || runtime.last_needs != view.homeostatic_needs(agent)
        || runtime.last_wounds != view.wounds(agent)
        || runtime.last_commodity_signature != commodity_signature(view, agent)
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
    use super::AgentTickDriver;
    use crate::PlanningBudget;
    use worldwake_core::{
        build_prototype_world, CauseRef, CommodityKind, ControlSource, DeprivationExposure,
        DriveThresholds, EventLog, HomeostaticNeeds, MetabolismProfile, Quantity, Seed, Tick,
        VisibilitySpec, WitnessData, World, WorldTxn,
    };
    use worldwake_sim::{
        step_tick, ActionDefRegistry, ActionHandlerRegistry, AutonomousControllerRuntime,
        ControllerState, DeterministicRng, RecipeRegistry, Scheduler, SystemDispatchTable,
        SystemManifest, TickStepServices,
    };
    use worldwake_systems::register_needs_actions;

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
                let bread = txn.create_item_lot(CommodityKind::Bread, Quantity(1)).unwrap();
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
            let mut controllers =
                AutonomousControllerRuntime::new(vec![&mut self.driver]);
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
}
