# S09TRAAWAPLASEA-001: Add Floyd-Warshall distance matrix to PlanningSnapshot

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — new field and methods on `PlanningSnapshot`
**Deps**: None (foundational ticket for S09)

## Problem

The GOAP plan search has no spatial awareness. To introduce an A* heuristic (tickets 003–004), the planner needs precomputed all-pairs shortest travel times between snapshot places. This ticket adds the distance matrix to `PlanningSnapshot` so subsequent tickets can query minimum travel distances.

## Assumption Reassessment (2026-03-17)

1. `PlanningSnapshot` is defined in `crates/worldwake-ai/src/planning_snapshot.rs:113-122` with fields `actor`, `entities`, `places`, `blocked_facility_uses`, `actor_known_entity_beliefs`, `actor_support_declarations`, `actor_confidence_policy`, `actor_tell_profile` — confirmed.
2. `SnapshotPlace` has `adjacent_places_with_travel_ticks: Vec<(EntityId, NonZeroU32)>` at line 110 — confirmed. This provides the edge weights for Floyd-Warshall initialization.
3. `PlanningSnapshot::build_with_blocked_facility_uses()` (lines 144-187) is the primary constructor; `build()` delegates to it — confirmed. The distance matrix must be computed at the end of this constructor.
4. `PlanningSnapshot.places` is `BTreeMap<EntityId, SnapshotPlace>` — confirmed. Typically 10-20 entries, so Floyd-Warshall (O(n^3)) is negligible.
5. The spec requires `BTreeMap<(EntityId, EntityId), u32>` for deterministic iteration — confirmed in spec Component 1.

## Architecture Check

1. Floyd-Warshall is the right algorithm: the snapshot already has all places and edges, and we need all-pairs distances (not single-source). For n≤20 places, O(n^3) < 8000 ops is negligible.
2. Storing as `BTreeMap<(EntityId, EntityId), u32>` (sparse) is correct — no need for a dense matrix with EntityId keys.

## What to Change

### 1. Add `shortest_travel_ticks` field to `PlanningSnapshot`

Add a new field `shortest_travel_ticks: BTreeMap<(EntityId, EntityId), u32>` to the `PlanningSnapshot` struct.

### 2. Implement Floyd-Warshall in snapshot construction

At the end of `build_with_blocked_facility_uses()`, after the `places` BTreeMap is populated:

1. Initialize the distance matrix from `adjacent_places_with_travel_ticks` for each place.
2. Run Floyd-Warshall: for each intermediate place k, for each pair (i, j), relax `dist[i][j] = min(dist[i][j], dist[i][k] + dist[k][j])`.
3. Store the result in `shortest_travel_ticks`.

Extract the Floyd-Warshall computation into a private helper function `compute_shortest_travel_ticks(places: &BTreeMap<EntityId, SnapshotPlace>) -> BTreeMap<(EntityId, EntityId), u32>` for clarity.

### 3. Add query methods

Add two public methods to `PlanningSnapshot`:

- `pub fn min_travel_ticks(&self, from: EntityId, to: EntityId) -> Option<u32>` — returns `Some(0)` if `from == to`, otherwise looks up `shortest_travel_ticks`.
- `pub fn min_travel_ticks_to_any(&self, from: EntityId, destinations: &[EntityId]) -> Option<u32>` — returns `Some(0)` if `from` is in `destinations`, otherwise returns the minimum distance to any destination.

## Files to Touch

- `crates/worldwake-ai/src/planning_snapshot.rs` (modify — add field, compute in constructor, add methods)

## Out of Scope

- Modifying `search.rs` (ticket 003)
- Adding `goal_relevant_places` to `GoalKindPlannerExt` (ticket 002)
- Travel pruning logic (ticket 004)
- Any changes to `worldwake-core` topology (the spec notes this as optional; Floyd-Warshall on the snapshot is sufficient)
- Any changes to `PlanningState`, `PlannerOpSemantics`, or any other AI module
- Modifying golden tests or test budgets

## Acceptance Criteria

### Tests That Must Pass

1. Unit test: `min_travel_ticks(place_a, place_a)` returns `Some(0)` for any place in the snapshot.
2. Unit test: `min_travel_ticks(place_a, place_b)` returns the correct shortest-path distance for directly adjacent places (equals edge weight).
3. Unit test: `min_travel_ticks(place_a, place_c)` returns the correct multi-hop shortest-path distance (e.g., on the prototype world, GeneralStore to OrchardFarm via VillageSquare→SouthGate→EastFieldTrail should equal the sum of those edge weights).
4. Unit test: `min_travel_ticks(place_a, unreachable_place)` returns `None` for a place not in the snapshot.
5. Unit test: `min_travel_ticks_to_any(place, &[dest1, dest2])` returns the minimum of the distances to dest1 and dest2.
6. Unit test: `min_travel_ticks_to_any(place, &[place])` returns `Some(0)`.
7. Unit test: `min_travel_ticks_to_any(place, &[])` returns `None`.
8. Determinism test: Two `PlanningSnapshot` instances built from the same world state produce identical `shortest_travel_ticks` maps.
9. Existing suite: `cargo test -p worldwake-ai`
10. `cargo clippy --workspace`

### Invariants

1. `shortest_travel_ticks` uses `BTreeMap` (not `HashMap`) for deterministic iteration.
2. The distance matrix is immutable after construction — no mutation methods exposed.
3. All existing tests pass unchanged — this ticket adds a new field but does not change search behavior.
4. Floyd-Warshall initialization uses `NonZeroU32::get()` to convert edge weights to `u32`.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/planning_snapshot.rs` (or a dedicated test module) — unit tests for Floyd-Warshall correctness, query methods, edge cases (empty destinations, self-distance, unreachable places)

### Commands

1. `cargo test -p worldwake-ai planning_snapshot`
2. `cargo test --workspace && cargo clippy --workspace`
