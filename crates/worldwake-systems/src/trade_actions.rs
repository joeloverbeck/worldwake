use std::collections::BTreeSet;
use worldwake_core::{
    BodyCostPerTick, CommodityKind, EntityId, EntityKind, EventTag, LotOperation,
    ProvenanceEntry, Quantity, VisibilitySpec, WorldTxn,
};
use worldwake_sim::{
    evaluate_trade_bundle, AbortReason, ActionDef, ActionDefId, ActionDefRegistry, ActionError,
    ActionHandler, ActionHandlerId, ActionHandlerRegistry, ActionInstance, ActionPayload,
    ActionProgress, ActionState, DurationExpr, Interruptibility, OmniscientBeliefView,
    Precondition, TargetSpec, TradeAcceptance, TradeActionPayload,
};

pub fn register_trade_action(
    defs: &mut ActionDefRegistry,
    handlers: &mut ActionHandlerRegistry,
) -> ActionDefId {
    let handler = handlers.register(ActionHandler::new(
        start_trade,
        tick_trade,
        commit_trade,
        abort_trade,
    ));
    defs.register(trade_action_def(ActionDefId(defs.len() as u32), handler))
}

fn trade_action_def(id: ActionDefId, handler: ActionHandlerId) -> ActionDef {
    ActionDef {
        id,
        name: "trade".to_string(),
        actor_constraints: vec![],
        targets: vec![TargetSpec::EntityAtActorPlace {
            kind: EntityKind::Agent,
        }],
        preconditions: vec![
            Precondition::ActorAlive,
            Precondition::TargetExists(0),
            Precondition::TargetAtActorPlace(0),
            Precondition::TargetKind {
                target_index: 0,
                kind: EntityKind::Agent,
            },
        ],
        reservation_requirements: vec![],
        duration: DurationExpr::ActorTradeDisposition,
        body_cost_per_tick: BodyCostPerTick::zero(),
        interruptibility: Interruptibility::FreelyInterruptible,
        commit_conditions: vec![
            Precondition::ActorAlive,
            Precondition::TargetExists(0),
            Precondition::TargetAtActorPlace(0),
            Precondition::TargetKind {
                target_index: 0,
                kind: EntityKind::Agent,
            },
        ],
        visibility: VisibilitySpec::SamePlace,
        causal_event_tags: BTreeSet::from([
            EventTag::Trade,
            EventTag::Transfer,
            EventTag::WorldMutation,
        ]),
        payload: ActionPayload::None,
        handler,
    }
}

fn trade_payload<'a>(
    def: &ActionDef,
    instance: &'a ActionInstance,
) -> Result<&'a TradeActionPayload, ActionError> {
    instance.payload.as_trade().ok_or_else(|| {
        ActionError::InternalError(format!(
            "action instance for def {} is missing trade payload",
            def.id
        ))
    })
}

#[allow(clippy::unnecessary_wraps)]
fn start_trade(
    def: &ActionDef,
    instance: &ActionInstance,
    _txn: &mut WorldTxn<'_>,
) -> Result<Option<ActionState>, ActionError> {
    let _ = trade_payload(def, instance)?;
    Ok(Some(ActionState::Empty))
}

#[allow(clippy::unnecessary_wraps)]
fn tick_trade(
    _def: &ActionDef,
    _instance: &ActionInstance,
    _txn: &mut WorldTxn<'_>,
) -> Result<ActionProgress, ActionError> {
    Ok(ActionProgress::Continue)
}

fn commit_trade(
    def: &ActionDef,
    instance: &ActionInstance,
    txn: &mut WorldTxn<'_>,
) -> Result<(), ActionError> {
    let payload = trade_payload(def, instance)?;
    let (counterparty, place) = validate_trade_context(txn, instance, payload)?;
    ensure_accessible_quantity(
        txn,
        instance.actor,
        payload.offered_commodity,
        payload.offered_quantity,
    )?;
    ensure_accessible_quantity(
        txn,
        counterparty,
        payload.requested_commodity,
        payload.requested_quantity,
    )?;
    ensure_bundle_accepted(txn, instance.actor, counterparty, payload, place)?;
    execute_trade_transfers(txn, instance.actor, counterparty, payload, place)
}

#[allow(clippy::unnecessary_wraps)]
fn abort_trade(
    _def: &ActionDef,
    _instance: &ActionInstance,
    _reason: &AbortReason,
    _txn: &mut WorldTxn<'_>,
) -> Result<(), ActionError> {
    Ok(())
}

