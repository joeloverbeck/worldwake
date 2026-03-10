# E10PROTRA-003: CarryCapacity + InTransitOnEdge components in worldwake-core

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — two new authoritative component registrations
**Deps**: E10PROTRA-001 (InTransitOnEdge uses EntityId and Tick which already exist in core)

## Problem

Transport requires two concrete components on agents: `CarryCapacity` (how much an agent can carry, using `LoadUnits`) and `InTransitOnEdge` (explicit route occupancy during travel). Without `CarryCapacity`, there is no carry limit enforcement. Without `InTransitOnEdge`, travel is teleportation — violating invariant 9.10 and Principle 7 (route presence must be concrete occupancy).

`InTransitOnEdge` is shared Phase 2 schema (Step 7a) needed by E12 (combat encounters on routes) and E13 (AI awareness of transit state).

## Assumption Reassessment (2026-03-10)

1. `LoadUnits(u32)` exists in `numerics.rs` — confirmed.
2. `EntityId` and `Tick` exist in `ids.rs` — confirmed.
3. No `CarryCapacity` or `InTransitOnEdge` types exist — confirmed.
4. `TravelEdgeId` exists in `ids.rs` as a newtype for travel edge identification.
5. The `InTransitOnEdge` spec uses `EntityId` for `edge_id`, `origin`, `destination` and `u64` for ticks. However, `Tick(u64)` already exists as a newtype — use that instead of raw `u64`.
6. Both components go on `EntityKind::Agent` — the kind predicate is agent-only.

## Architecture Check

1. `CarryCapacity(LoadUnits)` is a thin wrapper reusing existing `LoadUnits` infrastructure. Current load is derived from carried items (a read-model), not stored — consistent with Principle 3.
2. `InTransitOnEdge` makes route presence concrete and physical. An agent in transit is not at a place — they are on an edge. This enables future ambush, escort, and witness logic.
3. Both are authoritative stored state. "Can carry more" and "who is on this edge" are derived read-models.
4. Grouping these in one ticket is justified because both are transport-domain agent components with similar scope.

## What to Change

### 1. Add to `crates/worldwake-core/src/production.rs` (or new `transport.rs` module)

```rust
/// Maximum load an agent can carry.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub struct CarryCapacity(pub LoadUnits);
impl Component for CarryCapacity {}

/// Concrete route occupancy during travel.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct InTransitOnEdge {
    pub edge_id: TravelEdgeId,
    pub origin: EntityId,
    pub destination: EntityId,
    pub departure_tick: Tick,
    pub arrival_tick: Tick,
}
impl Component for InTransitOnEdge {}
```

Consider placing these in a new `crates/worldwake-core/src/transport.rs` module rather than `production.rs`, since they are transport-domain types.

### 2. Register both in `component_schema.rs`

Both restricted to `EntityKind::Agent`.

### 3. Schema fanout

Update imports/tests in `component_tables.rs`, `world.rs`, `delta.rs`.

### 4. Export from `lib.rs`

## Files to Touch

- `crates/worldwake-core/src/transport.rs` (new — or extend `production.rs`)
- `crates/worldwake-core/src/component_schema.rs` (modify — add 2 component registrations)
- `crates/worldwake-core/src/lib.rs` (modify — add module + re-exports)
- `crates/worldwake-core/src/component_tables.rs` (modify — schema fanout)
- `crates/worldwake-core/src/world.rs` (modify — generated API tests)
- `crates/worldwake-core/src/delta.rs` (modify — component inventory coverage)

## Out of Scope

- Travel action logic (E10PROTRA-010)
- Pick-up / put-down action logic (E10PROTRA-011)
- Carry limit enforcement logic (E10PROTRA-011)
- Route danger scoring or combat encounter logic (E12)
- AI perception of transit state (E13/E14)
- Topology changes or new edge types

## Acceptance Criteria

### Tests That Must Pass

1. `CarryCapacity` can be inserted/retrieved/removed on Agent entities through the `World` API.
2. `CarryCapacity` insertion is rejected for non-Agent kinds.
3. `InTransitOnEdge` can be inserted/retrieved/removed on Agent entities through the `World` API.
4. `InTransitOnEdge` insertion is rejected for non-Agent kinds.
5. Both round-trip through bincode.
6. `InTransitOnEdge` correctly stores `TravelEdgeId`, origin, destination, departure, and arrival ticks.
7. `ComponentKind::ALL` and `ComponentValue` coverage include both new components.
8. Existing suite: `cargo test -p worldwake-core`

### Invariants

1. `CarryCapacity` uses `LoadUnits` — no raw integers or floats for weight.
2. `InTransitOnEdge` uses `Tick` for temporal fields — no raw `u64`.
3. `InTransitOnEdge` uses `TravelEdgeId` — not raw EntityId for the edge.
4. Both components are agent-only authoritative stored state.
5. "Can carry more" and "who is on this edge" remain derived, not stored.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/transport.rs` — construction, serialization, trait bounds
2. `crates/worldwake-core/src/component_tables.rs` — table CRUD for both components
3. `crates/worldwake-core/src/world.rs` — kind-restricted insertion/query + wrong-kind rejection
4. `crates/worldwake-core/src/delta.rs` — component inventory coverage

### Commands

1. `cargo test -p worldwake-core`
2. `cargo clippy --workspace --all-targets -- -D warnings`
3. `cargo test --workspace`
