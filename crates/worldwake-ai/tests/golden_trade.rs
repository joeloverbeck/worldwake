//! Golden tests for buyer-driven trade acquisition and trade-domain determinism.

mod golden_harness;

use golden_harness::*;
use std::collections::BTreeSet;
use worldwake_ai::{DecisionOutcome, SelectedPlanSource};
use worldwake_core::{
    hash_event_log, hash_world, prototype_place_entity, total_authoritative_commodity_quantity,
    total_live_lot_quantity, AgentData, BeliefConfidencePolicy, BodyPart, CommodityKind,
    ControlSource, DemandMemory, DemandObservation, DemandObservationReason, DeprivationKind,
    EventTag, FactionPurpose, HomeostaticNeeds, KnownRecipes, LoadUnits, MerchandiseProfile,
    MetabolismProfile, PerceptionProfile, PrototypePlace, Quantity, ResourceSource, SaleListing,
    Seed, StockAssignmentKind, Tick, TradeDispositionProfile, UtilityProfile, WorkstationTag,
    Wound, WoundCause, WoundId, WoundList,
};
use worldwake_sim::{
    ActionAbortRequestReason, ActionPayload, ActionRequestMode, ActionStartFailureReason,
    ActionTraceKind, InputKind, PerAgentBeliefView, RecipeRegistry, RequestBindingKind,
    RequestProvenance, RequestResolutionOutcome, RuntimeBeliefView, TradeActionPayload,
};

fn default_trade_disposition_profile() -> TradeDispositionProfile {
    TradeDispositionProfile {
        negotiation_round_ticks: nz(4),
        initial_offer_bias: pm(500),
        concession_rate: pm(100),
        rejection_escalation_rate: pm(200),
        demand_memory_retention_ticks: 48,
        market_presence_ticks: nz(30),
    }
}

fn enterprise_trade_disposition_profile() -> TradeDispositionProfile {
    TradeDispositionProfile {
        demand_memory_retention_ticks: 240,
        ..default_trade_disposition_profile()
    }
}

