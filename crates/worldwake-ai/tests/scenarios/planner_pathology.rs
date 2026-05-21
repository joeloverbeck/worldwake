// Scenario 142: Dusty Trail Remote Water Acquisition Recovery
// Setup: A Dusty Trail guard uses the cli-evaluation place graph slice with a remote Thornwall Village well and local thirst pressure.
// Proves: Cross-location AcquireCommodity(Water) finds a lawful non-exhausted plan, commits drink, and lowers thirst.
// Chain: Dusty Trail thirst pressure -> AcquireCommodity(Water) found plan -> travel to Thornwall Village -> committed drink -> reduced thirst.
#[test]
fn cross_location_water_acquisition_succeeds_without_budget_exhaustion() {
    crate::planner_pathology_harness::assert_cross_location_water_acquisition_succeeds_without_budget_exhaustion();
}

// Scenario 144: Obligation satiation allows survival needs to override posting
// Setup: A guard remembers a hostile, starts at Hearthstone Inn with possessed Bread and Water, critical needs, and high notice-posting weight.
// Proves: Repeated PostNotice still happens, but eat and drink commits recover and notice posting does not dominate indefinitely.
// Chain: remembered hostile belief -> repeated PostNotice commits -> obligation satiation dampens notice motive -> self-care commits recover.
#[test]
fn obligation_satiation_allows_survival_needs_to_override_posting() {
    crate::planner_pathology_harness::assert_obligation_satiation_allows_survival_needs_to_override_posting();
}
