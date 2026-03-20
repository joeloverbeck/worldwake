use crate::{
    action_validation::validate_action_def_authoritatively, ActionDefRegistry, ActionDuration,
    ActionError, ActionExecutionAuthority, ActionExecutionContext, ActionHandlerRegistry,
    ActionInstance, ActionInstanceId, ActionStatus, Affordance,
};
use worldwake_core::{EventTag, Tick, TickRange, WitnessData, WorldError, WorldTxn};

pub fn start_action(
    affordance: &Affordance,
    registry: &ActionDefRegistry,
    handler_registry: &ActionHandlerRegistry,
    authority: ActionExecutionAuthority<'_>,
    next_instance_id: &mut ActionInstanceId,
    context: ActionExecutionContext,
) -> Result<ActionInstanceId, ActionError> {
    let ActionExecutionAuthority {
        active_actions,
        world,
        event_log,
        rng,
    } = authority;

    let def = registry
        .get(affordance.def_id)
        .ok_or(ActionError::UnknownActionDef(affordance.def_id))?;
    let handler = handler_registry
        .get(def.handler)
        .ok_or(ActionError::UnknownActionHandler(def.handler))?;

    validate_start_requirements(def, affordance, world)?;

    let actor_place = world.effective_place(affordance.actor);
    let effective_payload = resolve_effective_payload(affordance, def);
    validate_authoritative_payload(
        handler,
        def,
        registry,
        affordance.actor,
        &affordance.bound_targets,
        effective_payload,
        world,
    )?;

    let duration = resolve_action_duration(world, def, affordance)?;
    let mut txn = WorldTxn::new(
        world,
        context.tick,
        context.cause,
        Some(affordance.actor),
        actor_place,
        def.visibility,
        WitnessData::default(),
    );
    let mut reservation_ids = Vec::with_capacity(def.reservation_requirements.len());

    for req in &def.reservation_requirements {
        let target = affordance
            .bound_targets
            .get(usize::from(req.target_index))
            .copied()
            .ok_or_else(|| {
                ActionError::PreconditionFailed(format!(
                    "reservation target index {} missing from affordance binding",
                    req.target_index
                ))
            })?;
        if let Some(range) = reservation_range(context.tick, duration)? {
            match txn.try_reserve(target, affordance.actor, range) {
                Ok(reservation_id) => reservation_ids.push(reservation_id),
                Err(err) => {
                    release_reservations(&mut txn, &reservation_ids)?;
                    return Err(map_reservation_error(err, target));
                }
            }
        }
    }

    let instance_id = *next_instance_id;
    if active_actions.contains_key(&instance_id) {
        return Err(ActionError::InternalError(format!(
            "action instance id {instance_id} already exists in active actions"
        )));
    }

    let mut instance = build_action_instance(
        instance_id,
        def,
        affordance,
        context.tick,
        duration,
        reservation_ids,
    );

    instance.local_state = match (handler.on_start)(def, &instance, rng, &mut txn) {
        Ok(local_state) => local_state,
        Err(err) => {
            release_reservations(&mut txn, &instance.reservation_ids)?;
            return Err(err);
        }
    };

    *next_instance_id =
        ActionInstanceId(next_instance_id.0.checked_add(1).ok_or_else(|| {
            ActionError::InternalError("action instance id overflowed".to_string())
        })?);

    txn.add_tag(EventTag::ActionStarted);
    for target in &instance.targets {
        txn.add_target(*target);
    }
    let _ = txn.commit(event_log);
    let replaced = active_actions.insert(instance_id, instance);
    debug_assert!(replaced.is_none(), "active action id prechecked as unique");

    Ok(instance_id)
}

fn build_action_instance(
    instance_id: ActionInstanceId,
    def: &crate::ActionDef,
    affordance: &Affordance,
    start_tick: Tick,
    remaining_duration: ActionDuration,
    reservation_ids: Vec<worldwake_core::ReservationId>,
) -> ActionInstance {
    ActionInstance {
        instance_id,
        def_id: def.id,
        payload: affordance
            .payload_override
            .clone()
            .unwrap_or_else(|| def.payload.clone()),
        actor: affordance.actor,
        targets: affordance.bound_targets.clone(),
        start_tick,
        remaining_duration,
        status: ActionStatus::Active,
        reservation_ids,
        local_state: None,
    }
}

