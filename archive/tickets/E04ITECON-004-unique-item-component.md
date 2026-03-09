# E04ITECON-004: UniqueItemKind enum and UniqueItem ECS component

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — component_schema, component_tables, world API
**Deps**: E04ITECON-001 (items.rs module exists)

## Problem

Singular objects (weapons, contracts, artifacts) need unique identity distinct from stackable lots. Spec 3.6 explicitly requires weapons to be unique entities, not stackable lots. This ticket defines `UniqueItemKind`, the `UniqueItem` component, and registers it in the ECS.

## Assumption Reassessment (2026-03-09)

1. `EntityKind::UniqueItem` already exists in `entity.rs` — confirmed
2. Spec requires `BTreeMap<String, String>` for metadata (deterministic serialization) — confirmed
3. No existing `UniqueItem` or `UniqueItemKind` types — confirmed
4. `component_schema.rs` already defines authoritative ECS components once and expands that schema into both `ComponentTables` and `World` APIs — confirmed
5. `world.rs` already uses `create_entity_with(...)` plus focused public factories (`create_agent`, `create_office`, `create_faction`, `create_item_lot`) for non-topological entities — confirmed
6. The current tests already cover duplicate-component rejection, live-only queries, purge cleanup, rollback on factory failure, and bincode round-trips for existing components; this ticket should extend those patterns rather than invent a parallel path — confirmed

## Architecture Check

1. `UniqueItemKind` and `UniqueItem` belong in `items.rs` beside `CommodityKind`, `LotOperation`, `ProvenanceEntry`, and `ItemLot`; moving them into `components.rs` would split the item domain across modules for no gain
2. ECS registration should reuse the same macro schema path as `Name`, `AgentData`, and `ItemLot`; a one-off storage path would duplicate invariants and weaken the architecture
3. The world-level factory is beneficial because it centralizes canonical entity kind assignment and keeps string allocation at the boundary, but it should match existing `World` naming APIs by accepting `name: Option<&str>` and converting internally
4. `metadata` must use `BTreeMap` (never `HashMap`) per deterministic data policy
5. This ticket should stay focused on unique-item identity and ECS/world integration only; no load rules, pricing bridges, placement, ownership, or container semantics belong here

## What to Change

### 1. Add types to `items.rs`

```rust
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Serialize, Deserialize)]
pub enum UniqueItemKind {
    SimpleTool,
    Weapon,
    Contract,
    Artifact,
    OfficeInsignia,
    Misc,
}
```

```rust
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct UniqueItem {
    pub kind: UniqueItemKind,
    pub name: Option<String>,
    pub metadata: BTreeMap<String, String>,
}

impl Component for UniqueItem {}
```

### 2. Register in `component_schema.rs`

Add `UniqueItem` entry with:
- field name: `unique_items`
- kind check: `|kind| kind == EntityKind::UniqueItem`

### 3. Update `component_tables.rs` imports

Add `UniqueItem` import.

### 4. Update `lib.rs` re-exports

Re-export `UniqueItemKind` and `UniqueItem`.

### 5. Add `World::create_unique_item` factory

Convenience method that creates an `EntityKind::UniqueItem` entity and attaches a `UniqueItem` component.

Signature:

```rust
pub fn create_unique_item(
    &mut self,
    kind: UniqueItemKind,
    name: Option<&str>,
    metadata: BTreeMap<String, String>,
    tick: Tick,
) -> Result<EntityId, WorldError>
```

Rules:
- build through `create_entity_with(...)` so rollback behavior stays consistent with existing factories
- convert `Option<&str>` to `Option<String>` inside the factory to match the existing `create_agent` / `create_office` / `create_faction` boundary style
- do not introduce alias enums, compatibility shims, or a parallel item-construction path

## Files to Touch

- `crates/worldwake-core/src/items.rs` (modify)
- `crates/worldwake-core/src/component_schema.rs` (modify)
- `crates/worldwake-core/src/component_tables.rs` (modify)
- `crates/worldwake-core/src/world.rs` (modify — add factory)
- `crates/worldwake-core/src/lib.rs` (modify — add re-exports)

