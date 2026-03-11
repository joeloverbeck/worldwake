# HARPREE14-005: Replace Vec-sort frontier with BinaryHeap in search

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes -- search frontier data structure change
**Deps**: None (Wave 1, independent). Should be implemented BEFORE or AFTER HARPREE14-004 (same file, no logical dep)
**Spec Reference**: HARDENING-PRE-E14.md, HARDEN-C01

## Problem

`pop_next_node()` (lines 145-153 of `search.rs`) sorts the entire `Vec<SearchNode>` on every pop -- O(n log n) per expansion. With larger frontier sizes this becomes a bottleneck. A `BinaryHeap` gives O(log n) per push and O(log n) per pop.

## Assumption Reassessment (2026-03-11)

1. `pop_next_node()` exists at line 145, calls `frontier.sort_by(compare_search_nodes)` then `frontier.pop()` -- confirmed
2. `compare_search_nodes()` compares by `total_estimated_ticks`, then `steps.len()`, then steps lexicographic -- confirmed
3. The sort happens on every pop call -- confirmed (O(n log n) per expansion)

## Architecture Check

1. `BinaryHeap<Reverse<SearchNode>>` with a proper `Ord` impl is the standard Rust pattern for priority queues.
2. The `Ord` implementation must produce the EXACT same node selection order as `compare_search_nodes` to preserve determinism. This is the critical correctness requirement.
3. No backwards-compatibility shims.

## What to Change

### 1. Implement `Ord` for `SearchNode`

Implement `PartialOrd`, `Ord`, `PartialEq`, `Eq` for `SearchNode` matching the existing `compare_search_nodes` logic: first by `total_estimated_ticks` (ascending), then `steps.len()` (ascending), then lexicographic comparison of steps.

### 2. Replace `Vec<SearchNode>` with `BinaryHeap<Reverse<SearchNode>>`

Change the frontier data structure. Use `std::cmp::Reverse` to turn the max-heap into a min-heap (lowest cost first).

### 3. Remove `pop_next_node()` and `compare_search_nodes()`

Replace with direct `frontier.pop()` calls. The comparison logic lives in the `Ord` impl now.

### 4. Adapt beam pruning

If beam pruning currently relies on Vec sorting, adapt it to work with the BinaryHeap (drain and re-collect if needed, or use a Vec intermediary for the pruning step only).

## Files to Touch

- `crates/worldwake-ai/src/search.rs` (modify)

## Out of Scope

- Changing search algorithm logic (expansion strategy, goal checking)
- Modifying `PlanningBudget` parameters
- Changing `SearchNode` fields
- Changing `PlannedStep` or `GoalSemantics` (those are separate tickets)
- Any changes outside search.rs

## Acceptance Criteria

### Tests That Must Pass

1. All existing search tests pass with identical results
2. Golden e2e hashes identical (determinism preserved)
3. `cargo test --workspace` passes
4. `cargo clippy --workspace` -- no new warnings

### Invariants

1. **Determinism**: Node selection order is identical to the Vec-sort approach for all inputs
2. Search results are unchanged for all existing test scenarios
3. Golden e2e state hashes identical

## Test Plan

### New/Modified Tests

1. No new tests needed -- existing tests validate identical behavior. If desired, add a targeted test that the Ord impl matches compare_search_nodes for edge cases (equal costs, different depths).

### Commands

1. `cargo test -p worldwake-ai search` (targeted)
2. `cargo test -p worldwake-ai --test golden_e2e` (determinism check)
3. `cargo test --workspace`
4. `cargo clippy --workspace`
