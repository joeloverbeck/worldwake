use crate::{
    evaluate_precondition, AbortReason, ActionDefRegistry, ActionError, ActionHandlerRegistry,
    ActionInstance, ActionInstanceId, ActionProgress, ActionStatus, KnowledgeView,
    WorldKnowledgeView,
};
use std::collections::BTreeMap;
use worldwake_core::{CauseRef, EventLog, EventTag, Tick, WitnessData, World, WorldTxn};

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct TickActionContext {
    pub cause: CauseRef,
    pub tick: Tick,
}

pub struct TickActionAuthority<'a> {
    pub active_actions: &'a mut BTreeMap<ActionInstanceId, ActionInstance>,
    pub world: &'a mut World,
    pub event_log: &'a mut EventLog,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TickOutcome {
    Continuing,
    Committed,
    Aborted { reason: AbortReason },
}

pub fn tick_action(
    instance_id: ActionInstanceId,
    registry: &ActionDefRegistry,
    handler_registry: &ActionHandlerRegistry,
    authority: TickActionAuthority<'_>,
    context: TickActionContext,
) -> Result<TickOutcome, ActionError> {
    let TickActionAuthority {
        active_actions,
        world,
        event_log,
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
        context,
    );

    match result {
        Ok(TickOutcome::Continuing) => {
            let replaced = active_actions.insert(instance_id, instance);
            debug_assert!(replaced.is_none(), "active action should have been removed first");
            Ok(TickOutcome::Continuing)
        }
        Ok(outcome) => Ok(outcome),
        Err(err) => {
            let replaced = active_actions.insert(instance_id, instance);
            debug_assert!(replaced.is_none(), "active action should have been removed first");
            Err(err)
        }
    }
}

fn tick_action_inner(
    instance: &mut ActionInstance,
    registry: &ActionDefRegistry,
    handler_registry: &ActionHandlerRegistry,
    world: &mut World,
    event_log: &mut EventLog,
    context: TickActionContext,
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

    let actor_place = WorldKnowledgeView::new(world).effective_place(instance.actor);
    let mut txn = WorldTxn::new(
        world,
        context.tick,
        context.cause,
        Some(instance.actor),
        actor_place,
        def.visibility,
        WitnessData::default(),
    );

    if instance.remaining_ticks > 0 {
        instance.remaining_ticks -= 1;
    }

    let progress = (handler.on_tick)(instance, &mut txn)?;
    let should_finalize =
        matches!(progress, ActionProgress::Complete) || instance.remaining_ticks == 0;

    if !should_finalize {
        if txn_has_effects(&txn) {
            add_targets(&mut txn, &instance.targets);
            let _ = txn.commit(event_log);
        }
        return Ok(TickOutcome::Continuing);
    }

    let failure_reason = {
        let view = WorldKnowledgeView::new(&txn);
        def.commit_conditions
            .iter()
            .find(|precondition| {
                !evaluate_precondition(precondition, instance.actor, &instance.targets, &view)
            })
            .map(|precondition| AbortReason::CommitConditionFailed(format!("{precondition:?}")))
    };

    if let Some(reason) = failure_reason {
        instance.status = ActionStatus::Aborted;
        (handler.on_abort)(instance, &reason, &mut txn)?;
        release_reservations(&mut txn, &instance.reservation_ids)?;
        txn.add_tag(EventTag::ActionAborted);
        add_targets(&mut txn, &instance.targets);
        let _ = txn.commit(event_log);
        Ok(TickOutcome::Aborted { reason })
    } else {
        instance.status = ActionStatus::Committed;
        (handler.on_commit)(instance, &mut txn)?;
        release_reservations(&mut txn, &instance.reservation_ids)?;
        txn.add_tag(EventTag::ActionCommitted);
        for tag in &def.causal_event_tags {
            txn.add_tag(*tag);
        }
        add_targets(&mut txn, &instance.targets);
        let _ = txn.commit(event_log);
        Ok(TickOutcome::Committed)
    }
}

fn release_reservations(
    txn: &mut WorldTxn<'_>,
    reservation_ids: &[worldwake_core::ReservationId],
) -> Result<(), ActionError> {
    for reservation_id in reservation_ids.iter().rev().copied() {
        txn.release_reservation(reservation_id)
            .map_err(|err| ActionError::InternalError(err.to_string()))?;
    }
    Ok(())
}

