//! Golden tests for buyer-driven trade acquisition and trade-domain determinism.

mod golden_harness;

use golden_harness::*;
use std::collections::BTreeSet;
use worldwake_core::{
    hash_event_log, hash_world, prototype_place_entity, total_authoritative_commodity_quantity,
    total_live_lot_quantity, BeliefConfidencePolicy, CommodityKind, DemandMemory,
    DemandObservation, DemandObservationReason, EventTag, HomeostaticNeeds, KnownRecipes,
    MerchandiseProfile, MetabolismProfile, PerceptionProfile, PrototypePlace, Quantity,
    ResourceSource, Seed, Tick, TradeDispositionProfile, UtilityProfile, WorkstationTag,
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

fn enterprise_trade_disposition_profile() -> TradeDispositionProfile {
    TradeDispositionProfile {
        demand_memory_retention_ticks: 240,
        ..default_trade_disposition_profile()
    }
}

fn remembered_demand(
    commodity: CommodityKind,
    quantity: Quantity,
    place: worldwake_core::EntityId,
    counterparty: Option<worldwake_core::EntityId>,
) -> DemandMemory {
    DemandMemory {
        observations: vec![DemandObservation {
            commodity,
            quantity,
            place,
            tick: Tick(0),
            counterparty,
            reason: DemandObservationReason::WantedToBuyButSellerOutOfStock,
        }],
    }
}

#[allow(clippy::too_many_lines)]
fn run_buyer_driven_trade_scenario(
    seed: Seed,
) -> (worldwake_core::StateHash, worldwake_core::StateHash) {
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

    assert!(
        saw_trade_event,
        "scenario should execute at least one trade event"
    );
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

    (
        hash_world(&h.world).unwrap(),
        hash_event_log(&h.event_log).unwrap(),
    )
}

#[allow(clippy::too_many_lines)]
fn run_merchant_restock_return_stock_scenario(
    seed: Seed,
) -> (worldwake_core::StateHash, worldwake_core::StateHash) {
    let mut h = GoldenHarness::with_recipes(seed, build_recipes());
    let general_store = prototype_place_entity(PrototypePlace::GeneralStore);

    let merchant = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Merchant",
        general_store,
        HomeostaticNeeds::default(),
        MetabolismProfile::default(),
        UtilityProfile::default(),
    );

    let orchard_workstation = place_workstation_with_source(
        &mut h.world,
        &mut h.event_log,
        ORCHARD_FARM,
        WorkstationTag::OrchardRow,
        ResourceSource {
            commodity: CommodityKind::Apple,
            available_quantity: Quantity(10),
            max_quantity: Quantity(10),
            regeneration_ticks_per_unit: None,
            last_regeneration_tick: None,
        },
    );

    let mut txn = new_txn(&mut h.world, 0);
    txn.set_component_perception_profile(
        merchant,
        PerceptionProfile {
            memory_capacity: 64,
            memory_retention_ticks: 240,
            observation_fidelity: pm(875),
            confidence_policy: BeliefConfidencePolicy::default(),
        },
    )
    .unwrap();
    txn.set_component_merchandise_profile(
        merchant,
        MerchandiseProfile {
            sale_kinds: BTreeSet::from([CommodityKind::Apple]),
            home_market: Some(general_store),
        },
    )
    .unwrap();
    txn.set_component_trade_disposition_profile(merchant, enterprise_trade_disposition_profile())
        .unwrap();
    txn.set_component_demand_memory(
        merchant,
        remembered_demand(CommodityKind::Apple, Quantity(2), general_store, None),
    )
    .unwrap();
    commit_txn(txn, &mut h.event_log);
    seed_actor_beliefs(
        &mut h.world,
        &mut h.event_log,
        merchant,
        &[orchard_workstation],
        Tick(0),
        worldwake_core::PerceptionSource::Inference,
    );

    let initial_merchant_apples = h.agent_commodity_qty(merchant, CommodityKind::Apple);
    let initial_authoritative_apples =
        total_authoritative_commodity_quantity(&h.world, CommodityKind::Apple);

    let mut merchant_left_home = false;
    let mut merchant_controlled_apples_away_from_home = false;
    let mut merchant_returned_home_with_apples = false;

    for _ in 0..220 {
        h.step_once();

        let merchant_place = h.world.effective_place(merchant);
        let merchant_apples = h.agent_commodity_qty(merchant, CommodityKind::Apple);
        let authoritative_apples =
            total_authoritative_commodity_quantity(&h.world, CommodityKind::Apple);

        merchant_left_home |=
            h.world.is_in_transit(merchant) || merchant_place != Some(general_store);
        merchant_controlled_apples_away_from_home |= merchant_apples > Quantity(0)
            && (h.world.is_in_transit(merchant) || merchant_place != Some(general_store));
        merchant_returned_home_with_apples |=
            merchant_place == Some(general_store) && merchant_apples > Quantity(0);

        assert_eq!(
            initial_merchant_apples,
            Quantity(0),
            "merchant should start with zero apples at the home market"
        );
        assert!(
            authoritative_apples <= initial_authoritative_apples,
            "authoritative apples should never increase: initial={initial_authoritative_apples}, now={authoritative_apples}"
        );

        if merchant_left_home
            && merchant_controlled_apples_away_from_home
            && merchant_returned_home_with_apples
        {
            break;
        }
    }

    assert!(
        merchant_left_home,
        "merchant should leave the home market before completing the loop"
    );
    assert!(
        merchant_controlled_apples_away_from_home,
        "merchant should control apples away from the home market after restocking"
    );
    assert!(
        merchant_returned_home_with_apples,
        "merchant should return apples to the home market after restocking"
    );
    assert!(!h.agent_is_dead(merchant), "merchant must stay alive");

    (
        hash_world(&h.world).unwrap(),
        hash_event_log(&h.event_log).unwrap(),
    )
}

