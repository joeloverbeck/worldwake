# S36DECGOAREG-003: Migrate provenance family and relevant ops to declaration lookups

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: None
**Deps**: S36DECGOAREG-002

## Problem

`ranked_goal_provenance_family()` and `relevant_op_kinds()` are implemented as per-goal match statements in `goal_model.rs`. With declarations in place (002), these should route through the declaration table, making the trait methods thin wrappers and the `*_OPS` const arrays redundant (though removal of the arrays is deferred to after reverse-membership derivation in 004).

## Assumption Reassessment (2026-03-29)

1. `ranked_goal_provenance_family()` is at `goal_model.rs:518-550`. Exhaustive match, no wildcard. Returns `Option<RankedGoalProvenanceFamily>`. Consumed by `ranking.rs` via the trait method.
2. `relevant_op_kinds()` is at `goal_model.rs:552-578`. Exhaustive match, no wildcard. Returns `&'static [PlannerOpKind]`. Consumed by search (`search/` module) for plan-search filtering.
3. Both methods are on the `GoalKindPlannerExt` trait (`goal_model.rs:59-107`), which is implemented for `GoalKind`.
4. The 21 `*_OPS` const arrays (`goal_model.rs:109-183`) are referenced only by `relevant_op_kinds()` and will also be referenced by declarations (002). After this migration, the arrays remain because `planner_ops.rs` GOALS_* arrays reference `GoalKindTag` (a separate concern addressed in 004).
5. No other crate outside `worldwake-ai` calls these trait methods.

## Architecture Check

1. Routing through declarations makes the trait methods one-liners (`self.dispatch_key().declaration().field`), centralizing the source of truth. This is cleaner than maintaining two parallel match statements (the declaration table and the trait impl). P24 (Systems Interact Through State): consumers read declaration metadata rather than importing dispatch logic.
2. No backwards-compatibility shims. The trait methods remain on `GoalKindPlannerExt` with identical signatures and return types — callers are unaffected.

## Verification Layers

1. Provenance equivalence → focused unit test: for every `GoalKind` variant, old and new provenance results are identical.
2. Relevant-ops equivalence → focused unit test: for every `GoalKind` variant, old and new relevant-ops slices contain the same elements.
3. Behavioral equivalence → full AI test suite: all golden tests pass unchanged.
4. Single-layer ticket: dispatch routing only, no cross-system interaction.

## What to Change

### 1. Migrate `ranked_goal_provenance_family()` in `goal_model.rs`

Replace the exhaustive match body with:
```rust
fn ranked_goal_provenance_family(&self) -> Option<RankedGoalProvenanceFamily> {
    GoalDispatchKey::from_goal_kind(self).declaration().provenance_family
}
```

### 2. Migrate `relevant_op_kinds()` in `goal_model.rs`

Replace the exhaustive match body with:
```rust
fn relevant_op_kinds(&self) -> &'static [PlannerOpKind] {
    GoalDispatchKey::from_goal_kind(self).declaration().relevant_ops
}
```

### 3. Equivalence tests

Add pre-migration snapshot tests or direct comparison tests that verify identical results for all 21 GoalKind variants (including payload-sensitive splits).

## Files to Touch

- `crates/worldwake-ai/src/goal_model.rs` (modify — replace two method bodies)

## Out of Scope

- Removing the `*_OPS` const arrays from `goal_model.rs` (they may still be referenced by declarations; cleanup is a follow-up after 004 confirms they are dead code)
- Migrating `planner_ops.rs` GOALS_* arrays or reverse membership (ticket 004)
- Trace label migration (ticket 005)
- Strategy selectors for invalidation/feasibility (tickets 006–007)
- Any changes to the `GoalKindPlannerExt` trait signature
- Any changes to `worldwake-core`

## Acceptance Criteria

### Tests That Must Pass

1. `test_provenance_migration_equivalence`: For every `GoalKind` variant (including `AcquireCommodity` with all three purposes and `PunishAccused` with both punishment kinds), the declaration-routed provenance matches the pre-migration result.
2. `test_relevant_ops_migration_equivalence`: Same coverage — declaration-routed relevant_ops matches pre-migration result for all goal shapes.
3. Existing suite: `cargo test -p worldwake-ai`
4. Full workspace: `cargo test --workspace`

### Invariants

1. Zero behavioral change — all existing tests pass unchanged.
2. `GoalKindPlannerExt` trait signature is unchanged — no caller modifications needed.
3. The `*_OPS` const arrays in `goal_model.rs` remain available (not yet deleted).

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/goal_model.rs` (test module or `goal_dispatch_decl.rs` test module) — equivalence tests for provenance and relevant_ops across all goal shapes.

### Commands

1. `cargo test -p worldwake-ai -- ranked_goal_provenance`
2. `cargo test -p worldwake-ai -- relevant_op`
3. `cargo test -p worldwake-ai`
4. `cargo test --workspace`
5. `cargo clippy --workspace`