fn validate_trade_context(
    txn: &WorldTxn<'_>,
    instance: &ActionInstance,
    payload: &TradeActionPayload,
) -> Result<(EntityId, EntityId), ActionError> {
    let counterparty = *instance
        .targets
        .first()
        .ok_or(ActionError::InvalidTarget(instance.actor))?;
    if counterparty != payload.counterparty {
        return Err(ActionError::AbortRequested(format!(
            "trade counterparty target {counterparty} does not match payload {}",
            payload.counterparty
        )));
    }
    let place = txn.effective_place(instance.actor).ok_or_else(|| {
        ActionError::AbortRequested(format!("actor {} is not at a tradeable place", instance.actor))
    })?;
    if txn.effective_place(counterparty) != Some(place) {
        return Err(ActionError::AbortRequested(format!(
            "counterparty {counterparty} is no longer co-located"
        )));
    }
    Ok((counterparty, place))
}

fn ensure_bundle_accepted(
    txn: &WorldTxn<'_>,
    actor: EntityId,
    counterparty: EntityId,
    payload: &TradeActionPayload,
    place: EntityId,
) -> Result<(), ActionError> {
    let belief = OmniscientBeliefView::new(txn);
    let actor_acceptance = evaluate_for_participant(
        txn,
        &belief,
        actor,
        counterparty,
        place,
        [(payload.offered_commodity, payload.offered_quantity)],
        [(payload.requested_commodity, payload.requested_quantity)],
    );
    if actor_acceptance != TradeAcceptance::Accept {
        return Err(ActionError::AbortRequested(format!(
            "actor {actor} rejected trade bundle: {actor_acceptance:?}"
        )));
    }

    let counterparty_acceptance = evaluate_for_participant(
        txn,
        &belief,
        counterparty,
        actor,
        place,
        [(payload.requested_commodity, payload.requested_quantity)],
        [(payload.offered_commodity, payload.offered_quantity)],
    );
    if counterparty_acceptance != TradeAcceptance::Accept {
        return Err(ActionError::AbortRequested(format!(
            "counterparty {counterparty} rejected trade bundle: {counterparty_acceptance:?}"
        )));
    }

    Ok(())
}

fn evaluate_for_participant(
    txn: &WorldTxn<'_>,
    belief: &OmniscientBeliefView<'_>,
    actor: EntityId,
    excluded_counterparty: EntityId,
    place: EntityId,
    offered: [(CommodityKind, Quantity); 1],
    received: [(CommodityKind, Quantity); 1],
) -> TradeAcceptance {
    let alternatives = local_alternatives(txn, actor, excluded_counterparty, place);
    evaluate_trade_bundle(
        actor,
        belief,
        txn.get_component_homeostatic_needs(actor),
        txn.get_component_wound_list(actor),
        txn.controlled_commodity_quantity(actor, CommodityKind::Coin),
        &offered,
        &received,
        &alternatives,
        txn.get_component_demand_memory(actor),
    )
}

fn execute_trade_transfers(
    txn: &mut WorldTxn<'_>,
    actor: EntityId,
    counterparty: EntityId,
    payload: &TradeActionPayload,
    place: EntityId,
) -> Result<(), ActionError> {
    let offered_lots = resolve_trade_lots(
        txn,
        actor,
        payload.offered_commodity,
        payload.offered_quantity,
        place,
    )?;
    let requested_lots = resolve_trade_lots(
        txn,
        counterparty,
        payload.requested_commodity,
        payload.requested_quantity,
        place,
    )?;

    transfer_selected_lots(
        txn,
        &offered_lots,
        counterparty,
        place,
        payload.offered_commodity,
    )?;
    transfer_selected_lots(
        txn,
        &requested_lots,
        actor,
        place,
        payload.requested_commodity,
    )
}

fn transfer_selected_lots(
    txn: &mut WorldTxn<'_>,
    lots: &[(EntityId, Quantity)],
    new_holder: EntityId,
    place: EntityId,
    commodity: CommodityKind,
) -> Result<(), ActionError> {
    for (lot_id, quantity) in lots {
        transfer_trade_lot(txn, *lot_id, new_holder, place, *quantity, commodity)?;
    }
    Ok(())
}

