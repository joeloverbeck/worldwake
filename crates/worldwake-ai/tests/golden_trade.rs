//! Golden tests for buyer-driven trade acquisition and trade-domain determinism.

mod golden_harness;

use golden_harness::*;
use std::collections::BTreeSet;
use worldwake_ai::{DecisionOutcome, SelectedPlanSource};
use worldwake_core::{
    hash_event_log, hash_world, prototype_place_entity, total_authoritative_commodity_quantity,
    total_live_lot_quantity, AgentData, BeliefConfidencePolicy, BodyPart, CommodityKind,
    ControlSource, DemandMemory, DemandObservation, DemandObservationReason, DeprivationKind,
    EventTag, HomeostaticNeeds, KnownRecipes, MerchandiseProfile, MetabolismProfile,
    PerceptionProfile, PrototypePlace, Quantity, ResourceSource, Seed, Tick,
    TradeDispositionProfile, UtilityProfile, WorkstationTag, Wound, WoundCause, WoundId, WoundList,
};
use worldwake_sim::{
    ActionAbortRequestReason, ActionPayload, ActionRequestMode, ActionStartFailureReason,
    ActionTraceKind, InputKind, RecipeRegistry, RequestBindingKind, RequestProvenance,
    RequestResolutionOutcome, TradeActionPayload,
};

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

    give_commodity(
        &mut h.world,
        &mut h.event_log,
        seller,
        VILLAGE_SQUARE,
        CommodityKind::Bread,
        Quantity(2),
    );
    give_commodity(
        &mut h.world,
        &mut h.event_log,
        winner,
        VILLAGE_SQUARE,
        CommodityKind::Coin,
        Quantity(2),
    );
    give_commodity(
        &mut h.world,
        &mut h.event_log,
        loser,
        VILLAGE_SQUARE,
        CommodityKind::Coin,
        Quantity(2),
    );

    let mut txn = new_txn(&mut h.world, 1);
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
    txn.set_component_trade_disposition_profile(winner, instant_trade_disposition_profile())
        .unwrap();
    txn.set_component_trade_disposition_profile(loser, default_trade_disposition_profile())
        .unwrap();
    commit_txn(txn, &mut h.event_log);

    set_control_source(&mut h, loser, ControlSource::Ai, 1);
    let trade_def_id = h
        .defs
        .iter()
        .find(|def| def.name == "trade")
        .map(|def| def.id)
        .expect("full registries should include the trade action");
    for tick in [Tick(1), Tick(2)] {
        let _ = h.scheduler.input_queue_mut().enqueue(
            tick,
            InputKind::RequestAction {
                actor: winner,
                def_id: trade_def_id,
                targets: vec![seller],
                payload_override: Some(ActionPayload::Trade(TradeActionPayload {
                    counterparty: seller,
                    offered_commodity: CommodityKind::Coin,
                    offered_quantity: Quantity(1),
                    requested_commodity: CommodityKind::Bread,
                    requested_quantity: Quantity(1),
                })),
                mode: ActionRequestMode::BestEffort,
                provenance: RequestProvenance::External,
            },
        );
    }

    seed_actor_beliefs(
        &mut h.world,
        &mut h.event_log,
        winner,
        &[seller, remote_workstation],
        Tick(1),
        worldwake_core::PerceptionSource::Inference,
    );
    seed_actor_beliefs(
        &mut h.world,
        &mut h.event_log,
        loser,
        &[seller, remote_workstation],
        Tick(1),
        worldwake_core::PerceptionSource::Inference,
    );

    let loser_initial_hunger = h.agent_hunger(loser);

    h.step_once();

    let action_trace = h
        .action_trace_sink()
        .expect("action tracing should be enabled");
    let mut winner_trade_committed =
        action_trace
            .events_for_at(winner, Tick(1))
            .iter()
            .any(|event| {
                event.action_name == "trade"
                    && matches!(event.kind, ActionTraceKind::Committed { .. })
            });
    let mut local_stock_gone = h.agent_commodity_qty(seller, CommodityKind::Bread) == Quantity(0);
    assert!(
        action_trace
            .events_for_at(loser, Tick(1))
            .iter()
            .all(|event| event.action_name != "trade"),
        "loser should remain occupied and not start trade on the winner's initial acquisition tick"
    );

    for _ in 0..12 {
        if winner_trade_committed && local_stock_gone {
            break;
        }
        h.step_once();
        winner_trade_committed |= h
            .action_trace_sink()
            .expect("action tracing should remain enabled")
            .events_for(winner)
            .into_iter()
            .any(|event| {
                event.action_name == "trade"
                    && matches!(event.kind, ActionTraceKind::Committed { .. })
            });
        local_stock_gone |= h.agent_commodity_qty(seller, CommodityKind::Bread) == Quantity(0);
    }
    assert!(
        winner_trade_committed,
        "winner should eventually complete the local trade before the loser retries the stale branch; winner events={:?}, seller_bread={:?}",
        h.action_trace_sink()
            .expect("action tracing should remain enabled")
            .events_for(winner),
        h.agent_commodity_qty(seller, CommodityKind::Bread)
    );
    assert!(
        local_stock_gone,
        "winner's trade should consume the seller's only bread while the loser is still occupied"
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
                offered_commodity: CommodityKind::Coin,
                offered_quantity: Quantity(1),
                requested_commodity: CommodityKind::Bread,
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
    assert!(matches!(
        loser_planning_after_failure.action_start_failures[0].reason,
        ActionStartFailureReason::AbortRequested(
            ActionAbortRequestReason::HolderLacksAccessibleCommodity {
                holder,
                commodity: CommodityKind::Bread,
                quantity: Quantity(1),
            }
        ) if holder == seller
    ));
    assert!(
        loser_planning_after_failure.selection.selected_plan_source
            != Some(SelectedPlanSource::RetainedCurrentPlan),
        "start-failure reconciliation should clear the stale local trade plan"
    );
    assert!(
        h.world
            .get_component_blocked_intent_memory(loser)
            .is_some_and(|memory| memory.intents.iter().any(|intent| {
                intent.blocking_fact == worldwake_core::BlockingFact::SellerOutOfStock
                    && intent.related_entity == Some(seller)
            })),
        "the loser should remember that this seller is out of stock rather than blocking all food acquisition"
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
