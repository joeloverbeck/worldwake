//! S176 CI-owned collision golden: three agents share ONE wash basin and ONE
//! latrine over 1440 ticks. Sanitation degrades under multi-agent use and
//! recovers through labor (`clean_wash_basin` / `empty_latrine`) or diverts to
//! wilderness relief. Proves: no deaths, the S176 recovery branches fire,
//! facility state crosses-and-recovers (commits of clean/empty imply the
//! degradation gate blocked then cleared), typed `DegradedSelfCareOpportunity`
//! records, the FND-14B belief barrier (no remote-facility synthesis — all
//! self-care is co-located), and replay equivalence.

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use crate::golden_harness::*;
use worldwake_ai::{DegradedSelfCareCause, SurvivalForensicExtractor};
use worldwake_cli::scenario::{load_scenario_file, spawn_scenario};
use worldwake_core::{EntityId, Tick};
use worldwake_sim::ActionTraceKind;

const TICKS: u32 = 1440;

fn scenario_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../scenarios/survival-sanitation-breakdown-1440.ron")
}

struct SanitationObservation {
    all_alive: bool,
    committed_per_agent: BTreeMap<String, BTreeSet<String>>,
    degraded_causes: BTreeSet<DegradedSelfCareCause>,
    /// True iff every committed `clean_wash_basin` / `empty_latrine` occurred
    /// while the acting agent was co-located with the (single) shared camp.
    all_recovery_colocated: bool,
}

fn named_agents(h: &GoldenHarness) -> BTreeMap<String, EntityId> {
    h.world
        .query_name_and_agent_data()
        .map(|(entity, name, _)| (name.0.clone(), entity))
        .collect()
}

fn run() -> SanitationObservation {
    let def = load_scenario_file(&scenario_path()).expect("scenario should parse");
    let spawned = spawn_scenario(&def).expect("scenario should spawn");
    let mut h = GoldenHarness::from_simulation_state(&spawned.state);
    h.driver.enable_tracing();
    h.enable_action_tracing();

    let agents = named_agents(&h);
    let camp = h.world.effective_place(*agents.values().next().unwrap());
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
    let mut extractors: BTreeMap<String, SurvivalForensicExtractor> = agents
        .iter()
        .map(|(name, agent)| (name.clone(), SurvivalForensicExtractor::new(*agent)))
        .collect();
    let mut all_recovery_colocated = true;

    for tick_num in 0..TICKS {
        h.step_once();
        let tick = Tick(u64::from(tick_num));
        let action_sink = h.action_trace_sink().expect("action tracing enabled");
        for (name, agent) in &agents {
            // Belief barrier: any committed recovery labor this tick must be at
            // the agent's co-located place (the single shared camp), never a
            // remote facility synthesized without belief.
            for event in action_sink.events_for_at(*agent, tick) {
                if matches!(event.kind, ActionTraceKind::Committed { .. })
                    && (event.action_name == "clean_wash_basin"
                        || event.action_name == "empty_latrine")
                    && h.world.effective_place(*agent) != camp
                {
                    all_recovery_colocated = false;
                }
            }
            let needs = h
                .world
                .get_component_homeostatic_needs(*agent)
                .copied()
                .expect("agent should always have needs");
            observe_critical_windows(
                extractors.get_mut(name).unwrap(),
                &h,
                *agent,
                tick,
                &needs,
                thresholds.get(name).unwrap(),
            );
        }
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

    let mut degraded_causes = BTreeSet::new();
    for extractor in extractors.into_values() {
        for report in extractor.finalize() {
            for frame in &report.frames {
                for record in &frame.degraded_self_care_opportunities {
                    degraded_causes.insert(record.cause);
                }
            }
        }
    }

    let all_alive = agents.values().all(|agent| !h.agent_is_dead(*agent));

    SanitationObservation {
        all_alive,
        committed_per_agent,
        degraded_causes,
        all_recovery_colocated,
    }
}

// ---------------------------------------------------------------------------
// Scenario: 1440-tick shared-facility sanitation collision and recovery
// ---------------------------------------------------------------------------
//
// Systems: AI, Needs, Search, Contention, Perception
// GoalKinds: Wash, Relieve, Sleep
// ActionDomains: Needs
// Places: Shared Camp
// Principles: 1, 7, 8, 11, 14, 20, 26, 31
#[test]
#[ignore = "CI-only: long-running 1440-tick multi-agent sanitation collision; run via golden-survival workflow"]
fn shared_facilities_degrade_and_recover_over_1440_ticks() {
    let obs = run();

    assert!(obs.all_alive, "all agents should survive the 1440-tick run");

    let any_committed = |action: &str| {
        obs.committed_per_agent
            .values()
            .any(|set| set.contains(action))
    };
    // Recovery labor proves the degradation gates blocked then cleared: the
    // planner only inserts clean_wash_basin / empty_latrine when the wash /
    // toilet precondition is blocked by basin dirtiness / latrine fullness.
    assert!(
        any_committed("clean_wash_basin"),
        "the shared basin should be cleaned at least once (it crossed its dirtiness threshold)"
    );
    assert!(
        any_committed("empty_latrine"),
        "the shared latrine should be emptied at least once (it crossed its fullness threshold)"
    );
    // The gated self-care still recovers afterward.
    assert!(any_committed("wash"), "agents should wash after cleaning");
    assert!(
        any_committed("toilet"),
        "agents should toilet after emptying"
    );

    // Typed forensic evidence for both degradation causes.
    assert!(
        obs.degraded_causes
            .contains(&DegradedSelfCareCause::BasinTooDirty),
        "forensics should record a BasinTooDirty degraded self-care cause"
    );
    assert!(
        obs.degraded_causes
            .contains(&DegradedSelfCareCause::LatrineFull),
        "forensics should record a LatrineFull degraded self-care cause"
    );

    // Belief barrier: all recovery labor was co-located, never a remote
    // synthesized cleaning prerequisite (FND-14B).
    assert!(
        obs.all_recovery_colocated,
        "cleaning/emptying must only occur at the agent's co-located facility"
    );
}

// ---------------------------------------------------------------------------
// Scenario: 1440-tick sanitation collision is replay-deterministic (FND-31)
// ---------------------------------------------------------------------------
#[test]
#[ignore = "CI-only: long-running 1440-tick replay-equivalence check"]
fn shared_facility_collision_is_replay_deterministic() {
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
        "the 1440-tick sanitation collision should be replay-deterministic"
    );
}
