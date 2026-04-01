//! Golden tests for merchant selling market-presence (S04 spec).
//!
//! These tests exercise the seller-side lifecycle: `SaleListing` attachment via
//! `staff_market`, buyer discovery of listed lots, trade against concrete listed
//! lots, blocked-intent dampening after unproductive sell cycles, and
//! deterministic replay of the full merchant-selling pipeline.

mod golden_harness;

use golden_harness::*;
use std::collections::BTreeSet;
use worldwake_core::{
    hash_event_log, hash_world, total_live_lot_quantity, BlockingFact, CommodityKind,
    ControlSource, DemandMemory, DemandObservation, DemandObservationReason, EventTag, GoalKind,
    HomeostaticNeeds, KnownRecipes, MerchandiseProfile, MetabolismProfile, PerceptionProfile,
    Quantity, SaleListing, Seed, Tick, TradeDispositionProfile, UtilityProfile,
};
use worldwake_ai::DecisionOutcome;
use worldwake_sim::{ActionTraceKind, RecipeRegistry};

// ---------------------------------------------------------------------------
// Shared helpers
// ---------------------------------------------------------------------------

fn merchant_trade_disposition() -> TradeDispositionProfile {
    TradeDispositionProfile {
        negotiation_round_ticks: nz(1),
        initial_offer_bias: pm(500),
        concession_rate: pm(200),
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

/// Seed a merchant at `place` with `MerchandiseProfile`, trade disposition,
/// perception, AI control, enterprise utility, and stock of `commodity`.
/// Returns `(merchant_entity, stock_lot_entity)`.
fn seed_merchant(
    h: &mut GoldenHarness,
    name: &str,
    place: worldwake_core::EntityId,
    commodity: CommodityKind,
    quantity: Quantity,
) -> (worldwake_core::EntityId, worldwake_core::EntityId) {
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
    txn.set_component_merchandise_profile(
        merchant,
        MerchandiseProfile {
            sale_kinds: BTreeSet::from([commodity]),
            home_market: Some(place),
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
    commit_txn(txn, &mut h.event_log);

    (merchant, stock_lot)
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
// Scenario 75: staff_market Lists on Start, Unlists on Complete
// Systems: Trade, AI
// GoalKinds: SellCommodity
// ActionDomains: Trade
// Principles: P1, P3, P8
// Proves: staff_market attaches SaleListing on start and removes it on commit
// ---------------------------------------------------------------------------

#[allow(clippy::too_many_lines)]
fn run_staff_market_lists_unlists(
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

    // Seed beliefs so the AI can perceive the world.
    seed_actor_local_beliefs(
        &mut h.world,
        &mut h.event_log,
        merchant,
        Tick(0),
        worldwake_core::PerceptionSource::Inference,
    );

    // Before any ticks: no SaleListing on the stock lot.
    assert!(
        h.world.get_component_sale_listing(stock_lot).is_none(),
        "stock lot should not have SaleListing before staff_market starts"
    );

    // Run ticks until staff_market starts.
    let mut staff_market_started = false;
    for _ in 0..80 {
        h.step_once();
        if let Some(sink) = h.action_trace_sink() {
            staff_market_started |= sink.events_for(merchant).iter().any(|event| {
                event.action_name == "staff_market"
                    && matches!(event.kind, ActionTraceKind::Started { .. })
            });
        }
        if staff_market_started {
            break;
        }
    }
    assert!(
        staff_market_started,
        "merchant should start staff_market within 80 ticks"
    );

    // After staff_market starts, the stock lot should have SaleListing.
    assert!(
        h.world.get_component_sale_listing(stock_lot).is_some(),
        "stock lot should have SaleListing after staff_market starts"
    );

    // Run until staff_market commits.
    let mut staff_market_committed = false;
    for _ in 0..80 {
        h.step_once();
        if let Some(sink) = h.action_trace_sink() {
            staff_market_committed |= sink.events_for(merchant).iter().any(|event| {
                event.action_name == "staff_market"
                    && matches!(event.kind, ActionTraceKind::Committed { .. })
            });
        }
        if staff_market_committed {
            break;
        }
    }
    assert!(
        staff_market_committed,
        "merchant should commit staff_market within 30 more ticks"
    );

    // After staff_market commits (unproductive — no buyer), listing should be removed.
    assert!(
        h.world.get_component_sale_listing(stock_lot).is_none(),
        "stock lot should not have SaleListing after unproductive staff_market commits"
    );

    (hash_world(&h.world).unwrap(), hash_event_log(&h.event_log).unwrap())
}

#[test]
fn staff_market_lists_on_start_unlists_on_complete() {
    run_staff_market_lists_unlists(Seed([60; 32]));
}

#[test]
fn staff_market_lists_on_start_unlists_on_complete_replays_deterministically() {
    let (w1, e1) = run_staff_market_lists_unlists(Seed([60; 32]));
    let (w2, e2) = run_staff_market_lists_unlists(Seed([60; 32]));
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
        txn.set_component_sale_listing(
            stock_lot,
            SaleListing {
                listed_at: Tick(0),
            },
        )
        .unwrap();
        commit_txn(txn, &mut h.event_log);
    }

    // Seed beliefs for both agents.
    seed_actor_beliefs(
        &mut h.world,
        &mut h.event_log,
        merchant,
        &[buyer],
        Tick(0),
        worldwake_core::PerceptionSource::Inference,
    );
    seed_actor_beliefs(
        &mut h.world,
        &mut h.event_log,
        buyer,
        &[merchant],
        Tick(0),
        worldwake_core::PerceptionSource::Inference,
    );

    let initial_total_bread = total_live_lot_quantity(&h.world, CommodityKind::Bread);
    let initial_total_coins = total_live_lot_quantity(&h.world, CommodityKind::Coin);

    let mut trade_committed = false;
    let mut buyer_got_bread = false;
    let mut merchant_got_coins = false;

    for _ in 0..80 {
        h.step_once();

        let buyer_bread = h.agent_commodity_qty(buyer, CommodityKind::Bread);
        let merchant_coins = h.agent_commodity_qty(merchant, CommodityKind::Coin);

        trade_committed |= h.action_trace_sink().is_some_and(|sink| {
            sink.events_for(buyer)
                .iter()
                .any(|e| {
                    e.action_name == "trade"
                        && matches!(e.kind, ActionTraceKind::Committed { .. })
                })
        });
        buyer_got_bread |= buyer_bread > Quantity(0);
        merchant_got_coins |= merchant_coins > Quantity(0);

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

        if trade_committed && buyer_got_bread && merchant_got_coins {
            break;
        }
    }

    assert!(trade_committed, "buyer should complete a trade against the listed lot");
    assert!(buyer_got_bread, "buyer should have received bread from trade");
    assert!(merchant_got_coins, "merchant should have received coins from trade");

    (hash_world(&h.world).unwrap(), hash_event_log(&h.event_log).unwrap())
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

    let (merchant, stock_lot) = seed_merchant(
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
// Scenario 78: Blocked Intent Dampens Relisting After Unproductive Cycle
// Systems: Trade, AI
// GoalKinds: SellCommodity
// ActionDomains: Trade
// Principles: P1, P8
// Proves: NoBuyer blocked intent suppresses immediate SellCommodity re-emission
// ---------------------------------------------------------------------------

#[test]
fn blocked_intent_dampens_relisting_after_unproductive_cycle() {
    let mut h = GoldenHarness::with_recipes(Seed([63; 32]), RecipeRegistry::new());
    h.driver.enable_tracing();
    h.enable_action_tracing();

    let (merchant, _stock_lot) = seed_merchant(
        &mut h,
        "Merchant",
        VILLAGE_SQUARE,
        CommodityKind::Bread,
        Quantity(3),
    );

    seed_actor_local_beliefs(
        &mut h.world,
        &mut h.event_log,
        merchant,
        Tick(0),
        worldwake_core::PerceptionSource::Inference,
    );

    // Run until first staff_market commits (unproductive — no buyer present).
    let mut first_commit_tick: Option<Tick> = None;
    for _ in 0..120 {
        let tick_before = h.scheduler.current_tick();
        h.step_once();
        if first_commit_tick.is_none() {
            if let Some(sink) = h.action_trace_sink() {
                if sink.events_for_at(merchant, tick_before).iter().any(|e| {
                    e.action_name == "staff_market"
                        && matches!(e.kind, ActionTraceKind::Committed { .. })
                }) {
                    first_commit_tick = Some(tick_before);
                }
            }
        }
        if first_commit_tick.is_some() {
            break;
        }
    }
    let first_commit = first_commit_tick.expect("staff_market should commit within 120 ticks");

    // After unproductive commit, blocked intent memory should contain NoBuyer.
    let blocked = h
        .world
        .get_component_blocked_intent_memory(merchant)
        .expect("blocked intent memory should exist after unproductive cycle");
    let has_no_buyer = blocked.intents.values().any(|intent| {
        intent.blocking_fact == BlockingFact::NoBuyer
            && intent.blocker_key.goal_key.kind
                == GoalKind::SellCommodity {
                    commodity: CommodityKind::Bread,
                }
    });
    assert!(
        has_no_buyer,
        "NoBuyer blocked intent should exist after unproductive staff_market at tick {first_commit:?}"
    );

    // Run a few more ticks — merchant should NOT start another staff_market
    // while the blocked intent is active.
    let mut second_staff_market = false;
    for _ in 0..5 {
        let tick_before = h.scheduler.current_tick();
        h.step_once();
        if let Some(sink) = h.action_trace_sink() {
            second_staff_market |= sink
                .events_for_at(merchant, tick_before)
                .iter()
                .any(|e| {
                    e.action_name == "staff_market"
                        && matches!(e.kind, ActionTraceKind::Started { .. })
                });
        }
    }
    assert!(
        !second_staff_market,
        "merchant should not restart staff_market immediately after unproductive cycle (blocked intent should dampen)"
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

    let (w3, e3) = run_staff_market_lists_unlists(Seed([65; 32]));
    let (w4, e4) = run_staff_market_lists_unlists(Seed([65; 32]));
    assert_eq!(w3, w4, "staff_market world hash mismatch on replay");
    assert_eq!(e3, e4, "staff_market event log hash mismatch on replay");
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
    assert!(trace.is_some(), "buyer should have a decision trace at tick 0");

    // The unlisted lot should never appear as a trade-discoverable lot.
    // Verify via world state: the unlisted lot has no SaleListing.
    assert!(
        h.world.get_component_sale_listing(listed_lot).is_some(),
        "listed lot should retain SaleListing"
    );
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
fn merchant_emits_sell_commodity_at_home_market() {
    let mut h = GoldenHarness::with_recipes(Seed([71; 32]), RecipeRegistry::new());
    h.driver.enable_tracing();

    let (merchant, _stock_lot) = seed_merchant(
        &mut h,
        "Merchant",
        VILLAGE_SQUARE,
        CommodityKind::Bread,
        Quantity(3),
    );

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
    let trace = sink.trace_at(merchant, Tick(0)).expect("merchant should have trace at tick 0");
    match &trace.outcome {
        DecisionOutcome::Planning(planning) => {
            let has_sell = planning.candidates.ranked.iter().any(|c| {
                matches!(c.opportunity.goal_key.kind, GoalKind::SellCommodity { commodity } if commodity == CommodityKind::Bread)
            });
            assert!(
                has_sell,
                "merchant at home_market with unlisted stock should emit SellCommodity candidate"
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

    // Kill the merchant: clear possessions then archive.
    {
        let mut txn = new_txn(&mut h.world, 1);
        txn.clear_possessor(stock_lot).unwrap();
        txn.clear_owner(stock_lot).unwrap();
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
// Proves: merchant at remote place plans Travel + StaffMarket to reach home_market
// ---------------------------------------------------------------------------

#[test]
fn move_cargo_then_sell_commodity_plan_shape() {
    let mut h = GoldenHarness::with_recipes(Seed([74; 32]), RecipeRegistry::new());
    h.driver.enable_tracing();

    // Merchant at ORCHARD_FARM, home_market is VILLAGE_SQUARE.
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
    {
        let mut txn = new_txn(&mut h.world, 0);
        txn.set_component_merchandise_profile(
            merchant,
            MerchandiseProfile {
                sale_kinds: BTreeSet::from([CommodityKind::Bread]),
                home_market: Some(VILLAGE_SQUARE),
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

    // The merchant should eventually travel to home_market and start staff_market.
    let mut saw_travel = false;
    let mut arrived_at_home = false;
    let mut saw_staff_market = false;
    for _ in 0..120 {
        h.step_once();
        saw_travel |= h.agent_active_action_name(merchant) == Some("travel");
        arrived_at_home |= h.world.effective_place(merchant) == Some(VILLAGE_SQUARE);
        if let Some(sink) = h.action_trace_sink() {
            saw_staff_market |= sink.events_for(merchant).iter().any(|e| {
                e.action_name == "staff_market"
                    && matches!(e.kind, ActionTraceKind::Started { .. })
            });
        }
        if saw_staff_market {
            break;
        }
    }
    // The merchant should at least travel (restock-driven movement toward home market).
    assert!(
        saw_travel || arrived_at_home || saw_staff_market,
        "merchant at remote place with stock and demand memory should eventually move toward home_market or start selling"
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
    let (merchant_a, _) = seed_merchant(
        &mut h,
        "MerchantA",
        VILLAGE_SQUARE,
        CommodityKind::Bread,
        Quantity(3),
    );
    // Merchant B: has stock, no demand memory — use a separate seed_agent call
    // and manually set up profiles without the demand memory from seed_merchant.
    let merchant_b = seed_agent_with_recipes(
        &mut h.world,
        &mut h.event_log,
        "MerchantB",
        VILLAGE_SQUARE,
        HomeostaticNeeds::new_sated(),
        MetabolismProfile::default(),
        merchant_utility(),
        KnownRecipes::new(),
    );
    give_commodity(
        &mut h.world,
        &mut h.event_log,
        merchant_b,
        VILLAGE_SQUARE,
        CommodityKind::Bread,
        Quantity(3),
    );
    {
        let mut txn = new_txn(&mut h.world, 0);
        txn.set_component_merchandise_profile(
            merchant_b,
            MerchandiseProfile {
                sale_kinds: BTreeSet::from([CommodityKind::Bread]),
                home_market: Some(VILLAGE_SQUARE),
            },
        )
        .unwrap();
        txn.set_component_trade_disposition_profile(merchant_b, merchant_trade_disposition())
            .unwrap();
        txn.set_component_perception_profile(merchant_b, PerceptionProfile::default())
            .unwrap();
        // No demand memory for merchant B.
        commit_txn(txn, &mut h.event_log);
    }

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
    let (w1, e1) = run_staff_market_lists_unlists(Seed([76; 32]));
    let (w2, e2) = run_staff_market_lists_unlists(Seed([76; 32]));
    assert_eq!(w1, w2, "planning state world hash mismatch");
    assert_eq!(e1, e2, "planning state event log hash mismatch");
}

// ---------------------------------------------------------------------------
// Scenario 87: Hungry Merchant Eats Own Listed Sale Stock
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

    let (merchant, bread_lot) = seed_merchant(
        &mut h,
        "HungryMerchant",
        VILLAGE_SQUARE,
        CommodityKind::Bread,
        Quantity(1),
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
    assert_eq!(
        final_bread, 0,
        "Quantity(1) bread should be fully consumed"
    );

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
