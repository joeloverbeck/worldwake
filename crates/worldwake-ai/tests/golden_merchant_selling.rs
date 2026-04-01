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
// Test 3: staff_market lists on start, unlists on complete
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
// Test 4: buyer trades against listed lot
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
// Test 5: unlisted stock not sellable
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
// Test 8: blocked intent dampens relisting
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
// Test 12: deterministic replay (uses test 4 scenario)
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
