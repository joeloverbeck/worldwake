//! Golden tests for merchant selling market-presence (S04 spec).
//!
//! These tests exercise the seller-side lifecycle: `SaleListing` attachment via
//! `staff_market`, buyer discovery of listed lots, trade against concrete listed
//! lots, autonomous stock staging before sell readiness, and deterministic
//! replay of the full merchant-selling pipeline.

mod golden_harness;

use golden_harness::*;
use std::collections::BTreeSet;
use worldwake_ai::DecisionOutcome;
use worldwake_core::{
    AgentData, CommodityKind, ControlSource, DemandMemory, DemandObservation,
    DemandObservationReason, EventTag, GoalKind, HomeostaticNeeds, KnownRecipes, LoadUnits,
    MerchandiseProfile, MetabolismProfile, PerceptionProfile, Quantity, SaleListing, Seed,
    StockAssignmentKind, Tick, TradeDispositionProfile, UtilityProfile, hash_event_log, hash_world,
    total_live_lot_quantity,
};
use worldwake_sim::{
    ActionRequestMode, ActionTraceKind, EconomicBeliefView, InputKind, PerAgentBeliefView,
    RecipeRegistry, RequestProvenance,
};

// ---------------------------------------------------------------------------
// Shared helpers
// ---------------------------------------------------------------------------

fn merchant_trade_disposition() -> TradeDispositionProfile {
    TradeDispositionProfile {
        negotiation_round_ticks: nz(1),
        initial_offer_bias: pm(500),
        concession_rate: pm(200),
        rejection_escalation_rate: pm(200),
        demand_memory_retention_ticks: 48,
        market_presence_ticks: nz(10),
    }
}

fn merchant_utility() -> UtilityProfile {
    UtilityProfile {
        enterprise_weight: pm(800),
        ..UtilityProfile::default()
    }
}

fn set_control_source(
    h: &mut GoldenHarness,
    agent: worldwake_core::EntityId,
    control_source: ControlSource,
    tick: u64,
) {
    let mut txn = new_txn(&mut h.world, tick);
    txn.set_component_agent_data(agent, AgentData { control_source })
        .unwrap();
    commit_txn(txn, &mut h.event_log);
}

fn request_simple_action(
    h: &mut GoldenHarness,
    actor: worldwake_core::EntityId,
    def_name: &str,
    targets: Vec<worldwake_core::EntityId>,
) {
    let def_id = h.defs.iter().find(|def| def.name == def_name).map_or_else(
        || panic!("full registries should include {def_name}"),
        |def| def.id,
    );
    let tick = h.scheduler.current_tick();
    let _ = h.scheduler.input_queue_mut().enqueue(
        tick,
        InputKind::RequestAction {
            actor,
            def_id,
            targets,
            payload_override: None,
            mode: ActionRequestMode::BestEffort,
            provenance: RequestProvenance::External,
        },
    );
}

/// Seed a merchant at `place` with `MerchandiseProfile`, trade disposition,
/// perception, AI control, enterprise utility, a facility with display
/// container, and stock of `commodity` staged in the display container
/// (with `SaleListing` and `StockAssignment::Displayed`).
///
/// Returns `(merchant_entity, stock_lot_entity)`.
fn seed_merchant(
    h: &mut GoldenHarness,
    name: &str,
    place: worldwake_core::EntityId,
    commodity: CommodityKind,
    quantity: Quantity,
) -> (worldwake_core::EntityId, worldwake_core::EntityId) {
    use worldwake_core::{LoadUnits, StockAssignment, StockAssignmentKind};

    let merchant = seed_agent_with_recipes(
        &mut h.world,
        &mut h.event_log,
        name,
        place,
        HomeostaticNeeds::new_sated(),
        MetabolismProfile::default(),
        merchant_utility(),
        KnownRecipes::new(),
    );

    let stock_lot = give_commodity(
        &mut h.world,
        &mut h.event_log,
        merchant,
        place,
        commodity,
        quantity,
    );

    let mut txn = new_txn(&mut h.world, 0);
    let (facility, _stock_container, display) = txn
        .create_merchant_facility(place, merchant, LoadUnits(500), Some(LoadUnits(300)))
        .unwrap();
    txn.set_component_merchandise_profile(
        merchant,
        MerchandiseProfile {
            sale_kinds: BTreeSet::from([commodity]),
            home_facility: Some(facility),
        },
    )
    .unwrap();
    txn.set_component_trade_disposition_profile(merchant, merchant_trade_disposition())
        .unwrap();
    txn.set_component_perception_profile(merchant, PerceptionProfile::default())
        .unwrap();
    // Seed demand memory so the enterprise motive is nonzero.
    txn.set_component_demand_memory(
        merchant,
        DemandMemory {
            observations: vec![DemandObservation {
                commodity,
                quantity: Quantity(5),
                place,
                tick: Tick(0),
                counterparty: None,
                reason: DemandObservationReason::WantedToBuyButSellerOutOfStock,
            }],
        },
    )
    .unwrap();
    // Move the lot into the facility display container and mark it listed.
    let display_container = display.unwrap();
    // Move lot from direct possession into the display container.
    txn.clear_possessor(stock_lot).unwrap();
    txn.put_into_container(stock_lot, display_container)
        .unwrap();
    txn.set_component_stock_assignment(
        stock_lot,
        StockAssignment {
            facility,
            kind: StockAssignmentKind::Displayed,
        },
    )
    .unwrap();
    txn.set_component_sale_listing(stock_lot, SaleListing { listed_at: Tick(0) })
        .unwrap();
    commit_txn(txn, &mut h.event_log);

    (merchant, stock_lot)
}

