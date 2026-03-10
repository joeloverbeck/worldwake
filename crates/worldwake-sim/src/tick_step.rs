use crate::scheduler::SchedulerActionRuntime;
use crate::{
    get_affordances, ActionDefId, ActionDefRegistry, ActionError, ActionExecutionContext,
    ActionHandlerRegistry, ActionInstanceId, ControlError, ControllerState, DeterministicRng,
    InputKind, Scheduler, SystemDispatchTable, SystemError, TickOutcome,
};
use std::collections::BTreeSet;
use std::fmt;
use worldwake_core::{
    CauseRef, EntityId, EventLog, EventTag, PendingEvent, Tick, VisibilitySpec, WitnessData, World,
};

#[derive(Copy, Clone)]
pub struct TickStepServices<'a> {
    pub action_defs: &'a ActionDefRegistry,
    pub action_handlers: &'a ActionHandlerRegistry,
    pub systems: &'a SystemDispatchTable,
}

struct TickStepRuntime<'a> {
    world: &'a mut World,
    event_log: &'a mut EventLog,
    scheduler: &'a mut Scheduler,
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
    RequestedAffordanceUnavailable {
        actor: EntityId,
        def_id: ActionDefId,
        targets: Vec<EntityId>,
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
            Self::RequestedAffordanceUnavailable {
                actor,
                def_id,
                targets,
            } => write!(
                f,
                "requested affordance is not currently available for actor {actor}: def {def_id}, targets {targets:?}"
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
    services: TickStepServices<'_>,
) -> Result<TickStepResult, TickStepError> {
    let mut runtime = TickStepRuntime {
        world,
        event_log,
        scheduler,
    };
    let tick = runtime.scheduler.current_tick();
    let events_before = runtime.event_log.len();
    let (inputs_processed, actions_started, actions_aborted) =
        process_inputs(&mut runtime, controller, tick, &services)?;
    let (actions_completed, progressed_action_aborts) =
        progress_active_actions(&mut runtime, tick, &services)?;
    let systems_ran = run_systems(&mut runtime, rng, tick, services)?;
    emit_end_of_tick_marker(runtime.event_log, tick);
    runtime.scheduler.increment_tick();

    Ok(TickStepResult {
        tick,
        inputs_processed,
        actions_started,
        actions_completed,
        actions_aborted: actions_aborted
            .checked_add(progressed_action_aborts)
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
        } => {
            let affordance =
                resolve_affordance(runtime.world, services.action_defs, actor, def_id, &targets)?;
            runtime
                .scheduler
                .start_affordance(
                    &affordance,
                    SchedulerActionRuntime {
                        action_defs: services.action_defs,
                        action_handlers: services.action_handlers,
                        world: runtime.world,
                        event_log: runtime.event_log,
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
            runtime
                .scheduler
                .abort_active_action(
                    action_instance_id,
                    SchedulerActionRuntime {
                        action_defs: services.action_defs,
                        action_handlers: services.action_handlers,
                        world: runtime.world,
                        event_log: runtime.event_log,
                    },
                    ActionExecutionContext {
                        cause: CauseRef::ExternalInput(sequence_no),
                        tick,
                    },
                    format!("cancelled by input {sequence_no}"),
                )
                .map_err(TickStepError::Action)?;
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
    actor: EntityId,
    def_id: ActionDefId,
    targets: &[EntityId],
) -> Result<crate::Affordance, TickStepError> {
    let view = crate::OmniscientBeliefView::new(world);
    get_affordances(&view, actor, action_defs)
        .into_iter()
        .find(|affordance| {
            affordance.actor == actor
                && affordance.def_id == def_id
                && affordance.bound_targets == targets
        })
        .ok_or(TickStepError::RequestedAffordanceUnavailable {
            actor,
            def_id,
            targets: targets.to_owned(),
        })
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
            TickOutcome::Aborted { .. } => {
                actions_aborted = actions_aborted
                    .checked_add(1)
                    .expect("tick-step action-abort counter overflowed");
            }
        }
    }

    Ok((actions_completed, actions_aborted))
}

fn run_systems(
    runtime: &mut TickStepRuntime<'_>,
    rng: &mut DeterministicRng,
    tick: Tick,
    services: TickStepServices<'_>,
) -> Result<u32, TickStepError> {
    let mut systems_ran = 0u32;

    for system_id in runtime
        .scheduler
        .system_manifest()
        .ordered_ids()
        .iter()
        .copied()
    {
        let mut system_rng = rng.substream(tick, system_id, 0);
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
    use super::{step_tick, TickStepError, TickStepResult, TickStepServices};
    use crate::{
        ActionDef, ActionDefId, ActionDefRegistry, ActionError, ActionHandler, ActionHandlerId,
        ActionHandlerRegistry, ActionInstance, ActionInstanceId, ActionPayload, ActionProgress,
        ActionState, ControllerState, DeterministicRng, DurationExpr, InputKind,
        Interruptibility, Scheduler, SystemDispatchTable, SystemError, SystemExecutionContext,
        SystemManifest,
    };
    use std::collections::BTreeSet;
    use std::num::NonZeroU32;
    use std::sync::{Mutex, OnceLock};
    use worldwake_core::{
        build_prototype_world, BodyCostPerTick, CauseRef, ControlSource, EntityId, EventLog,
        EventTag, Seed, Tick, VisibilitySpec, WitnessData, World, WorldTxn,
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
        _txn: &mut WorldTxn<'_>,
    ) -> Result<Option<ActionState>, ActionError> {
        hook_log().lock().unwrap().starts.push(instance.def_id);
        Ok(Some(ActionState::Empty))
    }

    #[allow(clippy::unnecessary_wraps)]
    fn tick_continue(
        _def: &ActionDef,
        instance: &ActionInstance,
        _txn: &mut WorldTxn<'_>,
    ) -> Result<ActionProgress, ActionError> {
        hook_log().lock().unwrap().ticks.push(instance.instance_id);
        Ok(ActionProgress::Continue)
    }

    #[allow(clippy::unnecessary_wraps)]
    fn tick_complete(
        _def: &ActionDef,
        instance: &ActionInstance,
        _txn: &mut WorldTxn<'_>,
    ) -> Result<ActionProgress, ActionError> {
        hook_log().lock().unwrap().ticks.push(instance.instance_id);
        Ok(ActionProgress::Complete)
    }

    #[allow(clippy::unnecessary_wraps)]
    fn commit_noop(
        _def: &ActionDef,
        _instance: &ActionInstance,
        _txn: &mut WorldTxn<'_>,
    ) -> Result<(), ActionError> {
        Ok(())
    }

    #[allow(clippy::unnecessary_wraps)]
    fn abort_record(
        _def: &ActionDef,
        instance: &ActionInstance,
        _reason: &crate::AbortReason,
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
            action_registry(),
            handler_registry(),
        )
    }

    fn controlled_actor(controller: &ControllerState) -> EntityId {
        controller.controlled_entity().unwrap()
    }

    #[test]
    fn empty_tick_increments_and_emits_end_of_tick_event() {
        let _guard = test_lock().lock().unwrap();
        reset_hooks();
        let (mut world, mut event_log, mut scheduler, mut controller, mut rng, defs, handlers) =
            build_state();

        let result = step_tick(
            &mut world,
            &mut event_log,
            &mut scheduler,
            &mut controller,
            &mut rng,
            TickStepServices {
                action_defs: &defs,
                action_handlers: &handlers,
                systems: &SystemDispatchTable::canonical_noop(),
            },
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
        let (mut world, mut event_log, mut scheduler, mut controller, mut rng, defs, handlers) =
            build_state();
        let actor = controlled_actor(&controller);

        scheduler.input_queue_mut().enqueue(
            Tick(0),
            crate::InputKind::RequestAction {
                actor,
                def_id: ActionDefId(1),
                targets: Vec::new(),
            },
        );
        scheduler.input_queue_mut().enqueue(
            Tick(0),
            crate::InputKind::RequestAction {
                actor,
                def_id: ActionDefId(0),
                targets: Vec::new(),
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
                systems: &SystemDispatchTable::canonical_noop(),
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
        let (mut world, mut event_log, mut scheduler, mut controller, mut rng, defs, handlers) =
            build_state();
        let actor = controlled_actor(&controller);

        scheduler.input_queue_mut().enqueue(
            Tick(0),
            crate::InputKind::RequestAction {
                actor,
                def_id: ActionDefId(99),
                targets: Vec::new(),
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
                systems: &SystemDispatchTable::canonical_noop(),
            },
        )
        .unwrap_err();

        assert_eq!(
            error,
            TickStepError::RequestedAffordanceUnavailable {
                actor,
                def_id: ActionDefId(99),
                targets: Vec::new(),
            }
        );
    }

    #[test]
    fn cancel_action_aborts_and_removes_matching_active_action() {
        let _guard = test_lock().lock().unwrap();
        reset_hooks();
        let (mut world, mut event_log, mut scheduler, mut controller, mut rng, defs, handlers) =
            build_state();
        let actor = controlled_actor(&controller);

        scheduler.input_queue_mut().enqueue(
            Tick(0),
            crate::InputKind::RequestAction {
                actor,
                def_id: ActionDefId(0),
                targets: Vec::new(),
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
                systems: &SystemDispatchTable::canonical_noop(),
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
                systems: &SystemDispatchTable::canonical_noop(),
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
        let (mut world, mut event_log, mut scheduler, mut controller, mut rng, defs, handlers) =
            build_state();
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
                systems: &SystemDispatchTable::canonical_noop(),
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
        let (mut world, mut event_log, mut scheduler, mut controller, mut rng, defs, handlers) =
            build_state();
        let actor = controlled_actor(&controller);

        for _ in 0..3 {
            scheduler.input_queue_mut().enqueue(
                Tick(0),
                crate::InputKind::RequestAction {
                    actor,
                    def_id: ActionDefId(0),
                    targets: Vec::new(),
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
                systems: &SystemDispatchTable::canonical_noop(),
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
        let (mut world, mut event_log, mut scheduler, mut controller, mut rng, defs, handlers) =
            build_state();
        let actor = controlled_actor(&controller);

        scheduler.input_queue_mut().enqueue(
            Tick(0),
            crate::InputKind::RequestAction {
                actor,
                def_id: ActionDefId(1),
                targets: Vec::new(),
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
                systems: &SystemDispatchTable::canonical_noop(),
            },
        )
        .unwrap();

        assert_eq!(result.actions_completed, 1);
        assert!(scheduler.active_actions().is_empty());
    }

    #[test]
    fn systems_run_in_manifest_order() {
        let _guard = test_lock().lock().unwrap();
        reset_hooks();
        let (mut world, mut event_log, mut scheduler, mut controller, mut rng, defs, handlers) =
            build_state();

        step_tick(
            &mut world,
            &mut event_log,
            &mut scheduler,
            &mut controller,
            &mut rng,
            TickStepServices {
                action_defs: &defs,
                action_handlers: &handlers,
                systems: &ordered_systems(),
            },
        )
        .unwrap();

        assert_eq!(hook_log().lock().unwrap().systems, crate::SystemId::ALL);
    }

    #[test]
    fn systems_receive_active_actions_and_action_registry_through_context() {
        let _guard = test_lock().lock().unwrap();
        reset_hooks();
        let (mut world, mut event_log, mut scheduler, mut controller, mut rng, defs, handlers) =
            build_state();
        let actor = controlled_actor(&controller);

        scheduler.input_queue_mut().enqueue(
            Tick(0),
            InputKind::RequestAction {
                actor,
                def_id: ActionDefId(0),
                targets: Vec::new(),
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
                systems: &ordered_systems(),
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
    fn identical_runs_produce_identical_results_and_logs() {
        let _guard = test_lock().lock().unwrap();
        reset_hooks();
        let (mut world_a, mut log_a, mut scheduler_a, mut controller_a, mut rng_a, defs, handlers) =
            build_state();
        let actor = controlled_actor(&controller_a);
        scheduler_a.input_queue_mut().enqueue(
            Tick(0),
            crate::InputKind::RequestAction {
                actor,
                def_id: ActionDefId(0),
                targets: Vec::new(),
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
                systems: &SystemDispatchTable::canonical_noop(),
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
                systems: &SystemDispatchTable::canonical_noop(),
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
