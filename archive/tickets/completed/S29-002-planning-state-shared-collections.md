# S29-002: Migrate PlanningState to SharedMap/SharedSet

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: None
**Deps**: S29-001

## Problem

`PlanningState::clone()` deep-clones all 15 BTreeMap fields and 1 BTreeSet field at every search expansion. Typical expansions mutate only 1-4 fields, so the other 12-15 clones are wasted work. This ticket replaces the raw collections with the `SharedMap`/`SharedSet` wrappers from S29-001, making clone O(1) for unmutated fields.

## Assumption Reassessment (2026-03-27)

1. `PlanningState` at `crates/worldwake-ai/src/planning_state.rs` still has exactly 15 `BTreeMap` fields and 1 `BTreeSet` field. The `snapshot: &'snapshot PlanningSnapshot` reference and `next_hypothetical_id: u32` scalar remain outside the migration scope.
2. S29-001 is now completed and the wrappers live at `crates/worldwake-ai/src/shared_collections.rs` as crate-private `SharedMap`, `SharedSet`, and `SharedVec`. This ticket should consume those exact types rather than redefining or widening them.
3. The wrapper API that actually exists is intentionally minimal and matches the current `PlanningState` needs: `SharedMap` exposes `new`, `get`, `insert`, `remove`, `entry`, `iter`, `keys`, `is_empty`, `contains_key`, and `len`; `SharedSet` exposes `new`, `contains`, `insert`, `is_empty`, and `len`.
4. `PlanningState` mutation sites do line up with that surface, including the real `reservation_shadows.entry(entity).or_default()` path in `crates/worldwake-ai/src/planning_state.rs`, but the file is not a type-only swap. The current code has three raw `for ... in &self.<map>` loops over `support_declaration_*_overrides` that must change to explicit `.iter()` calls because `SharedMap` does not implement `IntoIterator` for `&SharedMap`.
5. `planning_state.rs` still uses `BTreeMap`/`BTreeSet` for local temporaries, traversal state, and test fixtures, so the standard-collection imports need to be narrowed rather than removed wholesale.
6. `PlanningState` derives `Clone`, and the wrapper derives preserve that surface. The ticket should continue to rely on deriving `Clone`, not custom clone logic.
7. `PlanningState` is transient search state and is never serialized. No save/load or cross-crate data-contract change is involved.
8. `crates/worldwake-ai/src/planning_state.rs` already has extensive focused tests, including runtime-belief-view parity checks and clone-heavy hypothetical-branch tests. Those remain the main behavioral proof surface, but the migration should add at least one focused branch-isolation test that exercises cloned overlay state directly because the optimization target is clone behavior, not planner semantics.
9. `crates/worldwake-ai/src/shared_collections.rs` still carries a file-level `#![allow(dead_code)]`. Reassessment shows `SharedMap` and `SharedSet` become live production code in this ticket, but `SharedVec` remains deferred to S29-003, so the allowance should be narrowed to the still-deferred `SharedVec` surface instead of removed wholesale.

## Architecture Check

1. This is a contained migration of the planner's transient overlay state onto the structural-sharing layer already introduced in S29-001. That is cleaner than adding ad hoc clone-avoidance special cases inside `PlanningState` methods because the clone policy stays centralized in one internal abstraction.
2. The cleaner architecture is to keep `SharedMap`/`SharedSet` narrow and explicit rather than teaching them to impersonate full `BTreeMap`/`BTreeSet` through additional iterator traits just to preserve a few call sites. Small `.iter()` adjustments inside `PlanningState` are preferable to widening the abstraction surface before there is a second real consumer.
3. The migration should stay exact and narrow: replace the overlay storage types, keep the `PlanningState` public method surface stable, and avoid broad local rewrites in a file that already carries a lot of planner semantics.
4. No backwards-compatibility shims. The raw `BTreeMap`/`BTreeSet` overlay fields are replaced, not aliased or kept in parallel.

## Verification Layers

1. `PlanningState::clone()` produces logically identical state → existing `planning_state` unit tests
2. Search outcomes unchanged → all golden tests (`cargo test -p worldwake-ai`)
3. Determinism preserved at planner/search outcome level -> existing search and golden tests
4. Branch clone isolation across migrated overlay fields -> focused `planning_state` unit tests
5. Staged infrastructure hygiene (`SharedMap` / `SharedSet` are now real consumers, so temporary unused-code allowance can shrink or disappear) -> `cargo clippy --workspace`

## What to Change

### 1. Update `PlanningState` struct fields

Replace all 15 `BTreeMap<K, V>` fields with `SharedMap<K, V>` and `removed_entities: BTreeSet<PlanningEntityRef>` with `SharedSet<PlanningEntityRef>`.

### 2. Update `PlanningState::new()`

Replace all `BTreeMap::new()` with `SharedMap::new()` and `BTreeSet::new()` with `SharedSet::new()`.

### 3. Update mutation methods

