# S29-001: SharedMap, SharedSet, and SharedVec Wrapper Types

**Status**: ✅ COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: None
**Deps**: None

## Problem

`PlanningState::clone()` deep-clones 15 BTreeMaps, 1 BTreeSet, and a growing Vec per search expansion. This ticket introduces the copy-on-write wrapper types that subsequent tickets will use to replace those collections.

## Assumption Reassessment (2026-03-27)

1. Shared abstraction boundary under audit: the transient search-state clone surface formed by `PlanningState<'snapshot>` in `crates/worldwake-ai/src/planning_state.rs` and `SearchNode::steps` in `crates/worldwake-ai/src/search/mod.rs`, with the live clone sites at `crates/worldwake-ai/src/search/transition.rs:96` (`node.state.clone()`) and `crates/worldwake-ai/src/search/transition.rs:124` (`node.steps.clone()`).
2. `PlanningState` currently contains exactly 15 `BTreeMap` fields, 1 `BTreeSet` field, 1 snapshot reference, and `next_hypothetical_id: u32`. Confirmed from `crates/worldwake-ai/src/planning_state.rs`.
3. `SearchNode` currently stores `steps: Vec<PlannedStep>`, and `root_node` initializes that field with `Vec::new()` in `crates/worldwake-ai/src/search/heuristic.rs`.
4. `SharedMap`, `SharedSet`, and `SharedVec` do not yet exist in the codebase. Confirmed by searching the `worldwake-ai` crate for `shared_collections` and the proposed type names.
5. The wrapper layer is crate-internal infrastructure, not cross-crate API. It should stay `pub(crate)` and must not become another alias path or semi-public abstraction. S29-002 and S29-003 are the only planned consumers.
6. The original threading justification was too loose. The important current fact is narrower: these wrappers live only inside single-search transient AI state, and no current code sends that state across threads. This ticket should not make or depend on broader `Send`/`Sync` claims.
7. `SharedMap::entry` is not speculative API. `PlanningState` already uses `reservation_shadows.entry(entity).or_default().push(range)` at `crates/worldwake-ai/src/planning_state.rs:629-632`, so preserving an `entry` surface is required for the planned migration.
8. Verification commands need to reference real current targets. `cargo test -p worldwake-ai -- --list` succeeds today and confirms the crate test inventory; this ticket should keep targeted verification to new wrapper tests plus `cargo test -p worldwake-ai`, with broader workspace checks still required before archival.

## Architecture Check

1. `Rc<BTreeMap>` / `Rc<BTreeSet>` / `Rc<Vec>` with `Rc::make_mut` is the cleanest fit for the current architecture because the problem is localized to repeated cloning of transient search state, not to authoritative storage or cross-system contracts. It gives structural sharing without changing planner meaning, determinism, or ownership boundaries.
2. This is cleaner than ad hoc field-specific clone avoidance inside `PlanningState` methods because it centralizes the optimization in one crate-private abstraction instead of scattering clone policy across planner code.
3. This is cleaner than introducing a third-party persistent collection crate because the search state only needs shallow structural sharing, deterministic iteration, and standard mutation semantics. Pulling in a heavier persistence model would add conceptual surface area without solving a broader architectural problem.
4. The wrapper API should remain minimal and consumer-driven. The goal is not to recreate the entire `BTreeMap`/`BTreeSet`/`Vec` surface, only the methods that S29-002 and S29-003 actually need.
5. No backwards-compatibility shims or parallel aliases. These wrappers are new internal implementation details that will replace the current direct collections in subsequent tickets rather than coexist as alternate paths.

## Verification Layers

1. Wrapper structural-sharing semantics (`Clone` stays O(1) until first mutation, shared allocation before write) -> focused unit tests in `shared_collections.rs`
2. Wrapper mutation semantics (`insert`/`remove`/`entry`/`push` change only the mutated clone and preserve original contents) -> focused unit tests in `shared_collections.rs`
3. Deterministic iteration/insertion ordering parity with `BTreeMap` / `BTreeSet` / `Vec` -> focused unit tests in `shared_collections.rs`
4. Crate integration proof (new module registration and wrapper tests do not regress existing AI crate behavior before any consumer migration) -> `cargo test -p worldwake-ai`
5. Workspace hygiene proof (no new lint failures or cross-crate regressions) -> `cargo clippy --workspace` and `cargo test --workspace`

## What to Change

### 1. New file: `shared_collections.rs`

Create `crates/worldwake-ai/src/shared_collections.rs` containing:

- `SharedMap<K, V>` — `Rc<BTreeMap<K, V>>` wrapper with only the methods required by the current S29 plan surface: `new`, `get`, `insert`, `remove`, `entry`, `iter`, `keys`, `is_empty`, `contains_key`, `len`, and `Clone` (O(1) via `Rc`).
- `SharedSet<K>` — `Rc<BTreeSet<K>>` wrapper with `new`, `contains`, `insert`, `is_empty`, `len`, and `Clone` (O(1) via `Rc`).
- `SharedVec<T>` — `Rc<Vec<T>>` wrapper with `new`, `push`, `as_slice`, `len`, `is_empty`, `iter`, `into_vec`, and `Clone` (O(1) via `Rc`).

