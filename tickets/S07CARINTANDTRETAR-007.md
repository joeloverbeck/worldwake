# S07CARINTANDTRETAR-007: Update goal policy, failure handling, and remaining Heal→TreatWounds match arms

**Status**: PENDING
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — goal policy, failure handling, interrupts in worldwake-ai
**Deps**: S07CARINTANDTRETAR-001 (TreatWounds variant exists)

## Problem

After the core type change (ticket 001), several `worldwake-ai` modules still have `GoalKind::Heal` match arms that must be updated to `GoalKind::TreatWounds`:
- `goal_policy.rs`: Goal family policy for care
- `failure_handling.rs`: Blocker messages and `TargetGone` resolution
- `interrupts.rs`: Interrupt evaluation match arms
- `search.rs`: Any `CommodityPurpose::Treatment` references

This is a mechanical rename ticket for the remaining match arms not covered by tickets 004-006.

## Assumption Reassessment (2026-03-17)

1. `goal_policy.rs` has `GoalKind::Heal { .. }` match arm at lines 143-147 — confirmed via grep
2. `failure_handling.rs` has `GoalKind::Heal { .. }` match arm at line 457 — confirmed via grep
3. `interrupts.rs` has `GoalKind::Heal` references at lines 401-406 and 774-776 — confirmed via grep
4. `search.rs` has `CommodityPurpose::Treatment` reference at line 2108 — confirmed via grep
5. The policy for `Heal` is `{ suppression: Never, penalty_interrupt: Never, free_interrupt: Reactive }` — confirmed per spec D10

## Architecture Check

1. `TreatWounds` policy is identical to current `Heal` policy — `{ suppression: Never, penalty_interrupt: Never, free_interrupt: Reactive }`. No behavior change, just rename.
2. Failure handling for `TreatWounds` should reference patient identity in blocker messages. `BlockingFact::TargetGone` resolution checks patient aliveness.
3. Interrupt evaluation arms are mechanical renames — same logic, new variant name.

## What to Change

### 1. `goal_policy.rs`: Replace `Heal` match arm with `TreatWounds`

```rust
GoalKind::TreatWounds { .. } => GoalFamilyPolicy {
    suppression: SuppressionRule::Never,
    penalty_interrupt: PenaltyInterruptEligibility::Never,
    free_interrupt: FreeInterruptRole::Reactive,
},
```

### 2. `failure_handling.rs`: Replace `Heal` match arms with `TreatWounds`

Update match arm at line 457. Ensure blocker messages reference patient identity (not generic "treatment acquisition failed").

### 3. `interrupts.rs`: Replace `GoalKind::Heal` references with `GoalKind::TreatWounds`

Mechanical rename in interrupt evaluation match arms (lines 401-406, 774-776).

### 4. `search.rs`: Remove `CommodityPurpose::Treatment` references

Update test code at line 2108 that uses `CommodityPurpose::Treatment` — use a different purpose or restructure the test.

## Files to Touch

- `crates/worldwake-ai/src/goal_policy.rs` (modify)
- `crates/worldwake-ai/src/failure_handling.rs` (modify)
- `crates/worldwake-ai/src/interrupts.rs` (modify)
- `crates/worldwake-ai/src/search.rs` (modify — test code)

## Out of Scope

- Goal model changes (ticket 004)
- Candidate generation (ticket 005)
- Ranking changes (ticket 006)
- Golden tests (ticket 008)
- Changing policy semantics (they stay identical)
- Adding new failure handling logic beyond the rename

## Acceptance Criteria

### Tests That Must Pass

1. `TreatWounds` goal policy is `{ suppression: Never, penalty_interrupt: Never, free_interrupt: Reactive }` — test in goal_policy.rs
2. `TreatWounds` blocker messages reference patient identity — test in failure_handling.rs
3. `BlockingFact::TargetGone` resolution checks patient aliveness for `TreatWounds` — test in failure_handling.rs
4. No `GoalKind::Heal` match arms remain in any worldwake-ai file
5. No `CommodityPurpose::Treatment` references remain in any worldwake-ai file
6. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. `TreatWounds` policy is semantically identical to old `Heal` policy
2. Interrupt behavior for care goals is unchanged
3. All exhaustive matches compile without `Heal` variant

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/goal_policy.rs` — update existing `Heal` policy test to assert `TreatWounds` policy
2. `crates/worldwake-ai/src/failure_handling.rs` — update `Heal` failure test to assert `TreatWounds` with patient identity
3. `crates/worldwake-ai/src/interrupts.rs` — update interrupt tests using `Heal` to use `TreatWounds`
4. `crates/worldwake-ai/src/search.rs` — update test at line 2108 to remove `CommodityPurpose::Treatment`

### Commands

1. `cargo test -p worldwake-ai`
2. `cargo clippy -p worldwake-ai`
