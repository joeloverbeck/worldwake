# S36DECGOAREG-004: Derive planner-op reverse membership from declarations

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: None
**Deps**: S36DECGOAREG-002

## Problem

`planner_ops.rs` maintains 21 manually-curated `GOALS_*` arrays that map `PlannerOpKind` → `&[GoalKindTag]`. This is the inverse of the `relevant_ops` mapping in declarations (`GoalDispatchKey` → `&[PlannerOpKind]`). Maintaining both matrices manually creates a real two-table drift risk. The reverse membership should be derived by iterating the declaration table and inverting the `relevant_ops` mapping.

## Assumption Reassessment (2026-03-29)

1. `GOALS_*` arrays are defined at `planner_ops.rs:59-132`. There are 21 arrays covering all 28 `PlannerOpKind` variants that have associated goals.
2. These arrays are consumed by `PlannerOpSemantics.relevant_goal_kinds` field (`planner_ops.rs:41-48`), which is constructed in `semantics_for()` (`planner_ops.rs:207-305`) and `social_or_combat_semantics()`.
3. `PlannerOpSemantics.relevant_goal_kinds` is typed as `&'static [GoalKindTag]`. The reverse-derived version must also produce `GoalKindTag` slices to maintain the contract.
4. The current arrays use `GoalKindTag` (coarse), not `GoalDispatchKey` (fine). The derivation must map `GoalDispatchKey → relevant_ops` back to `GoalKindTag` for each op, since `PlannerOpSemantics` uses `GoalKindTag`. This requires a `GoalDispatchKey::goal_kind_tag()` method or equivalent.
5. `build_semantics_table()` at `planner_ops.rs:135-144` constructs the full semantics table from the action def registry. The `GOALS_*` arrays are only referenced from `semantics_for()` and `social_or_combat_semantics()`.

## Architecture Check

1. Deriving reverse membership eliminates the two-table drift risk identified in the spec. When a new goal adds `PlannerOpKind::Trade` to its `relevant_ops`, the reverse membership for `Trade` automatically includes that goal's tag. P26 (No Backward Compatibility): the manual arrays are fully replaced, not shimmed.
2. The derivation can be done at initialization time (in `build_semantics_table()`) or via `lazy_static`/`OnceLock`. Since `PlannerOpSemantics` requires `&'static [GoalKindTag]`, the derived slices need static lifetime — this may require `OnceLock<Vec<GoalKindTag>>` with leaked allocations, or a compile-time approach. The implementation should choose the simplest approach that maintains the `&'static` contract.

## Verification Layers

1. Reverse membership equivalence → focused unit test: for every `PlannerOpKind`, the derived `relevant_goal_kinds` set equals the current manual `GOALS_*` set.
2. Behavioral equivalence → full AI test suite: all golden tests pass unchanged (search uses `relevant_goal_kinds` to filter candidates).
3. Single-layer ticket: static data derivation only.

## What to Change

### 1. Add `GoalDispatchKey::goal_kind_tag()` method

Return the coarse `GoalKindTag` for a dispatch key. Multiple keys may map to the same tag (e.g., `AcquireNeedDriven` and `AcquireRestock` both → `GoalKindTag::AcquireCommodity`).

### 2. Add `GoalDispatchKey::all_keys()` iterator

Return an iterator or slice of all `GoalDispatchKey` variants (needed for derivation). Can be a `const` array.

### 3. Derive reverse membership function

Create a function that iterates all declaration keys, collects `relevant_ops` per key, and inverts the mapping to produce `PlannerOpKind → BTreeSet<GoalKindTag>`. Deduplicate tags (multiple dispatch keys may map to the same `GoalKindTag`).

### 4. Replace `GOALS_*` arrays in `planner_ops.rs`

Replace the 21 manually-curated `GOALS_*` arrays with derived data. Update `semantics_for()` and `social_or_combat_semantics()` to use the derived reverse membership. The `PlannerOpSemantics.relevant_goal_kinds` field type and semantics remain unchanged.

### 5. Remove dead `GOALS_*` constants

After migration, the 21 `GOALS_*` constants in `planner_ops.rs` are dead code. Remove them.

## Files to Touch

- `crates/worldwake-ai/src/goal_dispatch_key.rs` (modify — add `goal_kind_tag()`, `all_keys()`)
- `crates/worldwake-ai/src/planner_ops.rs` (modify — replace GOALS_* with derived data)

## Out of Scope

- Changing the `PlannerOpSemantics.relevant_goal_kinds` field type (remains `&'static [GoalKindTag]`)
- Migrating `GoalKindTag` consumers to use `GoalDispatchKey` instead
- Trace label migration (ticket 005)
- Invalidation/feasibility strategy migration (tickets 006–007)
- Removing `*_OPS` const arrays from `goal_model.rs` (may still be used by declarations)

## Acceptance Criteria

### Tests That Must Pass

1. `test_reverse_membership_equivalence`: For every `PlannerOpKind` that has a `GOALS_*` array, the derived reverse membership produces the same set of `GoalKindTag` values as the current manual array.
2. `test_reverse_membership_completeness`: Every `PlannerOpKind` that appears in any declaration's `relevant_ops` has a non-empty reverse membership set.
3. Existing suite: `cargo test -p worldwake-ai`
4. Full workspace: `cargo test --workspace`

### Invariants

1. Zero behavioral change — plan search filtering produces identical results.
2. `PlannerOpSemantics.relevant_goal_kinds` field type and contract unchanged.
3. No manual `GOALS_*` arrays remain in `planner_ops.rs` after migration.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/planner_ops.rs` or `goal_dispatch_key.rs` (test module) — reverse membership equivalence and completeness tests.

### Commands

1. `cargo test -p worldwake-ai -- reverse_membership`
2. `cargo test -p worldwake-ai`
3. `cargo test --workspace`
4. `cargo clippy --workspace`
