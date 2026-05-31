//! SANBASINCLEAN-001 golden: proactive wash-basin cleaning under realistic
//! food/water competition.
//!
//! Three agents share ONE default-scale wash basin (`max_effective_dirtiness:
//! 1000`) at a self-sufficient camp for 1440 ticks while hunger and thirst stay
//! live (unlike `survival-sanitation-breakdown-1440`, which zeroes food/water
//! and relies on the hard `TargetWashBasinNotTooDirty` block). The agents'
//! default `wash_worthwhile_effectiveness_floor` (500‰) triggers
//! `clean_wash_basin` once shared use degrades the basin below half
//! effectiveness — well before the hard block — so the FND-11 maintenance
//! dampener engages in the regime where it previously stalled.
//!
//! Proves: all agents survive within the authored dirtiness critical-run limit;
//! the basin's `dirtiness_level` crosses the proactive floor and is reset by
//! cleaning at least once; and the basin never reaches its hard block, so the
//! recovery was the *proactive* trigger, not the absolute legality gate.

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use crate::golden_harness::*;
use worldwake_cli::scenario::{load_scenario_file, spawn_scenario};
use worldwake_core::{EntityId, WorkstationTag};
use worldwake_sim::ActionTraceKind;

const TICKS: u32 = 1440;

fn scenario_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../scenarios/survival-basin-competition-1440.ron")
}

fn named_agents(h: &GoldenHarness) -> BTreeMap<String, EntityId> {
    h.world
        .query_name_and_agent_data()
        .map(|(entity, name, _)| (name.0.clone(), entity))
        .collect()
}

fn shared_basin(h: &GoldenHarness) -> EntityId {
    h.world
        .entities()
        .find(|entity| {
            h.world
                .get_component_workstation_marker(*entity)
                .is_some_and(|marker| marker.0 == WorkstationTag::WashBasin)
        })
        .expect("scenario should include a wash basin facility")
}

struct CompetitionObservation {
    contract: worldwake_cli::scenario::types::SurvivalHealthContractDef,
    alive: BTreeMap<String, bool>,
    thresholds: BTreeMap<String, worldwake_core::DriveThresholds>,
    runs: BTreeMap<String, SurvivalNeedRunTracker>,
    committed_per_agent: BTreeMap<String, BTreeSet<String>>,
    /// Highest `dirtiness_level` the shared basin reached over the run.
    basin_max_dirtiness: u16,
    /// The basin's authored hard block (`max_effective_dirtiness`).
    basin_hard_block: u16,
    /// The proactive trigger point: dirtiness above which a 500‰-floor agent
    /// stops finding the wash worthwhile (`max_effective * (1000 - 500) / 1000`).
    basin_floor_trigger: u16,
    /// True iff the basin crossed the proactive trigger and was then reset by a
    /// cleaning commit (basin dirtiness only ever drops via `clean_wash_basin`;
    /// washing only raises it, and the basin has no natural decay).
    basin_cleaned_after_crossing_floor: bool,
}

fn run() -> CompetitionObservation {
    let def = load_scenario_file(&scenario_path()).expect("scenario should parse");
    let spawned = spawn_scenario(&def).expect("scenario should spawn");
    let mut h = GoldenHarness::from_simulation_state(&spawned.state);
    h.driver.enable_tracing();
    h.enable_action_tracing();

    let contract =
        expect_survival_health_contract(def.survival_health_contract.as_ref(), "basin competition")
            .clone();
    let agents = named_agents(&h);
    let basin = shared_basin(&h);
    let basin_state = h
        .world
        .get_component_wash_basin_state(basin)
        .cloned()
        .expect("wash basin should carry WashBasinState");
    let basin_hard_block = basin_state.max_effective_dirtiness.value();
    // The default worthwhile-wash floor is 500‰; cleaning triggers once the
    // effective fraction (max_effective - dirtiness) / max_effective drops below
    // it, i.e. once dirtiness exceeds max_effective * (1000 - 500) / 1000.
    let basin_floor_trigger =
        u16::try_from(u32::from(basin_hard_block) * (1000 - 500) / 1000).unwrap_or(u16::MAX);

    let thresholds: BTreeMap<String, _> = agents
        .iter()
        .map(|(name, agent)| {
            (
                name.clone(),
                h.world
                    .get_component_drive_thresholds(*agent)
                    .copied()
                    .expect("agent should have drive thresholds"),
            )
        })
        .collect();
    let mut runs: BTreeMap<String, SurvivalNeedRunTracker> = agents
        .keys()
        .cloned()
        .map(|name| (name, SurvivalNeedRunTracker::default()))
        .collect();

    let mut basin_max_dirtiness = basin_state.dirtiness_level.value();
    let mut prev_dirtiness = basin_state.dirtiness_level.value();
    let mut basin_cleaned_after_crossing_floor = false;

    for _ in 0..TICKS {
        h.step_once();

        for (name, agent) in &agents {
            let needs = h
                .world
                .get_component_homeostatic_needs(*agent)
                .copied()
                .expect("agent should always have needs");
            runs.get_mut(name)
                .unwrap()
                .observe(&needs, thresholds.get(name).unwrap());
        }

        let dirtiness = h
            .world
            .get_component_wash_basin_state(basin)
            .map_or(prev_dirtiness, |state| state.dirtiness_level.value());
        basin_max_dirtiness = basin_max_dirtiness.max(dirtiness);
        // The basin only loses dirtiness through a cleaning commit (no decay,
        // washing only adds). A drop while the basin sat above the proactive
        // trigger is the proactive clean we are proving.
        if dirtiness < prev_dirtiness && prev_dirtiness > basin_floor_trigger {
            basin_cleaned_after_crossing_floor = true;
        }
        prev_dirtiness = dirtiness;
    }

    let action_sink = h.action_trace_sink().expect("action tracing enabled");
    let committed_per_agent = agents
        .iter()
        .map(|(name, agent)| {
            let committed = action_sink
                .events_for(*agent)
                .iter()
                .filter(|event| matches!(event.kind, ActionTraceKind::Committed { .. }))
                .map(|event| event.action_name.clone())
                .collect::<BTreeSet<_>>();
            (name.clone(), committed)
        })
        .collect::<BTreeMap<_, _>>();

    let alive = agents
        .iter()
        .map(|(name, agent)| (name.clone(), !h.agent_is_dead(*agent)))
        .collect();

    CompetitionObservation {
        contract,
        alive,
        thresholds,
        runs,
        committed_per_agent,
        basin_max_dirtiness,
        basin_hard_block,
        basin_floor_trigger,
        basin_cleaned_after_crossing_floor,
    }
}

