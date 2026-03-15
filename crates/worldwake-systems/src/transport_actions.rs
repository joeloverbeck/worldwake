use std::collections::BTreeSet;
use std::num::NonZeroU32;
use worldwake_core::{
    load_of_entity, load_per_unit, ActionDefId, BodyCostPerTick, EntityId, EntityKind, EventTag,
    Quantity, VisibilitySpec, WorldTxn,
};
use worldwake_sim::{
    AbortReason, ActionDef, ActionDefRegistry, ActionError, ActionHandler, ActionHandlerRegistry,
    ActionInstance, ActionPayload, ActionProgress, CommitOutcome, Constraint, DeterministicRng,
    DurationExpr, Interruptibility, Materialization, MaterializationTag, Precondition, TargetSpec,
    TransportActionPayload,
};

use crate::inventory::{move_entity_to_direct_possession, remaining_capacity};

pub fn register_transport_actions(
    defs: &mut ActionDefRegistry,
    handlers: &mut ActionHandlerRegistry,
) -> Vec<ActionDefId> {
    let pick_up_handler = handlers.register(
        ActionHandler::new(
            start_pick_up,
            tick_transport,
            commit_pick_up,
            abort_transport,
        )
        .with_payload_override_validator(validate_pick_up_payload_override),
    );
    let put_down_handler = handlers.register(ActionHandler::new(
        start_put_down,
        tick_transport,
        commit_put_down,
        abort_transport,
    ));

    let pick_up_id = ActionDefId(defs.len() as u32);
    let put_down_id = ActionDefId(pick_up_id.0 + 1);

    vec![
        defs.register(ActionDef {
            id: pick_up_id,
            name: "pick_up".to_string(),
            domain: worldwake_sim::ActionDomain::Transport,
            actor_constraints: vec![Constraint::ActorAlive, Constraint::ActorHasControl],
            targets: vec![TargetSpec::EntityAtActorPlace {
                kind: EntityKind::ItemLot,
            }],
            preconditions: vec![
                Precondition::TargetExists(0),
                Precondition::TargetAtActorPlace(0),
                Precondition::TargetKind {
                    target_index: 0,
                    kind: EntityKind::ItemLot,
                },
                Precondition::TargetNotInContainer(0),
                Precondition::TargetUnpossessed(0),
            ],
            reservation_requirements: Vec::new(),
            duration: DurationExpr::Fixed(NonZeroU32::MIN),
            body_cost_per_tick: BodyCostPerTick::zero(),
            interruptibility: Interruptibility::InterruptibleWithPenalty,
            commit_conditions: vec![
                Precondition::TargetExists(0),
                Precondition::TargetAtActorPlace(0),
                Precondition::TargetKind {
                    target_index: 0,
                    kind: EntityKind::ItemLot,
                },
                Precondition::TargetNotInContainer(0),
                Precondition::TargetUnpossessed(0),
            ],
            visibility: VisibilitySpec::ParticipantsOnly,
            causal_event_tags: BTreeSet::from([
                EventTag::WorldMutation,
                EventTag::Inventory,
                EventTag::Transfer,
            ]),
            payload: ActionPayload::None,
            handler: pick_up_handler,
        }),
        defs.register(ActionDef {
            id: put_down_id,
            name: "put_down".to_string(),
            domain: worldwake_sim::ActionDomain::Transport,
            actor_constraints: vec![Constraint::ActorAlive, Constraint::ActorHasControl],
            targets: vec![TargetSpec::EntityDirectlyPossessedByActor {
                kind: EntityKind::ItemLot,
            }],
            preconditions: vec![
                Precondition::TargetExists(0),
                Precondition::TargetAtActorPlace(0),
                Precondition::TargetKind {
                    target_index: 0,
                    kind: EntityKind::ItemLot,
                },
                Precondition::TargetDirectlyPossessedByActor(0),
            ],
            reservation_requirements: Vec::new(),
            duration: DurationExpr::Fixed(NonZeroU32::MIN),
            body_cost_per_tick: BodyCostPerTick::zero(),
            interruptibility: Interruptibility::InterruptibleWithPenalty,
            commit_conditions: vec![
                Precondition::TargetExists(0),
                Precondition::TargetAtActorPlace(0),
                Precondition::TargetKind {
                    target_index: 0,
                    kind: EntityKind::ItemLot,
                },
                Precondition::TargetDirectlyPossessedByActor(0),
            ],
            visibility: VisibilitySpec::ParticipantsOnly,
            causal_event_tags: BTreeSet::from([
                EventTag::WorldMutation,
                EventTag::Inventory,
                EventTag::Transfer,
            ]),
            payload: ActionPayload::None,
            handler: put_down_handler,
        }),
    ]
}

