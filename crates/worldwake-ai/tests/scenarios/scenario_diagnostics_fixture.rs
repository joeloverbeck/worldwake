// Scenario 421: Survival-baseline diagnostics fixture
// Setup: Runs scenarios/survival-baseline.ron for 1440 ticks with decision tracing enabled.
// Proves: Aggregate scenario diagnostics are schema-covered, observer-JSON round-trippable, and fixture-stable.
// Chain: Scenario spawn -> AI decision traces -> scenario diagnostics aggregation -> observer JSON representation.
#[test]
#[ignore = "CI-only: 1440-tick traced diagnostics fixture; run via golden-scenario-diagnostics workflow"]
fn golden_scenario_diagnostics_survival_baseline_fixture_is_stable() {
    crate::scenario_diagnostics_harness::assert_survival_baseline_fixture_is_stable();
}
