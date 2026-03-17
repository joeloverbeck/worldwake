//! Golden E2E tests for the multi-role supply chain (S02c).
//!
//! These tests exercise the supply chain in two proven segments:
//!
//! 1. **Merchant restock cycle**: merchant travels to orchard, harvests apples,
//!    returns to General Store with stock. Validates multi-hop travel + production
//!    with decision trace assertions.
//!
//! 2. **Consumer co-located trade**: consumer trades with a merchant who already
//!    has apples, then eats to reduce hunger. Validates trade + consumption with
//!    decision trace assertions.
//!
//! The full end-to-end chain (restock → trade → consumption in one simulation)
//! is blocked on `specs/S10-bilateral-trade-negotiation.md`: the trade system
//! lacks price negotiation, so the consumer's fixed 1-coin offer is rejected by
//! the enterprise-focused merchant. S09 (A* heuristic + travel pruning) resolved
//! the earlier plan search budget exhaustion; perception (E14) correctly updates
//! beliefs. The remaining blocker is purely trade valuation mechanics.

mod golden_harness;

use golden_harness::*;
use std::collections::BTreeSet;
use worldwake_ai::{AgentTickDriver, DecisionOutcome, PlanningBudget};
use worldwake_core::{
    hash_event_log, hash_world, prototype_place_entity, total_authoritative_commodity_quantity,
    total_live_lot_quantity, BeliefConfidencePolicy, CommodityKind, DemandMemory,
    DemandObservation, DemandObservationReason, GoalKind, HomeostaticNeeds, KnownRecipes,
    MerchandiseProfile, MetabolismProfile, PerceptionProfile, PerceptionSource, PrototypePlace,
    Quantity, ResourceSource, Seed, StateHash, Tick, TradeDispositionProfile, UtilityProfile,
    WorkstationTag,
};

fn default_trade_disposition() -> TradeDispositionProfile {
    TradeDispositionProfile {
        negotiation_round_ticks: nz(4),
        initial_offer_bias: pm(500),
        concession_rate: pm(100),
        demand_memory_retention_ticks: 48,
    }
}

fn enterprise_trade_disposition() -> TradeDispositionProfile {
    TradeDispositionProfile {
        demand_memory_retention_ticks: 240,
        ..default_trade_disposition()
    }
}

// ---------------------------------------------------------------------------
// Segment 1: Merchant Restock Cycle (with traces)
// ---------------------------------------------------------------------------

