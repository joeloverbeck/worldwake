# E10PROTRA-003: CarryCapacity + InTransitOnEdge components in worldwake-core

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — two new authoritative component registrations
**Deps**: `archive/tickets/completed/E10PROTRA-001.md` (completed shared-schema extraction for `production.rs`); `TravelEdgeId`, `EntityId`, `Tick`, and `LoadUnits` already exist in core

## Problem

Transport requires two concrete components on agents: `CarryCapacity` (how much an agent can carry, using `LoadUnits`) and `InTransitOnEdge` (explicit route occupancy during travel). Without `CarryCapacity`, there is no carry limit enforcement. Without `InTransitOnEdge`, travel is teleportation — violating invariant 9.10 and Principle 7 (route presence must be concrete occupancy).

`InTransitOnEdge` is shared Phase 2 schema (Step 7a) needed by E12 (combat encounters on routes) and E13 (AI awareness of transit state).

## Assumption Reassessment (2026-03-10)

1. `LoadUnits(u32)` exists in `numerics.rs` — confirmed.
2. `EntityId` and `Tick` exist in `ids.rs` — confirmed.
3. No `CarryCapacity` or `InTransitOnEdge` types exist — confirmed.
4. `TravelEdgeId` exists in `ids.rs` as a newtype for travel edge identification.
5. `production.rs` already exists as the shared Phase 2 schema home for E10 core types (`WorkstationTag`, `RecipeId`, `ResourceSource`). Extending that module is cleaner than creating a new `transport.rs` split for just two types.
6. The E10 spec text still shows `edge_id: EntityId` and raw `u64` tick fields for `InTransitOnEdge`, but the current core already has stronger types available. This ticket should use `TravelEdgeId` and `Tick` directly.
7. The world already has a generic authoritative `RelationKind::InTransit` / `world.is_in_transit(...)` placement relation. `InTransitOnEdge` must refine that generic physical-placement state with route-specific occupancy; it must not replace or duplicate the placement semantics.
8. Both new components go on `EntityKind::Agent` — the kind predicate is agent-only.
9. `component_schema.rs` is the single authoritative declaration point for ECS components, and its macro fanout affects `component_tables.rs`, `world.rs`, `delta.rs`, and `world_txn.rs`. This ticket must update all of those seams, not just the component declarations.

## Architecture Check

1. `CarryCapacity(LoadUnits)` is a thin wrapper reusing existing `LoadUnits` infrastructure. Current load is derived from carried items (a read-model), not stored — consistent with Principle 3.
2. `InTransitOnEdge` makes route presence concrete and physical, but it should layer on top of the existing generic `InTransit` placement relation rather than creating a second competing placement model. During travel, the agent should still be generically "in transit"; this component adds which edge, from where, to where, and for how long.
3. Both are authoritative stored state. "Can carry more" and "who is on this edge" are derived read-models.
4. Grouping these in one ticket is justified because both are transport-domain agent components with similar scope.
5. `InTransitOnEdge` is transient action state that E10PROTRA-010 will set and remove through `WorldTxn`. To keep the delta pipeline coherent, this ticket should wire the new component types into the transaction-layer simple-component support now instead of forcing E10PROTRA-010 to invent a side path.
6. Do not create a separate `transport.rs` yet. The current architecture intentionally groups shared E10 schema in `production.rs`; a split would be premature until the transport surface is materially larger.

## What to Change

### 1. Add to `crates/worldwake-core/src/production.rs`

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

### 2. Register both in `component_schema.rs`

Both restricted to `EntityKind::Agent`.

### 3. Schema fanout

Update imports/tests in `component_tables.rs`, `world.rs`, and `delta.rs`.

### 4. Transaction-layer component support

Add both component types to `with_txn_simple_set_components!` so later transport systems can emit typed component deltas through `WorldTxn::set_component_carry_capacity(...)` and `WorldTxn::set_component_in_transit_on_edge(...)`.

### 5. Export from `lib.rs`

## Files to Touch

