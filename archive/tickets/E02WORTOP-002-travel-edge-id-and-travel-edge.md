# E02WORTOP-002: TravelEdgeId and TravelEdge Struct

**Status**: ✅ COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: None
**Deps**: E01 (EntityId, Permille), E02WORTOP-001 (topology module exists)

## Problem

The directed graph topology requires typed edge identifiers and edge structs carrying travel time, capacity, danger, and visibility. These must be fixed-point (no floats) and fully serializable.

## Assumption Reassessment (2026-03-09)

1. `Permille` exists in `numerics.rs` with `new()` validation for `0..=1000` — confirmed.
2. `EntityId` exists in `ids.rs` — confirmed.
3. `topology.rs` already exists from E02WORTOP-001 and currently contains `PlaceTag` / `Place` plus topology-focused unit tests — confirmed.
4. `WorldError` already exists in `error.rs` as the crate-level legality/error type — confirmed and preferable to ad hoc string errors for constructor validation.
5. Spec requires `travel_time_ticks >= 1`, and that invariant must survive both normal construction and serde deserialization.

## Architecture Check

1. `TravelEdgeId(u32)` follows the same newtype pattern as `EntityId`, `EventId`, etc. Derives the full set of deterministic traits.
2. `TravelEdge` uses `Permille` for `danger` and `visibility`, keeping everything fixed-point.
3. The original ticket shape was too weak: a validating constructor alone is not enough if the struct remains directly constructible or deserializable with `travel_time_ticks == 0`.
4. To make the invariant structural instead of advisory, store `travel_time_ticks` as `NonZeroU32` internally and expose it via an accessor returning `u32`. This still satisfies the spec's semantic contract while producing a cleaner, harder-to-misuse edge type.
5. `TravelEdge::new(...) -> Result<Self, WorldError>` should be the canonical construction path. Accessors should be provided for later topology/pathfinding code instead of relying on public fields.
6. No shims needed — new types.

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
    id: TravelEdgeId,
    from: EntityId,
    to: EntityId,
    travel_time_ticks: NonZeroU32,
    capacity: Option<NonZeroU16>,
    danger: Permille,
    visibility: Permille,
}
```

With:
- `TravelEdge::new(...) -> Result<Self, WorldError>` validating `travel_time_ticks >= 1`
- field accessors (`id()`, `from()`, `to()`, `travel_time_ticks()`, `capacity()`, `danger()`, `visibility()`)

Rationale:
- `NonZeroU32` enforces the invariant at the data-shape level
- serde deserialization of an invalid zero travel time fails automatically
- later tickets can consume a stable API without depending on direct field access

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
5. Deserializing a raw edge payload with `travel_time_ticks == 0` fails.
6. `TravelEdge` `danger` and `visibility` fields are always `Permille` (inherently bounded `0..=1000`).
7. Existing suite: `cargo test -p worldwake-core`.

### Invariants

1. No floating-point fields in `TravelEdge`.
2. `danger` and `visibility` are `Permille` (0..=1000 enforced by type).
3. `travel_time_ticks >= 1` is enforced structurally via `NonZeroU32`, including deserialization.
4. `TravelEdgeId` ordering is deterministic (derives `Ord`).

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/ids.rs` — `TravelEdgeId` trait bound assertions, display, bincode round-trip.
2. `crates/worldwake-core/src/topology.rs` — `TravelEdge` construction validation, round-trip, accessor coverage, and invalid-deserialization invariant checks.

### Commands

1. `cargo test -p worldwake-core travel_edge`
2. `cargo clippy --workspace && cargo test --workspace`

## Outcome

- Completion date: 2026-03-09
- What actually changed:
  - Added `TravelEdgeId` to `crates/worldwake-core/src/ids.rs` with deterministic trait coverage, `Display`, and bincode round-trip tests.
  - Added `TravelEdge` to `crates/worldwake-core/src/topology.rs` with `TravelEdge::new(...) -> Result<Self, WorldError>`, accessor methods, and serde-backed invariant protection for nonzero travel time.
  - Re-exported `TravelEdgeId` and `TravelEdge` from `crates/worldwake-core/src/lib.rs`.
  - Strengthened `worldwake-core` tests to cover constructor rejection of zero travel time, acceptance of the minimum valid value, round-trip serialization, and rejection of invalid zero-tick deserialization payloads.
  - Fixed pre-existing `worldwake-core` test-helper/doc-comment issues required for `cargo clippy --workspace --all-targets -- -D warnings` to pass.
- Deviations from original plan:
  - The original ticket proposed a raw `u32` field plus constructor validation. That was corrected before implementation because it does not protect the invariant across serde deserialization and is too easy to bypass over time.
  - The implementation stores travel time as `NonZeroU32` internally and exposes it through an accessor, which is a stricter and more durable architecture while preserving the spec's `>= 1` semantics.
- Verification results:
  - `cargo test -p worldwake-core travel_edge` passed.
  - `cargo test -p worldwake-core` passed.
  - `cargo clippy --workspace --all-targets -- -D warnings` passed.
  - `cargo test --workspace` passed.
