# S92FRECARCAP-003: Focused parity tests for unified contract

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: None
**Deps**: `archive/tickets/S92FRECARCAP-001.md`, `archive/tickets/S92FRECARCAP-002.md`, `specs/S92-free-carry-capacity-zero-step-loop-fix.md`

## Problem

The unified `FreeCarryCapacity` contract from tickets 001-002 needs focused parity tests proving that satisfaction, emission, and ranking all agree on the same disposal state. Without these tests, future changes could silently re-introduce the contract divergence that caused the zero-step loop pathology.

## Assumption Reassessment (2026-04-11)

1. After 001+002, the shared helper is the canonical contract for all three call sites. Existing focused tests now cover module-local `FreeCarryCapacity` behavior in `goal_model.rs`, `candidate_generation.rs`, and `ranking.rs`, but no explicit cross-layer parity proof yet ties those three consumers to the same root/actionable fixture contract. Corrected 2026-04-11.
2. `golden_waste_disposal_cycle` in `crates/worldwake-ai/tests/golden_production.rs:4449` covers the happy-path disposal cycle (plan → drop → goal stops) but does not cover the boundary conditions: sub-threshold inactivity, partial-drop still-above-threshold, or emission/ranking zero-score for non-actionable state.
5. Live `GoalKind`: `FreeCarryCapacity`. Operator: `PlannerOpKind::DropItem`. Tests must construct planning states and belief views that exercise the contract boundaries.

## Architecture Check

1. Focused parity tests are the correct proof surface for cross-layer contract consistency. Golden tests prove end-to-end behavior but cannot isolate which layer breaks if the contract diverges again. These tests pin the contract at the unit level.
2. No backward-compatibility shims. Tests validate the new unified contract directly.

## Verification Layers

1. Strained root with actionable waste -> not satisfied -> focused unit test on `is_satisfied()`
2. Single drop below threshold -> satisfied -> focused unit test on `is_satisfied()` with modified state
3. Partial drop still above threshold -> not satisfied -> focused unit test
4. Below-threshold or no-waste -> emission inactive, motive score zero -> existing focused unit tests plus any added parity assertions
5. Only directly possessed non-empty Waste lots are targets -> existing focused emission test plus any added parity assertions
6. Single-layer ticket: all tests exercise `worldwake-ai` planner logic only.

## What to Change

### 1. Add focused tests in `goal_model.rs` (or nearby test module)

Add `#[cfg(test)]` tests proving:

1. **Strained root, actionable waste -> not satisfied**: Construct a PlanningState where the actor has carried load above threshold and directly-possessed non-empty Waste. Assert `is_satisfied()` returns `false`.
2. **Single drop below threshold -> satisfied**: From the strained state, simulate a drop (modify commodity quantities in PlanningState overrides) so load falls below threshold. Assert `is_satisfied()` returns `true`.
3. **Partial drop, still above threshold -> not satisfied**: Simulate a partial drop that reduces load but stays above threshold. Assert `is_satisfied()` returns `false`.

### 2. Add focused tests for emission and ranking parity

Add `#[cfg(test)]` tests in `candidate_generation.rs` and/or `ranking.rs` proving:

4. **Below threshold -> emission inactive, motive score zero**: Construct context where load is below threshold. Assert no candidates emitted and motive score is 0.
5. **No waste targets -> emission inactive**: Construct context where load is above threshold but no directly-possessed Waste lots exist. Assert no candidates emitted.

### 3. Verify test naming consistency

Test names should follow the existing pattern in their respective modules. Prefix with `free_carry_capacity_` for discoverability.

## Files to Touch

- `crates/worldwake-ai/src/goal_model.rs` (modify — add `#[cfg(test)]` tests)
- `crates/worldwake-ai/src/candidate_generation.rs` (modify — add `#[cfg(test)]` tests)
- `crates/worldwake-ai/src/ranking.rs` (modify — add `#[cfg(test)]` tests)

## Out of Scope

- Modifying the shared helper or satisfaction logic — done in 001-002
- Golden E2E test changes — done in S92FRECARCAP-004
- Testing unrelated goal kinds or pathologies

## Acceptance Criteria

### Tests That Must Pass

1. All 5 new focused tests pass
2. `cargo test -p worldwake-ai` — no regressions
3. `cargo clippy --workspace --all-targets -- -D warnings` — clean clippy

### Invariants

1. Satisfaction, emission, and ranking agree: actionable ↔ not-satisfied ↔ candidates-emitted ↔ nonzero-motive at the root
2. Satisfaction requires both load decrease relative to root baseline AND load below threshold
3. Only directly possessed, non-empty Waste lots qualify as disposal targets

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/goal_model.rs` — strained root not satisfied, drop below threshold satisfied, partial drop not satisfied
2. `crates/worldwake-ai/src/candidate_generation.rs` — below-threshold no emission, no-waste no emission
3. `crates/worldwake-ai/src/ranking.rs` — below-threshold zero motive score

### Commands

1. `cargo test -p worldwake-ai free_carry_capacity_ -- --nocapture`
2. `cargo test -p worldwake-ai`
3. `cargo clippy --workspace --all-targets -- -D warnings`
