# E02WORTOP-003: Topology Storage and Query APIs

**Status**: ✅ COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: None
**Deps**: E02WORTOP-001 (PlaceTag, Place), E02WORTOP-002 (TravelEdgeId, TravelEdge)

## Problem

The world topology needs an ordered graph storage model with deterministic query APIs. This is the central data structure that holds places and directed travel edges, with efficient neighbor lookups.

## Assumption Reassessment (2026-03-09)

1. `Place` and `PlaceTag` already exist in `crates/worldwake-core/src/topology.rs` from archived ticket E02WORTOP-001 — confirmed.
2. `TravelEdgeId` already exists in `crates/worldwake-core/src/ids.rs`, and `TravelEdge` already exists in `crates/worldwake-core/src/topology.rs` from archived ticket E02WORTOP-002 — confirmed.
3. `BTreeMap` is the required ordered map per the deterministic data policy in `crates/worldwake-core/src/lib.rs` — confirmed.
4. `worldwake-core` already has policy tests in `crates/worldwake-core/tests/policy.rs`, so `Topology` must avoid `HashMap`/`HashSet` and any nondeterministic iteration.
5. `TravelEdge` already uses private fields plus accessors and stores travel time as `NonZeroU32` internally. `Topology` should follow the same architecture and guard graph invariants at mutation boundaries instead of exposing raw mutable state.
6. The original ticket underspecified graph legality. A robust topology store must reject duplicate place IDs, duplicate edge IDs, and edges whose endpoints do not exist, rather than silently accepting structurally invalid graphs.

## Architecture Check

1. `Topology` struct uses four `BTreeMap`s as specified: `places`, `edges`, `outgoing`, `incoming`. This gives O(log n) lookups and deterministic iteration.
2. Edge lists (`outgoing`/`incoming`) store `Vec<TravelEdgeId>` sorted by `TravelEdgeId` value, ensuring deterministic query results.
3. Mutation methods must be the only place that can change adjacency state. They should maintain sorted invariants on insertion and reject illegal graph states up front.
4. `Topology` fields should stay private. Read-only query methods return references, slices, or deterministic `Vec`s without exposing mutation through queries.
5. `add_place` and `add_edge` should return `Result<(), WorldError>` rather than silently overwriting or accepting invalid input. This is materially cleaner than a permissive bag-of-maps API and better matches the crate's Phase 1 legality model.
6. Reachability can remain a simple deterministic graph traversal using `BTreeSet` for visited nodes. Pathfinding and route construction still belong to E02WORTOP-004.

## What to Change

### 1. Add `Topology` struct to `topology.rs`

```rust
pub struct Topology {
    places: BTreeMap<EntityId, Place>,
    edges: BTreeMap<TravelEdgeId, TravelEdge>,
    outgoing: BTreeMap<EntityId, Vec<TravelEdgeId>>,
    incoming: BTreeMap<EntityId, Vec<TravelEdgeId>>,
}
```

### 2. Builder methods

- `new() -> Self` — empty topology.
- `add_place(id: EntityId, place: Place) -> Result<(), WorldError>` — insert a place, rejecting duplicate place IDs.
- `add_edge(edge: TravelEdge) -> Result<(), WorldError>` — insert an edge, rejecting duplicate edge IDs and edges whose `from`/`to` places do not already exist; update outgoing/incoming lists and maintain sorted order.

### 3. Query methods (per spec)

- `place(&self, id: EntityId) -> Option<&Place>`
- `edge(&self, id: TravelEdgeId) -> Option<&TravelEdge>`
- `outgoing_edges(&self, place: EntityId) -> &[TravelEdgeId]`
- `incoming_edges(&self, place: EntityId) -> &[TravelEdgeId]`
- `neighbors(&self, place: EntityId) -> Vec<EntityId>` — distinct destination EntityIds from outgoing edges, sorted.
- `is_reachable(&self, from: EntityId, to: EntityId) -> bool` — BFS/DFS reachability check.
- `place_count(&self) -> usize`
- `edge_count(&self) -> usize`

### 4. Graph legality behavior