## Out of Scope

- Load weight of unique items (E04ITECON-007)
- Physical placement / ownership relations (E05)
- `ItemLot` component (E04ITECON-003 — may be done in parallel)
- Container component (E04ITECON-005)
- Trade pricing or economic value

## Acceptance Criteria

### Tests That Must Pass

1. All 6 `UniqueItemKind` variants bincode round-trip
2. `UniqueItemKind` satisfies `Copy + Clone + Eq + Ord + Hash + Debug + Serialize + DeserializeOwned`
3. `UniqueItem` with populated `metadata` bincode round-trips with deterministic key order
4. `UniqueItem` with empty metadata and `name: None` bincode round-trips
5. `World::create_unique_item(Weapon, Some("Rusty Sword"), metadata, tick)` produces alive entity with kind `UniqueItem` and a matching `UniqueItem` component
6. Inserting `UniqueItem` on a non-`UniqueItem` entity kind returns error
7. Duplicate `UniqueItem` insertion returns `WorldError::DuplicateComponent`
8. `query_unique_item()` returns only live `UniqueItem` entities
9. Purging an archived `UniqueItem` entity removes its `UniqueItem` component and leaves no stale query results
10. `ComponentTables::remove_all` clears `UniqueItem` storage alongside existing component columns
11. `metadata` serialization is deterministic (same keys produce same bytes)
12. Weapons are `UniqueItem` entities, not `ItemLot` (spec 3.6 enforcement)
13. Existing suite: `cargo test -p worldwake-core`

### Invariants

1. `metadata` uses `BTreeMap`, never `HashMap` — enforced by type definition
2. Unique items are indivisible — no split/merge operations
3. `UniqueItem` can only be attached to `EntityKind::UniqueItem` entities
4. `World::create_unique_item` is the canonical constructor for newly created unique-item entities in core
5. All existing tests continue to pass unchanged

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/items.rs` — `UniqueItemKind` and `UniqueItem` trait bounds, canonical variant list, bincode round-trips, ordering, metadata determinism
2. `crates/worldwake-core/src/component_tables.rs` — generated `UniqueItem` storage CRUD plus `remove_all` coverage
3. `crates/worldwake-core/src/world.rs` — factory behavior, duplicate rejection, kind-check rejection, live-only query behavior, purge cleanup, world bincode coverage

### Commands

1. `cargo test -p worldwake-core unique_item`
2. `cargo test -p worldwake-core`
3. `cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`

## Outcome

- Completion date: 2026-03-09
- What actually changed:
  - Added `UniqueItemKind` and `UniqueItem` to `crates/worldwake-core/src/items.rs`
  - Added canonical `UniqueItemKind::ALL` coverage plus inline tests for trait bounds, ordering, bincode round-trips, empty-name/empty-metadata handling, and deterministic metadata serialization
  - Registered `UniqueItem` in the authoritative component schema so `ComponentTables` and `World` gained generated typed CRUD/query/count APIs
  - Added `World::create_unique_item(kind, name: Option<&str>, metadata, tick)` using the existing `create_entity_with(...)` factory path
  - Re-exported `UniqueItemKind` and `UniqueItem` from `worldwake-core::lib`
  - Extended `component_tables.rs` and `world.rs` tests for generated storage access, duplicate rejection, kind enforcement, live-only queries, purge cleanup, and world bincode round-trips
- Deviations from original plan:
  - Tightened the ticket to match the real architecture: this work extends the existing macro-schema plus factory pattern instead of introducing any new storage or construction path
  - The factory now accepts `Option<&str>` rather than owned `Option<String>` so it matches the existing `World` factory boundary style and keeps allocation inside the world API
  - Added explicit `ComponentTables` test coverage because component registration changes are not fully validated by world-level tests alone
- Verification results:
  - `cargo test -p worldwake-core unique_item` passed
  - `cargo test -p worldwake-core` passed
  - `cargo clippy --workspace --all-targets -- -D warnings` passed
  - `cargo test --workspace` passed
