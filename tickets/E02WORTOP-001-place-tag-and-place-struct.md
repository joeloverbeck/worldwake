# E02WORTOP-001: PlaceTag Enum and Place Struct

**Status**: PENDING
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: None
**Deps**: E01 (EntityId, Permille, core traits)

## Problem

E02 requires typed place nodes for the world topology graph. Before building the graph itself, we need the `PlaceTag` enum (categorizing places) and the `Place` component struct that attaches to place entities.

## Assumption Reassessment (2026-03-09)

1. `EntityId` exists in `crates/worldwake-core/src/ids.rs` — confirmed.
2. `Component` trait exists in `crates/worldwake-core/src/traits.rs` — confirmed; requires `'static + Send + Sync + Clone + Debug + Serialize + DeserializeOwned`.
3. `BTreeSet` is allowed per the deterministic data policy in `lib.rs` — confirmed.
4. `NonZeroU16` is a standard library type, no external dep needed — confirmed.

## Architecture Check

1. `PlaceTag` is a simple flat enum with derived `Ord` for `BTreeSet` storage. No variant carries data — just tags. This keeps it serializable and deterministic.
2. `Place` implements `Component` so it can later be stored in the ECS. It holds `name`, `capacity`, and `tags`.
3. No shims or backwards-compatibility needed — these are new types.

## What to Change

### 1. New module `topology.rs` (or `topology/mod.rs`)

Define:

```rust
/// Categorizes a place in the world graph.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Serialize, Deserialize)]
pub enum PlaceTag {
    Village,
    Farm,
    Store,
    Inn,
    Hall,
    Barracks,
    Latrine,
    Crossroads,
    Forest,
    Camp,
    Road,
    Trail,
    Field,
    Gate,
}
```

```rust
/// A named location in the world graph.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Place {
    pub name: String,
    pub capacity: Option<NonZeroU16>,
    pub tags: BTreeSet<PlaceTag>,
}
impl Component for Place {}
```

### 2. Register module in `lib.rs`

Add `pub mod topology;` and re-export `PlaceTag` and `Place`.

## Files to Touch

- `crates/worldwake-core/src/topology.rs` (new)
- `crates/worldwake-core/src/lib.rs` (modify — add module + re-exports)

## Out of Scope

- `TravelEdge`, `TravelEdgeId`, topology graph storage, pathfinding — separate tickets.
- ECS integration (typed component storage registration) — that's E03.
- World builder logic — E02WORTOP-005.
- Any gameplay logic using places.

## Acceptance Criteria

### Tests That Must Pass

1. `PlaceTag` variants are `Copy + Clone + Eq + Ord + Hash + Debug + Serialize + DeserializeOwned`.
2. `Place` satisfies the `Component` trait bounds.
3. `PlaceTag` sorts deterministically (e.g., `Village < Farm` is consistent with derive order).
4. `Place` with a `BTreeSet<PlaceTag>` containing multiple tags serializes and deserializes via bincode round-trip.
5. `Place` with `capacity: None` and `capacity: Some(NonZeroU16)` both round-trip correctly.
6. Existing suite: `cargo test -p worldwake-core`.

### Invariants

1. No `HashMap` or `HashSet` in any authoritative state (deterministic data policy).
2. `PlaceTag` iteration order in `BTreeSet` is stable and deterministic.
3. No floating-point types in `Place` or `PlaceTag`.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/topology.rs` (inline `#[cfg(test)]`) — trait bound assertions, bincode round-trips, sort order verification.

### Commands

1. `cargo test -p worldwake-core topology`
2. `cargo clippy --workspace && cargo test --workspace`
