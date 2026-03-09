# E04ITECON-005: Container ECS component

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — component_schema, component_tables, world API
**Deps**: E04ITECON-001 (CommodityKind for `allowed_commodities`)

## Problem

Containers enforce capacity limits and commodity restrictions. Without a `Container` component, there is no way to model inventories, storage crates, or agent carrying capacity with deterministic type-safe accounting.

## Assumption Reassessment (2026-03-09)

1. `EntityKind::Container` already exists in `entity.rs` — confirmed
2. `LoadUnits` newtype already exists in `numerics.rs` — confirmed
3. `BTreeSet` is available and required for deterministic sets — confirmed
4. The macro-driven component registration is established — confirmed

## Architecture Check

1. Same macro registration pattern as other components
2. `allowed_commodities` uses `Option<BTreeSet<CommodityKind>>` — `None` means "accept all"
3. Container component is attached to `EntityKind::Container` entities only

## What to Change

### 1. Define `ContainerData` struct in `items.rs`

```rust
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ContainerData {
    pub capacity: LoadUnits,
    pub allowed_commodities: Option<BTreeSet<CommodityKind>>,
    pub allows_unique_items: bool,
    pub allows_nested_containers: bool,
}

impl Component for ContainerData {}
```

Named `ContainerData` (not `Container`) to avoid collision with `EntityKind::Container`.

### 2. Register in `component_schema.rs`

Add `ContainerData` entry with:
- field name: `containers`
- kind check: `|kind| kind == EntityKind::Container`

### 3. Update `component_tables.rs` imports

Add `ContainerData` import.

### 4. Update `lib.rs` re-exports

Re-export `ContainerData`.

### 5. Add `World::create_container` factory

Convenience method that creates an `EntityKind::Container` entity and attaches a `ContainerData` component. Capacity must be > 0.

## Files to Touch

- `crates/worldwake-core/src/items.rs` (modify)
- `crates/worldwake-core/src/component_schema.rs` (modify)
- `crates/worldwake-core/src/component_tables.rs` (modify)
- `crates/worldwake-core/src/world.rs` (modify — add factory)
- `crates/worldwake-core/src/lib.rs` (modify — add re-exports)

## Out of Scope

- Load accounting functions (E04ITECON-007)
- Containment relations / `contained_by` (E05)
- Containment cycle detection (E05 — invariant 9.18)
- Conservation verification (E04ITECON-008)
- Nested container load recursion (E04ITECON-007)
- Physical placement (E05)

## Acceptance Criteria

### Tests That Must Pass

1. `ContainerData` with `allowed_commodities: None` bincode round-trips
2. `ContainerData` with `allowed_commodities: Some(BTreeSet)` bincode round-trips with deterministic order
3. `ContainerData` satisfies `Clone + Debug + Eq + Serialize + DeserializeOwned + Component`
4. `World::create_container(capacity, tick)` produces alive entity with kind `Container`
5. Creating a container with `LoadUnits(0)` capacity returns error
6. Inserting `ContainerData` on a non-`Container` entity kind returns error
7. `query_container_data()` returns only live container entities
8. Existing suite: `cargo test -p worldwake-core`

### Invariants

1. `allowed_commodities` uses `BTreeSet`, never `HashSet`
2. `ContainerData` can only be attached to `EntityKind::Container` entities
3. Capacity is measured in `LoadUnits`, not raw integers
4. All existing tests continue to pass unchanged

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/items.rs` — `ContainerData` bincode round-trip, trait bounds
2. `crates/worldwake-core/src/world.rs` — factory tests, kind-check tests, query tests

### Commands

1. `cargo test -p worldwake-core`
2. `cargo clippy --workspace && cargo test --workspace`
