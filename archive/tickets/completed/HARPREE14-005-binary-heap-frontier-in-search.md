# HARPREE14-005: Replace Vec-sort frontier with BinaryHeap in search

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes -- search frontier data structure change
**Deps**: None (Wave 1, independent). Should be implemented BEFORE or AFTER HARPREE14-004 (same file, no logical dep)
**Spec Reference**: HARDENING-PRE-E14.md, HARDEN-C01

## Problem

`pop_next_node()` in `crates/worldwake-ai/src/search.rs` sorts the entire `Vec<SearchNode>` on every pop, then removes index 0. That makes each frontier pop O(n log n) for the sort plus O(n) for the front removal. With larger frontier sizes this becomes a bottleneck. A heap-backed frontier reduces push/pop overhead to O(log n) while preserving the planner's current best-first behavior.

## Assumption Reassessment (2026-03-11)

1. `pop_next_node()` exists, but currently lives around lines 161-168, not 145-153. It calls `frontier.sort_by(compare_search_nodes)` and then `frontier.remove(0)`, not `frontier.pop()`.
2. `compare_search_nodes()` compares by `total_estimated_ticks`, then `steps.len()`, then `steps` lexicographically -- confirmed.
3. The sort happens on every pop call -- confirmed.
4. Beam pruning does not operate on the frontier. It sorts a separate `successors` vector and truncates that vector before pushing into the frontier. This ticket does not need a beam-pruning redesign.
5. `search.rs` already contains focused unit tests for plan depth, node budget, beam width, combat commitment, travel/consume/trade behavior, and failure paths. The ticket must preserve and extend this coverage rather than relying only on existing tests.

## Architecture Check

1. Replacing the frontier `Vec` with a heap is beneficial. The current architecture pays sorting cost on every expansion for a purely priority-queue concern.
2. The clean implementation is not to make `SearchNode` itself the ordered type. `SearchNode` contains `PlanningState`, and ordering a full node while ignoring planner state would be a misleading abstraction.
3. Use a dedicated frontier entry / priority wrapper that owns the `SearchNode` and defines ordering from the existing search priority tuple: `total_estimated_ticks`, `steps.len()`, then lexicographic `steps`.
4. The ordering logic used by the heap and the successor sort should come from the same comparison path so there is one authoritative definition of search priority.
5. No backwards-compatibility shims.

## What to Change

### 1. Implement `Ord` for `SearchNode`

Replace this with an ordered frontier entry type. Implement `PartialOrd`, `Ord`, `PartialEq`, and `Eq` on the heap entry (or a separate priority key) using the existing priority logic: first `total_estimated_ticks` ascending, then `steps.len()` ascending, then lexicographic `steps`.

### 2. Replace `Vec<SearchNode>` with `BinaryHeap<Reverse<SearchNode>>`

Change the frontier data structure to a heap-backed priority queue. `BinaryHeap<Reverse<FrontierEntry>>` is acceptable, but a custom `Ord` that directly expresses "best node first" is also acceptable if it keeps the ordering logic explicit and shared.

### 3. Remove `pop_next_node()` and `compare_search_nodes()`

Remove `pop_next_node()` and replace it with direct heap `pop()` calls. `compare_search_nodes()` may be removed if the shared priority comparison helper fully replaces it, but keeping a small shared comparison helper is acceptable if it remains the single source of truth for both heap ordering and successor sorting.

### 4. Adapt beam pruning

No beam-pruning redesign is required. Successor ranking should continue to sort/truncate the temporary `successors` vector before frontier insertion.

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

1. All existing `search.rs` unit tests pass with identical behavior.
2. Add at least one targeted search test covering heap ordering tie-breaks so the frontier ordering invariant is explicit in tests.
3. Golden e2e hashes remain identical.
4. `cargo test --workspace` passes.
5. `cargo clippy --workspace` passes with no new warnings.

### Invariants

1. **Determinism**: Node selection order is identical to the Vec-sort approach for all inputs
2. Search results are unchanged for all existing test scenarios
3. Golden e2e state hashes identical

## Test Plan

### New/Modified Tests

1. Add a targeted unit test in `crates/worldwake-ai/src/search.rs` that proves frontier ordering remains deterministic across tie-break cases:
   - lower `total_estimated_ticks` wins first
   - if equal, shorter `steps.len()` wins
   - if equal again, lexicographic `steps` order wins
2. Existing search unit tests remain required because they exercise beam width, node budget, duration failure, and terminal behavior.
3. Golden e2e remains required because this ticket changes planner internals and must preserve deterministic hashes.

### Commands

1. `cargo test -p worldwake-ai search` (targeted)
2. `cargo test -p worldwake-ai --test golden_e2e` (determinism check)
3. `cargo test --workspace`
4. `cargo clippy --workspace`

## Outcome

1. Replaced the frontier `Vec` sort/remove path with a heap-backed frontier in `crates/worldwake-ai/src/search.rs`.
2. Implemented the ordering on a dedicated frontier entry wrapper, not on `SearchNode` itself. This keeps planner state out of ordering semantics and is cleaner than the original proposed `Ord`-on-`SearchNode` approach.
3. Kept beam pruning unchanged because it already operates on the temporary successor list rather than on the frontier.
4. Added a targeted unit test covering frontier priority tie-breaks in addition to preserving the existing search and golden coverage.
5. Verified with:
   - `cargo test -p worldwake-ai search`
   - `cargo test -p worldwake-ai --test golden_e2e`
   - `cargo test --workspace`
   - `cargo clippy --workspace -- -D warnings`
