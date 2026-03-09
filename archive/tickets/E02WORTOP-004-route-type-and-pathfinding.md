# E02WORTOP-004: Route Type and Deterministic Pathfinding

**Status**: ✅ COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: None
**Deps**: E02WORTOP-003 (Topology storage and queries)

## Problem

Agents need shortest-path routing through the world graph. The pathfinding must be fully deterministic: same graph + same query = same route, even when multiple equal-cost paths exist. A `Route` value type captures the full path (places + edges + total cost).

## Assumption Reassessment (2026-03-09)

1. `Topology` already exists in `crates/worldwake-core/src/topology.rs` from archived ticket E02WORTOP-003, with private `places`, `edges`, `outgoing`, and `incoming` `BTreeMap`s plus deterministic query APIs — confirmed.
2. `TravelEdge` already exists and stores `travel_time_ticks` internally as `NonZeroU32`, exposing it through `travel_time_ticks() -> u32`. The spec-level type is still effectively `u32 >= 1`, but the code already enforces the stronger invariant.
3. `worldwake-core` already has policy tests in `crates/worldwake-core/tests/policy.rs` that reject `HashMap` and `HashSet`, so pathfinding must stay within deterministic ordered collections and queue semantics.
4. Spec `specs/E02-world-topology.corrected.md` requires: if multiple routes have equal total travel time, choose the one with the lexicographically smallest `edges` sequence. This is the canonical tie-break rule.
5. The existing topology API uses private fields plus validating mutation methods. Pathfinding should extend that architecture cleanly instead of introducing aliasing, exposing internals, or relying on ad hoc graph state outside `Topology`.

## Architecture Check

1. `Route` should be a first-class value in `topology.rs`: `places: Vec<EntityId>`, `edges: Vec<TravelEdgeId>`, `total_travel_time: u32`, derived for clone/debug/equality plus serde round-trip. This is cleaner than returning only place IDs and is the right foundation for later movement, occupancy, and event provenance.
2. `shortest_path` belongs on `Topology`. Keeping routing logic co-located with authoritative graph queries is materially cleaner than creating a separate helper that would mirror topology internals or duplicate invariants.
3. Use deterministic Dijkstra, but do not overfit the queue design. A `BinaryHeap` with an explicit deterministic ordering is fine, provided stale entries are discarded and all authoritative best-path state lives in ordered maps. The queue does not need to carry entire path vectors if the implementation can compare and update best-known routes deterministically.
4. On relaxation, compare candidate routes by `(total_travel_time, edges sequence)` in that order. Lower total cost wins; on equal cost, lexicographically smaller `edges` wins. This should be encoded explicitly in a helper rather than spread across inline conditionals.
5. For this graph size and Phase 1 scope, it is acceptable for the best-known state per destination to retain the current best edge sequence and place sequence if that makes the implementation clearer and easier to validate. Favor correctness and explicit determinism over a prematurely optimized predecessor-only design.
6. Returns `Option<Route>` — `None` for disconnected nodes or for any query whose endpoints do not exist. `shortest_path(x, x)` should return the zero-cost degenerate route when `x` exists as a place.

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

Deterministic Dijkstra requirements:
- Maintain best-known route state in `BTreeMap`s keyed by destination place.
- Use a deterministic queue strategy (`BinaryHeap` with explicit `Ord`, or ordered-map frontier) that does not depend on hash iteration or pointer identity.
- When equal-cost paths exist, compare edge sequences lexicographically and keep only the canonical winner.
- Keep the path-comparison logic explicit and unit-tested.
- No `HashMap` or `HashSet` in the implementation.

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
- Refactoring `Topology` storage or `TravelEdge` representation beyond what pathfinding strictly needs. The existing private-field, validating-API architecture from E02WORTOP-003 is the correct base to extend here.

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
9. Missing endpoints return `None` rather than panicking or synthesizing partial routes.
10. No `HashMap` used internally (verified by existing policy test and code review).
11. Existing suite: `cargo test -p worldwake-core`.

### Invariants

1. Determinism: same graph + same query = identical `Route` (byte-equal after serialization).
2. No hash-dependent ordering in priority queue or visited set.
3. Tie-break rule: lexicographically smallest edge sequence wins among equal-cost paths.
4. `Route.total_travel_time` == sum of `travel_time_ticks` for all edges in `Route.edges`.
5. The `places` and `edges` sequences in `Route` stay aligned: `places.len() == edges.len() + 1` for non-empty routes.
6. `shortest_path` does not expose or mutate topology internals.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/topology.rs` — pathfinding unit tests: linear, diamond, disconnected, self-path, complex multi-hop, tie-break verification.

### Commands

1. `cargo test -p worldwake-core shortest_path`
2. `cargo test -p worldwake-core route`
3. `cargo test -p worldwake-core`
4. `cargo clippy --workspace --all-targets -- -D warnings`
5. `cargo test --workspace`

## Outcome

- Completion date: 2026-03-09
- What actually changed:
  - Added `Route` as a first-class serializable topology value in `crates/worldwake-core/src/topology.rs`.
  - Implemented `Topology::shortest_path(from, to) -> Option<Route>` with deterministic Dijkstra semantics and explicit equal-cost tie-breaking on lexicographically ordered `TravelEdgeId` sequences.
  - Re-exported `Route` from `crates/worldwake-core/src/lib.rs`.
  - Added focused topology tests for route serialization, zero-cost self-routes, missing/disconnected endpoints, single-edge routing, multi-hop cost accumulation, globally shortest selection, equal-cost tie-break behavior, and route shape invariants.
- Deviations from original plan:
  - The original ticket assumed `Topology` still needed to be introduced and described `TravelEdge.travel_time_ticks` as a raw stored `u32`. The ticket was corrected first to match the actual codebase, where `Topology` already exists and `TravelEdge` stores travel time as `NonZeroU32`.
  - The original queue sketch suggested carrying full edge paths directly inside the priority key. The final implementation keeps canonical best-route state in ordered maps and uses the heap only as a deterministic frontier, which is cleaner and easier to validate against the current architecture.
- Verification results:
  - `cargo test -p worldwake-core shortest_path` passed.
  - `cargo test -p worldwake-core route` passed.
  - `cargo test -p worldwake-core` passed.
  - `cargo clippy --workspace --all-targets -- -D warnings` passed.
  - `cargo test --workspace` passed.