fn resolve_action_duration(
    world: &worldwake_core::World,
    def: &crate::ActionDef,
    affordance: &Affordance,
) -> Result<ActionDuration, ActionError> {
    def.duration
        .resolve_for(
            world,
            affordance.actor,
            &affordance.bound_targets,
            affordance.payload_override.as_ref().unwrap_or(&def.payload),
        )
        .map_err(ActionError::PreconditionFailed)
}

fn resolve_effective_payload<'a>(
    affordance: &'a Affordance,
    def: &'a crate::ActionDef,
) -> &'a crate::ActionPayload {
    affordance.payload_override.as_ref().unwrap_or(&def.payload)
}

fn validate_authoritative_payload(
    handler: &crate::ActionHandler,
    def: &crate::ActionDef,
    registry: &ActionDefRegistry,
    actor: worldwake_core::EntityId,
    targets: &[worldwake_core::EntityId],
    payload: &crate::ActionPayload,
    world: &worldwake_core::World,
) -> Result<(), ActionError> {
    (handler.authoritative_payload_is_valid)(def, registry, actor, targets, payload, world)
}

fn validate_start_requirements(
    def: &crate::ActionDef,
    affordance: &Affordance,
    world: &worldwake_core::World,
) -> Result<(), ActionError> {
    validate_action_def_authoritatively(world, def, affordance.actor, &affordance.bound_targets)
}

