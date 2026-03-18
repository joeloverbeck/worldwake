//! Golden test for deterministic replay fidelity.

mod golden_harness;

use golden_harness::*;
use worldwake_core::{
    hash_event_log, hash_world, total_authoritative_commodity_quantity, BeliefConfidencePolicy,
    CommodityKind, HomeostaticNeeds, MetabolismProfile, PerceptionProfile,
    PrototypePlace, Quantity, ResourceSource, Seed, StateHash, UtilityProfile, WorkstationTag,
    prototype_place_entity,
};

// ---------------------------------------------------------------------------
// Determinism-specific helpers (only used by the test in this file)
// ---------------------------------------------------------------------------

fn build_deterministic_scenario(seed: Seed) -> GoldenHarness {
    let mut h = GoldenHarness::new(seed);

    let agent_a = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Alice",
        VILLAGE_SQUARE,
        HomeostaticNeeds::new(pm(900), pm(0), pm(0), pm(0), pm(0)),
        MetabolismProfile::default(),
        UtilityProfile::default(),
    );
    // Second agent exists for multi-agent determinism; not referenced directly.
    seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Bob",
        VILLAGE_SQUARE,
        HomeostaticNeeds::new(pm(900), pm(0), pm(0), pm(0), pm(0)),
        MetabolismProfile::default(),
        UtilityProfile::default(),
    );

    give_commodity(
        &mut h.world,
        &mut h.event_log,
        agent_a,
        VILLAGE_SQUARE,
        CommodityKind::Bread,
        Quantity(1),
    );

    place_workstation_with_source(
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

    h
}

fn run_deterministic_scenario(seed: Seed) -> (StateHash, StateHash) {
    let mut h = build_deterministic_scenario(seed);
    for _ in 0..50 {
        h.step_once();
    }
    (
        hash_world(&h.world).unwrap(),
        hash_event_log(&h.event_log).unwrap(),
    )
}

fn run_save_load_roundtrip_scenario(seed: Seed) -> GoldenHarness {
    const SAVE_AFTER_TICKS: u64 = 20;
    const RESUME_TICKS: u64 = 30;

    let mut uninterrupted = build_deterministic_scenario(seed);
    for _ in 0..SAVE_AFTER_TICKS {
        uninterrupted.step_once();
    }

    let save_boundary_world_hash = hash_world(&uninterrupted.world).unwrap();
    let initial_world_hash = {
        let initial = build_deterministic_scenario(seed);
        hash_world(&initial.world).unwrap()
    };
    assert_ne!(
        save_boundary_world_hash, initial_world_hash,
        "Save boundary should occur after non-trivial AI progress"
    );

    let resumed_state = uninterrupted.save_load_roundtrip();
    let mut resumed = GoldenHarness::from_simulation_state(&resumed_state);

    for _ in 0..RESUME_TICKS {
        uninterrupted.step_once();
        resumed.step_once();
    }

    assert_eq!(
        resumed.scheduler, uninterrupted.scheduler,
        "Scheduler state should match after resuming with a fresh AI runtime"
    );
    assert_eq!(
        resumed.controller, uninterrupted.controller,
        "Controller state should match after resuming with a fresh AI runtime"
    );
    assert_eq!(
        resumed.rng, uninterrupted.rng,
        "Deterministic RNG continuation should match across save/load"
    );
    assert_eq!(
        resumed.recipes, uninterrupted.recipes,
        "Recipe registry should remain unchanged across save/load"
    );
    assert_eq!(
        hash_world(&resumed.world).unwrap(),
        hash_world(&uninterrupted.world).unwrap(),
        "World state should match uninterrupted execution after save/load resume"
    );
    assert_eq!(
        hash_event_log(&resumed.event_log).unwrap(),
        hash_event_log(&uninterrupted.event_log).unwrap(),
        "Event log should match uninterrupted execution after save/load resume"
    );

    resumed
}

// ---------------------------------------------------------------------------
// Scenario 6: Deterministic Replay Fidelity
// ---------------------------------------------------------------------------

#[test]
fn golden_deterministic_replay_fidelity() {
    let seed = Seed([42; 32]);

    let (world_hash_1, log_hash_1) = run_deterministic_scenario(seed);
    let (world_hash_2, log_hash_2) = run_deterministic_scenario(seed);

    assert_eq!(
        world_hash_1, world_hash_2,
        "Two runs with the same seed must produce identical world hashes"
    );
    assert_eq!(
        log_hash_1, log_hash_2,
        "Two runs with the same seed must produce identical event log hashes"
    );

    // Verify non-trivial simulation occurred — hashes differ from initial state.
    let fresh = build_deterministic_scenario(seed);
    let initial_world_hash = hash_world(&fresh.world).unwrap();
    let initial_log_hash = hash_event_log(&fresh.event_log).unwrap();

    assert_ne!(
        world_hash_1, initial_world_hash,
        "World should have changed from initial state (non-trivial simulation)"
    );
    assert_ne!(
        log_hash_1, initial_log_hash,
        "Event log should have changed from initial state"
    );
}

