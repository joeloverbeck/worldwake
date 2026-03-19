use crate::{
    action_termination::{
        add_targets, finalize_failed_action, release_reservations, FailedActionTermination,
    },
    action_validation::evaluate_txn_precondition_authoritatively,
    AbortReason, ActionDefRegistry, ActionError, ActionExecutionAuthority, ActionExecutionContext,
    ActionHandlerRegistry, ActionInstance, ActionInstanceId, ActionProgress, ActionStatus,
    CommitOutcome, DeterministicRng, ExternalAbortReason, ReplanNeeded,
};
use worldwake_core::{EventLog, EventTag, WitnessData, World, WorldTxn};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TickOutcome {
    Continuing,
    Committed {
        outcome: CommitOutcome,
    },
    Aborted {
        reason: AbortReason,
        replan: ReplanNeeded,
    },
}

pub fn tick_action(
    instance_id: ActionInstanceId,
    registry: &ActionDefRegistry,
    handler_registry: &ActionHandlerRegistry,
    authority: ActionExecutionAuthority<'_>,
    context: ActionExecutionContext,
) -> Result<TickOutcome, ActionError> {
    let ActionExecutionAuthority {
        active_actions,
        world,
        event_log,
        rng,
    } = authority;

    let mut instance = active_actions
        .remove(&instance_id)
        .ok_or(ActionError::UnknownActionInstance(instance_id))?;

    let result = tick_action_inner(
        &mut instance,
        registry,
        handler_registry,
        world,
        event_log,
        rng,
        context,
    );

    match result {
        Ok(TickOutcome::Continuing) => {
            let replaced = active_actions.insert(instance_id, instance);
            debug_assert!(
                replaced.is_none(),
                "active action should have been removed first"
            );
            Ok(TickOutcome::Continuing)
        }
        Ok(outcome) => Ok(outcome),
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

fn abort_requested_during_tick(
    def: &crate::ActionDef,
    instance: &mut ActionInstance,
    handler: &crate::ActionHandler,
    txn: WorldTxn<'_>,
    event_log: &mut EventLog,
    rng: &mut DeterministicRng,
    reason: crate::ActionAbortRequestReason,
) -> Result<TickOutcome, ActionError> {
    let reason = AbortReason::external_abort(ExternalAbortReason::HandlerRequested { reason });
    let replan = finalize_failed_action(
        def,
        instance,
        handler,
        txn,
        event_log,
        rng,
        &FailedActionTermination {
            status: ActionStatus::Aborted,
            reason: reason.clone(),
            event_tag: EventTag::ActionAborted,
        },
    )?;
    Ok(TickOutcome::Aborted { reason, replan })
}

fn tick_action_inner(
    instance: &mut ActionInstance,
    registry: &ActionDefRegistry,
    handler_registry: &ActionHandlerRegistry,
    world: &mut World,
    event_log: &mut EventLog,
    rng: &mut DeterministicRng,
    context: ActionExecutionContext,
) -> Result<TickOutcome, ActionError> {
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

    let actor_place = world.effective_place(instance.actor);
    let mut txn = WorldTxn::new(
        world,
        context.tick,
        context.cause,
        Some(instance.actor),
        actor_place,
        def.visibility,
        WitnessData::default(),
    );

    let duration_elapsed = instance.remaining_duration.advance();
    let progress = match (handler.on_tick)(def, instance, rng, &mut txn) {
        Ok(progress) => progress,
        Err(ActionError::AbortRequested(reason)) => {
            return abort_requested_during_tick(def, instance, handler, txn, event_log, rng, reason);
        }
        Err(err) => return Err(err),
    };
    let should_finalize = matches!(progress, ActionProgress::Complete) || duration_elapsed;

    if !should_finalize {
        if txn_has_effects(&txn) {
            add_targets(&mut txn, &instance.targets);
            let _ = txn.commit(event_log);
        }
        return Ok(TickOutcome::Continuing);
    }

    let failure_reason = {
        def.commit_conditions
            .iter()
            .find(|precondition| {
                !evaluate_txn_precondition_authoritatively(
                    &txn,
                    **precondition,
                    instance.actor,
                    &instance.targets,
                )
            })
            .map(|precondition| AbortReason::commit_condition_failed(*precondition))
    };

    if let Some(reason) = failure_reason {
        let replan = finalize_failed_action(
            def,
            instance,
            handler,
            txn,
            event_log,
            rng,
            &FailedActionTermination {
                status: ActionStatus::Aborted,
                reason: reason.clone(),
                event_tag: EventTag::ActionAborted,
            },
        )?;
        Ok(TickOutcome::Aborted { reason, replan })
    } else {
        match (handler.on_commit)(def, instance, rng, &mut txn) {
            Ok(outcome) => {
                instance.status = ActionStatus::Committed;
                release_reservations(&mut txn, &instance.reservation_ids)?;
                txn.add_tag(EventTag::ActionCommitted);
                for tag in &def.causal_event_tags {
                    txn.add_tag(*tag);
                }
                add_targets(&mut txn, &instance.targets);
                let _ = txn.commit(event_log);
                Ok(TickOutcome::Committed { outcome })
            }
            Err(ActionError::AbortRequested(reason)) => {
                let reason =
                    AbortReason::external_abort(ExternalAbortReason::HandlerRequested { reason });
                let replan = finalize_failed_action(
                    def,
                    instance,
                    handler,
                    txn,
                    event_log,
                    rng,
                    &FailedActionTermination {
                        status: ActionStatus::Aborted,
                        reason: reason.clone(),
                        event_tag: EventTag::ActionAborted,
                    },
                )?;
                Ok(TickOutcome::Aborted { reason, replan })
            }
            Err(err) => Err(err),
        }
    }
}

fn txn_has_effects(txn: &WorldTxn<'_>) -> bool {
    !txn.deltas().is_empty() || !txn.tags().is_empty()
}

#[cfg(test)]
mod tests {
    use super::{tick_action, TickOutcome};
    use crate::{
        start_action, AbortReason, ActionDef, ActionDefRegistry, ActionDomain, ActionDuration,
        ActionError, ActionExecutionAuthority, ActionExecutionContext, ActionHandler,
        ActionHandlerId, ActionHandlerRegistry, ActionInstance, ActionInstanceId, ActionPayload,
        ActionProgress, ActionState, ActionStatus, Affordance, CommitOutcome, Constraint,
        DeterministicRng, DurationExpr, Interruptibility, Materialization, MaterializationTag,
        Precondition, ReplanNeeded, ReservationReq, TargetSpec,
    };
    use std::collections::{BTreeMap, BTreeSet};
    use std::num::NonZeroU32;
    use std::sync::{Mutex, OnceLock};
    use worldwake_core::{
        build_prototype_world, ActionDefId, BodyCostPerTick, CauseRef, CommodityKind,
        ControlSource, EntityId, EntityKind, EventLog, EventTag, EventView, Quantity, Seed, Tick,
        VisibilitySpec, WitnessData, World, WorldTxn,
    };

    #[derive(Clone, Debug, Default, Eq, PartialEq)]
    struct HookState {
        tick_calls: usize,
        commit_calls: usize,
        abort_calls: usize,
        complete_on_tick: bool,
        mutate_on_tick: bool,
        fail_after_tick_mutation: bool,
        abort_requested_on_tick: Option<crate::ActionAbortRequestReason>,
        replace_local_state: Option<ActionState>,
        abort_reasons: Vec<AbortReason>,
        commit_outcome: CommitOutcome,
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

    fn test_rng() -> DeterministicRng {
        DeterministicRng::new(Seed([0x44; 32]))
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
    fn start_empty_state(
        _def: &ActionDef,
        _instance: &ActionInstance,
        _rng: &mut DeterministicRng,
        _txn: &mut WorldTxn<'_>,
    ) -> Result<Option<ActionState>, ActionError> {
        Ok(Some(ActionState::Empty))
    }

    fn tick_handler(
        _def: &ActionDef,
        instance: &mut ActionInstance,
        _rng: &mut DeterministicRng,
        txn: &mut WorldTxn<'_>,
    ) -> Result<ActionProgress, ActionError> {
        let mut state = hook_state().lock().unwrap();
        state.tick_calls += 1;
        if let Some(next) = state.replace_local_state {
            instance.local_state = Some(next);
        }
        if state.mutate_on_tick {
            let name = format!("tick-agent-{}", state.tick_calls);
            txn.create_agent(&name, ControlSource::Ai)
                .map_err(|err| ActionError::InternalError(err.to_string()))?;
        }
        if state.fail_after_tick_mutation {
            return Err(ActionError::InternalError(
                "tick handler failed after staging mutation".to_string(),
            ));
        }
        if let Some(reason) = state.abort_requested_on_tick.clone() {
            return Err(ActionError::AbortRequested(reason));
        }
        Ok(if state.complete_on_tick {
            ActionProgress::Complete
        } else {
            ActionProgress::Continue
        })
    }

    #[allow(clippy::unnecessary_wraps)]
    fn commit_handler(
        _def: &ActionDef,
        _instance: &ActionInstance,
        _rng: &mut DeterministicRng,
        _txn: &mut WorldTxn<'_>,
    ) -> Result<CommitOutcome, ActionError> {
        let mut state = hook_state().lock().unwrap();
        state.commit_calls += 1;
        Ok(state.commit_outcome.clone())
    }

    #[allow(clippy::unnecessary_wraps)]
    fn abort_handler(
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
        duration: NonZeroU32,
        reservation_requirements: Vec<ReservationReq>,
        commit_conditions: Vec<Precondition>,
        causal_event_tags: BTreeSet<EventTag>,
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
            reservation_requirements,
            duration: DurationExpr::Fixed(duration),
            body_cost_per_tick: BodyCostPerTick::zero(),
            interruptibility: Interruptibility::FreelyInterruptible,
            commit_conditions,
            visibility: VisibilitySpec::SamePlace,
            causal_event_tags,
            payload: ActionPayload::None,
            handler,
        }
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
        duration: NonZeroU32,
        reservation_requirements: Vec<ReservationReq>,
        commit_conditions: Vec<Precondition>,
        causal_event_tags: BTreeSet<EventTag>,
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
            duration,
            reservation_requirements,
            commit_conditions,
            causal_event_tags,
        ));

        let mut handlers = ActionHandlerRegistry::new();
        handlers.register(ActionHandler::new(
            start_none,
            tick_handler,
            commit_handler,
            abort_handler,
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

    fn start_indefinite_sample_action() -> (
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
        defs.register(ActionDef {
            id: ActionDefId(0),
            name: "defend".to_string(),
            domain: ActionDomain::Combat,
            actor_constraints: vec![Constraint::ActorAlive],
            targets: vec![TargetSpec::SpecificEntity(entity(99))],
            preconditions: vec![
                Precondition::TargetExists(0),
                Precondition::TargetAtActorPlace(0),
            ],
            reservation_requirements: Vec::new(),
            duration: DurationExpr::Indefinite,
            body_cost_per_tick: BodyCostPerTick::zero(),
            interruptibility: Interruptibility::FreelyInterruptible,
            commit_conditions: vec![Precondition::ActorAlive],
            visibility: VisibilitySpec::SamePlace,
            causal_event_tags: BTreeSet::new(),
            payload: ActionPayload::None,
            handler: ActionHandlerId(0),
        });

        let mut handlers = ActionHandlerRegistry::new();
        handlers.register(ActionHandler::new(
            start_none,
            tick_handler,
            commit_handler,
            abort_handler,
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

    fn start_stateful_sample_action() -> (
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
            NonZeroU32::new(3).unwrap(),
            Vec::new(),
            vec![Precondition::ActorAlive],
            BTreeSet::new(),
        ));

        let mut handlers = ActionHandlerRegistry::new();
        handlers.register(ActionHandler::new(
            start_empty_state,
            tick_handler,
            commit_handler,
            abort_handler,
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
    fn tick_action_decrements_finite_duration_and_reinserts_active_instance() {
        let _guard = test_lock().lock().unwrap();
        reset_hooks();
        let (mut world, mut log, mut active_actions, defs, handlers, instance_id, _, _) =
            start_sample_action(
                NonZeroU32::new(3).unwrap(),
                vec![ReservationReq { target_index: 0 }],
                vec![Precondition::ActorAlive],
                BTreeSet::from([EventTag::Travel]),
            );
        let mut rng = test_rng();

        let outcome = tick_action(
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
        )
        .unwrap();

        assert_eq!(outcome, TickOutcome::Continuing);
        let instance = active_actions.get(&instance_id).unwrap();
        assert_eq!(instance.remaining_duration, ActionDuration::Finite(2));
        assert_eq!(instance.status, ActionStatus::Active);
        assert_eq!(log.len(), 1);
        assert_eq!(hook_state().lock().unwrap().tick_calls, 1);
    }

    #[test]
    fn tick_action_persists_updated_local_state_from_handler() {
        let _guard = test_lock().lock().unwrap();
        reset_hooks();
        hook_state().lock().unwrap().replace_local_state =
            Some(ActionState::Heal { medicine_spent: true });
        let (mut world, mut log, mut active_actions, defs, handlers, instance_id, _, _) =
            start_stateful_sample_action();
        let mut rng = test_rng();

        let outcome = tick_action(
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
        )
        .unwrap();

        assert_eq!(outcome, TickOutcome::Continuing);
        assert_eq!(
            active_actions.get(&instance_id).unwrap().local_state,
            Some(ActionState::Heal {
                medicine_spent: true,
            })
        );
    }

    #[test]
    fn tick_action_converts_handler_requested_tick_abort_into_action_abort() {
        let _guard = test_lock().lock().unwrap();
        reset_hooks();
        hook_state().lock().unwrap().abort_requested_on_tick =
            Some(crate::ActionAbortRequestReason::TargetHasNoWounds { target: entity(99) });
        let (mut world, mut log, mut active_actions, defs, handlers, instance_id, _, target) =
            start_sample_action(
                NonZeroU32::new(3).unwrap(),
                vec![ReservationReq { target_index: 0 }],
                vec![Precondition::ActorAlive],
                BTreeSet::new(),
            );
        let mut rng = test_rng();

        let outcome = tick_action(
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
        )
        .unwrap();

        assert!(matches!(outcome, TickOutcome::Aborted { .. }));
        assert!(!active_actions.contains_key(&instance_id));
        assert!(world.reservations_for(target).is_empty());
        assert_eq!(hook_state().lock().unwrap().abort_calls, 1);
        assert_eq!(log.events_by_tag(EventTag::ActionAborted).len(), 1);
    }

    #[test]
    fn tick_action_commits_when_finite_duration_reaches_zero() {
        let _guard = test_lock().lock().unwrap();
        reset_hooks();
        let (mut world, mut log, mut active_actions, defs, handlers, instance_id, _, target) =
            start_sample_action(
                NonZeroU32::MIN,
                vec![ReservationReq { target_index: 0 }],
                vec![Precondition::ActorAlive, Precondition::TargetExists(0)],
                BTreeSet::from([EventTag::Travel]),
            );
        let mut rng = test_rng();

        let outcome = tick_action(
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
        )
        .unwrap();

        assert_eq!(
            outcome,
            TickOutcome::Committed {
                outcome: CommitOutcome::empty(),
            }
        );
        assert!(!active_actions.contains_key(&instance_id));
        assert!(world.reservations_for(target).is_empty());
        assert_eq!(log.events_by_tag(EventTag::ActionCommitted).len(), 1);
        let event_id = log.events_by_tag(EventTag::ActionCommitted)[0];
        let record = log.get(event_id).unwrap();
        assert!(record.tags().contains(&EventTag::ActionCommitted));
        assert!(record.tags().contains(&EventTag::Travel));
        assert_eq!(record.target_ids(), vec![target]);

        let state = hook_state().lock().unwrap().clone();
        assert_eq!(state.tick_calls, 1);
        assert_eq!(state.commit_calls, 1);
        assert_eq!(state.abort_calls, 0);
    }

    #[test]
    fn tick_action_keeps_indefinite_actions_active_until_handler_completes() {
        let _guard = test_lock().lock().unwrap();
        reset_hooks();
        let (mut world, mut log, mut active_actions, defs, handlers, instance_id, _, _) =
            start_indefinite_sample_action();
        let mut rng = test_rng();

        let outcome = tick_action(
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
        )
        .unwrap();

        assert_eq!(outcome, TickOutcome::Continuing);
        assert_eq!(
            active_actions.get(&instance_id).unwrap().remaining_duration,
            ActionDuration::Indefinite
        );
        assert_eq!(hook_state().lock().unwrap().tick_calls, 1);
    }

    #[test]
    fn tick_action_commits_indefinite_actions_when_handler_reports_completion() {
        let _guard = test_lock().lock().unwrap();
        reset_hooks();
        hook_state().lock().unwrap().complete_on_tick = true;
        let (mut world, mut log, mut active_actions, defs, handlers, instance_id, _, _) =
            start_indefinite_sample_action();
        let mut rng = test_rng();

        let outcome = tick_action(
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
        )
        .unwrap();

        assert_eq!(
            outcome,
            TickOutcome::Committed {
                outcome: CommitOutcome::empty(),
            }
        );
        assert!(!active_actions.contains_key(&instance_id));
        assert_eq!(hook_state().lock().unwrap().commit_calls, 1);
        assert_eq!(log.events_by_tag(EventTag::ActionCommitted).len(), 1);
    }

    #[test]
    fn tick_action_returns_handler_commit_outcome() {
        let _guard = test_lock().lock().unwrap();
        reset_hooks();
        hook_state().lock().unwrap().commit_outcome = CommitOutcome {
            materializations: vec![Materialization {
                tag: MaterializationTag::SplitOffLot,
                entity: entity(77),
            }],
        };
        let (mut world, mut log, mut active_actions, defs, handlers, instance_id, _, _) =
            start_sample_action(
                NonZeroU32::MIN,
                Vec::new(),
                vec![Precondition::ActorAlive],
                BTreeSet::new(),
            );
        let mut rng = test_rng();

        let outcome = tick_action(
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
        )
        .unwrap();

        assert_eq!(
            outcome,
            TickOutcome::Committed {
                outcome: CommitOutcome {
                    materializations: vec![Materialization {
                        tag: MaterializationTag::SplitOffLot,
                        entity: entity(77),
                    }],
                },
            }
        );
    }

    #[test]
    fn tick_action_aborts_and_releases_reservations_when_commit_conditions_fail() {
        let _guard = test_lock().lock().unwrap();
        reset_hooks();
        let (mut world, mut log, mut active_actions, defs, handlers, instance_id, actor, target) =
            start_sample_action(
                NonZeroU32::MIN,
                vec![ReservationReq { target_index: 0 }],
                vec![Precondition::TargetKind {
                    target_index: 0,
                    kind: EntityKind::Container,
                }],
                BTreeSet::from([EventTag::Travel]),
            );
        let mut rng = test_rng();

        let outcome = tick_action(
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
        )
        .unwrap();

        assert_eq!(
            outcome,
            TickOutcome::Aborted {
                reason: AbortReason::commit_condition_failed(Precondition::TargetKind {
                    target_index: 0,
                    kind: EntityKind::Container,
                }),
                replan: ReplanNeeded {
                    agent: actor,
                    failed_action_def: ActionDefId(0),
                    failed_instance: instance_id,
                    reason: AbortReason::commit_condition_failed(Precondition::TargetKind {
                        target_index: 0,
                        kind: EntityKind::Container,
                    }),
                    tick: Tick(11),
                },
            }
        );
        assert!(!active_actions.contains_key(&instance_id));
        assert!(world.reservations_for(target).is_empty());
        assert_eq!(log.events_by_tag(EventTag::ActionAborted).len(), 1);
        let record = log
            .get(log.events_by_tag(EventTag::ActionAborted)[0])
            .unwrap();
        assert!(record.tags().contains(&EventTag::ActionAborted));
        assert!(!record.tags().contains(&EventTag::Travel));
        assert_eq!(record.target_ids(), vec![target]);

        let state = hook_state().lock().unwrap().clone();
        assert_eq!(state.tick_calls, 1);
        assert_eq!(state.commit_calls, 0);
        assert_eq!(state.abort_calls, 1);
        assert_eq!(
            state.abort_reasons,
            vec![AbortReason::commit_condition_failed(
                Precondition::TargetKind {
                    target_index: 0,
                    kind: EntityKind::Container,
                }
            )]
        );
    }

    #[test]
    fn tick_action_persists_continuing_tick_mutations() {
        let _guard = test_lock().lock().unwrap();
        reset_hooks();
        hook_state().lock().unwrap().mutate_on_tick = true;
        let (mut world, mut log, mut active_actions, defs, handlers, instance_id, actor, _) =
            start_sample_action(
                NonZeroU32::new(3).unwrap(),
                vec![ReservationReq { target_index: 0 }],
                vec![Precondition::ActorAlive],
                BTreeSet::new(),
            );
        let before_agents = world.query_agent_data().count();
        let mut rng = test_rng();

        let outcome = tick_action(
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
        )
        .unwrap();

        assert_eq!(outcome, TickOutcome::Continuing);
        assert_eq!(world.query_agent_data().count(), before_agents + 1);
        assert_eq!(log.len(), 2);
        let record = log.get(worldwake_core::EventId(1)).unwrap();
        assert_eq!(record.actor_id(), Some(actor));
        assert_eq!(record.tick(), Tick(11));
        assert!(!record.state_deltas().is_empty());
    }

    #[test]
    fn tick_action_does_not_emit_empty_event_for_noop_continuation() {
        let _guard = test_lock().lock().unwrap();
        reset_hooks();
        let (mut world, mut log, mut active_actions, defs, handlers, instance_id, _, _) =
            start_sample_action(
                NonZeroU32::new(3).unwrap(),
                vec![ReservationReq { target_index: 0 }],
                vec![Precondition::ActorAlive],
                BTreeSet::new(),
            );
        let mut rng = test_rng();

        let outcome = tick_action(
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
        )
        .unwrap();

        assert_eq!(outcome, TickOutcome::Continuing);
        assert_eq!(log.len(), 1);
    }

    #[test]
    fn tick_action_handler_error_after_staging_does_not_leak_world_state() {
        let _guard = test_lock().lock().unwrap();
        reset_hooks();
        {
            let mut state = hook_state().lock().unwrap();
            state.mutate_on_tick = true;
            state.fail_after_tick_mutation = true;
        }

        let (mut world, mut log, mut active_actions, defs, handlers, instance_id, _, _) =
            start_sample_action(
                NonZeroU32::new(3).unwrap(),
                vec![ReservationReq { target_index: 0 }],
                vec![Precondition::ActorAlive],
                BTreeSet::new(),
            );
        let before_agents = world.query_agent_data().count();
        let mut rng = test_rng();

        let err = tick_action(
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
        )
        .unwrap_err();

        assert_eq!(
            err,
            ActionError::InternalError("tick handler failed after staging mutation".to_string())
        );
        assert_eq!(world.query_agent_data().count(), before_agents);
        assert_eq!(log.len(), 1);
        assert_eq!(
            active_actions.get(&instance_id).unwrap().status,
            ActionStatus::Active
        );
    }

    #[test]
    fn tick_action_returns_structured_error_for_missing_instance() {
        let _guard = test_lock().lock().unwrap();
        reset_hooks();
        let mut world = World::new(build_prototype_world()).unwrap();
        let mut log = EventLog::new();
        let mut active_actions = BTreeMap::new();
        let defs = ActionDefRegistry::new();
        let handlers = ActionHandlerRegistry::new();
        let mut rng = test_rng();

        let err = tick_action(
            ActionInstanceId(77),
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
        )
        .unwrap_err();

        assert_eq!(
            err,
            ActionError::UnknownActionInstance(ActionInstanceId(77))
        );
    }

    #[test]
    fn tick_action_returns_structured_error_for_non_active_instance() {
        let _guard = test_lock().lock().unwrap();
        reset_hooks();
        let (mut world, mut log, mut active_actions, defs, handlers, instance_id, _, _) =
            start_sample_action(
                NonZeroU32::new(3).unwrap(),
                vec![ReservationReq { target_index: 0 }],
                vec![Precondition::ActorAlive],
                BTreeSet::new(),
            );
        active_actions.get_mut(&instance_id).unwrap().status = ActionStatus::Committed;
        let mut rng = test_rng();

        let err = tick_action(
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
        )
        .unwrap_err();

        assert_eq!(
            err,
            ActionError::InvalidActionStatus {
                instance_id,
                status: ActionStatus::Committed,
            }
        );
        assert_eq!(
            active_actions.get(&instance_id).unwrap().status,
            ActionStatus::Committed
        );
    }
}
