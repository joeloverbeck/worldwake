//! Golden tests for buyer-driven trade acquisition and trade-domain determinism.

mod golden_harness;

use golden_harness::*;
use std::collections::BTreeSet;
use worldwake_core::{
    hash_event_log, hash_world, total_live_lot_quantity, CommodityKind, EventTag,
    HomeostaticNeeds, KnownRecipes, MerchandiseProfile, MetabolismProfile, Quantity, Seed,
    TradeDispositionProfile, UtilityProfile,
};
use worldwake_sim::RecipeRegistry;

fn default_trade_disposition_profile() -> TradeDispositionProfile {
    TradeDispositionProfile {
        negotiation_round_ticks: nz(4),
        initial_offer_bias: pm(500),
        concession_rate: pm(100),
        demand_memory_retention_ticks: 48,
    }
}

fn run_buyer_driven_trade_scenario(seed: Seed) -> (worldwake_core::StateHash, worldwake_core::StateHash) {
    let mut h = GoldenHarness::with_recipes(seed, RecipeRegistry::new());

    let seller = seed_agent_with_recipes(
        &mut h.world,
        &mut h.event_log,
        "Seller",
        VILLAGE_SQUARE,
        HomeostaticNeeds::default(),
        MetabolismProfile::default(),
        UtilityProfile::default(),
        KnownRecipes::new(),
    );
    let buyer = seed_agent_with_recipes(
        &mut h.world,
        &mut h.event_log,
        "Buyer",
        VILLAGE_SQUARE,
        HomeostaticNeeds::new(pm(900), pm(0), pm(0), pm(0), pm(0)),
        MetabolismProfile::default(),
        UtilityProfile::default(),
        KnownRecipes::new(),
    );

    give_commodity(
        &mut h.world,
        &mut h.event_log,
        seller,
        VILLAGE_SQUARE,
        CommodityKind::Bread,
        Quantity(3),
    );
    give_commodity(
        &mut h.world,
        &mut h.event_log,
        buyer,
        VILLAGE_SQUARE,
        CommodityKind::Coin,
        Quantity(3),
    );

    let mut txn = new_txn(&mut h.world, 0);
    txn.set_component_merchandise_profile(
        seller,
        MerchandiseProfile {
            sale_kinds: BTreeSet::from([CommodityKind::Bread]),
            home_market: Some(VILLAGE_SQUARE),
        },
    )
    .unwrap();
    txn.set_component_trade_disposition_profile(seller, default_trade_disposition_profile())
        .unwrap();
    txn.set_component_trade_disposition_profile(buyer, default_trade_disposition_profile())
        .unwrap();
    commit_txn(txn, &mut h.event_log);

    let initial_buyer_hunger = h.agent_hunger(buyer);
    let initial_seller_bread = h.agent_commodity_qty(seller, CommodityKind::Bread);
    let initial_seller_coins = h.agent_commodity_qty(seller, CommodityKind::Coin);
    let initial_buyer_coins = h.agent_commodity_qty(buyer, CommodityKind::Coin);
    let initial_total_bread = total_live_lot_quantity(&h.world, CommodityKind::Bread);
    let initial_total_coins = total_live_lot_quantity(&h.world, CommodityKind::Coin);

    let mut saw_trade_event = false;
    let mut buyer_received_bread = false;
    let mut seller_received_coins = false;
    let mut buyer_spent_coins = false;
    let mut seller_lost_bread = false;
    let mut buyer_hunger_decreased = false;

    for _ in 0..80 {
        h.step_once();

        let buyer_bread = h.agent_commodity_qty(buyer, CommodityKind::Bread);
        let seller_bread = h.agent_commodity_qty(seller, CommodityKind::Bread);
        let seller_coins = h.agent_commodity_qty(seller, CommodityKind::Coin);
        let buyer_coins = h.agent_commodity_qty(buyer, CommodityKind::Coin);
        let current_total_bread = total_live_lot_quantity(&h.world, CommodityKind::Bread);
        let current_total_coins = total_live_lot_quantity(&h.world, CommodityKind::Coin);

        saw_trade_event |= !h.event_log.events_by_tag(EventTag::Trade).is_empty();
        buyer_received_bread |= buyer_bread > Quantity(0);
        seller_received_coins |= seller_coins > initial_seller_coins;
        buyer_spent_coins |= buyer_coins < initial_buyer_coins;
        seller_lost_bread |= seller_bread < initial_seller_bread;
        buyer_hunger_decreased |= h.agent_hunger(buyer) < initial_buyer_hunger;

        assert!(
            current_total_bread <= initial_total_bread,
            "bread lots should not increase: initial={initial_total_bread}, now={current_total_bread}"
        );
        assert_eq!(
            current_total_coins, initial_total_coins,
            "coin lots should stay conserved exactly through trade"
        );

        if saw_trade_event
            && buyer_received_bread
            && seller_received_coins
            && buyer_spent_coins
            && seller_lost_bread
            && buyer_hunger_decreased
        {
            break;
        }
    }

    assert!(saw_trade_event, "scenario should execute at least one trade event");
    assert!(
        buyer_received_bread,
        "buyer should receive bread from the seller through trade"
    );
    assert!(
        seller_received_coins,
        "seller should receive coins from the buyer through trade"
    );
    assert!(
        buyer_spent_coins,
        "buyer should spend coins during the trade"
    );
    assert!(
        seller_lost_bread,
        "seller bread inventory should decrease during the trade"
    );
    assert!(
        buyer_hunger_decreased,
        "buyer hunger should decrease after consuming acquired bread"
    );
    assert!(!h.agent_is_dead(buyer), "buyer must stay alive");
    assert!(!h.agent_is_dead(seller), "seller must stay alive");

    (hash_world(&h.world).unwrap(), hash_event_log(&h.event_log).unwrap())
}

#[test]
fn golden_buyer_driven_trade_acquisition() {
    let _ = run_buyer_driven_trade_scenario(Seed([12; 32]));
}

#[test]
fn golden_buyer_driven_trade_acquisition_replays_deterministically() {
    let first = run_buyer_driven_trade_scenario(Seed([13; 32]));
    let second = run_buyer_driven_trade_scenario(Seed([13; 32]));

    assert_eq!(first, second, "trade scenario should replay deterministically");
}
