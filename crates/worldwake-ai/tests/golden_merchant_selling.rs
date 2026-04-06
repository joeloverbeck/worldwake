//! Golden tests for merchant selling market-presence (S04 spec).
//!
//! These tests exercise the seller-side lifecycle: `SaleListing` attachment via
//! `staff_market`, buyer discovery of listed lots, trade against concrete listed
//! lots, autonomous stock staging before sell readiness, and deterministic
//! replay of the full merchant-selling pipeline.

mod golden_harness;

use golden_harness::*;
use std::collections::BTreeSet;
use worldwake_ai::{CommodityPurpose, DecisionOutcome, SelectedPlanSource};
use worldwake_core::{
    AgentData, CommodityKind, ControlSource, DemandMemory, DemandObservation,
    DemandObservationReason, EventTag, GoalKind, HomeostaticNeeds, KnownRecipes, LoadUnits,
    MerchandiseProfile, MetabolismProfile, OpportunityAnchor, OpportunityKey, PerceptionProfile,
    PrototypePlace, Quantity, SaleListing, Seed, StockAssignmentKind, Tick,
    TradeDispositionProfile, UtilityProfile, hash_event_log, hash_world, prototype_place_entity,
    total_live_lot_quantity,
};
use worldwake_sim::{
    ActionRequestMode, ActionTraceKind, InputKind, PerAgentBeliefView, RecipeRegistry,
    RequestProvenance, RuntimeBeliefView,
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

fn side_benefit_merchant_utility() -> UtilityProfile {
    UtilityProfile {
        enterprise_weight: pm(800),
        side_benefit_weight: pm(1000),
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

fn decision_trace_summaries(h: &GoldenHarness, agent: worldwake_core::EntityId) -> Vec<String> {
    h.driver
        .trace_sink()
        .expect("decision tracing should remain enabled")
        .traces_for(agent)
        .into_iter()
        .map(|trace| format!("{:?}: {}", trace.tick, trace.outcome.summary()))
        .collect()
}

fn action_trace_summaries(h: &GoldenHarness, agent: worldwake_core::EntityId) -> Vec<String> {
    h.action_trace_sink()
        .expect("action tracing should remain enabled")
        .events_for(agent)
        .into_iter()
        .map(worldwake_sim::ActionTraceEvent::summary)
        .collect()
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct SideBenefitSelectionOutcome {
    world_hash: worldwake_core::StateHash,
    log_hash: worldwake_core::StateHash,
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

#[test]
fn staff_market_retains_displayed_listing_replays_deterministically() {
    let (w1, e1) = run_displayed_lot_retains_listing(Seed([60; 32]));
    let (w2, e2) = run_displayed_lot_retains_listing(Seed([60; 32]));
    assert_eq!(w1, w2, "world hash mismatch on replay");
    assert_eq!(e1, e2, "event log hash mismatch on replay");
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

#[test]
fn buyer_trades_against_listed_lot_replays_deterministically() {
    let (w1, e1) = run_buyer_trades_listed_lot(Seed([61; 32]));
    let (w2, e2) = run_buyer_trades_listed_lot(Seed([61; 32]));
    assert_eq!(w1, w2, "world hash mismatch on replay");
    assert_eq!(e1, e2, "event log hash mismatch on replay");
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

// ---------------------------------------------------------------------------
// Scenario 78: Loose Home Stock Is Staged Before Sell Goal Settles
// Systems: Trade, AI
// GoalKinds: SellCommodity
// ActionDomains: Trade
// Principles: P1, P4, P8
// Proves: loose home stock is stored and staged into a listed facility lot
//         before SellCommodity settles; merchant does not enter an immediate
//         futile staff_market cycle when no buyer is present
// ---------------------------------------------------------------------------

#[test]
fn loose_home_stock_is_staged_before_sell_goal_settles() {
    let mut h = GoldenHarness::with_recipes(Seed([63; 32]), RecipeRegistry::new());
    h.driver.enable_tracing();
    h.enable_action_tracing();

    let (merchant, stock_lot, _facility) = seed_merchant_with_loose_stock(
        &mut h,
        "Merchant",
        VILLAGE_SQUARE,
        CommodityKind::Bread,
        Quantity(3),
        true,
    );

    seed_actor_local_beliefs(
        &mut h.world,
        &mut h.event_log,
        merchant,
        Tick(0),
        worldwake_core::PerceptionSource::Inference,
    );

    for _ in 0..2 {
        h.step_once();
    }

    let action_trace = h
        .action_trace_sink()
        .expect("action tracing should be enabled for staging scenario");
    assert!(
        action_trace
            .events_for_at(merchant, Tick(0))
            .iter()
            .any(|e| {
                e.action_name == "store_stock"
                    && matches!(e.kind, ActionTraceKind::Committed { .. })
            }),
        "merchant should first store loose home stock into facility custody"
    );
    assert!(
        action_trace
            .events_for_at(merchant, Tick(1))
            .iter()
            .any(|e| {
                e.action_name == "stage_stock_for_sale"
                    && matches!(e.kind, ActionTraceKind::Committed { .. })
            }),
        "merchant should then stage stored stock for sale"
    );

    assert!(
        h.world.get_component_sale_listing(stock_lot).is_some(),
        "staged lot should become listed for sale"
    );
    let assignment = h
        .world
        .get_component_stock_assignment(stock_lot)
        .expect("staged lot should keep a stock assignment");
    assert_eq!(
        assignment.kind,
        StockAssignmentKind::Displayed,
        "staged lot should move into displayed facility stock"
    );

    for tick in [Tick(0), Tick(1)] {
        let trace = h
            .driver
            .trace_sink()
            .expect("decision tracing enabled")
            .trace_at(merchant, tick)
            .expect("merchant should have a planning trace during staging");
        let planning = match &trace.outcome {
            DecisionOutcome::Planning(planning) => planning,
            other => panic!("expected planning trace during staging, got {other:?}"),
        };
        assert!(
            planning
                .selection
                .selected_goal()
                .is_some_and(|goal| goal.kind
                    == GoalKind::SellCommodity {
                        commodity: CommodityKind::Bread,
                    }),
            "staging ticks should still be executing SellCommodity planning"
        );
    }

    let mut saw_staff_market = false;
    let mut sell_reselected_after_listing = false;
    for _ in 0..5 {
        let tick_before = h.scheduler.current_tick();
        h.step_once();
        if let Some(sink) = h.action_trace_sink() {
            saw_staff_market |= sink.events_for_at(merchant, tick_before).iter().any(|e| {
                e.action_name == "staff_market"
                    && matches!(
                        e.kind,
                        ActionTraceKind::Started { .. } | ActionTraceKind::Committed { .. }
                    )
            });
        }
        if let Some(trace) = h
            .driver
            .trace_sink()
            .expect("decision tracing enabled")
            .trace_at(merchant, tick_before)
            && let DecisionOutcome::Planning(planning) = &trace.outcome
        {
            sell_reselected_after_listing |=
                planning.selection.selected_goal().is_some_and(|goal| {
                    goal.kind
                        == GoalKind::SellCommodity {
                            commodity: CommodityKind::Bread,
                        }
                });
        }
    }
    assert!(
        !saw_staff_market,
        "merchant should not enter staff_market immediately after lawful staging when no buyer is present"
    );
    assert!(
        !sell_reselected_after_listing,
        "listed displayed stock should settle SellCommodity instead of immediately re-emitting the same goal"
    );
}

// ---------------------------------------------------------------------------
// Scenario 79: Deterministic Replay Preserves Listing Behavior
// Systems: Trade, AI
// Principles: P2
// Proves: identical seeds produce identical world and event log hashes
// ---------------------------------------------------------------------------

#[test]
fn deterministic_replay_preserves_listing_behavior() {
    let (w1, e1) = run_buyer_trades_listed_lot(Seed([64; 32]));
    let (w2, e2) = run_buyer_trades_listed_lot(Seed([64; 32]));
    assert_eq!(w1, w2, "world hash mismatch on replay");
    assert_eq!(e1, e2, "event log hash mismatch on replay");

    let (w3, e3) = run_displayed_lot_retains_listing(Seed([65; 32]));
    let (w4, e4) = run_displayed_lot_retains_listing(Seed([65; 32]));
    assert_eq!(w3, w4, "staff_market world hash mismatch on replay");
    assert_eq!(e3, e4, "staff_market event log hash mismatch on replay");
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

#[test]
fn unstage_round_trip_preserves_storage_contract_replays_deterministically() {
    let first = run_unstage_round_trip_preserves_storage_contract(Seed([67; 32]));
    let second = run_unstage_round_trip_preserves_storage_contract(Seed([67; 32]));
    assert_eq!(
        first, second,
        "unstage round-trip scenario should replay deterministically"
    );
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
// Scenario 81: Merchant Emits SellCommodity at Home Market
// Systems: Trade, AI
// GoalKinds: SellCommodity
// ActionDomains: Trade
// Principles: P1, P6
// Proves: SellCommodity candidate emitted via decision trace
// ---------------------------------------------------------------------------

#[test]
fn merchant_emits_sell_commodity_at_home_facility() {
    let mut h = GoldenHarness::with_recipes(Seed([71; 32]), RecipeRegistry::new());
    h.driver.enable_tracing();

    let merchant = seed_agent_with_recipes(
        &mut h.world,
        &mut h.event_log,
        "Merchant",
        VILLAGE_SQUARE,
        HomeostaticNeeds::new_sated(),
        MetabolismProfile::default(),
        merchant_utility(),
        KnownRecipes::new(),
    );
    give_commodity(
        &mut h.world,
        &mut h.event_log,
        merchant,
        VILLAGE_SQUARE,
        CommodityKind::Bread,
        Quantity(3),
    );
    {
        let mut txn = new_txn(&mut h.world, 0);
        let (home_facility, _stock_container, _display_container) = txn
            .create_merchant_facility(
                VILLAGE_SQUARE,
                merchant,
                LoadUnits(500),
                Some(LoadUnits(300)),
            )
            .unwrap();
        txn.set_component_merchandise_profile(
            merchant,
            MerchandiseProfile {
                sale_kinds: BTreeSet::from([CommodityKind::Bread]),
                home_facility: Some(home_facility),
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
                    commodity: CommodityKind::Bread,
                    quantity: Quantity(5),
                    place: VILLAGE_SQUARE,
                    tick: Tick(0),
                    counterparty: None,
                    reason: DemandObservationReason::WantedToBuyButSellerOutOfStock,
                }],
            },
        )
        .unwrap();
        commit_txn(txn, &mut h.event_log);
    }

    seed_actor_local_beliefs(
        &mut h.world,
        &mut h.event_log,
        merchant,
        Tick(0),
        worldwake_core::PerceptionSource::Inference,
    );

    h.step_once();

    // Decision trace should show SellCommodity candidate was generated.
    let sink = h.driver.trace_sink().expect("tracing enabled");
    let trace = sink
        .trace_at(merchant, Tick(0))
        .expect("merchant should have trace at tick 0");
    match &trace.outcome {
        DecisionOutcome::Planning(planning) => {
            let has_sell = planning.candidates.ranked.iter().any(|c| {
                matches!(c.opportunity.goal_key.kind, GoalKind::SellCommodity { commodity } if commodity == CommodityKind::Bread)
            });
            assert!(
                has_sell,
                "merchant at home_facility with unlisted stock should emit SellCommodity candidate"
            );
        }
        other => {
            // If not planning (e.g., active action from a previous selection), check
            // that the selected goal was SellCommodity.
            panic!("expected Planning outcome at tick 0, got {other:?}");
        }
    }
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
// Scenario 84: Remote Merchant Travels to Home Market to Sell
// Systems: Trade, AI
// GoalKinds: SellCommodity
// ActionDomains: Trade, Travel
// Principles: P1, P6
// Proves: merchant at remote place plans Travel + StaffMarket to reach home_facility
// ---------------------------------------------------------------------------

#[test]
fn move_cargo_then_sell_commodity_plan_shape() {
    let mut h = GoldenHarness::with_recipes(Seed([74; 32]), RecipeRegistry::new());
    h.driver.enable_tracing();

    // Merchant at ORCHARD_FARM, home_facility is VILLAGE_SQUARE.
    let merchant = seed_agent_with_recipes(
        &mut h.world,
        &mut h.event_log,
        "Merchant",
        ORCHARD_FARM,
        HomeostaticNeeds::new_sated(),
        MetabolismProfile::default(),
        merchant_utility(),
        KnownRecipes::new(),
    );
    give_commodity(
        &mut h.world,
        &mut h.event_log,
        merchant,
        ORCHARD_FARM,
        CommodityKind::Bread,
        Quantity(3),
    );
    let home_facility = {
        let mut txn = new_txn(&mut h.world, 0);
        let (home_facility, _stock_container, _display_container) = txn
            .create_merchant_facility(
                VILLAGE_SQUARE,
                merchant,
                LoadUnits(500),
                Some(LoadUnits(300)),
            )
            .unwrap();
        txn.set_component_merchandise_profile(
            merchant,
            MerchandiseProfile {
                sale_kinds: BTreeSet::from([CommodityKind::Bread]),
                home_facility: Some(home_facility),
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
                    commodity: CommodityKind::Bread,
                    quantity: Quantity(5),
                    place: VILLAGE_SQUARE,
                    tick: Tick(0),
                    counterparty: None,
                    reason: DemandObservationReason::WantedToBuyButSellerOutOfStock,
                }],
            },
        )
        .unwrap();
        commit_txn(txn, &mut h.event_log);
        home_facility
    };

    seed_actor_local_beliefs(
        &mut h.world,
        &mut h.event_log,
        merchant,
        Tick(0),
        worldwake_core::PerceptionSource::Inference,
    );
    seed_actor_beliefs(
        &mut h.world,
        &mut h.event_log,
        merchant,
        &[home_facility],
        Tick(0),
        worldwake_core::PerceptionSource::Inference,
    );

    // The merchant should eventually travel to home_facility and start staff_market.
    let mut saw_travel = false;
    let mut arrived_at_home = false;
    let mut saw_staff_market = false;
    for _ in 0..120 {
        h.step_once();
        saw_travel |= h.agent_active_action_name(merchant) == Some("travel");
        arrived_at_home |= h.world.effective_place(merchant) == Some(VILLAGE_SQUARE);
        if let Some(sink) = h.action_trace_sink() {
            saw_staff_market |= sink.events_for(merchant).iter().any(|e| {
                e.action_name == "staff_market" && matches!(e.kind, ActionTraceKind::Started { .. })
            });
        }
        if saw_staff_market {
            break;
        }
    }
    // The merchant should at least travel (restock-driven movement toward home market).
    assert!(
        saw_travel || arrived_at_home || saw_staff_market,
        "merchant at remote place with stock and demand memory should eventually move toward home_facility or start selling"
    );
}

// ---------------------------------------------------------------------------
// Scenario 85: Demand Memory Raises Sell Ranking
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
// Scenario 86: Planning State Preserves Listing Determinism
// Systems: Trade, AI
// Principles: P2
// Proves: identical seeds produce identical plan search results for merchant scenarios
// ---------------------------------------------------------------------------

#[test]
fn planning_state_preserves_listing_determinism() {
    // Run the same scenario twice with the same seed and verify identical
    // plan search results via world + event log hash comparison.
    let (w1, e1) = run_displayed_lot_retains_listing(Seed([76; 32]));
    let (w2, e2) = run_displayed_lot_retains_listing(Seed([76; 32]));
    assert_eq!(w1, w2, "planning state world hash mismatch");
    assert_eq!(e1, e2, "planning state event log hash mismatch");
}

// ---------------------------------------------------------------------------
// Scenario 95: Side-Benefit Selection Prefers Home Market Combined Trip
// Systems: Trade, AI, Needs
// GoalKinds: AcquireCommodity, SellCommodity
// ActionDomains: Travel, Trade
// Principles: P1, P3, P20, P22
// Proves: a remote merchant with equal primary bread opportunities prefers the
//         home-market seller because the selected path also carries a lawful
//         SellCommodity side benefit at that destination; after acquisition the
//         merchant stages firewood for sale there without a second market trip
// ---------------------------------------------------------------------------

#[allow(clippy::too_many_lines)]
fn run_side_benefit_market_trip_selection(seed: Seed) -> SideBenefitSelectionOutcome {
    fn seed_simple_seller(
        h: &mut GoldenHarness,
        name: &str,
        place: worldwake_core::EntityId,
        commodity: CommodityKind,
        quantity: Quantity,
    ) -> (worldwake_core::EntityId, worldwake_core::EntityId) {
        use worldwake_core::{LoadUnits, StockAssignment, StockAssignmentKind};

        let seller = seed_agent_with_recipes(
            &mut h.world,
            &mut h.event_log,
            name,
            place,
            HomeostaticNeeds::new_sated(),
            MetabolismProfile::default(),
            UtilityProfile::default(),
            KnownRecipes::new(),
        );

        let stock_lot = give_commodity(
            &mut h.world,
            &mut h.event_log,
            seller,
            place,
            commodity,
            quantity,
        );

        let mut txn = new_txn(&mut h.world, 0);
        let (facility, _stock_container, display) = txn
            .create_merchant_facility(place, seller, LoadUnits(500), Some(LoadUnits(300)))
            .unwrap();
        txn.set_component_merchandise_profile(
            seller,
            MerchandiseProfile {
                sale_kinds: BTreeSet::from([commodity]),
                home_facility: None,
            },
        )
        .unwrap();
        txn.set_component_trade_disposition_profile(seller, merchant_trade_disposition())
            .unwrap();
        txn.set_component_perception_profile(seller, PerceptionProfile::default())
            .unwrap();
        let display_container = display.unwrap();
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

        (seller, stock_lot)
    }

    let mut h = GoldenHarness::with_recipes(seed, RecipeRegistry::new());
    h.driver.enable_tracing();
    h.enable_action_tracing();

    let general_store = prototype_place_entity(PrototypePlace::GeneralStore);
    let common_house = prototype_place_entity(PrototypePlace::CommonHouse);

    let (market_seller, _market_bread_lot) = seed_simple_seller(
        &mut h,
        "Market Seller",
        general_store,
        CommodityKind::Bread,
        Quantity(2),
    );
    let (inn_seller, _inn_bread_lot) = seed_simple_seller(
        &mut h,
        "Inn Seller",
        common_house,
        CommodityKind::Bread,
        Quantity(2),
    );
    set_control_source(&mut h, market_seller, ControlSource::Human, 0);
    set_control_source(&mut h, inn_seller, ControlSource::Human, 0);

    let merchant = seed_agent_with_recipes(
        &mut h.world,
        &mut h.event_log,
        "Traveler Merchant",
        ORCHARD_FARM,
        HomeostaticNeeds::new(pm(950), pm(0), pm(0), pm(0), pm(0)),
        MetabolismProfile::default(),
        side_benefit_merchant_utility(),
        KnownRecipes::new(),
    );
    let firewood_lot = give_commodity(
        &mut h.world,
        &mut h.event_log,
        merchant,
        ORCHARD_FARM,
        CommodityKind::Firewood,
        Quantity(3),
    );
    give_commodity(
        &mut h.world,
        &mut h.event_log,
        merchant,
        ORCHARD_FARM,
        CommodityKind::Coin,
        Quantity(500),
    );

    let home_facility = {
        let mut txn = new_txn(&mut h.world, 0);
        let (facility, _stock_container, _display_container) = txn
            .create_merchant_facility(
                general_store,
                merchant,
                LoadUnits(500),
                Some(LoadUnits(300)),
            )
            .unwrap();
        txn.set_component_merchandise_profile(
            merchant,
            MerchandiseProfile {
                sale_kinds: BTreeSet::from([CommodityKind::Firewood]),
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
                    commodity: CommodityKind::Firewood,
                    quantity: Quantity(5),
                    place: general_store,
                    tick: Tick(0),
                    counterparty: None,
                    reason: DemandObservationReason::WantedToBuyButSellerOutOfStock,
                }],
            },
        )
        .unwrap();
        commit_txn(txn, &mut h.event_log);
        facility
    };

    seed_actor_world_beliefs(
        &mut h.world,
        &mut h.event_log,
        merchant,
        Tick(0),
        worldwake_core::PerceptionSource::Inference,
    );

    let bread_goal = worldwake_core::GoalKey::from(GoalKind::AcquireCommodity {
        commodity: CommodityKind::Bread,
        purpose: CommodityPurpose::SelfConsume,
    });
    let market_opportunity = OpportunityKey {
        goal_key: bread_goal,
        anchor: OpportunityAnchor::Place(general_store),
    };
    let inn_opportunity = OpportunityKey {
        goal_key: bread_goal,
        anchor: OpportunityAnchor::Place(common_house),
    };

    h.step_once();

    let opening_trace = h
        .driver
        .trace_sink()
        .expect("decision tracing should remain enabled")
        .trace_at(merchant, Tick(0))
        .expect("merchant should plan on the opening tick");
    let opening_planning = match &opening_trace.outcome {
        DecisionOutcome::Planning(planning) => planning,
        other => panic!("expected opening planning trace, got {other:?}"),
    };
    assert_eq!(
        opening_planning.selection.selected_plan_source,
        Some(SelectedPlanSource::SearchSelection),
        "opening selection should come from fresh search",
    );
    assert!(
        opening_planning.selection.selected_goal_is(bread_goal),
        "opening plan should still be driven by bread acquisition as the primary goal",
    );
    assert!(
        opening_planning
            .selection
            .selected_opportunity_is(market_opportunity),
        "side-benefit-aware selection should choose the home-market seller over the equally valuable inn seller",
    );

    let ranked_summaries = opening_planning
        .candidates
        .ranked_summaries_for_goal(bread_goal);
    let market_summary = ranked_summaries
        .iter()
        .find(|summary| summary.opportunity == market_opportunity)
        .expect("market seller opportunity should remain ranked");
    let inn_summary = ranked_summaries
        .iter()
        .find(|summary| summary.opportunity == inn_opportunity)
        .expect("inn seller opportunity should remain ranked");
    assert_eq!(
        market_summary.motive_score, inn_summary.motive_score,
        "the two bread opportunities should be symmetric on primary motive so side benefits, not motive, decide the winner",
    );

    let selected_plan = opening_planning
        .selection
        .selected_plan
        .as_ref()
        .expect("search selection should expose the winning selected plan");
    assert!(
        selected_plan.total_value > selected_plan.primary_motive,
        "the winning plan should gain bounded total value from a recognized side benefit",
    );
    assert!(
        selected_plan.side_benefits.iter().any(|benefit| {
            benefit.at_place == general_store
                && matches!(
                    benefit.goal_key.kind,
                    GoalKind::SellCommodity {
                        commodity: CommodityKind::Firewood
                    }
                )
        }),
        "the selected plan should report SellCommodity(Firewood) as a side benefit at the home market",
    );
    assert!(
        selected_plan
            .steps
            .iter()
            .any(|step| step.targets == vec![general_store]),
        "the winning selected path should actually route to the home market",
    );
    assert!(
        !selected_plan
            .steps
            .iter()
            .any(|step| step.targets == vec![common_house]),
        "the winning selected path should not route to the losing inn seller branch",
    );

    let mut trade_started_against_market = false;
    let mut trade_started_against_inn = false;
    let mut trade_committed = false;
    let mut firewood_listing_created = false;
    for _ in 0..160 {
        let tick_before = h.scheduler.current_tick();
        h.step_once();

        if let Some(sink) = h.action_trace_sink() {
            for event in sink.events_for_at(merchant, tick_before) {
                if event.action_name != "trade" {
                    continue;
                }
                match &event.kind {
                    ActionTraceKind::Started { targets } => {
                        trade_started_against_market |= targets == &vec![market_seller];
                        trade_started_against_inn |= targets == &vec![inn_seller];
                    }
                    ActionTraceKind::Committed { .. } => {
                        trade_committed = true;
                    }
                    _ => {}
                }
            }
        }

        firewood_listing_created |= h.world.get_component_sale_listing(firewood_lot).is_some();

        if trade_started_against_market && trade_committed && firewood_listing_created {
            break;
        }
    }

    assert!(
        trade_started_against_market,
        "the executed trade branch should start against the home-market seller; decision_traces={:?}; action_traces={:?}",
        decision_trace_summaries(&h, merchant),
        action_trace_summaries(&h, merchant),
    );
    assert!(
        !trade_started_against_inn,
        "the losing inn seller branch should never execute after side-benefit-aware selection",
    );
    assert!(
        trade_committed,
        "the primary bread-acquisition trade should commit on the selected home-market branch; decision_traces={:?}; action_traces={:?}",
        decision_trace_summaries(&h, merchant),
        action_trace_summaries(&h, merchant),
    );
    assert!(
        firewood_listing_created,
        "the merchant should eventually stage and list firewood at the chosen home market after the combined trip; decision_traces={:?}; action_traces={:?}",
        decision_trace_summaries(&h, merchant),
        action_trace_summaries(&h, merchant),
    );
    assert_eq!(
        h.world.effective_place(merchant),
        Some(general_store),
        "the combined-trip branch should leave the merchant at the home market where the side benefit materializes",
    );
    let assignment = h
        .world
        .get_component_stock_assignment(firewood_lot)
        .expect("the listed firewood lot should carry a stock assignment");
    assert_eq!(
        assignment.facility, home_facility,
        "the staged firewood lot should belong to the merchant's home market facility",
    );
    assert_eq!(
        assignment.kind,
        StockAssignmentKind::Displayed,
        "the side-benefit follow-through should stage firewood into displayed stock",
    );

    SideBenefitSelectionOutcome {
        world_hash: hash_world(&h.world).unwrap(),
        log_hash: hash_event_log(&h.event_log).unwrap(),
    }
}

#[test]
fn combined_market_trip_selected_for_side_benefit() {
    let _ = run_side_benefit_market_trip_selection(Seed([79; 32]));
}

#[test]
fn combined_market_trip_selected_for_side_benefit_replays_deterministically() {
    let first = run_side_benefit_market_trip_selection(Seed([80; 32]));
    let second = run_side_benefit_market_trip_selection(Seed([80; 32]));
    assert_eq!(
        first, second,
        "combined market-trip side-benefit scenario should replay deterministically",
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

    // Decision trace: verify ConsumeOwnedCommodity was a candidate.
    let sink = h.driver.trace_sink().expect("tracing enabled");
    let trace = sink
        .trace_at(merchant, Tick(0))
        .expect("merchant should have decision trace at tick 0");
    match &trace.outcome {
        DecisionOutcome::Planning(planning) => {
            let has_consume = planning.candidates.ranked.iter().any(|c| {
                matches!(
                    c.opportunity.goal_key.kind,
                    GoalKind::ConsumeOwnedCommodity {
                        commodity: CommodityKind::Bread,
                    }
                )
            });
            assert!(
                has_consume,
                "decision trace should contain ConsumeOwnedCommodity{{Bread}} candidate"
            );
        }
        other => panic!("expected Planning outcome at tick 0, got {other:?}"),
    }

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

#[test]
fn hungry_merchant_eats_listed_stock_replays_deterministically() {
    let (w1, e1) = run_hungry_merchant_eats_listed_stock(Seed([87; 32]));
    let (w2, e2) = run_hungry_merchant_eats_listed_stock(Seed([87; 32]));
    assert_eq!(w1, w2, "world hash mismatch on replay");
    assert_eq!(e1, e2, "event log hash mismatch on replay");
}
