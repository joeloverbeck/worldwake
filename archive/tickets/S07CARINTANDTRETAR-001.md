# S07CARINTANDTRETAR-001: Replace GoalKind::Heal with TreatWounds and remove CommodityPurpose::Treatment

**Status**: ✅ COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — goal identity types in worldwake-core
**Deps**: S03 (completed), E14 (completed)

## Problem

The current care model splits treatment across `GoalKind::Heal { target }` and `AcquireCommodity { purpose: Treatment }`. This ticket replaces both with a single patient-anchored `GoalKind::TreatWounds { patient }` and removes `CommodityPurpose::Treatment`. This is the foundational type change that all subsequent tickets depend on.

## Assumption Reassessment (2026-03-17)

1. `GoalKind::Heal { target: EntityId }` exists in `goal.rs:30-32` — confirmed
2. `CommodityPurpose::Treatment` exists in `goal.rs:11` — confirmed
3. `GoalKey::from()` maps `Heal { target }` to `(None, Some(target), None)` at line 88 — confirmed
4. `GoalKind` has 17 variants currently — confirmed
5. Exhaustive matches exist across `goal_model.rs`, `goal_policy.rs`, `ranking.rs`, `candidate_generation.rs`, `failure_handling.rs`, `interrupts.rs`, `search.rs`, `blocked_intent.rs` — these will break at compile time, which is the desired forcing function

## Architecture Check

1. Renaming to `TreatWounds` disambiguates from `PlannerOpKind::Heal` (action op vs goal). The `patient` field name is clearer than `target` for care semantics.
2. No backward-compatibility aliases — `Heal` and `CommodityPurpose::Treatment` are removed outright per Principle 26.

## What to Change

### 1. Replace `GoalKind::Heal` with `GoalKind::TreatWounds`

In `goal.rs`, replace:
```rust
Heal { target: EntityId },
```
with:
```rust
TreatWounds { patient: EntityId },
```

### 2. Remove `CommodityPurpose::Treatment`

Delete the `Treatment` variant from the `CommodityPurpose` enum.

### 3. Update `GoalKey::from()`

Replace the `Heal { target }` match arm with `TreatWounds { patient }`:
- `GoalKind::TreatWounds { patient }` maps to `(None, Some(patient), None)`

Remove `Heal { target }` from the shared arm with `EngageHostile`, `LootCorpse`, `ClaimOffice`.

### 4. Fix all compile errors in test code within goal.rs

Update any tests that reference `CommodityPurpose::Treatment` or `GoalKind::Heal`.

### 5. Fix compile errors in `blocked_intent.rs` tests

The test at line 162 uses `CommodityPurpose::Treatment` — switch to another purpose variant (e.g., `SelfConsume`).

## Files to Touch

- `crates/worldwake-core/src/goal.rs` (modify)
- `crates/worldwake-core/src/blocked_intent.rs` (modify — test code only)

## Out of Scope

- Updating `worldwake-ai` match arms (those are separate tickets that will get compile errors as forcing functions)
- Updating `worldwake-sim` or `worldwake-systems` (separate tickets)
- Adding `care_weight` to `UtilityProfile` (ticket 002)
- Changing candidate generation, ranking, or planner semantics
- Updating golden tests

## Acceptance Criteria

### Tests That Must Pass

1. `GoalKind::TreatWounds` satisfies value bounds (Clone, Eq, Ord, Serialize, Deserialize) — existing `goal_model_types_satisfy_value_bounds` test
2. `GoalKey::from(TreatWounds { patient })` extracts `entity: Some(patient)`, `commodity: None`, `place: None` — new test
3. `TreatWounds` roundtrips through bincode — new test
4. `CommodityPurpose` does not contain `Treatment` — enforced at compile time by exhaustive match
5. Existing suite: `cargo test -p worldwake-core`

### Invariants

1. `GoalKind` enum has exactly 17 variants (Heal removed, TreatWounds added — net zero)
2. `CommodityPurpose` has exactly 3 variants (Treatment removed)
3. `GoalKey` extraction for `TreatWounds` uses `entity: Some(patient)`, matching the patient-anchored identity model
4. No backward-compatibility aliases for `Heal` or `CommodityPurpose::Treatment` exist

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/goal.rs` — new `goal_key_extracts_patient_for_treat_wounds` test
2. `crates/worldwake-core/src/goal.rs` — new `treat_wounds_goal_roundtrips_through_bincode` test
3. `crates/worldwake-core/src/blocked_intent.rs` — update existing test to use non-Treatment purpose

### Commands

1. `cargo test -p worldwake-core`
2. `cargo clippy -p worldwake-core`

Note: `cargo test --workspace` will NOT pass after this ticket alone — downstream crates have exhaustive matches on `GoalKind` that will fail to compile. This is intentional; subsequent tickets fix those.

## Outcome

- **Completion date**: 2026-03-17
- **What changed**:
  - `goal.rs`: Renamed `GoalKind::Heal { target }` to `GoalKind::TreatWounds { patient }`, removed `CommodityPurpose::Treatment`, updated `GoalKey::from()` shared match arm
  - `blocked_intent.rs`: Test switched from `CommodityPurpose::Treatment` to `SelfConsume`
  - Added `goal_key_extracts_patient_for_treat_wounds` and `treat_wounds_goal_roundtrips_through_bincode` tests
- **Deviations**: None — implemented exactly as specified
- **Verification**: `cargo test -p worldwake-core` 683 passed / 0 failed; `cargo clippy -p worldwake-core` clean