fn require_item_lot_target(instance: &ActionInstance) -> Result<EntityId, ActionError> {
    instance
        .targets
        .first()
        .copied()
        .ok_or(ActionError::InvalidTarget(instance.actor))
}

fn validate_pick_up(
    txn: &WorldTxn<'_>,
    actor: EntityId,
    target: EntityId,
    requested_quantity: Option<Quantity>,
) -> Result<(), ActionError> {
    let actor_place = txn
        .effective_place(actor)
        .ok_or_else(|| ActionError::PreconditionFailed(format!("actor {actor} has no place")))?;
    if txn.effective_place(target) != Some(actor_place) {
        return Err(ActionError::PreconditionFailed(format!(
            "target {target} is not at actor {actor} place {actor_place}"
        )));
    }
    if txn.entity_kind(target) != Some(EntityKind::ItemLot) {
        return Err(ActionError::InvalidTarget(target));
    }
    if txn.direct_container(target).is_some() {
        return Err(ActionError::PreconditionFailed(format!(
            "target {target} is inside a container"
        )));
    }
    if txn.possessor_of(target).is_some() {
        return Err(ActionError::PreconditionFailed(format!(
            "target {target} is already possessed"
        )));
    }
    let lot = txn
        .get_component_item_lot(target)
        .ok_or(ActionError::InvalidTarget(target))?;
    let per_unit = load_per_unit(lot.commodity).0;
    let remaining = remaining_capacity(txn, actor)?.0;
    if remaining < per_unit {
        return Err(ActionError::PreconditionFailed(format!(
            "actor {actor} has insufficient carry capacity for any {:?}",
            lot.commodity
        )));
    }
    if let Some(quantity) = requested_quantity {
        let max_quantity = Quantity((remaining / per_unit).min(lot.quantity.0));
        if quantity == Quantity(0) || quantity > max_quantity {
            return Err(ActionError::PreconditionFailed(format!(
                "requested pickup quantity {quantity:?} exceeds available movable quantity {max_quantity:?}",
            )));
        }
    }
    Ok(())
}

fn execute_pick_up(
    txn: &mut WorldTxn<'_>,
    actor: EntityId,
    target: EntityId,
    requested_quantity: Option<Quantity>,
) -> Result<EntityId, ActionError> {
    validate_pick_up(txn, actor, target, requested_quantity)?;
    let actor_place = txn
        .effective_place(actor)
        .ok_or_else(|| ActionError::PreconditionFailed(format!("actor {actor} has no place")))?;
    let lot = txn
        .get_component_item_lot(target)
        .cloned()
        .ok_or(ActionError::InvalidTarget(target))?;
    let remaining = remaining_capacity(txn, actor)?.0;
    let per_unit = load_per_unit(lot.commodity).0;
    let requested_quantity =
        requested_quantity.unwrap_or(Quantity((remaining / per_unit).min(lot.quantity.0)));
    let moved_entity = if load_of_entity(txn, target)
        .map_err(|err| ActionError::InternalError(err.to_string()))?
        .0
        <= remaining
        && requested_quantity == lot.quantity
    {
        target
    } else {
        let (_, split_off) = txn
            .split_lot(target, requested_quantity)
            .map_err(|err| ActionError::InternalError(err.to_string()))?;
        split_off
    };

    move_entity_to_direct_possession(txn, moved_entity, actor, actor_place)?;
    Ok(moved_entity)
}

