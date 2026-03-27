# S29-002: Migrate PlanningState to SharedMap/SharedSet

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: None
**Deps**: S29-001

## Problem

`PlanningState::clone()` deep-clones all 15 BTreeMap fields and 1 BTreeSet field at every search expansion. Typical expansions mutate only 1-4 fields, so the other 12-15 clones are wasted work. This ticket replaces the raw collections with the `SharedMap`/`SharedSet` wrappers from S29-001, making clone O(1) for unmutated fields.

## Assumption Reassessment (2026-03-27)

1. `PlanningState` at `planning_state.rs:38-60` has exactly 15 `BTreeMap` fields and 1 `BTreeSet` field. The `snapshot: &'snapshot PlanningSnapshot` reference and `next_hypothetical_id: u32` scalar are NOT collection fields and remain unchanged.
2. All mutation methods on `PlanningState` (`move_entity_ref`, `with_support_declaration`, `set_needs`, `set_pain`, `consume_resource`, `split_lot`, etc.) take `mut self` and modify specific BTreeMap fields via `.insert()`, `.remove()`, `.entry()`. These map exactly to the `SharedMap` API from S29-001.
3. All read methods (`entity_place`, `direct_container_of`, `is_removed`, `homeostatic_needs_for`, etc.) use `.get()`, `.contains_key()`, `.iter()`, `.keys()` — all available on `SharedMap`/`SharedSet`.
4. `PlanningState` derives `Clone` — this derive works with `SharedMap`/`SharedSet` since they also derive `Clone`.
5. `PlanningState` is never serialized (transient within `search_plan`). No save/load impact.
6. The file is ~3879 lines. Most mutation/read methods need only type changes (BTreeMap → SharedMap, BTreeSet → SharedSet) in the field declarations; the API calls are the same.
7. `planning_state.rs` also has `#[cfg(test)] mod tests` at the bottom with existing tests — these must continue to pass unchanged.

## Architecture Check

1. This is a mechanical type substitution. `SharedMap` exposes the same API surface as `BTreeMap` for the operations used by `PlanningState`. No new abstractions, no new indirection patterns.
2. No backwards-compatibility shims. The old BTreeMap fields are replaced, not wrapped or aliased.

## Verification Layers

1. `PlanningState::clone()` produces logically identical state → existing `planning_state` unit tests
2. Search outcomes unchanged → all golden tests (`cargo test -p worldwake-ai`)
3. Determinism preserved → golden hash comparisons in existing golden tests
4. Single-crate ticket: no cross-system interaction.

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

Add `use crate::shared_collections::{SharedMap, SharedSet};` and remove `use std::collections::{BTreeMap, BTreeSet};` (or keep BTreeMap if still used for local temporaries).

## Files to Touch

- `crates/worldwake-ai/src/planning_state.rs` (modify — field types, constructor, imports)

## Out of Scope

- `SearchNode.steps` migration (that is S29-003).
- `search/mod.rs`, `search/transition.rs` (that is S29-003).
- Benchmarking (that is S29-004).
- Adding new methods to `SharedMap`/`SharedSet` beyond what S29-001 provides (if needed, update S29-001 first).
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
6. `PlanningState` is still `!Send` (Rc is !Send, matching the existing &'snapshot borrow constraint).

## Test Plan

### New/Modified Tests

1. None — this is a pure type-substitution refactor. All behavioral verification comes from the existing test suite. No new tests needed because the existing planning_state, search, and golden test suites cover all affected code paths.

### Commands

1. `cargo test -p worldwake-ai` (targeted — covers planning_state tests, search tests, and golden tests)
2. `cargo clippy --workspace && cargo test --workspace` (full verification)
