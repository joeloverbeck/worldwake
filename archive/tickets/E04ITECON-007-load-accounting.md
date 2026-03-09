# E04ITECON-007: Load accounting helpers

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Additive public API only (new pure helper module; no ECS schema or lifecycle changes)
**Deps**: E04ITECON-003 (ItemLot), E04ITECON-004 (UniqueItem), E04ITECON-005 (Container)

## Problem

Container capacity enforcement requires deterministic load calculation. Without load accounting, there is no way to check whether an item fits in a container before transfer. The spec requires `LoadUnits`-based capacity, not raw quantity.

## Assumption Reassessment (2026-03-09)

1. `LoadUnits` already exists in `crates/worldwake-core/src/numerics.rs` — confirmed
2. `ItemLot`, `UniqueItem`, and `Container` already exist in `crates/worldwake-core/src/items.rs`; the ticket reference to `ContainerData` is stale and must not drive the implementation — confirmed
3. `World` already exposes stable read/query APIs for live `ItemLot`, `UniqueItem`, and `Container` components (`get_component_*`, `query_*`, `entities_with_*`) — confirmed
4. There is still no containment graph or `contained_by` relation in core. E05 remains the first ticket that can supply authoritative parent/child container membership — confirmed
5. No existing load-accounting module or helper functions exist yet — confirmed
6. `WorldError` already includes the right error vocabulary for this work (`EntityNotFound`, `ArchivedEntity`, `ComponentNotFound`, `InvariantViolation`) — confirmed

## Architecture Check

