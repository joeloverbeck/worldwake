// Scenario 143: CLI Evaluation Lina 0-step FreeCarryCapacity Loop
// Setup: Rebuilds the Forager Lina Eldergrove Forest slice from cli-evaluation with local apples, water, orchard source, and seed 7777.
// Proves: The late-run window has no repeated 0-step FreeCarryCapacity loop and hunger pressure keeps recovering.
// Chain: harvest/eat/waste accumulation -> carry strain from actual load -> no spurious FreeCarryCapacity loop -> hunger remains controlled.
#[test]
#[ignore = "CI-only: 900-tick cli-evaluation pathology replay; run via golden-planner-pathology workflow"]
fn degenerate_zero_step_loop_blocks_actionable_goals() {
    crate::planner_pathology_harness::assert_degenerate_zero_step_loop_blocks_actionable_goals();
}
