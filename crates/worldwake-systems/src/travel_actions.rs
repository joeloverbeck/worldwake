use std::collections::BTreeSet;
use worldwake_core::{
    BodyCostPerTick, EntityId, EntityKind, EventTag, Tick, TravelEdgeId, VisibilitySpec,
    WorldTxn,
};
use worldwake_sim::{
    AbortReason, ActionDef, ActionDefId, ActionDefRegistry, ActionError, ActionHandler,
    ActionHandlerRegistry, ActionInstance, ActionPayload, ActionProgress, ActionState,
    Constraint, DurationExpr, Interruptibility, Precondition, TargetSpec,
};

pub fn register_travel_actions(
    defs: &mut ActionDefRegistry,
    handlers: &mut ActionHandlerRegistry,
) -> ActionDefId {
    let handler = handlers.register(ActionHandler::new(
        start_travel,
        tick_travel,
        commit_travel,
        abort_travel,
    ));
    let id = ActionDefId(defs.len() as u32);
    defs.register(ActionDef {
        id,
        name: "travel".to_string(),
        actor_constraints: vec![
            Constraint::ActorAlive,
            Constraint::ActorHasControl,
            Constraint::ActorNotInTransit,
        ],
        targets: vec![TargetSpec::AdjacentPlace],
        preconditions: vec![
            Precondition::TargetExists(0),
            Precondition::TargetAdjacentToActor(0),
            Precondition::TargetKind {
                target_index: 0,
                kind: EntityKind::Place,
            },
        ],
        reservation_requirements: Vec::new(),
        duration: DurationExpr::TravelToTarget { target_index: 0 },
        body_cost_per_tick: BodyCostPerTick::zero(),
        interruptibility: Interruptibility::InterruptibleWithPenalty,
        commit_conditions: vec![
            Precondition::TargetExists(0),
            Precondition::TargetKind {
                target_index: 0,
                kind: EntityKind::Place,
            },
        ],
        visibility: VisibilitySpec::ParticipantsOnly,
        causal_event_tags: BTreeSet::from([EventTag::Travel]),
        payload: ActionPayload::None,
        handler,
    })
}

fn travel_state(instance: &ActionInstance) -> Result<(TravelEdgeId, EntityId, EntityId, Tick, Tick), ActionError> {
    match instance.local_state {
        Some(ActionState::Travel {
            edge_id,
            origin,
            destination,
            departure_tick,
            arrival_tick,
        }) => Ok((edge_id, origin, destination, departure_tick, arrival_tick)),
        Some(ActionState::Empty) | None => Err(ActionError::InternalError(format!(
            "travel action instance {} is missing travel state",
            instance.instance_id
        ))),
    }
}

fn direct_possessions(txn: &WorldTxn<'_>, actor: EntityId) -> Vec<EntityId> {
    let mut possessions = txn.possessions_of(actor);
    possessions.sort();
    possessions
}

fn resolve_travel(
    txn: &WorldTxn<'_>,
    actor: EntityId,
    destination: EntityId,
) -> Result<(TravelEdgeId, u32, EntityId), ActionError> {
    let origin = txn.effective_place(actor).ok_or_else(|| {
        ActionError::PreconditionFailed(format!("actor {actor} has no origin place"))
    })?;
    let edge = txn
        .topology()
        .unique_direct_edge(origin, destination)
        .map_err(|err| ActionError::PreconditionFailed(err.to_string()))?
        .ok_or_else(|| {
            ActionError::PreconditionFailed(format!(
                "no directed travel edge connects {origin} -> {destination}"
            ))
        })?;
    Ok((edge.id(), edge.travel_time_ticks(), origin))
}

fn start_travel(
    _def: &ActionDef,
    instance: &ActionInstance,
    txn: &mut WorldTxn<'_>,
) -> Result<Option<ActionState>, ActionError> {
    let destination = *instance
        .targets
        .first()
        .ok_or(ActionError::InvalidTarget(instance.actor))?;
    let (edge_id, travel_time_ticks, origin) = resolve_travel(txn, instance.actor, destination)?;
    let departure_tick = instance.start_tick;
    let arrival_tick = Tick(
        departure_tick
            .0
            .checked_add(u64::from(travel_time_ticks))
            .ok_or_else(|| ActionError::InternalError("travel arrival tick overflowed".to_string()))?,
    );

    txn.set_in_transit(instance.actor)
        .map_err(|err| ActionError::InternalError(err.to_string()))?;
    for entity in direct_possessions(txn, instance.actor) {
        txn.set_in_transit(entity)
            .map_err(|err| ActionError::InternalError(err.to_string()))?;
    }
    txn.set_component_in_transit_on_edge(
        instance.actor,
        worldwake_core::InTransitOnEdge {
            edge_id,
            origin,
            destination,
            departure_tick,
            arrival_tick,
        },
    )
    .map_err(|err| ActionError::InternalError(err.to_string()))?;
    txn.add_tag(EventTag::Travel);

    Ok(Some(ActionState::Travel {
        edge_id,
        origin,
        destination,
        departure_tick,
        arrival_tick,
    }))
}