fn validate_put_down(
    txn: &WorldTxn<'_>,
    actor: EntityId,
    target: EntityId,
) -> Result<EntityId, ActionError> {
    let actor_place = txn
        .effective_place(actor)
        .ok_or_else(|| ActionError::PreconditionFailed(format!("actor {actor} has no place")))?;
    if txn.effective_place(target) != Some(actor_place) {
        return Err(ActionError::PreconditionFailed(format!(
            "target {target} is not at actor {actor} place {actor_place}"
        )));
    }
    if txn.entity_kind(target) != Some(EntityKind::ItemLot) {
        return Err(ActionError::InvalidTarget(target));
    }
    if txn.possessor_of(target) != Some(actor) {
        return Err(ActionError::PreconditionFailed(format!(
            "actor {actor} does not directly possess target {target}"
        )));
    }
    Ok(actor_place)
}

fn start_pick_up(
    _def: &ActionDef,
    instance: &ActionInstance,
    _rng: &mut DeterministicRng,
    txn: &mut WorldTxn<'_>,
) -> Result<Option<worldwake_sim::ActionState>, ActionError> {
    validate_pick_up(
        txn,
        instance.actor,
        require_item_lot_target(instance)?,
        requested_pick_up_quantity(&instance.payload)?,
    )?;
    Ok(None)
}

fn commit_pick_up(
    _def: &ActionDef,
    instance: &ActionInstance,
    _rng: &mut DeterministicRng,
    txn: &mut WorldTxn<'_>,
) -> Result<CommitOutcome, ActionError> {
    let target = require_item_lot_target(instance)?;
    let moved_entity = execute_pick_up(
        txn,
        instance.actor,
        target,
        requested_pick_up_quantity(&instance.payload)?,
    )?;
    if moved_entity == target {
        Ok(CommitOutcome::empty())
    } else {
        Ok(CommitOutcome {
            materializations: vec![Materialization {
                tag: MaterializationTag::SplitOffLot,
                entity: moved_entity,
            }],
        })
    }
}

fn start_put_down(
    _def: &ActionDef,
    instance: &ActionInstance,
    _rng: &mut DeterministicRng,
    txn: &mut WorldTxn<'_>,
) -> Result<Option<worldwake_sim::ActionState>, ActionError> {
    validate_put_down(txn, instance.actor, require_item_lot_target(instance)?)?;
    Ok(None)
}

fn commit_put_down(
    _def: &ActionDef,
    instance: &ActionInstance,
    _rng: &mut DeterministicRng,
    txn: &mut WorldTxn<'_>,
) -> Result<CommitOutcome, ActionError> {
    let target = require_item_lot_target(instance)?;
    let actor_place = validate_put_down(txn, instance.actor, target)?;
    txn.clear_possessor(target)
        .map_err(|err| ActionError::InternalError(err.to_string()))?;
    txn.set_ground_location(target, actor_place)
        .map_err(|err| ActionError::InternalError(err.to_string()))?;
    txn.add_target(target);
    Ok(CommitOutcome::empty())
}

#[allow(clippy::unnecessary_wraps)]
fn tick_transport(
    _def: &ActionDef,
    _instance: &ActionInstance,
    _rng: &mut DeterministicRng,
    _txn: &mut WorldTxn<'_>,
) -> Result<ActionProgress, ActionError> {
    Ok(ActionProgress::Continue)
}

#[allow(clippy::unnecessary_wraps)]
fn abort_transport(
    _def: &ActionDef,
    _instance: &ActionInstance,
    _reason: &AbortReason,
    _rng: &mut DeterministicRng,
    _txn: &mut WorldTxn<'_>,
) -> Result<(), ActionError> {
    Ok(())
}

fn requested_pick_up_quantity(payload: &ActionPayload) -> Result<Option<Quantity>, ActionError> {
    match payload {
        ActionPayload::None => Ok(None),
        ActionPayload::Transport(TransportActionPayload { quantity }) => Ok(Some(*quantity)),
        _ => Err(ActionError::PreconditionFailed(
            "pick_up received non-transport payload".to_string(),
        )),
    }
}