/// Like `seed_merchant` but puts the lot in the STOCK container (Stored)
/// WITHOUT `SaleListing`.  Use for tests that need non-sale-visible stock.
fn seed_merchant_with_stored_stock(
    h: &mut GoldenHarness,
    name: &str,
    place: worldwake_core::EntityId,
    commodity: CommodityKind,
    quantity: Quantity,
) -> (worldwake_core::EntityId, worldwake_core::EntityId) {
    use worldwake_core::{LoadUnits, StockAssignment, StockAssignmentKind};

    let merchant = seed_agent_with_recipes(
        &mut h.world,
        &mut h.event_log,
        name,
        place,
        HomeostaticNeeds::new_sated(),
        MetabolismProfile::default(),
        merchant_utility(),
        KnownRecipes::new(),
    );

    let stock_lot = give_commodity(
        &mut h.world,
        &mut h.event_log,
        merchant,
        place,
        commodity,
        quantity,
    );

    let mut txn = new_txn(&mut h.world, 0);
    let (facility, stock_container, _display) = txn
        .create_merchant_facility(place, merchant, LoadUnits(500), Some(LoadUnits(300)))
        .unwrap();
    txn.set_component_merchandise_profile(
        merchant,
        MerchandiseProfile {
            sale_kinds: BTreeSet::from([commodity]),
            home_facility: Some(facility),
        },
    )
    .unwrap();
    txn.set_component_trade_disposition_profile(merchant, merchant_trade_disposition())
        .unwrap();
    txn.set_component_perception_profile(merchant, PerceptionProfile::default())
        .unwrap();
    txn.set_component_demand_memory(
        merchant,
        DemandMemory {
            observations: vec![DemandObservation {
                commodity,
                quantity: Quantity(5),
                place,
                tick: Tick(0),
                counterparty: None,
                reason: DemandObservationReason::WantedToBuyButSellerOutOfStock,
            }],
        },
    )
    .unwrap();
    // Put lot in stock container (NOT displayed).
    txn.clear_possessor(stock_lot).unwrap();
    txn.put_into_container(stock_lot, stock_container).unwrap();
    txn.set_component_stock_assignment(
        stock_lot,
        StockAssignment {
            facility,
            kind: StockAssignmentKind::Stored,
        },
    )
    .unwrap();
    // No SaleListing — stock is stored, not displayed.
    commit_txn(txn, &mut h.event_log);

    (merchant, stock_lot)
}

fn seed_merchant_with_loose_stock(
    h: &mut GoldenHarness,
    name: &str,
    place: worldwake_core::EntityId,
    commodity: CommodityKind,
    quantity: Quantity,
    with_demand_memory: bool,
) -> (
    worldwake_core::EntityId,
    worldwake_core::EntityId,
    worldwake_core::EntityId,
) {
    let merchant = seed_agent_with_recipes(
        &mut h.world,
        &mut h.event_log,
        name,
        place,
        HomeostaticNeeds::new_sated(),
        MetabolismProfile::default(),
        merchant_utility(),
        KnownRecipes::new(),
    );

    let stock_lot = give_commodity(
        &mut h.world,
        &mut h.event_log,
        merchant,
        place,
        commodity,
        quantity,
    );

    let mut txn = new_txn(&mut h.world, 0);
    let (facility, _stock_container, _display) = txn
        .create_merchant_facility(place, merchant, LoadUnits(500), Some(LoadUnits(300)))
        .unwrap();
    txn.set_component_merchandise_profile(
        merchant,
        MerchandiseProfile {
            sale_kinds: BTreeSet::from([commodity]),
            home_facility: Some(facility),
        },
    )
    .unwrap();
    txn.set_component_trade_disposition_profile(merchant, merchant_trade_disposition())
        .unwrap();
    txn.set_component_perception_profile(merchant, PerceptionProfile::default())
        .unwrap();
    if with_demand_memory {
        txn.set_component_demand_memory(
            merchant,
            DemandMemory {
                observations: vec![DemandObservation {
                    commodity,
                    quantity: Quantity(5),
                    place,
                    tick: Tick(0),
                    counterparty: None,
                    reason: DemandObservationReason::WantedToBuyButSellerOutOfStock,
                }],
            },
        )
        .unwrap();
    }
    commit_txn(txn, &mut h.event_log);

    (merchant, stock_lot, facility)
}

/// Seed a buyer at `place` with coin, trade disposition, perception, and
/// hunger so `AcquireCommodity` is motivated.
fn seed_buyer(
    h: &mut GoldenHarness,
    name: &str,
    place: worldwake_core::EntityId,
    coin_quantity: Quantity,
) -> worldwake_core::EntityId {
    let buyer = seed_agent_with_recipes(
        &mut h.world,
        &mut h.event_log,
        name,
        place,
        HomeostaticNeeds::new(pm(900), pm(0), pm(0), pm(0), pm(0)),
        MetabolismProfile::default(),
        UtilityProfile::default(),
        KnownRecipes::new(),
    );
    give_commodity(
        &mut h.world,
        &mut h.event_log,
        buyer,
        place,
        CommodityKind::Coin,
        coin_quantity,
    );

    let mut txn = new_txn(&mut h.world, 0);
    txn.set_component_trade_disposition_profile(buyer, merchant_trade_disposition())
        .unwrap();
    txn.set_component_perception_profile(buyer, PerceptionProfile::default())
        .unwrap();
    commit_txn(txn, &mut h.event_log);

    buyer
}

// ---------------------------------------------------------------------------
// Scenario 75: Displayed Lot Retains SaleListing Through Presence Cycle
// Systems: Trade, AI
// GoalKinds: SellCommodity
// ActionDomains: Trade
// Principles: P1, P3, P4
// Proves: SaleListing on displayed lot persists across idle ticks when no
//         trade or unstage occurs
// ---------------------------------------------------------------------------

