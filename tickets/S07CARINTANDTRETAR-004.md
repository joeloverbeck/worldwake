# S07CARINTANDTRETAR-004: TreatWounds goal model — ops, satisfaction, binding, tag

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — goal model semantics in worldwake-ai
**Deps**: S07CARINTANDTRETAR-001 (TreatWounds variant must exist in GoalKind)

## Problem

The planner's goal model must map `TreatWounds { patient }` to the correct op set, satisfaction condition, affordance binding, hypothetical outcome, relevant places, and relevant commodities. Currently these are split between `Heal` and `AcquireCommodity { purpose: Treatment }`. This ticket unifies them under `TreatWounds`.

## Assumption Reassessment (2026-03-17)

1. `GoalKindTag::Heal` exists in `goal_model.rs:18` enum — confirmed
2. `GoalKindPlannerExt` trait has methods: `goal_kind_tag`, `relevant_op_kinds`, `relevant_observed_commodities`, `is_satisfied`, `hypothetical_outcome`, `relevant_places`, `matches_binding` — confirmed via grep
3. `HEAL_OPS` constant exists with `[Travel, Heal, Trade, QueueForFacilityUse, Craft]` — to be confirmed in file
4. `matches_binding()` for `Heal` does exact-bound patient check on terminal `Heal` op per S03 — confirmed
5. Satisfaction for `Heal` checks `pain_summary(patient) == Some(Permille(0))` — to be confirmed

## Architecture Check

1. `TreatWounds` reuses the same op set as `Heal` (renamed constant `TREAT_WOUNDS_OPS`). The planner treats medicine procurement as subordinate steps.
2. No separate `AcquireCommodity { purpose: Treatment }` path exists after this — all treatment acquisition flows through `TREAT_WOUNDS_OPS` containing `Trade` and `Craft`.
3. Patient identity exact-binding on terminal `Heal` op leverages S03's `matches_binding()` infrastructure directly.

## What to Change

### 1. Replace `GoalKindTag::Heal` with `GoalKindTag::TreatWounds`

In the `GoalKindTag` enum, rename the variant.

### 2. Update `goal_kind_tag()` match

`GoalKind::TreatWounds { .. } => GoalKindTag::TreatWounds`

### 3. Rename `HEAL_OPS` to `TREAT_WOUNDS_OPS`

Same contents: `[Travel, Heal, Trade, QueueForFacilityUse, Craft]`.

### 4. Update `relevant_op_kinds()`

`GoalKind::TreatWounds { .. }` returns `&TREAT_WOUNDS_OPS`.

Remove `AcquireCommodity { purpose: Treatment }` special-casing if any exists for treatment-specific ops.

### 5. Update `is_satisfied()`

`GoalKind::TreatWounds { patient }` satisfied when `pain_summary(patient) == Some(Permille(0))`.

### 6. Update `matches_binding()`

`GoalKind::TreatWounds { patient }`:
- Terminal `Heal` op: `authoritative_targets.contains(patient)` — exact match
- Auxiliary ops (Travel, Trade, QueueForFacilityUse, Craft): always pass

### 7. Update `hypothetical_outcome()`, `relevant_places()`, `relevant_observed_commodities()`

Replace `Heal` arms with `TreatWounds` arms.

### 8. Update `PlannerOpSemantics` relevant_goal_kinds references

Any `relevant_goal_kinds` arrays that reference `GoalKindTag::Heal` must be updated to `GoalKindTag::TreatWounds`.

### 9. Fix all tests in `goal_model.rs`

Update tests referencing `GoalKind::Heal` or `GoalKindTag::Heal` to use `TreatWounds`.

## Files to Touch

- `crates/worldwake-ai/src/goal_model.rs` (modify)
- `crates/worldwake-ai/src/planner_ops.rs` (modify — `relevant_goal_kinds` references)

## Out of Scope

- Candidate generation changes (ticket 005)
- Ranking changes (ticket 006)
- Failure handling changes (ticket 007)
- Goal policy changes (ticket 007)
- Golden tests (ticket 008)
- Removing `AcquireCommodity { purpose: Treatment }` ranking branches (ticket 006)

## Acceptance Criteria

### Tests That Must Pass

1. `GoalKindTag::TreatWounds` exists and `GoalKind::TreatWounds { .. }.goal_kind_tag()` returns it
2. `TreatWounds` ops include `[Travel, Heal, Trade, QueueForFacilityUse, Craft]`
3. `TreatWounds` satisfied when `pain_summary(patient) == Some(Permille(0))`
4. `matches_binding` exact-bound for terminal `Heal` op, pass for auxiliaries
5. No `GoalKindTag::Heal` variant exists
6. Existing suite: `cargo test -p worldwake-ai` (may require other tickets for full pass)

### Invariants

1. `GoalKindTag` has exactly 17 variants (Heal replaced by TreatWounds)
2. `TreatWounds` satisfaction is patient-semantic (pain-free), NOT commodity-semantic (holding medicine)
3. Patient identity exact-bound via `matches_binding` for terminal Heal op
4. Auxiliary ops always pass binding — planner can freely acquire, craft, travel

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/goal_model.rs` — update `goal_kind_tag` test to assert `TreatWounds` variant
2. `crates/worldwake-ai/src/goal_model.rs` — update or add test for `TREAT_WOUNDS_OPS` content
3. `crates/worldwake-ai/src/goal_model.rs` — update satisfaction test for `TreatWounds`
4. `crates/worldwake-ai/src/goal_model.rs` — update binding test for `TreatWounds`

### Commands

1. `cargo test -p worldwake-ai`
2. `cargo clippy -p worldwake-ai`
