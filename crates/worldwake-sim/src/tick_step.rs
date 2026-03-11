use crate::scheduler::SchedulerActionRuntime;
use crate::{
    get_affordances, ActionDefId, ActionDefRegistry, ActionError, ActionExecutionContext,
    ActionHandlerRegistry, ActionInstanceId, ControlError, ControllerState, DeterministicRng,
    ExternalAbortReason, InputKind, RecipeRegistry, Scheduler, SystemDispatchTable, SystemError,
    TickInputContext, TickInputError, TickInputProducer, TickOutcome,
};
use std::collections::BTreeSet;
use std::fmt;
use worldwake_core::{
    CauseRef, EntityId, EventLog, EventTag, PendingEvent, Tick, VisibilitySpec, WitnessData, World,
};

pub struct TickStepServices<'a> {
    pub action_defs: &'a ActionDefRegistry,
    pub action_handlers: &'a ActionHandlerRegistry,
    pub recipe_registry: &'a RecipeRegistry,
    pub systems: &'a SystemDispatchTable,
    pub input_producer: Option<&'a mut dyn TickInputProducer>,
}

struct TickStepRuntime<'a> {
    world: &'a mut World,
    event_log: &'a mut EventLog,
    scheduler: &'a mut Scheduler,
    rng: &'a mut DeterministicRng,
}

#[derive(Copy, Clone, Debug, Default, Eq, PartialEq)]
struct InputOutcome {
    actions_started: u32,
    actions_aborted: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TickStepResult {
    pub tick: Tick,
    pub inputs_processed: u32,
    pub actions_started: u32,
    pub actions_completed: u32,
    pub actions_aborted: u32,
    pub systems_ran: u32,
    pub events_emitted_count: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TickStepError {
    Control(ControlError),
    Action(ActionError),
    InputProducer(TickInputError),
    RequestedAffordanceUnavailable {
        actor: EntityId,
        def_id: ActionDefId,
        targets: Vec<EntityId>,
        payload_override: Option<crate::ActionPayload>,
    },
    CancelActorMismatch {
        actor: EntityId,
        action_instance_id: ActionInstanceId,
        actual_actor: EntityId,
    },
    System {
        system_id: crate::SystemId,
        source: SystemError,
    },
}

impl fmt::Display for TickStepError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Control(err) => write!(f, "control switch failed: {err:?}"),
            Self::Action(err) => write!(f, "action execution failed: {err:?}"),
            Self::InputProducer(err) => {
                write!(f, "tick input producer failed before input drain: {err}")
            }
            Self::RequestedAffordanceUnavailable {
                actor,
                def_id,
                targets,
                payload_override,
            } => write!(
                f,
                "requested affordance is not currently available for actor {actor}: def {def_id}, targets {targets:?}, payload {payload_override:?}"
            ),
            Self::CancelActorMismatch {
                actor,
                action_instance_id,
                actual_actor,
            } => write!(
                f,
                "cancel input actor {actor} does not own action {action_instance_id}; actual actor is {actual_actor}"
            ),
            Self::System { system_id, source } => {
                write!(f, "system {system_id} failed during tick step: {source}")
            }
        }
    }
}

impl std::error::Error for TickStepError {}

pub fn step_tick(
    world: &mut World,
    event_log: &mut EventLog,
    scheduler: &mut Scheduler,
    controller: &mut ControllerState,
    rng: &mut DeterministicRng,
    mut services: TickStepServices<'_>,
) -> Result<TickStepResult, TickStepError> {
    let mut runtime = TickStepRuntime {
        world,
        event_log,
        scheduler,
        rng,
    };
    let tick = runtime.scheduler.current_tick();
    let events_before = runtime.event_log.len();
    produce_tick_inputs(&mut runtime, tick, &mut services)?;
    let pre_progress_dead_aborts = abort_actions_for_dead_actors(&mut runtime, tick, &services)?;
    let (inputs_processed, actions_started, actions_aborted) =
        process_inputs(&mut runtime, controller, tick, &services)?;
    let (actions_completed, progressed_action_aborts) =
        progress_active_actions(&mut runtime, tick, &services)?;
    let systems_ran = run_systems(&mut runtime, tick, &services)?;
    let post_system_dead_aborts = abort_actions_for_dead_actors(&mut runtime, tick, &services)?;
    emit_end_of_tick_marker(runtime.event_log, tick);
    runtime.scheduler.increment_tick();

    Ok(TickStepResult {
        tick,
        inputs_processed,
        actions_started,
        actions_completed,
        actions_aborted: pre_progress_dead_aborts
            .checked_add(actions_aborted)
            .expect("tick-step action-abort counter overflowed")
            .checked_add(progressed_action_aborts)
            .expect("tick-step action-abort counter overflowed")
            .checked_add(post_system_dead_aborts)
            .expect("tick-step action-abort counter overflowed"),
        systems_ran,
        events_emitted_count: u32::try_from(runtime.event_log.len() - events_before)
            .expect("tick-step emitted-event count exceeds u32"),
    })
}

fn process_inputs(
    runtime: &mut TickStepRuntime<'_>,
    controller: &mut ControllerState,
    tick: Tick,
    services: &TickStepServices<'_>,
) -> Result<(u32, u32, u32), TickStepError> {
    let inputs = runtime.scheduler.drain_current_tick_inputs();
    let mut inputs_processed = 0u32;
    let mut outcome = InputOutcome::default();

    for input in inputs {
        inputs_processed = inputs_processed
            .checked_add(1)
            .expect("tick-step input counter overflowed");
        let delta = apply_input(
            runtime,
            controller,
            tick,
            services,
            input.sequence_no,
            input.kind,
        )?;
        outcome.actions_started = outcome
            .actions_started
            .checked_add(delta.actions_started)
            .expect("tick-step action-start counter overflowed");
        outcome.actions_aborted = outcome
            .actions_aborted
            .checked_add(delta.actions_aborted)
            .expect("tick-step action-abort counter overflowed");
    }

    Ok((
        inputs_processed,
        outcome.actions_started,
        outcome.actions_aborted,
    ))
}

