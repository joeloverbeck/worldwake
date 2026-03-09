# E02WORTOP-002: TravelEdgeId and TravelEdge Struct

**Status**: PENDING
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: None
**Deps**: E01 (EntityId, Permille), E02WORTOP-001 (topology module exists)

## Problem

The directed graph topology requires typed edge identifiers and edge structs carrying travel time, capacity, danger, and visibility. These must be fixed-point (no floats) and fully serializable.

## Assumption Reassessment (2026-03-09)

1. `Permille` exists in `numerics.rs` with `new()` validation for `0..=1000` — confirmed.
2. `EntityId` exists in `ids.rs` — confirmed.
3. The topology module from E02WORTOP-001 will exist at `crates/worldwake-core/src/topology.rs` — dependency.
4. Spec requires `travel_time_ticks >= 1` — must be enforced at construction.

## Architecture Check

1. `TravelEdgeId(u32)` follows the same newtype pattern as `EntityId`, `EventId`, etc. Derives the full set of deterministic traits.
2. `TravelEdge` uses `Permille` for `danger` and `visibility`, keeping everything fixed-point.
3. `travel_time_ticks` minimum of 1 is enforced by a constructor, not by the type system alone (a `NonZeroU32` would also work but the spec says `u32` with a runtime check).
4. No shims needed — new types.

## What to Change

### 1. Add `TravelEdgeId` to `ids.rs`

```rust
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Serialize, Deserialize)]
pub struct TravelEdgeId(pub u32);
```

With `Display` impl following existing patterns.

### 2. Add `TravelEdge` to `topology.rs`

```rust
pub struct TravelEdge {
    pub id: TravelEdgeId,
    pub from: EntityId,
    pub to: EntityId,
    pub travel_time_ticks: u32,
    pub capacity: Option<NonZeroU16>,
    pub danger: Permille,
    pub visibility: Permille,
}
```

With a constructor that validates `travel_time_ticks >= 1`.

### 3. Re-export from `lib.rs`

Add `TravelEdgeId` to the `ids` re-exports and `TravelEdge` to the `topology` re-exports.

## Files to Touch

- `crates/worldwake-core/src/ids.rs` (modify — add `TravelEdgeId`)
- `crates/worldwake-core/src/topology.rs` (modify — add `TravelEdge`)
- `crates/worldwake-core/src/lib.rs` (modify — add re-exports)

## Out of Scope

- Topology graph storage (`BTreeMap` adjacency) — E02WORTOP-003.
- Pathfinding — E02WORTOP-004.
- World builder — E02WORTOP-005.
- Edge mutation or occupancy tracking — future epics.

## Acceptance Criteria

### Tests That Must Pass

1. `TravelEdgeId` satisfies `Copy + Clone + Eq + Ord + Hash + Debug + Display + Serialize + DeserializeOwned`.
2. `TravelEdge` bincode round-trip with various `danger`/`visibility` Permille values.
3. `TravelEdge` construction rejects `travel_time_ticks == 0`.
4. `TravelEdge` construction accepts `travel_time_ticks == 1` (minimum valid).
5. `TravelEdge` `danger` and `visibility` fields are always `Permille` (inherently bounded `0..=1000`).
6. Existing suite: `cargo test -p worldwake-core`.

### Invariants

1. No floating-point fields in `TravelEdge`.
2. `danger` and `visibility` are `Permille` (0..=1000 enforced by type).
3. `travel_time_ticks >= 1` enforced at construction.
4. `TravelEdgeId` ordering is deterministic (derives `Ord`).

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/ids.rs` — `TravelEdgeId` trait bound assertions, display, bincode round-trip.
2. `crates/worldwake-core/src/topology.rs` — `TravelEdge` construction validation, round-trip, invariant checks.

### Commands

1. `cargo test -p worldwake-core travel_edge`
2. `cargo clippy --workspace && cargo test --workspace`