All methods using `.insert()`, `.remove()`, `.entry()` on the BTreeMap/BTreeSet fields now call the same methods on `SharedMap`/`SharedSet`. Most method bodies should remain unchanged, but the three raw borrowed-map loops must switch to `.iter()` because the wrapper API is intentionally narrower than `BTreeMap`.

### 4. Update read methods

Methods using `.get()`, `.contains()`, `.keys()`, `.iter()`, `.is_empty()` work identically on `SharedMap`/`SharedSet`. No semantic changes are expected.

### 5. Update imports

Add `use crate::shared_collections::{SharedMap, SharedSet};` and keep `BTreeMap` / `BTreeSet` imports only where local temporaries or tests still need them.

### 6. Narrow staged dead-code allowance

Replace the file-level `#![allow(dead_code)]` in `crates/worldwake-ai/src/shared_collections.rs` with a narrower allowance that only covers the still-deferred `SharedVec` surface used by S29-003.

## Files to Touch

- `crates/worldwake-ai/src/planning_state.rs` (modify — field types, constructor, imports)
- `crates/worldwake-ai/src/shared_collections.rs` (modify — narrow dead-code allowance now that `SharedMap` / `SharedSet` are live)

## Out of Scope

- `SearchNode.steps` migration (that is S29-003).
- `search/mod.rs`, `search/transition.rs` (that is S29-003).
- Benchmarking (that is S29-004).
- Adding new methods to `SharedMap`/`SharedSet` beyond what S29-001 now provides unless reassessment proves `PlanningState` has a real unmet consumer need.
- Any file outside `planning_state.rs` and `shared_collections.rs`.
- `planner_ops.rs`, `search/*.rs`, or any other consumer of `PlanningState` — they only call public methods whose signatures do not change.

## Acceptance Criteria

### Tests That Must Pass

1. All existing `planning_state` unit tests pass unchanged: `cargo test -p worldwake-ai planning_state`
2. All existing golden tests pass unchanged: `cargo test -p worldwake-ai golden`
3. All existing search tests pass unchanged: `cargo test -p worldwake-ai search`
4. Full workspace: `cargo test --workspace`
5. `cargo clippy --workspace` — no new warnings.

### Invariants

1. `PlanningState::clone()` is O(1) when no fields have been mutated since construction.
2. `PlanningState::clone()` is O(k) where k = number of fields mutated (via `Rc::make_mut` triggering inner clone only on mutated fields).
3. All `PlanningState` query methods return identical results before and after this change.
4. Search plans produced are bit-identical (same steps, same terminal kinds, same expansion counts).
5. Golden test hash values are unchanged (determinism preserved).
6. Any temporary unused-code allowance introduced in S29-001 is narrowed so only the still-deferred `SharedVec` surface remains covered.

## Test Plan

### New/Modified Tests

1. `planning_state` focused clone-isolation coverage in `crates/worldwake-ai/src/planning_state.rs`
Rationale: prove that after the migration, cloning `PlanningState` still yields independent overlay branches when one branch mutates shared maps/sets.
2. Existing `shared_collections` unit tests in `crates/worldwake-ai/src/shared_collections.rs`
Rationale: they remain the direct proof that `SharedMap` / `SharedSet` preserve copy-on-write semantics while the planner becomes a real production consumer.

### Commands

1. `cargo test -p worldwake-ai` (targeted — covers planning_state tests, search tests, and golden tests)
2. `cargo test --workspace` (full verification)
3. `cargo clippy --workspace` (lint verification)

## Outcome

- Completed: 2026-03-27
- Actual changes:
  - Migrated all `PlanningState` overlay `BTreeMap` / `BTreeSet` fields in `crates/worldwake-ai/src/planning_state.rs` to `SharedMap` / `SharedSet`.
  - Kept the wrapper abstraction narrow and updated the three borrowed-map loops in `PlanningState` to use explicit `.iter()` calls instead of widening `SharedMap` to mimic raw `BTreeMap` iteration traits.
  - Narrowed the staged dead-code allowance in `crates/worldwake-ai/src/shared_collections.rs` from a file-level allowance to the still-deferred `SharedVec` surface plus the currently staged `SharedMap` / `SharedSet` methods not yet used in production call sites.
  - Added focused clone-isolation coverage in `planning_state::tests::cloned_overlay_mutations_do_not_leak_between_branches`.
- Deviations from original plan:
  - The migration was not a type-declaration-only swap. `PlanningState` needed small method-body updates at the real iteration sites over support-declaration override maps.
  - A second production file (`crates/worldwake-ai/src/shared_collections.rs`) was updated to keep staged dead-code hygiene accurate after `PlanningState` became a live consumer.
- Verification results:
  - `cargo test -p worldwake-ai planning_state` ✅
  - `cargo test -p worldwake-ai shared_collections` ✅
  - `cargo test -p worldwake-ai` ✅
  - `cargo test --workspace` ✅
  - `cargo clippy --workspace` ✅