fn apply_input(
    runtime: &mut TickStepRuntime<'_>,
    controller: &mut ControllerState,
    tick: Tick,
    services: &TickStepServices<'_>,
    sequence_no: u64,
    kind: InputKind,
) -> Result<InputOutcome, TickStepError> {
    match kind {
        InputKind::SwitchControl { from, to } => {
            controller
                .switch_control(from, to)
                .map_err(TickStepError::Control)?;
            Ok(InputOutcome::default())
        }
        InputKind::RequestAction {
            actor,
            def_id,
            targets,
            payload_override,
        } => {
            let affordance = resolve_affordance(
                runtime.world,
                services.action_defs,
                services.action_handlers,
                actor,
                def_id,
                &targets,
                payload_override,
            )?;
            runtime
                .scheduler
                .start_affordance(
                    &affordance,
                    SchedulerActionRuntime {
                        action_defs: services.action_defs,
                        action_handlers: services.action_handlers,
                        world: runtime.world,
                        event_log: runtime.event_log,
                        rng: runtime.rng,
                    },
                    ActionExecutionContext {
                        cause: CauseRef::ExternalInput(sequence_no),
                        tick,
                    },
                )
                .map_err(TickStepError::Action)?;
            Ok(InputOutcome {
                actions_started: 1,
                actions_aborted: 0,
            })
        }
        InputKind::CancelAction {
            actor,
            action_instance_id,
        } => {
            validate_cancel_actor(runtime.scheduler, actor, action_instance_id)?;
            let replan = runtime
                .scheduler
                .abort_active_action(
                    action_instance_id,
                    SchedulerActionRuntime {
                        action_defs: services.action_defs,
                        action_handlers: services.action_handlers,
                        world: runtime.world,
                        event_log: runtime.event_log,
                        rng: runtime.rng,
                    },
                    ActionExecutionContext {
                        cause: CauseRef::ExternalInput(sequence_no),
                        tick,
                    },
                    ExternalAbortReason::CancelledByInput { sequence_no },
                )
                .map_err(TickStepError::Action)?;
            runtime.scheduler.retain_replan(replan);
            Ok(InputOutcome {
                actions_started: 0,
                actions_aborted: 1,
            })
        }
    }
}

fn resolve_affordance(
    world: &World,
    action_defs: &ActionDefRegistry,
    action_handlers: &ActionHandlerRegistry,
    actor: EntityId,
    def_id: ActionDefId,
    targets: &[EntityId],
    payload_override: Option<crate::ActionPayload>,
) -> Result<crate::Affordance, TickStepError> {
    let view = crate::OmniscientBeliefView::new(world);
    let Some(def) = action_defs.get(def_id) else {
        return Err(TickStepError::RequestedAffordanceUnavailable {
            actor,
            def_id,
            targets: targets.to_owned(),
            payload_override,
        });
    };
    let mut affordance = get_affordances(&view, actor, action_defs, action_handlers)
        .into_iter()
        .find(|affordance| {
            affordance.matches_request_identity(def, actor, targets, payload_override.as_ref())
        })
        .ok_or(TickStepError::RequestedAffordanceUnavailable {
            actor,
            def_id,
            targets: targets.to_owned(),
            payload_override: payload_override.clone(),
        })?;
    affordance.payload_override = payload_override;
    Ok(affordance)
}

fn validate_cancel_actor(
    scheduler: &Scheduler,
    actor: EntityId,
    action_instance_id: ActionInstanceId,
) -> Result<(), TickStepError> {
    if let Some(actual_actor) = scheduler.active_action_actor(action_instance_id) {
        if actual_actor != actor {
            return Err(TickStepError::CancelActorMismatch {
                actor,
                action_instance_id,
                actual_actor,
            });
        }
    }

    Ok(())
}

fn progress_active_actions(
    runtime: &mut TickStepRuntime<'_>,
    tick: Tick,
    services: &TickStepServices<'_>,
) -> Result<(u32, u32), TickStepError> {
    let active_action_ids = runtime
        .scheduler
        .active_actions()
        .keys()
        .copied()
        .collect::<Vec<_>>();
    let mut actions_completed = 0u32;
    let mut actions_aborted = 0u32;

    for instance_id in active_action_ids {
        match runtime
            .scheduler
            .tick_active_action(
                instance_id,
                SchedulerActionRuntime {
                    action_defs: services.action_defs,
                    action_handlers: services.action_handlers,
                    world: runtime.world,
                    event_log: runtime.event_log,
                    rng: runtime.rng,
                },
                ActionExecutionContext {
                    cause: CauseRef::SystemTick(tick),
                    tick,
                },
            )
            .map_err(TickStepError::Action)?
        {
            TickOutcome::Continuing => {}
            TickOutcome::Committed => {
                actions_completed = actions_completed
                    .checked_add(1)
                    .expect("tick-step action-complete counter overflowed");
            }
            TickOutcome::Aborted { replan, .. } => {
                runtime.scheduler.retain_replan(replan);
                actions_aborted = actions_aborted
                    .checked_add(1)
                    .expect("tick-step action-abort counter overflowed");
            }
        }
    }

    Ok((actions_completed, actions_aborted))
}