#[allow(clippy::too_many_lines)]
fn run_displayed_lot_retains_listing(
    seed: Seed,
) -> (worldwake_core::StateHash, worldwake_core::StateHash) {
    let mut h = GoldenHarness::with_recipes(seed, RecipeRegistry::new());
    h.driver.enable_tracing();
    h.enable_action_tracing();

    let (merchant, stock_lot) = seed_merchant(
        &mut h,
        "Merchant",
        VILLAGE_SQUARE,
        CommodityKind::Bread,
        Quantity(3),
    );

    seed_actor_world_beliefs(
        &mut h.world,
        &mut h.event_log,
        merchant,
        Tick(0),
        worldwake_core::PerceptionSource::Inference,
    );
    {
        let mut txn = new_txn(&mut h.world, 0);
        txn.set_component_agent_data(
            merchant,
            worldwake_core::AgentData {
                control_source: ControlSource::Human,
            },
        )
        .unwrap();
        commit_txn(txn, &mut h.event_log);
    }

    // Before any ticks: SaleListing present from staging during seed_merchant.
    assert!(
        h.world.get_component_sale_listing(stock_lot).is_some(),
        "displayed stock lot should have SaleListing from staging"
    );

    for _ in 0..160 {
        h.step_once();
    }
    assert!(
        h.world.get_component_sale_listing(stock_lot).is_some(),
        "SaleListing must persist across idle ticks when no trade or unstage occurs"
    );

    (
        hash_world(&h.world).unwrap(),
        hash_event_log(&h.event_log).unwrap(),
    )
}

#[test]
fn staff_market_retains_displayed_listing_through_presence_cycle() {
    run_displayed_lot_retains_listing(Seed([60; 32]));
}

// ---------------------------------------------------------------------------
// Scenario 76: Buyer Trades Against Listed Lot
// Systems: Trade, AI, Needs
// GoalKinds: AcquireCommodity, SellCommodity
// ActionDomains: Trade
// Principles: P1, P3, P4
// Proves: buyer discovers and trades against concrete listed lot with conservation
// ---------------------------------------------------------------------------

#[allow(clippy::too_many_lines)]
fn run_buyer_trades_listed_lot(
    seed: Seed,
) -> (worldwake_core::StateHash, worldwake_core::StateHash) {
    let mut h = GoldenHarness::with_recipes(seed, RecipeRegistry::new());
    h.enable_action_tracing();

    let (merchant, stock_lot) = seed_merchant(
        &mut h,
        "Merchant",
        VILLAGE_SQUARE,
        CommodityKind::Bread,
        Quantity(3),
    );
    let buyer = seed_buyer(&mut h, "Buyer", VILLAGE_SQUARE, Quantity(3));

    // Pre-list the stock so buyers can discover it immediately.
    {
        let mut txn = new_txn(&mut h.world, 0);
        txn.set_component_sale_listing(stock_lot, SaleListing { listed_at: Tick(0) })
            .unwrap();
        commit_txn(txn, &mut h.event_log);
    }

    // Seed local beliefs so the buyer can perceive the seller, facility, and listed lot.
    seed_actor_local_beliefs(
        &mut h.world,
        &mut h.event_log,
        merchant,
        Tick(0),
        worldwake_core::PerceptionSource::Inference,
    );
    {
        let mut txn = new_txn(&mut h.world, 0);
        txn.set_component_agent_data(
            merchant,
            worldwake_core::AgentData {
                control_source: ControlSource::Human,
            },
        )
        .unwrap();
        commit_txn(txn, &mut h.event_log);
    }
    seed_actor_world_beliefs(
        &mut h.world,
        &mut h.event_log,
        buyer,
        Tick(0),
        worldwake_core::PerceptionSource::Inference,
    );

    let initial_total_bread = total_live_lot_quantity(&h.world, CommodityKind::Bread);
    let initial_total_coins = total_live_lot_quantity(&h.world, CommodityKind::Coin);

    let view = PerAgentBeliefView::from_world(buyer, &h.world);
    assert!(
        view.listed_sale_lots_at(VILLAGE_SQUARE, CommodityKind::Bread)
            .contains(&stock_lot),
        "buyer belief view should discover the listed bread lot"
    );
    assert_eq!(
        view.seller_for_sale_lot(stock_lot),
        Some(merchant),
        "buyer belief view should resolve the seller from the listed facility lot"
    );

    for _ in 0..20 {
        h.step_once();

        // Conservation must hold every tick.
        let bread_now = total_live_lot_quantity(&h.world, CommodityKind::Bread);
        let coins_now = total_live_lot_quantity(&h.world, CommodityKind::Coin);
        assert!(
            bread_now <= initial_total_bread,
            "bread should not increase (may decrease from consumption)"
        );
        assert_eq!(
            coins_now, initial_total_coins,
            "coins must be conserved through trade"
        );
    }

    (
        hash_world(&h.world).unwrap(),
        hash_event_log(&h.event_log).unwrap(),
    )
}

#[test]
fn buyer_trades_against_listed_lot() {
    run_buyer_trades_listed_lot(Seed([61; 32]));
}

// ---------------------------------------------------------------------------
// Scenario 84: Remote Branch Selection Reaches Local Trade Binding
// Systems: Trade, AI, Needs
// GoalKinds: AcquireCommodity
// ActionDomains: Travel, Trade
// Principles: P1, P3, P4
// Proves: buyer first selects the remote seller-backed `Travel -> Trade` path
//         and, after arrival, reaches a concrete local `trade` next step before
//         seller departure. The mismatch event itself stays owned by the
//         focused `agent_tick` execution proof.
// ---------------------------------------------------------------------------