#[allow(clippy::too_many_lines)]
fn run_merchant_restock_with_traces(
    seed: Seed,
) -> (StateHash, StateHash) {
    let general_store = prototype_place_entity(PrototypePlace::GeneralStore);
    let mut h = GoldenHarness::new(seed);

    // OrchardRow workstation + ResourceSource at Orchard Farm.
    let orchard_ws = place_workstation_with_source(
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

    // Producer at Orchard Farm — low needs, knows harvest, sells apples.
    let producer = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Producer",
        ORCHARD_FARM,
        HomeostaticNeeds::default(),
        MetabolismProfile::default(),
        UtilityProfile::default(),
    );
    set_agent_perception_profile(
        &mut h.world,
        &mut h.event_log,
        producer,
        PerceptionProfile {
            memory_capacity: 64,
            memory_retention_ticks: 240,
            observation_fidelity: pm(875),
            confidence_policy: BeliefConfidencePolicy::default(),
        },
    );
    {
        let mut txn = new_txn(&mut h.world, 0);
        txn.set_component_merchandise_profile(
            producer,
            MerchandiseProfile {
                sale_kinds: BTreeSet::from([CommodityKind::Apple]),
                home_market: Some(ORCHARD_FARM),
            },
        )
        .unwrap();
        txn.set_component_trade_disposition_profile(producer, default_trade_disposition())
            .unwrap();
        commit_txn(txn, &mut h.event_log);
    }
    seed_actor_local_beliefs(
        &mut h.world,
        &mut h.event_log,
        producer,
        Tick(0),
        PerceptionSource::DirectObservation,
    );

    // Merchant at General Store — enterprise-focused, has coins, restock demand.
    let merchant = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Merchant",
        general_store,
        HomeostaticNeeds::default(),
        MetabolismProfile::default(),
        UtilityProfile {
            enterprise_weight: pm(900),
            ..UtilityProfile::default()
        },
    );
    give_commodity(
        &mut h.world,
        &mut h.event_log,
        merchant,
        general_store,
        CommodityKind::Coin,
        Quantity(5),
    );
    {
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
        txn.set_component_trade_disposition_profile(merchant, enterprise_trade_disposition())
            .unwrap();
        txn.set_component_demand_memory(
            merchant,
            DemandMemory {
                observations: vec![DemandObservation {
                    commodity: CommodityKind::Apple,
                    quantity: Quantity(2),
                    place: general_store,
                    tick: Tick(0),
                    counterparty: None,
                    reason: DemandObservationReason::WantedToBuyButSellerOutOfStock,
                }],
            },
        )
        .unwrap();
        commit_txn(txn, &mut h.event_log);
    }
    seed_actor_beliefs(
        &mut h.world,
        &mut h.event_log,
        merchant,
        &[orchard_ws, producer],
        Tick(0),
        PerceptionSource::Inference,
    );

    // Plan continuation (SUPPLYCHAINFIX-001) allows default budget in multi-agent
    // scenarios by skipping expensive plan search when the current goal is still
    // top-ranked and the next step revalidates.
    h.driver = AgentTickDriver::new(PlanningBudget::default());

    // Enable tracing and run.
    h.driver.enable_tracing();

    let initial_authoritative_apples =
        total_authoritative_commodity_quantity(&h.world, CommodityKind::Apple);

    let mut merchant_left_home = false;
    let mut merchant_acquired_apples = false;
    let mut merchant_returned_with_apples = false;

    for _ in 0..220 {
        h.step_once();

        let merchant_place = h.world.effective_place(merchant);
        let merchant_apples = h.agent_commodity_qty(merchant, CommodityKind::Apple);
        let authoritative_apples =
            total_authoritative_commodity_quantity(&h.world, CommodityKind::Apple);

        merchant_left_home |=
            h.world.is_in_transit(merchant) || merchant_place != Some(general_store);
        merchant_acquired_apples |= merchant_apples > Quantity(0);
        merchant_returned_with_apples |=
            merchant_place == Some(general_store) && merchant_apples > Quantity(0);

        assert!(
            authoritative_apples <= initial_authoritative_apples,
            "authoritative apples should never increase: initial={initial_authoritative_apples}, now={authoritative_apples}"
        );

        if merchant_left_home && merchant_acquired_apples && merchant_returned_with_apples {
            break;
        }
    }

    // Supply chain assertions.
    assert!(
        merchant_left_home,
        "Merchant should leave General Store to restock"
    );
    assert!(
        merchant_acquired_apples,
        "Merchant should acquire apples (via harvest at Orchard Farm)"
    );
    assert!(
        merchant_returned_with_apples,
        "Merchant should return to General Store with apples"
    );
    assert!(!h.agent_is_dead(producer), "Producer must stay alive");
    assert!(!h.agent_is_dead(merchant), "Merchant must stay alive");

    // Trace assertion: Merchant generated RestockCommodity(Apple) in the first 20 ticks.
    let sink = h.driver.trace_sink().expect("tracing should be enabled");
    let merchant_early_restock = (0u64..=20).any(|t| {
        sink.trace_at(merchant, Tick(t))
            .map_or(false, |trace| match &trace.outcome {
                DecisionOutcome::Planning(planning) => {
                    planning.candidates.generated.iter().any(|g| {
                        matches!(
                            g.kind,
                            GoalKind::RestockCommodity {
                                commodity: CommodityKind::Apple
                            }
                        )
                    })
                }
                _ => false,
            })
    });
    assert!(
        merchant_early_restock,
        "Merchant should generate RestockCommodity(Apple) in the first 20 ticks. \
         Use `sink.dump_agent(merchant, &h.defs)` to diagnose."
    );

    (
        hash_world(&h.world).unwrap(),
        hash_event_log(&h.event_log).unwrap(),
    )
}

// ---------------------------------------------------------------------------
// Segment 2: Consumer Co-located Trade (with traces)
// ---------------------------------------------------------------------------