- Missing places must not require callers to special-case adjacency maps; `outgoing_edges` and `incoming_edges` should return an empty slice when the place has no entries.
- `neighbors` must be sorted and deduplicated even if multiple edges target the same destination.
- Duplicate place IDs and duplicate edge IDs must return `WorldError::InvalidOperation(...)`.
- `add_edge` must return `WorldError::EntityNotFound(...)` if either endpoint place is absent.

### 5. Re-export `Topology` from `lib.rs`

## Files to Touch

- `crates/worldwake-core/src/topology.rs` (modify — add `Topology` struct, builder and query methods)
- `crates/worldwake-core/src/lib.rs` (modify — add `Topology` re-export)

## Out of Scope

- Pathfinding / shortest path (Dijkstra) — E02WORTOP-004.
- `Route` type — E02WORTOP-004.
- World builder (prototype place graph) — E02WORTOP-005.
- Serialization round-trip of `Topology` — E02WORTOP-006.
- Edge mutation after insertion (not needed in Phase 1).
- Place removal (not needed in Phase 1).

## Acceptance Criteria

### Tests That Must Pass

1. `add_place` followed by `place(id)` returns the inserted place.
2. `add_place` rejects duplicate place IDs.
3. `add_edge` followed by `edge(id)` returns the inserted edge.
4. `add_edge` rejects duplicate edge IDs.
5. `add_edge` rejects edges whose `from` or `to` place is missing.
6. `outgoing_edges` returns edge IDs in sorted order.
7. `incoming_edges` returns edge IDs in sorted order.
8. `neighbors` returns sorted, deduplicated destination `EntityId`s.
9. `is_reachable` returns `true` for connected nodes in a test graph.
10. `is_reachable` returns `false` for disconnected nodes.
11. Graph traversal (via `is_reachable` or neighbors iteration) visits all reachable nodes — spec test: "Graph traversal visits all reachable nodes."
12. Empty topology queries return `None` / empty slices gracefully.
13. Existing suite: `cargo test -p worldwake-core`.

### Invariants

1. All `BTreeMap` keys are ordered — deterministic iteration.
2. Outgoing/incoming edge lists are always sorted by `TravelEdgeId`.
3. `neighbors()` output is deterministic (sorted `EntityId`s).
4. No `HashMap` or `HashSet` anywhere in `Topology`.
5. `Topology` never stores an edge whose endpoints are absent from `places`.
6. `Topology` never silently overwrites an existing place or edge entry.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/topology.rs` — unit tests for all query methods, edge cases (empty graph, single node, disconnected subgraphs).

### Commands

1. `cargo test -p worldwake-core topology`
2. `cargo test -p worldwake-core`
3. `cargo clippy --workspace --all-targets -- -D warnings`
4. `cargo test --workspace`

## Outcome

- Completion date: 2026-03-09
- What actually changed:
  - Added `Topology` to `crates/worldwake-core/src/topology.rs` using ordered `BTreeMap` storage for places, edges, outgoing adjacency, and incoming adjacency.
  - Implemented deterministic query APIs for place lookup, edge lookup, outgoing/incoming adjacency, neighbor discovery, reachability, and counts.
  - Strengthened the original plan by making `add_place` and `add_edge` validating mutation boundaries that reject duplicate IDs and dangling edge endpoints with `WorldError`.
  - Re-exported `Topology` from `crates/worldwake-core/src/lib.rs`.
  - Added topology tests covering deterministic adjacency ordering, deduplicated neighbors, reachability, duplicate rejection, missing-endpoint rejection, and empty-query behavior.
- Deviations from original plan:
  - The original ticket treated `Topology` mostly as storage plus queries. That was corrected before implementation because a passive map bag would permit invalid graph state over time.
  - The final API uses private fields and `Result`-returning mutation methods so legality is enforced when the graph is built, which is cleaner and more extensible than accepting duplicates or dangling references and hoping later code compensates.
- Verification results:
  - `cargo test -p worldwake-core topology` passed.
  - `cargo test -p worldwake-core` passed.
  - `cargo clippy --workspace --all-targets -- -D warnings` passed.
  - `cargo test --workspace` passed.
