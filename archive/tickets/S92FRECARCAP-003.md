# S92FRECARCAP-003: Focused parity tests for unified contract

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: None — focused test coverage only
**Deps**: `archive/tickets/S92FRECARCAP-001.md`, `archive/tickets/S92FRECARCAP-002.md`, `specs/S92-free-carry-capacity-zero-step-loop-fix.md`

## Problem

The unified `FreeCarryCapacity` contract from tickets 001-002 needs focused parity tests proving that satisfaction, emission, and ranking all agree on the same disposal state. Without these tests, future changes could silently re-introduce the contract divergence that caused the zero-step loop pathology.

## Assumption Reassessment (2026-04-11)

1. After 001+002, the shared helper is the canonical contract for all three call sites. Existing focused tests now cover module-local `FreeCarryCapacity` behavior in `goal_model.rs`, `candidate_generation.rs`, and `ranking.rs`, but no explicit cross-layer parity proof yet ties those three consumers to the same root/actionable fixture contract. Corrected 2026-04-11.
2. `golden_waste_disposal_cycle` in `crates/worldwake-ai/tests/golden_production.rs:4449` covers the happy-path disposal cycle (plan → drop → goal stops) but does not cover the remaining focused boundary gap: a disposal step that lowers carried load relative to the root baseline while still leaving the actor above threshold must keep `FreeCarryCapacity` unsatisfied. Existing emission/ranking non-actionable cases already landed in 002. Corrected 2026-04-11.
5. Live `GoalKind`: `FreeCarryCapacity`. Operator: `PlannerOpKind::DropItem`. Tests must construct planning states and belief views that exercise the contract boundaries.

## Architecture Check

1. Focused parity tests are the correct proof surface for cross-layer contract consistency. Golden tests prove end-to-end behavior but cannot isolate which layer breaks if the contract diverges again. These tests pin the contract at the unit level.
2. No backward-compatibility shims. Tests validate the new unified contract directly.

## Verification Layers

1. Strained root with actionable waste -> not satisfied -> existing focused unit test on `is_satisfied()`
2. Single drop below threshold -> satisfied -> existing focused unit test on `is_satisfied()` with modified state
3. Partial drop still above threshold -> not satisfied -> remaining focused unit test owned by this ticket
4. Below-threshold or no-waste -> emission inactive, motive score zero -> existing focused unit tests delivered in 002
5. Only directly possessed non-empty Waste lots are targets -> existing focused emission test delivered in 002
6. Single-layer ticket: all tests exercise `worldwake-ai` planner logic only.

## What to Change

### 1. Add the remaining focused satisfaction boundary test in `goal_model.rs`

Add the remaining `#[cfg(test)]` case proving:

1. **Partial drop, still above threshold -> not satisfied**: From the existing strained root fixture, simulate a disposal step that lowers carried load relative to the root baseline but keeps the actor above the active threshold. Assert `GoalKind::FreeCarryCapacity.is_satisfied()` remains `false`.

### 2. Verify naming consistency

Test names should follow the existing pattern in their respective modules. Prefix with `free_carry_capacity_` for discoverability.

## Files to Touch

- `crates/worldwake-ai/src/goal_model.rs` (modify — add the remaining `#[cfg(test)]` parity boundary case)

## Out of Scope

- Modifying the shared helper or satisfaction logic — done in 001-002
- Golden E2E test changes — done in S92FRECARCAP-004
- Testing unrelated goal kinds or pathologies

## Acceptance Criteria

### Tests That Must Pass

1. The remaining focused parity boundary test for "partial drop still above threshold" passes, alongside the existing `FreeCarryCapacity` focused tests from 001-002
2. `cargo test -p worldwake-ai` — no regressions
3. `cargo clippy --workspace --all-targets -- -D warnings` — clean clippy

### Invariants

1. Satisfaction, emission, and ranking agree: actionable ↔ not-satisfied ↔ candidates-emitted ↔ nonzero-motive at the root
2. Satisfaction requires both load decrease relative to root baseline AND load below threshold
3. Only directly possessed, non-empty Waste lots qualify as disposal targets

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/goal_model.rs` — add the remaining partial-drop-still-above-threshold satisfaction case
2. Existing `candidate_generation.rs` / `ranking.rs` `free_carry_capacity_*` tests remain the proof surface for emission/ranking parity delivered in 002

### Commands

1. `cargo test -p worldwake-ai free_carry_capacity_ -- --nocapture`
2. `cargo test -p worldwake-ai`
3. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

Completed on 2026-04-11.

- Reassessed the ticket against the delivered `001/002` state and narrowed it to the remaining honest delta: the existing module-local `FreeCarryCapacity` tests in `candidate_generation.rs` and `ranking.rs` already covered the emission/ranking non-actionable cases, so this ticket only needed the missing satisfaction-boundary proof.
- Added `free_carry_capacity_is_not_satisfied_after_partial_drop_still_at_threshold()` in `crates/worldwake-ai/src/goal_model.rs` to prove that disposal progress relative to the root baseline is still insufficient when the actor remains at or above the active threshold with lawful Waste drop targets.
- Kept the existing `free_carry_capacity_*` tests from `goal_model.rs`, `candidate_generation.rs`, and `ranking.rs` as the focused parity proof surface established across tickets 001-003.

## Verification Result

- Passed `cargo test -p worldwake-ai free_carry_capacity_ -- --nocapture`
- Passed `cargo test -p worldwake-ai`
- Passed `cargo clippy --workspace --all-targets -- -D warnings`