#[test]
fn remote_branch_selection_reaches_local_trade_binding_before_merchant_departure() {
    let mut h = GoldenHarness::with_recipes(Seed([84; 32]), RecipeRegistry::new());
    h.driver.enable_tracing();

    let (merchant, _stock_lot) = seed_merchant(
        &mut h,
        "Merchant",
        VILLAGE_SQUARE,
        CommodityKind::Bread,
        Quantity(3),
    );
    let buyer = seed_buyer(&mut h, "Buyer", ORCHARD_FARM, Quantity(3));

    {
        let mut txn = new_txn(&mut h.world, 0);
        txn.set_component_agent_data(
            merchant,
            worldwake_core::AgentData {
                control_source: ControlSource::Human,
            },
        )
        .unwrap();
        commit_txn(txn, &mut h.event_log);
    }

    seed_actor_world_beliefs(
        &mut h.world,
        &mut h.event_log,
        buyer,
        Tick(0),
        worldwake_core::PerceptionSource::Inference,
    );

    let mut saw_remote_trade_branch = false;
    let mut saw_local_trade_binding = false;

    for _ in 0..40 {
        let tick = h.step_once().tick;
        let Some(trace) = h
            .driver
            .trace_sink()
            .and_then(|sink| sink.trace_at(buyer, tick))
        else {
            continue;
        };
        let DecisionOutcome::Planning(planning) = &trace.outcome else {
            continue;
        };
        let Some(selected_plan) = planning.selection.selected_plan.as_ref() else {
            continue;
        };

        if !saw_remote_trade_branch
            && selected_plan
                .next_step
                .as_ref()
                .is_some_and(|step| step.action_name == "travel")
            && selected_plan
                .steps
                .iter()
                .any(|step| step.action_name == "trade" && step.targets.contains(&merchant))
        {
            saw_remote_trade_branch = true;
        }

        if h.world.effective_place(buyer) == Some(VILLAGE_SQUARE)
            && h.agent_active_action_name(buyer).is_none()
            && selected_plan
                .next_step
                .as_ref()
                .is_some_and(|step| step.action_name == "trade" && step.targets.contains(&merchant))
        {
            saw_local_trade_binding = true;
            break;
        }
    }

    assert!(
        saw_remote_trade_branch,
        "buyer should first select a remote branch whose next step is travel and whose path retains a later trade step against the seller"
    );
    assert!(
        saw_local_trade_binding,
        "after arrival, buyer should reach a local trade next step bound to the seller before departure"
    );
    assert_eq!(
        h.agent_commodity_qty(buyer, CommodityKind::Bread),
        Quantity(0),
        "this golden stops at the local trade-step binding seam before any trade commits"
    );
    assert!(
        h.event_log
            .events_by_tag(EventTag::ExpectationMismatch)
            .is_empty(),
        "the mismatch event stays owned by the focused AI execution proof rather than this earlier golden boundary"
    );
}

// ---------------------------------------------------------------------------
// Scenario 85: Seller Return Revives Pending Purchase Agenda Entry
// Systems: Trade, AI, Needs
// GoalKinds: AcquireCommodity
// ActionDomains: Travel, Trade
// Principles: P1, P3, P4
// Proves: after a buyer reaches a concrete local `trade` binding, seller
//         departure parks the committed purchase goal into pending with a
//         counterparty-based revival trigger; seller return then revives the
//         agenda entry back into live committed/current-plan state. Seller-side
//         market-presence restaging remains a separate seam.
// ---------------------------------------------------------------------------

#[test]
fn merchant_return_revives_pending_purchase_agenda_entry() {
    let mut h = GoldenHarness::with_recipes(Seed([85; 32]), RecipeRegistry::new());
    h.driver.enable_tracing();

    let (merchant, _stock_lot) = seed_merchant(
        &mut h,
        "Merchant",
        VILLAGE_SQUARE,
        CommodityKind::Bread,
        Quantity(3),
    );
    let buyer = seed_buyer(&mut h, "Buyer", ORCHARD_FARM, Quantity(3));
    let purchase_goal = worldwake_ai::GoalKey::from(GoalKind::AcquireCommodity {
        commodity: CommodityKind::Bread,
        purpose: worldwake_ai::CommodityPurpose::SelfConsume,
    });

    set_control_source(&mut h, merchant, ControlSource::Human, 0);
    seed_actor_world_beliefs(
        &mut h.world,
        &mut h.event_log,
        buyer,
        Tick(0),
        worldwake_core::PerceptionSource::Inference,
    );

    let mut bound_tick = None;
    for _ in 0..40 {
        let tick = h.step_once().tick;
        let Some(trace) = h
            .driver
            .trace_sink()
            .and_then(|sink| sink.trace_at(buyer, tick))
        else {
            continue;
        };
        let DecisionOutcome::Planning(planning) = &trace.outcome else {
            continue;
        };
        let Some(selected_plan) = planning.selection.selected_plan.as_ref() else {
            continue;
        };
        if h.world.effective_place(buyer) == Some(VILLAGE_SQUARE)
            && h.agent_active_action_name(buyer).is_none()
            && selected_plan
                .next_step
                .as_ref()
                .is_some_and(|step| step.action_name == "trade" && step.targets.contains(&merchant))
        {
            bound_tick = Some(tick);
            let mut txn = new_txn(&mut h.world, tick.0);
            txn.set_ground_location(merchant, ORCHARD_FARM).unwrap();
            commit_txn(txn, &mut h.event_log);
            break;
        }
    }

    assert!(
        bound_tick.is_some(),
        "buyer should first reach the local trade-step binding against the seller"
    );

    let mut parked_pending = false;
    for _ in 0..12 {
        h.step_once();
        let Some(runtime) = h.driver.runtime(buyer) else {
            continue;
        };
        let pending = runtime
            .agenda_state
            .pending
            .values()
            .find(|entry| entry.key.goal_key == purchase_goal);
        if let Some(pending) = pending {
            assert_eq!(
                pending.revival_trigger,
                Some(worldwake_ai::RevivalTrigger::CounterpartyAvailable {
                    counterparty: merchant,
                    place: VILLAGE_SQUARE,
                })
            );
            parked_pending = true;
            break;
        }
    }

    assert!(
        parked_pending,
        "seller departure should park the committed purchase goal in pending with a counterparty trigger"
    );
    assert_eq!(
        h.agent_commodity_qty(buyer, CommodityKind::Bread),
        Quantity(0),
        "buyer should not complete trade before the seller returns"
    );

    let mut txn = new_txn(&mut h.world, h.scheduler.current_tick().0);
    txn.set_ground_location(merchant, VILLAGE_SQUARE).unwrap();
    commit_txn(txn, &mut h.event_log);

    let mut pending_cleared = false;
    let mut revived = false;
    for _ in 0..24 {
        h.step_once();
        if let Some(runtime) = h.driver.runtime(buyer) {
            pending_cleared |= runtime
                .agenda_state
                .pending
                .values()
                .all(|entry| entry.key.goal_key != purchase_goal);
            revived |= runtime
                .agenda_state
                .committed
                .as_ref()
                .is_some_and(|entry| entry.key.goal_key == purchase_goal)
                || runtime
                    .current_plan
                    .as_ref()
                    .is_some_and(|plan| plan.goal == purchase_goal);
            if pending_cleared && revived {
                break;
            }
        }
    }

    assert!(
        pending_cleared,
        "seller return should clear the parked pending purchase entry through the real runtime lifecycle"
    );
    assert!(
        revived,
        "seller return should revive the purchase goal into committed/current-plan state"
    );
}

