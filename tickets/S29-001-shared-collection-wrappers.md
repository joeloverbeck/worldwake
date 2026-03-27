# S29-001: SharedMap, SharedSet, and SharedVec Wrapper Types

**Status**: PENDING
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: None
**Deps**: None

## Problem

`PlanningState::clone()` deep-clones 15 BTreeMaps, 1 BTreeSet, and a growing Vec per search expansion. This ticket introduces the copy-on-write wrapper types that subsequent tickets will use to replace those collections.

## Assumption Reassessment (2026-03-27)

1. `PlanningState` at `crates/worldwake-ai/src/planning_state.rs:38-60` currently contains exactly 15 `BTreeMap` fields, 1 `BTreeSet` field, 1 `&'snapshot` ref, and 1 `u32`. Confirmed by reading the struct definition.
2. `SearchNode` at `crates/worldwake-ai/src/search/mod.rs:27-36` contains `steps: Vec<PlannedStep>`. Confirmed by reading the struct.
3. `SharedMap`, `SharedSet`, `SharedVec` do not yet exist in the codebase (globbed for `shared_collections*` — no files found).
4. This ticket is self-contained: the wrappers are private types with no cross-crate surface. No other ticket or spec depends on their existence until S29-002/003 consume them.
5. `Rc` is not `Send`/`Sync`, but `PlanningState` is already `!Send` (contains `&'snapshot` borrow). No threading concern.
6. Single-layer ticket: only introduces new types with unit tests. No behavioral/system interaction.

## Architecture Check

1. `Rc<BTreeMap>` with `Rc::make_mut` is the simplest CoW pattern in Rust's stdlib — no external crates, no unsafe, no lifetime gymnastics. It preserves BTreeMap's deterministic iteration order.
2. No backwards-compatibility shims. These are brand-new private types.

## Verification Layers

1. Clone sharing (Rc refcount == 2 after clone) → unit test
2. Mutation independence (mutating clone does not affect original) → unit test
3. Iteration-order parity with BTreeMap/BTreeSet/Vec → unit test
4. Single-layer ticket: wrapper types have no cross-system interaction.

## What to Change

### 1. New file: `shared_collections.rs`

Create `crates/worldwake-ai/src/shared_collections.rs` containing:

- `SharedMap<K, V>` — `Rc<BTreeMap<K, V>>` wrapper with `new`, `get`, `insert`, `remove`, `entry`, `iter`, `keys`, `is_empty`, `contains_key`, `len`, `Clone` (O(1) via Rc).
- `SharedSet<K>` — `Rc<BTreeSet<K>>` wrapper with `new`, `contains`, `insert`, `is_empty`, `Clone` (O(1) via Rc).
- `SharedVec<T>` — `Rc<Vec<T>>` wrapper with `new`, `push`, `as_slice`, `len`, `is_empty`, `iter`, `into_vec`, `Clone` (O(1) via Rc).

All three types derive `Clone` (the derive invokes `Rc::clone`, which is O(1)). All three are `pub(crate)` — not exported outside `worldwake-ai`.

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
9. `SharedVec`: push + as_slice works.
10. `SharedVec`: clone sharing + mutation independence.
11. `SharedVec`: `into_vec` returns owned Vec with correct contents.
12. Existing suite: `cargo test -p worldwake-ai` (no regressions).
13. Existing suite: `cargo clippy --workspace` (no new warnings).

### Invariants

1. `SharedMap::get` returns identical results to `BTreeMap::get` for any key.
2. Iteration order of `SharedMap` is identical to `BTreeMap` (BTree ordering preserved).
3. `SharedSet::contains` returns identical results to `BTreeSet::contains`.
4. `SharedVec` maintains insertion order identical to `Vec`.
5. Cloning any shared wrapper is O(1) (reference count increment only).

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/shared_collections.rs` (inline `#[cfg(test)] mod tests`) — comprehensive unit tests for all three types covering construction, mutation, clone sharing (Rc::ptr_eq), mutation independence, iteration order, and edge cases (empty collections, single-element, entry API).

### Commands

1. `cargo test -p worldwake-ai shared_collections` (targeted)
2. `cargo clippy --workspace && cargo test --workspace` (full verification)