#[allow(clippy::too_many_lines)]
fn run_consumer_trade_with_traces(seed: Seed) -> (StateHash, StateHash) {
    use worldwake_sim::RecipeRegistry;

    let general_store = prototype_place_entity(PrototypePlace::GeneralStore);
    let mut h = GoldenHarness::with_recipes(seed, RecipeRegistry::new());

    // Merchant at General Store with apples ready to sell.
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
    give_commodity(
        &mut h.world,
        &mut h.event_log,
        merchant,
        general_store,
        CommodityKind::Apple,
        Quantity(3),
    );

    // Consumer at General Store — hungry, has coins.
    let consumer = seed_agent_with_recipes(
        &mut h.world,
        &mut h.event_log,
        "Consumer",
        general_store,
        HomeostaticNeeds::new(pm(900), pm(0), pm(0), pm(0), pm(0)),
        MetabolismProfile::default(),
        UtilityProfile::default(),
        KnownRecipes::new(),
    );
    give_commodity(
        &mut h.world,
        &mut h.event_log,
        consumer,
        general_store,
        CommodityKind::Coin,
        Quantity(3),
    );

    {
        let mut txn = new_txn(&mut h.world, 0);
        txn.set_component_merchandise_profile(
            merchant,
            MerchandiseProfile {
                sale_kinds: BTreeSet::from([CommodityKind::Apple]),
                home_market: Some(general_store),
            },
        )
        .unwrap();
        txn.set_component_trade_disposition_profile(merchant, default_trade_disposition())
            .unwrap();
        txn.set_component_trade_disposition_profile(consumer, default_trade_disposition())
            .unwrap();
        commit_txn(txn, &mut h.event_log);
    }

    // Enable tracing and run.
    h.driver.enable_tracing();

    let initial_consumer_hunger = h.agent_hunger(consumer);
    let initial_total_coins = total_live_lot_quantity(&h.world, CommodityKind::Coin);

    let mut consumer_acquired_apples = false;
    let mut consumer_hunger_decreased = false;

    for _ in 0..80 {
        h.step_once();

        let consumer_apples = h.agent_commodity_qty(consumer, CommodityKind::Apple);
        consumer_acquired_apples |= consumer_apples > Quantity(0);
        consumer_hunger_decreased |= h.agent_hunger(consumer) < initial_consumer_hunger;

        // Coin conservation per tick.
        let current_coins = total_live_lot_quantity(&h.world, CommodityKind::Coin);
        assert_eq!(
            current_coins, initial_total_coins,
            "Coin conservation violated"
        );

        if consumer_acquired_apples && consumer_hunger_decreased {
            break;
        }
    }

    // Assertions.
    assert!(
        consumer_acquired_apples,
        "Consumer should acquire apples via trade with merchant"
    );
    assert!(
        consumer_hunger_decreased,
        "Consumer hunger should decrease after eating acquired apples"
    );
    assert!(!h.agent_is_dead(consumer), "Consumer must stay alive");
    assert!(!h.agent_is_dead(merchant), "Merchant must stay alive");

    // Trace assertion: Consumer generated AcquireCommodity(Apple) in the first 10 ticks.
    let sink = h.driver.trace_sink().expect("tracing should be enabled");
    let consumer_acquire_goal = (0u64..=10).any(|t| {
        sink.trace_at(consumer, Tick(t))
            .map_or(false, |trace| match &trace.outcome {
                DecisionOutcome::Planning(planning) => {
                    planning.candidates.generated.iter().any(|g| {
                        matches!(
                            g.kind,
                            GoalKind::AcquireCommodity {
                                commodity: CommodityKind::Apple,
                                ..
                            }
                        )
                    })
                }
                _ => false,
            })
    });
    assert!(
        consumer_acquire_goal,
        "Consumer should generate AcquireCommodity(Apple) in the first 10 ticks. \
         Use `sink.dump_agent(consumer, &h.defs)` to diagnose."
    );

    (
        hash_world(&h.world).unwrap(),
        hash_event_log(&h.event_log).unwrap(),
    )
}

// ---------------------------------------------------------------------------
// Full combined: Merchant Restock → Consumer Trade (with traces)
// ---------------------------------------------------------------------------