fn reservation_range(
    current_tick: Tick,
    duration: ActionDuration,
) -> Result<Option<TickRange>, ActionError> {
    match duration {
        ActionDuration::Finite(0) => Ok(None),
        ActionDuration::Finite(duration) => {
            let end = current_tick
                .0
                .checked_add(u64::from(duration))
                .ok_or_else(|| {
                    ActionError::InternalError("reservation range overflowed".to_string())
                })?;
            TickRange::new(current_tick, Tick(end))
                .map(Some)
                .map_err(|err| ActionError::InternalError(err.to_string()))
        }
        ActionDuration::Indefinite => Err(ActionError::PreconditionFailed(
            "indefinite actions cannot reserve targets until reservation lifecycle support exists"
                .to_string(),
        )),
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

fn map_reservation_error(err: WorldError, entity: worldwake_core::EntityId) -> ActionError {
    match err {
        WorldError::ConflictingReservation { .. } => ActionError::ReservationUnavailable(entity),
        WorldError::PreconditionFailed(msg) => ActionError::PreconditionFailed(msg),
        other => ActionError::InternalError(other.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::start_action;
    use crate::{
        AbortReason, ActionDef, ActionDefRegistry, ActionDomain, ActionDuration, ActionError,
        ActionExecutionAuthority, ActionExecutionContext, ActionHandler, ActionHandlerId,
        ActionHandlerRegistry, ActionInstanceId, ActionPayload, ActionProgress, ActionState,
        Affordance, CombatActionPayload, Constraint, DeterministicRng, DurationExpr,
        Interruptibility, Precondition, ReservationReq, TargetSpec,
    };
    use std::collections::{BTreeMap, BTreeSet};
    use std::num::NonZeroU32;
    use worldwake_core::{
        build_prototype_world, ActionDefId, BodyCostPerTick, CauseRef, CombatProfile,
        CombatWeaponRef, CommodityKind, ControlSource, EntityId, EventLog, EventTag, EventView,
        Quantity, Seed, Tick, TickRange, VisibilitySpec, WitnessData, World, WorldTxn,
    };

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
    fn start_empty(
        _def: &ActionDef,
        _instance: &crate::ActionInstance,
        _rng: &mut DeterministicRng,
        _txn: &mut WorldTxn<'_>,
    ) -> Result<Option<ActionState>, ActionError> {
        Ok(Some(ActionState::Empty))
    }

    #[allow(clippy::unnecessary_wraps)]
    fn start_none(
        _def: &ActionDef,
        _instance: &crate::ActionInstance,
        _rng: &mut DeterministicRng,
        _txn: &mut WorldTxn<'_>,
    ) -> Result<Option<ActionState>, ActionError> {
        Ok(None)
    }

    #[allow(clippy::unnecessary_wraps)]
    fn tick_continue(
        _def: &ActionDef,
        _instance: &mut crate::ActionInstance,
        _rng: &mut DeterministicRng,
        _txn: &mut WorldTxn<'_>,
    ) -> Result<ActionProgress, ActionError> {
        Ok(ActionProgress::Continue)
    }

    #[allow(clippy::unnecessary_wraps)]
    fn commit_noop(
        _def: &ActionDef,
        _instance: &crate::ActionInstance,
        _rng: &mut DeterministicRng,
        _txn: &mut WorldTxn<'_>,
    ) -> Result<crate::CommitOutcome, ActionError> {
        Ok(crate::CommitOutcome::empty())
    }

    #[allow(clippy::unnecessary_wraps)]
    fn abort_noop(
        _def: &ActionDef,
        _instance: &crate::ActionInstance,
        _reason: &AbortReason,
        _rng: &mut DeterministicRng,
        _txn: &mut WorldTxn<'_>,
    ) -> Result<(), ActionError> {
        Ok(())
    }

    fn test_rng() -> DeterministicRng {
        DeterministicRng::new(Seed([0x33; 32]))
    }

    fn sample_def(
        id: ActionDefId,
        handler: ActionHandlerId,
        actor_constraints: Vec<Constraint>,
        preconditions: Vec<Precondition>,
        reservation_requirements: Vec<ReservationReq>,
        duration: NonZeroU32,
    ) -> ActionDef {
        ActionDef {
            id,
            name: format!("action-{}", id.0),
            domain: ActionDomain::Generic,
            actor_constraints,
            targets: vec![TargetSpec::SpecificEntity(entity(99))],
            preconditions,
            reservation_requirements,
            duration: DurationExpr::Fixed(duration),
            body_cost_per_tick: BodyCostPerTick::zero(),
            interruptibility: Interruptibility::FreelyInterruptible,
            commit_conditions: vec![Precondition::ActorAlive],
            visibility: VisibilitySpec::SamePlace,
            causal_event_tags: BTreeSet::from([EventTag::ActionStarted]),
            payload: ActionPayload::None,
            handler,
        }
    }

    fn setup_actor_and_target(world: &mut World) -> (EntityId, EntityId, EntityId) {
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
        (actor, target, place)
    }

    #[test]
    fn start_action_creates_active_instance_and_emits_start_event() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let (actor, target, place) = setup_actor_and_target(&mut world);
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
            vec![Constraint::ActorAlive],
            vec![
                Precondition::TargetExists(0),
                Precondition::TargetAtActorPlace(0),
            ],
            vec![ReservationReq { target_index: 0 }],
            NonZeroU32::new(3).unwrap(),
        ));
        let mut handlers = ActionHandlerRegistry::new();
        handlers.register(ActionHandler::new(
            start_empty,
            tick_continue,
            commit_noop,
            abort_noop,
        ));
        let mut active_actions = BTreeMap::new();
        let mut log = EventLog::new();
        let mut next_instance_id = ActionInstanceId(12);
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
                tick: Tick(5),
            },
        )
        .unwrap();

        assert_eq!(instance_id, ActionInstanceId(12));
        let instance = active_actions.get(&instance_id).unwrap();
        assert_eq!(instance.def_id, ActionDefId(0));
        assert_eq!(instance.payload, defs.get(ActionDefId(0)).unwrap().payload);
        assert_eq!(instance.actor, actor);
        assert_eq!(instance.targets, vec![target]);
        assert_eq!(instance.start_tick, Tick(5));
        assert_eq!(instance.remaining_duration, ActionDuration::Finite(3));
        assert_eq!(instance.status, crate::ActionStatus::Active);
        assert_eq!(instance.local_state, Some(ActionState::Empty));
        assert_eq!(instance.reservation_ids.len(), 1);
        assert_eq!(next_instance_id, ActionInstanceId(13));
        assert_eq!(world.reservations_for(target).len(), 1);
        assert_eq!(log.len(), 1);
        assert_eq!(log.events_by_tag(EventTag::ActionStarted).len(), 1);

        let record = log.get(worldwake_core::EventId(0)).unwrap();
        assert_eq!(record.cause(), CauseRef::Bootstrap);
        assert_eq!(record.actor_id(), Some(actor));
        assert_eq!(record.place_id(), Some(place));
        assert_eq!(record.target_ids(), vec![target]);
        assert!(record.tags().contains(&EventTag::ActionStarted));
        assert_eq!(record.state_deltas().len(), 1);
    }

    #[test]
    fn start_action_uses_payload_override_when_resolving_combat_duration() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let actor = {
            let mut txn = new_txn(&mut world, 1);
            let actor = txn.create_agent("Aster", ControlSource::Ai).unwrap();
            txn.set_component_combat_profile(
                actor,
                CombatProfile::new(
                    worldwake_core::Permille::new(1000).unwrap(),
                    worldwake_core::Permille::new(700).unwrap(),
                    worldwake_core::Permille::new(600).unwrap(),
                    worldwake_core::Permille::new(550).unwrap(),
                    worldwake_core::Permille::new(75).unwrap(),
                    worldwake_core::Permille::new(20).unwrap(),
                    worldwake_core::Permille::new(15).unwrap(),
                    worldwake_core::Permille::new(120).unwrap(),
                    worldwake_core::Permille::new(30).unwrap(),
                    NonZeroU32::new(6).unwrap(),
                    NonZeroU32::new(10).unwrap(),
                ),
            )
            .unwrap();
            commit_txn(txn);
            actor
        };
        let affordance = Affordance {
            def_id: ActionDefId(0),
            actor,
            bound_targets: Vec::new(),
            payload_override: Some(ActionPayload::Combat(CombatActionPayload {
                target: entity(55),
                weapon: CombatWeaponRef::Commodity(CommodityKind::Bow),
            })),
            explanation: None,
        };
        let mut defs = ActionDefRegistry::new();
        defs.register(ActionDef {
            id: ActionDefId(0),
            name: "attack".to_string(),
            domain: ActionDomain::Combat,
            actor_constraints: vec![Constraint::ActorAlive],
            targets: Vec::new(),
            preconditions: vec![Precondition::ActorAlive],
            reservation_requirements: Vec::new(),
            duration: DurationExpr::CombatWeapon,
            body_cost_per_tick: BodyCostPerTick::zero(),
            interruptibility: Interruptibility::FreelyInterruptible,
            commit_conditions: vec![Precondition::ActorAlive],
            visibility: VisibilitySpec::SamePlace,
            causal_event_tags: BTreeSet::from([EventTag::ActionStarted]),
            payload: ActionPayload::Combat(CombatActionPayload {
                target: entity(56),
                weapon: CombatWeaponRef::Unarmed,
            }),
            handler: ActionHandlerId(0),
        });
        let mut handlers = ActionHandlerRegistry::new();
        handlers.register(ActionHandler::new(
            start_none,
            tick_continue,
            commit_noop,
            abort_noop,
        ));
        let mut active_actions = BTreeMap::new();
        let mut log = EventLog::new();
        let mut next_instance_id = ActionInstanceId(0);
        let mut rng = test_rng();

        let action_id = start_action(
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
                tick: Tick(2),
            },
        )
        .unwrap();

        assert_eq!(
            active_actions.get(&action_id).unwrap().remaining_duration,
            ActionDuration::Finite(
                CommodityKind::Bow
                    .spec()
                    .combat_weapon_profile
                    .unwrap()
                    .attack_duration_ticks
                    .get()
            )
        );
        assert_eq!(
            active_actions.get(&action_id).unwrap().payload,
            affordance.payload_override.unwrap()
        );
    }

    #[test]
    fn start_action_supports_indefinite_duration_when_no_reservations_are_needed() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let actor = {
            let mut txn = new_txn(&mut world, 1);
            let actor = txn.create_agent("Aster", ControlSource::Ai).unwrap();
            commit_txn(txn);
            actor
        };
        let affordance = Affordance {
            def_id: ActionDefId(0),
            actor,
            bound_targets: Vec::new(),
            payload_override: None,
            explanation: None,
        };
        let mut defs = ActionDefRegistry::new();
        defs.register(ActionDef {
            id: ActionDefId(0),
            name: "defend".to_string(),
            domain: ActionDomain::Combat,
            actor_constraints: vec![Constraint::ActorAlive],
            targets: Vec::new(),
            preconditions: vec![Precondition::ActorAlive],
            reservation_requirements: Vec::new(),
            duration: DurationExpr::Indefinite,
            body_cost_per_tick: BodyCostPerTick::zero(),
            interruptibility: Interruptibility::FreelyInterruptible,
            commit_conditions: vec![Precondition::ActorAlive],
            visibility: VisibilitySpec::SamePlace,
            causal_event_tags: BTreeSet::from([EventTag::ActionStarted]),
            payload: ActionPayload::None,
            handler: ActionHandlerId(0),
        });
        let mut handlers = ActionHandlerRegistry::new();
        handlers.register(ActionHandler::new(
            start_none,
            tick_continue,
            commit_noop,
            abort_noop,
        ));
        let mut active_actions = BTreeMap::new();
        let mut log = EventLog::new();
        let mut next_instance_id = ActionInstanceId(0);
        let mut rng = test_rng();

        let action_id = start_action(
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
                tick: Tick(2),
            },
        )
        .unwrap();

        assert_eq!(
            active_actions.get(&action_id).unwrap().remaining_duration,
            ActionDuration::Indefinite
        );
    }

    #[test]
    fn start_action_rejects_indefinite_duration_when_reservations_are_required() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let (actor, target, _place) = setup_actor_and_target(&mut world);
        let affordance = Affordance {
            def_id: ActionDefId(0),
            actor,
            bound_targets: vec![target],
            payload_override: None,
            explanation: None,
        };
        let mut defs = ActionDefRegistry::new();
        let mut def = sample_def(
            ActionDefId(0),
            ActionHandlerId(0),
            vec![Constraint::ActorAlive],
            vec![
                Precondition::TargetExists(0),
                Precondition::TargetAtActorPlace(0),
            ],
            vec![ReservationReq { target_index: 0 }],
            NonZeroU32::MIN,
        );
        def.duration = DurationExpr::Indefinite;
        defs.register(def);
        let mut handlers = ActionHandlerRegistry::new();
        handlers.register(ActionHandler::new(
            start_none,
            tick_continue,
            commit_noop,
            abort_noop,
        ));
        let mut active_actions = BTreeMap::new();
        let mut log = EventLog::new();
        let mut next_instance_id = ActionInstanceId(0);
        let mut rng = test_rng();

        let err = start_action(
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
                tick: Tick(2),
            },
        )
        .unwrap_err();

        assert_eq!(
            err,
            ActionError::PreconditionFailed(
                "indefinite actions cannot reserve targets until reservation lifecycle support exists"
                    .to_string()
            )
        );
    }

    #[test]
    fn start_action_revalidates_actor_constraints_against_authoritative_state() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let actor = {
            let mut txn = new_txn(&mut world, 1);
            let actor = txn.create_agent("Dormant", ControlSource::None).unwrap();
            commit_txn(txn);
            actor
        };
        let affordance = Affordance {
            def_id: ActionDefId(0),
            actor,
            bound_targets: Vec::new(),
            payload_override: None,
            explanation: None,
        };
        let mut defs = ActionDefRegistry::new();
        defs.register(sample_def(
            ActionDefId(0),
            ActionHandlerId(0),
            vec![Constraint::ActorHasControl],
            vec![Precondition::ActorAlive],
            Vec::new(),
            NonZeroU32::new(2).unwrap(),
        ));
        let mut handlers = ActionHandlerRegistry::new();
        handlers.register(ActionHandler::new(
            start_none,
            tick_continue,
            commit_noop,
            abort_noop,
        ));
        let mut active_actions = BTreeMap::new();
        let mut log = EventLog::new();
        let mut next_instance_id = ActionInstanceId(0);
        let mut rng = test_rng();

        let err = start_action(
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
                tick: Tick(3),
            },
        )
        .unwrap_err();

        assert_eq!(
            err,
            ActionError::ConstraintFailed("ActorHasControl".to_string())
        );
        assert_eq!(log.len(), 0);
        assert!(active_actions.is_empty());
        assert_eq!(next_instance_id, ActionInstanceId(0));
    }

    #[test]
    fn start_action_revalidates_preconditions_against_authoritative_state() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let (actor, target, _place) = setup_actor_and_target(&mut world);
        let other_place = world.topology().place_ids().nth(1).unwrap();
        {
            let mut txn = new_txn(&mut world, 4);
            txn.set_ground_location(target, other_place).unwrap();
            commit_txn(txn);
        }
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
            vec![Constraint::ActorAlive],
            vec![Precondition::TargetAtActorPlace(0)],
            Vec::new(),
            NonZeroU32::new(2).unwrap(),
        ));
        let mut handlers = ActionHandlerRegistry::new();
        handlers.register(ActionHandler::new(
            start_none,
            tick_continue,
            commit_noop,
            abort_noop,
        ));
        let mut active_actions = BTreeMap::new();
        let mut log = EventLog::new();
        let mut next_instance_id = ActionInstanceId(0);
        let mut rng = test_rng();

        let err = start_action(
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
                tick: Tick(5),
            },
        )
        .unwrap_err();

        assert_eq!(
            err,
            ActionError::PreconditionFailed("TargetAtActorPlace(0)".to_string())
        );
        assert_eq!(log.len(), 0);
        assert!(active_actions.is_empty());
        assert_eq!(next_instance_id, ActionInstanceId(0));
    }

    #[test]
    fn start_action_releases_acquired_reservations_when_later_one_conflicts() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let place = world.topology().place_ids().next().unwrap();
        let (actor, blocker, first_target, second_target) = {
            let mut txn = new_txn(&mut world, 1);
            let actor = txn.create_agent("Aster", ControlSource::Ai).unwrap();
            let blocker = txn.create_agent("Bram", ControlSource::Ai).unwrap();
            let first_target = txn
                .create_item_lot(CommodityKind::Bread, Quantity(1))
                .unwrap();
            let second_target = txn
                .create_item_lot(CommodityKind::Coin, Quantity(1))
                .unwrap();
            commit_txn(txn);
            (actor, blocker, first_target, second_target)
        };
        {
            let mut txn = new_txn(&mut world, 2);
            txn.set_ground_location(actor, place).unwrap();
            txn.set_ground_location(blocker, place).unwrap();
            txn.set_ground_location(first_target, place).unwrap();
            txn.set_ground_location(second_target, place).unwrap();
            commit_txn(txn);
        }
        {
            let mut txn = new_txn(&mut world, 3);
            txn.try_reserve(
                second_target,
                blocker,
                TickRange::new(Tick(5), Tick(8)).unwrap(),
            )
            .unwrap();
            commit_txn(txn);
        }
        let affordance = Affordance {
            def_id: ActionDefId(0),
            actor,
            bound_targets: vec![first_target, second_target],
            payload_override: None,
            explanation: None,
        };
        let mut defs = ActionDefRegistry::new();
        let mut def = sample_def(
            ActionDefId(0),
            ActionHandlerId(0),
            vec![Constraint::ActorAlive],
            vec![Precondition::TargetExists(0), Precondition::TargetExists(1)],
            vec![
                ReservationReq { target_index: 0 },
                ReservationReq { target_index: 1 },
            ],
            NonZeroU32::new(3).unwrap(),
        );
        def.targets = vec![
            TargetSpec::SpecificEntity(first_target),
            TargetSpec::SpecificEntity(second_target),
        ];
        defs.register(def);
        let mut handlers = ActionHandlerRegistry::new();
        handlers.register(ActionHandler::new(
            start_none,
            tick_continue,
            commit_noop,
            abort_noop,
        ));
        let mut active_actions = BTreeMap::new();
        let mut log = EventLog::new();
        let mut next_instance_id = ActionInstanceId(0);
        let mut rng = test_rng();

        let err = start_action(
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
                tick: Tick(5),
            },
        )
        .unwrap_err();

        assert_eq!(err, ActionError::ReservationUnavailable(second_target));
        assert_eq!(world.reservations_for(first_target), Vec::new());
        assert_eq!(world.reservations_for(second_target).len(), 1);
        assert_eq!(log.len(), 0);
        assert!(active_actions.is_empty());
        assert_eq!(next_instance_id, ActionInstanceId(0));
    }

    #[test]
    fn start_action_assigns_instance_ids_monotonically_across_calls() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let actor = {
            let mut txn = new_txn(&mut world, 1);
            let actor = txn.create_agent("Aster", ControlSource::Ai).unwrap();
            commit_txn(txn);
            actor
        };
        let affordance = Affordance {
            def_id: ActionDefId(0),
            actor,
            bound_targets: Vec::new(),
            payload_override: None,
            explanation: None,
        };
        let mut defs = ActionDefRegistry::new();
        defs.register(sample_def(
            ActionDefId(0),
            ActionHandlerId(0),
            vec![Constraint::ActorAlive],
            vec![Precondition::ActorAlive],
            Vec::new(),
            NonZeroU32::MIN,
        ));
        let mut handlers = ActionHandlerRegistry::new();
        handlers.register(ActionHandler::new(
            start_none,
            tick_continue,
            commit_noop,
            abort_noop,
        ));
        let mut active_actions = BTreeMap::new();
        let mut log = EventLog::new();
        let mut next_instance_id = ActionInstanceId(7);
        let mut rng = test_rng();

        let first = start_action(
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
                tick: Tick(2),
            },
        )
        .unwrap();
        let second = start_action(
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
                tick: Tick(3),
            },
        )
        .unwrap();

        assert_eq!(first, ActionInstanceId(7));
        assert_eq!(second, ActionInstanceId(8));
        assert_eq!(active_actions.len(), 2);
        assert!(active_actions.contains_key(&first));
        assert!(active_actions.contains_key(&second));
        assert_eq!(next_instance_id, ActionInstanceId(9));
        assert_eq!(log.len(), 2);
    }

    #[test]
    fn start_action_errors_when_definition_or_handler_is_missing() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let actor = {
            let mut txn = new_txn(&mut world, 1);
            let actor = txn.create_agent("Aster", ControlSource::Ai).unwrap();
            commit_txn(txn);
            actor
        };
        let affordance = Affordance {
            def_id: ActionDefId(9),
            actor,
            bound_targets: Vec::new(),
            payload_override: None,
            explanation: None,
        };
        let defs = ActionDefRegistry::new();
        let handlers = ActionHandlerRegistry::new();
        let mut active_actions = BTreeMap::new();
        let mut log = EventLog::new();
        let mut next_instance_id = ActionInstanceId(0);
        let mut rng = test_rng();

        let missing_def = start_action(
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
                tick: Tick(1),
            },
        )
        .unwrap_err();

        assert_eq!(missing_def, ActionError::UnknownActionDef(ActionDefId(9)));

        let affordance = Affordance {
            def_id: ActionDefId(0),
            actor,
            bound_targets: Vec::new(),
            payload_override: None,
            explanation: None,
        };
        let mut defs = ActionDefRegistry::new();
        defs.register(sample_def(
            ActionDefId(0),
            ActionHandlerId(3),
            vec![Constraint::ActorAlive],
            vec![Precondition::ActorAlive],
            Vec::new(),
            NonZeroU32::MIN,
        ));

        let missing_handler = start_action(
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
                tick: Tick(1),
            },
        )
        .unwrap_err();

        assert_eq!(
            missing_handler,
            ActionError::UnknownActionHandler(ActionHandlerId(3))
        );
        assert_eq!(log.len(), 0);
        assert!(active_actions.is_empty());
    }

    #[test]
    fn start_action_rejects_duplicate_active_action_ids_in_the_authoritative_store() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let actor = {
            let mut txn = new_txn(&mut world, 1);
            let actor = txn.create_agent("Aster", ControlSource::Ai).unwrap();
            commit_txn(txn);
            actor
        };
        let affordance = Affordance {
            def_id: ActionDefId(0),
            actor,
            bound_targets: Vec::new(),
            payload_override: None,
            explanation: None,
        };
        let mut defs = ActionDefRegistry::new();
        defs.register(sample_def(
            ActionDefId(0),
            ActionHandlerId(0),
            vec![Constraint::ActorAlive],
            vec![Precondition::ActorAlive],
            Vec::new(),
            NonZeroU32::MIN,
        ));
        let mut handlers = ActionHandlerRegistry::new();
        handlers.register(ActionHandler::new(
            start_none,
            tick_continue,
            commit_noop,
            abort_noop,
        ));
        let mut active_actions = BTreeMap::from([(
            ActionInstanceId(4),
            crate::ActionInstance {
                instance_id: ActionInstanceId(4),
                def_id: ActionDefId(99),
                payload: ActionPayload::None,
                actor,
                targets: Vec::new(),
                start_tick: Tick(0),
                remaining_duration: ActionDuration::Finite(1),
                status: crate::ActionStatus::Active,
                reservation_ids: Vec::new(),
                local_state: None,
            },
        )]);
        let mut log = EventLog::new();
        let mut next_instance_id = ActionInstanceId(4);
        let mut rng = test_rng();

        let err = start_action(
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
                tick: Tick(2),
            },
        )
        .unwrap_err();

        assert_eq!(
            err,
            ActionError::InternalError(
                "action instance id ai4 already exists in active actions".to_string()
            )
        );
        assert_eq!(active_actions.len(), 1);
        assert_eq!(log.len(), 0);
        assert_eq!(next_instance_id, ActionInstanceId(4));
    }
}
