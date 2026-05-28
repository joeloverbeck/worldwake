//! S175 Scenario A golden coverage: exhaustion collapse cascade.
//!
//! Proves the full fatigue-collapse chain end to end: repeated hostile-proximity
//! sleep interruptions keep fatigue pinned critical, fatigue critical exposure
//! accumulates, the S175 consequence path creates an Exhaustion deprivation
//! wound (and worsens it), wound load eventually exceeds capacity, and the agent
//! dies with `DeathCause::NeedDeprivation { need: Fatigue }`. The derived
//! `CriticalWindowReport.exhaustion_collapse_observed` flag is set, and the
//! aggregated `failed_rest_opportunities` records carry `HostileProximity`.

use std::path::PathBuf;

use crate::golden_harness::*;
use worldwake_ai::survival_forensics::FailedRestKind;
use worldwake_ai::{CriticalWindowReport, SurvivalForensicExtractor};
use worldwake_cli::scenario::{load_scenario_file, spawn_scenario, types::ScenarioDef};
use worldwake_core::{
    DeathCause, DeprivationKind, EntityId, HomeostaticNeedId, Permille, SleepFailureCause,
    StateHash, Tick, hash_serializable,
};
use worldwake_sim::ActionTraceKind;

const TICK_BUDGET: u32 = 240;

#[derive(Clone, Debug, Eq, PartialEq)]
struct ExhaustionCollapseObservation {
    first_exhaustion_wound_tick: Option<Tick>,
    fatigue_ticks_reset_after_wound: bool,
    exhaustion_wound_worsened: bool,
    death_tick: Option<Tick>,
    death_cause: Option<DeathCause>,
    post_death_started_actions: Vec<String>,
    exhaustion_collapse_observed: bool,
    interrupted_failed_rest: usize,
    non_hostile_failed_rest: usize,
    event_log_hash: StateHash,
}

fn scenario_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../scenarios/survival-exhaustion-collapse.ron")
}

fn load_harness() -> (GoldenHarness, ScenarioDef) {
    let def =
        load_scenario_file(&scenario_path()).expect("exhaustion collapse scenario should parse");
    let spawned = spawn_scenario(&def).expect("exhaustion collapse scenario should spawn");
    let mut h = GoldenHarness::from_simulation_state(&spawned.state);
    h.driver.enable_tracing();
    h.enable_action_tracing();
    (h, def)
}

fn named_agent(h: &GoldenHarness, name: &str) -> EntityId {
    h.world
        .query_name_and_agent_data()
        .find_map(|(entity, agent_name, _)| (agent_name.0 == name).then_some(entity))
        .expect("named scenario agent should exist")
}

fn exhaustion_wound_severity(h: &GoldenHarness, agent: EntityId) -> Option<Permille> {
    h.world
        .get_component_wound_list(agent)
        .and_then(|wounds| wounds.find_deprivation_wound(DeprivationKind::Exhaustion))
        .map(|wound| wound.severity)
}

fn observe_collapse() -> ExhaustionCollapseObservation {
    let (mut h, _def) = load_harness();
    let aster = named_agent(&h, "Aster");
    let thresholds = h
        .world
        .get_component_drive_thresholds(aster)
        .copied()
        .expect("Aster should carry drive thresholds");
    let mut extractor = SurvivalForensicExtractor::new(aster);

    let mut first_exhaustion_wound_tick = None;
    let mut fatigue_ticks_reset_after_wound = false;
    let mut exhaustion_wound_worsened = false;
    let mut previous_exhaustion_severity: Option<Permille> = None;
    let mut death_tick = None;
    let mut death_cause = None;

    for tick_num in 0..TICK_BUDGET {
        h.step_once();
        let tick = Tick(u64::from(tick_num));
        let needs = *h
            .world
            .get_component_homeostatic_needs(aster)
            .expect("Aster should carry needs");
        observe_critical_windows(&mut extractor, &h, aster, tick, &needs, &thresholds);

        if let Some(severity) = exhaustion_wound_severity(&h, aster) {
            if first_exhaustion_wound_tick.is_none() {
                first_exhaustion_wound_tick = Some(tick);
                // After the wound is created this tick, the fatigue critical
                // exposure counter must reset to 0 (S175 D2).
                fatigue_ticks_reset_after_wound = h
                    .world
                    .get_component_deprivation_exposure(aster)
                    .is_some_and(|exposure| exposure.fatigue_critical_ticks == 0);
            }
            if let Some(previous) = previous_exhaustion_severity
                && severity > previous
            {
                exhaustion_wound_worsened = true;
            }
            previous_exhaustion_severity = Some(severity);
        }

        if let Some(dead_at) = h.world.get_component_dead_at(aster).copied() {
            death_tick = Some(dead_at.tick);
            death_cause = Some(dead_at.cause);
            // Run a few more ticks to confirm no post-death actions start.
            for _ in 0..3 {
                h.step_once();
            }
            break;
        }
    }

    let post_death_started_actions = death_tick
        .map(|dead_tick| {
            h.action_trace_sink()
                .expect("action tracing should be enabled")
                .events_for(aster)
                .into_iter()
                .filter(|event| {
                    event.tick.0 > dead_tick.0
                        && matches!(event.kind, ActionTraceKind::Started { .. })
                })
                .map(|event| event.action_name.clone())
                .collect()
        })
        .unwrap_or_default();

    let reports = extractor.finalize();
    let exhaustion_collapse_observed = reports
        .iter()
        .any(|report| report.exhaustion_collapse_observed);
    let (interrupted_failed_rest, non_hostile_failed_rest) = classify_failed_rest(&reports);

    ExhaustionCollapseObservation {
        first_exhaustion_wound_tick,
        fatigue_ticks_reset_after_wound,
        exhaustion_wound_worsened,
        death_tick,
        death_cause,
        post_death_started_actions,
        exhaustion_collapse_observed,
        interrupted_failed_rest,
        non_hostile_failed_rest,
        event_log_hash: hash_serializable(&h.event_log).expect("event log should hash"),
    }
}