fn resolve_trade_lots(
    txn: &mut WorldTxn<'_>,
    holder: EntityId,
    commodity: CommodityKind,
    quantity: Quantity,
    place: EntityId,
) -> Result<Vec<(EntityId, Quantity)>, ActionError> {
    let mut remaining = quantity;
    let mut selected = Vec::new();
    let mut lots = txn
        .query_item_lot()
        .filter_map(|(entity, lot)| {
            (lot.commodity == commodity
                && txn.can_exercise_control(holder, entity).is_ok()
                && txn.effective_place(entity) == Some(place))
            .then_some((entity, lot.quantity))
        })
        .collect::<Vec<_>>();
    lots.sort_by_key(|(entity, _)| *entity);

    for (lot_id, available) in lots {
        if remaining == Quantity(0) {
            break;
        }
        if available > remaining {
            let (_, split_off) = txn
                .split_lot(lot_id, remaining)
                .map_err(|err| ActionError::InternalError(err.to_string()))?;
            selected.push((split_off, remaining));
            remaining = Quantity(0);
            break;
        }

        selected.push((lot_id, available));
        remaining = remaining
            .checked_sub(available)
            .ok_or_else(|| ActionError::InternalError("trade lot accounting underflowed".to_string()))?;
    }

    if remaining != Quantity(0) {
        return Err(ActionError::AbortRequested(format!(
            "holder {holder} lacks accessible {quantity:?} of {commodity:?}"
        )));
    }

    Ok(selected)
}

fn ensure_accessible_quantity(
    txn: &WorldTxn<'_>,
    holder: EntityId,
    commodity: CommodityKind,
    quantity: Quantity,
) -> Result<(), ActionError> {
    let available = txn.controlled_commodity_quantity(holder, commodity);
    if available < quantity {
        return Err(ActionError::AbortRequested(format!(
            "holder {holder} lacks accessible {quantity:?} of {commodity:?}"
        )));
    }
    Ok(())
}

fn transfer_trade_lot(
    txn: &mut WorldTxn<'_>,
    lot_id: EntityId,
    new_holder: EntityId,
    place: EntityId,
    quantity: Quantity,
    commodity: CommodityKind,
) -> Result<(), ActionError> {
    if txn.direct_container(lot_id).is_some() {
        txn.remove_from_container(lot_id)
            .map_err(|err| ActionError::InternalError(err.to_string()))?;
    }
    if txn.possessor_of(lot_id).is_some() {
        txn.clear_possessor(lot_id)
            .map_err(|err| ActionError::InternalError(err.to_string()))?;
    }
    if txn.effective_place(lot_id) != Some(place) {
        txn.set_ground_location(lot_id, place)
            .map_err(|err| ActionError::InternalError(err.to_string()))?;
    }
    txn.set_owner(lot_id, new_holder)
        .map_err(|err| ActionError::InternalError(err.to_string()))?;
    txn.set_possessor(lot_id, new_holder)
        .map_err(|err| ActionError::InternalError(err.to_string()))?;
    txn.append_lot_provenance(
        lot_id,
        ProvenanceEntry {
            tick: txn.tick(),
            event_id: None,
            operation: LotOperation::Traded,
            related_lot: None,
            amount: quantity,
        },
    )
    .map_err(|err| ActionError::InternalError(err.to_string()))?;
    txn.add_target(lot_id);
    debug_assert_eq!(
        txn.get_component_item_lot(lot_id).map(|lot| lot.commodity),
        Some(commodity)
    );
    Ok(())
}

fn local_alternatives(
    txn: &WorldTxn<'_>,
    focal: EntityId,
    counterparty: EntityId,
    place: EntityId,
) -> Vec<(EntityId, CommodityKind, Quantity)> {
    let mut alternatives = Vec::new();
    let mut others = txn.entities_effectively_at(place);
    others.sort();
    others.dedup();
    for other in others {
        if other == focal || other == counterparty || txn.entity_kind(other) != Some(EntityKind::Agent)
        {
            continue;
        }
        for commodity in CommodityKind::ALL {
            let quantity = txn.controlled_commodity_quantity(other, commodity);
            if quantity != Quantity(0) {
                alternatives.push((other, commodity, quantity));
            }
        }
    }
    alternatives
}

#[cfg(test)]
mod tests {
    use super::register_trade_action;
    use crate::trade_actions::local_alternatives;
    use std::collections::BTreeMap;
    use std::num::NonZeroU32;
    use worldwake_core::{
        build_prototype_world, verify_live_lot_conservation, CauseRef, CommodityKind,
        ControlSource, EntityId, EventLog, EventTag, HomeostaticNeeds, LotOperation, Permille,
        Quantity, Tick, TradeDispositionProfile, VisibilitySpec, WitnessData, World, WorldTxn,
    };
    use worldwake_sim::{
        start_action, tick_action, ActionDefId, ActionDefRegistry, ActionExecutionAuthority,
        ActionExecutionContext, ActionHandlerRegistry, ActionInstanceId, ActionPayload,
        ActionStatus, Affordance, TickOutcome, TradeActionPayload,
    };