fn add_targets(txn: &mut WorldTxn<'_>, targets: &[worldwake_core::EntityId]) {
    for target in targets {
        txn.add_target(*target);
    }
}

fn txn_has_effects(txn: &WorldTxn<'_>) -> bool {
    !txn.deltas().is_empty() || !txn.tags().is_empty()
}

#[cfg(test)]
mod tests {
    use super::{tick_action, TickActionAuthority, TickActionContext, TickOutcome};
    use crate::{
        start_action, AbortReason, ActionDef, ActionDefId, ActionDefRegistry, ActionError,
        ActionHandler, ActionHandlerId, ActionHandlerRegistry, ActionInstance, ActionInstanceId,
        ActionProgress, ActionState, ActionStatus, Affordance, Constraint, DurationExpr,
        Interruptibility, Precondition, ReservationReq, StartActionAuthority, StartActionContext,
        TargetSpec,
    };
    use std::collections::{BTreeMap, BTreeSet};
    use std::sync::{Mutex, OnceLock};
    use worldwake_core::{
        build_prototype_world, CauseRef, CommodityKind, ControlSource, EntityId, EntityKind,
        EventLog, EventTag, Quantity, Tick, VisibilitySpec, WitnessData, World, WorldTxn,
    };

    #[derive(Clone, Debug, Default, Eq, PartialEq)]
    struct HookState {
        tick_calls: usize,
        commit_calls: usize,
        abort_calls: usize,
        complete_on_tick: bool,
        mutate_on_tick: bool,
        fail_after_tick_mutation: bool,
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
    fn start_none(_instance: &ActionInstance) -> Result<Option<ActionState>, ActionError> {
        Ok(None)
    }

    fn tick_handler(
        _instance: &ActionInstance,
        txn: &mut WorldTxn<'_>,
    ) -> Result<ActionProgress, ActionError> {
        let mut state = hook_state().lock().unwrap();
        state.tick_calls += 1;
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
        Ok(if state.complete_on_tick {
            ActionProgress::Complete
        } else {
            ActionProgress::Continue
        })
    }

    #[allow(clippy::unnecessary_wraps)]
    fn commit_handler(
        _instance: &ActionInstance,
        _txn: &mut WorldTxn<'_>,
    ) -> Result<(), ActionError> {
        let mut state = hook_state().lock().unwrap();
        state.commit_calls += 1;
        Ok(())
    }

    #[allow(clippy::unnecessary_wraps)]
    fn abort_handler(
        _instance: &ActionInstance,
        reason: &AbortReason,
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
        duration: u32,
        reservation_requirements: Vec<ReservationReq>,
        commit_conditions: Vec<Precondition>,
        causal_event_tags: BTreeSet<EventTag>,
    ) -> ActionDef {
        ActionDef {
            id,
            name: format!("action-{}", id.0),
            actor_constraints: vec![Constraint::ActorAlive],
            targets: vec![TargetSpec::SpecificEntity(entity(99))],
            preconditions: vec![Precondition::TargetExists(0), Precondition::TargetAtActorPlace(0)],
            reservation_requirements,
            duration: DurationExpr::Fixed(duration),
            interruptibility: Interruptibility::FreelyInterruptible,
            commit_conditions,
            visibility: VisibilitySpec::SamePlace,
            causal_event_tags,
            handler,
        }
    }