- `crates/worldwake-core/src/production.rs` (modify — add `CarryCapacity`, `InTransitOnEdge`)
- `crates/worldwake-core/src/component_schema.rs` (modify — add 2 component registrations and txn setter entries)
- `crates/worldwake-core/src/lib.rs` (modify — add module + re-exports)
- `crates/worldwake-core/src/component_tables.rs` (modify — schema fanout)
- `crates/worldwake-core/src/world.rs` (modify — generated API tests)
- `crates/worldwake-core/src/delta.rs` (modify — component inventory coverage)
- `crates/worldwake-core/src/world_txn.rs` (modify — transaction-layer setter coverage)

## Out of Scope

- Travel action logic (E10PROTRA-010)
- Pick-up / put-down action logic (E10PROTRA-011)
- Carry limit enforcement logic (E10PROTRA-011)
- Generic placement-relation redesign
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
8. `WorldTxn::set_component_carry_capacity(...)` records a typed `ComponentDelta::Set` and updates the world on commit.
9. `WorldTxn::set_component_in_transit_on_edge(...)` records a typed `ComponentDelta::Set` and updates the world on commit.
10. Existing suite: `cargo test -p worldwake-core`

### Invariants

1. `CarryCapacity` uses `LoadUnits` — no raw integers or floats for weight.
2. `InTransitOnEdge` uses `Tick` for temporal fields — no raw `u64`.
3. `InTransitOnEdge` uses `TravelEdgeId` — not raw EntityId for the edge.
4. Both components are agent-only authoritative stored state.
5. `InTransitOnEdge` refines existing `InTransit` placement state; this ticket does not create a second independent transit model.
6. "Can carry more" and "who is on this edge" remain derived, not stored.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/production.rs` — construction, serialization, trait bounds
2. `crates/worldwake-core/src/component_tables.rs` — table CRUD for both components
3. `crates/worldwake-core/src/world.rs` — kind-restricted insertion/query + wrong-kind rejection
4. `crates/worldwake-core/src/delta.rs` — component inventory coverage
5. `crates/worldwake-core/src/world_txn.rs` — typed `ComponentDelta::Set` coverage for both components

### Commands

1. `cargo test -p worldwake-core carry_capacity`
2. `cargo test -p worldwake-core in_transit_on_edge`
3. `cargo test -p worldwake-core`
4. `cargo clippy --workspace --all-targets -- -D warnings`
5. `cargo test --workspace`

## Outcome

- Completion date: 2026-03-10
- What actually changed:
  - Added `CarryCapacity(LoadUnits)` and `InTransitOnEdge` to `crates/worldwake-core/src/production.rs`.
  - Registered both components in the authoritative schema as agent-only components.
  - Re-exported both types from `worldwake-core`.
  - Extended schema fanout coverage in `component_tables.rs`, `world.rs`, `delta.rs`, and `world_txn.rs`.
  - Added focused tests for component serialization, table CRUD, world API kind enforcement, and transaction-layer typed component deltas.
  - Updated the stale `ComponentKind::ALL` expectation in `crates/worldwake-systems/tests/e09_needs_integration.rs` so workspace-wide verification reflects the expanded authoritative schema.
- Deviations from original plan:
  - Corrected the ticket first to match the actual codebase: `E10PROTRA-001` already exists in the completed archive, `production.rs` is the current shared E10 schema home, and `InTransitOnEdge` needs to refine the existing generic `InTransit` placement relation rather than replace it.
  - Kept the new types in `production.rs` instead of creating a premature `transport.rs` split.
  - Expanded scope slightly to include `WorldTxn` setter support for both new components because later transport actions need the standard typed delta pipeline rather than an ad hoc mutation path.
  - No placement-relation redesign or travel action logic was added here.
- Verification results:
  - `cargo test -p worldwake-core carry_capacity` ✅
  - `cargo test -p worldwake-core in_transit_on_edge` ✅
  - `cargo test -p worldwake-core` ✅
  - `cargo clippy --workspace --all-targets -- -D warnings` ✅
  - `cargo test --workspace` ✅
- Outcome amended: 2026-03-10
- Post-completion refinement:
  - Added transactional clear/remove support for macro-driven simple components in `WorldTxn`, including `clear_component_carry_capacity(...)` and `clear_component_in_transit_on_edge(...)`.
  - This closes the architectural gap noted after completion: future travel completion/abort code can now remove transient route-occupancy state through the standard typed delta pipeline instead of requiring an ad hoc removal path.
  - Added focused removal/no-op tests in `crates/worldwake-core/src/world_txn.rs`.