    fn entity(slot: u32) -> EntityId {
        EntityId {
            slot,
            generation: 1,
        }
    }

    fn pm(value: u16) -> Permille {
        Permille::new(value).unwrap()
    }

    fn nz(value: u32) -> NonZeroU32 {
        NonZeroU32::new(value).unwrap()
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

    struct TradeHarness {
        world: World,
        defs: ActionDefRegistry,
        handlers: ActionHandlerRegistry,
        log: EventLog,
        next_instance_id: ActionInstanceId,
        actor: EntityId,
        counterparty: EntityId,
        actor_offer: EntityId,
        counterparty_offer: EntityId,
        place: EntityId,
        def_id: ActionDefId,
        payload: TradeActionPayload,
    }

    impl TradeHarness {
        fn new(
            payload: &TradeActionPayload,
            actor_ticks: u32,
            actor_needs: HomeostaticNeeds,
        ) -> Self {
            let mut world = World::new(build_prototype_world()).unwrap();
            let place = world.topology().place_ids().next().unwrap();
            let (actor, counterparty, actor_offer, counterparty_offer) = {
                let mut txn = new_txn(&mut world, 1);
                let actor = txn.create_agent("Aster", ControlSource::Ai).unwrap();
                let counterparty = txn.create_agent("Bram", ControlSource::Ai).unwrap();
                let actor_offer = txn
                    .create_item_lot(payload.offered_commodity, payload.offered_quantity)
                    .unwrap();
                let counterparty_offer = txn
                    .create_item_lot(payload.requested_commodity, payload.requested_quantity)
                    .unwrap();
                commit_txn(txn);
                (actor, counterparty, actor_offer, counterparty_offer)
            };
            let payload = TradeActionPayload {
                counterparty,
                ..payload.clone()
            };
            {
                let mut txn = new_txn(&mut world, 2);
                for entity in [actor, counterparty, actor_offer, counterparty_offer] {
                    txn.set_ground_location(entity, place).unwrap();
                }
                txn.set_possessor(actor_offer, actor).unwrap();
                txn.set_owner(actor_offer, actor).unwrap();
                txn.set_possessor(counterparty_offer, counterparty).unwrap();
                txn.set_owner(counterparty_offer, counterparty).unwrap();
                txn.set_component_trade_disposition_profile(
                    actor,
                    TradeDispositionProfile {
                        negotiation_round_ticks: nz(actor_ticks),
                        initial_offer_bias: pm(500),
                        concession_rate: pm(200),
                        demand_memory_retention_ticks: 10,
                    },
                )
                .unwrap();
                txn.set_component_trade_disposition_profile(
                    counterparty,
                    TradeDispositionProfile {
                        negotiation_round_ticks: nz(1),
                        initial_offer_bias: pm(500),
                        concession_rate: pm(200),
                        demand_memory_retention_ticks: 10,
                    },
                )
                .unwrap();
                txn.set_component_homeostatic_needs(actor, actor_needs).unwrap();
                txn.set_component_homeostatic_needs(counterparty, HomeostaticNeeds::new_sated())
                    .unwrap();
                commit_txn(txn);
            }

            let mut defs = ActionDefRegistry::new();
            let mut handlers = ActionHandlerRegistry::new();
            let def_id = register_trade_action(&mut defs, &mut handlers);
            Self {
                world,
                defs,
                handlers,
                log: EventLog::new(),
                next_instance_id: ActionInstanceId(0),
                actor,
                counterparty,
                actor_offer,
                counterparty_offer,
                place,
                def_id,
                payload,
            }
        }

        fn start_with_active(
            &mut self,
        ) -> (ActionInstanceId, BTreeMap<ActionInstanceId, worldwake_sim::ActionInstance>) {
            let affordance = Affordance {
                def_id: self.def_id,
                actor: self.actor,
                bound_targets: vec![self.counterparty],
                payload_override: Some(ActionPayload::Trade(self.payload.clone())),
                explanation: None,
            };
            let mut active = BTreeMap::new();
            let instance_id = start_action(
                &affordance,
                &self.defs,
                &self.handlers,
                ActionExecutionAuthority {
                    active_actions: &mut active,
                    world: &mut self.world,
                    event_log: &mut self.log,
                },
                &mut self.next_instance_id,
                ActionExecutionContext {
                    cause: CauseRef::Bootstrap,
                    tick: Tick(3),
                },
            )
            .unwrap();
            (instance_id, active)
        }
    }

    #[test]
    fn trade_action_duration_resolves_from_actor_profile() {
        let payload = TradeActionPayload {
            counterparty: entity(2),
            offered_commodity: CommodityKind::Coin,
            offered_quantity: Quantity(1),
            requested_commodity: CommodityKind::Bread,
            requested_quantity: Quantity(1),
        };
        let mut harness =
            TradeHarness::new(&payload, 3, HomeostaticNeeds::new(pm(900), pm(0), pm(0), pm(0), pm(0)));
        let (_instance_id, active) = harness.start_with_active();
        let instance = active.values().next().unwrap();

        assert_eq!(instance.remaining_ticks, 3);
        assert_eq!(instance.status, ActionStatus::Active);
    }

    #[test]
    fn successful_trade_transfers_goods_and_coin_with_trade_tags_and_provenance() {
        let payload = TradeActionPayload {
            counterparty: entity(2),
            offered_commodity: CommodityKind::Coin,
            offered_quantity: Quantity(1),
            requested_commodity: CommodityKind::Bread,
            requested_quantity: Quantity(1),
        };
        let mut harness =
            TradeHarness::new(&payload, 1, HomeostaticNeeds::new(pm(900), pm(0), pm(0), pm(0), pm(0)));
        let (instance_id, mut active) = harness.start_with_active();

        let outcome = tick_action(
            instance_id,
            &harness.defs,
            &harness.handlers,
            ActionExecutionAuthority {
                active_actions: &mut active,
                world: &mut harness.world,
                event_log: &mut harness.log,
            },
            ActionExecutionContext {
                cause: CauseRef::Bootstrap,
                tick: Tick(4),
            },
        )
        .unwrap();

        assert_eq!(outcome, TickOutcome::Committed);
        assert_eq!(harness.world.possessor_of(harness.actor_offer), Some(harness.counterparty));
        assert_eq!(harness.world.owner_of(harness.actor_offer), Some(harness.counterparty));
        assert_eq!(harness.world.possessor_of(harness.counterparty_offer), Some(harness.actor));
        assert_eq!(harness.world.owner_of(harness.counterparty_offer), Some(harness.actor));

        let trade_events = harness.log.events_by_tag(EventTag::Trade);
        assert_eq!(trade_events.len(), 1);
        let record = harness.log.get(trade_events[0]).unwrap();
        assert!(record.tags.contains(&EventTag::ActionCommitted));
        assert!(record.tags.contains(&EventTag::Transfer));
        assert!(record.tags.contains(&EventTag::Trade));

        let traded_entry = harness
            .world
            .get_component_item_lot(harness.counterparty_offer)
            .unwrap()
            .provenance
            .last()
            .unwrap();
        assert_eq!(traded_entry.operation, LotOperation::Traded);
        assert_eq!(traded_entry.amount, Quantity(1));

        verify_live_lot_conservation(&harness.world, CommodityKind::Coin, 1).unwrap();
        verify_live_lot_conservation(&harness.world, CommodityKind::Bread, 1).unwrap();
    }

    #[test]
    fn partial_lot_trade_splits_and_preserves_conservation() {
        let payload = TradeActionPayload {
            counterparty: entity(2),
            offered_commodity: CommodityKind::Coin,
            offered_quantity: Quantity(2),
            requested_commodity: CommodityKind::Bread,
            requested_quantity: Quantity(2),
        };
        let mut harness =
            TradeHarness::new(&payload, 1, HomeostaticNeeds::new(pm(900), pm(0), pm(0), pm(0), pm(0)));
        {
            let mut txn = new_txn(&mut harness.world, 3);
            txn.clear_possessor(harness.counterparty_offer).unwrap();
            txn.clear_owner(harness.counterparty_offer).unwrap();
            txn.archive_entity(harness.counterparty_offer).unwrap();
            let replacement = txn.create_item_lot(CommodityKind::Bread, Quantity(3)).unwrap();
            txn.set_ground_location(replacement, harness.place).unwrap();
            txn.set_possessor(replacement, harness.counterparty).unwrap();
            txn.set_owner(replacement, harness.counterparty).unwrap();
            commit_txn(txn);
            harness.counterparty_offer = replacement;
        }
        let (instance_id, mut active) = harness.start_with_active();
        let outcome = tick_action(
            instance_id,
            &harness.defs,
            &harness.handlers,
            ActionExecutionAuthority {
                active_actions: &mut active,
                world: &mut harness.world,
                event_log: &mut harness.log,
            },
            ActionExecutionContext {
                cause: CauseRef::Bootstrap,
                tick: Tick(4),
            },
        )
        .unwrap();

        assert_eq!(outcome, TickOutcome::Committed);
        verify_live_lot_conservation(&harness.world, CommodityKind::Bread, 3).unwrap();
    }

    #[test]
    fn trade_aborts_when_counterparty_leaves_before_commit() {
        let payload = TradeActionPayload {
            counterparty: entity(2),
            offered_commodity: CommodityKind::Coin,
            offered_quantity: Quantity(1),
            requested_commodity: CommodityKind::Bread,
            requested_quantity: Quantity(1),
        };
        let mut harness =
            TradeHarness::new(&payload, 1, HomeostaticNeeds::new(pm(900), pm(0), pm(0), pm(0), pm(0)));
        let (instance_id, mut active) = harness.start_with_active();
        let other_place = harness.world.topology().place_ids().nth(1).unwrap();
        {
            let mut txn = new_txn(&mut harness.world, 4);
            txn.set_ground_location(harness.counterparty, other_place).unwrap();
            commit_txn(txn);
        }

        let outcome = tick_action(
            instance_id,
            &harness.defs,
            &harness.handlers,
            ActionExecutionAuthority {
                active_actions: &mut active,
                world: &mut harness.world,
                event_log: &mut harness.log,
            },
            ActionExecutionContext {
                cause: CauseRef::Bootstrap,
                tick: Tick(5),
            },
        )
        .unwrap();

        assert!(matches!(outcome, TickOutcome::Aborted { .. }));
        assert_eq!(harness.log.events_by_tag(EventTag::ActionAborted).len(), 1);
    }

    #[test]
    fn trade_aborts_when_bundle_is_rejected() {
        let payload = TradeActionPayload {
            counterparty: entity(2),
            offered_commodity: CommodityKind::Coin,
            offered_quantity: Quantity(1),
            requested_commodity: CommodityKind::Bread,
            requested_quantity: Quantity(1),
        };
        let mut harness = TradeHarness::new(&payload, 1, HomeostaticNeeds::new_sated());
        let (instance_id, mut active) = harness.start_with_active();

        let outcome = tick_action(
            instance_id,
            &harness.defs,
            &harness.handlers,
            ActionExecutionAuthority {
                active_actions: &mut active,
                world: &mut harness.world,
                event_log: &mut harness.log,
            },
            ActionExecutionContext {
                cause: CauseRef::Bootstrap,
                tick: Tick(4),
            },
        )
        .unwrap();

        assert!(matches!(outcome, TickOutcome::Aborted { .. }));
        assert_eq!(harness.world.possessor_of(harness.actor_offer), Some(harness.actor));
        assert_eq!(
            harness.world.possessor_of(harness.counterparty_offer),
            Some(harness.counterparty)
        );
    }

    #[test]
    fn local_alternatives_exclude_focal_and_counterparty() {
        let payload = TradeActionPayload {
            counterparty: entity(2),
            offered_commodity: CommodityKind::Coin,
            offered_quantity: Quantity(1),
            requested_commodity: CommodityKind::Bread,
            requested_quantity: Quantity(1),
        };
        let mut harness = TradeHarness::new(&payload, 1, HomeostaticNeeds::new_sated());
        let bystander = {
            let mut txn = new_txn(&mut harness.world, 3);
            let bystander = txn.create_agent("Cato", ControlSource::Ai).unwrap();
            let stock = txn.create_item_lot(CommodityKind::Bread, Quantity(2)).unwrap();
            txn.set_ground_location(bystander, harness.place).unwrap();
            txn.set_ground_location(stock, harness.place).unwrap();
            txn.set_possessor(stock, bystander).unwrap();
            txn.set_owner(stock, bystander).unwrap();
            commit_txn(txn);
            bystander
        };

        let txn = new_txn(&mut harness.world, 4);
        let alternatives = local_alternatives(&txn, harness.actor, harness.counterparty, harness.place);
        drop(txn);

        assert_eq!(alternatives, vec![(bystander, CommodityKind::Bread, Quantity(2))]);
    }
}
