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
//! is blocked by production issues documented in `tickets/SUPPLYCHAINFIX-001.md`:
//! plan search budget exhaustion at hub nodes, BestEffort silent failures, and
//! merchant goal oscillation after restock return.

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

    // Multi-agent scenarios trigger SnapshotChanged replanning every tick.
    // From hub nodes (VillageSquare, 7+ edges), the default 512-expansion budget
    // exhausts before finding multi-hop plans. 1024 is sufficient.
    // See tickets/SUPPLYCHAINFIX-001.md Issue 1 for the production fix.
    h.driver = AgentTickDriver::new(PlanningBudget {
        max_node_expansions: 1024,
        ..PlanningBudget::default()
    });

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
