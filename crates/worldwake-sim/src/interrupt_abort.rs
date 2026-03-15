use crate::{
    action_termination::{finalize_failed_action, FailedActionTermination},
    ActionDefRegistry, ActionError, ActionExecutionAuthority, ActionExecutionContext,
    ActionHandlerRegistry, ActionInstance, ActionInstanceId, ActionStatus, ExternalAbortReason,
    InterruptReason, Interruptibility, ReplanNeeded,
};
use worldwake_core::{EventLog, EventTag, WitnessData, World, WorldTxn};

pub fn interrupt_action(
    instance_id: ActionInstanceId,
    registry: &ActionDefRegistry,
    handler_registry: &ActionHandlerRegistry,
    authority: ActionExecutionAuthority<'_>,
    context: ActionExecutionContext,
    reason: InterruptReason,
) -> Result<ReplanNeeded, ActionError> {
    transition_action(
        instance_id,
        registry,
        handler_registry,
        authority,
        context,
        &TransitionKind::Interrupt(reason),
    )
}

pub fn abort_action(
    instance_id: ActionInstanceId,
    registry: &ActionDefRegistry,
    handler_registry: &ActionHandlerRegistry,
    authority: ActionExecutionAuthority<'_>,
    context: ActionExecutionContext,
    reason: ExternalAbortReason,
) -> Result<ReplanNeeded, ActionError> {
    transition_action(
        instance_id,
        registry,
        handler_registry,
        authority,
        context,
        &TransitionKind::Abort(reason),
    )
}

#[derive(Clone)]
enum TransitionKind {
    Interrupt(InterruptReason),
    Abort(ExternalAbortReason),
}

struct TransitionRuntime<'a> {
    world: &'a mut World,
    event_log: &'a mut EventLog,
    rng: &'a mut crate::DeterministicRng,
}

fn transition_action(
    instance_id: ActionInstanceId,
    registry: &ActionDefRegistry,
    handler_registry: &ActionHandlerRegistry,
    authority: ActionExecutionAuthority<'_>,
    context: ActionExecutionContext,
    kind: &TransitionKind,
) -> Result<ReplanNeeded, ActionError> {
    let ActionExecutionAuthority {
        active_actions,
        world,
        event_log,
        rng,
    } = authority;

    let mut instance = active_actions
        .remove(&instance_id)
        .ok_or(ActionError::UnknownActionInstance(instance_id))?;

    let result = transition_action_inner(
        &mut instance,
        registry,
        handler_registry,
        TransitionRuntime {
            world,
            event_log,
            rng,
        },
        context,
        kind,
    );

    match result {
        Ok(replan) => Ok(replan),
        Err(err) => {
            let replaced = active_actions.insert(instance_id, instance);
            debug_assert!(
                replaced.is_none(),
                "active action should have been removed first"
            );
            Err(err)
        }
    }
}

fn transition_action_inner(
    instance: &mut ActionInstance,
    registry: &ActionDefRegistry,
    handler_registry: &ActionHandlerRegistry,
    runtime: TransitionRuntime<'_>,
    context: ActionExecutionContext,
    kind: &TransitionKind,
) -> Result<ReplanNeeded, ActionError> {
    let TransitionRuntime {
        world,
        event_log,
        rng,
    } = runtime;

    if instance.status != ActionStatus::Active {
        return Err(ActionError::InvalidActionStatus {
            instance_id: instance.instance_id,
            status: instance.status,
        });
    }

    let def = registry
        .get(instance.def_id)
        .ok_or(ActionError::UnknownActionDef(instance.def_id))?;
    let handler = handler_registry
        .get(def.handler)
        .ok_or(ActionError::UnknownActionHandler(def.handler))?;

    let termination = match kind {
        TransitionKind::Interrupt(reason) => {
            if def.interruptibility == Interruptibility::NonInterruptible {
                return Err(ActionError::InterruptBlocked {
                    instance_id: instance.instance_id,
                    interruptibility: def.interruptibility,
                });
            }
            FailedActionTermination {
                status: ActionStatus::Interrupted,
                reason: crate::AbortReason::interrupted(*reason),
                event_tag: EventTag::ActionInterrupted,
            }
        }
        TransitionKind::Abort(reason) => FailedActionTermination {
            status: ActionStatus::Aborted,
            reason: crate::AbortReason::external_abort(reason.clone()),
            event_tag: EventTag::ActionAborted,
        },
    };

    let actor_place = world.effective_place(instance.actor);
    let txn = WorldTxn::new(
        world,
        context.tick,
        context.cause,
        Some(instance.actor),
        actor_place,
        def.visibility,
        WitnessData::default(),
    );

    finalize_failed_action(def, instance, handler, txn, event_log, rng, &termination)
}

