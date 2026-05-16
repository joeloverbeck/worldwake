mod golden_scenario_diagnostics_harness;

// Scenario 422: Survival-baseline diagnostics replay
// Setup: Runs scenarios/survival-baseline.ron for 1440 ticks with decision tracing enabled.
// Proves: Aggregate scenario diagnostics replay deterministically against the fixture run.
// Chain: Scenario spawn -> AI decision traces -> scenario diagnostics aggregation -> deterministic report equality.
#[test]
#[ignore = "CI-only: second 1440-tick traced diagnostics run; run via golden-scenario-diagnostics workflow"]
fn golden_scenario_diagnostics_survival_baseline_replays_deterministically() {
    golden_scenario_diagnostics_harness::assert_survival_baseline_replays_deterministically();
}
