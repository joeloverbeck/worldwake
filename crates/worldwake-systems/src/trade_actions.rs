use std::{collections::{BTreeMap, BTreeSet}, num::NonZeroU32};
use worldwake_core::{
    ActionDefId, BlockedIntent, BlockerKey, BlockingFact, BodyCostPerTick,
    CommodityKind, DemandMemory, DemandObservation, DemandObservationReason, EntityId, EntityKind,
    EventTag, GoalKey, GoalKind, MerchandiseProfile, Quantity, Tick, VisibilitySpec,
    WorldTxn, WoundList,
};
use worldwake_sim::{
    commodity_opportunity_score, evaluate_trade_bundle, AbortReason, ActionAbortRequestReason,
    ActionDef, ActionDefRegistry, ActionError, ActionHandler, ActionHandlerId,
    ActionHandlerRegistry, ActionInstance, ActionPayload, ActionProgress, ActionState,
    CommitOutcome, DeterministicRng, DurationExpr, GoalBeliefView, Interruptibility,
    PayloadEntityRole, PerAgentBeliefView, Precondition, RecipeRegistry, RuntimeBeliefView,
    StaffMarketPayload, TargetSpec, TradeAcceptance, TradeActionPayload, TradeRejectionReason,
};

pub fn register_trade_action(
    defs: &mut ActionDefRegistry,
    handlers: &mut ActionHandlerRegistry,
) -> ActionDefId {
    let handler = handlers.register(
        ActionHandler::new(start_trade, tick_trade, commit_trade, abort_trade)
            .with_affordance_payloads(enumerate_trade_payloads)
            .with_payload_override_validator(validate_trade_payload_override),
    );
    defs.register(trade_action_def(ActionDefId(defs.len() as u32), handler))
}

