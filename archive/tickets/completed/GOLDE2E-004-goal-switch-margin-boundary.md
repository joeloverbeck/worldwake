# GOLDE2E-004: Goal-Switch Margin Boundary

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: None expected
**Deps**: None

## Problem

The original ticket assumed the exact goal-switch margin boundary was only exercised indirectly through golden scenarios. That assumption is no longer correct.

The current codebase already tests the exact boundary at the correct layers:
- `crates/worldwake-ai/src/goal_switching.rs` proves the base `compare_goal_switch()` arithmetic with `1099 -> no switch` and `1100 -> switch`.
- `crates/worldwake-ai/src/interrupts.rs` proves the freely interruptible runtime path applies the same exact boundary.
- `crates/worldwake-ai/src/plan_selection.rs` proves plan replacement uses the same exact boundary.
- `crates/worldwake-ai/src/journey_switch_policy.rs` proves relation-aware journey margins.
- Existing golden scenarios 2, 3c, and 7d already prove behavior-level switching under the real AI loop.

What is missing is not correctness coverage of the arithmetic itself. The only remaining question is whether a new golden test would add durable architectural value. After reassessment, the answer is no: this boundary is already well-covered below the golden layer, and forcing a new e2e arithmetic threshold test would mostly duplicate lower-level guarantees with a more brittle setup.

## Report Reference

Backlog item **P-NEW-3** in `reports/golden-e2e-coverage-analysis.md` (Tier 2, composite score 4). This ticket resolves that backlog item by correcting the gap analysis rather than by adding a redundant golden test.

## Assumption Reassessment (2026-03-13)

1. `compare_goal_switch()` exists in `worldwake-ai/src/goal_switching.rs`.
2. Relation-aware switching is split across `goal_switching.rs`, `journey_switch_policy.rs`, `interrupts.rs`, and `plan_selection.rs`, not just `goal_switching.rs`.
3. `GoalSwitchKind` and switch margins use `Permille` arithmetic, but motive scores themselves are integer products of utility weights and pressures.
4. The golden harness can configure precise need levels and utility weights, but exact arithmetic boundary checks already exist in unit/integration tests and do not need to be duplicated in golden form.
5. Existing tests already verify the exact `1099/1100` boundary as well as higher effective journey margins; the remaining golden scenarios cover the emergent runtime behavior.

## Architecture Check

1. The current architecture is cleaner than the ticket assumed: pure switch arithmetic is isolated in focused units, while golden tests cover cross-system emergent behavior.
2. Adding an e2e test just to restate `1099/1100` boundary math would make the suite more brittle without strengthening the architecture.
3. The robust, extensible design is to keep arithmetic guarantees in focused unit/integration tests and reserve golden tests for behavior that only the full AI loop can prove.

## Engine-First Mandate

If future work reveals that goal-switch margins are architecturally unsound, inconsistent across priority classes, or rely on magic numbers rather than profile-driven parameters, the fix should be made in the engine and its focused tests first. This ticket does not justify a redundant golden scenario today.

## What to Change

### 1. Correct the ticket scope

Record that the exact switch-margin boundary is already covered by focused tests and does not warrant a new golden scenario.

### 2. Remove the stale golden backlog item from the report

Update `reports/golden-e2e-coverage-analysis.md` so it no longer lists P-NEW-3 as a missing golden scenario. The report should reflect that:
- behavior-level goal switching is already covered by the golden suite;
- exact margin arithmetic is covered at lower test layers;
- the golden backlog should prioritize missing cross-system behaviors, not duplicate arithmetic checks.

## Files to Touch

- `reports/golden-e2e-coverage-analysis.md` (modify)
- `tickets/GOLDE2E-004-goal-switch-margin-boundary.md` (modify, then archive)

## Out of Scope

- Adding a new golden test that duplicates the exact arithmetic boundary already covered elsewhere
- Changing switch-margin values
- Refactoring goal-switch architecture without a demonstrated engine deficiency

## Acceptance Criteria

### Tests That Must Pass

1. Existing focused switch-margin tests in `worldwake-ai` pass.
2. Existing golden suite in `worldwake-ai` passes.
3. Full workspace tests pass.
4. `cargo clippy --workspace` passes.
5. `reports/golden-e2e-coverage-analysis.md` no longer lists P-NEW-3 as a golden gap.

### Invariants

1. No backward-compatibility shims or alias paths are introduced.
2. Exact switch arithmetic remains covered by focused tests.
3. Golden tests remain focused on emergent cross-system behavior.

## Post-Implementation

After resolving this ticket, archive it with an Outcome section documenting that no code or test changes were needed because the reported gap was a documentation/analysis error, not an engine or coverage defect.

## Test Plan

### New/Modified Tests

- None.

### Commands

1. `cargo test -p worldwake-ai`
2. `cargo test --workspace`
3. `cargo clippy --workspace`

## Outcome

### Completion date

2026-03-13

### What actually changed

- Corrected the ticket assumptions to match the current architecture and test suite.
- Removed `P-NEW-3` from `reports/golden-e2e-coverage-analysis.md` because the reported gap was stale.
- Kept the engine and tests unchanged because the exact switch-margin boundary is already covered by focused tests in `worldwake-ai`.

### Deviations from the original plan

- The original plan proposed a new golden e2e test.
- Reassessment showed that would duplicate existing arithmetic-boundary coverage in unit/integration tests and add brittleness without improving architectural confidence.
- The resolved scope is therefore documentation and backlog correction, not code or test changes.

### Verification results

- `cargo test -p worldwake-ai`
- `cargo test --workspace`
- `cargo clippy --workspace`
