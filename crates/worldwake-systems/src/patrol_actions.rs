use std::collections::BTreeSet;
use worldwake_core::{
    ActionDefId, BodyCostPerTick, EntityId, EntityKind, EventTag, PatrolRoute, VisibilitySpec,
    WorldTxn,
};
use worldwake_sim::{
    AbortReason, ActionDef, ActionDefRegistry, ActionError, ActionHandler, ActionHandlerId,
    ActionHandlerRegistry, ActionInstance, ActionPayload, ActionProgress, CommitOutcome,
    Constraint, DeterministicRng, DurationExpr, Interruptibility, Precondition, RuntimeBeliefView,
    TargetSpec,
};

pub fn register_patrol_action(
    defs: &mut ActionDefRegistry,
    handlers: &mut ActionHandlerRegistry,
) -> ActionDefId {
    let handler = handlers.register(
        ActionHandler::new(start_patrol, tick_patrol, commit_patrol, abort_patrol)
            .with_affordance_targets(enumerate_patrol_targets),
    );
    let id = ActionDefId(defs.len() as u32);
    defs.register(patrol_action_def(id, handler))
}

fn patrol_action_def(id: ActionDefId, handler: ActionHandlerId) -> ActionDef {
    ActionDef {
        id,
        name: "patrol".to_string(),
        domain: worldwake_core::ActionDomain::Generic,
        actor_constraints: vec![
            Constraint::ActorAlive,
            Constraint::ActorHasControl,
            Constraint::ActorNotIncapacitated,
            Constraint::ActorNotInTransit,
        ],
        targets: vec![TargetSpec::ActorPlace],
        preconditions: vec![
            Precondition::TargetExists(0),
            Precondition::TargetKind {
                target_index: 0,
                kind: EntityKind::Place,
            },
        ],
        reservation_requirements: Vec::new(),
        duration: DurationExpr::ActorPatrolProfile,
        body_cost_per_tick: BodyCostPerTick::zero(),
        attention_cost: worldwake_core::Permille::new_unchecked(100),
        interruptibility: Interruptibility::FreelyInterruptible,
        commit_conditions: vec![
            Precondition::TargetExists(0),
            Precondition::TargetKind {
                target_index: 0,
                kind: EntityKind::Place,
            },
        ],
        visibility: VisibilitySpec::SamePlace,
        causal_event_tags: BTreeSet::from([EventTag::Patrol]),
        payload: ActionPayload::None,
        handler,
    }
}

fn current_waypoint(route: &PatrolRoute) -> Result<EntityId, ActionError> {
    route
        .assigned_places
        .get(route.current_index)
        .copied()
        .ok_or_else(|| {
            ActionError::PreconditionFailed(format!(
                "patrol route current_index {} is out of bounds for {} waypoint(s)",
                route.current_index,
                route.assigned_places.len()
            ))
        })
}

fn patrol_route_and_waypoint(
    txn: &WorldTxn<'_>,
    actor: EntityId,
) -> Result<(PatrolRoute, EntityId), ActionError> {
    let route = txn
        .get_component_patrol_route(actor)
        .cloned()
        .ok_or_else(|| {
            ActionError::PreconditionFailed(format!("actor {actor} lacks PatrolRoute"))
        })?;
    if route.assigned_places.is_empty() {
        return Err(ActionError::PreconditionFailed(format!(
            "actor {actor} has an empty PatrolRoute"
        )));
    }
    let waypoint = current_waypoint(&route)?;
    Ok((route, waypoint))
}

fn patrol_target(instance: &ActionInstance) -> Result<EntityId, ActionError> {
    instance
        .targets
        .first()
        .copied()
        .ok_or(ActionError::InvalidTarget(instance.actor))
}

fn enumerate_patrol_targets(
    _def: &ActionDef,
    actor: EntityId,
    view: &dyn RuntimeBeliefView,
) -> Vec<Vec<EntityId>> {
    let Some(route) = view.patrol_route(actor) else {
        return Vec::new();
    };
    if route.assigned_places.is_empty() {
        return Vec::new();
    }
    if view.patrol_profile(actor).is_none() {
        return Vec::new();
    }
    let Some(waypoint) = route.assigned_places.get(route.current_index).copied() else {
        return Vec::new();
    };
    (view.effective_place(actor) == Some(waypoint))
        .then_some(vec![vec![waypoint]])
        .unwrap_or_default()
}

