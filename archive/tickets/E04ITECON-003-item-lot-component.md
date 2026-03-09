# E04ITECON-003: ItemLot ECS component

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — component_schema, component_tables, world API
**Deps**: E04ITECON-001 (CommodityKind), E04ITECON-002 (ProvenanceEntry)

## Problem

Stackable commodity lots need to be first-class ECS components so the world can store, query, and iterate over them. This requires registering `ItemLot` in the macro-driven component schema alongside `Name` and `AgentData`.

## Assumption Reassessment (2026-03-09)

1. `CommodityKind`, `LotOperation`, and `ProvenanceEntry` already exist in `crates/worldwake-core/src/items.rs` from completed tickets E04ITECON-001 and E04ITECON-002 — confirmed
2. `component_schema.rs` uses the shared `with_authoritative_components!` macro to declare every authoritative component once, then expands that schema into both `ComponentTables` and `World` APIs — confirmed
3. `component_tables.rs` stores authoritative components in deterministic `BTreeMap<EntityId, T>` columns generated from that schema — confirmed
4. `world.rs` already has a stable factory pattern via `create_entity_with`, plus generated typed component CRUD/query helpers from `world_component_api!` — confirmed
5. `EntityKind::ItemLot` already exists in `entity.rs`, but no `ItemLot` component, schema entry, or lot factory exists yet — confirmed
6. The current world tests already cover duplicate-component errors, archived-entity rejection, live-only queries, purge cleanup, and bincode round-trips for existing components; this ticket should extend those patterns instead of inventing new ones — confirmed

## Architecture Check

1. `ItemLot` should live in `items.rs`, not `components.rs`, because it is item-domain state composed directly from existing item-domain types (`CommodityKind`, `Quantity`, `ProvenanceEntry`) and will be reused by later lot algebra and load-accounting work
2. ECS storage should still be registered through the existing macro schema used by `Name` and `AgentData`; adding a one-off storage path would weaken the architecture and duplicate invariants
3. `ItemLot` must be restricted to `EntityKind::ItemLot` entities via the schema kind-check closure so the world keeps entity classification and attached component state aligned
4. The world-level factory is worth keeping because it centralizes the `quantity > 0` invariant and canonical `Created` provenance bootstrapping instead of pushing that responsibility to every caller
5. This ticket should not introduce split/merge helpers, alias types, or early container/location semantics; those belong to later Epic 04 and E05 tickets

## What to Change

### 1. Extend `items.rs` with `ItemLot`

```rust
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ItemLot {
    pub commodity: CommodityKind,
    pub quantity: Quantity,
    pub provenance: Vec<ProvenanceEntry>,
}

impl Component for ItemLot {}
```

Notes:
- reuse the existing `CommodityKind`, `Quantity`, and `ProvenanceEntry` types already defined in `items.rs`
- do not duplicate provenance constructors elsewhere just to make the factory compile

### 2. Register in `component_schema.rs`

Add an `ItemLot` entry to `with_authoritative_components!` with:
- field name: `item_lots`
- kind check: `|kind| kind == EntityKind::ItemLot`

### 3. Update `component_tables.rs` imports

Import `ItemLot` from `items`, not `components`.

### 4. Update `lib.rs` re-exports

Re-export `ItemLot`.

### 5. Add `World::create_item_lot` factory

Add a convenience method that creates an `EntityKind::ItemLot` entity and attaches an `ItemLot` component with one initial `Created` provenance entry:

```rust
ProvenanceEntry {
    tick,
    event_id: None,
    operation: LotOperation::Created,
    related_lot: None,
    amount: quantity,
}
```

Rules:
- reject `Quantity(0)` before inserting the component
- build the entity through `create_entity_with` so factory rollback behavior stays consistent with the rest of `World`
- keep the API minimal: commodity, quantity, tick; later tickets can add richer provenance/event hooks if needed

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
8. Purging an archived `ItemLot` entity removes its `ItemLot` component and leaves no stale query results
9. `ComponentTables::remove_all` clears `ItemLot` storage alongside existing component columns
10. Existing suite: `cargo test -p worldwake-core`

### Invariants

1. Live lots always have `quantity > 0` (enforced at creation; zero-quantity archival is in E04ITECON-006)
2. Provenance entries are append-only (Vec only grows)
3. `ItemLot` can only be attached to `EntityKind::ItemLot` entities
4. All existing tests continue to pass unchanged
5. Conserved lot counts use `Quantity`, not raw integers

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/items.rs` — `ItemLot` component trait bounds and bincode round-trip
2. `crates/worldwake-core/src/component_tables.rs` — `ItemLot` storage CRUD/removal coverage through the generated tables API
3. `crates/worldwake-core/src/world.rs` — factory tests, zero-quantity rejection, kind-check rejection, duplicate-component rejection, live-only query behavior, purge cleanup

### Commands

1. `cargo test -p worldwake-core item_lot`
2. `cargo test -p worldwake-core`
3. `cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`

## Outcome

- Outcome amended: 2026-03-09
- Completion date: 2026-03-09
- What actually changed:
  - Added `ItemLot` to `crates/worldwake-core/src/items.rs` as the item-domain lot component built from existing `CommodityKind`, `Quantity`, and `ProvenanceEntry` types
  - Registered `ItemLot` in the authoritative component schema so `ComponentTables` and `World` gained the generated typed CRUD/query/count APIs without introducing a parallel storage path
  - Added `World::create_item_lot(commodity, quantity, tick)` to centralize non-zero quantity validation and canonical `Created` provenance initialization
  - Re-exported `ItemLot` from `worldwake-core::lib`
  - Strengthened tests across `items.rs`, `component_tables.rs`, and `world.rs` for round-tripping, generated table storage, factory behavior, zero-quantity rejection, kind enforcement, duplicate insertion rejection, live-only queries, and purge cleanup
- Deviations from original plan:
  - Tightened the ticket scope during reassessment to reflect that the taxonomy and provenance dependencies were already completed; this work only added the ECS component and world integration layer
  - Kept `ItemLot` in `items.rs` rather than `components.rs` because it is domain state that composes existing item-domain types and will be reused by later lot algebra/load-accounting work
  - Reused the existing `create_entity_with` rollback path instead of introducing a special-case item-lot constructor path
  - The example `Created` provenance entry now uses `related_lot: None` after the provenance model was generalized from a one-way source field to a relationship-oriented field
- Verification results:
  - `cargo test -p worldwake-core item_lot` passed
  - `cargo test -p worldwake-core items` passed
  - `cargo test -p worldwake-core` passed
  - `cargo clippy --workspace --all-targets -- -D warnings` passed
  - `cargo test --workspace` passed