#[cfg(test)]
mod tests {
    use super::{abort_action, interrupt_action};
    use crate::{
        start_action, AbortReason, ActionDef, ActionDefRegistry, ActionDomain, ActionError,
        ActionExecutionAuthority, ActionExecutionContext, ActionHandler, ActionHandlerId,
        ActionHandlerRegistry, ActionInstance, ActionInstanceId, ActionPayload, ActionProgress,
        ActionState, ActionStatus, Affordance, Constraint, DeterministicRng, DurationExpr,
        ExternalAbortReason, InterruptReason, Interruptibility, Precondition, ReservationReq,
        TargetSpec,
    };
    use std::collections::{BTreeMap, BTreeSet};
    use std::num::NonZeroU32;
    use std::sync::{Mutex, OnceLock};
    use worldwake_core::{
        build_prototype_world, ActionDefId, BodyCostPerTick, CauseRef, CommodityKind,
        ControlSource, EntityId, EventLog, EventTag, EventView, Quantity, Seed, Tick,
        VisibilitySpec, WitnessData, World, WorldTxn,
    };

    #[derive(Clone, Debug, Default, Eq, PartialEq)]
    struct HookState {
        abort_calls: usize,
        abort_reasons: Vec<AbortReason>,
    }

    fn test_lock() -> &'static Mutex<()> {
        static TEST_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        TEST_LOCK.get_or_init(|| Mutex::new(()))
    }

    fn hook_state() -> &'static Mutex<HookState> {
        static HOOK_STATE: OnceLock<Mutex<HookState>> = OnceLock::new();
        HOOK_STATE.get_or_init(|| Mutex::new(HookState::default()))
    }

    fn reset_hooks() {
        *hook_state().lock().unwrap() = HookState::default();
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

    fn commit_txn(txn: WorldTxn<'_>) {
        let mut log = EventLog::new();
        let _ = txn.commit(&mut log);
    }

    #[allow(clippy::unnecessary_wraps)]
    fn start_none(
        _def: &ActionDef,
        _instance: &ActionInstance,
        _rng: &mut DeterministicRng,
        _txn: &mut WorldTxn<'_>,
    ) -> Result<Option<ActionState>, ActionError> {
        Ok(None)
    }

    #[allow(clippy::unnecessary_wraps)]
    fn tick_continue(
        _def: &ActionDef,
        _instance: &ActionInstance,
        _rng: &mut DeterministicRng,
        _txn: &mut WorldTxn<'_>,
    ) -> Result<ActionProgress, ActionError> {
        Ok(ActionProgress::Continue)
    }

    #[allow(clippy::unnecessary_wraps)]
    fn commit_noop(
        _def: &ActionDef,
        _instance: &ActionInstance,
        _rng: &mut DeterministicRng,
        _txn: &mut WorldTxn<'_>,
    ) -> Result<crate::CommitOutcome, ActionError> {
        Ok(crate::CommitOutcome::empty())
    }

    #[allow(clippy::unnecessary_wraps)]
    fn abort_hook(
        _def: &ActionDef,
        _instance: &ActionInstance,
        reason: &AbortReason,
        _rng: &mut DeterministicRng,
        _txn: &mut WorldTxn<'_>,
    ) -> Result<(), ActionError> {
        let mut state = hook_state().lock().unwrap();
        state.abort_calls += 1;
        state.abort_reasons.push(reason.clone());
        Ok(())
    }

    fn sample_def(
        id: ActionDefId,
        handler: ActionHandlerId,
        interruptibility: Interruptibility,
    ) -> ActionDef {
        ActionDef {
            id,
            name: format!("action-{}", id.0),
            domain: ActionDomain::Generic,
            actor_constraints: vec![Constraint::ActorAlive],
            targets: vec![TargetSpec::SpecificEntity(entity(99))],
            preconditions: vec![
                Precondition::TargetExists(0),
                Precondition::TargetAtActorPlace(0),
            ],
            reservation_requirements: vec![ReservationReq { target_index: 0 }],
            duration: DurationExpr::Fixed(NonZeroU32::new(3).unwrap()),
            body_cost_per_tick: BodyCostPerTick::zero(),
            interruptibility,
            commit_conditions: vec![Precondition::ActorAlive],
            visibility: VisibilitySpec::SamePlace,
            causal_event_tags: BTreeSet::new(),
            payload: ActionPayload::None,
            handler,
        }
    }

    fn test_rng() -> DeterministicRng {
        DeterministicRng::new(Seed([0x55; 32]))
    }

    fn setup_actor_and_target(world: &mut World) -> (EntityId, EntityId) {
        let place = world.topology().place_ids().next().unwrap();
        let (actor, target) = {
            let mut txn = new_txn(world, 1);
            let actor = txn.create_agent("Aster", ControlSource::Ai).unwrap();
            let target = txn
                .create_item_lot(CommodityKind::Bread, Quantity(2))
                .unwrap();
            commit_txn(txn);
            (actor, target)
        };
        {
            let mut txn = new_txn(world, 2);
            txn.set_ground_location(actor, place).unwrap();
            txn.set_ground_location(target, place).unwrap();
            commit_txn(txn);
        }
        (actor, target)
    }

    fn start_sample_action(
        interruptibility: Interruptibility,
    ) -> (
        World,
        EventLog,
        BTreeMap<ActionInstanceId, ActionInstance>,
        ActionDefRegistry,
        ActionHandlerRegistry,
        ActionInstanceId,
        EntityId,
        EntityId,
    ) {
        let mut world = World::new(build_prototype_world()).unwrap();
        let (actor, target) = setup_actor_and_target(&mut world);
        let affordance = Affordance {
            def_id: ActionDefId(0),
            actor,
            bound_targets: vec![target],
            payload_override: None,
            explanation: None,
        };

        let mut defs = ActionDefRegistry::new();
        defs.register(sample_def(
            ActionDefId(0),
            ActionHandlerId(0),
            interruptibility,
        ));

        let mut handlers = ActionHandlerRegistry::new();
        handlers.register(ActionHandler::new(
            start_none,
            tick_continue,
            commit_noop,
            abort_hook,
        ));

        let mut active_actions = BTreeMap::new();
        let mut log = EventLog::new();
        let mut next_instance_id = ActionInstanceId(5);
        let mut rng = test_rng();
        let instance_id = start_action(
            &affordance,
            &defs,
            &handlers,
            ActionExecutionAuthority {
                active_actions: &mut active_actions,
                world: &mut world,
                event_log: &mut log,
                rng: &mut rng,
            },
            &mut next_instance_id,
            ActionExecutionContext {
                cause: CauseRef::Bootstrap,
                tick: Tick(10),
            },
        )
        .unwrap();

        (
            world,
            log,
            active_actions,
            defs,
            handlers,
            instance_id,
            actor,
            target,
        )
    }

    #[test]
    fn interrupt_non_interruptible_returns_error_and_keeps_instance_active() {
        let _guard = test_lock().lock().unwrap();
        reset_hooks();
        let (mut world, mut log, mut active_actions, defs, handlers, instance_id, _, target) =
            start_sample_action(Interruptibility::NonInterruptible);
        let mut rng = test_rng();

        let err = interrupt_action(
            instance_id,
            &defs,
            &handlers,
            ActionExecutionAuthority {
                active_actions: &mut active_actions,
                world: &mut world,
                event_log: &mut log,
                rng: &mut rng,
            },
            ActionExecutionContext {
                cause: CauseRef::Bootstrap,
                tick: Tick(11),
            },
            InterruptReason::DangerNearby,
        )
        .unwrap_err();

        assert_eq!(
            err,
            ActionError::InterruptBlocked {
                instance_id,
                interruptibility: Interruptibility::NonInterruptible,
            }
        );
        assert_eq!(
            active_actions.get(&instance_id).unwrap().status,
            ActionStatus::Active
        );
        assert_eq!(world.reservations_for(target).len(), 1);
        assert_eq!(log.len(), 1);
        assert_eq!(hook_state().lock().unwrap().abort_calls, 0);
    }

    #[test]
    fn interrupt_freely_interruptible_releases_reservations_and_returns_replan() {
        let _guard = test_lock().lock().unwrap();
        reset_hooks();
        let (mut world, mut log, mut active_actions, defs, handlers, instance_id, actor, target) =
            start_sample_action(Interruptibility::FreelyInterruptible);
        let mut rng = test_rng();

        let replan = interrupt_action(
            instance_id,
            &defs,
            &handlers,
            ActionExecutionAuthority {
                active_actions: &mut active_actions,
                world: &mut world,
                event_log: &mut log,
                rng: &mut rng,
            },
            ActionExecutionContext {
                cause: CauseRef::Bootstrap,
                tick: Tick(11),
            },
            InterruptReason::DangerNearby,
        )
        .unwrap();

        assert!(!active_actions.contains_key(&instance_id));
        assert!(world.reservations_for(target).is_empty());
        assert_eq!(replan.agent, actor);
        assert_eq!(replan.failed_action_def, ActionDefId(0));
        assert_eq!(replan.failed_instance, instance_id);
        assert_eq!(
            replan.reason,
            AbortReason::Interrupted {
                kind: InterruptReason::DangerNearby,
                detail: None,
            }
        );
        assert_eq!(replan.tick, Tick(11));
        let record = log
            .get(log.events_by_tag(EventTag::ActionInterrupted)[0])
            .unwrap();
        assert!(record.tags().contains(&EventTag::ActionInterrupted));
        assert_eq!(record.target_ids(), vec![target]);
        let state = hook_state().lock().unwrap().clone();
        assert_eq!(state.abort_calls, 1);
        assert_eq!(state.abort_reasons, vec![replan.reason.clone()]);
    }

    #[test]
    fn interrupt_with_penalty_currently_uses_same_cleanup_path() {
        let _guard = test_lock().lock().unwrap();
        reset_hooks();
        let (mut world, mut log, mut active_actions, defs, handlers, instance_id, _, target) =
            start_sample_action(Interruptibility::InterruptibleWithPenalty);
        let mut rng = test_rng();

        let replan = interrupt_action(
            instance_id,
            &defs,
            &handlers,
            ActionExecutionAuthority {
                active_actions: &mut active_actions,
                world: &mut world,
                event_log: &mut log,
                rng: &mut rng,
            },
            ActionExecutionContext {
                cause: CauseRef::Bootstrap,
                tick: Tick(11),
            },
            InterruptReason::Reprioritized,
        )
        .unwrap();

        assert!(!active_actions.contains_key(&instance_id));
        assert!(world.reservations_for(target).is_empty());
        assert_eq!(
            replan.reason,
            AbortReason::Interrupted {
                kind: InterruptReason::Reprioritized,
                detail: None,
            }
        );
        assert_eq!(log.events_by_tag(EventTag::ActionInterrupted).len(), 1);
    }

    #[test]
    fn abort_always_succeeds_even_for_non_interruptible_actions() {
        let _guard = test_lock().lock().unwrap();
        reset_hooks();
        let (mut world, mut log, mut active_actions, defs, handlers, instance_id, actor, target) =
            start_sample_action(Interruptibility::NonInterruptible);
        let mut rng = test_rng();

        let replan = abort_action(
            instance_id,
            &defs,
            &handlers,
            ActionExecutionAuthority {
                active_actions: &mut active_actions,
                world: &mut world,
                event_log: &mut log,
                rng: &mut rng,
            },
            ActionExecutionContext {
                cause: CauseRef::Bootstrap,
                tick: Tick(11),
            },
            ExternalAbortReason::TargetDestroyed,
        )
        .unwrap();

        assert!(!active_actions.contains_key(&instance_id));
        assert!(world.reservations_for(target).is_empty());
        assert_eq!(replan.agent, actor);
        assert_eq!(replan.failed_instance, instance_id);
        assert_eq!(
            replan.reason,
            AbortReason::ExternalAbort {
                kind: ExternalAbortReason::TargetDestroyed,
                detail: None,
            }
        );
        let record = log
            .get(log.events_by_tag(EventTag::ActionAborted)[0])
            .unwrap();
        assert!(record.tags().contains(&EventTag::ActionAborted));
        let state = hook_state().lock().unwrap().clone();
        assert_eq!(state.abort_calls, 1);
        assert_eq!(state.abort_reasons, vec![replan.reason.clone()]);
    }
}
