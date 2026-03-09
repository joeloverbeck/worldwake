# E02WORTOP-004: Route Type and Deterministic Pathfinding

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: None
**Deps**: E02WORTOP-003 (Topology storage and queries)

## Problem

Agents need shortest-path routing through the world graph. The pathfinding must be fully deterministic: same graph + same query = same route, even when multiple equal-cost paths exist. A `Route` value type captures the full path (places + edges + total cost).

## Assumption Reassessment (2026-03-09)

1. `Topology` struct with `places`, `edges`, `outgoing` BTreeMaps will exist from E02WORTOP-003 — dependency.
2. `TravelEdge.travel_time_ticks` is `u32` — confirmed in spec.
3. Spec requires: "if multiple routes have equal total travel time, choose the one with the lexicographically smallest `edges` sequence" — this is the tie-break rule.
4. Priority queue must not depend on hash iteration or pointer identity — use `BinaryHeap` with a deterministic `Ord` impl or a `BTreeMap`-based queue.

## Architecture Check

1. `Route` is a plain data struct: `places: Vec<EntityId>`, `edges: Vec<TravelEdgeId>`, `total_travel_time: u32`. Serializable for replay/save.
2. Dijkstra using `BinaryHeap<Reverse<(cost, EntityId, edge_path)>>` or equivalent. The key insight for determinism: when costs are equal, compare by `EntityId` (which is `Ord`), and track the edge sequence to apply the lexicographic tie-break.
3. Implementation approach: standard Dijkstra, but when relaxing edges, if `new_cost == existing_cost`, compare the candidate edge sequence lexicographically against the stored one. Only replace if strictly smaller.
4. Returns `Option<Route>` — `None` for disconnected nodes.

## What to Change

### 1. Add `Route` struct to `topology.rs`

```rust
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Route {
    pub places: Vec<EntityId>,
    pub edges: Vec<TravelEdgeId>,
    pub total_travel_time: u32,
}
```

### 2. Add `shortest_path` method to `Topology`

```rust
impl Topology {
    pub fn shortest_path(&self, from: EntityId, to: EntityId) -> Option<Route> { ... }
}
```

Dijkstra with deterministic tie-breaking:
- Priority: `(cost, EntityId)` — lower cost first, then lower EntityId.
- When equal cost paths exist, compare edge sequences lexicographically.
- No `HashMap` in the implementation — use `BTreeMap` for visited/cost tracking.

### 3. Re-export `Route` from `lib.rs`

## Files to Touch

- `crates/worldwake-core/src/topology.rs` (modify — add `Route`, `shortest_path`)
- `crates/worldwake-core/src/lib.rs` (modify — add `Route` re-export)

## Out of Scope

- A* heuristic optimization — Dijkstra is sufficient for the prototype graph size (12-20 nodes).
- Danger-weighted or visibility-weighted routing — Phase 1 uses `travel_time_ticks` only.
- Route caching or memoization.
- World builder — E02WORTOP-005.
- Multi-destination routing.

## Acceptance Criteria

### Tests That Must Pass

1. Shortest path on a simple linear graph (A→B→C) returns correct route with correct `total_travel_time`.
2. Path costs equal the sum of edge travel times — spec test.
3. Pathfinding returns `None` for disconnected nodes — spec test.
4. Equal-cost shortest paths resolve with lexicographically smallest edge sequence — spec test. Build a diamond graph (A→B→D, A→C→D with equal costs) where the tie-break is exercised.
5. `shortest_path(x, x)` returns a zero-cost route with just the origin place and no edges.
6. `Route` bincode round-trip.
7. Pathfinding on a single-edge graph works correctly.
8. Pathfinding with multiple hops selects the globally shortest path, not just greedy first edge.
9. No `HashMap` used internally (verified by code review / grep).
10. Existing suite: `cargo test -p worldwake-core`.

### Invariants

1. Determinism: same graph + same query = identical `Route` (byte-equal after serialization).
2. No hash-dependent ordering in priority queue or visited set.
3. Tie-break rule: lexicographically smallest edge sequence wins among equal-cost paths.
4. `Route.total_travel_time` == sum of `travel_time_ticks` for all edges in `Route.edges`.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/topology.rs` — pathfinding unit tests: linear, diamond, disconnected, self-path, complex multi-hop, tie-break verification.

### Commands

1. `cargo test -p worldwake-core shortest_path`
2. `cargo test -p worldwake-core route`
3. `cargo clippy --workspace && cargo test --workspace`