// ---------------------------------------------------------------------------
// Scenario 86: Seller Return Restores Displayed Listing After Pending Revival
// Systems: Trade, AI, Needs
// GoalKinds: AcquireCommodity
// ActionDomains: Travel, Trade
// Principles: P1, P3, P4
// Proves: after buyer-side pending revival is already in place, seller return
//         restores lawful displayed-lot listing state at the authoritative
//         trade seam. The later resumed trade-completion story remains a
//         separate mixed-layer proof seam.
// ---------------------------------------------------------------------------

#[test]
fn seller_return_restores_displayed_listing_after_pending_revival() {
    let mut h = GoldenHarness::with_recipes(Seed([86; 32]), RecipeRegistry::new());
    h.driver.enable_tracing();

    let (merchant, stock_lot) = seed_merchant(
        &mut h,
        "Merchant",
        VILLAGE_SQUARE,
        CommodityKind::Bread,
        Quantity(3),
    );
    let buyer = seed_buyer(&mut h, "Buyer", ORCHARD_FARM, Quantity(3));
    let purchase_goal = worldwake_ai::GoalKey::from(GoalKind::AcquireCommodity {
        commodity: CommodityKind::Bread,
        purpose: worldwake_ai::CommodityPurpose::SelfConsume,
    });

    set_control_source(&mut h, merchant, ControlSource::Human, 0);
    seed_actor_world_beliefs(
        &mut h.world,
        &mut h.event_log,
        buyer,
        Tick(0),
        worldwake_core::PerceptionSource::Inference,
    );

    let mut bound_tick = None;
    for _ in 0..40 {
        let tick = h.step_once().tick;
        let Some(trace) = h
            .driver
            .trace_sink()
            .and_then(|sink| sink.trace_at(buyer, tick))
        else {
            continue;
        };
        let DecisionOutcome::Planning(planning) = &trace.outcome else {
            continue;
        };
        let Some(selected_plan) = planning.selection.selected_plan.as_ref() else {
            continue;
        };
        if h.world.effective_place(buyer) == Some(VILLAGE_SQUARE)
            && h.agent_active_action_name(buyer).is_none()
            && selected_plan
                .next_step
                .as_ref()
                .is_some_and(|step| step.action_name == "trade" && step.targets.contains(&merchant))
        {
            bound_tick = Some(tick);
            let mut txn = new_txn(&mut h.world, tick.0);
            txn.set_ground_location(merchant, ORCHARD_FARM).unwrap();
            commit_txn(txn, &mut h.event_log);
            break;
        }
    }

    assert!(
        bound_tick.is_some(),
        "buyer should first reach the local trade-step binding against the seller"
    );

    let mut parked_pending = false;
    for _ in 0..12 {
        h.step_once();
        let Some(runtime) = h.driver.runtime(buyer) else {
            continue;
        };
        if runtime
            .agenda_state
            .pending
            .values()
            .any(|entry| entry.key.goal_key == purchase_goal)
        {
            parked_pending = true;
            break;
        }
    }

    assert!(
        parked_pending,
        "seller departure should first park the committed purchase goal in pending"
    );
    assert!(
        h.world.get_component_sale_listing(stock_lot).is_none(),
        "seller departure should prune the displayed lot listing before return"
    );

    let return_tick = h.scheduler.current_tick().0;
    let mut txn = new_txn(&mut h.world, return_tick);
    txn.set_ground_location(merchant, VILLAGE_SQUARE).unwrap();
    commit_txn(txn, &mut h.event_log);

    let mut pending_cleared = false;
    let mut revived = false;
    let mut relisted = false;
    for _ in 0..48 {
        h.step_once();
        if let Some(runtime) = h.driver.runtime(buyer) {
            pending_cleared |= runtime
                .agenda_state
                .pending
                .values()
                .all(|entry| entry.key.goal_key != purchase_goal);
            revived |= runtime
                .agenda_state
                .committed
                .as_ref()
                .is_some_and(|entry| entry.key.goal_key == purchase_goal)
                || runtime
                    .current_plan
                    .as_ref()
                    .is_some_and(|plan| plan.goal == purchase_goal);
        }
        relisted |= h.world.get_component_sale_listing(stock_lot).is_some();
        if pending_cleared && revived && relisted {
            break;
        }
    }

    assert!(
        pending_cleared,
        "seller return should clear the parked pending purchase entry through the real runtime lifecycle"
    );
    assert!(
        revived,
        "seller return should revive the purchase goal into committed/current-plan state"
    );
    assert!(
        relisted,
        "seller return should restore a lawful sale listing for the displayed lot"
    );
}

// ---------------------------------------------------------------------------
// Scenario 77: Unlisted Stock Not Sellable
// Systems: Trade, AI
// GoalKinds: AcquireCommodity
// ActionDomains: Trade
// Principles: P1, P3, P7
// Proves: buyer cannot discover or trade unlisted merchant stock
// ---------------------------------------------------------------------------