fn validate_pick_up_payload_override(
    def: &ActionDef,
    actor: EntityId,
    targets: &[EntityId],
    payload: &ActionPayload,
    view: &dyn worldwake_sim::RuntimeBeliefView,
) -> bool {
    if def.name != "pick_up" {
        return false;
    }
    let Some(TransportActionPayload { quantity }) = payload.as_transport() else {
        return false;
    };
    if *quantity == Quantity(0) {
        return false;
    }
    let Some(target) = targets.first().copied() else {
        return false;
    };
    let Some(commodity) = view.item_lot_commodity(target) else {
        return false;
    };
    let lot_quantity = view.commodity_quantity(target, commodity);
    let Some(carry_capacity) = view.carry_capacity(actor) else {
        return false;
    };
    let Some(load) = view.load_of_entity(actor) else {
        return false;
    };
    let per_unit = load_per_unit(commodity).0;
    let max_quantity =
        Quantity((carry_capacity.0.saturating_sub(load.0) / per_unit).min(lot_quantity.0));
    *quantity <= max_quantity
}

#[cfg(test)]
mod tests {
    use super::register_transport_actions;
    use std::collections::BTreeMap;
    use worldwake_core::{
        build_believed_entity_state, build_prototype_world, AgentBeliefStore, CarryCapacity,
        CauseRef, CommodityKind, Container, ControlSource, EventLog, EventView, LoadUnits,
        PerceptionSource, Place, Quantity, Seed, Tick, Topology, TravelEdge, TravelEdgeId,
        VisibilitySpec, WitnessData, World, WorldTxn,
    };
    use worldwake_sim::{
        get_affordances, start_action, tick_action, ActionDefRegistry, ActionExecutionAuthority,
        ActionExecutionContext, ActionHandlerRegistry, ActionInstance, ActionInstanceId,
        DeterministicRng, PerAgentBeliefView, TickOutcome,
    };

    use super::*;

    fn entity(slot: u32) -> EntityId {
        EntityId {
            slot,
            generation: 0,
        }
    }