fn trade_action_def(id: ActionDefId, handler: ActionHandlerId) -> ActionDef {
    ActionDef {
        id,
        name: "trade".to_string(),
        domain: worldwake_core::ActionDomain::Trade,
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

fn enumerate_trade_payloads(
    _def: &ActionDef,
    actor: EntityId,
    targets: &[EntityId],
    view: &dyn RuntimeBeliefView,
) -> Vec<ActionPayload> {
    let Some(counterparty) = targets.first().copied() else {
        return Vec::new();
    };
    let Some(place) = view.effective_place(actor) else {
        return Vec::new();
    };
    if view.merchandise_profile(counterparty).is_none() {
        return Vec::new();
    }
    if view.commodity_quantity(actor, CommodityKind::Coin) == Quantity(0) {
        return Vec::new();
    }
    let Some(disposition) = view.trade_disposition_profile(actor) else {
        return Vec::new();
    };

    // Discover concrete listed sale lots at this place for each commodity the
    // counterparty is selling.  We iterate sale_kinds to know which commodities
    // to look for, then find listed lots for each.
    let profile = view.merchandise_profile(counterparty).unwrap();
    let mut payloads: Vec<ActionPayload> = Vec::new();
    for commodity in &profile.sale_kinds {
        let reservation = buyer_reservation_price(
            view.homeostatic_needs(actor).as_ref(),
            wounds_for(view, actor).as_ref(),
            *commodity,
            view.commodity_quantity(actor, CommodityKind::Coin),
            count_local_alternatives(view, actor, counterparty, place, *commodity),
        );
        if reservation < Quantity(1) {
            continue;
        }
        let opening_offer = derive_opening_offer(
            worldwake_core::TradeRole::Buyer,
            reservation,
            disposition.initial_offer_bias,
            disposition.rejection_escalation_rate,
            rejection_count_for(view, actor, counterparty, *commodity),
        );
        for lot in view.listed_sale_lots_at(place, *commodity) {
            if view.seller_for_sale_lot(lot) != Some(counterparty) {
                continue;
            }
            let payload = TradeActionPayload {
                counterparty,
                sale_lot: lot,
                offered_commodity: CommodityKind::Coin,
                offered_quantity: opening_offer,
                requested_quantity: Quantity(1),
            };
            payloads.push(ActionPayload::Trade(payload));
        }
    }
    payloads.sort();
    payloads.dedup();
    payloads
}

/// Derives the requested commodity from the sale lot's item lot component.
/// The sale lot is the authoritative source; `requested_commodity` is never
/// stored in the payload.
fn sale_lot_commodity(txn: &WorldTxn<'_>, sale_lot: EntityId) -> Result<CommodityKind, ActionError> {
    txn.get_component_item_lot(sale_lot)
        .map(|lot| lot.commodity)
        .ok_or_else(|| {
            ActionError::AbortRequested(ActionAbortRequestReason::PayloadEntityMismatch {
                role: PayloadEntityRole::SaleLot,
                expected: sale_lot,
                actual: sale_lot,
            })
        })
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SubstituteTradeCandidate {
    pub seller: EntityId,
    pub commodity: CommodityKind,
    pub quantity: Quantity,
}

#[allow(clippy::unnecessary_wraps)]
fn start_trade(
    def: &ActionDef,
    instance: &mut ActionInstance,
    _context: &worldwake_sim::ActionExecutionContext<'_>,
    _rng: &mut DeterministicRng,
    txn: &mut WorldTxn<'_>,
) -> Result<Option<ActionState>, ActionError> {
    let payload = trade_payload(def, instance)?;
    let _ = validate_trade_context_for_negotiation(txn, instance, payload)?;
    Ok(Some(ActionState::Trade {
        round: 0,
        initiator_role: worldwake_core::TradeRole::Buyer,
        initiator_last_offer: Some(payload.offered_quantity),
        responder_last_offer: None,
        agreed_price: None,
    }))
}

fn tick_trade(
    def: &ActionDef,
    instance: &mut ActionInstance,
    context: &worldwake_sim::ActionExecutionContext<'_>,
    _rng: &mut DeterministicRng,
    txn: &mut WorldTxn<'_>,
) -> Result<ActionProgress, ActionError> {
    let payload = trade_payload(def, instance)?;
    let (counterparty, place, requested_commodity) =
        validate_trade_context_for_negotiation(txn, instance, payload)?;
    let state = match instance.local_state {
        Some(ActionState::Trade {
            round,
            initiator_role,
            initiator_last_offer,
            responder_last_offer,
            agreed_price,
        }) => (
            round,
            initiator_role,
            initiator_last_offer,
            responder_last_offer,
            agreed_price,
        ),
        _ => {
            return Err(ActionError::InternalError(
                "trade action missing negotiation state".to_string(),
            ))
        }
    };
    let (round, initiator_role, initiator_last_offer, responder_last_offer, agreed_price) = state;
    if agreed_price.is_some() {
        return Ok(ActionProgress::Complete);
    }

    let responder_role = opposite_trade_role(initiator_role);
    let (current_actor, current_role, current_offer) = if round % 2 == 0 {
        (counterparty, responder_role, initiator_last_offer)
    } else {
        (instance.actor, initiator_role, responder_last_offer)
    };
    let current_offer = current_offer.ok_or_else(|| {
        ActionError::InternalError("trade negotiation lost current offer".to_string())
    })?;
    let reservation = reservation_price_for_actor(
        txn,
        context.recipe_registry,
        current_actor,
        current_role,
        payload.offered_commodity,
        requested_commodity,
        place,
        if current_actor == instance.actor {
            counterparty
        } else {
            instance.actor
        },
    );

    if offer_satisfies_reservation(current_role, current_offer, reservation) {
        instance.local_state = Some(ActionState::Trade {
            round,
            initiator_role,
            initiator_last_offer,
            responder_last_offer,
            agreed_price: Some(current_offer),
        });
        return Ok(ActionProgress::Complete);
    }

    let profile = txn
        .get_component_trade_disposition_profile(current_actor)
        .cloned()
        .ok_or_else(|| {
            ActionError::AbortRequested(ActionAbortRequestReason::TradeBundleRejected {
                participant: current_actor,
                acceptance: TradeAcceptance::Reject {
                    reason: TradeRejectionReason::InsufficientPayment,
                },
            })
        })?;
    let deadline = urgency_modulated_deadline(
        profile.negotiation_round_ticks,
        txn.get_component_homeostatic_needs(current_actor),
        requested_commodity,
    );
    if round >= deadline {
        return Err(ActionError::AbortRequested(
            ActionAbortRequestReason::TradeBundleRejected {
                participant: current_actor,
                acceptance: TradeAcceptance::Reject {
                    reason: TradeRejectionReason::InsufficientPayment,
                },
            },
        ));
    }

    let opening = if current_role == initiator_role {
        initiator_last_offer.unwrap_or_else(|| {
            derive_opening_offer(
                current_role,
                reservation,
                profile.initial_offer_bias,
                profile.rejection_escalation_rate,
                0,
            )
        })
    } else {
        responder_last_offer.unwrap_or_else(|| {
            derive_opening_offer(
                current_role,
                reservation,
                profile.initial_offer_bias,
                profile.rejection_escalation_rate,
                0,
            )
        })
    };
    let generated = generate_offer(
        current_role,
        reservation,
        opening,
        round,
        deadline,
        profile.concession_rate,
    );
    let next_offer = match current_role {
        worldwake_core::TradeRole::Buyer => initiator_last_offer.map_or(generated, |prev| {
            Quantity(generated.0.max(prev.0))
        }),
        worldwake_core::TradeRole::Seller => responder_last_offer.map_or(generated, |prev| {
            Quantity(generated.0.min(prev.0))
        }),
    };

    instance.local_state = Some(ActionState::Trade {
        round: round.saturating_add(1),
        initiator_role,
        initiator_last_offer: if current_role == initiator_role {
            Some(next_offer)
        } else {
            initiator_last_offer
        },
        responder_last_offer: if current_role == responder_role {
            Some(next_offer)
        } else {
            responder_last_offer
        },
        agreed_price: None,
    });
    Ok(ActionProgress::Continue)
}

fn commit_trade(
    def: &ActionDef,
    instance: &ActionInstance,
    _context: &worldwake_sim::ActionExecutionContext<'_>,
    _event_log: &worldwake_core::EventLog,
    _rng: &mut DeterministicRng,
    txn: &mut WorldTxn<'_>,
) -> Result<CommitOutcome, ActionError> {
    let payload = trade_payload(def, instance)?;
    let Some(ActionState::Trade {
        agreed_price: Some(agreed_price),
        ..
    }) = instance.local_state
    else {
        return Err(ActionError::AbortRequested(
            ActionAbortRequestReason::TradeBundleRejected {
                participant: instance.actor,
                acceptance: TradeAcceptance::Reject {
                    reason: TradeRejectionReason::InsufficientPayment,
                },
            },
        ));
    };
    let mut agreed_payload = payload.clone();
    agreed_payload.offered_quantity = agreed_price;
    let (counterparty, place, requested_commodity) =
        validate_trade_context_for_negotiation(txn, instance, &agreed_payload)?;
    execute_trade_transfers(txn, instance.actor, counterparty, &agreed_payload, place)?;
    record_trade_observation(
        txn,
        instance.actor,
        requested_commodity,
        agreed_price,
        place,
        Some(counterparty),
        DemandObservationReason::TradeAgreed,
    );
    record_trade_observation(
        txn,
        counterparty,
        requested_commodity,
        agreed_price,
        place,
        Some(instance.actor),
        DemandObservationReason::TradeAgreed,
    );
    Ok(CommitOutcome::empty())
}

#[allow(clippy::unnecessary_wraps)]
fn abort_trade(
    _def: &ActionDef,
    instance: &ActionInstance,
    _context: &worldwake_sim::ActionExecutionContext<'_>,
    _reason: &AbortReason,
    _event_log: &worldwake_core::EventLog,
    _rng: &mut DeterministicRng,
    txn: &mut WorldTxn<'_>,
) -> Result<(), ActionError> {
    let Some(payload) = instance.payload.as_trade() else {
        return Ok(());
    };
    let Ok((counterparty, place)) = validate_trade_context(txn, instance, payload) else {
        return Ok(());
    };
    let commodity = sale_lot_commodity(txn, payload.sale_lot).unwrap_or(CommodityKind::Coin);
    record_trade_observation(
        txn,
        instance.actor,
        commodity,
        payload.requested_quantity,
        place,
        Some(counterparty),
        DemandObservationReason::WantedToBuyButTooExpensive,
    );
    record_trade_observation(
        txn,
        counterparty,
        commodity,
        payload.requested_quantity,
        place,
        Some(instance.actor),
        DemandObservationReason::WantedToSellButNoBuyer,
    );
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
        return Err(ActionError::AbortRequested(
            ActionAbortRequestReason::PayloadEntityMismatch {
                role: PayloadEntityRole::Counterparty,
                expected: counterparty,
                actual: payload.counterparty,
            },
        ));
    }
    let place = txn.effective_place(instance.actor).ok_or({
        ActionError::AbortRequested(ActionAbortRequestReason::ActorNotPlaced {
            actor: instance.actor,
        })
    })?;
    if txn.effective_place(counterparty) != Some(place) {
        return Err(ActionError::AbortRequested(
            ActionAbortRequestReason::TargetNotColocated {
                actor: instance.actor,
                target: counterparty,
            },
        ));
    }
    Ok((counterparty, place))
}

fn validate_trade_context_for_negotiation(
    txn: &WorldTxn<'_>,
    instance: &ActionInstance,
    payload: &TradeActionPayload,
) -> Result<(EntityId, EntityId, CommodityKind), ActionError> {
    let (counterparty, place) = validate_trade_context(txn, instance, payload)?;
    let requested_commodity = sale_lot_commodity(txn, payload.sale_lot)?;
    if txn.get_component_sale_listing(payload.sale_lot).is_none() {
        return Err(ActionError::AbortRequested(
            ActionAbortRequestReason::SaleLotNotListed {
                sale_lot: payload.sale_lot,
            },
        ));
    }
    if txn.can_exercise_control(counterparty, payload.sale_lot).is_err() {
        return Err(ActionError::AbortRequested(
            ActionAbortRequestReason::SaleLotNotPossessedBySeller {
                sale_lot: payload.sale_lot,
                seller: counterparty,
            },
        ));
    }
    ensure_accessible_quantity(
        txn,
        instance.actor,
        payload.offered_commodity,
        payload.offered_quantity,
    )?;
    ensure_accessible_quantity(
        txn,
        counterparty,
        requested_commodity,
        payload.requested_quantity,
    )?;
    Ok((counterparty, place, requested_commodity))
}

fn validate_trade_payload_override(
    _def: &ActionDef,
    actor: EntityId,
    _targets: &[EntityId],
    payload: &ActionPayload,
    view: &dyn RuntimeBeliefView,
) -> bool {
    let Some(payload) = payload.as_trade() else {
        return false;
    };
    payload.offered_commodity == CommodityKind::Coin
        && payload.offered_quantity >= Quantity(1)
        && payload.offered_quantity <= view.commodity_quantity(actor, CommodityKind::Coin)
}

fn local_trade_alternatives(
    view: &dyn RuntimeBeliefView,
    actor: EntityId,
    excluded_counterparty: EntityId,
    place: EntityId,
) -> Vec<(EntityId, CommodityKind, Quantity)> {
    let mut alternatives = view
        .entities_at(place)
        .into_iter()
        .filter(|entity| {
            *entity != actor
                && *entity != excluded_counterparty
                && view.entity_kind(*entity) == Some(EntityKind::Agent)
        })
        .flat_map(|seller| {
            view.merchandise_profile(seller)
                .into_iter()
                .flat_map(move |profile| {
                    profile.sale_kinds.into_iter().filter_map(move |commodity| {
                        let quantity = view.commodity_quantity(seller, commodity);
                        (quantity > Quantity(0)).then_some((seller, commodity, quantity))
                    })
                })
        })
        .collect::<Vec<_>>();
    alternatives.sort();
    alternatives
}

#[allow(dead_code)]
pub(crate) fn count_local_alternatives(
    view: &dyn RuntimeBeliefView,
    actor: EntityId,
    excluded_counterparty: EntityId,
    place: EntityId,
    commodity: CommodityKind,
) -> u32 {
    local_trade_alternatives(view, actor, excluded_counterparty, place)
        .into_iter()
        .filter(|(_, kind, quantity)| *kind == commodity && *quantity > Quantity(0))
        .count() as u32
}

#[allow(dead_code)]
pub(crate) fn rejection_count_for(
    view: &dyn RuntimeBeliefView,
    actor: EntityId,
    counterparty: EntityId,
    commodity: CommodityKind,
) -> u32 {
    view.demand_memory(actor)
        .into_iter()
        .filter(|obs| {
            obs.reason == DemandObservationReason::WantedToBuyButTooExpensive
                && obs.counterparty == Some(counterparty)
                && obs.commodity == commodity
        })
        .count() as u32
}

#[allow(dead_code)]
pub(crate) fn buyer_reservation_price(
    needs: Option<&worldwake_core::HomeostaticNeeds>,
    wounds: Option<&WoundList>,
    commodity: CommodityKind,
    current_coin: Quantity,
    local_alternatives: u32,
) -> Quantity {
    if current_coin == Quantity(0) {
        return Quantity(0);
    }

    let need_pressure = commodity_need_pressure(needs, wounds, commodity);
    if need_pressure == 0 {
        return Quantity(1).min(current_coin);
    }

    let scarcity_adjusted = need_pressure.div_ceil(local_alternatives.saturating_add(1));
    Quantity(scarcity_adjusted.max(1)).min(current_coin)
}

#[allow(dead_code)]
pub(crate) fn seller_reservation_price(
    needs: Option<&worldwake_core::HomeostaticNeeds>,
    commodity: CommodityKind,
    current_stock: Quantity,
    demand_memory: Option<&DemandMemory>,
) -> Quantity {
    let stock_units = current_stock.0.max(1);
    let self_need_pressure = commodity_need_pressure(needs, None, commodity);
    let demand_pressure = remembered_demand_pressure(demand_memory, commodity);
    let scarcity_floor = 1_u32.div_ceil(stock_units);
    let scarcity_need = self_need_pressure.div_ceil(stock_units);
    let scarcity_demand = demand_pressure.div_ceil(stock_units);

    Quantity(
        scarcity_floor
            .saturating_add(scarcity_need)
            .saturating_add(scarcity_demand)
            .max(1),
    )
}

#[allow(dead_code)]
pub(crate) fn generate_offer(
    role: worldwake_core::TradeRole,
    reservation: Quantity,
    opening: Quantity,
    round: u32,
    deadline: u32,
    concession_rate: worldwake_core::Permille,
) -> Quantity {
    let progress = concession_progress(round, deadline, concession_rate);
    match role {
        worldwake_core::TradeRole::Buyer => {
            let floor = 1;
            let ceiling = reservation.0.max(floor);
            let start = opening.0.clamp(floor, ceiling);
            let span = ceiling.saturating_sub(start);
            Quantity(start.saturating_add(scale_by_permille(span, progress)).min(ceiling))
        }
        worldwake_core::TradeRole::Seller => {
            let floor = reservation.0.max(1);
            let start = opening.0.max(floor);
            let span = start.saturating_sub(floor);
            Quantity(start.saturating_sub(scale_by_permille(span, progress)).max(floor))
        }
    }
}

#[allow(dead_code)]
pub(crate) fn derive_opening_offer(
    role: worldwake_core::TradeRole,
    reservation: Quantity,
    initial_offer_bias: worldwake_core::Permille,
    rejection_escalation_rate: worldwake_core::Permille,
    prior_rejections: u32,
) -> Quantity {
    let bias = u32::from(initial_offer_bias.value());
    let capped_rejections = prior_rejections.min(4);
    let shift_per_rejection =
        scale_by_permille(reservation.0.max(1), u32::from(rejection_escalation_rate.value()));
    let total_shift = shift_per_rejection.saturating_mul(capped_rejections);

    match role {
        worldwake_core::TradeRole::Buyer => {
            let generous_floor = 1;
            let span = reservation.0.saturating_sub(generous_floor);
            let base =
                reservation.0.saturating_sub(scale_by_permille(span, bias)).max(generous_floor);
            Quantity(base.saturating_add(total_shift).min(reservation.0.max(generous_floor)))
        }
        worldwake_core::TradeRole::Seller => {
            let floor = 1;
            let span = reservation.0.saturating_sub(floor);
            let base = reservation.0.saturating_add(scale_by_permille(span, bias));
            Quantity(base.saturating_sub(total_shift).max(floor))
        }
    }
}

#[allow(dead_code)]
pub(crate) fn urgency_modulated_deadline(
    base_patience: NonZeroU32,
    needs: Option<&worldwake_core::HomeostaticNeeds>,
    commodity: CommodityKind,
) -> u32 {
    let urgency = relevant_trade_urgency(needs, commodity);
    let scaled = (u64::from(base_patience.get()) * u64::from(1000_u32.saturating_sub(urgency))
        / 1000) as u32;
    scaled.max(1)
}

#[allow(dead_code)]
fn commodity_need_pressure(
    needs: Option<&worldwake_core::HomeostaticNeeds>,
    wounds: Option<&WoundList>,
    commodity: CommodityKind,
) -> u32 {
    let Some(needs) = needs else {
        return treatment_pressure(wounds, commodity);
    };

    let consumable_pressure = commodity
        .spec()
        .consumable_profile
        .map_or(0, |profile| {
            let hunger_units = units_to_cover_pressure(
                u32::from(needs.hunger.value()),
                u32::from(profile.hunger_relief_per_unit.value()),
            );
            let thirst_units = units_to_cover_pressure(
                u32::from(needs.thirst.value()),
                u32::from(profile.thirst_relief_per_unit.value()),
            );
            hunger_units.max(thirst_units)
        });

    consumable_pressure.max(treatment_pressure(wounds, commodity))
}

#[allow(dead_code)]
fn treatment_pressure(wounds: Option<&WoundList>, commodity: CommodityKind) -> u32 {
    let Some(profile) = commodity.spec().treatment_profile else {
        return 0;
    };
    let Some(wounds) = wounds else {
        return 0;
    };
    let total_wound_load = wounds.wound_load();
    if total_wound_load == 0 {
        return 0;
    }

    let per_unit_treatment = u32::from(profile.severity_reduction_per_tick.value())
        .saturating_mul(profile.treatment_ticks_per_unit.get());
    units_to_cover_pressure(total_wound_load, per_unit_treatment)
}

#[allow(dead_code)]
fn remembered_demand_pressure(demand_memory: Option<&DemandMemory>, commodity: CommodityKind) -> u32 {
    demand_memory
        .map_or(0, |memory| {
            memory
                .observations
                .iter()
                .filter(|obs| obs.commodity == commodity)
                .map(|obs| obs.quantity.0)
                .sum()
        })
}

#[allow(dead_code)]
fn units_to_cover_pressure(pressure: u32, relief_per_unit: u32) -> u32 {
    if pressure == 0 || relief_per_unit == 0 {
        return 0;
    }
    pressure.div_ceil(relief_per_unit)
}

#[allow(dead_code)]
fn concession_progress(round: u32, deadline: u32, concession_rate: worldwake_core::Permille) -> u32 {
    let capped_deadline = deadline.max(1);
    let normalized_round = round.min(capped_deadline);
    let linear = ((u64::from(normalized_round) * 1000) / u64::from(capped_deadline)) as u32;
    let slow_curve = scale_by_permille(linear, linear);
    let remaining = 1000_u32.saturating_sub(linear);
    let fast_curve = 1000_u32.saturating_sub(scale_by_permille(remaining, remaining));
    let rate = u32::from(concession_rate.value());

    match rate.cmp(&500) {
        std::cmp::Ordering::Less => blend_permille(linear, slow_curve, (500 - rate) * 2),
        std::cmp::Ordering::Equal => linear,
        std::cmp::Ordering::Greater => blend_permille(linear, fast_curve, (rate - 500) * 2),
    }
}

#[allow(dead_code)]
fn relevant_trade_urgency(
    needs: Option<&worldwake_core::HomeostaticNeeds>,
    commodity: CommodityKind,
) -> u32 {
    let Some(needs) = needs else {
        return 0;
    };

    match commodity {
        CommodityKind::Water => u32::from(needs.thirst.value()),
        CommodityKind::Apple | CommodityKind::Grain | CommodityKind::Bread => {
            u32::from(needs.hunger.value())
        }
        _ => 0,
    }
}

#[allow(dead_code)]
fn scale_by_permille(value: u32, permille: u32) -> u32 {
    ((u64::from(value) * u64::from(permille)) / 1000) as u32
}

#[allow(dead_code)]
fn blend_permille(start: u32, end: u32, weight: u32) -> u32 {
    let capped = weight.min(1000);
    let start_weight = 1000_u32.saturating_sub(capped);
    ((u64::from(start) * u64::from(start_weight) + u64::from(end) * u64::from(capped)) / 1000)
        as u32
}

fn wounds_for(view: &dyn RuntimeBeliefView, actor: EntityId) -> Option<WoundList> {
    let wounds = view.wounds(actor);
    (!wounds.is_empty()).then_some(WoundList { wounds })
}

#[allow(dead_code)]
fn demand_memory_for(
    view: &dyn RuntimeBeliefView,
    actor: EntityId,
) -> Option<worldwake_core::DemandMemory> {
    let observations = view.demand_memory(actor);
    (!observations.is_empty()).then_some(worldwake_core::DemandMemory { observations })
}

fn opposite_trade_role(role: worldwake_core::TradeRole) -> worldwake_core::TradeRole {
    match role {
        worldwake_core::TradeRole::Buyer => worldwake_core::TradeRole::Seller,
        worldwake_core::TradeRole::Seller => worldwake_core::TradeRole::Buyer,
    }
}

fn offer_satisfies_reservation(
    role: worldwake_core::TradeRole,
    offer: Quantity,
    reservation: Quantity,
) -> bool {
    match role {
        worldwake_core::TradeRole::Buyer => offer <= reservation,
        worldwake_core::TradeRole::Seller => offer >= reservation,
    }
}

#[allow(clippy::too_many_arguments)]
fn reservation_price_for_actor(
    txn: &WorldTxn<'_>,
    recipe_registry: &RecipeRegistry,
    actor: EntityId,
    role: worldwake_core::TradeRole,
    offered_commodity: CommodityKind,
    requested_commodity: CommodityKind,
    place: EntityId,
    counterparty: EntityId,
) -> Quantity {
    let belief = PerAgentBeliefView::from_world_with_recipes(actor, txn, recipe_registry);
    match role {
        worldwake_core::TradeRole::Buyer => {
            buyer_reservation_price_for_view(&belief, actor, counterparty, place, requested_commodity)
        }
        worldwake_core::TradeRole::Seller => seller_reservation_price_for_view(
            &belief,
            actor,
            counterparty,
            place,
            offered_commodity,
            requested_commodity,
        ),
    }
}

fn record_trade_observation(
    txn: &mut WorldTxn<'_>,
    actor: EntityId,
    commodity: CommodityKind,
    quantity: Quantity,
    place: EntityId,
    counterparty: Option<EntityId>,
    reason: DemandObservationReason,
) {
    let mut memory = txn
        .get_component_demand_memory(actor)
        .cloned()
        .unwrap_or(DemandMemory {
            observations: Vec::new(),
        });
    memory.observations.push(DemandObservation {
        commodity,
        quantity,
        place,
        tick: txn.tick(),
        counterparty,
        reason,
    });
    let _ = txn.set_component_demand_memory(actor, memory);
}

fn evaluate_for_participant(
    txn: &WorldTxn<'_>,
    recipe_registry: &RecipeRegistry,
    actor: EntityId,
    excluded_counterparty: EntityId,
    place: EntityId,
    offered: [(CommodityKind, Quantity); 1],
    received: [(CommodityKind, Quantity); 1],
) -> TradeAcceptance {
    let belief = PerAgentBeliefView::from_world_with_recipes(actor, txn, recipe_registry);
    let alternatives = local_alternatives(txn, actor, excluded_counterparty, place);
    evaluate_trade_bundle(
        actor,
        &belief,
        txn.get_component_homeostatic_needs(actor),
        txn.get_component_wound_list(actor),
        txn.controlled_commodity_quantity(actor, CommodityKind::Coin),
        &offered,
        &received,
        &alternatives,
        txn.get_component_demand_memory(actor),
    )
}

/// Selects the first locally available, valuation-approved substitute trade in stored preference order.
pub fn select_substitute_trade_candidate(
    txn: &WorldTxn<'_>,
    buyer: EntityId,
    desired_commodity: CommodityKind,
    desired_quantity: Quantity,
    offered_commodity: CommodityKind,
    offered_quantity: Quantity,
    place: EntityId,
) -> Option<SubstituteTradeCandidate> {
    let preferences = txn.get_component_substitute_preferences(buyer)?;
    let desired_category = desired_commodity.spec().trade_category;
    let substitutes = preferences.preferences.get(&desired_category)?;

    let mut sellers = txn.entities_effectively_at(place);
    sellers.sort();
    sellers.dedup();

    for substitute in substitutes.iter().copied() {
        if substitute == desired_commodity {
            continue;
        }

        for seller in sellers.iter().copied() {
            if seller == buyer || txn.entity_kind(seller) != Some(EntityKind::Agent) {
                continue;
            }
            if txn.controlled_commodity_quantity(seller, substitute) < desired_quantity {
                continue;
            }

            let acceptance = evaluate_for_participant(
                txn,
                &RecipeRegistry::new(),
                buyer,
                seller,
                place,
                [(offered_commodity, offered_quantity)],
                [(substitute, desired_quantity)],
            );
            if acceptance == TradeAcceptance::Accept {
                return Some(SubstituteTradeCandidate {
                    seller,
                    commodity: substitute,
                    quantity: desired_quantity,
                });
            }
        }
    }

    None
}

fn holdings_from_view(
    view: &dyn GoalBeliefView,
    actor: EntityId,
) -> BTreeMap<CommodityKind, u32> {
    CommodityKind::ALL
        .into_iter()
        .map(|kind| (kind, view.commodity_quantity(actor, kind).0))
        .collect()
}

fn alternative_supply_map(
    view: &dyn RuntimeBeliefView,
    actor: EntityId,
    excluded_counterparty: EntityId,
    place: EntityId,
) -> BTreeMap<CommodityKind, u32> {
    let mut by_kind = BTreeMap::new();
    for (_, commodity, quantity) in local_trade_alternatives(view, actor, excluded_counterparty, place) {
        *by_kind.entry(commodity).or_insert(0) += quantity.0;
    }
    by_kind
}

fn total_commodity_opportunity<V: RuntimeBeliefView + GoalBeliefView>(
    view: &V,
    actor: EntityId,
    holdings: &BTreeMap<CommodityKind, u32>,
    alternatives: &BTreeMap<CommodityKind, u32>,
) -> u32 {
    CommodityKind::ALL
        .into_iter()
        .map(|kind| {
            let breakdown = commodity_opportunity_score(actor, kind, view, holdings, alternatives);
            breakdown
                .direct_survival_score
                .saturating_add(breakdown.treatment_score)
                .saturating_add(breakdown.enterprise_score)
                .saturating_add(breakdown.indirect_recipe_score)
        })
        .sum()
}

fn buyer_reservation_price_for_view<V: RuntimeBeliefView + GoalBeliefView>(
    view: &V,
    actor: EntityId,
    excluded_counterparty: EntityId,
    place: EntityId,
    commodity: CommodityKind,
) -> Quantity {
    let current_coin = GoalBeliefView::commodity_quantity(view, actor, CommodityKind::Coin);
    if current_coin == Quantity(0) {
        return Quantity(0);
    }

    let baseline = buyer_reservation_price(
        GoalBeliefView::homeostatic_needs(view, actor).as_ref(),
        wounds_for(view, actor).as_ref(),
        commodity,
        current_coin,
        count_local_alternatives(view, actor, excluded_counterparty, place, commodity),
    );
    let mut holdings = holdings_from_view(view, actor);
    let alternatives = alternative_supply_map(view, actor, excluded_counterparty, place);
    let current = total_commodity_opportunity(view, actor, &holdings, &alternatives);
    *holdings.entry(commodity).or_insert(0) += 1;
    let improved = total_commodity_opportunity(view, actor, &holdings, &alternatives);
    Quantity(improved.saturating_sub(current).max(baseline.0)).min(current_coin)
}

fn seller_reservation_price_for_view<V: RuntimeBeliefView + GoalBeliefView>(
    view: &V,
    actor: EntityId,
    excluded_counterparty: EntityId,
    place: EntityId,
    offered_commodity: CommodityKind,
    requested_commodity: CommodityKind,
) -> Quantity {
    let mut holdings = holdings_from_view(view, actor);
    let current_stock = holdings.get(&requested_commodity).copied().unwrap_or(0);
    if current_stock == 0 {
        return Quantity(1);
    }
    let remembered_demand = DemandMemory {
        observations: GoalBeliefView::demand_memory(view, actor),
    };
    let baseline = seller_reservation_price(
        GoalBeliefView::homeostatic_needs(view, actor).as_ref(),
        requested_commodity,
        Quantity(current_stock),
        Some(&remembered_demand),
    );
    let alternatives = alternative_supply_map(view, actor, excluded_counterparty, place);
    let current = total_commodity_opportunity(view, actor, &holdings, &alternatives);
    holdings.insert(requested_commodity, current_stock.saturating_sub(1));
    let reduced = total_commodity_opportunity(view, actor, &holdings, &alternatives);
    Quantity(current.saturating_sub(reduced).max(baseline.0)).max(
        if offered_commodity == CommodityKind::Coin {
            Quantity(1)
        } else {
            Quantity(0)
        },
    )
}

fn execute_trade_transfers(
    txn: &mut WorldTxn<'_>,
    actor: EntityId,
    counterparty: EntityId,
    payload: &TradeActionPayload,
    place: EntityId,
) -> Result<(), ActionError> {
    let requested_commodity = sale_lot_commodity(txn, payload.sale_lot)?;
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
        requested_commodity,
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
        requested_commodity,
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
        remaining = remaining.checked_sub(available).ok_or_else(|| {
            ActionError::InternalError("trade lot accounting underflowed".to_string())
        })?;
    }

    if remaining != Quantity(0) {
        return Err(ActionError::AbortRequested(
            ActionAbortRequestReason::HolderLacksAccessibleCommodity {
                holder,
                commodity,
                quantity,
            },
        ));
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
        return Err(ActionError::AbortRequested(
            ActionAbortRequestReason::HolderLacksAccessibleCommodity {
                holder,
                commodity,
                quantity,
            },
        ));
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
    // The buyer is not the seller — remove any SaleListing and
    // StockAssignment from the transferred lot.
    if txn.get_component_sale_listing(lot_id).is_some() {
        let _ = txn.clear_component_sale_listing(lot_id);
    }
    if txn.get_component_stock_assignment(lot_id).is_some() {
        let _ = txn.clear_component_stock_assignment(lot_id);
    }
    txn.append_transfer_provenance(lot_id, quantity)
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
        if other == focal
            || other == counterparty
            || txn.entity_kind(other) != Some(EntityKind::Agent)
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

// ---------------------------------------------------------------------------
// staff_market action — seller-side market presence
// ---------------------------------------------------------------------------

fn validate_staff_market_payload_override(
    _def: &ActionDef,
    _actor: EntityId,
    _targets: &[EntityId],
    payload: &ActionPayload,
    _view: &dyn RuntimeBeliefView,
) -> bool {
    payload.as_staff_market().is_some()
}

pub fn register_staff_market_action(
    defs: &mut ActionDefRegistry,
    handlers: &mut ActionHandlerRegistry,
) -> ActionDefId {
    let handler = handlers.register(
        ActionHandler::new(
            start_staff_market,
            tick_staff_market,
            commit_staff_market,
            abort_staff_market,
        )
        .with_payload_override_validator(validate_staff_market_payload_override),
    );
    defs.register(staff_market_action_def(
        ActionDefId(defs.len() as u32),
        handler,
    ))
}

fn staff_market_action_def(id: ActionDefId, handler: ActionHandlerId) -> ActionDef {
    ActionDef {
        id,
        name: "staff_market".to_string(),
        domain: worldwake_core::ActionDomain::Trade,
        actor_constraints: vec![],
        targets: vec![],
        preconditions: vec![Precondition::ActorAlive],
        reservation_requirements: vec![],
        duration: DurationExpr::ActorMarketPresence,
        body_cost_per_tick: BodyCostPerTick::zero(),
        interruptibility: Interruptibility::FreelyInterruptible,
        commit_conditions: vec![Precondition::ActorAlive],
        visibility: VisibilitySpec::SamePlace,
        causal_event_tags: BTreeSet::from([EventTag::Trade, EventTag::WorldMutation]),
        payload: ActionPayload::None,
        handler,
    }
}

fn staff_market_payload<'a>(
    def: &ActionDef,
    instance: &'a ActionInstance,
) -> Result<&'a StaffMarketPayload, ActionError> {
    instance.payload.as_staff_market().ok_or_else(|| {
        ActionError::InternalError(format!(
            "action instance for def {} is missing staff_market payload",
            def.id
        ))
    })
}

/// Validate that actor is at the place of their bound home facility, has
/// `MerchandiseProfile` with the payload commodity in `sale_kinds`, and has
/// accessible local stock for that exact facility.
fn validate_staff_market_preconditions(
    txn: &WorldTxn<'_>,
    actor: EntityId,
    commodity: CommodityKind,
) -> Result<(EntityId, MerchandiseProfile), ActionError> {
    let place = txn.effective_place(actor).ok_or(ActionError::AbortRequested(
        ActionAbortRequestReason::ActorNotPlaced { actor },
    ))?;
    let profile = txn
        .get_component_merchandise_profile(actor)
        .cloned()
        .ok_or_else(|| {
            ActionError::InternalError(format!("actor {actor} lacks MerchandiseProfile"))
        })?;
    let home_facility = profile.home_facility.ok_or(ActionError::AbortRequested(
        ActionAbortRequestReason::ActorNotPlaced { actor },
    ))?;
    if txn.effective_place(home_facility) != Some(place) {
        return Err(ActionError::AbortRequested(
            ActionAbortRequestReason::ActorNotPlaced { actor },
        ));
    }
    txn.can_exercise_control(actor, home_facility)
        .map_err(|err| ActionError::PreconditionFailed(err.to_string()))?;
    if !profile.sale_kinds.contains(&commodity) {
        return Err(ActionError::InternalError(format!(
            "commodity {commodity:?} not in actor {actor} sale_kinds"
        )));
    }
    let has_local_stock = staff_market_has_sellable_stock(txn, actor, home_facility, commodity);
    if !has_local_stock {
        return Err(ActionError::AbortRequested(
            ActionAbortRequestReason::HolderLacksAccessibleCommodity {
                holder: actor,
                commodity,
                quantity: Quantity(1),
            },
        ));
    }
    Ok((place, profile))
}

fn staff_market_has_sellable_stock(
    txn: &WorldTxn<'_>,
    actor: EntityId,
    home_facility: EntityId,
    commodity: CommodityKind,
) -> bool {
    let Some(place) = txn.effective_place(actor) else {
        return false;
    };
    txn.possessions_of(actor).into_iter().any(|entity| {
        txn.get_component_item_lot(entity)
            .is_some_and(|lot| lot.commodity == commodity && lot.quantity > Quantity(0))
            && txn.effective_place(entity) == Some(place)
    }) || displayed_sale_lots_at_facility(txn, home_facility, commodity)
}

/// Check whether any lots with `SaleListing` and `StockAssignment::Displayed`
/// exist for the given facility and commodity.
fn displayed_sale_lots_at_facility(
    txn: &WorldTxn<'_>,
    facility: EntityId,
    commodity: CommodityKind,
) -> bool {
    let Some(place) = txn.effective_place(facility) else {
        return false;
    };
    txn.entities_effectively_at(place).into_iter().any(|entity| {
        txn.get_component_item_lot(entity)
            .is_some_and(|lot| lot.commodity == commodity && lot.quantity > Quantity(0))
            && txn.has_component_sale_listing(entity)
            && txn
                .get_component_stock_assignment(entity)
                .is_some_and(|a| {
                    a.kind == worldwake_core::StockAssignmentKind::Displayed
                        && a.facility == facility
                })
    })
}

#[allow(clippy::unnecessary_wraps)]
fn start_staff_market(
    def: &ActionDef,
    instance: &mut ActionInstance,
    _context: &worldwake_sim::ActionExecutionContext<'_>,
    _rng: &mut DeterministicRng,
    txn: &mut WorldTxn<'_>,
) -> Result<Option<ActionState>, ActionError> {
    let payload = staff_market_payload(def, instance)?;
    let commodity = payload.commodity;
    let (_place, _profile) = validate_staff_market_preconditions(txn, instance.actor, commodity)?;
    // Presence-only: SaleListing is managed by stage_stock_for_sale/unstage_stock.
    // This action represents the merchant being present at the market.
    Ok(Some(ActionState::Empty))
}

#[allow(clippy::unnecessary_wraps)]
fn tick_staff_market(
    _def: &ActionDef,
    _instance: &mut ActionInstance,
    _context: &worldwake_sim::ActionExecutionContext<'_>,
    _rng: &mut DeterministicRng,
    _txn: &mut WorldTxn<'_>,
) -> Result<ActionProgress, ActionError> {
    Ok(ActionProgress::Continue)
}

fn commit_staff_market(
    def: &ActionDef,
    instance: &ActionInstance,
    _context: &worldwake_sim::ActionExecutionContext<'_>,
    _event_log: &worldwake_core::EventLog,
    _rng: &mut DeterministicRng,
    txn: &mut WorldTxn<'_>,
) -> Result<CommitOutcome, ActionError> {
    let payload = staff_market_payload(def, instance)?;
    let commodity = payload.commodity;
    // Presence-only: SaleListing is managed by stage/unstage, not staff_market.
    // Record unproductive demand if displayed stock remains unsold.
    if let Some(profile) = txn.get_component_merchandise_profile(instance.actor) {
        if let Some(home_facility) = profile.home_facility {
            if let Some(place) = txn.effective_place(home_facility) {
                if staff_market_has_sellable_stock(txn, instance.actor, home_facility, commodity) {
                    record_unproductive_demand(txn, instance.actor, commodity, place);
                    record_sell_blocked_intent(txn, instance.actor, commodity, place);
                }
            }
        }
    }
    Ok(CommitOutcome::empty())
}

#[allow(clippy::unnecessary_wraps)]
fn abort_staff_market(
    _def: &ActionDef,
    _instance: &ActionInstance,
    _context: &worldwake_sim::ActionExecutionContext<'_>,
    _reason: &AbortReason,
    _event_log: &worldwake_core::EventLog,
    _rng: &mut DeterministicRng,
    _txn: &mut WorldTxn<'_>,
) -> Result<(), ActionError> {
    // Presence-only: SaleListing is managed by stage/unstage, not staff_market.
    Ok(())
}

/// Record a `WantedToSellButNoBuyer` demand observation on unproductive commit.
fn record_unproductive_demand(
    txn: &mut WorldTxn<'_>,
    actor: EntityId,
    commodity: CommodityKind,
    place: EntityId,
) {
    let current_tick = txn.tick();
    let mut memory = txn
        .get_component_demand_memory(actor)
        .cloned()
        .unwrap_or(DemandMemory {
            observations: Vec::new(),
        });
    memory.observations.push(DemandObservation {
        commodity,
        quantity: Quantity(1),
        place,
        tick: current_tick,
        counterparty: None,
        reason: DemandObservationReason::WantedToSellButNoBuyer,
    });
    let _ = txn.set_component_demand_memory(actor, memory);
}

/// Create a `BlockedIntent` for `SellCommodity { commodity }` after an unproductive
/// market-presence cycle.  The blocking period equals `market_presence_ticks` from
/// the actor's `TradeDispositionProfile`, so per-agent diversity is preserved.
fn record_sell_blocked_intent(
    txn: &mut WorldTxn<'_>,
    actor: EntityId,
    commodity: CommodityKind,
    place: EntityId,
) {
    let current_tick = txn.tick();
    let blocking_period = txn
        .get_component_trade_disposition_profile(actor)
        .map_or(30, |profile| profile.market_presence_ticks.get());
    let mut memory = txn
        .get_component_blocked_intent_memory(actor)
        .cloned()
        .unwrap_or_default();
    memory.record(BlockedIntent {
        blocker_key: BlockerKey {
            goal_key: GoalKey::from(GoalKind::SellCommodity { commodity }),
            place: Some(place),
            target: None,
            action_def: None,
        },
        blocking_fact: BlockingFact::NoBuyer,
        diagnostic_context: None,
        observed_tick: current_tick,
        expires_tick: Tick(current_tick.0 + u64::from(blocking_period)),
    });
    let _ = txn.set_component_blocked_intent_memory(actor, memory);
}

#[cfg(test)]
mod tests {
    use super::{
        buyer_reservation_price, count_local_alternatives, derive_opening_offer, generate_offer,
        register_trade_action, rejection_count_for, select_substitute_trade_candidate,
        seller_reservation_price, urgency_modulated_deadline, validate_trade_payload_override,
        SubstituteTradeCandidate,
    };
    use crate::trade_actions::local_alternatives;
    use std::collections::{BTreeMap, BTreeSet};
    use std::num::NonZeroU32;
    use worldwake_core::ActionDefId;
    use worldwake_core::{
        build_believed_entity_state, build_prototype_world, verify_live_lot_conservation,
        AgentBeliefStore, BlockingFact, CauseRef, CommodityKind, ControlSource, DemandMemory,
        DemandObservation, DemandObservationReason, EntityId, EventLog, EventTag, EventView,
        GoalKind, HomeostaticNeeds, LotOperation, MerchandiseProfile, PerceptionSource, Permille,
        Quantity, SaleListing, Seed, SubstitutePreferences, Tick, TradeCategory,
        TradeDispositionProfile, VisibilitySpec, WitnessData, World, WorldTxn,
    };
    use worldwake_sim::{
        get_affordances, start_action, tick_action, ActionAbortRequestReason, ActionDefRegistry,
        ActionError, ActionExecutionAuthority, ActionHandlerRegistry,
        ActionInstanceId, ActionPayload, ActionState, ActionStatus, Affordance, DeterministicRng,
        PayloadEntityRole, PerAgentBeliefView, TickOutcome, TradeActionPayload,
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

    fn test_rng() -> DeterministicRng {
        DeterministicRng::new(Seed([0x72; 32]))
    }

    fn food_substitutes(kinds: Vec<CommodityKind>) -> SubstitutePreferences {
        SubstitutePreferences {
            preferences: BTreeMap::from([(TradeCategory::Food, kinds)]),
        }
    }

    fn remembered_demand(kind: CommodityKind) -> DemandMemory {
        DemandMemory {
            observations: vec![DemandObservation {
                commodity: kind,
                quantity: Quantity(1),
                place: entity(99),
                tick: Tick(2),
                counterparty: Some(entity(88)),
                reason: DemandObservationReason::WantedToBuyButNoSeller,
            }],
        }
    }

    fn expensive_rejection(
        commodity: CommodityKind,
        counterparty: EntityId,
        tick: u64,
    ) -> DemandObservation {
        DemandObservation {
            commodity,
            quantity: Quantity(1),
            place: entity(99),
            tick: Tick(tick),
            counterparty: Some(counterparty),
            reason: DemandObservationReason::WantedToBuyButTooExpensive,
        }
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
    ) -> Vec<Affordance> {
        let beliefs = test_belief_store(world, actor);
        let view = PerAgentBeliefView::new(actor, world, &beliefs);
        get_affordances(&view, actor, defs, handlers)
    }

    #[test]
    fn buyer_reservation_price_returns_higher_value_for_higher_hunger() {
        let low = buyer_reservation_price(
            Some(&HomeostaticNeeds::new(pm(150), pm(0), pm(0), pm(0), pm(0))),
            None,
            CommodityKind::Apple,
            Quantity(10),
            0,
        );
        let high = buyer_reservation_price(
            Some(&HomeostaticNeeds::new(pm(800), pm(0), pm(0), pm(0), pm(0))),
            None,
            CommodityKind::Apple,
            Quantity(10),
            0,
        );

        assert!(high > low);
    }

    #[test]
    fn buyer_reservation_price_never_exceeds_current_coin() {
        let reservation = buyer_reservation_price(
            Some(&HomeostaticNeeds::new(pm(1000), pm(0), pm(0), pm(0), pm(0))),
            None,
            CommodityKind::Apple,
            Quantity(2),
            0,
        );

        assert!(reservation <= Quantity(2));
    }

    #[test]
    fn buyer_reservation_price_returns_lower_values_with_more_alternatives() {
        let scarce = buyer_reservation_price(
            Some(&HomeostaticNeeds::new(pm(900), pm(0), pm(0), pm(0), pm(0))),
            None,
            CommodityKind::Apple,
            Quantity(10),
            0,
        );
        let common = buyer_reservation_price(
            Some(&HomeostaticNeeds::new(pm(900), pm(0), pm(0), pm(0), pm(0))),
            None,
            CommodityKind::Apple,
            Quantity(10),
            3,
        );

        assert!(scarce > common);
    }

    #[test]
    fn buyer_reservation_price_returns_minimum_when_no_needs_or_wounds() {
        let reservation = buyer_reservation_price(
            Some(&HomeostaticNeeds::new_sated()),
            None,
            CommodityKind::Apple,
            Quantity(10),
            0,
        );

        assert_eq!(reservation, Quantity(1));
    }

    #[test]
    fn seller_reservation_price_returns_higher_value_with_fewer_stock_units() {
        let demand = DemandMemory {
            observations: vec![
                DemandObservation {
                    commodity: CommodityKind::Apple,
                    quantity: Quantity(1),
                    place: entity(90),
                    tick: Tick(1),
                    counterparty: Some(entity(91)),
                    reason: DemandObservationReason::WantedToBuyButNoSeller,
                },
                DemandObservation {
                    commodity: CommodityKind::Apple,
                    quantity: Quantity(1),
                    place: entity(90),
                    tick: Tick(2),
                    counterparty: Some(entity(92)),
                    reason: DemandObservationReason::WantedToBuyButSellerOutOfStock,
                },
            ],
        };
        let scarce = seller_reservation_price(
            Some(&HomeostaticNeeds::new(pm(400), pm(0), pm(0), pm(0), pm(0))),
            CommodityKind::Apple,
            Quantity(1),
            Some(&demand),
        );
        let abundant = seller_reservation_price(
            Some(&HomeostaticNeeds::new(pm(400), pm(0), pm(0), pm(0), pm(0))),
            CommodityKind::Apple,
            Quantity(4),
            Some(&demand),
        );

        assert!(scarce > abundant);
    }

    #[test]
    fn seller_reservation_price_returns_higher_value_with_more_demand_observations() {
        let none = seller_reservation_price(
            None,
            CommodityKind::Apple,
            Quantity(2),
            None,
        );
        let remembered = seller_reservation_price(
            None,
            CommodityKind::Apple,
            Quantity(2),
            Some(&DemandMemory {
                observations: vec![
                    DemandObservation {
                        commodity: CommodityKind::Apple,
                        quantity: Quantity(1),
                        place: entity(90),
                        tick: Tick(1),
                        counterparty: Some(entity(91)),
                        reason: DemandObservationReason::WantedToBuyButNoSeller,
                    },
                    DemandObservation {
                        commodity: CommodityKind::Apple,
                        quantity: Quantity(2),
                        place: entity(90),
                        tick: Tick(2),
                        counterparty: Some(entity(92)),
                        reason: DemandObservationReason::WantedToBuyButTooExpensive,
                    },
                ],
            }),
        );

        assert!(remembered > none);
    }

    #[test]
    fn seller_reservation_price_returns_at_least_one() {
        let reservation = seller_reservation_price(None, CommodityKind::Apple, Quantity(0), None);

        assert!(reservation >= Quantity(1));
    }

    #[test]
    fn count_local_alternatives_counts_other_sellers_for_matching_commodity() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let place = world.topology().place_ids().next().unwrap();
        let (actor, excluded, seller_a, seller_b) = {
            let mut txn = new_txn(&mut world, 1);
            let actor = txn.create_agent("Actor", ControlSource::Ai).unwrap();
            let excluded = txn.create_agent("Excluded", ControlSource::Ai).unwrap();
            let seller_a = txn.create_agent("Seller A", ControlSource::Ai).unwrap();
            let seller_b = txn.create_agent("Seller B", ControlSource::Ai).unwrap();
            let actor_apple = txn.create_item_lot(CommodityKind::Apple, Quantity(1)).unwrap();
            let excluded_apple = txn.create_item_lot(CommodityKind::Apple, Quantity(1)).unwrap();
            let orchard_stock = txn.create_item_lot(CommodityKind::Apple, Quantity(2)).unwrap();
            let grocer_stock = txn.create_item_lot(CommodityKind::Apple, Quantity(3)).unwrap();
            let baker_stock = txn.create_item_lot(CommodityKind::Bread, Quantity(2)).unwrap();

            for entity in [
                actor,
                excluded,
                seller_a,
                seller_b,
                actor_apple,
                excluded_apple,
                orchard_stock,
                grocer_stock,
                baker_stock,
            ] {
                txn.set_ground_location(entity, place).unwrap();
            }

            for (holder, lot) in [
                (actor, actor_apple),
                (excluded, excluded_apple),
                (seller_a, orchard_stock),
                (seller_b, grocer_stock),
                (seller_b, baker_stock),
            ] {
                txn.set_owner(lot, holder).unwrap();
                txn.set_possessor(lot, holder).unwrap();
            }

            txn.set_component_merchandise_profile(
                seller_a,
                MerchandiseProfile {
                    sale_kinds: BTreeSet::from([CommodityKind::Apple]),
                    home_facility: None,
                },
            )
            .unwrap();
            txn.set_component_merchandise_profile(
                seller_b,
                MerchandiseProfile {
                    sale_kinds: BTreeSet::from([CommodityKind::Apple, CommodityKind::Bread]),
                    home_facility: None,
                },
            )
            .unwrap();
            commit_txn(txn);
            (actor, excluded, seller_a, seller_b)
        };

        let beliefs = test_belief_store(&world, actor);
        let view = PerAgentBeliefView::new(actor, &world, &beliefs);

        let apple_count =
            count_local_alternatives(&view, actor, excluded, place, CommodityKind::Apple);
        let bread_count =
            count_local_alternatives(&view, actor, excluded, place, CommodityKind::Bread);

        assert_eq!(apple_count, 2);
        assert_eq!(bread_count, 1);
        assert_ne!(seller_a, seller_b);
    }

    #[test]
    fn generate_offer_boulware_concedes_slowly_then_rapidly() {
        let early = generate_offer(
            worldwake_core::TradeRole::Buyer,
            Quantity(10),
            Quantity(2),
            2,
            10,
            pm(100),
        );
        let mid = generate_offer(
            worldwake_core::TradeRole::Buyer,
            Quantity(10),
            Quantity(2),
            5,
            10,
            pm(100),
        );
        let late = generate_offer(
            worldwake_core::TradeRole::Buyer,
            Quantity(10),
            Quantity(2),
            9,
            10,
            pm(100),
        );

        assert!(mid.0 - early.0 <= 2);
        assert!(late.0 - mid.0 >= 3);
    }

    #[test]
    fn generate_offer_conceder_concedes_rapidly_then_slowly() {
        let early = generate_offer(
            worldwake_core::TradeRole::Buyer,
            Quantity(10),
            Quantity(2),
            2,
            10,
            pm(900),
        );
        let mid = generate_offer(
            worldwake_core::TradeRole::Buyer,
            Quantity(10),
            Quantity(2),
            5,
            10,
            pm(900),
        );
        let late = generate_offer(
            worldwake_core::TradeRole::Buyer,
            Quantity(10),
            Quantity(2),
            9,
            10,
            pm(900),
        );

        assert!(mid.0 - early.0 >= 2);
        assert!(late.0 - mid.0 <= 2);
    }

    #[test]
    fn generate_offer_linear_concedes_uniformly() {
        let early = generate_offer(
            worldwake_core::TradeRole::Buyer,
            Quantity(10),
            Quantity(2),
            2,
            10,
            pm(500),
        );
        let mid = generate_offer(
            worldwake_core::TradeRole::Buyer,
            Quantity(10),
            Quantity(2),
            5,
            10,
            pm(500),
        );
        let late = generate_offer(
            worldwake_core::TradeRole::Buyer,
            Quantity(10),
            Quantity(2),
            8,
            10,
            pm(500),
        );

        assert_eq!(early, Quantity(3));
        assert_eq!(mid, Quantity(6));
        assert_eq!(late, Quantity(8));
    }

    #[test]
    fn generate_offer_preserves_monotonic_concession_for_buyer_and_seller() {
        let mut previous_buyer = Quantity(1);
        let mut previous_seller = Quantity(10);
        for round in 0..=10 {
            let buyer = generate_offer(
                worldwake_core::TradeRole::Buyer,
                Quantity(10),
                Quantity(1),
                round,
                10,
                pm(700),
            );
            let seller = generate_offer(
                worldwake_core::TradeRole::Seller,
                Quantity(4),
                Quantity(10),
                round,
                10,
                pm(300),
            );
            assert!(buyer >= previous_buyer);
            assert!(seller <= previous_seller);
            assert!((1..=10).contains(&buyer.0));
            assert!((4..=10).contains(&seller.0));
            previous_buyer = buyer;
            previous_seller = seller;
        }
    }

    #[test]
    fn derive_opening_offer_without_rejections_returns_bias_derived_base() {
        let buyer = derive_opening_offer(
            worldwake_core::TradeRole::Buyer,
            Quantity(10),
            pm(500),
            pm(200),
            0,
        );
        let seller = derive_opening_offer(
            worldwake_core::TradeRole::Seller,
            Quantity(10),
            pm(500),
            pm(200),
            0,
        );

        assert_eq!(buyer, Quantity(6));
        assert_eq!(seller, Quantity(14));
    }

    #[test]
    fn derive_opening_offer_shifts_toward_counterparty_after_rejections() {
        let base = derive_opening_offer(
            worldwake_core::TradeRole::Buyer,
            Quantity(10),
            pm(700),
            pm(200),
            0,
        );
        let shifted = derive_opening_offer(
            worldwake_core::TradeRole::Buyer,
            Quantity(10),
            pm(700),
            pm(200),
            3,
        );

        assert!(shifted > base);
    }

    #[test]
    fn derive_opening_offer_respects_rejection_escalation_rate() {
        let gentle = derive_opening_offer(
            worldwake_core::TradeRole::Buyer,
            Quantity(10),
            pm(700),
            pm(100),
            3,
        );
        let assertive = derive_opening_offer(
            worldwake_core::TradeRole::Buyer,
            Quantity(10),
            pm(700),
            pm(300),
            3,
        );

        assert!(assertive > gentle);
    }

    #[test]
    fn urgency_modulated_deadline_preserves_base_patience_at_zero_urgency() {
        let deadline = urgency_modulated_deadline(
            nz(8),
            Some(&HomeostaticNeeds::new_sated()),
            CommodityKind::Apple,
        );

        assert_eq!(deadline, 8);
    }

    #[test]
    fn urgency_modulated_deadline_never_drops_below_one() {
        let deadline = urgency_modulated_deadline(
            nz(8),
            Some(&HomeostaticNeeds::new(pm(1000), pm(0), pm(0), pm(0), pm(0))),
            CommodityKind::Apple,
        );

        assert_eq!(deadline, 1);
    }

    struct TradeHarness {
        world: World,
        defs: ActionDefRegistry,
        handlers: ActionHandlerRegistry,
        log: EventLog,
        rng: DeterministicRng,
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
            offered_commodity: CommodityKind,
            offered_quantity: Quantity,
            requested_commodity: CommodityKind,
            requested_quantity: Quantity,
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
                    .create_item_lot(offered_commodity, offered_quantity)
                    .unwrap();
                let counterparty_offer = txn
                    .create_item_lot(requested_commodity, requested_quantity)
                    .unwrap();
                commit_txn(txn);
                (actor, counterparty, actor_offer, counterparty_offer)
            };
            {
                let mut txn = new_txn(&mut world, 2);
                for entity in [actor, counterparty, actor_offer, counterparty_offer] {
                    txn.set_ground_location(entity, place).unwrap();
                }
                txn.set_possessor(actor_offer, actor).unwrap();
                txn.set_owner(actor_offer, actor).unwrap();
                // Create facility for counterparty and stage the sale lot.
                let facility = {
                {
                    use worldwake_core::{LoadUnits, StockAssignment, StockAssignmentKind};
                    let (facility, _stock, display) = txn
                        .create_merchant_facility(place, counterparty, LoadUnits(200), Some(LoadUnits(100)))
                        .unwrap();
                    let display = display.unwrap();
                    txn.put_into_container(counterparty_offer, display).unwrap();
                    txn.set_component_stock_assignment(
                        counterparty_offer,
                        StockAssignment {
                            facility,
                            kind: StockAssignmentKind::Displayed,
                        },
                    )
                    .unwrap();
                    txn.set_component_sale_listing(
                        counterparty_offer,
                        SaleListing {
                            listed_at: Tick(2),
                        },
                    )
                    .unwrap();
                    facility
                }
                };
                txn.set_component_trade_disposition_profile(
                    actor,
                    TradeDispositionProfile {
                        negotiation_round_ticks: nz(actor_ticks),
                        initial_offer_bias: pm(500),
                        concession_rate: pm(200),
                        rejection_escalation_rate: pm(200),
                        demand_memory_retention_ticks: 10,
                        market_presence_ticks: nz(30),
                    },
                )
                .unwrap();
                txn.set_component_trade_disposition_profile(
                    counterparty,
                    TradeDispositionProfile {
                        negotiation_round_ticks: nz(1),
                        initial_offer_bias: pm(500),
                        concession_rate: pm(200),
                        rejection_escalation_rate: pm(200),
                        demand_memory_retention_ticks: 10,
                        market_presence_ticks: nz(30),
                    },
                )
                .unwrap();
                txn.set_component_homeostatic_needs(actor, actor_needs)
                    .unwrap();
                txn.set_component_homeostatic_needs(counterparty, HomeostaticNeeds::new_sated())
                    .unwrap();
                txn.set_component_merchandise_profile(
                    counterparty,
                    MerchandiseProfile {
                        sale_kinds: [requested_commodity].into_iter().collect(),
                        home_facility: Some(facility),
                    },
                )
                .unwrap();
                commit_txn(txn);
            }

            let payload = TradeActionPayload {
                counterparty,
                sale_lot: counterparty_offer,
                offered_commodity,
                offered_quantity,
                requested_quantity,
            };

            let mut defs = ActionDefRegistry::new();
            let mut handlers = ActionHandlerRegistry::new();
            let def_id = register_trade_action(&mut defs, &mut handlers);
            Self {
                world,
                defs,
                handlers,
                log: EventLog::new(),
                rng: test_rng(),
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
        ) -> (
            ActionInstanceId,
            BTreeMap<ActionInstanceId, worldwake_sim::ActionInstance>,
        ) {
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
                    rng: &mut self.rng,
                },
                &mut self.next_instance_id,
                worldwake_sim::ActionExecutionContext::without_recipes(CauseRef::Bootstrap, Tick(3)),
            )
            .unwrap();
            (instance_id, active)
        }

        fn start_result(&mut self) -> Result<ActionInstanceId, ActionError> {
            let affordance = Affordance {
                def_id: self.def_id,
                actor: self.actor,
                bound_targets: vec![self.counterparty],
                payload_override: Some(ActionPayload::Trade(self.payload.clone())),
                explanation: None,
            };
            let mut active = BTreeMap::new();
            start_action(
                &affordance,
                &self.defs,
                &self.handlers,
                ActionExecutionAuthority {
                    active_actions: &mut active,
                    world: &mut self.world,
                    event_log: &mut self.log,
                    rng: &mut self.rng,
                },
                &mut self.next_instance_id,
                worldwake_sim::ActionExecutionContext::without_recipes(CauseRef::Bootstrap, Tick(3)),
            )
        }

        fn remove_counterparty_offer(&mut self) {
            let mut txn = new_txn(&mut self.world, 4);
            txn.clear_possessor(self.counterparty_offer).unwrap();
            txn.clear_owner(self.counterparty_offer).unwrap();
            txn.archive_entity(self.counterparty_offer).unwrap();
            commit_txn(txn);
        }

        fn set_counterparty_demand_memory(&mut self, observations: Vec<DemandObservation>) {
            let mut txn = new_txn(&mut self.world, 3);
            txn.set_component_demand_memory(self.counterparty, DemandMemory { observations })
                .unwrap();
            commit_txn(txn);
        }

        fn set_actor_trade_profile(&mut self, profile: TradeDispositionProfile) {
            let mut txn = new_txn(&mut self.world, 3);
            txn.set_component_trade_disposition_profile(self.actor, profile)
                .unwrap();
            commit_txn(txn);
        }

        fn set_counterparty_trade_profile(&mut self, profile: TradeDispositionProfile) {
            let mut txn = new_txn(&mut self.world, 3);
            txn.set_component_trade_disposition_profile(self.counterparty, profile)
                .unwrap();
            commit_txn(txn);
        }

        fn tick(
            &mut self,
            instance_id: ActionInstanceId,
            active: &mut BTreeMap<ActionInstanceId, worldwake_sim::ActionInstance>,
            tick: u64,
        ) -> Result<TickOutcome, ActionError> {
            tick_action(
                instance_id,
                &self.defs,
                &self.handlers,
                ActionExecutionAuthority {
                    active_actions: active,
                    world: &mut self.world,
                    event_log: &mut self.log,
                    rng: &mut self.rng,
                },
                worldwake_sim::ActionExecutionContext::without_recipes(CauseRef::Bootstrap, Tick(tick)),
            )
        }
    }

    #[test]
    fn trade_action_duration_resolves_from_actor_profile() {
        let mut harness = TradeHarness::new(
            CommodityKind::Coin,
            Quantity(1),
            CommodityKind::Bread,
            Quantity(1),
            1,
            HomeostaticNeeds::new(pm(900), pm(0), pm(0), pm(0), pm(0)),
        );
        let (_instance_id, active) = harness.start_with_active();
        let instance = active.values().next().unwrap();

        assert_eq!(
            instance.remaining_duration,
            worldwake_sim::ActionDuration::new(1)
        );
        assert_eq!(instance.status, ActionStatus::Active);
    }

    #[test]
    fn trade_affordance_enumerates_concrete_bundle_payloads_from_handler() {
        let harness = TradeHarness::new(
            CommodityKind::Coin,
            Quantity(10),
            CommodityKind::Bread,
            Quantity(1),
            1,
            HomeostaticNeeds::new(pm(900), pm(0), pm(0), pm(0), pm(0)),
        );

        let affordances = affordances_for(
            &harness.world,
            harness.actor,
            &harness.defs,
            &harness.handlers,
        );

        assert!(affordances.iter().any(|affordance| {
            affordance.def_id == harness.def_id
                && affordance.bound_targets == vec![harness.counterparty]
                && affordance.payload_override.as_ref().and_then(ActionPayload::as_trade).is_some_and(
                    |payload| {
                        payload.counterparty == harness.counterparty
                            && payload.sale_lot == harness.counterparty_offer
                            && payload.offered_commodity == CommodityKind::Coin
                            && payload.offered_quantity > Quantity(1)
                            && payload.requested_quantity == Quantity(1)
                    },
                )
        }));
    }

    #[test]
    fn trade_affordance_returns_empty_when_buyer_has_no_coins() {
        let mut harness = TradeHarness::new(
            CommodityKind::Coin,
            Quantity(1),
            CommodityKind::Bread,
            Quantity(1),
            1,
            HomeostaticNeeds::new(pm(900), pm(0), pm(0), pm(0), pm(0)),
        );
        {
            let mut txn = new_txn(&mut harness.world, 3);
            txn.clear_possessor(harness.actor_offer).unwrap();
            txn.clear_owner(harness.actor_offer).unwrap();
            txn.archive_entity(harness.actor_offer).unwrap();
            commit_txn(txn);
        }

        let affordances = affordances_for(
            &harness.world,
            harness.actor,
            &harness.defs,
            &harness.handlers,
        );

        assert!(
            !affordances
                .iter()
                .any(|affordance| affordance.def_id == harness.def_id && affordance.bound_targets == vec![harness.counterparty])
        );
    }

    #[test]
    fn trade_affordance_is_emitted_even_when_fixed_one_coin_bundle_would_be_rejected() {
        let mut harness = TradeHarness::new(
            CommodityKind::Coin,
            Quantity(8),
            CommodityKind::Bread,
            Quantity(1),
            1,
            HomeostaticNeeds::new(pm(900), pm(0), pm(0), pm(0), pm(0)),
        );
        {
            let mut txn = new_txn(&mut harness.world, 3);
            txn.set_component_demand_memory(
                harness.counterparty,
                DemandMemory {
                    observations: vec![
                        expensive_rejection(CommodityKind::Bread, harness.actor, 1),
                        expensive_rejection(CommodityKind::Bread, harness.actor, 2),
                    ],
                },
            )
            .unwrap();
            commit_txn(txn);
        }

        let affordances = affordances_for(
            &harness.world,
            harness.actor,
            &harness.defs,
            &harness.handlers,
        );

        assert!(affordances.iter().any(|affordance| {
            affordance.def_id == harness.def_id
                && affordance.bound_targets == vec![harness.counterparty]
                && affordance.payload_override.as_ref().and_then(ActionPayload::as_trade).is_some()
        }));
    }

    #[test]
    fn rejection_count_for_filters_by_reason_counterparty_and_commodity() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let (actor, counterparty) = {
            let mut txn = new_txn(&mut world, 1);
            let actor = txn.create_agent("Actor", ControlSource::Ai).unwrap();
            let counterparty = txn.create_agent("Seller", ControlSource::Ai).unwrap();
            let other = txn.create_agent("Other", ControlSource::Ai).unwrap();
            txn.set_component_demand_memory(
                actor,
                DemandMemory {
                    observations: vec![
                        expensive_rejection(CommodityKind::Bread, counterparty, 1),
                        expensive_rejection(CommodityKind::Bread, counterparty, 2),
                        expensive_rejection(CommodityKind::Apple, counterparty, 3),
                        expensive_rejection(CommodityKind::Bread, other, 4),
                        DemandObservation {
                            commodity: CommodityKind::Bread,
                            quantity: Quantity(1),
                            place: entity(99),
                            tick: Tick(5),
                            counterparty: Some(counterparty),
                            reason: DemandObservationReason::WantedToBuyButNoSeller,
                        },
                    ],
                },
            )
            .unwrap();
            commit_txn(txn);
            (actor, counterparty)
        };

        let beliefs = test_belief_store(&world, actor);
        let view = PerAgentBeliefView::new(actor, &world, &beliefs);

        assert_eq!(
            rejection_count_for(&view, actor, counterparty, CommodityKind::Bread),
            2
        );
    }

    #[test]
    fn trade_payload_override_validator_accepts_valid_variable_price() {
        let harness = TradeHarness::new(
            CommodityKind::Coin,
            Quantity(5),
            CommodityKind::Bread,
            Quantity(1),
            1,
            HomeostaticNeeds::new(pm(900), pm(0), pm(0), pm(0), pm(0)),
        );
        let beliefs = test_belief_store(&harness.world, harness.actor);
        let view = PerAgentBeliefView::new(harness.actor, &harness.world, &beliefs);
        let def = harness.defs.get(harness.def_id).unwrap();
        let payload = ActionPayload::Trade(TradeActionPayload {
            counterparty: harness.counterparty,
            sale_lot: harness.counterparty_offer,
            offered_commodity: CommodityKind::Coin,
            offered_quantity: Quantity(3),
            requested_quantity: Quantity(1),
        });

        assert!(validate_trade_payload_override(
            def,
            harness.actor,
            &[harness.counterparty],
            &payload,
            &view,
        ));
    }

    #[test]
    fn trade_payload_override_validator_rejects_zero_or_excessive_offers() {
        let harness = TradeHarness::new(
            CommodityKind::Coin,
            Quantity(2),
            CommodityKind::Bread,
            Quantity(1),
            1,
            HomeostaticNeeds::new(pm(900), pm(0), pm(0), pm(0), pm(0)),
        );
        let beliefs = test_belief_store(&harness.world, harness.actor);
        let view = PerAgentBeliefView::new(harness.actor, &harness.world, &beliefs);
        let def = harness.defs.get(harness.def_id).unwrap();
        let zero = ActionPayload::Trade(TradeActionPayload {
            counterparty: harness.counterparty,
            sale_lot: harness.counterparty_offer,
            offered_commodity: CommodityKind::Coin,
            offered_quantity: Quantity(0),
            requested_quantity: Quantity(1),
        });
        let excessive = ActionPayload::Trade(TradeActionPayload {
            counterparty: harness.counterparty,
            sale_lot: harness.counterparty_offer,
            offered_commodity: CommodityKind::Coin,
            offered_quantity: Quantity(3),
            requested_quantity: Quantity(1),
        });

        assert!(!validate_trade_payload_override(
            def,
            harness.actor,
            &[harness.counterparty],
            &zero,
            &view,
        ));
        assert!(!validate_trade_payload_override(
            def,
            harness.actor,
            &[harness.counterparty],
            &excessive,
            &view,
        ));
    }

    #[test]
    fn successful_trade_transfers_goods_and_coin_with_trade_tags_and_provenance() {
        let mut harness = TradeHarness::new(
            CommodityKind::Coin,
            Quantity(1),
            CommodityKind::Bread,
            Quantity(1),
            1,
            HomeostaticNeeds::new(pm(900), pm(0), pm(0), pm(0), pm(0)),
        );
        let (instance_id, mut active) = harness.start_with_active();

        let outcome = tick_action(
            instance_id,
            &harness.defs,
            &harness.handlers,
            ActionExecutionAuthority {
                active_actions: &mut active,
                world: &mut harness.world,
                event_log: &mut harness.log,
                rng: &mut harness.rng,
            },
            worldwake_sim::ActionExecutionContext::without_recipes(CauseRef::Bootstrap, Tick(4)),
        )
        .unwrap();

        assert!(matches!(outcome, TickOutcome::Committed { .. }));
        assert_eq!(
            harness.world.possessor_of(harness.actor_offer),
            Some(harness.counterparty)
        );
        assert_eq!(
            harness.world.owner_of(harness.actor_offer),
            Some(harness.counterparty)
        );
        assert_eq!(
            harness.world.possessor_of(harness.counterparty_offer),
            Some(harness.actor)
        );
        assert_eq!(
            harness.world.owner_of(harness.counterparty_offer),
            Some(harness.actor)
        );

        let trade_events = harness.log.events_by_tag(EventTag::Trade);
        assert_eq!(trade_events.len(), 1);
        let record = harness.log.get(trade_events[0]).unwrap();
        assert!(record.tags().contains(&EventTag::ActionCommitted));
        assert!(record.tags().contains(&EventTag::Transfer));
        assert!(record.tags().contains(&EventTag::Trade));

        let transferred_entry = harness
            .world
            .get_component_item_lot(harness.counterparty_offer)
            .unwrap()
            .provenance
            .last()
            .unwrap();
        assert_eq!(transferred_entry.operation, LotOperation::Transferred);
        assert_eq!(transferred_entry.amount, Quantity(1));
        assert_eq!(transferred_entry.event_id, Some(trade_events[0]));

        verify_live_lot_conservation(&harness.world, CommodityKind::Coin, 1).unwrap();
        verify_live_lot_conservation(&harness.world, CommodityKind::Bread, 1).unwrap();
    }

    #[test]
    fn negotiation_converges_and_commits_at_agreed_price() {
        let mut harness = TradeHarness::new(
            CommodityKind::Coin,
            Quantity(5),
            CommodityKind::Bread,
            Quantity(1),
            20,
            HomeostaticNeeds::new(pm(900), pm(0), pm(0), pm(0), pm(0)),
        );
        harness.payload.offered_quantity = Quantity(1);
        harness.set_actor_trade_profile(TradeDispositionProfile {
            negotiation_round_ticks: nz(20),
            initial_offer_bias: pm(500),
            concession_rate: pm(700),
            rejection_escalation_rate: pm(200),
            demand_memory_retention_ticks: 10,
            market_presence_ticks: nz(30),
        });
        harness.set_counterparty_trade_profile(TradeDispositionProfile {
            negotiation_round_ticks: nz(5),
            initial_offer_bias: pm(500),
            concession_rate: pm(200),
            rejection_escalation_rate: pm(200),
            demand_memory_retention_ticks: 10,
            market_presence_ticks: nz(30),
        });
        harness.set_counterparty_demand_memory(vec![
            expensive_rejection(CommodityKind::Bread, harness.actor, 1),
            expensive_rejection(CommodityKind::Bread, harness.actor, 2),
        ]);

        let (instance_id, mut active) = harness.start_with_active();
        let first = harness.tick(instance_id, &mut active, 4).unwrap();
        assert!(matches!(first, TickOutcome::Continuing));
        let second = harness.tick(instance_id, &mut active, 5).unwrap();

        assert!(matches!(second, TickOutcome::Committed { .. }));
        assert_eq!(
            harness.world.controlled_commodity_quantity(harness.counterparty, CommodityKind::Coin),
            Quantity(4)
        );
        assert_eq!(
            harness.world.controlled_commodity_quantity(harness.actor, CommodityKind::Coin),
            Quantity(1)
        );
        let buyer_memory = harness.world.get_component_demand_memory(harness.actor).unwrap();
        let seller_memory = harness
            .world
            .get_component_demand_memory(harness.counterparty)
            .unwrap();
        assert!(buyer_memory
            .observations
            .iter()
            .any(|obs| obs.reason == DemandObservationReason::TradeAgreed && obs.quantity == Quantity(4)));
        assert!(seller_memory
            .observations
            .iter()
            .any(|obs| obs.reason == DemandObservationReason::TradeAgreed && obs.quantity == Quantity(4)));
    }

    #[test]
    fn negotiation_walkaway_records_failed_trade_observations() {
        let mut harness = TradeHarness::new(
            CommodityKind::Coin,
            Quantity(1),
            CommodityKind::Bread,
            Quantity(1),
            3,
            HomeostaticNeeds::new_sated(),
        );
        harness.set_counterparty_trade_profile(TradeDispositionProfile {
            negotiation_round_ticks: nz(1),
            initial_offer_bias: pm(500),
            concession_rate: pm(200),
            rejection_escalation_rate: pm(200),
            demand_memory_retention_ticks: 10,
            market_presence_ticks: nz(30),
        });
        harness.set_counterparty_demand_memory(vec![
            expensive_rejection(CommodityKind::Bread, harness.actor, 1),
            expensive_rejection(CommodityKind::Bread, harness.actor, 2),
            expensive_rejection(CommodityKind::Bread, harness.actor, 3),
        ]);

        let (instance_id, mut active) = harness.start_with_active();
        let first = harness.tick(instance_id, &mut active, 4).unwrap();
        assert!(matches!(first, TickOutcome::Continuing));
        let second = harness.tick(instance_id, &mut active, 5).unwrap();
        let third = harness.tick(instance_id, &mut active, 6).unwrap();

        assert!(matches!(second, TickOutcome::Continuing));
        assert!(matches!(third, TickOutcome::Aborted { .. }));
        let buyer_memory = harness.world.get_component_demand_memory(harness.actor).unwrap();
        let seller_memory = harness
            .world
            .get_component_demand_memory(harness.counterparty)
            .unwrap();
        assert!(buyer_memory.observations.iter().any(|obs| {
            obs.reason == DemandObservationReason::WantedToBuyButTooExpensive
                && obs.counterparty == Some(harness.counterparty)
        }));
        assert!(seller_memory.observations.iter().any(|obs| {
            obs.reason == DemandObservationReason::WantedToSellButNoBuyer
                && obs.counterparty == Some(harness.actor)
        }));
    }

    #[test]
    fn negotiation_tick_advances_round_and_preserves_monotonic_concession() {
        let mut harness = TradeHarness::new(
            CommodityKind::Coin,
            Quantity(8),
            CommodityKind::Bread,
            Quantity(1),
            100,
            HomeostaticNeeds::new(pm(900), pm(0), pm(0), pm(0), pm(0)),
        );
        harness.payload.offered_quantity = Quantity(1);
        harness.set_actor_trade_profile(TradeDispositionProfile {
            negotiation_round_ticks: nz(100),
            initial_offer_bias: pm(0),
            concession_rate: pm(300),
            rejection_escalation_rate: pm(200),
            demand_memory_retention_ticks: 10,
            market_presence_ticks: nz(30),
        });
        harness.set_counterparty_trade_profile(TradeDispositionProfile {
            negotiation_round_ticks: nz(6),
            initial_offer_bias: pm(1000),
            concession_rate: pm(100),
            rejection_escalation_rate: pm(200),
            demand_memory_retention_ticks: 10,
            market_presence_ticks: nz(30),
        });
        harness.set_counterparty_demand_memory(vec![
            expensive_rejection(CommodityKind::Bread, harness.actor, 1),
            expensive_rejection(CommodityKind::Bread, harness.actor, 2),
            expensive_rejection(CommodityKind::Bread, harness.actor, 3),
            expensive_rejection(CommodityKind::Bread, harness.actor, 4),
            expensive_rejection(CommodityKind::Bread, harness.actor, 5),
        ]);

        let (instance_id, mut active) = harness.start_with_active();
        let outcome_one = harness.tick(instance_id, &mut active, 4).unwrap();
        assert!(matches!(outcome_one, TickOutcome::Continuing));
        let state_one = active.get(&instance_id).unwrap().local_state;
        let ActionState::Trade {
            round: round_one,
            initiator_last_offer: initiator_one,
            responder_last_offer: responder_one,
            ..
        } = state_one.unwrap()
        else {
            panic!("trade state should be present after first negotiation tick");
        };
        assert_eq!(round_one, 1);

        let outcome_two = harness.tick(instance_id, &mut active, 5).unwrap();
        assert!(matches!(outcome_two, TickOutcome::Continuing));
        let state_two = active.get(&instance_id).unwrap().local_state;
        let ActionState::Trade {
            round: round_two,
            initiator_last_offer: initiator_two,
            responder_last_offer: responder_two,
            ..
        } = state_two.unwrap()
        else {
            panic!("trade state should be present after second negotiation tick");
        };
        assert_eq!(round_two, 2);
        assert!(initiator_two.unwrap() >= initiator_one.unwrap());
        assert!(responder_two.unwrap() <= responder_one.unwrap());
    }

    #[test]
    fn trade_start_rejects_when_counterparty_lacks_requested_commodity() {
        let mut harness = TradeHarness::new(
            CommodityKind::Coin,
            Quantity(1),
            CommodityKind::Bread,
            Quantity(1),
            1,
            HomeostaticNeeds::new(pm(900), pm(0), pm(0), pm(0), pm(0)),
        );
        harness.remove_counterparty_offer();

        let err = harness
            .start_result()
            .expect_err("trade start should fail once the requested stock is already gone");

        assert_eq!(
            err,
            ActionError::AbortRequested(ActionAbortRequestReason::PayloadEntityMismatch {
                role: PayloadEntityRole::SaleLot,
                expected: harness.counterparty_offer,
                actual: harness.counterparty_offer,
            })
        );
        assert!(
            harness
                .log
                .events_by_tag(EventTag::ActionCommitted)
                .is_empty(),
            "failing to start should not commit any trade event"
        );
    }

    #[test]
    fn partial_lot_trade_splits_and_preserves_conservation() {
        let mut harness = TradeHarness::new(
            CommodityKind::Coin,
            Quantity(2),
            CommodityKind::Bread,
            Quantity(2),
            1,
            HomeostaticNeeds::new(pm(900), pm(0), pm(0), pm(0), pm(0)),
        );
        {
            let mut txn = new_txn(&mut harness.world, 3);
            txn.clear_possessor(harness.counterparty_offer).unwrap();
            txn.clear_owner(harness.counterparty_offer).unwrap();
            txn.archive_entity(harness.counterparty_offer).unwrap();
            let replacement = txn
                .create_item_lot(CommodityKind::Bread, Quantity(3))
                .unwrap();
            txn.set_ground_location(replacement, harness.place).unwrap();
            txn.set_possessor(replacement, harness.counterparty)
                .unwrap();
            txn.set_owner(replacement, harness.counterparty).unwrap();
            txn.set_component_sale_listing(
                replacement,
                SaleListing {
                    listed_at: Tick(3),
                },
            )
            .unwrap();
            commit_txn(txn);
            harness.counterparty_offer = replacement;
            harness.payload.sale_lot = replacement;
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
                rng: &mut harness.rng,
            },
            worldwake_sim::ActionExecutionContext::without_recipes(CauseRef::Bootstrap, Tick(4)),
        )
        .unwrap();

        assert!(matches!(outcome, TickOutcome::Committed { .. }));
        verify_live_lot_conservation(&harness.world, CommodityKind::Bread, 3).unwrap();
    }

    #[test]
    fn trade_aborts_when_counterparty_leaves_before_commit() {
        let mut harness = TradeHarness::new(
            CommodityKind::Coin,
            Quantity(1),
            CommodityKind::Bread,
            Quantity(1),
            1,
            HomeostaticNeeds::new(pm(900), pm(0), pm(0), pm(0), pm(0)),
        );
        let (instance_id, mut active) = harness.start_with_active();
        let other_place = harness.world.topology().place_ids().nth(1).unwrap();
        {
            let mut txn = new_txn(&mut harness.world, 4);
            txn.set_ground_location(harness.counterparty, other_place)
                .unwrap();
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
                rng: &mut harness.rng,
            },
            worldwake_sim::ActionExecutionContext::without_recipes(CauseRef::Bootstrap, Tick(5)),
        )
        .unwrap();

        assert!(matches!(outcome, TickOutcome::Aborted { .. }));
        assert_eq!(harness.log.events_by_tag(EventTag::ActionAborted).len(), 1);
    }

    #[test]
    fn trade_aborts_when_counterparty_loses_requested_commodity_before_commit() {
        let mut harness = TradeHarness::new(
            CommodityKind::Coin,
            Quantity(1),
            CommodityKind::Bread,
            Quantity(1),
            1,
            HomeostaticNeeds::new(pm(900), pm(0), pm(0), pm(0), pm(0)),
        );
        let (instance_id, mut active) = harness.start_with_active();
        harness.remove_counterparty_offer();

        let outcome = tick_action(
            instance_id,
            &harness.defs,
            &harness.handlers,
            ActionExecutionAuthority {
                active_actions: &mut active,
                world: &mut harness.world,
                event_log: &mut harness.log,
                rng: &mut harness.rng,
            },
            worldwake_sim::ActionExecutionContext::without_recipes(CauseRef::Bootstrap, Tick(4)),
        )
        .unwrap();

        assert!(matches!(outcome, TickOutcome::Aborted { .. }));
        assert_eq!(harness.log.events_by_tag(EventTag::ActionAborted).len(), 1);
        verify_live_lot_conservation(&harness.world, CommodityKind::Coin, 1).unwrap();
    }

    #[test]
    fn trade_start_initializes_negotiation_state_without_bundle_acceptance() {
        let mut harness = TradeHarness::new(
            CommodityKind::Coin,
            Quantity(1),
            CommodityKind::Bread,
            Quantity(1),
            3,
            HomeostaticNeeds::new_sated(),
        );
        harness.set_counterparty_demand_memory(vec![
            expensive_rejection(CommodityKind::Bread, harness.actor, 1),
            expensive_rejection(CommodityKind::Bread, harness.actor, 2),
        ]);

        let (_instance_id, active) = harness.start_with_active();
        let instance = active.values().next().unwrap();

        assert_eq!(
            instance.local_state,
            Some(ActionState::Trade {
                round: 0,
                initiator_role: worldwake_core::TradeRole::Buyer,
                initiator_last_offer: Some(Quantity(1)),
                responder_last_offer: None,
                agreed_price: None,
            })
        );
        assert_eq!(
            harness.world.possessor_of(harness.actor_offer),
            Some(harness.actor)
        );
        assert!(
            harness
                .world
                .can_exercise_control(harness.counterparty, harness.counterparty_offer)
                .is_ok(),
            "counterparty should still control the sale lot after trade start"
        );
    }

    #[test]
    fn local_alternatives_exclude_focal_and_counterparty() {
        let mut harness = TradeHarness::new(
            CommodityKind::Coin,
            Quantity(1),
            CommodityKind::Bread,
            Quantity(1),
            1,
            HomeostaticNeeds::new_sated(),
        );
        let bystander = {
            let mut txn = new_txn(&mut harness.world, 3);
            let bystander = txn.create_agent("Cato", ControlSource::Ai).unwrap();
            let stock = txn
                .create_item_lot(CommodityKind::Bread, Quantity(2))
                .unwrap();
            txn.set_ground_location(bystander, harness.place).unwrap();
            txn.set_ground_location(stock, harness.place).unwrap();
            txn.set_possessor(stock, bystander).unwrap();
            txn.set_owner(stock, bystander).unwrap();
            commit_txn(txn);
            bystander
        };

        let txn = new_txn(&mut harness.world, 4);
        let alternatives =
            local_alternatives(&txn, harness.actor, harness.counterparty, harness.place);
        drop(txn);

        assert_eq!(
            alternatives,
            vec![(bystander, CommodityKind::Bread, Quantity(2))]
        );
    }

    #[test]
    fn substitute_selection_chooses_first_acceptable_preference_in_order() {
        let mut harness = TradeHarness::new(
            CommodityKind::Coin,
            Quantity(1),
            CommodityKind::Bread,
            Quantity(1),
            1,
            HomeostaticNeeds::new(pm(900), pm(0), pm(0), pm(0), pm(0)),
        );
        let (apple_seller, grain_seller) = {
            let mut txn = new_txn(&mut harness.world, 3);
            txn.set_component_substitute_preferences(
                harness.actor,
                food_substitutes(vec![CommodityKind::Apple, CommodityKind::Grain]),
            )
            .unwrap();

            let apple_seller = txn.create_agent("Apple Seller", ControlSource::Ai).unwrap();
            let apple_stock = txn
                .create_item_lot(CommodityKind::Apple, Quantity(1))
                .unwrap();
            txn.set_ground_location(apple_seller, harness.place)
                .unwrap();
            txn.set_ground_location(apple_stock, harness.place).unwrap();
            txn.set_possessor(apple_stock, apple_seller).unwrap();
            txn.set_owner(apple_stock, apple_seller).unwrap();

            let grain_seller = txn.create_agent("Grain Seller", ControlSource::Ai).unwrap();
            let grain_stock = txn
                .create_item_lot(CommodityKind::Grain, Quantity(1))
                .unwrap();
            txn.set_ground_location(grain_seller, harness.place)
                .unwrap();
            txn.set_ground_location(grain_stock, harness.place).unwrap();
            txn.set_possessor(grain_stock, grain_seller).unwrap();
            txn.set_owner(grain_stock, grain_seller).unwrap();
            commit_txn(txn);
            (apple_seller, grain_seller)
        };

        let txn = new_txn(&mut harness.world, 4);
        let candidate = select_substitute_trade_candidate(
            &txn,
            harness.actor,
            CommodityKind::Bread,
            Quantity(1),
            CommodityKind::Coin,
            Quantity(1),
            harness.place,
        );
        drop(txn);

        assert_eq!(
            candidate,
            Some(SubstituteTradeCandidate {
                seller: apple_seller,
                commodity: CommodityKind::Apple,
                quantity: Quantity(1),
            })
        );
        assert_ne!(candidate.unwrap().seller, grain_seller);
    }

    #[test]
    fn substitute_selection_skips_unavailable_earlier_preference() {
        let mut harness = TradeHarness::new(
            CommodityKind::Coin,
            Quantity(1),
            CommodityKind::Bread,
            Quantity(1),
            1,
            HomeostaticNeeds::new(pm(900), pm(0), pm(0), pm(0), pm(0)),
        );
        let grain_seller = {
            let other_place = harness.world.topology().place_ids().nth(1).unwrap();
            let mut txn = new_txn(&mut harness.world, 3);
            txn.set_component_substitute_preferences(
                harness.actor,
                food_substitutes(vec![CommodityKind::Apple, CommodityKind::Grain]),
            )
            .unwrap();

            let remote_seller = txn.create_agent("Remote Apple", ControlSource::Ai).unwrap();
            let remote_stock = txn
                .create_item_lot(CommodityKind::Apple, Quantity(1))
                .unwrap();
            txn.set_ground_location(remote_seller, other_place).unwrap();
            txn.set_ground_location(remote_stock, other_place).unwrap();
            txn.set_possessor(remote_stock, remote_seller).unwrap();
            txn.set_owner(remote_stock, remote_seller).unwrap();

            let grain_seller = txn.create_agent("Grain Seller", ControlSource::Ai).unwrap();
            let grain_stock = txn
                .create_item_lot(CommodityKind::Grain, Quantity(1))
                .unwrap();
            txn.set_ground_location(grain_seller, harness.place)
                .unwrap();
            txn.set_ground_location(grain_stock, harness.place).unwrap();
            txn.set_possessor(grain_stock, grain_seller).unwrap();
            txn.set_owner(grain_stock, grain_seller).unwrap();
            commit_txn(txn);
            grain_seller
        };

        let txn = new_txn(&mut harness.world, 4);
        let candidate = select_substitute_trade_candidate(
            &txn,
            harness.actor,
            CommodityKind::Bread,
            Quantity(1),
            CommodityKind::Coin,
            Quantity(1),
            harness.place,
        );
        drop(txn);

        assert_eq!(
            candidate,
            Some(SubstituteTradeCandidate {
                seller: grain_seller,
                commodity: CommodityKind::Grain,
                quantity: Quantity(1),
            })
        );
    }

    #[test]
    fn substitute_selection_returns_none_without_preferences() {
        let mut harness = TradeHarness::new(
            CommodityKind::Coin,
            Quantity(1),
            CommodityKind::Bread,
            Quantity(1),
            1,
            HomeostaticNeeds::new(pm(900), pm(0), pm(0), pm(0), pm(0)),
        );

        let txn = new_txn(&mut harness.world, 4);
        let candidate = select_substitute_trade_candidate(
            &txn,
            harness.actor,
            CommodityKind::Bread,
            Quantity(1),
            CommodityKind::Coin,
            Quantity(1),
            harness.place,
        );
        drop(txn);

        assert_eq!(candidate, None);
    }

    #[test]
    fn substitute_selection_ignores_non_colocated_sellers() {
        let mut harness = TradeHarness::new(
            CommodityKind::Coin,
            Quantity(1),
            CommodityKind::Bread,
            Quantity(1),
            1,
            HomeostaticNeeds::new(pm(900), pm(0), pm(0), pm(0), pm(0)),
        );
        {
            let other_place = harness.world.topology().place_ids().nth(1).unwrap();
            let mut txn = new_txn(&mut harness.world, 3);
            txn.set_component_substitute_preferences(
                harness.actor,
                food_substitutes(vec![CommodityKind::Apple]),
            )
            .unwrap();

            let remote_seller = txn.create_agent("Remote Apple", ControlSource::Ai).unwrap();
            let remote_stock = txn
                .create_item_lot(CommodityKind::Apple, Quantity(1))
                .unwrap();
            txn.set_ground_location(remote_seller, other_place).unwrap();
            txn.set_ground_location(remote_stock, other_place).unwrap();
            txn.set_possessor(remote_stock, remote_seller).unwrap();
            txn.set_owner(remote_stock, remote_seller).unwrap();
            commit_txn(txn);
        }

        let txn = new_txn(&mut harness.world, 4);
        let candidate = select_substitute_trade_candidate(
            &txn,
            harness.actor,
            CommodityKind::Bread,
            Quantity(1),
            CommodityKind::Coin,
            Quantity(1),
            harness.place,
        );
        drop(txn);

        assert_eq!(candidate, None);
    }

    #[test]
    fn substitute_selection_skips_valuation_rejected_candidate_for_later_acceptable_one() {
        let mut harness = TradeHarness::new(
            CommodityKind::Coin,
            Quantity(1),
            CommodityKind::Bread,
            Quantity(1),
            1,
            HomeostaticNeeds::new_sated(),
        );
        let grain_seller = {
            let mut txn = new_txn(&mut harness.world, 3);
            txn.set_component_substitute_preferences(
                harness.actor,
                food_substitutes(vec![CommodityKind::Apple, CommodityKind::Grain]),
            )
            .unwrap();
            txn.set_component_demand_memory(harness.actor, remembered_demand(CommodityKind::Grain))
                .unwrap();

            let apple_seller = txn.create_agent("Apple Seller", ControlSource::Ai).unwrap();
            let apple_stock = txn
                .create_item_lot(CommodityKind::Apple, Quantity(1))
                .unwrap();
            txn.set_ground_location(apple_seller, harness.place)
                .unwrap();
            txn.set_ground_location(apple_stock, harness.place).unwrap();
            txn.set_possessor(apple_stock, apple_seller).unwrap();
            txn.set_owner(apple_stock, apple_seller).unwrap();

            let grain_seller = txn.create_agent("Grain Seller", ControlSource::Ai).unwrap();
            let grain_stock = txn
                .create_item_lot(CommodityKind::Grain, Quantity(1))
                .unwrap();
            txn.set_ground_location(grain_seller, harness.place)
                .unwrap();
            txn.set_ground_location(grain_stock, harness.place).unwrap();
            txn.set_possessor(grain_stock, grain_seller).unwrap();
            txn.set_owner(grain_stock, grain_seller).unwrap();
            commit_txn(txn);
            grain_seller
        };

        let txn = new_txn(&mut harness.world, 4);
        let candidate = select_substitute_trade_candidate(
            &txn,
            harness.actor,
            CommodityKind::Bread,
            Quantity(1),
            CommodityKind::Coin,
            Quantity(1),
            harness.place,
        );
        drop(txn);

        assert_eq!(
            candidate,
            Some(SubstituteTradeCandidate {
                seller: grain_seller,
                commodity: CommodityKind::Grain,
                quantity: Quantity(1),
            })
        );
    }

    // -----------------------------------------------------------------------
    // sale_lot commit validation tests
    // -----------------------------------------------------------------------

    #[test]
    fn trade_aborts_when_sale_lot_listing_removed_before_commit() {
        let mut harness = TradeHarness::new(
            CommodityKind::Coin,
            Quantity(1),
            CommodityKind::Bread,
            Quantity(1),
            1,
            HomeostaticNeeds::new(pm(900), pm(0), pm(0), pm(0), pm(0)),
        );
        let (instance_id, mut active) = harness.start_with_active();
        // Remove the SaleListing between start and commit.
        {
            let mut txn = new_txn(&mut harness.world, 4);
            let _ = txn.clear_component_sale_listing(harness.counterparty_offer);
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
                rng: &mut harness.rng,
            },
            worldwake_sim::ActionExecutionContext::without_recipes(CauseRef::Bootstrap, Tick(5)),
        )
        .unwrap();

        assert!(matches!(outcome, TickOutcome::Aborted { .. }));
    }

    #[test]
    fn trade_aborts_when_seller_loses_control_of_sale_lot_before_commit() {
        let mut harness = TradeHarness::new(
            CommodityKind::Coin,
            Quantity(1),
            CommodityKind::Bread,
            Quantity(1),
            1,
            HomeostaticNeeds::new(pm(900), pm(0), pm(0), pm(0), pm(0)),
        );
        let (instance_id, mut active) = harness.start_with_active();
        // Remove the lot from the display container and clear its
        // StockAssignment so the seller no longer controls it.
        {
            let mut txn = new_txn(&mut harness.world, 4);
            txn.remove_from_container(harness.counterparty_offer)
                .unwrap();
            txn.set_ground_location(harness.counterparty_offer, harness.place)
                .unwrap();
            txn.clear_component_stock_assignment(harness.counterparty_offer)
                .unwrap();
            // Clear ownership so the seller has no control path at all.
            txn.clear_owner(harness.counterparty_offer).unwrap();
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
                rng: &mut harness.rng,
            },
            worldwake_sim::ActionExecutionContext::without_recipes(CauseRef::Bootstrap, Tick(5)),
        )
        .unwrap();

        assert!(matches!(outcome, TickOutcome::Aborted { .. }));
    }

    #[test]
    fn trade_removes_sale_listing_from_transferred_lot() {
        let mut harness = TradeHarness::new(
            CommodityKind::Coin,
            Quantity(1),
            CommodityKind::Bread,
            Quantity(1),
            1,
            HomeostaticNeeds::new(pm(900), pm(0), pm(0), pm(0), pm(0)),
        );
        // Confirm listing exists before trade.
        assert!(harness
            .world
            .get_component_sale_listing(harness.counterparty_offer)
            .is_some());

        let (instance_id, mut active) = harness.start_with_active();
        let _outcome = tick_action(
            instance_id,
            &harness.defs,
            &harness.handlers,
            ActionExecutionAuthority {
                active_actions: &mut active,
                world: &mut harness.world,
                event_log: &mut harness.log,
                rng: &mut harness.rng,
            },
            worldwake_sim::ActionExecutionContext::without_recipes(CauseRef::Bootstrap, Tick(5)),
        )
        .unwrap();

        // After trade, the transferred lot should not have SaleListing.
        assert!(harness
            .world
            .get_component_sale_listing(harness.counterparty_offer)
            .is_none());
    }

    #[test]
    fn trade_preserves_sale_listing_on_seller_remainder_after_partial_trade() {
        // Seller has Quantity(3) of Bread, buyer wants Quantity(1).
        // After split + transfer, seller's original lot retains SaleListing.
        let mut harness = TradeHarness::new(
            CommodityKind::Coin,
            Quantity(1),
            CommodityKind::Bread,
            // Create counterparty lot with 3 units so a split occurs.
            Quantity(3),
            1,
            HomeostaticNeeds::new(pm(900), pm(0), pm(0), pm(0), pm(0)),
        );
        // Override requested_quantity to 1 so only part of the lot transfers.
        harness.payload.requested_quantity = Quantity(1);

        let (instance_id, mut active) = harness.start_with_active();
        let _outcome = tick_action(
            instance_id,
            &harness.defs,
            &harness.handlers,
            ActionExecutionAuthority {
                active_actions: &mut active,
                world: &mut harness.world,
                event_log: &mut harness.log,
                rng: &mut harness.rng,
            },
            worldwake_sim::ActionExecutionContext::without_recipes(CauseRef::Bootstrap, Tick(5)),
        )
        .unwrap();

        // Seller's original lot should retain SaleListing (it still has Quantity(2)).
        assert!(harness
            .world
            .get_component_sale_listing(harness.counterparty_offer)
            .is_some());
        // Seller's remainder should still have quantity.
        let remainder = harness
            .world
            .get_component_item_lot(harness.counterparty_offer)
            .unwrap();
        assert_eq!(remainder.quantity, Quantity(2));
    }

    // -----------------------------------------------------------------------
    // staff_market action tests
    // -----------------------------------------------------------------------

    use super::register_staff_market_action;
    use worldwake_sim::StaffMarketPayload;

    struct StaffMarketHarness {
        world: World,
        defs: ActionDefRegistry,
        handlers: ActionHandlerRegistry,
        log: EventLog,
        rng: DeterministicRng,
        next_instance_id: ActionInstanceId,
        actor: EntityId,
        lot: EntityId,
        place: EntityId,
        def_id: ActionDefId,
        commodity: CommodityKind,
    }

    impl StaffMarketHarness {
        fn new() -> Self {
            Self::with_commodity(CommodityKind::Bread)
        }

        fn with_commodity(commodity: CommodityKind) -> Self {
            use worldwake_core::{LoadUnits, StockAssignment, StockAssignmentKind};

            let mut world = World::new(build_prototype_world()).unwrap();
            let place = world.topology().place_ids().next().unwrap();
            let (actor, lot) = {
                let mut txn = new_txn(&mut world, 1);
                let actor = txn.create_agent("Merchant", ControlSource::Ai).unwrap();
                let lot = txn.create_item_lot(commodity, Quantity(5)).unwrap();
                commit_txn(txn);
                (actor, lot)
            };
            {
                let mut txn = new_txn(&mut world, 2);
                txn.set_ground_location(actor, place).unwrap();
                // Create facility with display container and stage the lot.
                let (facility, _stock, display) = txn
                    .create_merchant_facility(place, actor, LoadUnits(200), Some(LoadUnits(100)))
                    .unwrap();
                let display = display.unwrap();
                txn.put_into_container(lot, display).unwrap();
                txn.set_component_stock_assignment(
                    lot,
                    StockAssignment {
                        facility,
                        kind: StockAssignmentKind::Displayed,
                    },
                )
                .unwrap();
                txn.set_component_sale_listing(
                    lot,
                    worldwake_core::SaleListing {
                        listed_at: worldwake_core::Tick(2),
                    },
                )
                .unwrap();
                txn.set_component_merchandise_profile(
                    actor,
                    MerchandiseProfile {
                        sale_kinds: [commodity].into_iter().collect(),
                        home_facility: Some(facility),
                    },
                )
                .unwrap();
                txn.set_component_trade_disposition_profile(
                    actor,
                    TradeDispositionProfile {
                        negotiation_round_ticks: nz(5),
                        initial_offer_bias: pm(500),
                        concession_rate: pm(200),
                        rejection_escalation_rate: pm(200),
                        demand_memory_retention_ticks: 10,
                        market_presence_ticks: nz(10),
                    },
                )
                .unwrap();
                txn.set_component_homeostatic_needs(actor, HomeostaticNeeds::new_sated())
                    .unwrap();
                commit_txn(txn);
            }

            let mut defs = ActionDefRegistry::new();
            let mut handlers = ActionHandlerRegistry::new();
            let def_id = register_staff_market_action(&mut defs, &mut handlers);
            Self {
                world,
                defs,
                handlers,
                log: EventLog::new(),
                rng: test_rng(),
                next_instance_id: ActionInstanceId(0),
                actor,
                lot,
                place,
                def_id,
                commodity,
            }
        }

        fn start_with_active(
            &mut self,
        ) -> (
            ActionInstanceId,
            BTreeMap<ActionInstanceId, worldwake_sim::ActionInstance>,
        ) {
            let affordance = Affordance {
                def_id: self.def_id,
                actor: self.actor,
                bound_targets: vec![],
                payload_override: Some(ActionPayload::StaffMarket(StaffMarketPayload {
                    commodity: self.commodity,
                })),
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
                    rng: &mut self.rng,
                },
                &mut self.next_instance_id,
                worldwake_sim::ActionExecutionContext::without_recipes(CauseRef::Bootstrap, Tick(3)),
            )
            .unwrap();
            (instance_id, active)
        }

        fn start_result(&mut self) -> Result<ActionInstanceId, ActionError> {
            self.start_result_with_commodity(self.commodity)
        }

        fn start_result_with_commodity(
            &mut self,
            commodity: CommodityKind,
        ) -> Result<ActionInstanceId, ActionError> {
            let affordance = Affordance {
                def_id: self.def_id,
                actor: self.actor,
                bound_targets: vec![],
                payload_override: Some(ActionPayload::StaffMarket(StaffMarketPayload {
                    commodity,
                })),
                explanation: None,
            };
            let mut active = BTreeMap::new();
            start_action(
                &affordance,
                &self.defs,
                &self.handlers,
                ActionExecutionAuthority {
                    active_actions: &mut active,
                    world: &mut self.world,
                    event_log: &mut self.log,
                    rng: &mut self.rng,
                },
                &mut self.next_instance_id,
                worldwake_sim::ActionExecutionContext::without_recipes(CauseRef::Bootstrap, Tick(3)),
            )
        }
    }

    #[test]
    fn staff_market_start_does_not_modify_sale_listing() {
        // Presence-only: SaleListing is managed by stage/unstage, not
        // staff_market.  The listing set during harness setup must survive
        // start unchanged.
        let mut h = StaffMarketHarness::new();
        let listing_before = h.world.get_component_sale_listing(h.lot).cloned();
        assert!(listing_before.is_some(), "harness should pre-stage lot");

        let (_id, _active) = h.start_with_active();

        let listing_after = h.world.get_component_sale_listing(h.lot);
        assert_eq!(
            listing_after,
            listing_before.as_ref(),
            "staff_market start must not modify SaleListing"
        );
    }

    #[test]
    fn staff_market_start_does_not_double_list_already_listed_lots() {
        let mut h = StaffMarketHarness::new();
        // Pre-list the lot at tick 1.
        {
            let mut txn = new_txn(&mut h.world, 2);
            txn.set_component_sale_listing(h.lot, SaleListing { listed_at: Tick(1) })
                .unwrap();
            commit_txn(txn);
        }

        let (_id, _active) = h.start_with_active();

        // Should still have the original listing, not overwritten.
        let listing = h.world.get_component_sale_listing(h.lot).unwrap();
        assert_eq!(listing.listed_at, Tick(1));
    }

    #[test]
    fn staff_market_commit_preserves_displayed_listings() {
        // Presence-only: SaleListing is managed by stage/unstage. After
        // staff_market commit, the displayed lot's SaleListing must remain.
        let mut h = StaffMarketHarness::new();
        let (instance_id, mut active) = h.start_with_active();
        assert!(h.world.get_component_sale_listing(h.lot).is_some());

        for tick in 4..14 {
            let outcome = tick_action(
                instance_id,
                &h.defs,
                &h.handlers,
                ActionExecutionAuthority {
                    active_actions: &mut active,
                    world: &mut h.world,
                    event_log: &mut h.log,
                    rng: &mut h.rng,
                },
                worldwake_sim::ActionExecutionContext::without_recipes(CauseRef::Bootstrap, Tick(tick)),
            )
            .unwrap();
            if matches!(outcome, TickOutcome::Committed { .. }) {
                break;
            }
        }

        // After commit, listing must still be present (managed by unstage, not staff_market).
        assert!(
            h.world.get_component_sale_listing(h.lot).is_some(),
            "staff_market commit must not remove displayed SaleListing"
        );
    }

    #[test]
    fn staff_market_commit_records_wanted_to_sell_but_no_buyer() {
        let mut h = StaffMarketHarness::new();
        let (instance_id, mut active) = h.start_with_active();

        // Run to completion.
        for tick in 4..14 {
            let outcome = tick_action(
                instance_id,
                &h.defs,
                &h.handlers,
                ActionExecutionAuthority {
                    active_actions: &mut active,
                    world: &mut h.world,
                    event_log: &mut h.log,
                    rng: &mut h.rng,
                },
                worldwake_sim::ActionExecutionContext::without_recipes(CauseRef::Bootstrap, Tick(tick)),
            )
            .unwrap();
            if matches!(outcome, TickOutcome::Committed { .. }) {
                break;
            }
        }

        let memory = h.world.get_component_demand_memory(h.actor).unwrap();
        assert_eq!(memory.observations.len(), 1);
        let obs = &memory.observations[0];
        assert_eq!(obs.commodity, CommodityKind::Bread);
        assert_eq!(obs.reason, DemandObservationReason::WantedToSellButNoBuyer);
        assert_eq!(obs.place, h.place);
    }

    #[test]
    fn staff_market_unproductive_commit_creates_blocked_intent_for_sell_commodity() {
        let mut h = StaffMarketHarness::new();
        let (instance_id, mut active) = h.start_with_active();

        // Run to completion without any trades occurring.
        for tick in 4..14 {
            let outcome = tick_action(
                instance_id,
                &h.defs,
                &h.handlers,
                ActionExecutionAuthority {
                    active_actions: &mut active,
                    world: &mut h.world,
                    event_log: &mut h.log,
                    rng: &mut h.rng,
                },
                worldwake_sim::ActionExecutionContext::without_recipes(CauseRef::Bootstrap, Tick(tick)),
            )
            .unwrap();
            if matches!(outcome, TickOutcome::Committed { .. }) {
                break;
            }
        }

        let blocked = h
            .world
            .get_component_blocked_intent_memory(h.actor)
            .expect("blocked intent memory should exist after unproductive cycle");
        let sell_blocker = blocked.intents.values().find(|intent| {
            intent.blocking_fact == BlockingFact::NoBuyer
                && intent.blocker_key.goal_key.kind
                    == GoalKind::SellCommodity {
                        commodity: CommodityKind::Bread,
                    }
                && intent.blocker_key.place == Some(h.place)
        });
        assert!(
            sell_blocker.is_some(),
            "unproductive staff_market commit should create a NoBuyer blocked intent for SellCommodity"
        );
        let intent = sell_blocker.unwrap();
        // Blocking period should equal market_presence_ticks (10 in harness).
        assert!(intent.expires_tick.0 > intent.observed_tick.0);
    }

    #[test]
    fn staff_market_abort_preserves_displayed_listings() {
        let mut h = StaffMarketHarness::new();
        let (instance_id, mut active) = h.start_with_active();
        assert!(h.world.get_component_sale_listing(h.lot).is_some());

        // Tick once then abort.
        let outcome = tick_action(
            instance_id,
            &h.defs,
            &h.handlers,
            ActionExecutionAuthority {
                active_actions: &mut active,
                world: &mut h.world,
                event_log: &mut h.log,
                rng: &mut h.rng,
            },
            worldwake_sim::ActionExecutionContext::without_recipes(CauseRef::Bootstrap, Tick(4)),
        )
        .unwrap();
        assert!(matches!(outcome, TickOutcome::Continuing));

        // Call the abort handler directly.
        let instance = active.get(&instance_id).unwrap();
        let def = h.defs.get(instance.def_id).unwrap();
        let handler_entry = h.handlers.get(def.handler).unwrap();
        let mut txn = WorldTxn::new(
            &mut h.world,
            Tick(5),
            CauseRef::Bootstrap,
            None,
            None,
            VisibilitySpec::SamePlace,
            WitnessData::default(),
        );
        (handler_entry.on_abort)(
            def,
            instance,
            &worldwake_sim::ActionExecutionContext::without_recipes(CauseRef::Bootstrap, txn.tick()),
            &worldwake_sim::AbortReason::Interrupted {
                kind: worldwake_sim::InterruptReason::Other,
                detail: None,
            },
            &EventLog::new(),
            &mut h.rng,
            &mut txn,
        )
        .unwrap();
        let mut log = EventLog::new();
        let _ = txn.commit(&mut log);

        // Presence-only: abort must not remove displayed SaleListing.
        assert!(
            h.world.get_component_sale_listing(h.lot).is_some(),
            "staff_market abort must not remove displayed SaleListing"
        );
    }

    #[test]
    fn staff_market_fails_if_actor_not_at_home_facility() {
        let mut h = StaffMarketHarness::new();
        // Move actor to a different place.
        let other_place = h
            .world
            .topology()
            .place_ids()
            .find(|p| *p != h.place)
            .unwrap();
        {
            let mut txn = new_txn(&mut h.world, 3);
            txn.set_ground_location(h.actor, other_place).unwrap();
            commit_txn(txn);
        }

        let result = h.start_result();
        assert!(result.is_err());
    }

    #[test]
    fn staff_market_fails_if_commodity_not_in_sale_kinds() {
        let mut h = StaffMarketHarness::new();
        // Try to list Sword, which is not in sale_kinds (only Bread is).
        let result = h.start_result_with_commodity(CommodityKind::Sword);
        assert!(result.is_err());
    }

    #[test]
    fn staff_market_fails_if_no_local_stock() {
        let mut h = StaffMarketHarness::new();
        // Remove the lot from actor's possession.
        {
            let mut txn = new_txn(&mut h.world, 3);
            txn.clear_possessor(h.lot).unwrap();
            txn.clear_owner(h.lot).unwrap();
            txn.archive_entity(h.lot).unwrap();
            commit_txn(txn);
        }

        let result = h.start_result();
        assert!(result.is_err());
    }

    #[test]
    fn staff_market_duration_resolves_to_market_presence_ticks() {
        let mut h = StaffMarketHarness::new();
        let (_id, active) = h.start_with_active();
        let instance = active.values().next().unwrap();

        // market_presence_ticks = 10
        assert_eq!(
            instance.remaining_duration,
            worldwake_sim::ActionDuration::new(10)
        );
    }
}