#[test]
fn merchant_route_knowledge_alone_does_not_unlock_remote_restock() {
    let mut h = GoldenHarness::with_recipes(Seed([16; 32]), build_recipes());
    let general_store = prototype_place_entity(PrototypePlace::GeneralStore);

    let merchant = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Merchant",
        general_store,
        HomeostaticNeeds::default(),
        MetabolismProfile::default(),
        UtilityProfile::default(),
    );

    let _orchard_workstation = place_workstation_with_source(
        &mut h.world,
        &mut h.event_log,
        ORCHARD_FARM,
        WorkstationTag::OrchardRow,
        ResourceSource {
            commodity: CommodityKind::Apple,
            available_quantity: Quantity(10),
            max_quantity: Quantity(10),
            regeneration_ticks_per_unit: None,
            last_regeneration_tick: None,
        },
    );

    let mut txn = new_txn(&mut h.world, 0);
    txn.set_component_perception_profile(
        merchant,
        PerceptionProfile {
            memory_capacity: 64,
            memory_retention_ticks: 240,
            observation_fidelity: pm(875),
            confidence_policy: BeliefConfidencePolicy::default(),
        },
    )
    .unwrap();
    txn.set_component_merchandise_profile(
        merchant,
        MerchandiseProfile {
            sale_kinds: BTreeSet::from([CommodityKind::Apple]),
            home_market: Some(general_store),
        },
    )
    .unwrap();
    txn.set_component_trade_disposition_profile(merchant, enterprise_trade_disposition_profile())
        .unwrap();
    txn.set_component_demand_memory(
        merchant,
        remembered_demand(CommodityKind::Apple, Quantity(2), general_store, None),
    )
    .unwrap();
    commit_txn(txn, &mut h.event_log);

    let mut merchant_left_home = false;
    let mut merchant_controlled_apples = false;

    for _ in 0..120 {
        h.step_once();
        merchant_left_home |= h.world.is_in_transit(merchant)
            || h.world.effective_place(merchant) != Some(general_store);
        merchant_controlled_apples |=
            h.agent_commodity_qty(merchant, CommodityKind::Apple) > Quantity(0);
    }

    assert!(
        !merchant_left_home,
        "public route knowledge alone should not unlock remote restock travel"
    );
    assert!(
        !merchant_controlled_apples,
        "merchant should not acquire remote stock without explicit orchard knowledge"
    );
}

#[test]
fn golden_buyer_driven_trade_acquisition() {
    let _ = run_buyer_driven_trade_scenario(Seed([12; 32]));
}

#[test]
fn golden_buyer_driven_trade_acquisition_replays_deterministically() {
    let first = run_buyer_driven_trade_scenario(Seed([13; 32]));
    let second = run_buyer_driven_trade_scenario(Seed([13; 32]));

    assert_eq!(
        first, second,
        "trade scenario should replay deterministically"
    );
}

#[test]
fn golden_merchant_restock_return_stock() {
    let _ = run_merchant_restock_return_stock_scenario(Seed([14; 32]));
}

#[test]
fn golden_merchant_restock_return_stock_replays_deterministically() {
    let first = run_merchant_restock_return_stock_scenario(Seed([15; 32]));
    let second = run_merchant_restock_return_stock_scenario(Seed([15; 32]));

    assert_eq!(
        first, second,
        "merchant restock-return stock scenario should replay deterministically"
    );
}
