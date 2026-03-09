# E04ITECON-007: Load accounting helpers

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: None (pure functions on World)
**Deps**: E04ITECON-003 (ItemLot), E04ITECON-004 (UniqueItem), E04ITECON-005 (ContainerData)

## Problem

Container capacity enforcement requires deterministic load calculation. Without load accounting, there is no way to check whether an item fits in a container before transfer. The spec requires `LoadUnits`-based capacity, not raw quantity.

## Assumption Reassessment (2026-03-09)

1. `LoadUnits` newtype exists in `numerics.rs` — confirmed
2. `ItemLot`, `UniqueItem`, `ContainerData` components will exist after their tickets — dependencies
3. Containment relations (`contained_by`) do not exist yet (E05) — load calculation for "items in a container" requires some way to find children; this ticket defines the accounting logic and uses a simple query approach
4. No existing load functions — confirmed

## Architecture Check

1. Load weight functions are pure functions in a new `load.rs` module (not methods on components)
2. Load-per-unit for commodities and unique item kinds is defined via match arms — single source of truth for weight tables
3. Container load calculation requires iterating entities that reference a container — since `contained_by` relations don't exist yet (E05), this ticket provides the weight-per-item functions and a `current_container_load` that takes an explicit list of contained entity IDs. E05 will wire containment queries into this.
4. `ItemLot.quantity` is a `Quantity`; load math should cross that wrapper only at the arithmetic boundary instead of spreading raw integers back through item-domain APIs

## What to Change

### 1. Create `crates/worldwake-core/src/load.rs`

```rust
pub fn load_per_unit(commodity: CommodityKind) -> LoadUnits { ... }
pub fn load_of_lot(lot: &ItemLot) -> LoadUnits { ... }
pub fn load_of_unique_item_kind(kind: UniqueItemKind) -> LoadUnits { ... }
pub fn load_of_unique_item(item: &UniqueItem) -> LoadUnits { ... }
```

Weight table (initial values, tunable later):
- Apple, Grain, Bread: `LoadUnits(1)` per unit
- Water: `LoadUnits(2)` per unit
- Firewood: `LoadUnits(3)` per unit
- Medicine: `LoadUnits(1)` per unit
- Coin: `LoadUnits(1)` per unit (small but non-zero)
- Waste: `LoadUnits(1)` per unit

Unique item weights:
- SimpleTool: `LoadUnits(5)`
- Weapon: `LoadUnits(10)`
- Contract: `LoadUnits(1)`
- Artifact: `LoadUnits(5)`
- OfficeInsignia: `LoadUnits(2)`
- Misc: `LoadUnits(3)`

### 2. Container load query helpers

```rust
pub fn load_of_entity(world: &World, entity_id: EntityId) -> LoadUnits { ... }
```

Returns the load of a single entity by checking if it has `ItemLot` or `UniqueItem` and computing accordingly. Returns `LoadUnits(0)` for entities without item components.

```rust
pub fn current_container_load(world: &World, container_id: EntityId, contained: &[EntityId]) -> Result<LoadUnits, WorldError> { ... }
```

Sums load of all contained entities. The `contained` slice is provided by the caller (E05 will provide containment queries).

```rust
pub fn remaining_container_capacity(world: &World, container_id: EntityId, contained: &[EntityId]) -> Result<LoadUnits, WorldError> { ... }
```

Returns `capacity - current_load`. Errors if container doesn't exist or load exceeds capacity (invariant violation).

### 3. Register module in `lib.rs`

Add `pub mod load;` and re-export key functions.

## Files to Touch

- `crates/worldwake-core/src/load.rs` (new)
- `crates/worldwake-core/src/lib.rs` (modify — add module + re-exports)

## Out of Scope

- Containment relations / `contained_by` query (E05 — this ticket takes explicit entity lists)
- Recursive nested container load (deferred: requires containment graph from E05)
- Capacity enforcement during transfer (E05)
- Modifying weight tables at runtime (future tuning)
- Conservation verification (E04ITECON-008)

## Acceptance Criteria

### Tests That Must Pass

1. `load_of_lot` for `Quantity(10)` apples returns `LoadUnits(10)` (1 per unit × 10)
2. `load_of_lot` for `Quantity(5)` water returns `LoadUnits(10)` (2 per unit × 5)
3. `load_of_unique_item` for a Weapon returns `LoadUnits(10)`
4. `load_of_entity` correctly dispatches between lot and unique item
5. `load_of_entity` for an entity with no item component returns `LoadUnits(0)`
6. `current_container_load` sums loads of all contained entities correctly
7. `remaining_container_capacity` returns `capacity - load` for a valid container
8. `remaining_container_capacity` errors if container entity has no `ContainerData`
9. Container load calculations include both lots and unique items
10. Existing suite: `cargo test -p worldwake-core`

### Invariants

1. Capacity is measured in `LoadUnits`, not raw quantity (spec requirement)
2. Load functions are pure and deterministic
3. No `HashMap` or floating-point in load calculations
4. All existing tests continue to pass unchanged
5. Quantity wrappers are preserved in item-domain APIs and only unwrapped at the load-arithmetic boundary

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/load.rs` (inline `#[cfg(test)]` module) — weight table correctness, entity dispatch, container load sums, remaining capacity

### Commands

1. `cargo test -p worldwake-core load`
2. `cargo clippy --workspace && cargo test --workspace`