    fn setup_actor_and_target(world: &mut World) -> (EntityId, EntityId) {
        let place = world.topology().place_ids().next().unwrap();
        let (actor, target) = {
            let mut txn = new_txn(world, 1);
            let actor = txn.create_agent("Aster", ControlSource::Ai).unwrap();
            let target = txn.create_item_lot(CommodityKind::Bread, Quantity(2)).unwrap();
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
        duration: u32,
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
        let instance_id = start_action(
            &affordance,
            &defs,
            &handlers,
            StartActionAuthority {
                active_actions: &mut active_actions,
                world: &mut world,
                event_log: &mut log,
                next_instance_id: &mut next_instance_id,
            },
            StartActionContext {
                cause: CauseRef::Bootstrap,
                tick: Tick(10),
            },
        )
        .unwrap();

        (world, log, active_actions, defs, handlers, instance_id, actor, target)
    }

    #[test]
    fn tick_action_decrements_remaining_ticks_and_reinserts_active_instance() {
        let _guard = test_lock().lock().unwrap();
        reset_hooks();
        let (mut world, mut log, mut active_actions, defs, handlers, instance_id, _, _) =
            start_sample_action(
                3,
                vec![ReservationReq { target_index: 0 }],
                vec![Precondition::ActorAlive],
                BTreeSet::from([EventTag::Travel]),
            );

        let outcome = tick_action(
            instance_id,
            &defs,
            &handlers,
            TickActionAuthority {
                active_actions: &mut active_actions,
                world: &mut world,
                event_log: &mut log,
            },
            TickActionContext {
                cause: CauseRef::Bootstrap,
                tick: Tick(11),
            },
        )
        .unwrap();

        assert_eq!(outcome, TickOutcome::Continuing);
        let instance = active_actions.get(&instance_id).unwrap();
        assert_eq!(instance.remaining_ticks, 2);
        assert_eq!(instance.status, ActionStatus::Active);
        assert_eq!(log.len(), 1);
        assert_eq!(hook_state().lock().unwrap().tick_calls, 1);
    }

    #[test]
    fn tick_action_commits_when_remaining_ticks_reach_zero() {
        let _guard = test_lock().lock().unwrap();
        reset_hooks();
        let (mut world, mut log, mut active_actions, defs, handlers, instance_id, _, target) =
            start_sample_action(
                1,
                vec![ReservationReq { target_index: 0 }],
                vec![Precondition::ActorAlive, Precondition::TargetExists(0)],
                BTreeSet::from([EventTag::Travel]),
            );

        let outcome = tick_action(
            instance_id,
            &defs,
            &handlers,
            TickActionAuthority {
                active_actions: &mut active_actions,
                world: &mut world,
                event_log: &mut log,
            },
            TickActionContext {
                cause: CauseRef::Bootstrap,
                tick: Tick(11),
            },
        )
        .unwrap();

        assert_eq!(outcome, TickOutcome::Committed);
        assert!(!active_actions.contains_key(&instance_id));
        assert!(world.reservations_for(target).is_empty());
        assert_eq!(log.events_by_tag(EventTag::ActionCommitted).len(), 1);
        let event_id = log.events_by_tag(EventTag::ActionCommitted)[0];
        let record = log.get(event_id).unwrap();
        assert!(record.tags.contains(&EventTag::ActionCommitted));
        assert!(record.tags.contains(&EventTag::Travel));
        assert_eq!(record.target_ids, vec![target]);

        let state = hook_state().lock().unwrap().clone();
        assert_eq!(state.tick_calls, 1);
        assert_eq!(state.commit_calls, 1);
        assert_eq!(state.abort_calls, 0);
    }

    #[test]
    fn tick_action_zero_duration_action_reaches_commit_without_underflow() {
        let _guard = test_lock().lock().unwrap();
        reset_hooks();
        let (mut world, mut log, mut active_actions, defs, handlers, instance_id, _, target) =
            start_sample_action(0, vec![], vec![Precondition::ActorAlive], BTreeSet::new());

        let outcome = tick_action(
            instance_id,
            &defs,
            &handlers,
            TickActionAuthority {
                active_actions: &mut active_actions,
                world: &mut world,
                event_log: &mut log,
            },
            TickActionContext {
                cause: CauseRef::Bootstrap,
                tick: Tick(11),
            },
        )
        .unwrap();

        assert_eq!(outcome, TickOutcome::Committed);
        assert!(!active_actions.contains_key(&instance_id));
        assert!(world.reservations_for(target).is_empty());
        assert_eq!(log.events_by_tag(EventTag::ActionCommitted).len(), 1);
    }

    #[test]
    fn tick_action_aborts_and_releases_reservations_when_commit_conditions_fail() {
        let _guard = test_lock().lock().unwrap();
        reset_hooks();
        let (mut world, mut log, mut active_actions, defs, handlers, instance_id, _, target) =
            start_sample_action(
                1,
                vec![ReservationReq { target_index: 0 }],
                vec![Precondition::TargetKind {
                    target_index: 0,
                    kind: EntityKind::Container,
                }],
                BTreeSet::from([EventTag::Travel]),
            );

        let outcome = tick_action(
            instance_id,
            &defs,
            &handlers,
            TickActionAuthority {
                active_actions: &mut active_actions,
                world: &mut world,
                event_log: &mut log,
            },
            TickActionContext {
                cause: CauseRef::Bootstrap,
                tick: Tick(11),
            },
        )
        .unwrap();

        assert_eq!(
            outcome,
            TickOutcome::Aborted {
                reason: AbortReason::CommitConditionFailed(
                    "TargetKind { target_index: 0, kind: Container }".to_string(),
                ),
            }
        );
        assert!(!active_actions.contains_key(&instance_id));
        assert!(world.reservations_for(target).is_empty());
        assert_eq!(log.events_by_tag(EventTag::ActionAborted).len(), 1);
        let record = log.get(log.events_by_tag(EventTag::ActionAborted)[0]).unwrap();
        assert!(record.tags.contains(&EventTag::ActionAborted));
        assert!(!record.tags.contains(&EventTag::Travel));
        assert_eq!(record.target_ids, vec![target]);

        let state = hook_state().lock().unwrap().clone();
        assert_eq!(state.tick_calls, 1);
        assert_eq!(state.commit_calls, 0);
        assert_eq!(state.abort_calls, 1);
        assert_eq!(
            state.abort_reasons,
            vec![AbortReason::CommitConditionFailed(
                "TargetKind { target_index: 0, kind: Container }".to_string(),
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
                3,
                vec![ReservationReq { target_index: 0 }],
                vec![Precondition::ActorAlive],
                BTreeSet::new(),
            );
        let before_agents = world.query_agent_data().count();

        let outcome = tick_action(
            instance_id,
            &defs,
            &handlers,
            TickActionAuthority {
                active_actions: &mut active_actions,
                world: &mut world,
                event_log: &mut log,
            },
            TickActionContext {
                cause: CauseRef::Bootstrap,
                tick: Tick(11),
            },
        )
        .unwrap();

        assert_eq!(outcome, TickOutcome::Continuing);
        assert_eq!(world.query_agent_data().count(), before_agents + 1);
        assert_eq!(log.len(), 2);
        let record = log.get(worldwake_core::EventId(1)).unwrap();
        assert_eq!(record.actor_id, Some(actor));
        assert_eq!(record.tick, Tick(11));
        assert!(!record.state_deltas.is_empty());
    }

    #[test]
    fn tick_action_does_not_emit_empty_event_for_noop_continuation() {
        let _guard = test_lock().lock().unwrap();
        reset_hooks();
        let (mut world, mut log, mut active_actions, defs, handlers, instance_id, _, _) =
            start_sample_action(
                3,
                vec![ReservationReq { target_index: 0 }],
                vec![Precondition::ActorAlive],
                BTreeSet::new(),
            );

        let outcome = tick_action(
            instance_id,
            &defs,
            &handlers,
            TickActionAuthority {
                active_actions: &mut active_actions,
                world: &mut world,
                event_log: &mut log,
            },
            TickActionContext {
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
                3,
                vec![ReservationReq { target_index: 0 }],
                vec![Precondition::ActorAlive],
                BTreeSet::new(),
            );
        let before_agents = world.query_agent_data().count();

        let err = tick_action(
            instance_id,
            &defs,
            &handlers,
            TickActionAuthority {
                active_actions: &mut active_actions,
                world: &mut world,
                event_log: &mut log,
            },
            TickActionContext {
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

        let err = tick_action(
            ActionInstanceId(77),
            &defs,
            &handlers,
            TickActionAuthority {
                active_actions: &mut active_actions,
                world: &mut world,
                event_log: &mut log,
            },
            TickActionContext {
                cause: CauseRef::Bootstrap,
                tick: Tick(11),
            },
        )
        .unwrap_err();

        assert_eq!(err, ActionError::UnknownActionInstance(ActionInstanceId(77)));
    }

    #[test]
    fn tick_action_returns_structured_error_for_non_active_instance() {
        let _guard = test_lock().lock().unwrap();
        reset_hooks();
        let (mut world, mut log, mut active_actions, defs, handlers, instance_id, _, _) =
            start_sample_action(
                3,
                vec![ReservationReq { target_index: 0 }],
                vec![Precondition::ActorAlive],
                BTreeSet::new(),
            );
        active_actions.get_mut(&instance_id).unwrap().status = ActionStatus::Committed;

        let err = tick_action(
            instance_id,
            &defs,
            &handlers,
            TickActionAuthority {
                active_actions: &mut active_actions,
                world: &mut world,
                event_log: &mut log,
            },
            TickActionContext {
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