fn start_patrol(
    _def: &ActionDef,
    instance: &mut ActionInstance,
    _context: &worldwake_sim::ActionExecutionContext<'_>,
    _rng: &mut DeterministicRng,
    txn: &mut WorldTxn<'_>,
) -> Result<Option<worldwake_sim::ActionState>, ActionError> {
    let target = patrol_target(instance)?;
    let (_, waypoint) = patrol_route_and_waypoint(txn, instance.actor)?;
    txn.get_component_patrol_profile(instance.actor)
        .ok_or_else(|| {
            ActionError::PreconditionFailed(format!("actor {} lacks PatrolProfile", instance.actor))
        })?;
    if txn.effective_place(instance.actor) != Some(target) {
        return Err(ActionError::PreconditionFailed(format!(
            "actor {} is not at patrol target {target}",
            instance.actor
        )));
    }
    if waypoint != target {
        return Err(ActionError::PreconditionFailed(format!(
            "patrol target {target} does not match current waypoint {waypoint} for actor {}",
            instance.actor
        )));
    }
    Ok(None)
}

#[allow(clippy::unnecessary_wraps)]
fn tick_patrol(
    _def: &ActionDef,
    _instance: &mut ActionInstance,
    _context: &worldwake_sim::ActionExecutionContext<'_>,
    _rng: &mut DeterministicRng,
    _txn: &mut WorldTxn<'_>,
) -> Result<ActionProgress, ActionError> {
    Ok(ActionProgress::Continue)
}

fn commit_patrol(
    _def: &ActionDef,
    instance: &ActionInstance,
    _context: &worldwake_sim::ActionExecutionContext<'_>,
    _event_log: &worldwake_core::EventLog,
    _rng: &mut DeterministicRng,
    txn: &mut WorldTxn<'_>,
) -> Result<CommitOutcome, ActionError> {
    txn.get_component_patrol_profile(instance.actor)
        .ok_or_else(|| {
            ActionError::PreconditionFailed(format!("actor {} lacks PatrolProfile", instance.actor))
        })?;
    let (mut route, waypoint) = patrol_route_and_waypoint(txn, instance.actor)?;
    let target = patrol_target(instance)?;
    if txn.effective_place(instance.actor) != Some(target) {
        return Err(ActionError::PreconditionFailed(format!(
            "actor {} is not at patrol target {target}",
            instance.actor
        )));
    }
    if waypoint != target {
        return Err(ActionError::PreconditionFailed(format!(
            "patrol target {target} no longer matches current waypoint {waypoint} for actor {}",
            instance.actor
        )));
    }
    route.current_index = (route.current_index + 1) % route.assigned_places.len();
    txn.set_component_patrol_route(instance.actor, route)
        .map_err(|err| ActionError::InternalError(err.to_string()))?;
    Ok(CommitOutcome::empty())
}