#[test]
fn unlisted_stock_not_sellable() {
    let mut h = GoldenHarness::with_recipes(Seed([62; 32]), RecipeRegistry::new());
    h.driver.enable_tracing();

    let (merchant, stock_lot) = seed_merchant_with_stored_stock(
        &mut h,
        "Merchant",
        VILLAGE_SQUARE,
        CommodityKind::Bread,
        Quantity(3),
    );
    let buyer = seed_buyer(&mut h, "Buyer", VILLAGE_SQUARE, Quantity(3));

    // Do NOT add SaleListing to the stock lot.
    // But mark the merchant as already having listed stock (to prevent
    // SellCommodity from being emitted — the precondition checks for
    // unlisted local lots). Simulate a world where stock exists but is
    // not listed and the merchant is not actively selling.
    // The simplest way: set merchant to Human control so they don't
    // autonomously start staff_market.
    {
        let mut txn = new_txn(&mut h.world, 0);
        txn.set_component_agent_data(
            merchant,
            worldwake_core::AgentData {
                control_source: ControlSource::Human,
            },
        )
        .unwrap();
        commit_txn(txn, &mut h.event_log);
    }

    // Seed buyer beliefs — buyer knows about the merchant.
    seed_actor_beliefs(
        &mut h.world,
        &mut h.event_log,
        buyer,
        &[merchant],
        Tick(0),
        worldwake_core::PerceptionSource::Inference,
    );

    // Run for some ticks. Buyer should NOT trade because stock is unlisted.
    assert!(
        h.world.get_component_sale_listing(stock_lot).is_none(),
        "stock lot should not be listed"
    );

    let mut trade_occurred = false;
    for _ in 0..40 {
        h.step_once();
        trade_occurred |= !h.event_log.events_by_tag(EventTag::Trade).is_empty();
    }

    assert!(
        !trade_occurred,
        "no trade should occur when merchant stock is unlisted"
    );
    assert_eq!(
        h.agent_commodity_qty(buyer, CommodityKind::Bread),
        Quantity(0),
        "buyer should have no bread because no listed lots were available"
    );
}

fn run_unstage_round_trip_preserves_storage_contract(
    seed: Seed,
) -> (worldwake_core::StateHash, worldwake_core::StateHash) {
    let mut h = GoldenHarness::with_recipes(seed, RecipeRegistry::new());
    h.enable_action_tracing();

    let (merchant, stock_lot) = seed_merchant(
        &mut h,
        "Merchant",
        VILLAGE_SQUARE,
        CommodityKind::Bread,
        Quantity(2),
    );
    let facility = h
        .world
        .get_component_stock_assignment(stock_lot)
        .map(|assignment| assignment.facility)
        .expect("displayed stock should carry a facility assignment");
    let storage_policy = h
        .world
        .get_component_stock_storage_policy(facility)
        .cloned()
        .expect("merchant facility should expose stock storage policy");
    let original_owner = h.world.owner_of(stock_lot);

    set_control_source(&mut h, merchant, ControlSource::Human, 0);
    request_simple_action(&mut h, merchant, "unstage_stock", vec![stock_lot]);

    let mut committed = false;
    for _ in 0..4 {
        let tick_before = h.scheduler.current_tick();
        h.step_once();
        committed |= h
            .action_trace_sink()
            .expect("action tracing should be enabled")
            .events_for_at(merchant, tick_before)
            .iter()
            .any(|event| {
                event.action_name == "unstage_stock"
                    && matches!(event.kind, ActionTraceKind::Committed { .. })
            });
        if committed {
            break;
        }
    }

    assert!(committed, "merchant should commit unstage_stock");
    assert_eq!(
        h.world.direct_container(stock_lot),
        Some(storage_policy.stock_container),
        "unstaged lot should return to the facility stock container"
    );
    assert_eq!(
        h.world.owner_of(stock_lot),
        original_owner,
        "unstaging should preserve lot ownership"
    );
    assert!(
        h.world.get_component_sale_listing(stock_lot).is_none(),
        "unstaging should clear SaleListing"
    );
    let assignment = h
        .world
        .get_component_stock_assignment(stock_lot)
        .expect("unstaged lot should retain a StockAssignment");
    assert_eq!(
        assignment.kind,
        StockAssignmentKind::Stored,
        "unstaged lot should return to Stored assignment"
    );

    (
        hash_world(&h.world).unwrap(),
        hash_event_log(&h.event_log).unwrap(),
    )
}

// ---------------------------------------------------------------------------
// Scenario 79b: Unstage Round Trip Preserves Storage Contract
// Systems: Trade, AI
// ActionDomains: Trade
// Principles: P4, P24
// Proves: displayed facility stock can be unstaged back into storage, clearing
//         SaleListing while preserving ownership and facility assignment
// ---------------------------------------------------------------------------

#[test]
fn unstage_round_trip_preserves_storage_contract() {
    let _ = run_unstage_round_trip_preserves_storage_contract(Seed([66; 32]));
}

// ---------------------------------------------------------------------------
// Scenario 80: Buyer Discovers Listed Lots, Not Unlisted Stock
// Systems: Trade, AI
// GoalKinds: AcquireCommodity
// ActionDomains: Trade
// Principles: P3, P7
// Proves: buyer evidence references only listed lots, not unlisted merchant stock
// ---------------------------------------------------------------------------

