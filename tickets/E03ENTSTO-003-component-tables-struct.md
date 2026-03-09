# E03ENTSTO-003: ComponentTables Struct and Table Macro

**Status**: TODO
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: None
**Deps**: E03ENTSTO-001 (EntityKind, EntityMeta), E01 (EntityId, Tick, Quantity, ControlSource)

## Problem

E03 requires explicit typed component tables — one `BTreeMap<EntityId, T>` per component type, grouped in a `ComponentTables` struct. No `TypeId`, `Any`, or trait-object storage. An internal macro may reduce boilerplate but the resulting storage must be explicit and typed.

This ticket creates the `ComponentTables` struct with the initial set of Phase 1 component types and a helper macro for per-table accessors.

## What to Change

### 1. Define initial component types

Create `crates/worldwake-core/src/components.rs` with the Phase 1 component structs that don't already exist elsewhere:

```rust
/// Human-readable name for any named entity.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Name(pub String);
impl Component for Name {}

/// Agent-specific data attached to Agent entities.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AgentData {
    pub control_source: ControlSource,
}
impl Component for AgentData {}
```

Other component types (Place already exists in topology.rs) will be added by E04/E05 tickets. This ticket only adds what's needed to make `ComponentTables` compilable and testable.

### 2. Create `ComponentTables` struct

In `crates/worldwake-core/src/component_tables.rs`:

```rust
/// Explicit typed component storage for all authoritative components.
///
/// Each field is a `BTreeMap<EntityId, T>` for deterministic iteration.
/// No `TypeId`, `Any`, or trait-object storage.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ComponentTables {
    pub(crate) names: BTreeMap<EntityId, Name>,
    pub(crate) agents: BTreeMap<EntityId, AgentData>,
    pub(crate) places: BTreeMap<EntityId, Place>,
    // Future tables added by E04/E05:
    // pub(crate) containers: BTreeMap<EntityId, ContainerData>,
    // pub(crate) item_lots: BTreeMap<EntityId, ItemLotData>,
    // pub(crate) unique_items: BTreeMap<EntityId, UniqueItemData>,
    // pub(crate) offices: BTreeMap<EntityId, OfficeData>,
    // pub(crate) facilities: BTreeMap<EntityId, FacilityData>,
}
```

### 3. Internal macro for per-table accessors

Define a macro (e.g., `component_accessors!`) that generates typed `insert`, `get`, `get_mut`, `remove`, `has`, and `iter` methods for each table. The macro is internal — it produces explicit typed methods, not generic runtime dispatch.

### 4. Implement `remove_all_components`

A method `ComponentTables::remove_all(&mut self, entity: EntityId)` that removes the entity from every table. Needed for entity purge.

### 5. Register modules in `lib.rs`

Add `pub mod components;` and `pub mod component_tables;` with appropriate re-exports.

## Files to Touch

- `crates/worldwake-core/src/components.rs` (new)
- `crates/worldwake-core/src/component_tables.rs` (new)
- `crates/worldwake-core/src/lib.rs` (modify — add modules + re-exports)

## Out of Scope

- Component CRUD on `World` — that's E03ENTSTO-004 (wraps ComponentTables with entity validation).
- Query API — E03ENTSTO-005.
- World struct assembly — E03ENTSTO-006.
- E04 component types (containers, item lots, unique items) — added by E04 tickets.
- E05 component types (relations, ownership) — added by E05 tickets.
- Event journaling hooks — E06.

## Acceptance Criteria

### Tests That Must Pass

1. **Name component**: implements `Component` trait bounds.
2. **AgentData component**: implements `Component` trait bounds.
3. **ComponentTables default**: `ComponentTables::default()` creates empty tables.
4. **Insert + get round-trip**: insert a `Name` for an entity, get it back by id.
5. **Remove returns value**: remove returns `Some(T)` and subsequent get returns `None`.
6. **has returns correct**: `has` is true after insert, false after remove.
7. **iter deterministic**: inserting entities in random order, iterating yields sorted EntityId order.
8. **remove_all cleans all tables**: after `remove_all`, entity has no components in any table.
9. **Serialization**: `ComponentTables` with populated data round-trips via bincode.
10. **No TypeId/Any**: source file does not contain `TypeId`, `Any`, or `dyn` in non-test code.
11. **Existing suite**: `cargo test -p worldwake-core` continues green.

### Invariants

1. All storage is `BTreeMap<EntityId, T>` — deterministic iteration.
2. No `HashMap`, `HashSet`, `TypeId`, `Any`, or trait-object storage in authoritative state.
3. Fields are `pub(crate)` — no direct external mutation of component tables.
4. Each table stores one concrete type — fully explicit and typed.

## Test Plan

### New Tests

In `crates/worldwake-core/src/components.rs`:
- `name_component_bounds`
- `agent_data_component_bounds`
- `name_bincode_roundtrip`
- `agent_data_bincode_roundtrip`

In `crates/worldwake-core/src/component_tables.rs`:
- `default_tables_are_empty`
- `insert_and_get_name`
- `insert_and_get_agent_data`
- `remove_returns_value`
- `has_component_correct`
- `iter_deterministic_order`
- `remove_all_clears_entity`
- `component_tables_bincode_roundtrip`

### Commands

```bash
cargo test -p worldwake-core components
cargo test -p worldwake-core component_tables
cargo clippy --workspace && cargo test --workspace
```