#[allow(clippy::unnecessary_wraps)]
fn tick_travel(
    _def: &ActionDef,
    _instance: &ActionInstance,
    _txn: &mut WorldTxn<'_>,
) -> Result<ActionProgress, ActionError> {
    Ok(ActionProgress::Continue)
}

fn commit_travel(
    _def: &ActionDef,
    instance: &ActionInstance,
    txn: &mut WorldTxn<'_>,
) -> Result<(), ActionError> {
    let (_, _, destination, _, _) = travel_state(instance)?;
    txn.clear_component_in_transit_on_edge(instance.actor)
        .map_err(|err| ActionError::InternalError(err.to_string()))?;
    txn.set_ground_location(instance.actor, destination)
        .map_err(|err| ActionError::InternalError(err.to_string()))?;
    for entity in direct_possessions(txn, instance.actor) {
        txn.set_ground_location(entity, destination)
            .map_err(|err| ActionError::InternalError(err.to_string()))?;
    }
    Ok(())
}

fn abort_travel(
    _def: &ActionDef,
    instance: &ActionInstance,
    _reason: &AbortReason,
    txn: &mut WorldTxn<'_>,
) -> Result<(), ActionError> {
    let (_, origin, _, _, _) = travel_state(instance)?;
    txn.clear_component_in_transit_on_edge(instance.actor)
        .map_err(|err| ActionError::InternalError(err.to_string()))?;
    txn.set_ground_location(instance.actor, origin)
        .map_err(|err| ActionError::InternalError(err.to_string()))?;
    for entity in direct_possessions(txn, instance.actor) {
        txn.set_ground_location(entity, origin)
            .map_err(|err| ActionError::InternalError(err.to_string()))?;
    }
    txn.add_tag(EventTag::Travel);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::register_travel_actions;
    use std::collections::BTreeMap;
    use worldwake_core::{
        CauseRef, Container, ControlSource, EventLog, InTransitOnEdge, LoadUnits, Place, Quantity,
        Topology, TravelEdge, WitnessData, World,
    };
    use worldwake_sim::{
        abort_action, get_affordances, start_action, tick_action, ActionExecutionAuthority,
        ActionExecutionContext, ActionInstance, ActionInstanceId, OmniscientBeliefView, TickOutcome,
    };

    use super::*;

    fn entity(slot: u32) -> EntityId {
        EntityId {
            slot,
            generation: 0,
        }
    }

    fn travel_topology() -> Topology {
        let mut topology = Topology::new();
        for (slot, name) in [(1, "Square"), (2, "Gate"), (3, "Forest")] {
            topology
                .add_place(
                    entity(slot),
                    Place {
                        name: name.to_string(),
                        capacity: None,
                        tags: std::collections::BTreeSet::default(),
                    },
                )
                .unwrap();
        }
        topology
            .add_edge(TravelEdge::new(TravelEdgeId(10), entity(1), entity(2), 3, None).unwrap())
            .unwrap();
        topology
            .add_edge(TravelEdge::new(TravelEdgeId(11), entity(2), entity(1), 3, None).unwrap())
            .unwrap();
        topology
            .add_edge(TravelEdge::new(TravelEdgeId(12), entity(2), entity(3), 2, None).unwrap())
            .unwrap();
        topology
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

    fn setup_world() -> (World, EntityId, EntityId, EntityId, EntityId, EntityId) {
        let mut world = World::new(travel_topology()).unwrap();
        let (actor, bag, bread) = {
            let mut txn = new_txn(&mut world, 1);
            let actor = txn.create_agent("Aster", ControlSource::Ai).unwrap();
            let bag = txn
                .create_container(Container {
                    capacity: LoadUnits(20),
                    allowed_commodities: None,
                    allows_unique_items: true,
                    allows_nested_containers: true,
                })
                .unwrap();
            let bread = txn
                .create_item_lot(worldwake_core::CommodityKind::Bread, Quantity(2))
                .unwrap();
            txn.set_ground_location(actor, entity(1)).unwrap();
            txn.set_ground_location(bag, entity(1)).unwrap();
            txn.put_into_container(bread, bag).unwrap();
            txn.set_possessor(bag, actor).unwrap();
            commit_txn(txn);
            (actor, bag, bread)
        };
        (world, actor, bag, bread, entity(1), entity(2))
    }

    fn setup_registries() -> (ActionDefRegistry, ActionHandlerRegistry, ActionDefId) {
        let mut defs = ActionDefRegistry::new();
        let mut handlers = ActionHandlerRegistry::new();
        let id = register_travel_actions(&mut defs, &mut handlers);
        (defs, handlers, id)
    }

    fn start_travel_action(
        world: &mut World,
        log: &mut EventLog,
        active_actions: &mut BTreeMap<ActionInstanceId, ActionInstance>,
        defs: &ActionDefRegistry,
        handlers: &ActionHandlerRegistry,
        actor: EntityId,
        destination: EntityId,
    ) -> ActionInstanceId {
        let affordance = get_affordances(&OmniscientBeliefView::new(world), actor, defs)
            .into_iter()
            .find(|affordance| affordance.bound_targets == vec![destination])
            .unwrap();
        let mut next_instance_id = ActionInstanceId(1);
        start_action(
            &affordance,
            defs,
            handlers,
            ActionExecutionAuthority {
                active_actions,
                world,
                event_log: log,
            },
            &mut next_instance_id,
            ActionExecutionContext {
                cause: CauseRef::Bootstrap,
                tick: Tick(5),
            },
        )
        .unwrap()
    }

    #[test]
    fn register_travel_actions_creates_single_generic_travel_def() {
        let (defs, _, id) = setup_registries();
        let def = defs.get(id).unwrap();

        assert_eq!(def.name, "travel");
        assert_eq!(def.targets, vec![TargetSpec::AdjacentPlace]);
        assert_eq!(
            def.actor_constraints,
            vec![
                Constraint::ActorAlive,
                Constraint::ActorHasControl,
                Constraint::ActorNotInTransit,
            ]
        );
        assert_eq!(def.duration, DurationExpr::TravelToTarget { target_index: 0 });
    }

    #[test]
    fn travel_affordances_only_offer_adjacent_places() {
        let (world, actor, _, _, _, destination) = setup_world();
        let (defs, _, _) = setup_registries();
        let affordances = get_affordances(&OmniscientBeliefView::new(&world), actor, &defs);

        assert_eq!(affordances.len(), 1);
        assert_eq!(affordances[0].bound_targets, vec![destination]);
    }

    #[test]
    fn travel_happy_path_moves_actor_and_possessions_through_transit() {
        let (mut world, actor, bag, bread, origin, destination) = setup_world();
        let (defs, handlers, _) = setup_registries();
        let mut log = EventLog::new();
        let mut active_actions = BTreeMap::new();
        let instance_id = start_travel_action(
            &mut world,
            &mut log,
            &mut active_actions,
            &defs,
            &handlers,
            actor,
            destination,
        );

        assert_eq!(world.effective_place(actor), None);
        assert_eq!(world.effective_place(bag), None);
        assert_eq!(world.effective_place(bread), None);
        assert!(world.is_in_transit(actor));
        assert!(world.is_in_transit(bag));
        assert!(world.is_in_transit(bread));
        assert_eq!(
            world.get_component_in_transit_on_edge(actor),
            Some(&InTransitOnEdge {
                edge_id: TravelEdgeId(10),
                origin,
                destination,
                departure_tick: Tick(5),
                arrival_tick: Tick(8),
            })
        );
        let start_record = log.get(log.events_by_tag(EventTag::ActionStarted)[0]).unwrap();
        assert!(start_record.tags.contains(&EventTag::Travel));

        for tick in [6, 7] {
            let outcome = tick_action(
                instance_id,
                &defs,
                &handlers,
                ActionExecutionAuthority {
                    active_actions: &mut active_actions,
                    world: &mut world,
                    event_log: &mut log,
                },
                ActionExecutionContext {
                    cause: CauseRef::Bootstrap,
                    tick: Tick(tick),
                },
            )
            .unwrap();
            assert_eq!(outcome, TickOutcome::Continuing);
            assert_eq!(world.effective_place(actor), None);
        }

        let outcome = tick_action(
            instance_id,
            &defs,
            &handlers,
            ActionExecutionAuthority {
                active_actions: &mut active_actions,
                world: &mut world,
                event_log: &mut log,
            },
            ActionExecutionContext {
                cause: CauseRef::Bootstrap,
                tick: Tick(8),
            },
        )
        .unwrap();

        assert_eq!(outcome, TickOutcome::Committed);
        assert_eq!(world.effective_place(actor), Some(destination));
        assert_eq!(world.effective_place(bag), Some(destination));
        assert_eq!(world.effective_place(bread), Some(destination));
        assert!(!world.is_in_transit(actor));
        assert!(!world.is_in_transit(bag));
        assert!(!world.is_in_transit(bread));
        assert_eq!(world.get_component_in_transit_on_edge(actor), None);

        let commit_record = log.get(log.events_by_tag(EventTag::ActionCommitted)[0]).unwrap();
        assert!(commit_record.tags.contains(&EventTag::Travel));
    }

    #[test]
    fn travel_fails_without_directed_edge() {
        let (mut world, actor, _, _, _, _) = setup_world();
        let (defs, handlers, travel_def) = setup_registries();
        let mut log = EventLog::new();
        let mut active_actions = BTreeMap::new();
        let affordance = worldwake_sim::Affordance {
            def_id: travel_def,
            actor,
            bound_targets: vec![entity(3)],
            explanation: None,
        };
        let mut next_instance_id = ActionInstanceId(1);

        let err = start_action(
            &affordance,
            &defs,
            &handlers,
            ActionExecutionAuthority {
                active_actions: &mut active_actions,
                world: &mut world,
                event_log: &mut log,
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
            ActionError::PreconditionFailed("TargetAdjacentToActor(0)".to_string())
        );
    }

    #[test]
    fn travel_fails_if_actor_is_already_in_transit() {
        let (mut world, actor, _, _, _, destination) = setup_world();
        let (defs, handlers, travel_def) = setup_registries();
        {
            let mut txn = new_txn(&mut world, 3);
            txn.set_in_transit(actor).unwrap();
            txn.set_component_in_transit_on_edge(
                actor,
                InTransitOnEdge {
                    edge_id: TravelEdgeId(10),
                    origin: entity(1),
                    destination,
                    departure_tick: Tick(3),
                    arrival_tick: Tick(6),
                },
            )
            .unwrap();
            commit_txn(txn);
        }

        let mut log = EventLog::new();
        let mut active_actions = BTreeMap::new();
        let affordance = worldwake_sim::Affordance {
            def_id: travel_def,
            actor,
            bound_targets: vec![destination],
            explanation: None,
        };
        let mut next_instance_id = ActionInstanceId(1);

        let err = start_action(
            &affordance,
            &defs,
            &handlers,
            ActionExecutionAuthority {
                active_actions: &mut active_actions,
                world: &mut world,
                event_log: &mut log,
            },
            &mut next_instance_id,
            ActionExecutionContext {
                cause: CauseRef::Bootstrap,
                tick: Tick(5),
            },
        )
        .unwrap_err();

        assert_eq!(err, ActionError::ConstraintFailed("ActorNotInTransit".to_string()));
    }

    #[test]
    fn aborted_travel_returns_actor_and_possessions_to_origin() {
        let (mut world, actor, bag, bread, origin, destination) = setup_world();
        let (defs, handlers, _) = setup_registries();
        let mut log = EventLog::new();
        let mut active_actions = BTreeMap::new();
        let instance_id = start_travel_action(
            &mut world,
            &mut log,
            &mut active_actions,
            &defs,
            &handlers,
            actor,
            destination,
        );

        let _ = abort_action(
            instance_id,
            &defs,
            &handlers,
            ActionExecutionAuthority {
                active_actions: &mut active_actions,
                world: &mut world,
                event_log: &mut log,
            },
            ActionExecutionContext {
                cause: CauseRef::Bootstrap,
                tick: Tick(6),
            },
            "cancelled".to_string(),
        )
        .unwrap();

        assert_eq!(world.effective_place(actor), Some(origin));
        assert_eq!(world.effective_place(bag), Some(origin));
        assert_eq!(world.effective_place(bread), Some(origin));
        assert_eq!(world.get_component_in_transit_on_edge(actor), None);

        let record = log.get(log.events_by_tag(EventTag::ActionAborted)[0]).unwrap();
        assert!(record.tags.contains(&EventTag::Travel));
    }
}
