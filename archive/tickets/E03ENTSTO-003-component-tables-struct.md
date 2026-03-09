# E03ENTSTO-003: ComponentTables Struct for Non-Topological Components

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: None
**Deps**: E03ENTSTO-001 (EntityKind, EntityMeta), E01 (EntityId, Tick, Quantity, ControlSource)

## Problem

E03 requires explicit typed component tables: one `BTreeMap<EntityId, T>` per component type, grouped in a `ComponentTables` struct. No `TypeId`, `Any`, or trait-object storage. Internal macros are allowed only to remove boilerplate; the resulting storage and APIs must remain explicit and typed.

The original ticket assumed `Place` should immediately move into `ComponentTables`. That does not match the current codebase. In `worldwake-core` today, `Topology` is already the authoritative owner of `Place` payloads and graph connectivity. Adding a second `places: BTreeMap<EntityId, Place>` table now would create duplicate authoritative state and force follow-on tickets to reconcile two mutation surfaces.

This ticket therefore establishes the component-table foundation only for non-topological Phase 1 components that are not already owned elsewhere. `Place` stays in `Topology` unless a later ticket explicitly extracts place payloads out of topology as part of a broader architectural refactor.

## What to Change

## Assumptions Reassessed

1. `Place` already exists and already implements `Component` in [`crates/worldwake-core/src/topology.rs`](../crates/worldwake-core/src/topology.rs).
2. `Topology` currently stores `places: BTreeMap<EntityId, Place>` and is the sole authoritative place store.
3. Duplicating `Place` into `ComponentTables` in this ticket would be a design regression, not progress.
4. The immediate architectural need is a clean typed table foundation for components that are not embedded in topology.

## What to Change

### 1. Define initial component types

Create `crates/worldwake-core/src/components.rs` with the Phase 1 component structs that do not already exist elsewhere:

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

`Place` is intentionally out of scope for this ticket because topology already owns it. Additional non-topological component types can be added by later E03/E04/E05 tickets once their authoritative home is clear.

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
    // Future non-topological tables added by later E03/E04/E05 tickets:
    // pub(crate) containers: BTreeMap<EntityId, ContainerData>,
    // pub(crate) item_lots: BTreeMap<EntityId, ItemLotData>,
    // pub(crate) unique_items: BTreeMap<EntityId, UniqueItemData>,
    // pub(crate) offices: BTreeMap<EntityId, OfficeData>,
    // pub(crate) facilities: BTreeMap<EntityId, FacilityData>,
}
```

Do not add a `places` table in this ticket.

### 3. Internal macro for per-table accessors

Define an internal macro (for example `component_table_methods!`) that generates explicit typed methods for each table:

- `insert_*`
- `get_*`
- `get_*_mut`
- `remove_*`
- `has_*`
- `iter_*`

The macro is internal only. It must not introduce a generic runtime registry, trait-object dispatch, or `TypeId`-based lookups.

### 4. Implement `remove_all`

A method `ComponentTables::remove_all(&mut self, entity: EntityId)` that removes the entity from every registered table. This is the only table-wide cleanup behavior in scope.

### 5. Register modules in `lib.rs`

Add `pub mod components;` and `pub mod component_tables;` with appropriate re-exports.

## Files to Touch

- `crates/worldwake-core/src/components.rs` (new)
- `crates/worldwake-core/src/component_tables.rs` (new)
- `crates/worldwake-core/src/lib.rs` (modify — add modules + re-exports)

## Out of Scope

- Moving `Place` out of `Topology` or introducing a second authoritative place store.
- Component CRUD on `World` — that's E03ENTSTO-004 (wraps `ComponentTables` with entity validation).
- Query API — E03ENTSTO-005.
- World struct assembly — E03ENTSTO-006.
- E04 component types (containers, item lots, unique items) — added by E04 tickets.
- E05 component types (relations, ownership) — added by E05 tickets.
- Event journaling hooks — E06.

## Acceptance Criteria

### Tests That Must Pass

1. **Name component**: implements `Component` trait bounds.
2. **AgentData component**: implements `Component` trait bounds.
3. **ComponentTables default**: `ComponentTables::default()` creates empty registered tables.
4. **Insert + get round-trip**: insert a `Name` for an entity, get it back by id.
5. **Second table round-trip**: insert an `AgentData` for an entity, get it back by id.
6. **Remove returns value**: remove returns `Some(T)` and subsequent get returns `None`.
7. **has returns correct**: `has` is true after insert, false after remove.
8. **iter deterministic**: inserting entities in random order, iterating yields sorted `EntityId` order.
9. **remove_all cleans all registered tables**: after `remove_all`, entity has no `Name` or `AgentData`.
10. **Serialization**: `ComponentTables` with populated data round-trips via bincode.
11. **No type-erased store**: no `TypeId`, `Any`, or trait-object dispatch is introduced in authoritative component-table code.
12. **Existing suite**: `cargo test -p worldwake-core` continues green.

### Invariants

1. All registered storage uses `BTreeMap<EntityId, T>` for deterministic iteration.
2. No `HashMap`, `HashSet`, `TypeId`, `Any`, or trait-object storage is introduced in authoritative component-table state.
3. Fields are `pub(crate)` so later `World` APIs can own the mutation surface.
4. Each table stores one concrete type with explicit typed methods.
5. `Topology` remains the sole authoritative owner of `Place` until a later refactor says otherwise.

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

## Outcome

- **Completed**: 2026-03-09
- **What actually changed**:
  - Added `components.rs` with explicit `Name` and `AgentData` component types.
  - Added `component_tables.rs` with deterministic `BTreeMap<EntityId, T>` storage for `Name` and `AgentData`.
  - Added explicit typed accessor methods generated by an internal macro plus `remove_all`.
  - Registered the new modules and re-exports in `lib.rs`.
- **Deviation from original plan**:
  - Did not add a `places` table. `Topology` already owns authoritative `Place` payloads, so duplicating `Place` in `ComponentTables` would have created two sources of truth.
  - Kept this ticket focused on non-topological components only.
- **Verification results**:
  - `cargo test -p worldwake-core components`
  - `cargo test -p worldwake-core component_tables`
  - `cargo test -p worldwake-core`
  - `cargo clippy --workspace --all-targets -- -D warnings`
  - `cargo test --workspace`