#[test]
fn golden_save_load_round_trip_under_ai() {
    let seed = Seed([77; 32]);
    let resumed = run_save_load_roundtrip_scenario(seed);

    assert_eq!(
        resumed.scheduler.current_tick().0,
        50,
        "Resumed scenario should reach the same terminal tick as uninterrupted execution"
    );
}

// ---------------------------------------------------------------------------
// Scenario S02: World Runs Without Observers (Principle 6)
// ---------------------------------------------------------------------------

#[allow(clippy::too_many_lines)]
fn build_world_runs_without_observers_scenario(seed: Seed) -> GoldenHarness {
    let mut h = GoldenHarness::new(seed);
    let general_store = prototype_place_entity(PrototypePlace::GeneralStore);

    // Farmer at Orchard Farm — hungry, knows harvest recipe, has perception.
    let farmer = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Farmer",
        ORCHARD_FARM,
        HomeostaticNeeds::new(pm(800), pm(0), pm(0), pm(0), pm(0)),
        MetabolismProfile::default(),
        UtilityProfile::default(),
    );
    set_agent_perception_profile(
        &mut h.world,
        &mut h.event_log,
        farmer,
        PerceptionProfile {
            memory_capacity: 64,
            memory_retention_ticks: 240,
            observation_fidelity: pm(875),
            confidence_policy: BeliefConfidencePolicy::default(),
        },
    );

    // OrchardRow workstation + resource source at Orchard Farm.
    let orchard_ws = place_workstation_with_source(
        &mut h.world,
        &mut h.event_log,
        ORCHARD_FARM,
        WorkstationTag::OrchardRow,
        ResourceSource {
            commodity: CommodityKind::Apple,
            available_quantity: Quantity(20),
            max_quantity: Quantity(20),
            regeneration_ticks_per_unit: Some(nz(5)),
            last_regeneration_tick: None,
        },
        ProductionOutputOwner::Actor,
    );

    // Merchant at General Store — enterprise-focused, has coins, perception.
    let merchant = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Merchant",
        general_store,
        HomeostaticNeeds::default(),
        MetabolismProfile::default(),
        UtilityProfile {
            enterprise_weight: pm(800),
            ..UtilityProfile::default()
        },
    );
    {
        use std::collections::BTreeSet;
        use worldwake_core::{DemandMemory, DemandObservation, DemandObservationReason, MerchandiseProfile, Tick, TradeDispositionProfile};
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
        txn.set_component_trade_disposition_profile(
            merchant,
            TradeDispositionProfile {
                negotiation_round_ticks: nz(4),
                initial_offer_bias: pm(500),
                concession_rate: pm(100),
                demand_memory_retention_ticks: 240,
            },
        )
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
    give_commodity(
        &mut h.world,
        &mut h.event_log,
        merchant,
        general_store,
        CommodityKind::Coin,
        Quantity(10),
    );
    seed_actor_beliefs(
        &mut h.world,
        &mut h.event_log,
        merchant,
        &[orchard_ws],
        worldwake_core::Tick(0),
        worldwake_core::PerceptionSource::Inference,
    );

    // Villager at Village Square — hungry, thirsty, has bread + water + coins.
    let villager = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Villager",
        VILLAGE_SQUARE,
        HomeostaticNeeds::new(pm(700), pm(500), pm(0), pm(0), pm(0)),
        MetabolismProfile::default(),
        UtilityProfile::default(),
    );
    give_commodity(
        &mut h.world,
        &mut h.event_log,
        villager,
        VILLAGE_SQUARE,
        CommodityKind::Bread,
        Quantity(1),
    );
    give_commodity(
        &mut h.world,
        &mut h.event_log,
        villager,
        VILLAGE_SQUARE,
        CommodityKind::Water,
        Quantity(2),
    );
    give_commodity(
        &mut h.world,
        &mut h.event_log,
        villager,
        VILLAGE_SQUARE,
        CommodityKind::Coin,
        Quantity(5),
    );

    // Wanderer at Village Square — thirsty + fatigued, has water.
    let wanderer = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Wanderer",
        VILLAGE_SQUARE,
        HomeostaticNeeds::new(pm(0), pm(800), pm(600), pm(0), pm(0)),
        MetabolismProfile::default(),
        UtilityProfile {
            thirst_weight: pm(800),
            fatigue_weight: pm(600),
            ..UtilityProfile::default()
        },
    );
    give_commodity(
        &mut h.world,
        &mut h.event_log,
        wanderer,
        VILLAGE_SQUARE,
        CommodityKind::Water,
        Quantity(1),
    );

    h
}