1. Load accounting should live in a dedicated `load.rs` module, not as ad hoc methods on `ItemLot`, `UniqueItem`, or `Container`. This keeps weight rules centralized and keeps `World` focused on lifecycle/state management instead of derived calculations.
2. Commodity and unique-item weight tables should remain explicit `match` expressions over the enums. That gives a single deterministic source of truth and forces future additions to make their load cost explicit at compile time.
3. Because E05 has not introduced containment relations yet, this ticket should not fake a containment model inside `World`. The cleanest architecture here is pure helper functions that accept an explicit iterable of contained entity IDs supplied by the caller.
4. `load_of_entity(...)` should error for missing or archived entities rather than silently returning zero. Returning zero is acceptable for alive non-item entities, but treating invalid entity references as weightless would hide broken containment inputs and weaken invariants.
5. `remaining_container_capacity(...)` should treat `current_load > capacity` as an invariant violation, not clamp to zero. Silent clamping would mask corruption and make later capacity enforcement harder to reason about.
6. `ItemLot.quantity` should stay strongly typed as `Quantity`; unwrapping to raw integers belongs only at the load arithmetic boundary.
7. The E04 spec’s ideal end-state is recursive nested-container load counted exactly once, but that cannot be implemented honestly before E05 supplies authoritative containment semantics. This ticket should therefore define non-recursive accounting primitives only, with an API shape E05 can compose later.

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
pub fn load_of_entity(world: &World, entity_id: EntityId) -> Result<LoadUnits, WorldError> { ... }
```

Returns the load of a single live entity by checking whether it has `ItemLot` or `UniqueItem` and computing accordingly.

Rules:
- return `Err(WorldError::EntityNotFound(_))` if the entity does not exist
- return `Err(WorldError::ArchivedEntity(_))` if the entity is archived
- return `Ok(LoadUnits(0))` for alive entities that are not item-bearing (`Office`, `Faction`, `Container`, etc.)
- do not special-case `Container` recursively in this ticket; nested-container recursion belongs after E05 defines authoritative containment

```rust
pub fn current_container_load(
    world: &World,
    container_id: EntityId,
    contained: impl IntoIterator<Item = EntityId>,
) -> Result<LoadUnits, WorldError> { ... }
```

Sums load of all explicitly supplied contained entities.

Rules:
- validate that `container_id` names a live entity with a `Container` component before summing
- sum by delegating to `load_of_entity(...)`
- accept any iterable of `EntityId` so E05 can pass query results without forced allocation
- reject duplicate contained entity IDs as `InvariantViolation` to avoid silently double-counting invalid membership input
- do not infer or query membership inside this helper; the caller remains responsible for supplying the intended contents

```rust
pub fn remaining_container_capacity(
    world: &World,
    container_id: EntityId,
    contained: impl IntoIterator<Item = EntityId>,
) -> Result<LoadUnits, WorldError> { ... }
```

Returns `capacity - current_load`.

Rules:
- error if `container_id` is missing, archived, or lacks a `Container` component
- return `Err(WorldError::InvariantViolation(_))` if `current_load > capacity`
- do not convert overflow/invariant failures into saturating math

### 3. Register module in `lib.rs`

Add `pub mod load;` and re-export key functions.

## Files to Touch

- `crates/worldwake-core/src/load.rs` (new)
- `crates/worldwake-core/src/lib.rs` (modify — add module + re-exports)

## Out of Scope

- Containment relations / `contained_by` or `located_in` query logic (E05 supplies authoritative membership)
- Recursive nested-container load accounting (deferred until E05 can define parent/child semantics without duplication or cycles)
- Capacity enforcement during transfer or placement (future work after load accounting and containment semantics meet)
- Admission-rule enforcement using `allowed_commodities`, `allows_unique_items`, or `allows_nested_containers`
- Modifying weight tables at runtime
- Conservation verification (E04ITECON-008)

## Acceptance Criteria

### Tests That Must Pass

1. `load_of_lot` for `Quantity(10)` apples returns `LoadUnits(10)` (1 per unit × 10)
2. `load_of_lot` for `Quantity(5)` water returns `LoadUnits(10)` (2 per unit × 5)
3. `load_of_unique_item` for a Weapon returns `LoadUnits(10)`
4. `load_of_entity` correctly dispatches between lot and unique item
5. `load_of_entity` for a live non-item entity returns `LoadUnits(0)`
6. `load_of_entity` errors for missing and archived entities
7. `current_container_load` sums loads of all supplied contained entities correctly
8. `current_container_load` errors if `container_id` is missing, archived, or lacks a `Container` component
9. `current_container_load` returns `InvariantViolation` if the supplied contained IDs contain duplicates
10. `remaining_container_capacity` returns `capacity - load` for a valid container
11. `remaining_container_capacity` returns `InvariantViolation` if supplied contents already exceed capacity
12. Container load calculations include both lots and unique items
13. Existing suite: `cargo test -p worldwake-core`

### Invariants

1. Capacity is measured in `LoadUnits`, not raw quantity (spec requirement)
2. Load functions are pure and deterministic
3. No `HashMap` or floating-point in load calculations
4. Missing or archived entities are never treated as zero-load placeholders
5. All existing tests continue to pass unchanged
6. Quantity wrappers are preserved in item-domain APIs and only unwrapped at the load-arithmetic boundary
7. Duplicate contained-entity inputs are rejected instead of double-counted
8. This ticket does not invent provisional containment semantics that E05 would later need to undo

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/load.rs` (inline `#[cfg(test)]` module) — weight table correctness, entity dispatch, invalid-entity handling, duplicate-membership rejection, container load sums, remaining capacity, and over-capacity invariant detection

### Commands

1. `cargo test -p worldwake-core load`
2. `cargo test -p worldwake-core`
3. `cargo clippy --workspace --all-targets -- -D warnings`
4. `cargo test --workspace`

## Outcome

- Completion date: 2026-03-09
- What actually changed:
  - Added `crates/worldwake-core/src/load.rs` with deterministic load tables for `CommodityKind` and `UniqueItemKind`
  - Added `load_of_lot`, `load_of_unique_item`, `load_of_entity`, `current_container_load`, and `remaining_container_capacity`
  - Re-exported the new module and helper functions from `crates/worldwake-core/src/lib.rs`
  - Added focused tests for weight tables, entity dispatch, missing/archived entity handling, duplicate contained-ID rejection, container aggregation, and over-capacity invariant detection
- Deviations from original plan:
  - Corrected the ticket away from the stale `ContainerData` name and aligned it to the existing `Container` component
  - Kept container aggregation explicitly non-recursive and caller-supplied because E05 still owns authoritative containment semantics
  - Strengthened the design so invalid entity references and duplicate contained IDs fail fast instead of being treated as zero-load or double-counted input
- Verification results:
  - `cargo test -p worldwake-core load` passed
  - `cargo test -p worldwake-core` passed
  - `cargo clippy --workspace --all-targets -- -D warnings` passed
  - `cargo test --workspace` passed
