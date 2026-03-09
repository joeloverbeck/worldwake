# E04ITECON-003: ItemLot ECS component

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — component_schema, component_tables, world API
**Deps**: E04ITECON-001 (CommodityKind), E04ITECON-002 (ProvenanceEntry)

## Problem

Stackable commodity lots need to be first-class ECS components so the world can store, query, and iterate over them. This requires registering `ItemLot` in the macro-driven component schema alongside `Name` and `AgentData`.

## Assumption Reassessment (2026-03-09)

1. `component_schema.rs` uses `with_authoritative_components!` macro to declare component columns — confirmed
2. `component_tables.rs` generates `BTreeMap<EntityId, T>` storage via the same macro — confirmed
3. `world.rs` generates typed accessors via `world_component_api!` macro — confirmed
4. `EntityKind::ItemLot` already exists in `entity.rs` — confirmed
5. Kind-check closures in the schema restrict which entity kinds can hold which components — confirmed

## Architecture Check

1. Follows the exact same macro pattern used for `Name` and `AgentData` — no new abstractions
2. `ItemLot` is restricted to `EntityKind::ItemLot` entities via the kind-check closure
3. Lot amounts should use `Quantity` so conserved inventory cannot silently degrade back into ad hoc `u32` arithmetic

## What to Change

### 1. Define `ItemLot` struct in `items.rs`

```rust
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ItemLot {
    pub commodity: CommodityKind,
    pub quantity: Quantity,
    pub provenance: Vec<ProvenanceEntry>,
}

impl Component for ItemLot {}
```

### 2. Register in `component_schema.rs`

Add an `ItemLot` entry to `with_authoritative_components!` with:
- field name: `item_lots`
- kind check: `|kind| kind == EntityKind::ItemLot`

### 3. Update `component_tables.rs` imports

Add `ItemLot` to the import from `components` (or `items` depending on module layout).

### 4. Update `lib.rs` re-exports

Re-export `ItemLot`.

### 5. Add `World::create_item_lot` factory

Convenience method that creates an `EntityKind::ItemLot` entity and attaches an `ItemLot` component with an initial `Created` provenance entry. Must validate `quantity > 0`.

## Files to Touch

- `crates/worldwake-core/src/items.rs` (modify — add `ItemLot` struct)
- `crates/worldwake-core/src/component_schema.rs` (modify — add entry)
- `crates/worldwake-core/src/component_tables.rs` (modify — add import)
- `crates/worldwake-core/src/world.rs` (modify — add factory method)
- `crates/worldwake-core/src/lib.rs` (modify — add re-export)

## Out of Scope

- Split/merge operations (E04ITECON-006)
- Load accounting (E04ITECON-007)
- Conservation verification (E04ITECON-008)
- Container capacity checks
- `UniqueItem` component (E04ITECON-004)
- Physical placement / `located_in` relations (E05)

## Acceptance Criteria

### Tests That Must Pass

1. `World::create_item_lot(CommodityKind::Apple, Quantity(10), tick)` produces an alive entity with kind `ItemLot`
2. Created lot has `quantity == Quantity(10)` and exactly one `ProvenanceEntry` with `LotOperation::Created`
3. `create_item_lot` with `quantity == Quantity(0)` returns an error
4. Inserting `ItemLot` on a non-`ItemLot` entity kind returns `WorldError::InvalidOperation`
5. `query_item_lot()` returns only live `ItemLot` entities
6. `ItemLot` component bincode round-trips correctly
7. Duplicate `ItemLot` insertion returns `WorldError::DuplicateComponent`
8. `remove_all` cleans up `ItemLot` components
9. Existing suite: `cargo test -p worldwake-core`

### Invariants

1. Live lots always have `quantity > 0` (enforced at creation; zero-quantity archival is in E04ITECON-006)
2. Provenance entries are append-only (Vec only grows)
3. `ItemLot` can only be attached to `EntityKind::ItemLot` entities
4. All existing tests continue to pass unchanged
5. Conserved lot counts use `Quantity`, not raw integers

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/items.rs` — `ItemLot` bincode round-trip, trait bounds
2. `crates/worldwake-core/src/world.rs` — factory method tests, kind-check tests, query tests

### Commands

1. `cargo test -p worldwake-core`
2. `cargo clippy --workspace && cargo test --workspace`