fn abort_actions_for_dead_actors(
    runtime: &mut TickStepRuntime<'_>,
    tick: Tick,
    services: &TickStepServices<'_>,
) -> Result<u32, TickStepError> {
    let action_ids = runtime
        .scheduler
        .active_actions()
        .iter()
        .filter_map(|(instance_id, instance)| {
            runtime
                .world
                .get_component_dead_at(instance.actor)
                .is_some()
                .then_some(*instance_id)
        })
        .collect::<Vec<_>>();
    let mut aborted = 0u32;

    for instance_id in action_ids {
        let replan = runtime
            .scheduler
            .abort_active_action(
                instance_id,
                SchedulerActionRuntime {
                    action_defs: services.action_defs,
                    action_handlers: services.action_handlers,
                    world: runtime.world,
                    event_log: runtime.event_log,
                    rng: runtime.rng,
                },
                ActionExecutionContext {
                    cause: CauseRef::SystemTick(tick),
                    tick,
                },
                ExternalAbortReason::ActorMarkedDead,
            )
            .map_err(TickStepError::Action)?;
        runtime.scheduler.retain_replan(replan);
        aborted = aborted
            .checked_add(1)
            .expect("tick-step action-abort counter overflowed");
    }

    Ok(aborted)
}

fn run_systems(
    runtime: &mut TickStepRuntime<'_>,
    tick: Tick,
    services: &TickStepServices<'_>,
) -> Result<u32, TickStepError> {
    let mut systems_ran = 0u32;

    for system_id in runtime
        .scheduler
        .system_manifest()
        .ordered_ids()
        .iter()
        .copied()
    {
        let mut system_rng = runtime.rng.substream(tick, system_id, 0);
        services.systems.get(system_id)(crate::SystemExecutionContext {
            world: runtime.world,
            event_log: runtime.event_log,
            rng: &mut system_rng,
            active_actions: runtime.scheduler.active_actions(),
            action_defs: services.action_defs,
            tick,
            system_id,
        })
        .map_err(|source| TickStepError::System { system_id, source })?;
        systems_ran = systems_ran
            .checked_add(1)
            .expect("tick-step system-run counter overflowed");
    }

    Ok(systems_ran)
}

fn produce_tick_inputs(
    runtime: &mut TickStepRuntime<'_>,
    tick: Tick,
    services: &mut TickStepServices<'_>,
) -> Result<(), TickStepError> {
    let Some(input_producer) = services.input_producer.as_deref_mut() else {
        return Ok(());
    };
    let pending_replans = runtime.scheduler.drain_pending_replans();

    input_producer
        .produce_inputs(TickInputContext {
            world: runtime.world,
            event_log: runtime.event_log,
            scheduler: runtime.scheduler,
            rng: runtime.rng,
            action_defs: services.action_defs,
            action_handlers: services.action_handlers,
            recipe_registry: services.recipe_registry,
            pending_replans: &pending_replans,
            tick,
        })
        .map_err(TickStepError::InputProducer)
}

fn emit_end_of_tick_marker(event_log: &mut EventLog, tick: Tick) {
    let _ = event_log.emit(PendingEvent::new(
        tick,
        CauseRef::SystemTick(tick),
        None,
        Vec::new(),
        None,
        Vec::new(),
        VisibilitySpec::Hidden,
        WitnessData::default(),
        BTreeSet::from([EventTag::System]),
    ));
}

#[cfg(test)]
mod tests {
    use super::{resolve_affordance, step_tick, TickStepError, TickStepResult, TickStepServices};
    use crate::{
        get_affordances, ActionDef, ActionDefId, ActionDefRegistry, ActionDomain, ActionError,
        ActionHandler, ActionHandlerId, ActionHandlerRegistry, ActionInstance, ActionInstanceId,
        ActionPayload, ActionProgress, ActionState, ActionStatus, ControllerState,
        DeterministicRng, DurationExpr, InputKind, Interruptibility, RecipeRegistry, Scheduler,
        SystemDispatchTable, SystemError, SystemExecutionContext, SystemManifest, TickInputContext,
        TickInputError, TickInputProducer,
    };
    use std::collections::BTreeSet;
    use std::num::NonZeroU32;
    use std::sync::{Mutex, OnceLock};
    use worldwake_core::{
        build_prototype_world, BodyCostPerTick, CauseRef, ControlSource, DeadAt, EntityId,
        EventLog, EventTag, Seed, Tick, VisibilitySpec, WitnessData, World, WorldTxn,
    };

    #[derive(Clone, Debug, Default, Eq, PartialEq)]
    struct HookLog {
        starts: Vec<ActionDefId>,
        ticks: Vec<ActionInstanceId>,
        aborts: Vec<ActionInstanceId>,
        systems: Vec<crate::SystemId>,
        system_active_action_counts: Vec<usize>,
        system_def_counts: Vec<usize>,
    }