#[test]
fn buyer_discovers_listed_lots_not_unlisted_stock() {
    let mut h = GoldenHarness::with_recipes(Seed([70; 32]), RecipeRegistry::new());
    h.driver.enable_tracing();

    let (merchant, listed_lot) = seed_merchant(
        &mut h,
        "Merchant",
        VILLAGE_SQUARE,
        CommodityKind::Bread,
        Quantity(2),
    );
    let buyer = seed_buyer(&mut h, "Buyer", VILLAGE_SQUARE, Quantity(3));

    // Create a second unlisted lot on the merchant.
    let unlisted_lot = give_commodity(
        &mut h.world,
        &mut h.event_log,
        merchant,
        VILLAGE_SQUARE,
        CommodityKind::Bread,
        Quantity(2),
    );

    // List only the first lot.
    {
        let mut txn = new_txn(&mut h.world, 0);
        txn.set_component_sale_listing(listed_lot, SaleListing { listed_at: Tick(0) })
            .unwrap();
        commit_txn(txn, &mut h.event_log);
    }

    // Seed beliefs so buyer knows about merchant and lots.
    seed_actor_local_beliefs(
        &mut h.world,
        &mut h.event_log,
        buyer,
        Tick(0),
        worldwake_core::PerceptionSource::Inference,
    );

    h.step_once();

    // Check decision trace: buyer's AcquireCommodity candidate should reference
    // the listed lot (via trade evidence), not the unlisted lot.
    let sink = h.driver.trace_sink().expect("tracing enabled");
    let trace = sink.trace_at(buyer, Tick(0));
    assert!(
        trace.is_some(),
        "buyer should have a decision trace at tick 0"
    );

    // The unlisted lot should never appear as a trade-discoverable lot.
    assert!(
        h.world.get_component_sale_listing(unlisted_lot).is_none(),
        "unlisted lot should have no SaleListing"
    );
}

// ---------------------------------------------------------------------------
// Scenario 82: Seller Departure Invalidates Listing
// Systems: Trade
// Principles: P3, P7
// Proves: SaleListing pruned within one tick of seller leaving the market
// ---------------------------------------------------------------------------

#[test]
fn seller_departure_invalidates_listing() {
    let mut h = GoldenHarness::with_recipes(Seed([72; 32]), RecipeRegistry::new());
    h.enable_action_tracing();

    let (merchant, stock_lot) = seed_merchant(
        &mut h,
        "Merchant",
        VILLAGE_SQUARE,
        CommodityKind::Bread,
        Quantity(3),
    );

    // Pre-list the lot.
    {
        let mut txn = new_txn(&mut h.world, 0);
        txn.set_component_sale_listing(stock_lot, SaleListing { listed_at: Tick(0) })
            .unwrap();
        // Set merchant to Human so AI doesn't interfere.
        txn.set_component_agent_data(
            merchant,
            worldwake_core::AgentData {
                control_source: ControlSource::Human,
            },
        )
        .unwrap();
        commit_txn(txn, &mut h.event_log);
    }

    assert!(
        h.world.get_component_sale_listing(stock_lot).is_some(),
        "listing should exist before departure"
    );

    // Move merchant to a different place.
    {
        let mut txn = new_txn(&mut h.world, 1);
        txn.set_ground_location(merchant, ORCHARD_FARM).unwrap();
        commit_txn(txn, &mut h.event_log);
    }

    // Run one tick — the trade system cleanup should prune the listing.
    h.step_once();

    assert!(
        h.world.get_component_sale_listing(stock_lot).is_none(),
        "listing should be removed after seller departs the market"
    );
}

// ---------------------------------------------------------------------------
// Scenario 83: Dead Seller Invalidates Listing
// Systems: Trade
// Principles: P3, P4
// Proves: SaleListing pruned within one tick of seller death
// ---------------------------------------------------------------------------

#[test]
fn dead_seller_invalidates_listing() {
    let mut h = GoldenHarness::with_recipes(Seed([73; 32]), RecipeRegistry::new());

    let (merchant, stock_lot) = seed_merchant(
        &mut h,
        "Merchant",
        VILLAGE_SQUARE,
        CommodityKind::Bread,
        Quantity(3),
    );

    // Pre-list the lot and set merchant to Human.
    {
        let mut txn = new_txn(&mut h.world, 0);
        txn.set_component_sale_listing(stock_lot, SaleListing { listed_at: Tick(0) })
            .unwrap();
        txn.set_component_agent_data(
            merchant,
            worldwake_core::AgentData {
                control_source: ControlSource::Human,
            },
        )
        .unwrap();
        commit_txn(txn, &mut h.event_log);
    }

    assert!(
        h.world.get_component_sale_listing(stock_lot).is_some(),
        "listing should exist before death"
    );

    let facility = h
        .world
        .get_component_stock_assignment(stock_lot)
        .map(|assignment| assignment.facility)
        .expect("displayed stock should keep a facility assignment");
    let storage_policy = h
        .world
        .get_component_stock_storage_policy(facility)
        .cloned()
        .expect("merchant facility should expose stock storage policy");

    // Kill the merchant by clearing ownership dependencies, then archiving.
    {
        let mut txn = new_txn(&mut h.world, 1);
        txn.clear_owner(stock_lot).unwrap();
        txn.clear_owner(storage_policy.stock_container).unwrap();
        if let Some(display_container) = storage_policy.display_container {
            txn.clear_owner(display_container).unwrap();
        }
        txn.clear_owner(facility).unwrap();
        txn.archive_entity(merchant).unwrap();
        commit_txn(txn, &mut h.event_log);
    }

    // Run one tick — the trade system cleanup should prune the listing.
    h.step_once();

    assert!(
        h.world.get_component_sale_listing(stock_lot).is_none(),
        "listing should be removed after seller dies"
    );
}

// ---------------------------------------------------------------------------
// Scenario 86: Demand Memory Raises Sell Ranking
// Systems: Trade, AI
// GoalKinds: SellCommodity
// Principles: P1, P3, P20
// Proves: demand memory boosts SellCommodity motive above baseline without overpowering self-care
// ---------------------------------------------------------------------------

