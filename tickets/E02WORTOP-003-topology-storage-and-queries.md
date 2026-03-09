# E02WORTOP-003: Topology Storage and Query APIs

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: None
**Deps**: E02WORTOP-001 (PlaceTag, Place), E02WORTOP-002 (TravelEdgeId, TravelEdge)

## Problem

The world topology needs an ordered graph storage model with deterministic query APIs. This is the central data structure that holds places and directed travel edges, with efficient neighbor lookups.

## Assumption Reassessment (2026-03-09)

1. `Place` and `PlaceTag` will exist from E02WORTOP-001 — dependency.
2. `TravelEdgeId` and `TravelEdge` will exist from E02WORTOP-002 — dependency.
3. `BTreeMap` is the required ordered map per the deterministic data policy — confirmed in `lib.rs`.
4. Spec requires outgoing/incoming edge lists in stable sorted order — `Vec<TravelEdgeId>` sorted after insertion.

## Architecture Check

1. `Topology` struct uses four `BTreeMap`s as specified: `places`, `edges`, `outgoing`, `incoming`. This gives O(log n) lookups and deterministic iteration.
2. Edge lists (`outgoing`/`incoming`) store `Vec<TravelEdgeId>` sorted by `TravelEdgeId` value, ensuring deterministic query results.
3. Builder methods (`add_place`, `add_edge`) maintain sorted invariants on insertion.
4. Read-only query methods return references or small `Vec`s — no mutation through queries.

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
- `add_place(id: EntityId, place: Place)` — insert a place.
- `add_edge(edge: TravelEdge)` — insert an edge, update outgoing/incoming lists, maintain sorted order.

### 3. Query methods (per spec)

- `place(&self, id: EntityId) -> Option<&Place>`
- `edge(&self, id: TravelEdgeId) -> Option<&TravelEdge>`
- `outgoing_edges(&self, place: EntityId) -> &[TravelEdgeId]`
- `incoming_edges(&self, place: EntityId) -> &[TravelEdgeId]`
- `neighbors(&self, place: EntityId) -> Vec<EntityId>` — distinct destination EntityIds from outgoing edges, sorted.
- `is_reachable(&self, from: EntityId, to: EntityId) -> bool` — BFS/DFS reachability check.
- `place_count(&self) -> usize`
- `edge_count(&self) -> usize`

### 4. Re-export `Topology` from `lib.rs`

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
2. `add_edge` followed by `edge(id)` returns the inserted edge.
3. `outgoing_edges` returns edge IDs in sorted order.
4. `incoming_edges` returns edge IDs in sorted order.
5. `neighbors` returns sorted, deduplicated destination EntityIds.
6. `is_reachable` returns `true` for connected nodes in a test graph.
7. `is_reachable` returns `false` for disconnected nodes.
8. Graph traversal (via `is_reachable` or neighbors iteration) visits all reachable nodes — spec test: "Graph traversal visits all reachable nodes."
9. Empty topology queries return `None` / empty slices gracefully.
10. Existing suite: `cargo test -p worldwake-core`.

### Invariants

1. All `BTreeMap` keys are ordered — deterministic iteration.
2. Outgoing/incoming edge lists are always sorted by `TravelEdgeId`.
3. `neighbors()` output is deterministic (sorted `EntityId`s).
4. No `HashMap` or `HashSet` anywhere in `Topology`.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/topology.rs` — unit tests for all query methods, edge cases (empty graph, single node, disconnected subgraphs).

### Commands

1. `cargo test -p worldwake-core topology`
2. `cargo clippy --workspace && cargo test --workspace`
