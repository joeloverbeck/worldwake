//! Golden test for deterministic replay fidelity.

mod golden_harness;

use golden_harness::*;
use worldwake_core::{
    hash_event_log, hash_world, CommodityKind, HomeostaticNeeds, MetabolismProfile, Quantity,
    ResourceSource, Seed, StateHash, UtilityProfile, WorkstationTag,
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
