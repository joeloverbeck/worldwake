# E04ITECON-005: Container ECS component

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — component_schema, component_tables, world API
**Deps**: E04ITECON-001 (CommodityKind), E04ITECON-004 (UniqueItem component exists), E04ITECON-007 (load accounting remains separate)

## Problem

Phase 1 needs first-class container policy state in the ECS so later containment and load-accounting work has a deterministic, typed place to attach capacity and admission rules. Without a `Container` component, `EntityKind::Container` is only a tag with no authoritative storage semantics behind it.

## Assumption Reassessment (2026-03-09)

1. `EntityKind::Container` already exists in `crates/worldwake-core/src/entity.rs` — confirmed
2. `LoadUnits` already exists in `crates/worldwake-core/src/numerics.rs` — confirmed
3. `component_schema.rs` is the single authoritative declaration point for ECS components and already expands into both `ComponentTables` and `World` APIs — confirmed
4. `world.rs` already uses `create_entity_with(...)` plus focused public factories (`create_agent`, `create_office`, `create_faction`, `create_item_lot`, `create_unique_item`) for non-topological entities — confirmed
5. The current world/component tests already cover duplicate-component rejection, kind enforcement, live-only queries, purge cleanup, rollback on factory failure, and bincode round-trips for existing components; this ticket should extend those patterns instead of inventing a parallel path — confirmed
6. The spec names the component `Container`, not `ContainerData`, and there is no Rust namespace collision with `EntityKind::Container` that would justify introducing an alias-type name — confirmed
7. E04 does not yet provide containment relations or container load calculation. This ticket can define container policy state, but it cannot truthfully claim to enforce inventory or carrying-capacity behavior on its own — confirmed

## Architecture Check

1. The component should be named `Container` to match the spec and keep the domain vocabulary clean. `ContainerData` would be an unnecessary alias that weakens the architecture instead of improving it.
2. `Container` belongs in `items.rs` beside `CommodityKind`, `ItemLot`, and `UniqueItem`, because it is item-domain inventory policy state. Moving it into `components.rs` would fragment the item model.
3. ECS registration must reuse the existing macro schema path used by all other authoritative components. A one-off storage path would duplicate invariants and erode the current architecture.
4. The world-level factory is still beneficial, but it should not invent hidden defaults for `allowed_commodities`, `allows_unique_items`, or `allows_nested_containers`. The canonical constructor must accept the full policy state so callers make those rules explicit.
5. This ticket should stay focused on defining and storing container policy. It should not pretend to deliver actual containment semantics, recursive load computation, or placement rules before E04ITECON-007 and E05 land.

## What to Change

### 1. Define `Container` in `items.rs`

```rust
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Container {
    pub capacity: LoadUnits,
    pub allowed_commodities: Option<BTreeSet<CommodityKind>>,
    pub allows_unique_items: bool,
    pub allows_nested_containers: bool,
}

impl Component for Container {}
```

Notes:
- use `BTreeSet`, never `HashSet`
- do not introduce `ContainerData`, `ContainerSpec`, or other alias names

### 2. Register in `component_schema.rs`

Add a `Container` entry to `with_authoritative_components!` with:
- field name: `containers`
- kind check: `|kind| kind == EntityKind::Container`

### 3. Update `component_tables.rs` imports

Import `Container` from `items`.

### 4. Update `lib.rs` re-exports

Re-export `Container`.

### 5. Add `World::create_container` factory

Add a convenience method that creates an `EntityKind::Container` entity and attaches a `Container` component.

Preferred signature:

```rust
pub fn create_container(
    &mut self,
    container: Container,
    tick: Tick,
) -> Result<EntityId, WorldError>
```

Rules:
- reject `Container { capacity: LoadUnits(0), .. }`
- build through `create_entity_with(...)` so rollback behavior stays consistent with the rest of `World`
- keep policy explicit; do not add hidden defaults or compatibility shims

