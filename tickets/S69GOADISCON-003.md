# S69GOADISCON-003: Migrate consumers and remove standalone functions

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: None — consumer migration and dead code removal, no runtime behavior change
**Deps**: S69GOADISCON-002

## Problem

After ticket 002 established the declaration table as the source of truth for `family_policy` and `progress_barrier_ops`, the standalone functions (`goal_family_policy()`) and inline checks (per-goal direct op_kind barriers in `is_progress_barrier()`) are redundant. This ticket switches all consumers to read from the declaration and removes the emptied code.

## Assumption Reassessment (2026-04-07)

1. `goal_family_policy()` callers are in `interrupts.rs` (lines 77, 98, 127) and `evaluate_suppression()` in `goal_policy.rs` (line 220). Import at `interrupts.rs:5`. All confirmed current.
2. `evaluate_suppression()` at `goal_policy.rs:219` calls `goal_family_policy(kind)` at line 220. After migration, it reads `GoalDispatchKey::from_goal_kind(kind).declaration().family_policy` instead.
3. `is_progress_barrier()` in `goal_model.rs:1041-1159` has 4 layers:
   - (a) `QueueForFacilityUse` check for 7 goal families (lines 1042-1053) — STAYS
   - (b) 13 per-goal direct op_kind barriers (lines 1055-1127) — REPLACED by `progress_barrier_ops.contains()`
   - (c) `ConsumeOwnedCommodity`/`MoveCargo` special case (lines 1129-1137) — STAYS
   - (d) `is_materialization_barrier` flag check (lines 1139-1159) — STAYS
4. `GoalFamilyPolicy` struct and its supporting types (`SuppressionRule`, `PenaltyInterruptEligibility`, `FreeInterruptRole`) remain in `goal_policy.rs` — they are the field type, not being removed.
5. Cross-validation tests from ticket 002 (`test_family_policy_matches_standalone_function`, `test_progress_barrier_ops_match_goal_model`) should be removed or converted to pure declaration-only tests after migration, since the standalone reference no longer exists.

## Architecture Check

1. After this ticket, static goal metadata lives exclusively in `GoalDispatchDeclaration`. Runtime-dependent decisions (`priority_class()`, `evaluate_suppression()`, residual `is_progress_barrier()` logic) remain in their functions. This is a clean separation: compile-time constants in the declaration table, runtime computations in functions.
2. No backward-compatibility shims. `goal_family_policy()` is deleted, not deprecated. The per-goal direct op_kind barriers are replaced with a single `contains()` call, not wrapped.

## Verification Layers

1. Behavioral equivalence → all existing golden tests pass with identical outcomes (`cargo test -p worldwake-ai`)
2. Family policy migration → `interrupts.rs` and `evaluate_suppression()` read from declaration; `goal_family_policy()` no longer exists (grep confirms removal)
3. Progress barrier migration → `is_progress_barrier()` uses `progress_barrier_ops.contains()` for direct barriers; QueueForFacilityUse, MoveCargo, and materialization barrier logic preserved
4. Dead code removal → `goal_family_policy()` function removed; no orphaned imports remain
5. Single-layer ticket (AI planner internals only) — no cross-system verification needed

## What to Change

### 1. Migrate `interrupts.rs` callers

Replace 3 call sites:
- Line 77: `goal_family_policy(&challenger.grounded.key.kind)` → `GoalDispatchKey::from_goal_kind(&challenger.grounded.key.kind).declaration().family_policy`
- Line 98: same pattern
- Line 127: same pattern (accessing `.free_interrupt`)

Update import at line 5: remove `goal_family_policy` from the `goal_policy` import, add `GoalDispatchKey` import.

### 2. Migrate `evaluate_suppression()` in `goal_policy.rs`

Line 220: replace `let policy = goal_family_policy(kind);` with `let policy = GoalDispatchKey::from_goal_kind(kind).declaration().family_policy;`

Add `use crate::GoalDispatchKey;` import if not already present.

### 3. Remove `goal_family_policy()` from `goal_policy.rs`