/// Full end-to-end supply chain in a single simulation:
/// Producer at Orchard Farm, Merchant at General Store, Consumer at General Store.
/// Merchant restocks apples from orchard, returns, consumer buys and eats.
#[allow(clippy::too_many_lines)]
fn run_full_supply_chain(seed: Seed) -> (StateHash, StateHash) {
    let general_store = prototype_place_entity(PrototypePlace::GeneralStore);
    let mut h = GoldenHarness::new(seed);

    // OrchardRow workstation + ResourceSource at Orchard Farm.
    let orchard_ws = place_workstation_with_source(
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

    // Producer at Orchard Farm — low needs, knows harvest, sells apples.
    let producer = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Producer",
        ORCHARD_FARM,
        HomeostaticNeeds::default(),
        MetabolismProfile::default(),
        UtilityProfile::default(),
    );
    set_agent_perception_profile(
        &mut h.world,
        &mut h.event_log,
        producer,
        PerceptionProfile {
            memory_capacity: 64,
            memory_retention_ticks: 240,
            observation_fidelity: pm(875),
            confidence_policy: BeliefConfidencePolicy::default(),
        },
    );
    {
        let mut txn = new_txn(&mut h.world, 0);
        txn.set_component_merchandise_profile(
            producer,
            MerchandiseProfile {
                sale_kinds: BTreeSet::from([CommodityKind::Apple]),
                home_market: Some(ORCHARD_FARM),
            },
        )
        .unwrap();
        txn.set_component_trade_disposition_profile(producer, default_trade_disposition())
            .unwrap();
        commit_txn(txn, &mut h.event_log);
    }
    seed_actor_local_beliefs(
        &mut h.world,
        &mut h.event_log,
        producer,
        Tick(0),
        PerceptionSource::DirectObservation,
    );

    // Merchant at General Store — enterprise-focused, has coins, restock demand.
    // Minimal non-enterprise metabolism so the merchant stays at General Store
    // after returning with stock, ensuring co-location for consumer trade.
    let merchant_metabolism = MetabolismProfile {
        hunger_rate: pm(1),
        thirst_rate: pm(0),
        fatigue_rate: pm(0),
        bladder_rate: pm(0),
        dirtiness_rate: pm(0),
        ..MetabolismProfile::default()
    };
    let merchant = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Merchant",
        general_store,
        HomeostaticNeeds::default(),
        merchant_metabolism,
        UtilityProfile {
            enterprise_weight: pm(900),
            ..UtilityProfile::default()
        },
    );
    give_commodity(
        &mut h.world,
        &mut h.event_log,
        merchant,
        general_store,
        CommodityKind::Coin,
        Quantity(5),
    );
    {
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
        txn.set_component_trade_disposition_profile(merchant, enterprise_trade_disposition())
            .unwrap();
        txn.set_component_demand_memory(
            merchant,
            DemandMemory {
                observations: vec![DemandObservation {
                    commodity: CommodityKind::Apple,
                    quantity: Quantity(2),
                    place: general_store,
                    tick: Tick(0),
                    counterparty: None,
                    reason: DemandObservationReason::WantedToBuyButSellerOutOfStock,
                }],
            },
        )
        .unwrap();
        commit_txn(txn, &mut h.event_log);
    }
    seed_actor_beliefs(
        &mut h.world,
        &mut h.event_log,
        merchant,
        &[orchard_ws, producer],
        Tick(0),
        PerceptionSource::Inference,
    );

    // Consumer at General Store — hungry, has coins, knows about merchant.
    // Hunger-only metabolism prevents the consumer from drifting away to
    // satisfy bladder/thirst during the ~150-tick wait for the merchant.
    let consumer_metabolism = MetabolismProfile {
        hunger_rate: pm(2),
        thirst_rate: pm(0),
        fatigue_rate: pm(0),
        bladder_rate: pm(0),
        dirtiness_rate: pm(0),
        ..MetabolismProfile::default()
    };
    let consumer = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Consumer",
        general_store,
        HomeostaticNeeds::new(pm(800), pm(0), pm(0), pm(0), pm(0)),
        consumer_metabolism,
        UtilityProfile::default(),
    );
    give_commodity(
        &mut h.world,
        &mut h.event_log,
        consumer,
        general_store,
        CommodityKind::Coin,
        Quantity(5),
    );
    {
        let mut txn = new_txn(&mut h.world, 0);
        txn.set_component_perception_profile(
            consumer,
            PerceptionProfile {
                memory_capacity: 64,
                memory_retention_ticks: 480,
                observation_fidelity: pm(875),
                confidence_policy: BeliefConfidencePolicy::default(),
            },
        )
        .unwrap();
        txn.set_component_trade_disposition_profile(consumer, default_trade_disposition())
            .unwrap();
        commit_txn(txn, &mut h.event_log);
    }
    seed_actor_local_beliefs(
        &mut h.world,
        &mut h.event_log,
        consumer,
        Tick(0),
        PerceptionSource::DirectObservation,
    );

    // Increased budget for the 3-agent combined scenario.
    // The segment tests prove plan continuation works at default 512 budget.
    // The full chain needs 1024 for the return-trip plan search from the
    // remote OrchardFarm through the high-branching VillageSquare hub.
    h.driver = AgentTickDriver::new(PlanningBudget {
        max_node_expansions: 1024,
        ..PlanningBudget::default()
    });
    h.driver.enable_tracing();

    let initial_authoritative_apples =
        total_authoritative_commodity_quantity(&h.world, CommodityKind::Apple);
    let initial_consumer_hunger = h.agent_hunger(consumer);

    let mut merchant_left_home = false;
    let mut merchant_acquired_apples = false;
    let mut merchant_returned_with_apples = false;
    let mut consumer_acquired_apples = false;
    let mut consumer_hunger_decreased = false;

    for _ in 0..500 {
        h.step_once();

        let merchant_place = h.world.effective_place(merchant);
        let merchant_apples = h.agent_commodity_qty(merchant, CommodityKind::Apple);
        let authoritative_apples =
            total_authoritative_commodity_quantity(&h.world, CommodityKind::Apple);

        merchant_left_home |=
            h.world.is_in_transit(merchant) || merchant_place != Some(general_store);
        merchant_acquired_apples |= merchant_apples > Quantity(0);
        merchant_returned_with_apples |=
            merchant_place == Some(general_store) && merchant_apples > Quantity(0);

        let consumer_apples = h.agent_commodity_qty(consumer, CommodityKind::Apple);
        consumer_acquired_apples |= consumer_apples > Quantity(0);
        consumer_hunger_decreased |= h.agent_hunger(consumer) < initial_consumer_hunger;

        assert!(
            authoritative_apples <= initial_authoritative_apples,
            "authoritative apples should never increase: initial={initial_authoritative_apples}, now={authoritative_apples}"
        );

        // Exit early once the full chain completes.
        if merchant_returned_with_apples && consumer_acquired_apples && consumer_hunger_decreased {
            break;
        }
    }

    // Supply chain assertions.
    assert!(
        merchant_left_home,
        "Merchant should leave General Store to restock"
    );
    assert!(
        merchant_acquired_apples,
        "Merchant should acquire apples (via harvest at Orchard Farm)"
    );
    assert!(
        merchant_returned_with_apples,
        "Merchant should return to General Store with apples"
    );
    assert!(
        consumer_acquired_apples,
        "Consumer should acquire apples via trade with merchant"
    );
    assert!(
        consumer_hunger_decreased,
        "Consumer hunger should decrease after eating acquired apples"
    );
    assert!(!h.agent_is_dead(producer), "Producer must stay alive");
    assert!(!h.agent_is_dead(merchant), "Merchant must stay alive");
    assert!(!h.agent_is_dead(consumer), "Consumer must stay alive");

    (
        hash_world(&h.world).unwrap(),
        hash_event_log(&h.event_log).unwrap(),
    )
}

