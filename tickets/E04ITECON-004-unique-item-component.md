# E04ITECON-004: UniqueItemKind enum and UniqueItem ECS component

**Status**: PENDING
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
4. The macro-driven component registration pattern is established — confirmed

## Architecture Check

1. Same macro registration pattern as `Name`, `AgentData`, `ItemLot`
2. `metadata` uses `BTreeMap` (never `HashMap`) per deterministic data policy

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
5. `World::create_unique_item(Weapon, Some("Rusty Sword"), metadata, tick)` produces alive entity with kind `UniqueItem`
6. Inserting `UniqueItem` on a non-`UniqueItem` entity kind returns error
7. `metadata` serialization is deterministic (same keys produce same bytes)
8. Weapons are `UniqueItem` entities, not `ItemLot` (spec 3.6 enforcement)
9. Existing suite: `cargo test -p worldwake-core`

### Invariants

1. `metadata` uses `BTreeMap`, never `HashMap` — enforced by type definition
2. Unique items are indivisible — no split/merge operations
3. `UniqueItem` can only be attached to `EntityKind::UniqueItem` entities
4. All existing tests continue to pass unchanged

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/items.rs` — `UniqueItemKind` and `UniqueItem` bincode round-trips, trait bounds, metadata determinism
2. `crates/worldwake-core/src/world.rs` — factory tests, kind-check tests

### Commands

1. `cargo test -p worldwake-core`
2. `cargo clippy --workspace && cargo test --workspace`
