//! Golden regression coverage for aggregate scenario diagnostics.
#![allow(dead_code)]

#[path = "../golden_harness/mod.rs"]
mod golden_harness;

use std::{fs, path::PathBuf, sync::OnceLock};

use golden_harness::GoldenHarness;
use worldwake_ai::{
    DecisionOutcome, ScenarioDiagnosticsReport, build_scenario_diagnostics,
    decision_trace::{PlanAttemptTrace, RepairAttemptTrace},
};
use worldwake_cli::{
    diagnostics_json::{
        scenario_diagnostics_report_from_json, scenario_diagnostics_report_to_json_pretty,
    },
    scenario::{load_scenario_file, spawn_scenario},
};
use worldwake_core::Tick;

const SURVIVAL_TICKS: u64 = 1440;
const EXPECTED_DIAGNOSTICS_PATH: &str = "tests/fixtures/expected-scenario-diagnostics.json";
const EXPECTED_DIAGNOSTICS_JSON: &str =
    include_str!("../fixtures/expected-scenario-diagnostics.json");
static SURVIVAL_BASELINE_DIAGNOSTICS: OnceLock<ScenarioDiagnosticsReport> = OnceLock::new();

fn scenario_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../scenarios/survival-baseline.ron")
}

fn run_survival_baseline_diagnostics() -> ScenarioDiagnosticsReport {
    let def = load_scenario_file(&scenario_path()).expect("survival-baseline should parse");
    let spawned = spawn_scenario(&def).expect("survival-baseline should spawn");
    let mut harness = GoldenHarness::from_simulation_state(&spawned.state);
    harness.driver.enable_tracing();

    for _ in 0..SURVIVAL_TICKS {
        harness.step_once();
    }

    let decision_traces = harness
        .driver
        .trace_sink()
        .map_or_else(Vec::new, |sink| sink.traces().to_vec());
    let mut plan_traces = Vec::<PlanAttemptTrace>::new();
    let mut repair_traces = Vec::<RepairAttemptTrace>::new();
    for trace in &decision_traces {
        if let DecisionOutcome::Planning(planning) = &trace.outcome {
            plan_traces.extend(planning.planning.attempts.iter().cloned());
        }
        repair_traces.extend(trace.repair_attempts.iter().cloned());
    }

    build_scenario_diagnostics(
        &decision_traces,
        &plan_traces,
        &repair_traces,
        &harness.event_log,
        (Tick(0), Tick(SURVIVAL_TICKS.saturating_sub(1))),
    )
}

fn survival_baseline_diagnostics() -> &'static ScenarioDiagnosticsReport {
    SURVIVAL_BASELINE_DIAGNOSTICS.get_or_init(run_survival_baseline_diagnostics)
}

fn assert_schema_covered(report: &ScenarioDiagnosticsReport) {
    assert_eq!(report.tick_range, (Tick(0), Tick(1439)));
    assert!(!report.goal_pressure.candidates_emitted_by_kind.is_empty());
    assert!(!report.goal_pressure.candidates_emitted_by_slot.is_empty());
    assert!(
        !report
            .goal_pressure
            .candidates_suppressed_by_category
            .is_empty()
    );
    assert!(!report.goal_pressure.top_k_not_planned.is_empty());
    assert!(report.planning.plan_attempts > 0);
    assert!(!report.planning.plan_attempts_by_kind.is_empty());
    assert!(report.planning.plan_depth.n > 0);
    assert!(!report.planning.terminal_kind_distribution.is_empty());
    assert!(report.performance.opportunity_compiled_count.n > 0);
    assert!(report.performance.search_expansions.n > 0);
}

fn assert_top_n_fixture_has_overflow(report: &ScenarioDiagnosticsReport) {
    assert!(
        report.goal_pressure.candidates_emitted_by_kind.len() > 3,
        "survival-baseline diagnostics should exercise --diagnostics-top-n 3 overflow"
    );
    assert!(
        report.goal_pressure.candidates_suppressed_by_category.len() > 3,
        "survival-baseline diagnostics should exercise suppression top-N overflow"
    );
}

pub fn assert_survival_baseline_fixture_is_stable() {
    let report = survival_baseline_diagnostics();

    assert_schema_covered(report);
    assert_top_n_fixture_has_overflow(report);

    let encoded =
        scenario_diagnostics_report_to_json_pretty(report).expect("observer JSON should encode");
    let decoded =
        scenario_diagnostics_report_from_json(&encoded).expect("observer JSON should parse");
    assert_eq!(decoded, *report, "observer JSON should round-trip");

    let expected = EXPECTED_DIAGNOSTICS_JSON.trim_end_matches('\n');
    if std::env::var_os("WORLDWAKE_UPDATE_SCENARIO_DIAGNOSTICS_FIXTURE").is_some() {
        fs::write(
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(EXPECTED_DIAGNOSTICS_PATH),
            format!("{encoded}\n"),
        )
        .expect("scenario diagnostics fixture should be writable");
        return;
    }

    assert_eq!(
        encoded, expected,
        "scenario diagnostics fixture drifted; regenerate expected-scenario-diagnostics.json intentionally"
    );
}

pub fn assert_survival_baseline_replays_deterministically() {
    let first = survival_baseline_diagnostics();
    let second = run_survival_baseline_diagnostics();
    assert_eq!(*first, second, "diagnostics report should be deterministic");
}