fn classify_failed_rest(reports: &[CriticalWindowReport]) -> (usize, usize) {
    let mut hostile = 0;
    let mut other = 0;
    for opportunity in reports
        .iter()
        .filter(|report| report.need == HomeostaticNeedId::Fatigue)
        .flat_map(|report| &report.frames)
        .flat_map(|frame| &frame.failed_rest_opportunities)
    {
        if opportunity.kind
            == (FailedRestKind::Interrupted {
                cause: SleepFailureCause::HostileProximity,
            })
        {
            hostile += 1;
        } else {
            other += 1;
        }
    }
    (hostile, other)
}

// Scenario 486: S175 Exhaustion Collapse Cascade
// Setup: A critically tired agent rough-sleeps in the open with a permanently co-located hostile, so every Sleep attempt is interrupted via HostileProximity and rough sleep recovers nothing.
// Proves: sustained fatigue critical exposure creates an Exhaustion deprivation wound, resets the exposure counter, worsens the wound across a second interval, and ends in wound-load death attributed to Fatigue; the failed-rest forensic chain and the exhaustion_collapse_observed flag are intact.
// Chain: hostile co-location -> HostileProximity sleep interruption -> DeprivationExposure fatigue ticks -> Exhaustion wound (WoundCause::Deprivation) -> wound load >= capacity -> DeadAt(NeedDeprivation { Fatigue }) -> exhaustion_collapse_observed.
#[test]
#[ignore = "CI-only: long-horizon exhaustion collapse; run via golden-survival workflow"]
fn scenario_a_exhaustion_collapse() {
    let observation = observe_collapse();

    assert!(
        observation.first_exhaustion_wound_tick.is_some(),
        "an Exhaustion wound should be created from sustained critical fatigue: {observation:#?}"
    );
    assert!(
        observation.fatigue_ticks_reset_after_wound,
        "fatigue critical exposure should reset to 0 on Exhaustion wound creation: {observation:#?}"
    );
    assert!(
        observation.exhaustion_wound_worsened,
        "continued failure should worsen the Exhaustion wound before death: {observation:#?}"
    );
    assert_eq!(
        observation.death_cause,
        Some(DeathCause::NeedDeprivation {
            need: HomeostaticNeedId::Fatigue,
        }),
        "wound-load death should be attributed to Fatigue: {observation:#?}"
    );
    assert!(
        observation.post_death_started_actions.is_empty(),
        "no actions should start after exhaustion-collapse death: {observation:#?}"
    );
    assert!(
        observation.exhaustion_collapse_observed,
        "the fatigue critical window should record exhaustion_collapse_observed: {observation:#?}"
    );
    assert!(
        observation.interrupted_failed_rest > 0,
        "the collapse should be fed by hostile-proximity interrupted rest: {observation:#?}"
    );
    assert_eq!(
        observation.non_hostile_failed_rest, 0,
        "every failed-rest record leading to collapse should be HostileProximity: {observation:#?}"
    );
}

#[test]
#[ignore = "CI-only: long-horizon exhaustion collapse; run via golden-survival workflow"]
fn scenario_a_exhaustion_collapse_replays_deterministically() {
    let first = observe_collapse();
    let second = observe_collapse();

    assert_eq!(first.event_log_hash, second.event_log_hash);
    assert_eq!(first, second);
}
