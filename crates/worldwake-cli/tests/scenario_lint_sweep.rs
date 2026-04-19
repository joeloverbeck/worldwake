//! CI guard: every committed scenario in `scenarios/` must pass lints
//! (with explicit overrides where homogeneity is intentional).

use std::path::PathBuf;

use worldwake_cli::scenario::{lints, load_scenario_file};

#[test]
fn every_committed_scenario_passes_lints() {
    let scenarios_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../scenarios");
    let mut failed = Vec::new();

    for entry in std::fs::read_dir(&scenarios_dir).expect("scenarios dir readable") {
        let path = entry.expect("scenario entry readable").path();
        if path.extension().and_then(|value| value.to_str()) != Some("ron") {
            continue;
        }

        let def = load_scenario_file(&path)
            .unwrap_or_else(|err| panic!("failed to load scenario {}: {err}", path.display()));
        let report = lints::run_lints(&def);
        let report =
            lints::filter_overrides(report, &def.scenario_lint_overrides).unwrap_or_else(|err| {
                panic!(
                    "failed to validate lint overrides for {}: {err}",
                    path.display()
                )
            });

        if !report.failures.is_empty() {
            failed.push((path, report.failures));
        }
    }

    assert!(
        failed.is_empty(),
        "scenarios with unsuppressed lint failures: {failed:#?}",
    );
}
