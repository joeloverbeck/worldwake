# S29-002: Migrate PlanningState to SharedMap/SharedSet

**Status**: PENDING
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
4. `PlanningState` mutation sites do line up with that surface, including the real `reservation_shadows.entry(entity).or_default()` path in `crates/worldwake-ai/src/planning_state.rs`. This remains the main method-body compatibility constraint.
5. The ticket is not a pure find-and-replace. `planning_state.rs` still uses `BTreeMap`/`BTreeSet` for local temporaries and test fixtures, so the imports need to be narrowed rather than removed wholesale.
6. `PlanningState` derives `Clone`, and the wrapper derives preserve that surface. The ticket should continue to rely on deriving `Clone`, not custom clone logic.
7. `PlanningState` is transient search state and is never serialized. No save/load or cross-crate data-contract change is involved.
8. `crates/worldwake-ai/src/planning_state.rs` already has extensive focused tests. Those existing tests remain the primary behavioral proof surface, but this ticket should also remove S29-001's temporary `#![allow(dead_code)]` if `PlanningState` becomes the last missing consumer keeping `SharedMap`/`SharedSet` unused.

## Architecture Check

1. This is a contained migration of the planner's transient overlay state onto the structural-sharing layer already introduced in S29-001. That is cleaner than adding ad hoc clone-avoidance special cases inside `PlanningState` methods because the clone policy stays centralized in one internal abstraction.
2. The migration should stay exact and narrow: replace the overlay storage types, keep the `PlanningState` public method surface stable, and avoid broad local rewrites in a file that already carries a lot of planner semantics.
3. No backwards-compatibility shims. The raw `BTreeMap`/`BTreeSet` overlay fields are replaced, not aliased or kept in parallel.

## Verification Layers

1. `PlanningState::clone()` produces logically identical state → existing `planning_state` unit tests
2. Search outcomes unchanged → all golden tests (`cargo test -p worldwake-ai`)
3. Determinism preserved at planner/search outcome level -> existing search and golden tests
4. Staged infrastructure hygiene (`SharedMap` / `SharedSet` are now real consumers, so temporary unused-code allowance can shrink or disappear) -> `cargo clippy --workspace`

## What to Change

### 1. Update `PlanningState` struct fields

Replace all 15 `BTreeMap<K, V>` fields with `SharedMap<K, V>` and `removed_entities: BTreeSet<PlanningEntityRef>` with `SharedSet<PlanningEntityRef>`.

### 2. Update `PlanningState::new()`

Replace all `BTreeMap::new()` with `SharedMap::new()` and `BTreeSet::new()` with `SharedSet::new()`.

### 3. Update mutation methods

All methods using `.insert()`, `.remove()`, `.entry()` on the BTreeMap/BTreeSet fields now call the same methods on `SharedMap`/`SharedSet`. The API is identical — this should be a find-and-replace in the type declarations only; method bodies should not change.

### 4. Update read methods

Methods using `.get()`, `.contains_key()`, `.keys()`, `.iter()`, `.is_empty()` work identically on `SharedMap`/`SharedSet`. No changes to method bodies expected.

### 5. Update imports

Add `use crate::shared_collections::{SharedMap, SharedSet};` and keep `BTreeMap` / `BTreeSet` imports only where local temporaries or tests still need them.

## Files to Touch

- `crates/worldwake-ai/src/planning_state.rs` (modify — field types, constructor, imports)

## Out of Scope

- `SearchNode.steps` migration (that is S29-003).
- `search/mod.rs`, `search/transition.rs` (that is S29-003).
- Benchmarking (that is S29-004).
- Adding new methods to `SharedMap`/`SharedSet` beyond what S29-001 now provides unless reassessment proves `PlanningState` has a real unmet consumer need.
- Any file outside `planning_state.rs`.
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
6. Any temporary unused-code allowance introduced in S29-001 is removed or narrowed once `PlanningState` starts consuming `SharedMap` / `SharedSet`.

## Test Plan

### New/Modified Tests

1. None expected unless reassessment uncovers a `PlanningState` edge path not already covered by the existing focused tests. This remains a migration ticket, not a semantics-changing planner ticket.

### Commands

1. `cargo test -p worldwake-ai` (targeted — covers planning_state tests, search tests, and golden tests)
2. `cargo clippy --workspace && cargo test --workspace` (full verification)