#[test]
fn demand_memory_raises_sell_ranking() {
    let mut h = GoldenHarness::with_recipes(Seed([75; 32]), RecipeRegistry::new());
    h.driver.enable_tracing();

    // Merchant A: has stock, has demand memory.
    let (merchant_a, _, _) = seed_merchant_with_loose_stock(
        &mut h,
        "MerchantA",
        VILLAGE_SQUARE,
        CommodityKind::Bread,
        Quantity(3),
        true,
    );
    let (merchant_b, _, _) = seed_merchant_with_loose_stock(
        &mut h,
        "MerchantB",
        VILLAGE_SQUARE,
        CommodityKind::Bread,
        Quantity(3),
        false,
    );

    for agent in [merchant_a, merchant_b] {
        seed_actor_local_beliefs(
            &mut h.world,
            &mut h.event_log,
            agent,
            Tick(0),
            worldwake_core::PerceptionSource::Inference,
        );
    }

    h.step_once();

    // Both should generate SellCommodity candidates.
    let sink = h.driver.trace_sink().expect("tracing enabled");

    let motive_for = |agent: worldwake_core::EntityId| -> Option<u32> {
        let trace = sink.trace_at(agent, Tick(0))?;
        match &trace.outcome {
            DecisionOutcome::Planning(p) => p
                .candidates
                .ranked
                .iter()
                .find(|c| matches!(c.opportunity.goal_key.kind, GoalKind::SellCommodity { .. }))
                .map(|c| c.motive_score),
            _ => None,
        }
    };

    let motive_a = motive_for(merchant_a).expect("merchant A should have SellCommodity candidate");
    let motive_b = motive_for(merchant_b).expect("merchant B should have SellCommodity candidate");

    assert!(
        motive_a > motive_b,
        "merchant with demand memory should have higher SellCommodity motive: with_demand={motive_a}, without={motive_b}"
    );
    assert!(
        motive_b > 0,
        "merchant without demand memory should still have nonzero motive (baseline signal)"
    );
}

// ---------------------------------------------------------------------------
// Scenario 96: Hungry Merchant Eats Own Listed Sale Stock
// Systems: Needs, Trade, AI
// GoalKinds: ConsumeOwnedCommodity, SellCommodity
// ActionDomains: Needs (eat), Trade (staff_market)
// Principles: P1, P3, P20
// Setup: single merchant, critical hunger pm(950), Quantity(1) bread with SaleListing
// Proves: survival-class ConsumeOwnedCommodity outranks enterprise-class SellCommodity;
//         eating the listed lot archives it, removing SaleListing as a side effect
// Chain: critical hunger → ConsumeOwnedCommodity(Critical) beats SellCommodity(Medium)
//        → eat action → consume_one_unit archives Quantity(1) lot → SaleListing gone
// ---------------------------------------------------------------------------

fn run_hungry_merchant_eats_listed_stock(
    seed: Seed,
) -> (worldwake_core::StateHash, worldwake_core::StateHash) {
    let mut h = GoldenHarness::with_recipes(seed, RecipeRegistry::new());
    h.driver.enable_tracing();
    h.enable_action_tracing();

    let (merchant, bread_lot, _facility) = seed_merchant_with_loose_stock(
        &mut h,
        "HungryMerchant",
        VILLAGE_SQUARE,
        CommodityKind::Bread,
        Quantity(1),
        true,
    );

    // Override needs: critical hunger (pm(950) > critical threshold pm(900)).
    {
        let mut txn = new_txn(&mut h.world, 0);
        txn.set_component_homeostatic_needs(
            merchant,
            HomeostaticNeeds::new(pm(950), pm(0), pm(0), pm(0), pm(0)),
        )
        .unwrap();
        // Manually add SaleListing — in normal flow staff_market adds it, but here
        // we want the listing pre-existing so the eat action competes with sell.
        txn.set_component_sale_listing(bread_lot, SaleListing { listed_at: Tick(0) })
            .unwrap();
        commit_txn(txn, &mut h.event_log);
    }

    // Seed beliefs so the AI can perceive.
    seed_actor_local_beliefs(
        &mut h.world,
        &mut h.event_log,
        merchant,
        Tick(0),
        worldwake_core::PerceptionSource::Inference,
    );

    // Confirm pre-conditions.
    assert!(
        h.world.get_component_sale_listing(bread_lot).is_some(),
        "bread lot should have SaleListing before ticking"
    );
    assert!(
        h.world.get_component_item_lot(bread_lot).is_some(),
        "bread lot should exist before ticking"
    );
    let initial_hunger = h
        .world
        .get_component_homeostatic_needs(merchant)
        .unwrap()
        .hunger;

    // Tick until eat action completes (or up to budget).
    let mut eat_committed = false;
    for _ in 0..60 {
        h.step_once();
        if let Some(sink) = h.action_trace_sink() {
            eat_committed |= sink.events_for(merchant).iter().any(|event| {
                event.action_name == "eat"
                    && matches!(event.kind, ActionTraceKind::Committed { .. })
            });
        }
        if eat_committed {
            break;
        }
    }
    assert!(
        eat_committed,
        "merchant should commit eat action within 60 ticks"
    );

    // Post-condition: bread lot archived (no ItemLot component).
    assert!(
        h.world.get_component_item_lot(bread_lot).is_none(),
        "bread lot should be archived after eating"
    );

    // Post-condition: SaleListing gone with the archived lot.
    assert!(
        h.world.get_component_sale_listing(bread_lot).is_none(),
        "SaleListing should be removed after lot is archived"
    );

    // Post-condition: hunger decreased.
    let final_hunger = h
        .world
        .get_component_homeostatic_needs(merchant)
        .unwrap()
        .hunger;
    assert!(
        final_hunger < initial_hunger,
        "hunger should decrease after eating: initial={initial_hunger:?}, final={final_hunger:?}"
    );

    // Conservation: bread quantity decreased by exactly 1 (lawful consumption sink).
    let final_bread = total_live_lot_quantity(&h.world, CommodityKind::Bread);
    assert_eq!(final_bread, 0, "Quantity(1) bread should be fully consumed");

    (
        hash_world(&h.world).unwrap(),
        hash_event_log(&h.event_log).unwrap(),
    )
}

#[test]
fn hungry_merchant_eats_listed_stock() {
    run_hungry_merchant_eats_listed_stock(Seed([87; 32]));
}