    fn transport_topology() -> Topology {
        let mut topology = Topology::new();
        for (slot, name) in [(1, "Square"), (2, "Storehouse"), (3, "Field")] {
            topology
                .add_place(
                    entity(slot),
                    Place {
                        name: name.to_string(),
                        capacity: None,
                        tags: BTreeSet::new(),
                    },
                )
                .unwrap();
        }
        topology
            .add_edge(TravelEdge::new(TravelEdgeId(10), entity(1), entity(2), 2, None).unwrap())
            .unwrap();
        topology
            .add_edge(TravelEdge::new(TravelEdgeId(11), entity(2), entity(1), 2, None).unwrap())
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

    fn test_rng() -> DeterministicRng {
        DeterministicRng::new(Seed([0x73; 32]))
    }

    fn test_belief_store(world: &World, actor: EntityId) -> AgentBeliefStore {
        let mut store = world
            .get_component_agent_belief_store(actor)
            .cloned()
            .unwrap_or_default();
        for entity in world.entities() {
            if entity == actor {
                continue;
            }
            if let Some(state) = build_believed_entity_state(
                world,
                entity,
                Tick(u64::MAX),
                PerceptionSource::DirectObservation,
            ) {
                store.update_entity(entity, state);
            }
        }
        store
    }

    fn affordances_for(
        world: &World,
        actor: EntityId,
        defs: &ActionDefRegistry,
        handlers: &ActionHandlerRegistry,
    ) -> Vec<worldwake_sim::Affordance> {
        let beliefs = test_belief_store(world, actor);
        let view = PerAgentBeliefView::new(actor, world, &beliefs);
        get_affordances(&view, actor, defs, handlers)
    }

    fn setup_world() -> (World, EntityId, EntityId, EntityId, EntityId) {
        let mut world = World::new(transport_topology()).unwrap();
        let place = entity(1);
        let other_place = entity(2);
        let (actor, lot) = {
            let mut txn = new_txn(&mut world, 1);
            let actor = txn.create_agent("Aster", ControlSource::Ai).unwrap();
            let lot = txn
                .create_item_lot(CommodityKind::Bread, Quantity(3))
                .unwrap();
            txn.set_ground_location(actor, place).unwrap();
            txn.set_ground_location(lot, place).unwrap();
            txn.set_component_carry_capacity(actor, CarryCapacity(LoadUnits(4)))
                .unwrap();
            commit_txn(txn);
            (actor, lot)
        };
        (world, actor, lot, place, other_place)
    }

    fn setup_registries() -> (
        ActionDefRegistry,
        ActionHandlerRegistry,
        ActionDefId,
        ActionDefId,
    ) {
        let mut defs = ActionDefRegistry::new();
        let mut handlers = ActionHandlerRegistry::new();
        let ids = register_transport_actions(&mut defs, &mut handlers);
        (defs, handlers, ids[0], ids[1])
    }

    #[allow(clippy::too_many_arguments)]
    fn start_action_for_target(
        world: &mut World,
        log: &mut EventLog,
        active_actions: &mut BTreeMap<ActionInstanceId, ActionInstance>,
        rng: &mut DeterministicRng,
        defs: &ActionDefRegistry,
        handlers: &ActionHandlerRegistry,
        actor: EntityId,
        target: EntityId,
    ) -> ActionInstanceId {
        let affordance = affordances_for(world, actor, defs, handlers)
            .into_iter()
            .find(|affordance| affordance.bound_targets == vec![target])
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
                rng,
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
    fn register_transport_actions_creates_pick_up_and_put_down_defs() {
        let (defs, _, pick_up_id, put_down_id) = setup_registries();
        let pick_up = defs.get(pick_up_id).unwrap();
        let put_down = defs.get(put_down_id).unwrap();

        assert_eq!(pick_up.name, "pick_up");
        assert_eq!(put_down.name, "put_down");
        assert!(pick_up
            .preconditions
            .contains(&Precondition::TargetNotInContainer(0)));
        assert!(pick_up
            .preconditions
            .contains(&Precondition::TargetUnpossessed(0)));
        assert!(put_down
            .preconditions
            .contains(&Precondition::TargetDirectlyPossessedByActor(0)));
    }

    #[test]
    fn pick_up_happy_path_moves_lot_into_actor_possession_and_emits_tags() {
        let (mut world, actor, lot, place, _) = setup_world();
        let (defs, handlers, _, _) = setup_registries();
        let mut log = EventLog::new();
        let mut active_actions = BTreeMap::new();
        let mut rng = test_rng();

        let instance_id = start_action_for_target(
            &mut world,
            &mut log,
            &mut active_actions,
            &mut rng,
            &defs,
            &handlers,
            actor,
            lot,
        );

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
                tick: Tick(6),
            },
        )
        .unwrap();

        assert_eq!(
            outcome,
            TickOutcome::Committed {
                outcome: CommitOutcome::empty(),
            }
        );
        assert_eq!(world.possessor_of(lot), Some(actor));
        assert_eq!(world.owner_of(lot), None);
        assert_eq!(world.effective_place(lot), Some(place));

        let record = log
            .get(log.events_by_tag(EventTag::ActionCommitted)[0])
            .unwrap();
        assert!(record.tags().contains(&EventTag::Inventory));
        assert!(record.tags().contains(&EventTag::Transfer));
    }

    #[test]
    fn pick_up_fails_when_target_not_colocated() {
        let (mut world, actor, lot, _, other_place) = setup_world();
        let (defs, handlers, pick_up_id, _) = setup_registries();
        {
            let mut txn = new_txn(&mut world, 2);
            txn.set_ground_location(lot, other_place).unwrap();
            commit_txn(txn);
        }

        let affordance = worldwake_sim::Affordance {
            def_id: pick_up_id,
            actor,
            bound_targets: vec![lot],
            payload_override: None,
            explanation: None,
        };
        let mut log = EventLog::new();
        let mut active_actions = BTreeMap::new();
        let mut next_instance_id = ActionInstanceId(1);
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
    }