All three types derive `Clone` (the derive invokes `Rc::clone`, which is O(1)). All three are `pub(crate)` and remain internal to `worldwake-ai`.

### 2. Register module in lib.rs

Add `mod shared_collections;` to `crates/worldwake-ai/src/lib.rs`.

## Files to Touch

- `crates/worldwake-ai/src/shared_collections.rs` (new)
- `crates/worldwake-ai/src/lib.rs` (modify — add `mod shared_collections;`)

## Out of Scope

- Modifying `PlanningState` fields (that is S29-002).
- Modifying `SearchNode` or `search/transition.rs` (that is S29-003).
- Benchmarking (that is S29-004).
- Any changes outside `worldwake-ai`.
- Making the wrappers `pub` (they are crate-internal).
- `Debug`, `PartialEq`, `Eq` derives — add only if needed by downstream tickets (S29-002 will determine).

## Acceptance Criteria

### Tests That Must Pass

1. `SharedMap`: insert + get returns correct value.
2. `SharedMap`: clone is O(1) sharing (both point to same allocation — `Rc::ptr_eq`).
3. `SharedMap`: mutation after clone does NOT affect the original (independence).
4. `SharedMap`: `remove` works correctly.
5. `SharedMap`: `entry` API works (insert via entry, or_default pattern).
6. `SharedMap`: `keys` and `iter` produce BTreeMap-ordered output.
7. `SharedSet`: insert + contains works.
8. `SharedSet`: clone sharing + mutation independence.
9. `SharedSet`: iteration order remains BTreeSet order.
10. `SharedVec`: push + as_slice works.
11. `SharedVec`: clone sharing + mutation independence.
12. `SharedVec`: `iter` and `into_vec` preserve insertion order and contents.
13. Existing suite: `cargo test -p worldwake-ai` (no regressions).
14. Existing suite: `cargo clippy --workspace` and `cargo test --workspace` (no new warnings or regressions).

### Invariants

1. `SharedMap::get` returns identical results to `BTreeMap::get` for any key.
2. Iteration order of `SharedMap` is identical to `BTreeMap` (BTree ordering preserved).
3. `SharedSet::contains` returns identical results to `BTreeSet::contains`.
4. `SharedVec` maintains insertion order identical to `Vec`.
5. Cloning any shared wrapper is O(1) (reference count increment only).

## Test Plan

### New/Modified Tests

1. `shared_collections::tests::shared_map_supports_basic_accessors` — proves the wrapper preserves the expected empty-state, insert, get, `contains_key`, and length semantics before any structural-sharing behavior is layered on top.
2. `shared_collections::tests::shared_map_clone_shares_until_mutation` — proves the optimization contract directly: clone shares allocation first, then copy-on-write splits allocations only on mutation and preserves source/clone independence.
3. `shared_collections::tests::shared_map_remove_and_entry_match_btreemap_behavior` — proves the two mutating map APIs S29-002 needs most, especially the `entry().or_default()` pattern already used by `PlanningState::reservation_shadows`.
4. `shared_collections::tests::shared_map_preserves_btree_order` — proves deterministic BTree iteration/key ordering is preserved, which matters for planner determinism.
5. `shared_collections::tests::shared_set_supports_clone_and_ordering` — proves set insertion, clone-on-share, mutation independence, and deterministic ordering for the `removed_entities` migration path.
6. `shared_collections::tests::shared_vec_supports_clone_iteration_and_into_vec` — proves `SharedVec` preserves insertion order, shares until mutation, and can recover an owned `Vec` for the final plan materialization path.

No existing tests were modified.

### Commands

1. `cargo test -p worldwake-ai shared_collections::tests::` (targeted wrapper tests)
2. `cargo test -p worldwake-ai` (crate regression)
3. `cargo clippy --workspace` (workspace lint)
4. `cargo test --workspace` (workspace regression)

## Outcome

Completed: 2026-03-27

1. Added `crates/worldwake-ai/src/shared_collections.rs` with crate-private `SharedMap`, `SharedSet`, and `SharedVec` wrappers built on `Rc` + `Rc::make_mut`.
2. Added `mod shared_collections;` to `crates/worldwake-ai/src/lib.rs` so downstream S29 tickets can consume the wrappers without widening the public API.
3. Added six focused unit tests that prove clone-sharing, copy-on-write independence, deterministic ordering, and the `entry().or_default()` mutation path needed by the planned `PlanningState` migration.
4. Contained temporary staged-ticket dead-code noise inside the module itself with a local `#![allow(dead_code)]`, because S29-001 intentionally lands infrastructure before S29-002/S29-003 start consuming it.

Deviations from original plan:

1. No consumer migration was pulled into this ticket. `PlanningState` and `SearchNode` still use raw `BTreeMap`/`BTreeSet`/`Vec`, which keeps S29-001 scoped to infrastructure only.
2. The wrapper API stayed minimal and crate-private rather than trying to mirror more of the standard collection surface than the current S29 plan needs.

Verification results:

1. `cargo test -p worldwake-ai shared_collections::tests::` passed.
2. `cargo test -p worldwake-ai` passed.
3. `cargo clippy --workspace` passed.
4. `cargo test --workspace` passed.