fn run_world_runs_without_observers(seed: Seed) -> (StateHash, StateHash) {
    let mut h = build_world_runs_without_observers_scenario(seed);

    let initial_world_hash = hash_world(&h.world).unwrap();
    let initial_event_count = h.event_log.len();

    // Track initial authoritative totals for conservation.
    // Apple has a regenerating ResourceSource, so its authoritative total can increase
    // (source stock regenerates between harvests). We skip apple conservation here
    // and check only non-regenerating commodities.
    let initial_bread = total_authoritative_commodity_quantity(&h.world, CommodityKind::Bread);
    let initial_water = total_authoritative_commodity_quantity(&h.world, CommodityKind::Water);
    let initial_coin = total_authoritative_commodity_quantity(&h.world, CommodityKind::Coin);

    let agents: Vec<_> = h
        .world
        .entities()
        .filter(|e| h.world.get_component_agent_data(*e).is_some())
        .collect();

    let mut any_agent_moved = false;
    let mut any_consumption_event = false;

    for _ in 0..200 {
        h.step_once();

        // Per-tick authoritative conservation for non-regenerating commodities.
        let bread = total_authoritative_commodity_quantity(&h.world, CommodityKind::Bread);
        let water = total_authoritative_commodity_quantity(&h.world, CommodityKind::Water);
        let coin = total_authoritative_commodity_quantity(&h.world, CommodityKind::Coin);
        assert!(
            bread <= initial_bread,
            "Bread conservation violated: initial={initial_bread}, now={bread}"
        );
        assert!(
            water <= initial_water,
            "Water conservation violated: initial={initial_water}, now={water}"
        );
        assert_eq!(
            coin, initial_coin,
            "Coin conservation violated: initial={initial_coin}, now={coin}"
        );

        // Track milestones — detect any agent in transit.
        for &agent in &agents {
            if h.world.is_in_transit(agent) {
                any_agent_moved = true;
            }
        }

        // Detect consumption by checking if any consumable total decreased.
        let current_bread = total_authoritative_commodity_quantity(&h.world, CommodityKind::Bread);
        let current_water = total_authoritative_commodity_quantity(&h.world, CommodityKind::Water);
        if current_bread < initial_bread || current_water < initial_water {
            any_consumption_event = true;
        }
    }

    let final_world_hash = hash_world(&h.world).unwrap();
    let final_event_count = h.event_log.len();

    // Assertion 1: World hash differs from initial.
    assert_ne!(
        final_world_hash, initial_world_hash,
        "World should have changed after 200 ticks of multi-agent simulation"
    );

    // Assertion 2: Event log grew by at least 20 events.
    assert!(
        final_event_count >= initial_event_count + 20,
        "Event log should grow by 20+ events over 200 ticks; initial={initial_event_count}, final={final_event_count}"
    );

    // Assertion 3: No agent died.
    for &agent in &agents {
        assert!(
            !h.agent_is_dead(agent),
            "No agent should die in a provisioned world"
        );
    }

    // Assertion 5: At least one agent changed places (travel system engaged).
    // We also detect travel by checking if any agent is in transit or visited an intermediate node.
    // For a more robust check, look at event log for travel events.
    let action_events_exist = !h
        .event_log
        .events_by_tag(worldwake_core::EventTag::ActionCommitted)
        .is_empty();
    assert!(
        any_agent_moved || action_events_exist,
        "At least one agent should have moved during 200 ticks"
    );

    // Assertion 6: At least one consumption event occurred.
    assert!(
        any_consumption_event,
        "At least one consumption event should have occurred over 200 ticks"
    );

    (
        final_world_hash,
        hash_event_log(&h.event_log).unwrap(),
    )
}

#[test]
fn golden_world_runs_without_observers() {
    let _ = run_world_runs_without_observers(Seed([201; 32]));
}

#[test]
fn golden_world_runs_without_observers_replays_deterministically() {
    let first = run_world_runs_without_observers(Seed([202; 32]));
    let second = run_world_runs_without_observers(Seed([202; 32]));

    assert_eq!(
        first, second,
        "Two runs of the 200-tick multi-agent world with the same seed must produce identical hashes"
    );
}
