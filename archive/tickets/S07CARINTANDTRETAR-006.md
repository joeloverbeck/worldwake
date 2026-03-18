# S07CARINTANDTRETAR-006: Split care ranking with pain_weight/care_weight and remove treatment helpers

**Status**: ✅ COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — ranking semantics in worldwake-ai
**Deps**: S07CARINTANDTRETAR-001 (TreatWounds variant), S07CARINTANDTRETAR-002 (care_weight field)

## Problem

Current ranking uses `treatment_pain()` which takes the max of actor's own pain and local patient pain — wrong for both self-care and third-party care. It also uses `treatment_score()` which doesn't differentiate self vs other. The spec requires:
- Self-care (`patient == agent`): priority from `classify_band(self_pain, thresholds.pain)`, motive from `score_product(pain_weight, self_pain)`
- Other-care (`patient != agent`): priority from `classify_band(patient_pain, thresholds.pain)`, motive from `score_product(care_weight, patient_pain)`

## Assumption Reassessment (2026-03-17)

1. `treatment_pain()` exists in `ranking.rs` (lines 373-390 per spec) — confirmed via grep
2. `treatment_score()` exists in `ranking.rs` (lines 392-403 per spec) — confirmed via grep
3. `AcquireCommodity { purpose: Treatment }` has special ranking branches in `priority_class()` (lines 152-159) and `motive_score()` (lines 254-257) — confirmed per spec
4. `GoalKind::Heal` has its own ranking arms — to be confirmed in file
5. `score_product()` and `classify_band()` helpers exist as general-purpose ranking utilities — to be confirmed

## Architecture Check

1. Self/other split using `pain_weight`/`care_weight` gives per-agent diversity (Principle 20). High-altruism agents prioritize others; pragmatic agents prioritize self.
2. Removing `treatment_pain()` and `treatment_score()` eliminates the conflated max-of-both-pains approach.
3. Removing `AcquireCommodity { purpose: Treatment }` ranking branches is safe because that variant no longer exists after ticket 001.

## What to Change

### 1. Add `TreatWounds` ranking to `priority_class()`

```rust
GoalKind::TreatWounds { patient } => {
    if patient == agent {
        classify_band(self_pain, &thresholds.pain)
    } else {
        // patient_pain derived from belief view
        classify_band(patient_pain, &thresholds.pain)
    }
}
```

### 2. Add `TreatWounds` ranking to `motive_score()`

```rust
GoalKind::TreatWounds { patient } => {
    if patient == agent {
        score_product(profile.pain_weight, self_pain)
    } else {
        score_product(profile.care_weight, patient_pain)
    }
}
```

### 3. Remove obsolete functions

- Delete `treatment_pain()` function
- Delete `treatment_score()` function

### 4. Remove `AcquireCommodity { purpose: Treatment }` ranking branches

Remove special-case branches in `priority_class()` and `motive_score()` that handle `CommodityPurpose::Treatment`.

### 5. Remove `GoalKind::Heal` match arms in ranking

Replace all `Heal` arms with the new `TreatWounds` arms.

## Files to Touch

- `crates/worldwake-ai/src/ranking.rs` (modify)

## Out of Scope

- Candidate generation (ticket 005)
- Goal model/planner semantics (ticket 004)
- Failure handling (ticket 007)
- Profile tuning (default values are set in ticket 002)
- Golden tests (ticket 008)

## Acceptance Criteria

### Tests That Must Pass

1. Self-`TreatWounds` uses `pain_weight` for motive score
2. Other-`TreatWounds` uses `care_weight` for motive score
3. Agent with high `care_weight` + low `pain_weight` ranks other-care above self-care at equal pain
4. Agent with high `pain_weight` + low `care_weight` ranks self-care above other-care at equal pain
5. `treatment_pain()` and `treatment_score()` no longer exist (compile check)
6. No `AcquireCommodity { purpose: Treatment }` ranking branches exist
7. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. Self-care ranking is governed by `pain_weight`, not `care_weight`
2. Other-care ranking is governed by `care_weight`, not `pain_weight`
3. Priority class for care is derived from the patient's pain level, not the max of actor+patient
4. All ranking is patient-aware — no commodity-only care ranking exists

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/ranking.rs` — new: self-TreatWounds motive uses `pain_weight`
2. `crates/worldwake-ai/src/ranking.rs` — new: other-TreatWounds motive uses `care_weight`
3. `crates/worldwake-ai/src/ranking.rs` — new: high-care-weight agent prioritizes other-care
4. `crates/worldwake-ai/src/ranking.rs` — new: high-pain-weight agent prioritizes self-care
5. `crates/worldwake-ai/src/ranking.rs` — remove tests for `treatment_pain` and `treatment_score`
6. `crates/worldwake-ai/src/ranking.rs` — remove/update tests for `AcquireCommodity { purpose: Treatment }` ranking

### Commands

1. `cargo test -p worldwake-ai`
2. `cargo clippy -p worldwake-ai`

## Outcome

- **Completion date**: 2026-03-18
- **What changed**:
  - Replaced `GoalKind::Heal` arms in `priority_class()` and `motive_score()` with `TreatWounds` self/other split (`pain_weight` vs `care_weight`)
  - Deleted `treatment_pain()`, `treatment_score()`, `promote_priority_class()`
  - Removed `CommodityPurpose::Treatment` branches from priority and motive ranking
  - Removed treatment-specific logic from `commodity_goal_priority()` and `commodity_goal_motive_score()`
  - Updated `goal_kind_discriminant()` from `Heal` to `TreatWounds`
  - Replaced 1 old test with 4 new tests (self-motive, other-motive, high-care-weight, high-pain-weight)
- **Deviations**: Removed `promote_priority_class()` (became dead code after removing danger-class promotion from TreatWounds priority, per spec D06 which does not include danger promotion)
- **Verification**: `cargo test --workspace` (1877 passed, 0 failed), `cargo clippy --workspace` clean
