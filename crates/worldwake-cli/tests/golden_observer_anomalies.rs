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

fn count_anomalies_of_kind(report: &str, kind: &str) -> usize {
    report
        .lines()
        .filter(|line| line.starts_with("### Anomaly ") && line.contains(kind))
        .count()
}

fn anomaly_headers_of_kind<'a>(report: &'a str, kind: &str) -> Vec<&'a str> {
    report
        .lines()
        .filter(|line| line.starts_with("### Anomaly ") && line.contains(kind))
        .collect()
}

fn section_three(report: &str) -> &str {
    let start = report
        .find("## Section 3 — Anomaly Flags")
        .expect("section 3 start");
    let end = report[start..]
        .find("## Section 4 —")
        .map_or(report.len(), |offset| start + offset);
    &report[start..end]
}

fn anomaly_block<'a>(report: &'a str, kind: &str) -> &'a str {
    let section = section_three(report);
    let header = section
        .lines()
        .find(|line| line.starts_with("### Anomaly ") && line.contains(kind))
        .unwrap_or_else(|| panic!("missing anomaly block for {kind}"));
    let start = section.find(header).expect("anomaly header position");
    let rest = &section[start..];
    let end = rest.find("\n### Anomaly ").unwrap_or(rest.len());
    &rest[..end]
}

#[test]
fn convergence_smell_fires_on_forced_hub_scenario() {
    let report = run_observer("tests/fixtures/observer_anomalies/convergence_hub.ron", 300);

    assert_eq!(count_anomalies_of_kind(&report, "GEOGRAPHIC_CONVERGENCE"), 1);
    let headers = anomaly_headers_of_kind(&report, "GEOGRAPHIC_CONVERGENCE");
    assert_eq!(headers.len(), 1);
    assert!(headers[0].contains("Alice"));
    assert!(headers[0].contains("Bob"));
    assert!(headers[0].contains("Carol"));
}

#[test]
fn convergence_smell_stays_absent_on_survival_baseline() {
    let report = run_observer("../../scenarios/survival-baseline.ron", 1440);

    assert_eq!(count_anomalies_of_kind(&report, "GEOGRAPHIC_CONVERGENCE"), 0);
}

#[test]
fn maintenance_starvation_fires_on_wash_gap() {
    let report = run_observer(
        "tests/fixtures/observer_anomalies/maintenance_starvation_wash_gap.ron",
        600,
    );

    assert_eq!(count_anomalies_of_kind(&report, "MAINTENANCE_STARVATION"), 2);
    let blocks = anomaly_headers_of_kind(&report, "MAINTENANCE_STARVATION");
    assert_eq!(blocks.len(), 2);
    let section = section_three(&report);
    assert!(section.contains("Mira"));
    assert!(section.contains("Noor"));
    assert!(section.contains("accumulated"));
    assert!(section.contains("relieved only"));
    assert!(section.contains("Net deficit"));
    assert!(section.contains("above high threshold"));
}

#[test]
fn recipe_monoculture_fires_on_single_food_dependency() {
    let report = run_observer(
        "tests/fixtures/observer_anomalies/recipe_monoculture_apples_vs_grain.ron",
        1000,
    );

    assert_eq!(count_anomalies_of_kind(&report, "RECIPE_MONOCULTURE"), 1);
    let headers = anomaly_headers_of_kind(&report, "RECIPE_MONOCULTURE");
    assert_eq!(headers.len(), 1);
    assert!(headers[0].contains("Agent A"));
    assert!(!headers[0].contains("Agent B"));

    let block = anomaly_block(&report, "RECIPE_MONOCULTURE");
    assert!(block.contains("Harvest Apples"));
    assert!(block.contains("Harvest Grain"));
    assert!(block.contains("FieldPlot"));
}

#[test]
fn acute_need_spike_fires_on_bounded_thirst_run() {
    let report = run_observer("tests/fixtures/observer_anomalies/acute_thirst_spike.ron", 200);

    assert_eq!(count_anomalies_of_kind(&report, "ACUTE_NEED_SPIKE"), 1);

    let block = anomaly_block(&report, "ACUTE_NEED_SPIKE");
    assert!(block.contains("thirst above critical threshold"));
    assert!(block.contains("Tick range:"));
    assert!(block.contains("dehydration tolerance"));
}