    #[test]
    fn pick_up_fails_when_actor_has_no_remaining_capacity() {
        let (mut world, actor, lot, place, _) = setup_world();
        let (defs, handlers, pick_up_id, _) = setup_registries();
        {
            let mut txn = new_txn(&mut world, 2);
            let load_filler = txn
                .create_item_lot(CommodityKind::Water, Quantity(2))
                .unwrap();
            txn.set_ground_location(load_filler, place).unwrap();
            txn.set_possessor(load_filler, actor).unwrap();
            commit_txn(txn);
        }

        let affordance = worldwake_sim::Affordance {
            def_id: pick_up_id,
            actor,
            bound_targets: vec![lot],
            payload_override: None,
            explanation: None,
        };
        let mut log = EventLog::new();
        let mut active_actions = BTreeMap::new();
        let mut next_instance_id = ActionInstanceId(1);
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

        assert!(
            matches!(err, ActionError::PreconditionFailed(message) if message.contains("insufficient carry capacity"))
        );
    }

    #[test]
    fn pick_up_splits_lot_when_only_partial_quantity_fits() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let place = world.topology().place_ids().next().unwrap();
        let (actor, lot) = {
            let mut txn = new_txn(&mut world, 1);
            let actor = txn.create_agent("Aster", ControlSource::Ai).unwrap();
            let lot = txn
                .create_item_lot(CommodityKind::Water, Quantity(3))
                .unwrap();
            txn.set_ground_location(actor, place).unwrap();
            txn.set_ground_location(lot, place).unwrap();
            txn.set_component_carry_capacity(actor, CarryCapacity(LoadUnits(4)))
                .unwrap();
            commit_txn(txn);
            (actor, lot)
        };
        let (defs, handlers, _, _) = setup_registries();
        let mut log = EventLog::new();
        let mut active_actions = BTreeMap::new();
        let mut rng = test_rng();