    fn test_lock() -> &'static Mutex<()> {
        static TEST_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        TEST_LOCK.get_or_init(|| Mutex::new(()))
    }

    fn hook_log() -> &'static Mutex<HookLog> {
        static HOOK_LOG: OnceLock<Mutex<HookLog>> = OnceLock::new();
        HOOK_LOG.get_or_init(|| Mutex::new(HookLog::default()))
    }

    fn reset_hooks() {
        *hook_log().lock().unwrap() = HookLog::default();
    }

    fn entity(slot: u32) -> EntityId {
        EntityId {
            slot,
            generation: 1,
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

    fn spawn_agent(world: &mut World, slot: u64, control_source: ControlSource) -> EntityId {
        let mut txn = new_txn(world, slot);
        let agent = txn
            .create_agent(&format!("agent-{slot}"), control_source)
            .unwrap();
        let mut log = EventLog::new();
        let _ = txn.commit(&mut log);
        agent
    }

    #[allow(clippy::unnecessary_wraps)]
    fn start_record(
        _def: &ActionDef,
        instance: &ActionInstance,
        _rng: &mut DeterministicRng,
        _txn: &mut WorldTxn<'_>,
    ) -> Result<Option<ActionState>, ActionError> {
        hook_log().lock().unwrap().starts.push(instance.def_id);
        Ok(Some(ActionState::Empty))
    }

    #[allow(clippy::unnecessary_wraps)]
    fn tick_continue(
        _def: &ActionDef,
        instance: &ActionInstance,
        _rng: &mut DeterministicRng,
        _txn: &mut WorldTxn<'_>,
    ) -> Result<ActionProgress, ActionError> {
        hook_log().lock().unwrap().ticks.push(instance.instance_id);
        Ok(ActionProgress::Continue)
    }

    #[allow(clippy::unnecessary_wraps)]
    fn tick_complete(
        _def: &ActionDef,
        instance: &ActionInstance,
        _rng: &mut DeterministicRng,
        _txn: &mut WorldTxn<'_>,
    ) -> Result<ActionProgress, ActionError> {
        hook_log().lock().unwrap().ticks.push(instance.instance_id);
        Ok(ActionProgress::Complete)
    }

    #[allow(clippy::unnecessary_wraps)]
    fn commit_noop(
        _def: &ActionDef,
        _instance: &ActionInstance,
        _rng: &mut DeterministicRng,
        _txn: &mut WorldTxn<'_>,
    ) -> Result<(), ActionError> {
        Ok(())
    }

    #[allow(clippy::unnecessary_wraps)]
    fn abort_record(
        _def: &ActionDef,
        instance: &ActionInstance,
        _reason: &crate::AbortReason,
        _rng: &mut DeterministicRng,
        _txn: &mut WorldTxn<'_>,
    ) -> Result<(), ActionError> {
        hook_log().lock().unwrap().aborts.push(instance.instance_id);
        Ok(())
    }

    #[allow(clippy::needless_pass_by_value, clippy::unnecessary_wraps)]
    fn record_system(context: SystemExecutionContext<'_>) -> Result<(), SystemError> {
        let mut log = hook_log().lock().unwrap();
        log.systems.push(context.system_id);
        log.system_active_action_counts
            .push(context.active_actions.len());
        log.system_def_counts
            .push(context.action_defs.iter().count());
        let _ = context.world;
        let _ = context.event_log;
        let _ = context.tick;
        let _ = context.rng.next_u32();
        Ok(())
    }

    fn handler_registry() -> ActionHandlerRegistry {
        let mut registry = ActionHandlerRegistry::new();
        let continue_id = registry.register(ActionHandler::new(
            start_record,
            tick_continue,
            commit_noop,
            abort_record,
        ));
        let complete_id = registry.register(ActionHandler::new(
            start_record,
            tick_complete,
            commit_noop,
            abort_record,
        ));

        assert_eq!(continue_id, ActionHandlerId(0));
        assert_eq!(complete_id, ActionHandlerId(1));
        registry
    }

    fn action_registry() -> ActionDefRegistry {
        let mut registry = ActionDefRegistry::new();
        registry.register(ActionDef {
            id: ActionDefId(0),
            name: "continue".to_string(),
            domain: ActionDomain::Generic,
            actor_constraints: vec![crate::Constraint::ActorAlive],
            targets: Vec::new(),
            preconditions: vec![crate::Precondition::ActorAlive],
            reservation_requirements: Vec::new(),
            duration: DurationExpr::Fixed(NonZeroU32::new(2).unwrap()),
            body_cost_per_tick: BodyCostPerTick::zero(),
            interruptibility: Interruptibility::FreelyInterruptible,
            commit_conditions: vec![crate::Precondition::ActorAlive],
            visibility: VisibilitySpec::SamePlace,
            causal_event_tags: BTreeSet::new(),
            payload: ActionPayload::None,
            handler: ActionHandlerId(0),
        });
        registry.register(ActionDef {
            id: ActionDefId(1),
            name: "complete".to_string(),
            domain: ActionDomain::Generic,
            actor_constraints: vec![crate::Constraint::ActorAlive],
            targets: Vec::new(),
            preconditions: vec![crate::Precondition::ActorAlive],
            reservation_requirements: Vec::new(),
            duration: DurationExpr::Fixed(NonZeroU32::MIN),
            body_cost_per_tick: BodyCostPerTick::zero(),
            interruptibility: Interruptibility::FreelyInterruptible,
            commit_conditions: vec![crate::Precondition::ActorAlive],
            visibility: VisibilitySpec::SamePlace,
            causal_event_tags: BTreeSet::from([EventTag::Travel]),
            payload: ActionPayload::None,
            handler: ActionHandlerId(1),
        });
        registry
    }

    fn ordered_systems() -> SystemDispatchTable {
        SystemDispatchTable::from_handlers([record_system; crate::SystemId::ALL.len()])
    }

    fn build_state() -> (
        World,
        EventLog,
        Scheduler,
        ControllerState,
        DeterministicRng,
        RecipeRegistry,
        ActionDefRegistry,
        ActionHandlerRegistry,
    ) {
        let mut world = World::new(build_prototype_world()).unwrap();
        let actor = spawn_agent(&mut world, 1, ControlSource::Ai);
        let event_log = EventLog::new();
        let scheduler = Scheduler::new(SystemManifest::canonical());
        let controller = ControllerState::with_entity(actor);
        let rng = DeterministicRng::new(Seed([9; 32]));
        (
            world,
            event_log,
            scheduler,
            controller,
            rng,
            RecipeRegistry::new(),
            action_registry(),
            handler_registry(),
        )
    }

    fn services<'a>(
        defs: &'a ActionDefRegistry,
        handlers: &'a ActionHandlerRegistry,
        recipes: &'a RecipeRegistry,
        systems: &'a SystemDispatchTable,
    ) -> TickStepServices<'a> {
        TickStepServices {
            action_defs: defs,
            action_handlers: handlers,
            recipe_registry: recipes,
            systems,
            input_producer: None,
        }
    }

    struct QueueingProducer {
        actor: EntityId,
        def_id: ActionDefId,
        observed_pending_replans: Vec<usize>,
    }

    impl TickInputProducer for QueueingProducer {
        fn produce_inputs(&mut self, ctx: TickInputContext<'_>) -> Result<(), TickInputError> {
            self.observed_pending_replans
                .push(ctx.pending_replans.len());
            ctx.scheduler.input_queue_mut().enqueue(
                ctx.tick,
                InputKind::RequestAction {
                    actor: self.actor,
                    def_id: self.def_id,
                    targets: Vec::new(),
                    payload_override: None,
                },
            );
            Ok(())
        }
    }

    fn controlled_actor(controller: &ControllerState) -> EntityId {
        controller.controlled_entity().unwrap()
    }

    #[test]
    fn empty_tick_increments_and_emits_end_of_tick_event() {
        let _guard = test_lock().lock().unwrap();
        reset_hooks();
        let (
            mut world,
            mut event_log,
            mut scheduler,
            mut controller,
            mut rng,
            recipes,
            defs,
            handlers,
        ) = build_state();

        let systems = SystemDispatchTable::canonical_noop();
        let services = services(&defs, &handlers, &recipes, &systems);
        let result = step_tick(
            &mut world,
            &mut event_log,
            &mut scheduler,
            &mut controller,
            &mut rng,
            services,
        )
        .unwrap();

        assert_eq!(
            result,
            TickStepResult {
                tick: Tick(0),
                inputs_processed: 0,
                actions_started: 0,
                actions_completed: 0,
                actions_aborted: 0,
                systems_ran: crate::SystemId::ALL.len() as u32,
                events_emitted_count: 1,
            }
        );
        assert_eq!(scheduler.current_tick(), Tick(1));
        assert_eq!(event_log.len(), 1);
        let tick_event = event_log.get(worldwake_core::EventId(0)).unwrap();
        assert_eq!(tick_event.cause, CauseRef::SystemTick(Tick(0)));
        assert!(tick_event.tags.contains(&EventTag::System));
    }

    #[test]
    fn request_inputs_are_processed_in_sequence_order() {
        let _guard = test_lock().lock().unwrap();
        reset_hooks();
        let (
            mut world,
            mut event_log,
            mut scheduler,
            mut controller,
            mut rng,
            recipes,
            defs,
            handlers,
        ) = build_state();
        let actor = controlled_actor(&controller);

        scheduler.input_queue_mut().enqueue(
            Tick(0),
            crate::InputKind::RequestAction {
                actor,
                def_id: ActionDefId(1),
                targets: Vec::new(),
                payload_override: None,
            },
        );
        scheduler.input_queue_mut().enqueue(
            Tick(0),
            crate::InputKind::RequestAction {
                actor,
                def_id: ActionDefId(0),
                targets: Vec::new(),
                payload_override: None,
            },
        );

        let result = step_tick(
            &mut world,
            &mut event_log,
            &mut scheduler,
            &mut controller,
            &mut rng,
            TickStepServices {
                action_defs: &defs,
                action_handlers: &handlers,
                recipe_registry: &recipes,
                systems: &SystemDispatchTable::canonical_noop(),
                input_producer: None,
            },
        )
        .unwrap();

        assert_eq!(result.inputs_processed, 2);
        assert_eq!(
            hook_log().lock().unwrap().starts,
            vec![ActionDefId(1), ActionDefId(0)]
        );
    }

    #[test]
    fn unavailable_request_returns_structured_error() {
        let _guard = test_lock().lock().unwrap();
        reset_hooks();
        let (
            mut world,
            mut event_log,
            mut scheduler,
            mut controller,
            mut rng,
            recipes,
            defs,
            handlers,
        ) = build_state();
        let actor = controlled_actor(&controller);

        scheduler.input_queue_mut().enqueue(
            Tick(0),
            crate::InputKind::RequestAction {
                actor,
                def_id: ActionDefId(99),
                targets: Vec::new(),
                payload_override: None,
            },
        );

        let error = step_tick(
            &mut world,
            &mut event_log,
            &mut scheduler,
            &mut controller,
            &mut rng,
            TickStepServices {
                action_defs: &defs,
                action_handlers: &handlers,
                recipe_registry: &recipes,
                systems: &SystemDispatchTable::canonical_noop(),
                input_producer: None,
            },
        )
        .unwrap_err();

        assert_eq!(
            error,
            TickStepError::RequestedAffordanceUnavailable {
                actor,
                def_id: ActionDefId(99),
                targets: Vec::new(),
                payload_override: None,
            }
        );
    }

    #[test]
    fn resolve_affordance_uses_shared_request_binding_rule() {
        let _guard = test_lock().lock().unwrap();
        reset_hooks();
        let (world, _event_log, _scheduler, controller, _rng, _recipes, defs, handlers) =
            build_state();
        let actor = controlled_actor(&controller);
        let view = crate::OmniscientBeliefView::new(&world);
        let def = defs.get(ActionDefId(0)).unwrap();
        let affordance = get_affordances(&view, actor, &defs, &handlers)
            .into_iter()
            .find(|affordance| affordance.matches_request_identity(def, actor, &[], None))
            .expect("expected the no-target affordance registered for the test actor");

        let resolved = resolve_affordance(
            &world,
            &defs,
            &handlers,
            actor,
            affordance.def_id,
            &[],
            None,
        )
        .unwrap();
        assert_eq!(resolved.def_id, affordance.def_id);
        assert!(resolved.bound_targets.is_empty());

        let error = resolve_affordance(
            &world,
            &defs,
            &handlers,
            actor,
            affordance.def_id,
            &[actor],
            None,
        )
        .unwrap_err();
        assert!(matches!(
            error,
            TickStepError::RequestedAffordanceUnavailable {
                actor: err_actor,
                def_id: err_def_id,
                ..
            } if err_actor == actor && err_def_id == affordance.def_id
        ));
    }

    #[test]
    fn cancel_action_aborts_and_removes_matching_active_action() {
        let _guard = test_lock().lock().unwrap();
        reset_hooks();
        let (
            mut world,
            mut event_log,
            mut scheduler,
            mut controller,
            mut rng,
            recipes,
            defs,
            handlers,
        ) = build_state();
        let actor = controlled_actor(&controller);

        scheduler.input_queue_mut().enqueue(
            Tick(0),
            crate::InputKind::RequestAction {
                actor,
                def_id: ActionDefId(0),
                targets: Vec::new(),
                payload_override: None,
            },
        );
        step_tick(
            &mut world,
            &mut event_log,
            &mut scheduler,
            &mut controller,
            &mut rng,
            TickStepServices {
                action_defs: &defs,
                action_handlers: &handlers,
                recipe_registry: &recipes,
                systems: &SystemDispatchTable::canonical_noop(),
                input_producer: None,
            },
        )
        .unwrap();
        let action_id = *scheduler.active_actions().keys().next().unwrap();

        scheduler.input_queue_mut().enqueue(
            Tick(1),
            crate::InputKind::CancelAction {
                actor,
                action_instance_id: action_id,
            },
        );

        let result = step_tick(
            &mut world,
            &mut event_log,
            &mut scheduler,
            &mut controller,
            &mut rng,
            TickStepServices {
                action_defs: &defs,
                action_handlers: &handlers,
                recipe_registry: &recipes,
                systems: &SystemDispatchTable::canonical_noop(),
                input_producer: None,
            },
        )
        .unwrap();

        assert_eq!(result.actions_aborted, 1);
        assert!(scheduler.active_actions().is_empty());
        assert_eq!(hook_log().lock().unwrap().aborts, vec![action_id]);
    }

    #[test]
    fn switch_control_mismatch_returns_structured_error() {
        let _guard = test_lock().lock().unwrap();
        reset_hooks();
        let (
            mut world,
            mut event_log,
            mut scheduler,
            mut controller,
            mut rng,
            recipes,
            defs,
            handlers,
        ) = build_state();
        let wrong = entity(999);
        let replacement = entity(1000);

        scheduler.input_queue_mut().enqueue(
            Tick(0),
            crate::InputKind::SwitchControl {
                from: Some(wrong),
                to: Some(replacement),
            },
        );

        let error = step_tick(
            &mut world,
            &mut event_log,
            &mut scheduler,
            &mut controller,
            &mut rng,
            TickStepServices {
                action_defs: &defs,
                action_handlers: &handlers,
                recipe_registry: &recipes,
                systems: &SystemDispatchTable::canonical_noop(),
                input_producer: None,
            },
        )
        .unwrap_err();

        assert_eq!(
            error,
            TickStepError::Control(crate::ControlError::MismatchedFrom {
                expected: Some(wrong),
                actual: controller.controlled_entity(),
            })
        );
    }

    #[test]
    fn active_actions_tick_in_sorted_instance_id_order() {
        let _guard = test_lock().lock().unwrap();
        reset_hooks();
        let (
            mut world,
            mut event_log,
            mut scheduler,
            mut controller,
            mut rng,
            recipes,
            defs,
            handlers,
        ) = build_state();
        let actor = controlled_actor(&controller);

        for _ in 0..3 {
            scheduler.input_queue_mut().enqueue(
                Tick(0),
                crate::InputKind::RequestAction {
                    actor,
                    def_id: ActionDefId(0),
                    targets: Vec::new(),
                    payload_override: None,
                },
            );
        }

        step_tick(
            &mut world,
            &mut event_log,
            &mut scheduler,
            &mut controller,
            &mut rng,
            TickStepServices {
                action_defs: &defs,
                action_handlers: &handlers,
                recipe_registry: &recipes,
                systems: &SystemDispatchTable::canonical_noop(),
                input_producer: None,
            },
        )
        .unwrap();

        assert_eq!(
            hook_log().lock().unwrap().ticks,
            vec![
                ActionInstanceId(0),
                ActionInstanceId(1),
                ActionInstanceId(2)
            ]
        );
    }

    #[test]
    fn completed_actions_are_removed_after_tick_progress() {
        let _guard = test_lock().lock().unwrap();
        reset_hooks();
        let (
            mut world,
            mut event_log,
            mut scheduler,
            mut controller,
            mut rng,
            recipes,
            defs,
            handlers,
        ) = build_state();
        let actor = controlled_actor(&controller);

        scheduler.input_queue_mut().enqueue(
            Tick(0),
            crate::InputKind::RequestAction {
                actor,
                def_id: ActionDefId(1),
                targets: Vec::new(),
                payload_override: None,
            },
        );

        let result = step_tick(
            &mut world,
            &mut event_log,
            &mut scheduler,
            &mut controller,
            &mut rng,
            TickStepServices {
                action_defs: &defs,
                action_handlers: &handlers,
                recipe_registry: &recipes,
                systems: &SystemDispatchTable::canonical_noop(),
                input_producer: None,
            },
        )
        .unwrap();

        assert_eq!(result.actions_completed, 1);
        assert!(scheduler.active_actions().is_empty());
    }

    #[test]
    fn dead_actor_actions_are_aborted_before_tick_progress() {
        let _guard = test_lock().lock().unwrap();
        reset_hooks();
        let (
            mut world,
            mut event_log,
            mut scheduler,
            mut controller,
            mut rng,
            recipes,
            defs,
            handlers,
        ) = build_state();
        let actor = controlled_actor(&controller);
        let action_id = ActionInstanceId(77);
        scheduler.insert_action(ActionInstance {
            instance_id: action_id,
            def_id: ActionDefId(0),
            payload: ActionPayload::None,
            actor,
            targets: Vec::new(),
            start_tick: Tick(0),
            remaining_duration: crate::ActionDuration::Finite(5),
            status: ActionStatus::Active,
            reservation_ids: Vec::new(),
            local_state: Some(ActionState::Empty),
        });
        {
            let mut txn = new_txn(&mut world, 1);
            txn.set_component_dead_at(actor, DeadAt(Tick(1))).unwrap();
            let _ = txn.commit(&mut event_log);
        }
        reset_hooks();

        let result = step_tick(
            &mut world,
            &mut event_log,
            &mut scheduler,
            &mut controller,
            &mut rng,
            TickStepServices {
                action_defs: &defs,
                action_handlers: &handlers,
                recipe_registry: &recipes,
                systems: &SystemDispatchTable::canonical_noop(),
                input_producer: None,
            },
        )
        .unwrap();

        assert_eq!(result.actions_aborted, 1);
        assert!(scheduler.active_actions().is_empty());
        assert_eq!(hook_log().lock().unwrap().aborts, vec![action_id]);
        assert!(hook_log().lock().unwrap().ticks.is_empty());
    }

    #[allow(clippy::items_after_statements)]
    #[test]
    fn actions_for_agents_who_die_in_systems_are_culled_before_tick_ends() {
        let _guard = test_lock().lock().unwrap();
        reset_hooks();
        let (
            mut world,
            mut event_log,
            mut scheduler,
            mut controller,
            mut rng,
            recipes,
            defs,
            handlers,
        ) = build_state();
        let actor = controlled_actor(&controller);
        let action_id = ActionInstanceId(88);
        scheduler.insert_action(ActionInstance {
            instance_id: action_id,
            def_id: ActionDefId(0),
            payload: ActionPayload::None,
            actor,
            targets: Vec::new(),
            start_tick: Tick(0),
            remaining_duration: crate::ActionDuration::Finite(5),
            status: ActionStatus::Active,
            reservation_ids: Vec::new(),
            local_state: Some(ActionState::Empty),
        });
        reset_hooks();

        #[allow(clippy::needless_pass_by_value)]
        fn kill_actor_system(context: SystemExecutionContext<'_>) -> Result<(), SystemError> {
            let actor = context
                .active_actions
                .values()
                .next()
                .map(|instance| instance.actor)
                .unwrap();
            let place = context.world.effective_place(actor);
            let mut txn = WorldTxn::new(
                context.world,
                context.tick,
                CauseRef::SystemTick(context.tick),
                Some(actor),
                place,
                VisibilitySpec::SamePlace,
                WitnessData::default(),
            );
            txn.set_component_dead_at(actor, DeadAt(context.tick))
                .map_err(|error| SystemError::new(error.to_string()))?;
            let _ = txn.commit(context.event_log);
            Ok(())
        }

        let result = step_tick(
            &mut world,
            &mut event_log,
            &mut scheduler,
            &mut controller,
            &mut rng,
            TickStepServices {
                action_defs: &defs,
                action_handlers: &handlers,
                recipe_registry: &recipes,
                systems: &SystemDispatchTable::from_handlers(
                    [kill_actor_system; crate::SystemId::ALL.len()],
                ),
                input_producer: None,
            },
        )
        .unwrap();

        assert_eq!(result.actions_aborted, 1);
        assert!(scheduler.active_actions().is_empty());
        assert_eq!(hook_log().lock().unwrap().aborts, vec![action_id]);
    }

    #[test]
    fn systems_run_in_manifest_order() {
        let _guard = test_lock().lock().unwrap();
        reset_hooks();
        let (
            mut world,
            mut event_log,
            mut scheduler,
            mut controller,
            mut rng,
            recipes,
            defs,
            handlers,
        ) = build_state();

        step_tick(
            &mut world,
            &mut event_log,
            &mut scheduler,
            &mut controller,
            &mut rng,
            TickStepServices {
                action_defs: &defs,
                action_handlers: &handlers,
                recipe_registry: &recipes,
                systems: &ordered_systems(),
                input_producer: None,
            },
        )
        .unwrap();

        assert_eq!(hook_log().lock().unwrap().systems, crate::SystemId::ALL);
    }

    #[test]
    fn systems_receive_active_actions_and_action_registry_through_context() {
        let _guard = test_lock().lock().unwrap();
        reset_hooks();
        let (
            mut world,
            mut event_log,
            mut scheduler,
            mut controller,
            mut rng,
            recipes,
            defs,
            handlers,
        ) = build_state();
        let actor = controlled_actor(&controller);

        scheduler.input_queue_mut().enqueue(
            Tick(0),
            InputKind::RequestAction {
                actor,
                def_id: ActionDefId(0),
                targets: Vec::new(),
                payload_override: None,
            },
        );

        let _ = step_tick(
            &mut world,
            &mut event_log,
            &mut scheduler,
            &mut controller,
            &mut rng,
            TickStepServices {
                action_defs: &defs,
                action_handlers: &handlers,
                recipe_registry: &recipes,
                systems: &ordered_systems(),
                input_producer: None,
            },
        )
        .unwrap();

        let log = hook_log().lock().unwrap().clone();
        assert_eq!(log.systems, crate::SystemId::ALL);
        assert_eq!(
            log.system_active_action_counts,
            vec![1; crate::SystemId::ALL.len()]
        );
        assert_eq!(
            log.system_def_counts,
            vec![defs.iter().count(); crate::SystemId::ALL.len()]
        );
    }

    #[test]
    fn input_producer_runs_before_current_tick_input_drain() {
        let _guard = test_lock().lock().unwrap();
        reset_hooks();
        let (
            mut world,
            mut event_log,
            mut scheduler,
            mut controller,
            mut rng,
            recipes,
            defs,
            handlers,
        ) = build_state();
        let actor = controlled_actor(&controller);
        let mut producer = QueueingProducer {
            actor,
            def_id: ActionDefId(1),
            observed_pending_replans: Vec::new(),
        };
        let systems = SystemDispatchTable::canonical_noop();

        let result = step_tick(
            &mut world,
            &mut event_log,
            &mut scheduler,
            &mut controller,
            &mut rng,
            TickStepServices {
                action_defs: &defs,
                action_handlers: &handlers,
                recipe_registry: &recipes,
                systems: &systems,
                input_producer: Some(&mut producer),
            },
        )
        .unwrap();

        assert_eq!(result.inputs_processed, 1);
        assert_eq!(result.actions_started, 1);
        assert_eq!(producer.observed_pending_replans, vec![0]);
    }

    #[test]
    fn retained_replans_are_visible_to_next_tick_input_producer() {
        let _guard = test_lock().lock().unwrap();
        reset_hooks();
        let (
            mut world,
            mut event_log,
            mut scheduler,
            mut controller,
            mut rng,
            recipes,
            defs,
            handlers,
        ) = build_state();
        let actor = controlled_actor(&controller);
        let action_id = ActionInstanceId(42);
        scheduler.insert_action(ActionInstance {
            instance_id: action_id,
            def_id: ActionDefId(0),
            payload: ActionPayload::None,
            actor,
            targets: Vec::new(),
            start_tick: Tick(0),
            remaining_duration: crate::ActionDuration::Finite(5),
            status: ActionStatus::Active,
            reservation_ids: Vec::new(),
            local_state: Some(ActionState::Empty),
        });
        scheduler.input_queue_mut().enqueue(
            Tick(0),
            InputKind::CancelAction {
                actor,
                action_instance_id: action_id,
            },
        );
        let systems = SystemDispatchTable::canonical_noop();

        let _ = step_tick(
            &mut world,
            &mut event_log,
            &mut scheduler,
            &mut controller,
            &mut rng,
            TickStepServices {
                action_defs: &defs,
                action_handlers: &handlers,
                recipe_registry: &recipes,
                systems: &systems,
                input_producer: None,
            },
        )
        .unwrap();
        assert_eq!(scheduler.pending_replans().len(), 1);

        let mut producer = QueueingProducer {
            actor,
            def_id: ActionDefId(1),
            observed_pending_replans: Vec::new(),
        };
        let _ = step_tick(
            &mut world,
            &mut event_log,
            &mut scheduler,
            &mut controller,
            &mut rng,
            TickStepServices {
                action_defs: &defs,
                action_handlers: &handlers,
                recipe_registry: &recipes,
                systems: &systems,
                input_producer: Some(&mut producer),
            },
        )
        .unwrap();

        assert_eq!(producer.observed_pending_replans, vec![1]);
        assert!(scheduler.pending_replans().is_empty());
    }

    #[test]
    fn identical_runs_produce_identical_results_and_logs() {
        let _guard = test_lock().lock().unwrap();
        reset_hooks();
        let (
            mut world_a,
            mut log_a,
            mut scheduler_a,
            mut controller_a,
            mut rng_a,
            recipes,
            defs,
            handlers,
        ) = build_state();
        let actor = controlled_actor(&controller_a);
        scheduler_a.input_queue_mut().enqueue(
            Tick(0),
            crate::InputKind::RequestAction {
                actor,
                def_id: ActionDefId(0),
                targets: Vec::new(),
                payload_override: None,
            },
        );

        let mut world_b = world_a.clone();
        let mut log_b = log_a.clone();
        let mut scheduler_b = scheduler_a.clone();
        let mut controller_b = controller_a.clone();
        let mut rng_b = rng_a.clone();

        let result_a = step_tick(
            &mut world_a,
            &mut log_a,
            &mut scheduler_a,
            &mut controller_a,
            &mut rng_a,
            TickStepServices {
                action_defs: &defs,
                action_handlers: &handlers,
                recipe_registry: &recipes,
                systems: &SystemDispatchTable::canonical_noop(),
                input_producer: None,
            },
        )
        .unwrap();
        let result_b = step_tick(
            &mut world_b,
            &mut log_b,
            &mut scheduler_b,
            &mut controller_b,
            &mut rng_b,
            TickStepServices {
                action_defs: &defs,
                action_handlers: &handlers,
                recipe_registry: &recipes,
                systems: &SystemDispatchTable::canonical_noop(),
                input_producer: None,
            },
        )
        .unwrap();

        assert_eq!(result_a, result_b);
        assert_eq!(scheduler_a, scheduler_b);
        assert_eq!(controller_a, controller_b);
        assert_eq!(log_a, log_b);
        assert_eq!(world_a, world_b);
    }
}
