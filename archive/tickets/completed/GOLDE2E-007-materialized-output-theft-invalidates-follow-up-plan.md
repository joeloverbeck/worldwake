# GOLDE2E-007: Materialized Output Theft Invalidates Follow-Up Plan

**Status**: ✅ COMPLETED
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Unclear — verify whether the real behavior is stale-plan failure or fresh replanning across a progress barrier
**Deps**: None

## Problem

Scenario 4 proves the happy-path materialization barrier chain. This gap tests the contested follow-up path: an agent crafts food, the output materializes as a ground lot, and another co-located hungry agent takes it before the crafter can benefit from it. Reassessment against the current runtime shows an even cleaner architecture than the original ticket assumed: craft completion ends at a progress barrier, so the crafter does not carry a stale follow-up step into the theft window. If the output disappears before the next plan is adopted, the runtime should simply replan from authoritative state rather than route through blocked-intent failure handling.

## Report Reference

Backlog item **P-NEW-6** in `reports/golden-e2e-coverage-analysis.md` (Tier 2, composite score 4).

## Assumption Reassessment (2026-03-13)

1. `MaterializationBindings` exists, but this scenario may never exercise them because craft completion currently terminates at a progress barrier.
2. `handle_plan_failure()` and `BlockedIntentMemory` exist, but they are not necessarily the correct or expected path for this scenario.
3. Two-agent same-place contention is feasible in the golden harness and is a better fit than same-workstation exclusivity for this scenario.
4. Crafted outputs currently materialize as unowned ground lots rather than directly entering the crafter's possession, so a second agent can legally take them under the current architecture.
5. The golden gap is therefore not "missing binding recovery"; it is "does the runtime replan cleanly when a barrier-created opportunity disappears before the next plan is chosen?"

## Architecture Check

1. Craft completion should remain a progress barrier. No stale continuation plan should survive across that barrier.
2. If the materialized output disappears before the next plan is selected, the runtime should simply choose a new plan from the updated world state.
3. No special-case recovery path and no bespoke binding error type.

## Engine-First Mandate

If implementing this e2e suite reveals that the runtime carries stale post-barrier continuations, depends on blocked-intent shims here, or otherwise handles the theft via a dirtier path than fresh replanning from state, do NOT patch around it. Instead, preserve or restore the cleaner architecture and document that change in the ticket outcome.

## What to Change

### 1. New golden test in `golden_production.rs`

**Setup**: Two hungry agents share a place containing a mill. Crafter has the recipe input and can craft bread locally. Thief has no recipe input and should prefer the newly materialized local bread over distant alternatives. A distant fallback food source should also exist so the crafter can recover by replanning instead of stalling forever.

**Assertions**:
- Crafter completes the craft action and bread materializes as a ground lot.
- Thief takes the bread before the crafter can benefit from it.
- Crafter does not retain a stale bread-follow-up plan across the craft barrier.
- Crafter cleanly replans from the updated world state instead of recording a stale `MissingInput(Bread)` blocker for this case.
- Crafter does not crash or deadlock and eventually recovers through a different emergent food path.

## Files to Touch

- `crates/worldwake-ai/tests/golden_production.rs` (modify)
- `crates/worldwake-ai/tests/golden_harness/mod.rs` (modify, if helpers needed)
- `crates/worldwake-ai/src/failure_handling.rs` (modify only if supporting characterization coverage is warranted)
- Engine files TBD only if the runtime is found to carry stale post-barrier plans

## Out of Scope

- Multi-step binding chains (bind-of-bind failures)
- Materialization binding success path (already proven elsewhere)
- Ownership/custody redesign for produced goods
- Reservation or queue-specific facility contention
- Forcing blocked-intent creation where the cleaner architecture should replan without it

## Acceptance Criteria

### Tests That Must Pass

1. `golden_materialized_output_theft_forces_replan` — crafter loses the crafted output, does not retain a stale post-barrier plan, and recovers
2. Existing suite: `cargo test -p worldwake-ai golden_`
3. Full workspace: `cargo test --workspace`

### Invariants

1. All behavior is emergent — no manual plan injection
2. Conservation holds (items transferred or consumed, never duplicated)
3. No crash or deadlock on post-materialization control loss
4. No stale `MissingInput(Bread)` blocker is recorded solely because the output disappeared before the next plan was adopted

## Post-Implementation

After implementing this suite, update `reports/golden-e2e-coverage-analysis.md`:
- Add the new scenario to Part 1 (Proven Emergent Scenarios)
- Remove P-NEW-6 from the Part 3 backlog
- Update Part 4 summary statistics and wording so it describes progress-barrier replanning rather than missing binding resolution

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_production.rs::golden_materialized_output_theft_forces_replan` — proves contested post-materialization recovery through fresh replanning
2. `crates/worldwake-ai/src/failure_handling.rs::<new focused unit test>` — characterizes how `handle_plan_failure()` classifies hypothetical consume loss if that lower-level path is exercised elsewhere

### Commands

1. `cargo test -p worldwake-ai golden_materialized_output_theft_forces_replan`
2. `cargo test -p worldwake-ai failure_handling`
2. `cargo test --workspace && cargo clippy --workspace`

## Outcome

- **Completion date**: 2026-03-13
- **What actually changed**:
  - Added `golden_materialized_output_theft_forces_replan` in `crates/worldwake-ai/tests/golden_production.rs`.
  - Added a focused `failure_handling` characterization test for hypothetical consume-loss classification.
  - Updated `reports/golden-e2e-coverage-analysis.md` to document the new proven scenario and remove the backlog item.
- **Deviations from original plan**:
  - Reassessment showed the original ticket diagnosis was wrong. The runtime does not hit a materialization-binding failure or blocked-intent path in this scenario.
  - The cleaner existing architecture is that craft completion ends at a progress barrier, the stolen output disappears before any stale follow-up step is adopted, and the crafter replans from authoritative state.
  - No engine behavior changes were required; the ticket was corrected to match the real architecture and the tests were written around that behavior.
- **Verification results**:
  - `cargo test -p worldwake-ai golden_materialized_output_theft_forces_replan`
  - `cargo test -p worldwake-ai golden_`
  - `cargo test --workspace`
  - `cargo clippy --workspace --all-targets -- -D warnings`