// ---------------------------------------------------------------------------
// Scenario: Proactive basin cleaning keeps shared self-care recoverable under
// realistic food/water competition (FND-11 dampener engagement)
// ---------------------------------------------------------------------------
//
// Systems: AI, Needs, Search, Contention, Perception
// GoalKinds: Wash, AcquireCommodity(SelfConsume), ConsumeOwnedCommodity, Relieve
// ActionDomains: Needs, Production
// Places: Riverside Camp
// Principles: 1, 2, 8, 11, 14, 26, 31
#[test]
#[ignore = "CI-only: long-running 1440-tick multi-agent proactive-cleaning competition; run via golden-survival workflow"]
fn proactive_cleaning_keeps_agents_recoverable_under_competition() {
    let obs = run();

    for (name, alive) in &obs.alive {
        assert!(
            *alive,
            "{name} should survive the 1440-tick competition run"
        );
    }

    for (name, runs) in &obs.runs {
        assert_authored_critical_runs(
            obs.contract.max_authored_critical_run_ticks,
            name,
            obs.thresholds.get(name).unwrap(),
            runs,
        );
    }

    let any_committed = |action: &str| {
        obs.committed_per_agent
            .values()
            .any(|set| set.contains(action))
    };
    assert!(
        any_committed("clean_wash_basin"),
        "agents should proactively clean the shared basin under competition; committed={:?}",
        obs.committed_per_agent
    );
    assert!(
        any_committed("wash"),
        "agents should still wash after cleaning; committed={:?}",
        obs.committed_per_agent
    );

    assert!(
        obs.basin_max_dirtiness > obs.basin_floor_trigger,
        "the shared basin should degrade past the proactive trigger ({}); observed max {}",
        obs.basin_floor_trigger,
        obs.basin_max_dirtiness
    );
    assert!(
        obs.basin_cleaned_after_crossing_floor,
        "the basin should be reset by proactive cleaning after crossing the worthwhile-wash floor"
    );
    // The recovery was the PROACTIVE trigger, not the hard authoritative block:
    // proactive cleaning keeps the basin from ever reaching max_effective_dirtiness.
    assert!(
        obs.basin_max_dirtiness < obs.basin_hard_block,
        "proactive cleaning should keep the basin below its hard block ({}); observed max {}",
        obs.basin_hard_block,
        obs.basin_max_dirtiness
    );
}

// ---------------------------------------------------------------------------
// Scenario: Proactive-cleaning competition run is replay-deterministic (FND-31)
// ---------------------------------------------------------------------------
#[test]
#[ignore = "CI-only: long-running 1440-tick replay-equivalence check"]
fn proactive_cleaning_competition_is_replay_deterministic() {
    let digest = || {
        let def = load_scenario_file(&scenario_path()).expect("scenario should parse");
        let spawned = spawn_scenario(&def).expect("scenario should spawn");
        let mut h = GoldenHarness::from_simulation_state(&spawned.state);
        for _ in 0..TICKS {
            h.step_once();
        }
        worldwake_core::hash_serializable(&h.world).expect("world should hash canonically")
    };
    assert_eq!(
        digest(),
        digest(),
        "the proactive-cleaning competition run should be replay-deterministic"
    );
}
