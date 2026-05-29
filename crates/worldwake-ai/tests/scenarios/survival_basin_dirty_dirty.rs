//! S176 focused golden: a wash basin degraded above its effective-dirtiness
//! threshold blocks Wash, the planner inserts `clean_wash_basin` as a
//! prerequisite, and a lawful wash recovers — proven end-to-end on the authored
//! `survival-basin-dirty-dirty.ron` scenario.

use std::collections::BTreeSet;
use std::path::PathBuf;

use crate::golden_harness::*;
use worldwake_ai::{DegradedSelfCareCause, DegradedSelfCareOutcome, SurvivalForensicExtractor};
use worldwake_cli::scenario::{load_scenario_file, spawn_scenario};
use worldwake_core::{CommodityKind, EntityId, Quantity, Tick};
use worldwake_sim::ActionTraceKind;

const TICKS: u32 = 80;

fn scenario_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../scenarios/survival-basin-dirty-dirty.ron")
}

fn washer(h: &GoldenHarness) -> EntityId {
    h.world
        .query_name_and_agent_data()
        .find(|(_, name, _)| name.0 == "Washer")
        .map(|(entity, _, _)| entity)
        .expect("scenario should include Washer")
}

// ---------------------------------------------------------------------------
// Scenario: Too-dirty basin forces clean-then-wash recovery (S176 D2/D6/D8)
// ---------------------------------------------------------------------------
//
// Systems: AI, Needs, Search, Perception
// GoalKinds: Wash
// ActionDomains: Needs
// Places: Wash Camp
// Principles: 1, 3, 8, 20, 31
//
// Setup: a single basin authored above its effective-dirtiness threshold blocks
// the Wash precondition; the agent is under sustained dirtiness pressure.
//
// Proves: the planner inserts `clean_wash_basin` ahead of the wash, the wash
// recovers afterward, cleaning leaves a concrete Waste lot, and the degraded
// self-care leaves a typed `DegradedSelfCareOpportunity { BasinTooDirty,
// Cleaned }` forensic record.
//
// Chain: authored too-dirty basin -> TargetWashBasinNotTooDirty blocks wash ->
// GOAP inserts clean_wash_basin prerequisite -> dirtiness reset + Waste lot ->
// lawful wash commits.
#[test]
fn too_dirty_basin_forces_clean_then_wash_recovery() {
    let def = load_scenario_file(&scenario_path()).expect("scenario should parse");
    let spawned = spawn_scenario(&def).expect("scenario should spawn");
    let mut h = GoldenHarness::from_simulation_state(&spawned.state);
    h.driver.enable_tracing();
    h.enable_action_tracing();

    let agent = washer(&h);
    let thresholds = h
        .world
        .get_component_drive_thresholds(agent)
        .copied()
        .expect("agent should have drive thresholds");
    let mut extractor = SurvivalForensicExtractor::new(agent);

    for tick_num in 0..TICKS {
        h.step_once();
        let tick = Tick(u64::from(tick_num));
        let needs = h
            .world
            .get_component_homeostatic_needs(agent)
            .copied()
            .expect("agent should always have needs");
        observe_critical_windows(&mut extractor, &h, agent, tick, &needs, &thresholds);
    }

    let committed: BTreeSet<String> = h
        .action_trace_sink()
        .expect("action tracing enabled")
        .events_for(agent)
        .iter()
        .filter(|event| matches!(event.kind, ActionTraceKind::Committed { .. }))
        .map(|event| event.action_name.clone())
        .collect();

    assert!(
        committed.contains("clean_wash_basin"),
        "planner should insert clean_wash_basin to unblock the too-dirty basin; committed={committed:?}"
    );
    assert!(
        committed.contains("wash"),
        "a lawful wash should recover after cleaning; committed={committed:?}"
    );

    // Cleaning leaves a concrete Waste lot on the ground (FND-4).
    let waste_lots = h
        .world
        .query_item_lot()
        .filter(|(_, lot)| lot.commodity == CommodityKind::Waste && lot.quantity > Quantity(0))
        .count();
    assert!(
        waste_lots >= 1,
        "cleaning should leave at least one Waste lot on the ground"
    );

    // Forensics: a BasinTooDirty / Cleaned record in the dirtiness critical window.
    let reports = extractor.finalize();
    let recorded = reports
        .iter()
        .flat_map(|report| &report.frames)
        .any(|frame| {
            frame.degraded_self_care_opportunities.iter().any(|record| {
                record.cause == DegradedSelfCareCause::BasinTooDirty
                    && record.outcome == DegradedSelfCareOutcome::Cleaned
            })
        });
    assert!(
        recorded,
        "degraded self-care forensics should record BasinTooDirty/Cleaned"
    );
}

// ---------------------------------------------------------------------------
// Scenario: Basin-degradation run is replay-deterministic (FND-31)
// ---------------------------------------------------------------------------
#[test]
fn basin_degradation_run_is_replay_deterministic() {
    let run = || {
        let def = load_scenario_file(&scenario_path()).expect("scenario should parse");
        let spawned = spawn_scenario(&def).expect("scenario should spawn");
        let mut h = GoldenHarness::from_simulation_state(&spawned.state);
        for _ in 0..TICKS {
            h.step_once();
        }
        worldwake_core::hash_serializable(&h.world).expect("world should hash canonically")
    };
    assert_eq!(
        run(),
        run(),
        "basin degradation run should be replay-deterministic"
    );
}
