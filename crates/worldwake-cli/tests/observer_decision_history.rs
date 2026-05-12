use std::path::PathBuf;
use std::process::Command;

use tempfile::tempdir;

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crate dir has workspace parent")
        .parent()
        .expect("workspace root")
        .to_path_buf()
}

fn scenario_path(relative: &str) -> PathBuf {
    workspace_root().join("crates/worldwake-cli").join(relative)
}

fn run_observer(relative_scenario_path: &str, ticks: u64) -> String {
    let scenario = scenario_path(relative_scenario_path);
    let output_dir = tempdir().expect("temp dir");
    let output = output_dir.path().join("observer-report.md");
    let status = Command::new(env!("CARGO_BIN_EXE_observer"))
        .arg(&scenario)
        .arg("--ticks")
        .arg(ticks.to_string())
        .arg("--output")
        .arg(&output)
        .status()
        .expect("observer binary should run");
    assert!(
        status.success(),
        "observer should succeed for {}",
        scenario.display()
    );
    std::fs::read_to_string(output).expect("observer report should exist")
}

fn decision_history_section(report: &str) -> &str {
    let start = report
        .find("## Section 3b — Decision History")
        .expect("decision history section start");
    let end = report[start..]
        .find("## Section 4 —")
        .map_or(report.len(), |offset| start + offset);
    &report[start..end]
}

#[test]
fn survival_baseline_decision_history_section_matches_golden() {
    let report = run_observer("../../scenarios/survival-baseline.ron", 5);
    let actual = decision_history_section(&report);
    let expected = include_str!("fixtures/observer_decision_history/survival_baseline_5_ticks.md");

    assert_eq!(actual.trim_end(), expected.trim_end());
}