Delete the function (lines 96-212 approximately) and its doc comment. The `GoalFamilyPolicy` struct, `SuppressionRule`, `PenaltyInterruptEligibility`, `FreeInterruptRole`, and their supporting types remain — they are the field type used by the declaration.

Remove any tests in `goal_policy.rs` that test `goal_family_policy()` directly (these are superseded by the declaration-level tests from ticket 002).

### 4. Simplify `is_progress_barrier()` in `goal_model.rs`

Replace the 13 per-goal-kind direct op_kind barrier checks (lines 1055-1127) with:

```rust
let decl = GoalDispatchKey::from_goal_kind(self).declaration();
if !decl.progress_barrier_ops.is_empty()
    && decl.progress_barrier_ops.contains(&step.op_kind)
{
    return true;
}
```

This single block replaces the 13 individual `if matches!(self, GoalKind::X) && step.op_kind == PlannerOpKind::Y` checks.

Preserve:
- The `QueueForFacilityUse` check (lines 1042-1053)
- The `ConsumeOwnedCommodity`/`MoveCargo` special case (lines 1129-1137)
- The `is_materialization_barrier` fallthrough (lines 1139-1159)

### 5. Update cross-validation tests

Remove `test_family_policy_matches_standalone_function` from `goal_dispatch_decl.rs` (the standalone function no longer exists). Replace with `test_family_policy_declarations_cover_all_policy_variants` that verifies all `SuppressionRule`/`FreeInterruptRole` variants appear across declarations (sanity check).

`test_progress_barrier_ops_match_goal_model` can be simplified to verify that no `progress_barrier_ops` entry references a nonexistent `PlannerOpKind` variant (the cross-validation against the removed inline checks is no longer possible).

## Files to Touch

- `crates/worldwake-ai/src/interrupts.rs` (modify)
- `crates/worldwake-ai/src/goal_policy.rs` (modify — remove function, update evaluate_suppression)
- `crates/worldwake-ai/src/goal_model.rs` (modify — simplify is_progress_barrier)
- `crates/worldwake-ai/src/goal_dispatch_decl.rs` (modify — update tests)

## Out of Scope

- Modifying `ranking.rs` or `priority_class()` — runtime-dependent, explicitly excluded from S69
- Changing `GoalDispatchKey` or its variants (done in ticket 001)
- Adding new declaration fields (done in ticket 002)
- Modifying QueueForFacilityUse, MoveCargo, or materialization barrier logic in `is_progress_barrier()`
- Changing `evaluate_suppression()` behavior — only its internal data source changes

## Acceptance Criteria

### Tests That Must Pass

1. All existing golden tests: `cargo test -p worldwake-ai` — identical behavior
2. Golden soak test: `cargo test -p worldwake-ai --features soak --test golden_soak` — identical emergence patterns
3. `test_declaration_completeness` — declarations still complete
4. Existing suite: `cargo test --workspace`

### Invariants

1. No behavioral change — all decisions produce identical outcomes before and after migration
2. `goal_family_policy()` function no longer exists in the codebase
3. `is_progress_barrier()` still handles all 4 layers: QueueForFacilityUse, direct op_kind barriers (now via declaration), MoveCargo special case, materialization barriers
4. No orphaned imports or dead code after removal

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/goal_dispatch_decl.rs::test_family_policy_declarations_cover_all_policy_variants` — replaces cross-validation test with coverage check
2. Modified: `crates/worldwake-ai/src/goal_policy.rs` tests — removed tests for deleted function
3. Modified: `crates/worldwake-ai/src/goal_dispatch_decl.rs::test_progress_barrier_ops_match_goal_model` — simplified to validity check

### Commands

1. `cargo test -p worldwake-ai`
2. `cargo test -p worldwake-ai --features soak --test golden_soak`
3. `cargo clippy --workspace --all-targets -- -D warnings`
4. `grep -r "goal_family_policy" crates/worldwake-ai/src/` — must return zero results (excluding comments/docs)
