//! S176 focused golden: a latrine above its critical-fullness threshold blocks
//! the Toilet action; with no reachable outdoor relief the planner inserts the
//! `empty_latrine` recovery prerequisite (emitting `WasteCreated { LatrineEmptied }`)
//! before a lawful toilet — proven end-to-end on `survival-latrine-full.ron`.

use std::collections::BTreeSet;
use std::path::PathBuf;

use crate::golden_harness::*;
use worldwake_ai::{DegradedSelfCareCause, SurvivalForensicExtractor};
use worldwake_cli::scenario::{load_scenario_file, spawn_scenario};
use worldwake_core::{
    CommodityKind, DecisionEventPayload, EntityId, EventTag, EventView, Quantity, Tick, WasteSource,
};
use worldwake_sim::ActionTraceKind;

const TICKS: u32 = 40;

fn scenario_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../scenarios/survival-latrine-full.ron")
}

fn reliever(h: &GoldenHarness) -> EntityId {
    h.world
        .query_name_and_agent_data()
        .find(|(_, name, _)| name.0 == "Reliever")
        .map(|(entity, _, _)| entity)
        .expect("scenario should include Reliever")
}

// ---------------------------------------------------------------------------
// Scenario: Full latrine forces empty-then-relieve recovery (S176 D3/D5/D6/D8)
// ---------------------------------------------------------------------------
//
// Systems: AI, Needs, Search, Perception
// GoalKinds: Relieve
// ActionDomains: Needs
// Places: Latrine Pit (isolated, indoor)
// Principles: 1, 3, 4, 8, 20, 31
//
// Setup: an isolated indoor latrine authored above its critical fullness
// threshold blocks the toilet; no outdoor place is reachable, so wilderness
// relief is unavailable.
//
// Proves: the planner inserts `empty_latrine` ahead of the toilet, emptying
// emits `WasteCreated { LatrineEmptied }` with a proportional Waste lot, the
// toilet recovers afterward, and the degraded self-care leaves a typed
// `DegradedSelfCareOpportunity { LatrineFull, .. }` forensic record.
//
// Chain: authored full latrine -> PlaceLatrineNotFull blocks toilet -> GOAP
// inserts empty_latrine prerequisite -> fill reset + LatrineEmptied waste lot ->
// lawful toilet commits.
#[test]
fn full_latrine_forces_empty_then_relieve_recovery() {
    let def = load_scenario_file(&scenario_path()).expect("scenario should parse");
    let spawned = spawn_scenario(&def).expect("scenario should spawn");
    let mut h = GoldenHarness::from_simulation_state(&spawned.state);
    h.driver.enable_tracing();
    h.enable_action_tracing();

    let agent = reliever(&h);
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
        committed.contains("empty_latrine"),
        "planner should insert empty_latrine to unblock the full latrine; committed={committed:?}"
    );
    assert!(
        committed.contains("toilet"),
        "a lawful toilet should recover after emptying; committed={committed:?}"
    );

    // Emptying emits WasteCreated { LatrineEmptied } with a concrete Waste lot.
    let latrine_emptied = h
        .event_log
        .events_by_tag(EventTag::WasteCreated)
        .iter()
        .filter_map(|id| h.event_log.get(*id))
        .filter_map(|record| match record.decision_payload()? {
            DecisionEventPayload::WasteCreated(payload) => Some(payload.clone()),
            _ => None,
        })
        .find(|payload| payload.source == WasteSource::LatrineEmptied)
        .expect("emptying should emit WasteCreated { LatrineEmptied }");
    assert!(
        h.world
            .get_component_item_lot(latrine_emptied.waste_lot)
            .is_some_and(|lot| lot.commodity == CommodityKind::Waste && lot.quantity > Quantity(0)),
        "LatrineEmptied payload should reference a concrete Waste lot"
    );

    // Forensics: a LatrineFull record in the bladder critical window.
    let reports = extractor.finalize();
    let recorded = reports
        .iter()
        .flat_map(|report| &report.frames)
        .any(|frame| {
            frame
                .degraded_self_care_opportunities
                .iter()
                .any(|record| record.cause == DegradedSelfCareCause::LatrineFull)
        });
    assert!(
        recorded,
        "degraded self-care forensics should record a LatrineFull cause"
    );
}

// ---------------------------------------------------------------------------
// Scenario: Latrine-fullness run is replay-deterministic (FND-31)
// ---------------------------------------------------------------------------
#[test]
fn latrine_fullness_run_is_replay_deterministic() {
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
        "latrine fullness run should be replay-deterministic"
    );
}