## Files to Touch

- `crates/worldwake-core/src/items.rs` (modify — add `Container`)
- `crates/worldwake-core/src/component_schema.rs` (modify — add entry)
- `crates/worldwake-core/src/component_tables.rs` (modify — add import)
- `crates/worldwake-core/src/world.rs` (modify — add factory and tests)
- `crates/worldwake-core/src/lib.rs` (modify — add re-export)

## Out of Scope

- Load accounting functions (E04ITECON-007)
- Enforcement of current vs remaining container capacity
- Containment relations / `contained_by` or `located_in` semantics (E05)
- Containment cycle detection (E05)
- Physical placement
- Trade or pricing behavior

## Acceptance Criteria

### Tests That Must Pass

1. `Container` with `allowed_commodities: None` bincode round-trips
2. `Container` with `allowed_commodities: Some(BTreeSet)` bincode round-trips with deterministic order
3. `Container` satisfies `Clone + Debug + Eq + Serialize + DeserializeOwned + Component`
4. `World::create_container(container, tick)` produces an alive entity with kind `Container`
5. Creating a container with `LoadUnits(0)` returns an error and does not leave an allocated entity behind
6. Inserting `Container` on a non-`Container` entity kind returns `WorldError::InvalidOperation`
7. Duplicate `Container` insertion returns `WorldError::DuplicateComponent`
8. `query_container()` returns only live container entities
9. Purging an archived container entity removes its `Container` component and leaves no stale query results
10. `ComponentTables::remove_all` clears `Container` storage alongside existing component columns
11. Existing suite: `cargo test -p worldwake-core`

### Invariants

1. `allowed_commodities` uses `BTreeSet`, never `HashSet`
2. `Container` can only be attached to `EntityKind::Container` entities
3. Capacity is measured in `LoadUnits`, not raw integers
4. Container policy is explicit at creation time; the factory does not invent defaults for admission rules
5. All existing tests continue to pass unchanged

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/items.rs` — `Container` component trait bounds, bincode round-trips, deterministic set serialization
2. `crates/worldwake-core/src/component_tables.rs` — generated `Container` storage CRUD plus `remove_all` coverage
3. `crates/worldwake-core/src/world.rs` — factory behavior, zero-capacity rejection, kind-check rejection, duplicate-component rejection, live-only query behavior, purge cleanup, rollback behavior

### Commands

1. `cargo test -p worldwake-core container`
2. `cargo test -p worldwake-core`
3. `cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`

## Outcome

- Completion date: 2026-03-09
- What actually changed:
  - Added `Container` to `crates/worldwake-core/src/items.rs` as the authoritative container-policy component with deterministic `BTreeSet` commodity restrictions
  - Registered `Container` in the shared component schema so `ComponentTables` and `World` gained generated typed CRUD/query/count APIs without introducing a parallel storage path
  - Added `World::create_container(container, tick)` using the existing `create_entity_with(...)` factory path and explicit zero-capacity validation
  - Re-exported `Container` from `worldwake-core::lib`
  - Strengthened tests across `items.rs`, `component_tables.rs`, and `world.rs` for trait bounds, bincode round-trips, deterministic set serialization, kind enforcement, duplicate rejection, live-only queries, purge cleanup, and world serialization coverage
- Deviations from original plan:
  - Replaced the proposed `ContainerData` alias with `Container` to match the spec and avoid introducing an unnecessary second name for the same domain concept
  - Tightened the factory design so callers provide the full `Container` policy explicitly rather than relying on hidden defaults for admission flags or allowed commodities
  - Kept the scope strictly on ECS/container-policy storage; no containment semantics or load-accounting behavior were added here because those belong to E04ITECON-007 and E05
- Verification results:
  - `cargo test -p worldwake-core container` passed
  - `cargo test -p worldwake-core` passed
  - `cargo clippy --workspace --all-targets -- -D warnings` passed
  - `cargo test --workspace` passed
