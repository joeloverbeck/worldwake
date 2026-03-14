# GOLDEN-001: Treatment Self-Acquisition Through AI Loop (Scenario 12)

**Status**: REJECTED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: None
**Deps**: None

## Problem

This ticket originally assumed that self-treatment was already supported by production code and that the only missing work was a golden test proving `AcquireCommodity(Treatment)` through a wounded agent's own pick-up → heal loop.

That premise is stale in two directions:

1. `AcquireCommodity(Treatment)` is already covered by the existing `golden_healer_acquires_ground_medicine_for_patient` scenario in `crates/worldwake-ai/tests/golden_care.rs`.
2. Self-healing is not an existing hidden path. The current care action explicitly forbids self-targeted `heal`, so this is not a test-only gap.

## Assumption Reassessment (2026-03-14)

1. `CommodityPurpose::Treatment` emission exists in `crates/worldwake-ai/src/candidate_generation.rs` and remains part of the current planner architecture.
2. The current golden suite already proves the treatment-acquisition chain for other-care: a healer with no medicine acquires accessible ground medicine and then heals a co-located patient.
3. `validate_heal_context()` in `crates/worldwake-systems/src/combat.rs` rejects `target == actor` via `SelfTargetForbidden { action: Heal }`. Self-heal is therefore an explicit engine-level non-feature, not an untested branch.
4. The combat action affordance tests also model heal as targeting other wounded agents, not the actor itself.
5. The coverage report was stale: it still listed `AcquireCommodity(Treatment)` as backlog even though the existing care test pair already exercises it end to end.

## Architecture Check

1. The proposed ticketed change is not more robust than the current architecture. Simply removing the self-target guard would collapse two distinct behaviors, other-care and self-care, into one action without revisiting their semantics, constraints, or occupancy model.
2. The current architecture is internally coherent: care is a same-place interpersonal action, and treatment acquisition is already proven without aliases or compatibility layers.
3. If self-treatment becomes a desired capability later, the clean design would be a deliberate engine change, either a dedicated self-care action/goal or a generalized treatment architecture with explicit self vs other semantics. That is materially larger than this ticket's claimed "test-only" scope and should not be smuggled in through a golden test.

## What Changed Instead

### 1. Correct the coverage report

Update `reports/golden-e2e-coverage-analysis.md` to reflect the code and tests that already exist:
- `golden_care.rs` contains four golden care tests, not two.
- `AcquireCommodity(Treatment)` is already covered.
- Scenario 12 should be removed from the pending backlog and recommended implementation order.

### 2. Archive this ticket as rejected

This ticket should not drive code changes. Its underlying assumptions were invalid, and its proposed implementation would require a new architecture decision rather than a small golden-test addition.

## Files to Touch

- `reports/golden-e2e-coverage-analysis.md`
- this ticket before archival

## Out of Scope

- Any production-code change to allow self-heal
- Any aliasing or compatibility path that treats self-heal as ordinary `heal`
- New care-domain golden tests for self-treatment
- Travel, trade, or distant-medicine treatment scenarios

## Acceptance Criteria

1. The coverage report accurately records the existing treatment-acquisition proof in `golden_care.rs`.
2. `AcquireCommodity(Treatment)` is marked as covered in the GoalKind matrix.
3. Scenario 12 is removed from the coverage backlog.
4. Relevant tests and repository-wide verification pass.

## Test Plan

### New/Modified Tests

None. Reassessment showed the claimed gap was already covered by the existing care golden tests.

### Commands

1. `cargo test -p worldwake-ai --test golden_care`
2. `cargo test --workspace`
3. `cargo clippy --workspace`

## Outcome

- **Rejected**: 2026-03-14
- **What changed**:
  - Reassessed the ticket against the current code and golden suite.
  - Confirmed that `AcquireCommodity(Treatment)` is already proven by the existing healer-acquires-ground-medicine scenario.
  - Corrected the coverage report to remove the stale backlog entry.
- **Deviations from original plan**:
  - No new tests were added because the original premise, "test-only uncovered path," was false.
  - No engine changes were made because self-heal is an explicit architecture decision, not a missing assertion.
- **Verification**:
  - `cargo test -p worldwake-ai --test golden_care`
  - `cargo test --workspace`
  - `cargo clippy --workspace`
