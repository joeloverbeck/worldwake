# E02WORTOP-005: Prototype World Builder

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: None
**Deps**: E02WORTOP-003 (Topology storage), E02WORTOP-004 (Route/pathfinding for connectivity validation)

## Problem

The simulation needs a fixed prototype world graph matching spec 4.1. A builder function must produce a deterministic, connected micro-world with 12-20 place nodes, stable IDs, and intentional connectivity for the playable scenario.

## Assumption Reassessment (2026-03-09)

1. `Topology`, `Place`, `PlaceTag`, `TravelEdge`, `TravelEdgeId` all exist from prior tickets — dependency.
2. Spec 4.1 requires specific place types: village core, farm, store, inn, hall, barracks, latrine, crossroads, forest route, bandit camp — confirmed.
3. "12-20 total place nodes" — the 10 listed above are mandatory; 2-10 additional connective nodes (roads, trails, fields, gates) fill the range.
4. Builder output must be stable: same code path => same place IDs, edge IDs, topology. No randomness in the builder.
5. Every place must have at least one incoming and one outgoing edge — spec requirement.

## Architecture Check

1. A `build_prototype_world() -> Topology` function in a dedicated submodule (e.g., `topology/builder.rs` or a `builder` function within `topology.rs`).
2. Place IDs are assigned from a fixed sequence (slot 0, 1, 2, ... with generation 0). Edge IDs similarly sequential.
3. The graph is hand-designed to be intentionally connected and navigable — not randomly generated.
4. All `danger` and `visibility` values are fixed `Permille` constants — no randomness.
5. The function is pure: no external state, no RNG, no I/O.

## What to Change

### 1. Add `build_prototype_world()` function

Located in `topology.rs` (or a `builder` submodule if the file grows too large).

Creates the following places (minimum, may add connective nodes):

| Slot | Name | Tags |
|------|------|------|
| 0 | "Village Square" | Village, Crossroads |
| 1 | "Greenfield Farm" | Farm, Field |
| 2 | "General Store" | Store, Village |
| 3 | "Weary Traveler Inn" | Inn, Village |
| 4 | "Ruler's Hall" | Hall, Village |
| 5 | "Guard Barracks" | Barracks, Village |
| 6 | "Public Latrine" | Latrine, Village |
| 7 | "Northern Crossroads" | Crossroads, Road |
| 8 | "Forest Path" | Forest, Trail |
| 9 | "Bandit Camp" | Camp, Forest |
| 10 | "South Gate" | Gate, Road |
| 11 | "East Trail" | Trail, Field |

Edges connect these into a navigable graph. Exact edge list is implementation detail but must satisfy:
- Every place has >= 1 incoming and >= 1 outgoing edge.
- The graph is fully connected (every place reachable from every other).
- Travel times are reasonable integers (e.g., 1-10 ticks for village internal, 5-20 for longer routes).
- Danger/visibility values differentiate safe village roads from dangerous forest paths.

### 2. Helper: `EntityId` factory

A simple counter that produces sequential `EntityId { slot: n, generation: 0 }` values. Can be a local helper in the builder, not a public API.

### 3. Re-export builder from `lib.rs`

## Files to Touch

- `crates/worldwake-core/src/topology.rs` (modify — add `build_prototype_world()` and any helpers)
- `crates/worldwake-core/src/lib.rs` (modify — re-export `build_prototype_world`)

## Out of Scope

- Random world generation — this is a fixed prototype.
- Population spawning (agents, NPCs) — that's E03+.
- Item placement — that's E04+.
- Any gameplay logic — later epics.
- Topology serialization round-trip — E02WORTOP-006.

## Acceptance Criteria

### Tests That Must Pass

1. `build_prototype_world()` creates 12-20 places — spec test.
2. Every prototype place has at least one incoming and one outgoing edge — spec test.
3. Graph traversal from any place visits all places (fully connected) — spec test.
4. Builder is deterministic: two calls produce byte-identical topologies (compare serialized output).
5. All `danger` values are valid `Permille` (0..=1000) — inherently enforced by type but tested.
6. All `visibility` values are valid `Permille` (0..=1000).
7. All `travel_time_ticks >= 1` for every edge.
8. Place names are non-empty strings.
9. Expected place tags are present (e.g., the village has `Village` tag, farm has `Farm`).
10. Existing suite: `cargo test -p worldwake-core`.

### Invariants

1. Builder output is stable: same code path => same place IDs, edge IDs, topology — spec invariant.
2. No randomness in the builder function (no RNG parameter, no system calls).
3. No floating-point values anywhere in the output.
4. The generated graph matches spec 4.1 place list.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/topology.rs` — builder tests: place count, edge count, connectivity, determinism, tag verification, invariant checks.

### Commands

1. `cargo test -p worldwake-core build_prototype`
2. `cargo test -p worldwake-core topology`
3. `cargo clippy --workspace && cargo test --workspace`