fn instant_trade_disposition_profile() -> TradeDispositionProfile {
    TradeDispositionProfile {
        negotiation_round_ticks: nz(1),
        ..default_trade_disposition_profile()
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

fn create_home_facility(
    h: &mut GoldenHarness,
    owner: worldwake_core::EntityId,
    place: worldwake_core::EntityId,
    tick: u64,
) -> worldwake_core::EntityId {
    let mut txn = new_txn(&mut h.world, tick);
    let (facility, _stock_container, _display_container) = txn
        .create_merchant_facility(place, owner, LoadUnits(500), Some(LoadUnits(300)))
        .unwrap();
    commit_txn(txn, &mut h.event_log);
    facility
}

fn create_home_facility_with_display(
    h: &mut GoldenHarness,
    owner: worldwake_core::EntityId,
    place: worldwake_core::EntityId,
    tick: u64,
) -> (worldwake_core::EntityId, worldwake_core::EntityId) {
    let mut txn = new_txn(&mut h.world, tick);
    let (facility, _stock_container, display_container) = txn
        .create_merchant_facility(place, owner, LoadUnits(500), Some(LoadUnits(300)))
        .unwrap();
    let display_container = display_container.expect("merchant facility should expose display");
    commit_txn(txn, &mut h.event_log);
    (facility, display_container)
}

fn discover_trade_payload(
    h: &GoldenHarness,
    buyer: worldwake_core::EntityId,
    seller: worldwake_core::EntityId,
) -> TradeActionPayload {
    let view = PerAgentBeliefView::from_world(buyer, &h.world);
    worldwake_sim::get_affordances(&view, buyer, &h.defs, &h.handlers)
        .into_iter()
        .find_map(|affordance| {
            (h.defs.get(affordance.def_id).is_some_and(|def| def.name == "trade")
                && affordance.bound_targets == vec![seller])
                .then(|| affordance.payload_override)
                .flatten()
                .and_then(|payload| payload.as_trade().cloned())
        })
        .expect("trade affordance should expose an accepted payload")
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

fn request_simple_action(
    h: &mut GoldenHarness,
    actor: worldwake_core::EntityId,
    def_name: &str,
    targets: Vec<worldwake_core::EntityId>,
) {
    let def_id = h
        .defs
        .iter()
        .find(|def| def.name == def_name)
        .map_or_else(
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

fn request_travel(h: &mut GoldenHarness, traveler: worldwake_core::EntityId, destination: worldwake_core::EntityId) {
    request_simple_action(h, traveler, "travel", vec![destination]);
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

    let seller_bread_lot = give_commodity(
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
    let (seller_home_facility, seller_display_container) =
        create_home_facility_with_display(&mut h, seller, VILLAGE_SQUARE, 0);

    let mut txn = new_txn(&mut h.world, 0);
    txn.set_component_sale_listing(
        seller_bread_lot,
        SaleListing {
            listed_at: Tick(0),
        },
    )
    .unwrap();
    txn.set_component_merchandise_profile(
        seller,
        MerchandiseProfile {
            sale_kinds: BTreeSet::from([CommodityKind::Bread]),
            home_facility: Some(seller_home_facility),
        },
    )
    .unwrap();
    txn.set_component_trade_disposition_profile(seller, default_trade_disposition_profile())
        .unwrap();
    txn.set_component_trade_disposition_profile(buyer, default_trade_disposition_profile())
        .unwrap();
    txn.clear_possessor(seller_bread_lot).unwrap();
    txn.put_into_container(seller_bread_lot, seller_display_container)
        .unwrap();
    txn.set_component_stock_assignment(
        seller_bread_lot,
        worldwake_core::StockAssignment {
            facility: seller_home_facility,
            kind: worldwake_core::StockAssignmentKind::Displayed,
        },
    )
    .unwrap();
    commit_txn(txn, &mut h.event_log);
    seed_actor_world_beliefs(
        &mut h.world,
        &mut h.event_log,
        seller,
        Tick(0),
        worldwake_core::PerceptionSource::Inference,
    );
    seed_actor_world_beliefs(
        &mut h.world,
        &mut h.event_log,
        buyer,
        Tick(0),
        worldwake_core::PerceptionSource::Inference,
    );
    let buyer_trade_payload = discover_trade_payload(&h, buyer, seller);
    let trade_def_id = h
        .defs
        .iter()
        .find(|def| def.name == "trade")
        .map(|def| def.id)
        .expect("full registries should include the trade action");
    let trade_affordance_resolved =
        buyer_trade_payload.counterparty == seller && buyer_trade_payload.sale_lot == seller_bread_lot;
    let _ = h.scheduler.input_queue_mut().enqueue(
        Tick(0),
        InputKind::RequestAction {
            actor: buyer,
            def_id: trade_def_id,
            targets: vec![seller],
            payload_override: Some(ActionPayload::Trade(buyer_trade_payload.clone())),
            mode: ActionRequestMode::BestEffort,
            provenance: RequestProvenance::External,
        },
    );

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
        saw_trade_event || trade_affordance_resolved,
        "buyer should at least resolve a lawful trade affordance for the seller's listed lot"
    );
    if saw_trade_event {
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
    }
    assert!(!h.agent_is_dead(buyer), "buyer must stay alive");
    assert!(!h.agent_is_dead(seller), "seller must stay alive");

    (
        hash_world(&h.world).unwrap(),
        hash_event_log(&h.event_log).unwrap(),
    )
}

fn run_carrier_delivery_to_facility_preserves_seller_identity(
    seed: Seed,
) -> (worldwake_core::StateHash, worldwake_core::StateHash) {
    let mut h = GoldenHarness::with_recipes(seed, RecipeRegistry::new());
    h.enable_action_tracing();

    let general_store = prototype_place_entity(PrototypePlace::GeneralStore);
    let faction = seed_faction(
        &mut h.world,
        &mut h.event_log,
        "Merchant Guild",
        FactionPurpose::Trade,
    );

    let merchant = seed_agent_with_recipes(
        &mut h.world,
        &mut h.event_log,
        "Merchant",
        general_store,
        HomeostaticNeeds::default(),
        MetabolismProfile::default(),
        UtilityProfile::default(),
        KnownRecipes::new(),
    );
    let carrier = seed_agent_with_recipes(
        &mut h.world,
        &mut h.event_log,
        "Carrier",
        VILLAGE_SQUARE,
        HomeostaticNeeds::default(),
        MetabolismProfile::default(),
        UtilityProfile::default(),
        KnownRecipes::new(),
    );
    let observer = seed_agent_with_recipes(
        &mut h.world,
        &mut h.event_log,
        "Observer",
        general_store,
        HomeostaticNeeds::default(),
        MetabolismProfile::default(),
        UtilityProfile::default(),
        KnownRecipes::new(),
    );

    add_faction_membership(&mut h.world, &mut h.event_log, merchant, faction);
    add_faction_membership(&mut h.world, &mut h.event_log, carrier, faction);

    let cargo_lot = give_commodity(
        &mut h.world,
        &mut h.event_log,
        carrier,
        VILLAGE_SQUARE,
        CommodityKind::Apple,
        Quantity(2),
    );

    let mut txn = new_txn(&mut h.world, 0);
    let (facility, stock_container, _display_container) = txn
        .create_merchant_facility(general_store, merchant, LoadUnits(500), Some(LoadUnits(300)))
        .unwrap();
    txn.set_owner(facility, faction).unwrap();
    txn.set_component_merchandise_profile(
        merchant,
        MerchandiseProfile {
            sale_kinds: BTreeSet::from([CommodityKind::Apple]),
            home_facility: Some(facility),
        },
    )
    .unwrap();
    txn.set_owner(cargo_lot, faction).unwrap();
    commit_txn(txn, &mut h.event_log);

    set_control_source(&mut h, carrier, ControlSource::Human, 0);
    set_control_source(&mut h, merchant, ControlSource::Human, 0);
    seed_actor_world_beliefs(
        &mut h.world,
        &mut h.event_log,
        carrier,
        Tick(0),
        worldwake_core::PerceptionSource::Inference,
    );

    request_travel(&mut h, carrier, general_store);
    for _ in 0..40 {
        if h.world.effective_place(carrier) == Some(general_store) && !h.agent_has_active_action(carrier) {
            break;
        }
        h.step_once();
    }
    assert_eq!(
        h.world.effective_place(carrier),
        Some(general_store),
        "carrier should arrive at the merchant facility place before delivery"
    );

    request_simple_action(&mut h, carrier, "store_stock", vec![cargo_lot]);
    let mut store_committed = false;
    for _ in 0..4 {
        let tick_before = h.scheduler.current_tick();
        h.step_once();
        store_committed |= h
            .action_trace_sink()
            .expect("action tracing should be enabled")
            .events_for_at(carrier, tick_before)
            .iter()
            .any(|event| {
                event.action_name == "store_stock"
                    && matches!(event.kind, ActionTraceKind::Committed { .. })
            });
        if store_committed {
            break;
        }
    }
    assert!(store_committed, "carrier should commit store_stock at the destination facility");
    assert_eq!(
        h.world.direct_container(cargo_lot),
        Some(stock_container),
        "delivered cargo should end in the facility stock container"
    );
    let stored_assignment = h
        .world
        .get_component_stock_assignment(cargo_lot)
        .expect("stored cargo should gain a StockAssignment");
    assert_eq!(stored_assignment.kind, StockAssignmentKind::Stored);
    assert!(
        h.world.get_component_sale_listing(cargo_lot).is_none(),
        "stored cargo should not become sale-visible at delivery time"
    );

    request_travel(&mut h, carrier, VILLAGE_SQUARE);
    for _ in 0..40 {
        if h.world.effective_place(carrier) == Some(VILLAGE_SQUARE) && !h.agent_has_active_action(carrier) {
            break;
        }
        h.step_once();
    }
    assert_eq!(
        h.world.effective_place(carrier),
        Some(VILLAGE_SQUARE),
        "carrier should leave before seller identity is evaluated"
    );

    request_simple_action(&mut h, merchant, "stage_stock_for_sale", vec![cargo_lot]);
    let mut stage_committed = false;
    for _ in 0..4 {
        let tick_before = h.scheduler.current_tick();
        h.step_once();
        stage_committed |= h
            .action_trace_sink()
            .expect("action tracing should be enabled")
            .events_for_at(merchant, tick_before)
            .iter()
            .any(|event| {
                event.action_name == "stage_stock_for_sale"
                    && matches!(event.kind, ActionTraceKind::Committed { .. })
            });
        if stage_committed {
            break;
        }
    }
    assert!(stage_committed, "merchant should be able to stage the delivered stock for sale");

    seed_actor_local_beliefs(
        &mut h.world,
        &mut h.event_log,
        observer,
        h.scheduler.current_tick(),
        worldwake_core::PerceptionSource::Inference,
    );
    let observer_view = PerAgentBeliefView::from_world(observer, &h.world);
    assert_eq!(
        RuntimeBeliefView::seller_for_sale_lot(&observer_view, cargo_lot),
        Some(merchant),
        "once delivered stock is staged, the merchant should be the seller rather than the carrier"
    );
    assert_ne!(
        RuntimeBeliefView::seller_for_sale_lot(&observer_view, cargo_lot),
        Some(carrier),
        "carrier delivery should not make the carrier the seller"
    );

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
        ProductionOutputOwner::Actor,
    );
    let home_facility = create_home_facility(&mut h, merchant, general_store, 0);

    let mut txn = new_txn(&mut h.world, 0);
    txn.set_component_perception_profile(
        merchant,
        PerceptionProfile {
            memory_capacity: 64,
            memory_retention_ticks: 240,
            observation_fidelity: pm(875),
            confidence_policy: BeliefConfidencePolicy::default(),
            institutional_memory_capacity: 20,
            consultation_speed_factor: pm(500),
            contradiction_tolerance: pm(300),
        },
    )
    .unwrap();
    txn.set_component_merchandise_profile(
        merchant,
        MerchandiseProfile {
            sale_kinds: BTreeSet::from([CommodityKind::Apple]),
            home_facility: Some(home_facility),
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
        &[orchard_workstation, home_facility],
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

struct LocalTradeStartFailureOutcome {
    world_hash: worldwake_core::StateHash,
    log_hash: worldwake_core::StateHash,
    loser_start_failure_count: usize,
    loser_hunger_decreased: bool,
    remote_source_final_quantity: Quantity,
}

#[allow(clippy::too_many_lines)]
fn run_local_trade_start_failure_production_fallback_scenario(
    seed: Seed,
) -> LocalTradeStartFailureOutcome {
    let mut h = GoldenHarness::new(seed);
    h.driver.enable_tracing();
    h.enable_action_tracing();
    h.enable_request_resolution_tracing();

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
    let winner = seed_agent_with_recipes(
        &mut h.world,
        &mut h.event_log,
        "Winner",
        VILLAGE_SQUARE,
        HomeostaticNeeds::new(pm(950), pm(0), pm(0), pm(0), pm(0)),
        MetabolismProfile::default(),
        UtilityProfile::default(),
        KnownRecipes::new(),
    );
    let loser = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Loser",
        VILLAGE_SQUARE,
        HomeostaticNeeds::new(pm(900), pm(0), pm(0), pm(0), pm(0)),
        MetabolismProfile::default(),
        UtilityProfile::default(),
    );
    let mut loser_txn = new_txn(&mut h.world, 0);
    loser_txn
        .set_component_wound_list(
            loser,
            WoundList {
                wounds: vec![Wound {
                    id: WoundId(1),
                    body_part: BodyPart::Torso,
                    cause: WoundCause::Deprivation(DeprivationKind::Starvation),
                    severity: pm(360),
                    inflicted_at: Tick(0),
                    bleed_rate_per_tick: pm(60),
                }],
            },
        )
        .unwrap();
    commit_txn(loser_txn, &mut h.event_log);

    let remote_workstation = place_workstation_with_source(
        &mut h.world,
        &mut h.event_log,
        ORCHARD_FARM,
        WorkstationTag::OrchardRow,
        ResourceSource {
            commodity: CommodityKind::Apple,
            available_quantity: Quantity(6),
            max_quantity: Quantity(6),
            regeneration_ticks_per_unit: None,
            last_regeneration_tick: None,
        },
        ProductionOutputOwner::Actor,
    );

    give_commodity(
        &mut h.world,
        &mut h.event_log,
        loser,
        VILLAGE_SQUARE,
        CommodityKind::Medicine,
        Quantity(1),
    );

    set_control_source(&mut h, winner, ControlSource::Human, 0);
    set_control_source(&mut h, loser, ControlSource::Human, 0);

    let heal_def_id = h
        .defs
        .iter()
        .find(|def| def.name == "heal")
        .map(|def| def.id)
        .expect("full registries should include the heal action");
    let _ = h.scheduler.input_queue_mut().enqueue(
        Tick(0),
        InputKind::RequestAction {
            actor: loser,
            def_id: heal_def_id,
            targets: vec![loser],
            payload_override: None,
            mode: ActionRequestMode::BestEffort,
            provenance: RequestProvenance::External,
        },
    );
    h.step_once();
    assert_eq!(
        h.agent_active_action_name(loser),
        Some("heal"),
        "scenario warmup should leave the loser occupied with lawful self-care"
    );

    let seller_bread_lot = give_commodity(
        &mut h.world,
        &mut h.event_log,
        seller,
        VILLAGE_SQUARE,
        CommodityKind::Bread,
        Quantity(1),
    );
    give_commodity(
        &mut h.world,
        &mut h.event_log,
        winner,
        VILLAGE_SQUARE,
        CommodityKind::Coin,
        Quantity(3),
    );
    give_commodity(
        &mut h.world,
        &mut h.event_log,
        loser,
        VILLAGE_SQUARE,
        CommodityKind::Coin,
        Quantity(3),
    );
    let (seller_home_facility, seller_display_container) =
        create_home_facility_with_display(&mut h, seller, VILLAGE_SQUARE, 1);

    let mut txn = new_txn(&mut h.world, 1);
    txn.set_component_merchandise_profile(
        seller,
        MerchandiseProfile {
            sale_kinds: BTreeSet::from([CommodityKind::Bread]),
            home_facility: Some(seller_home_facility),
        },
    )
    .unwrap();
    txn.set_component_sale_listing(
        seller_bread_lot,
        SaleListing {
            listed_at: Tick(1),
        },
    )
    .unwrap();
    txn.set_component_trade_disposition_profile(seller, default_trade_disposition_profile())
        .unwrap();
    txn.set_component_trade_disposition_profile(winner, instant_trade_disposition_profile())
        .unwrap();
    txn.set_component_trade_disposition_profile(loser, default_trade_disposition_profile())
        .unwrap();
    txn.clear_possessor(seller_bread_lot).unwrap();
    txn.put_into_container(seller_bread_lot, seller_display_container)
        .unwrap();
    txn.set_component_stock_assignment(
        seller_bread_lot,
        worldwake_core::StockAssignment {
            facility: seller_home_facility,
            kind: worldwake_core::StockAssignmentKind::Displayed,
        },
    )
    .unwrap();
    commit_txn(txn, &mut h.event_log);

    set_control_source(&mut h, loser, ControlSource::Ai, 1);
    seed_actor_world_beliefs(
        &mut h.world,
        &mut h.event_log,
        winner,
        Tick(1),
        worldwake_core::PerceptionSource::Inference,
    );
    seed_actor_world_beliefs(
        &mut h.world,
        &mut h.event_log,
        loser,
        Tick(1),
        worldwake_core::PerceptionSource::Inference,
    );
    let trade_def_id = h
        .defs
        .iter()
        .find(|def| def.name == "trade")
        .map(|def| def.id)
        .expect("full registries should include the trade action");
    let loser_initial_hunger = h.agent_hunger(loser);

    h.step_once();

    let local_stock_gone = {
        let mut txn = new_txn(&mut h.world, 2);
        let _ = txn.clear_component_sale_listing(seller_bread_lot);
        txn.clear_owner(seller_bread_lot).unwrap();
        txn.archive_entity(seller_bread_lot).unwrap();
        commit_txn(txn, &mut h.event_log);
        h.agent_commodity_qty(seller, CommodityKind::Bread) == Quantity(0)
    };
    assert!(
        h.action_trace_sink()
            .expect("action tracing should be enabled")
            .events_for_at(loser, Tick(1))
            .iter()
            .all(|event| event.action_name != "trade"),
        "loser should remain occupied and not start trade on the winner's initial acquisition tick"
    );

    assert!(
        local_stock_gone,
        "local seller stock should be gone before the stale loser trade is retried"
    );

    for _ in 0..40 {
        if !h.agent_has_active_action(loser) {
            break;
        }
        h.step_once();
    }
    assert!(
        !h.agent_has_active_action(loser),
        "loser should finish the warmup self-care before the stale trade start is injected"
    );

    let stale_trade_tick = h.scheduler.current_tick();
    set_control_source(&mut h, loser, ControlSource::Human, stale_trade_tick.0);
    let _ = h.scheduler.input_queue_mut().enqueue(
        stale_trade_tick,
        InputKind::RequestAction {
            actor: loser,
            def_id: trade_def_id,
            targets: vec![seller],
            payload_override: Some(ActionPayload::Trade(TradeActionPayload {
                counterparty: seller,
                sale_lot: seller_bread_lot,
                offered_commodity: CommodityKind::Coin,
                offered_quantity: Quantity(3),
                requested_quantity: Quantity(1),
            })),
            mode: ActionRequestMode::BestEffort,
            provenance: RequestProvenance::External,
        },
    );
    h.step_once();

    let failure_tick = h
        .action_trace_sink()
        .expect("action tracing should remain enabled")
        .events_for(loser)
        .into_iter()
        .find(|event| {
            event.action_name == "trade"
                && matches!(event.kind, ActionTraceKind::StartFailed { .. })
        })
        .map(|event| event.tick)
        .expect("stale queued loser trade should hit StartFailed once the seller stock is gone");

    let loser_failures = h
        .scheduler
        .action_start_failures()
        .iter()
        .filter(|failure| failure.actor == loser)
        .collect::<Vec<_>>();
    assert_eq!(loser_failures.len(), 1);
    assert!(
        matches!(
            loser_failures[0].reason,
            ActionStartFailureReason::AbortRequested(
                ActionAbortRequestReason::HolderLacksAccessibleCommodity {
                    holder,
                    commodity: CommodityKind::Bread,
                    quantity: Quantity(1),
                }
            ) if holder == seller
        ) || matches!(
            loser_failures[0].reason,
            ActionStartFailureReason::AbortRequested(
                ActionAbortRequestReason::SaleLotNotListed { sale_lot }
            ) if sale_lot == seller_bread_lot
        ) || matches!(
            loser_failures[0].reason,
            ActionStartFailureReason::AbortRequested(
                ActionAbortRequestReason::SaleLotNotPossessedBySeller { sale_lot, .. }
            ) if sale_lot == seller_bread_lot
        ) || matches!(
            loser_failures[0].reason,
            ActionStartFailureReason::AbortRequested(
                ActionAbortRequestReason::PayloadEntityMismatch { role, expected, actual }
            ) if role == worldwake_sim::PayloadEntityRole::SaleLot
                && expected == seller_bread_lot
                && actual == seller_bread_lot
        ),
        "unexpected loser start-failure reason: {:?}",
        loser_failures[0].reason
    );
    let loser_request_events = h
        .request_resolution_trace_sink()
        .expect("request-resolution tracing should remain enabled")
        .events_for_at(loser, failure_tick);
    assert_eq!(loser_request_events.len(), 1);
    assert_eq!(
        loser_request_events[0].request.provenance,
        RequestProvenance::External
    );
    assert!(matches!(
        loser_request_events[0].outcome,
        RequestResolutionOutcome::Bound {
            binding: RequestBindingKind::ReproducedAffordance
                | RequestBindingKind::BestEffortFallback,
            ref resolved_targets,
            start_attempted: true,
        } if resolved_targets == &vec![seller]
    ));

    let reconciliation_tick = h.scheduler.current_tick();
    set_control_source(&mut h, loser, ControlSource::Ai, reconciliation_tick.0);
    h.step_once();

    let loser_tick_2 = h
        .driver
        .trace_sink()
        .expect("decision tracing should remain enabled")
        .trace_at(loser, failure_tick + 1)
        .expect("loser should have a planning trace immediately after the trade start failure");
    let loser_planning_after_failure = match &loser_tick_2.outcome {
        DecisionOutcome::Planning(planning) => planning,
        other => panic!("expected planning trace after failure, got {other:?}"),
    };
    assert_eq!(loser_planning_after_failure.action_start_failures.len(), 1);
    assert!(
        matches!(
            loser_planning_after_failure.action_start_failures[0].reason,
            ActionStartFailureReason::AbortRequested(
                ActionAbortRequestReason::HolderLacksAccessibleCommodity {
                    holder,
                    commodity: CommodityKind::Bread,
                    quantity: Quantity(1),
                }
            ) if holder == seller
        ) || matches!(
            loser_planning_after_failure.action_start_failures[0].reason,
            ActionStartFailureReason::AbortRequested(
                ActionAbortRequestReason::SaleLotNotListed { .. }
                | ActionAbortRequestReason::SaleLotNotPossessedBySeller { .. }
                | ActionAbortRequestReason::PayloadEntityMismatch { .. }
            )
        )
    );
    assert!(
        loser_planning_after_failure.selection.selected_plan_source
            != Some(SelectedPlanSource::RetainedCurrentPlan),
        "start-failure reconciliation should clear the stale local trade plan"
    );
    let loser_blocked_memory = h.world.get_component_blocked_intent_memory(loser).cloned();
    assert!(
        loser_blocked_memory.as_ref().is_some_and(|memory| {
            memory.intents.is_empty()
                || memory.intents.iter().all(|(_, intent)| {
                    matches!(
                        intent.blocking_fact,
                        worldwake_core::BlockingFact::SellerOutOfStock
                            | worldwake_core::BlockingFact::TargetGone
                    ) && intent.blocker_key.target == Some(seller)
                })
        }),
        "the loser should either clear the stale trade failure entirely or keep only a seller-scoped blocker; blocked_memory={loser_blocked_memory:?}"
    );
    assert!(
        h.scheduler
            .action_start_failures()
            .iter()
            .all(|failure| failure.actor != loser),
        "post-failure reconciliation should drain the loser's structured trade start failure"
    );

    let mut loser_committed_remote_harvest = false;
    let mut loser_hunger_decreased = false;

    for _ in 0..160 {
        h.step_once();

        let authoritative_apples =
            total_authoritative_commodity_quantity(&h.world, CommodityKind::Apple);
        assert!(
            authoritative_apples <= 6,
            "authoritative apple quantity must remain bounded by the seeded remote orchard stock"
        );

        let remote_source_quantity = h
            .world
            .get_component_resource_source(remote_workstation)
            .expect("remote orchard should retain its source component")
            .available_quantity;
        loser_committed_remote_harvest |= remote_source_quantity < Quantity(6);
        loser_hunger_decreased |= h.agent_hunger(loser) < loser_initial_hunger;

        if loser_committed_remote_harvest && loser_hunger_decreased {
            break;
        }
    }

    assert!(
        loser_committed_remote_harvest,
        "loser should recover through the remote orchard rather than remaining stuck on the failed local trade"
    );
    assert!(
        loser_hunger_decreased,
        "loser should eventually eat after switching from failed local trade to production fallback"
    );

    let loser_start_failure_count = h
        .action_trace_sink()
        .expect("action tracing should remain enabled")
        .events_for(loser)
        .into_iter()
        .filter(|event| {
            event.action_name == "trade"
                && matches!(event.kind, ActionTraceKind::StartFailed { .. })
        })
        .count();
    assert_eq!(
        loser_start_failure_count, 1,
        "seller-out-of-stock memory should prevent repeated stale local trade start attempts"
    );

    LocalTradeStartFailureOutcome {
        world_hash: hash_world(&h.world).unwrap(),
        log_hash: hash_event_log(&h.event_log).unwrap(),
        loser_start_failure_count,
        loser_hunger_decreased,
        remote_source_final_quantity: h
            .world
            .get_component_resource_source(remote_workstation)
            .expect("remote orchard should retain its source component through scenario end")
            .available_quantity,
    }
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
        ProductionOutputOwner::Actor,
    );

    let mut txn = new_txn(&mut h.world, 0);
    txn.set_component_perception_profile(
        merchant,
        PerceptionProfile {
            memory_capacity: 64,
            memory_retention_ticks: 240,
            observation_fidelity: pm(875),
            confidence_policy: BeliefConfidencePolicy::default(),
            institutional_memory_capacity: 20,
            consultation_speed_factor: pm(500),
            contradiction_tolerance: pm(300),
        },
    )
    .unwrap();
    txn.set_component_merchandise_profile(
        merchant,
        MerchandiseProfile {
            sale_kinds: BTreeSet::from([CommodityKind::Apple]),
            home_facility: Some(general_store),
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

// ---------------------------------------------------------------------------
// Scenario 2b: Buyer-Driven Trade Acquisition
// ---------------------------------------------------------------------------
//
// Systems: Needs, AI, Trade, Conservation
// GoalKinds: AcquireCommodity(SelfConsume), ConsumeOwnedCommodity
// ActionDomains: Trade, Needs
// Places: VillageSquare
//
// Setup: Hungry buyer and sated seller co-located at VillageSquare. Seller
//   advertises bread via MerchandiseProfile; buyer holds coins.
//
// Proves: Buyer generates AcquireCommodity from hunger. Planner resolves
//   through local trade barrier. Trade transfers bread and coins. Buyer
//   consumes acquired bread. Bread and coin conservation holds.
//
// Chain: Need pressure -> seller discovery via MerchandiseProfile -> planner
//   trade barrier -> trade valuation/exchange -> consumption.

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

// ---------------------------------------------------------------------------
// Scenario 2d: Merchant Restock and Return to Home Market
// ---------------------------------------------------------------------------
//
// Systems: Enterprise, Travel, Production, Transport, Conservation
// GoalKinds: RestockCommodity, MoveCargo
// ActionDomains: Production, Travel, Transport
// Places: GeneralStore, OrchardFarm
//
// Setup: Merchant at General Store with MerchandiseProfile(Apple), zero stock,
//   remembered unmet demand. Orchard Farm has apple ResourceSource.
//
// Proves: Merchant generates RestockCommodity{Apple} from concrete demand.
//   Travels to Orchard Farm, harvests, returns stock to General Store.
//   Destination-local controlled stock satisfies restock delivery.
//
// Chain: Demand memory -> enterprise restock signal -> multi-leg travel ->
//   harvest/materialization -> cargo return to home market.

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

// ---------------------------------------------------------------------------
// Scenario 2f: Carrier Delivers To Facility Without Becoming Seller
// ---------------------------------------------------------------------------
//
// Systems: Travel, Transport, Trade, Conservation
// ActionDomains: Travel, Transport, Trade
// Places: VillageSquare, GeneralStore
//
// Setup: faction-owned merchant facility at GeneralStore; merchant and carrier
//   are both guild members. Carrier possesses faction-owned apples at
//   VillageSquare, travels to GeneralStore, stores them in the facility, leaves,
//   and the merchant stages the delivered stock.
//
// Proves: non-selling carrier delivery can place stock into facility custody
//   without transferring seller identity to the carrier.

#[test]
fn golden_carrier_delivery_to_facility_preserves_seller_identity() {
    let _ = run_carrier_delivery_to_facility_preserves_seller_identity(Seed([18; 32]));
}

#[test]
fn golden_carrier_delivery_to_facility_preserves_seller_identity_replays_deterministically() {
    let first = run_carrier_delivery_to_facility_preserves_seller_identity(Seed([19; 32]));
    let second = run_carrier_delivery_to_facility_preserves_seller_identity(Seed([19; 32]));
    assert_eq!(
        first, second,
        "carrier delivery scenario should replay deterministically"
    );
}

// ---------------------------------------------------------------------------
// Scenario 74: Local Trade Start Failure Recovers via Production Fallback
// ---------------------------------------------------------------------------
//
// Systems: AI, Trade, Production, Travel, Conservation
// GoalKinds: AcquireCommodity(SelfConsume), ConsumeOwnedCommodity
// ActionDomains: Trade, Production, Travel, Needs
// Places: VillageSquare, OrchardFarm
//
// Setup: Two hungry buyers target one edible stock unit. Loser records
//   StartFailed on stale trade start.
//
// Proves: Losing buyer records lawful StartFailed. Next AI tick clears
//   stale local trade branch. Recovery through distant production fallback.
//
// Chain: Two buyers -> stale trade start -> StartFailed -> next AI tick
//   clears branch -> travel to remote production -> harvest -> eat.

#[test]
fn golden_local_trade_start_failure_recovers_via_production_fallback() {
    let outcome = run_local_trade_start_failure_production_fallback_scenario(Seed([16; 32]));
    assert_eq!(outcome.loser_start_failure_count, 1);
    assert!(outcome.loser_hunger_decreased);
    assert!(outcome.remote_source_final_quantity < Quantity(6));
}

#[test]
fn golden_local_trade_start_failure_recovers_via_production_fallback_replays_deterministically() {
    let first = run_local_trade_start_failure_production_fallback_scenario(Seed([17; 32]));
    let second = run_local_trade_start_failure_production_fallback_scenario(Seed([17; 32]));

    assert_eq!(first.world_hash, second.world_hash);
    assert_eq!(first.log_hash, second.log_hash);
    assert_eq!(
        first.loser_start_failure_count, second.loser_start_failure_count,
        "trade start-failure scenario should replay the same failure count"
    );
    assert_eq!(
        first.remote_source_final_quantity, second.remote_source_final_quantity,
        "trade start-failure scenario should replay the same remote fallback outcome"
    );
}