#[allow(clippy::unnecessary_wraps)]
fn abort_patrol(
    _def: &ActionDef,
    _instance: &ActionInstance,
    _context: &worldwake_sim::ActionExecutionContext<'_>,
    _reason: &AbortReason,
    _event_log: &worldwake_core::EventLog,
    _rng: &mut DeterministicRng,
    _txn: &mut WorldTxn<'_>,
) -> Result<(), ActionError> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::register_patrol_action;
    use std::collections::BTreeMap;
    use worldwake_core::{
        CauseRef, ControlSource, EventLog, EventTag, EventView, PatrolProfile, PatrolRoute,
        Permille, PrototypePlace, Seed, Tick, VisibilitySpec, World, WorldTxn,
        build_prototype_world, prototype_place_entity,
    };
    use worldwake_sim::{
        ActionDefRegistry, ActionError, ActionExecutionAuthority, ActionHandlerRegistry,
        ActionInstance, ActionInstanceId, ActionPayload, DeterministicRng, InterruptReason,
        TickOutcome, get_affordances, interrupt_action, start_action, tick_action,
    };

    use super::*;

    fn new_txn(world: &mut World, tick: u64) -> WorldTxn<'_> {
        WorldTxn::new(
            world,
            Tick(tick),
            CauseRef::Bootstrap,
            None,
            None,
            VisibilitySpec::SamePlace,
            worldwake_core::WitnessData::default(),
        )
    }

    fn commit_txn(txn: WorldTxn<'_>) {
        let mut log = EventLog::new();
        let _ = txn.commit(&mut log);
    }

    fn test_rng() -> DeterministicRng {
        DeterministicRng::new(Seed([0x23; 32]))
    }

    fn patrol_profile(base_dwell_ticks: u32, vigilance: u16) -> PatrolProfile {
        PatrolProfile {
            base_dwell_ticks,
            dwell_vigilance_scale_ticks: base_dwell_ticks,
            vigilance: Permille::new(vigilance).unwrap(),
            route_adaptation_sensitivity: Permille::new(400).unwrap(),
            patrol_motive_weight: Permille::new(550).unwrap(),
        }
    }

    fn setup_registries() -> (ActionDefRegistry, ActionHandlerRegistry, ActionDefId) {
        let mut defs = ActionDefRegistry::new();
        let mut handlers = ActionHandlerRegistry::new();
        let patrol_id = register_patrol_action(&mut defs, &mut handlers);
        (defs, handlers, patrol_id)
    }

    fn setup_world(
        route: PatrolRoute,
        profile: Option<PatrolProfile>,
        actor_place: EntityId,
    ) -> (World, EntityId) {
        let mut world = World::new(build_prototype_world()).unwrap();
        let actor = {
            let mut txn = new_txn(&mut world, 1);
            let actor = txn.create_agent("Guard", ControlSource::Ai).unwrap();
            txn.set_ground_location(actor, actor_place).unwrap();
            txn.set_component_patrol_route(actor, route).unwrap();
            if let Some(profile) = profile {
                txn.set_component_patrol_profile(actor, profile).unwrap();
            }
            commit_txn(txn);
            actor
        };
        (world, actor)
    }

    fn patrol_affordance(
        def_id: ActionDefId,
        actor: EntityId,
        place: EntityId,
    ) -> worldwake_sim::Affordance {
        worldwake_sim::Affordance {
            def_id,
            actor,
            bound_targets: vec![place],
            payload_override: None,
            explanation: None,
            contention_status: worldwake_core::ContentionStatus::Unmanaged,
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn start_patrol_instance(
        world: &mut World,
        log: &mut EventLog,
        rng: &mut DeterministicRng,
        active_actions: &mut BTreeMap<ActionInstanceId, ActionInstance>,
        next_instance_id: &mut ActionInstanceId,
        defs: &ActionDefRegistry,
        handlers: &ActionHandlerRegistry,
        patrol_id: ActionDefId,
        actor: EntityId,
        place: EntityId,
    ) -> Result<ActionInstanceId, ActionError> {
        start_action(
            &patrol_affordance(patrol_id, actor, place),
            defs,
            handlers,
            ActionExecutionAuthority {
                active_actions,
                world,
                event_log: log,
                rng,
            },
            next_instance_id,
            worldwake_sim::ActionExecutionContext::without_recipes(CauseRef::Bootstrap, Tick(2)),
        )
    }

    #[test]
    fn register_patrol_action_creates_expected_definition() {
        let (defs, handlers, patrol_id) = setup_registries();
        let def = defs.get(patrol_id).unwrap();

        assert_eq!(handlers.len(), 1);
        assert_eq!(def.name, "patrol");
        assert_eq!(def.domain, worldwake_core::ActionDomain::Generic);
        assert_eq!(def.targets, vec![TargetSpec::ActorPlace]);
        assert_eq!(def.duration, DurationExpr::ActorPatrolProfile);
        assert_eq!(def.interruptibility, Interruptibility::FreelyInterruptible);
        assert_eq!(def.visibility, VisibilitySpec::SamePlace);
        assert_eq!(def.causal_event_tags, BTreeSet::from([EventTag::Patrol]));
        assert_eq!(def.payload, ActionPayload::None);
    }

    #[test]
    fn patrol_duration_scales_with_vigilance() {
        let (defs, _handlers, patrol_id) = setup_registries();
        let square = prototype_place_entity(PrototypePlace::VillageSquare);
        let route = PatrolRoute {
            assigned_places: vec![square],
            current_index: 0,
        };
        let (low_world, low_actor) = setup_world(route.clone(), Some(patrol_profile(8, 0)), square);
        let (high_world, high_actor) = setup_world(route, Some(patrol_profile(8, 1000)), square);
        let def = defs.get(patrol_id).unwrap();

        let low = def
            .duration
            .resolve_for(&low_world, low_actor, &[square], &ActionPayload::None)
            .unwrap();
        let high = def
            .duration
            .resolve_for(&high_world, high_actor, &[square], &ActionPayload::None)
            .unwrap();

        assert_eq!(low, worldwake_sim::ActionDuration::new(8));
        assert_eq!(high, worldwake_sim::ActionDuration::new(16));
        assert!(high.ticks() > low.ticks());
    }

    #[test]
    fn patrol_affordance_is_omitted_when_actor_is_off_current_waypoint() {
        let square = prototype_place_entity(PrototypePlace::VillageSquare);
        let gate = prototype_place_entity(PrototypePlace::SouthGate);
        let route = PatrolRoute {
            assigned_places: vec![square, gate],
            current_index: 1,
        };
        let (world, actor) = setup_world(route, Some(patrol_profile(1, 0)), square);
        let (defs, handlers, patrol_id) = setup_registries();
        let view = worldwake_sim::PerAgentBeliefView::from_world(actor, &world);

        let patrol_affordances = get_affordances(&view, actor, &defs, &handlers)
            .into_iter()
            .filter(|affordance| affordance.def_id == patrol_id)
            .collect::<Vec<_>>();

        assert!(
            patrol_affordances.is_empty(),
            "patrol should not expose an affordance until the actor reaches the current waypoint"
        );
    }

    #[test]
    fn patrol_commit_advances_current_index_and_records_patrol_tag() {
        let square = prototype_place_entity(PrototypePlace::VillageSquare);
        let hall = prototype_place_entity(PrototypePlace::RulersHall);
        let gate = prototype_place_entity(PrototypePlace::SouthGate);
        let route = PatrolRoute {
            assigned_places: vec![square, hall, gate],
            current_index: 0,
        };
        let (mut world, actor) = setup_world(route, Some(patrol_profile(1, 0)), square);
        let (defs, handlers, patrol_id) = setup_registries();
        let mut log = EventLog::new();
        let mut rng = test_rng();
        let mut active_actions = BTreeMap::new();
        let mut next_instance_id = ActionInstanceId(0);
        let instance_id = start_patrol_instance(
            &mut world,
            &mut log,
            &mut rng,
            &mut active_actions,
            &mut next_instance_id,
            &defs,
            &handlers,
            patrol_id,
            actor,
            square,
        )
        .unwrap();

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
            worldwake_sim::ActionExecutionContext::without_recipes(CauseRef::Bootstrap, Tick(3)),
        )
        .unwrap();

        assert!(matches!(outcome, TickOutcome::Committed { .. }));
        assert_eq!(
            world
                .get_component_patrol_route(actor)
                .unwrap()
                .current_index,
            1
        );
        let patrol_events = log.events_by_tag(EventTag::Patrol);
        assert_eq!(patrol_events.len(), 1);
        let record = log.get(patrol_events[0]).unwrap();
        assert!(record.tags().contains(&EventTag::Patrol));
        assert!(record.tags().contains(&EventTag::ActionCommitted));
    }

    #[test]
    fn patrol_commit_wraps_current_index_at_route_end() {
        let square = prototype_place_entity(PrototypePlace::VillageSquare);
        let hall = prototype_place_entity(PrototypePlace::RulersHall);
        let gate = prototype_place_entity(PrototypePlace::SouthGate);
        let route = PatrolRoute {
            assigned_places: vec![square, hall, gate],
            current_index: 2,
        };
        let (mut world, actor) = setup_world(route, Some(patrol_profile(1, 0)), gate);
        let (defs, handlers, patrol_id) = setup_registries();
        let mut log = EventLog::new();
        let mut rng = test_rng();
        let mut active_actions = BTreeMap::new();
        let mut next_instance_id = ActionInstanceId(0);
        let instance_id = start_patrol_instance(
            &mut world,
            &mut log,
            &mut rng,
            &mut active_actions,
            &mut next_instance_id,
            &defs,
            &handlers,
            patrol_id,
            actor,
            gate,
        )
        .unwrap();

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
            worldwake_sim::ActionExecutionContext::without_recipes(CauseRef::Bootstrap, Tick(3)),
        )
        .unwrap();

        assert!(matches!(outcome, TickOutcome::Committed { .. }));
        assert_eq!(
            world
                .get_component_patrol_route(actor)
                .unwrap()
                .current_index,
            0
        );
    }

    #[test]
    fn aborting_patrol_preserves_current_index() {
        let square = prototype_place_entity(PrototypePlace::VillageSquare);
        let hall = prototype_place_entity(PrototypePlace::RulersHall);
        let route = PatrolRoute {
            assigned_places: vec![square, hall],
            current_index: 0,
        };
        let (mut world, actor) = setup_world(route, Some(patrol_profile(2, 0)), square);
        let (defs, handlers, patrol_id) = setup_registries();
        let mut log = EventLog::new();
        let mut rng = test_rng();
        let mut active_actions = BTreeMap::new();
        let mut next_instance_id = ActionInstanceId(0);
        let instance_id = start_patrol_instance(
            &mut world,
            &mut log,
            &mut rng,
            &mut active_actions,
            &mut next_instance_id,
            &defs,
            &handlers,
            patrol_id,
            actor,
            square,
        )
        .unwrap();

        interrupt_action(
            instance_id,
            &defs,
            &handlers,
            ActionExecutionAuthority {
                active_actions: &mut active_actions,
                world: &mut world,
                event_log: &mut log,
                rng: &mut rng,
            },
            worldwake_sim::ActionExecutionContext::without_recipes(CauseRef::Bootstrap, Tick(3)),
            InterruptReason::Reprioritized,
        )
        .unwrap();

        assert_eq!(
            world
                .get_component_patrol_route(actor)
                .unwrap()
                .current_index,
            0
        );
        assert!(active_actions.is_empty());
    }

    #[test]
    fn patrol_start_rejects_actor_without_patrol_route() {
        let square = prototype_place_entity(PrototypePlace::VillageSquare);
        let mut world = World::new(build_prototype_world()).unwrap();
        let actor = {
            let mut txn = new_txn(&mut world, 1);
            let actor = txn.create_agent("Guard", ControlSource::Ai).unwrap();
            txn.set_ground_location(actor, square).unwrap();
            txn.set_component_patrol_profile(actor, patrol_profile(1, 0))
                .unwrap();
            commit_txn(txn);
            actor
        };
        let (defs, handlers, patrol_id) = setup_registries();
        let mut log = EventLog::new();
        let mut rng = test_rng();
        let mut active_actions = BTreeMap::new();
        let mut next_instance_id = ActionInstanceId(0);

        let err = start_patrol_instance(
            &mut world,
            &mut log,
            &mut rng,
            &mut active_actions,
            &mut next_instance_id,
            &defs,
            &handlers,
            patrol_id,
            actor,
            square,
        )
        .unwrap_err();

        assert_eq!(
            err,
            ActionError::PreconditionFailed(format!("actor {actor} lacks PatrolRoute"))
        );
    }

    #[test]
    fn patrol_start_rejects_actor_without_patrol_profile() {
        let square = prototype_place_entity(PrototypePlace::VillageSquare);
        let route = PatrolRoute {
            assigned_places: vec![square],
            current_index: 0,
        };
        let (mut world, actor) = setup_world(route, None, square);
        let (defs, handlers, patrol_id) = setup_registries();
        let mut log = EventLog::new();
        let mut rng = test_rng();
        let mut active_actions = BTreeMap::new();
        let mut next_instance_id = ActionInstanceId(0);

        let err = start_patrol_instance(
            &mut world,
            &mut log,
            &mut rng,
            &mut active_actions,
            &mut next_instance_id,
            &defs,
            &handlers,
            patrol_id,
            actor,
            square,
        )
        .unwrap_err();

        assert_eq!(
            err,
            ActionError::PreconditionFailed(format!("actor {actor} lacks patrol profile"))
        );
    }

    #[test]
    fn patrol_start_rejects_mismatched_waypoint_target() {
        let square = prototype_place_entity(PrototypePlace::VillageSquare);
        let hall = prototype_place_entity(PrototypePlace::RulersHall);
        let route = PatrolRoute {
            assigned_places: vec![square, hall],
            current_index: 0,
        };
        let (mut world, actor) = setup_world(route, Some(patrol_profile(1, 0)), hall);
        let (defs, handlers, patrol_id) = setup_registries();
        let mut log = EventLog::new();
        let mut rng = test_rng();
        let mut active_actions = BTreeMap::new();
        let mut next_instance_id = ActionInstanceId(0);

        let err = start_patrol_instance(
            &mut world,
            &mut log,
            &mut rng,
            &mut active_actions,
            &mut next_instance_id,
            &defs,
            &handlers,
            patrol_id,
            actor,
            hall,
        )
        .unwrap_err();

        assert_eq!(
            err,
            ActionError::PreconditionFailed(format!(
                "patrol target {hall} does not match current waypoint {square} for actor {actor}"
            ))
        );
    }
}