// ---------------------------------------------------------------------------
// Test entry points
// ---------------------------------------------------------------------------

#[test]
fn test_merchant_restock_with_traces() {
    let _ = run_merchant_restock_with_traces(Seed([100; 32]));
}

#[test]
fn test_merchant_restock_replay() {
    let first = run_merchant_restock_with_traces(Seed([101; 32]));
    let second = run_merchant_restock_with_traces(Seed([101; 32]));

    assert_eq!(
        first, second,
        "Merchant restock scenario should replay deterministically"
    );
}

#[test]
fn test_consumer_trade_with_traces() {
    let _ = run_consumer_trade_with_traces(Seed([102; 32]));
}

#[test]
fn test_consumer_trade_replay() {
    let first = run_consumer_trade_with_traces(Seed([103; 32]));
    let second = run_consumer_trade_with_traces(Seed([103; 32]));

    assert_eq!(
        first, second,
        "Consumer trade scenario should replay deterministically"
    );
}

/// Full combined supply chain test (merchant restock → consumer trade in one sim).
///
/// Ignored: blocked on `specs/S10-bilateral-trade-negotiation.md`. The consumer
/// correctly observes the merchant's return with apples (perception works) and
/// generates a trade plan, but the merchant rejects the fixed 1-coin offer as
/// `InsufficientPayment` given its enterprise weight and scarce stock. The trade
/// system lacks price negotiation — every offer is hardcoded at 1:1. S10
/// introduces multi-round bilateral negotiation with variable pricing derived
/// from concrete agent state, which will unblock this test.
#[test]
#[ignore]
fn test_full_supply_chain() {
    let _ = run_full_supply_chain(Seed([104; 32]));
}

#[test]
#[ignore]
fn test_full_supply_chain_replay() {
    let first = run_full_supply_chain(Seed([105; 32]));
    let second = run_full_supply_chain(Seed([105; 32]));

    assert_eq!(
        first, second,
        "Full supply chain scenario should replay deterministically"
    );
}