        let instance_id = start_action_for_target(
            &mut world,
            &mut log,
            &mut active_actions,
            &mut rng,
            &defs,
            &handlers,
            actor,
            lot,
        );
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
                tick: Tick(6),
            },
        )
        .unwrap();

        let direct_possessions = world.possessions_of(actor);
        assert_eq!(direct_possessions.len(), 1);
        let picked_up = direct_possessions[0];
        assert_eq!(
            outcome,
            TickOutcome::Committed {
                outcome: CommitOutcome {
                    materializations: vec![Materialization {
                        tag: MaterializationTag::SplitOffLot,
                        entity: picked_up,
                    }],
                },
            }
        );
        let carried_lot = world.get_component_item_lot(picked_up).unwrap();
        let remaining_lot = world.get_component_item_lot(lot).unwrap();
        assert_eq!(carried_lot.quantity, Quantity(2));
        assert_eq!(remaining_lot.quantity, Quantity(1));
        assert_eq!(world.possessor_of(picked_up), Some(actor));
        assert_eq!(world.owner_of(picked_up), None);
        assert_eq!(world.effective_place(picked_up), Some(place));
        assert_eq!(world.effective_place(lot), Some(place));
    }

    #[test]
    fn pick_up_transport_payload_moves_exact_requested_quantity() {
        let (mut world, actor, lot, place, _) = setup_world();
        let (defs, handlers, pick_up_id, _) = setup_registries();
        let affordance = worldwake_sim::Affordance {
            def_id: pick_up_id,
            actor,
            bound_targets: vec![lot],
            payload_override: Some(ActionPayload::Transport(TransportActionPayload {
                quantity: Quantity(1),
            })),
            explanation: None,
        };
        let mut log = EventLog::new();
        let mut active_actions = BTreeMap::new();
        let mut next_instance_id = ActionInstanceId(1);
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
                tick: Tick(6),
            },
        )
        .unwrap();

        let direct_possessions = world.possessions_of(actor);
        assert_eq!(direct_possessions.len(), 1);
        let picked_up = direct_possessions[0];
        assert_eq!(
            outcome,
            TickOutcome::Committed {
                outcome: CommitOutcome {
                    materializations: vec![Materialization {
                        tag: MaterializationTag::SplitOffLot,
                        entity: picked_up,
                    }],
                },
            }
        );
        assert_eq!(
            world.get_component_item_lot(picked_up).unwrap().quantity,
            Quantity(1)
        );
        assert_eq!(
            world.get_component_item_lot(lot).unwrap().quantity,
            Quantity(2)
        );
        assert_eq!(world.possessor_of(picked_up), Some(actor));
        assert_eq!(world.owner_of(picked_up), None);
        assert_eq!(world.effective_place(picked_up), Some(place));
        assert_eq!(world.effective_place(lot), Some(place));
    }

    #[test]
    fn put_down_happy_path_clears_possession_without_changing_owner() {
        let (mut world, actor, lot, place, _) = setup_world();
        let owner = {
            let mut txn = new_txn(&mut world, 2);
            let owner = txn.create_faction("Granary Guild").unwrap();
            txn.set_owner(lot, owner).unwrap();
            txn.set_possessor(lot, actor).unwrap();
            commit_txn(txn);
            owner
        };
        let (defs, handlers, _, _) = setup_registries();
        let mut log = EventLog::new();
        let mut active_actions = BTreeMap::new();
        let mut rng = test_rng();

        let instance_id = start_action_for_target(
            &mut world,
            &mut log,
            &mut active_actions,
            &mut rng,
            &defs,
            &handlers,
            actor,
            lot,
        );
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
                tick: Tick(6),
            },
        )
        .unwrap();

        assert!(matches!(outcome, TickOutcome::Committed { .. }));
        assert_eq!(world.possessor_of(lot), None);
        assert_eq!(world.owner_of(lot), Some(owner));
        assert_eq!(world.effective_place(lot), Some(place));
    }

    #[test]
    fn put_down_affordance_excludes_ground_and_nested_lots() {
        let (mut world, actor, ground_lot, place, _) = setup_world();
        let carried_lot = {
            let mut txn = new_txn(&mut world, 2);
            let carried_lot = txn
                .create_item_lot(CommodityKind::Apple, Quantity(1))
                .unwrap();
            let bag = txn
                .create_container(Container {
                    capacity: LoadUnits(10),
                    allowed_commodities: None,
                    allows_unique_items: true,
                    allows_nested_containers: true,
                })
                .unwrap();
            let nested_lot = txn
                .create_item_lot(CommodityKind::Bread, Quantity(1))
                .unwrap();
            txn.set_ground_location(bag, place).unwrap();
            txn.set_possessor(bag, actor).unwrap();
            txn.set_possessor(carried_lot, actor).unwrap();
            txn.set_ground_location(carried_lot, place).unwrap();
            txn.put_into_container(nested_lot, bag).unwrap();
            commit_txn(txn);
            carried_lot
        };
        let (defs, handlers, _, put_down_id) = setup_registries();

        let affordances = affordances_for(&world, actor, &defs, &handlers)
            .into_iter()
            .filter(|affordance| affordance.def_id == put_down_id)
            .collect::<Vec<_>>();

        assert_eq!(affordances.len(), 1);
        assert_eq!(affordances[0].bound_targets, vec![carried_lot]);
        assert_ne!(affordances[0].bound_targets, vec![ground_lot]);
    }

    #[test]
    fn put_down_fails_for_non_possessed_lot() {
        let (mut world, actor, lot, _, _) = setup_world();
        let (defs, handlers, _, put_down_id) = setup_registries();
        let affordance = worldwake_sim::Affordance {
            def_id: put_down_id,
            actor,
            bound_targets: vec![lot],
            payload_override: None,
            explanation: None,
        };
        let mut log = EventLog::new();
        let mut active_actions = BTreeMap::new();
        let mut next_instance_id = ActionInstanceId(1);
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
            ActionError::PreconditionFailed("TargetDirectlyPossessedByActor(0)".to_string())
        );
    }

    #[test]
    fn picked_up_lot_moves_with_travel_via_existing_possession_architecture() {
        let (mut world, actor, lot, _, destination) = setup_world();
        let (defs, handlers, _, _) = setup_registries();
        let mut log = EventLog::new();
        let mut active_actions = BTreeMap::new();
        let mut rng = test_rng();

        let pick_up_instance = start_action_for_target(
            &mut world,
            &mut log,
            &mut active_actions,
            &mut rng,
            &defs,
            &handlers,
            actor,
            lot,
        );
        let _ = tick_action(
            pick_up_instance,
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
                tick: Tick(6),
            },
        )
        .unwrap();

        let mut travel_defs = ActionDefRegistry::new();
        let mut travel_handlers = ActionHandlerRegistry::new();
        let travel_id =
            crate::travel_actions::register_travel_actions(&mut travel_defs, &mut travel_handlers);
        let travel_affordance = affordances_for(&world, actor, &travel_defs, &travel_handlers)
            .into_iter()
            .find(|affordance| {
                affordance.def_id == travel_id && affordance.bound_targets == vec![destination]
            })
            .unwrap();
        let mut next_instance_id = ActionInstanceId(2);
        let travel_instance = start_action(
            &travel_affordance,
            &travel_defs,
            &travel_handlers,
            ActionExecutionAuthority {
                active_actions: &mut active_actions,
                world: &mut world,
                event_log: &mut log,
                rng: &mut rng,
            },
            &mut next_instance_id,
            ActionExecutionContext {
                cause: CauseRef::Bootstrap,
                tick: Tick(7),
            },
        )
        .unwrap();

        let _ = tick_action(
            travel_instance,
            &travel_defs,
            &travel_handlers,
            ActionExecutionAuthority {
                active_actions: &mut active_actions,
                world: &mut world,
                event_log: &mut log,
                rng: &mut rng,
            },
            ActionExecutionContext {
                cause: CauseRef::Bootstrap,
                tick: Tick(8),
            },
        )
        .unwrap();
        let outcome = tick_action(
            travel_instance,
            &travel_defs,
            &travel_handlers,
            ActionExecutionAuthority {
                active_actions: &mut active_actions,
                world: &mut world,
                event_log: &mut log,
                rng: &mut rng,
            },
            ActionExecutionContext {
                cause: CauseRef::Bootstrap,
                tick: Tick(9),
            },
        )
        .unwrap();

        assert!(matches!(outcome, TickOutcome::Committed { .. }));
        assert_eq!(world.possessor_of(lot), Some(actor));
        assert_eq!(world.effective_place(lot), Some(destination));
    }

    #[test]
    fn pick_up_affordance_excludes_contained_lots() {
        let (mut world, actor, ground_lot, place, _) = setup_world();
        let contained_lot = {
            let mut txn = new_txn(&mut world, 2);
            let bag = txn
                .create_container(Container {
                    capacity: LoadUnits(10),
                    allowed_commodities: None,
                    allows_unique_items: true,
                    allows_nested_containers: true,
                })
                .unwrap();
            let contained_lot = txn
                .create_item_lot(CommodityKind::Bread, Quantity(1))
                .unwrap();
            txn.set_ground_location(bag, place).unwrap();
            txn.put_into_container(contained_lot, bag).unwrap();
            commit_txn(txn);
            contained_lot
        };
        let (defs, handlers, pick_up_id, _) = setup_registries();

        let affordances = affordances_for(&world, actor, &defs, &handlers)
            .into_iter()
            .filter(|affordance| affordance.def_id == pick_up_id)
            .collect::<Vec<_>>();

        assert!(affordances
            .iter()
            .any(|affordance| affordance.bound_targets == vec![ground_lot]));
        assert!(!affordances
            .iter()
            .any(|affordance| affordance.bound_targets == vec![contained_lot]));
    }
}
