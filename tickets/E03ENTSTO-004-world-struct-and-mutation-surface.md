# E03ENTSTO-004: World Struct and Mutation Surface

**Status**: TODO
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: None
**Deps**: E03ENTSTO-002 (EntityAllocator), E03ENTSTO-003 (ComponentTables), E02 (Topology)

## Problem

The `World` struct is the authoritative model that owns entity metadata, component tables, and topology. All fields must be private — no direct external mutation. Every persistent mutation path must be narrow enough to be wrapped by a journal in E06.

This ticket assembles the World struct, wires the allocator and component tables, and provides the controlled mutation API for entity lifecycle and component CRUD.

## What to Change

### 1. New module `world.rs` in `worldwake-core/src/`

```rust
/// The authoritative simulation world.
///
/// All fields are private. External code accesses state through
/// typed read methods and controlled mutation methods.
pub struct World {
    allocator: EntityAllocator,
    components: ComponentTables,
    topology: Topology,
}
```

### 2. Entity lifecycle methods

- `create_entity(&mut self, kind: EntityKind, tick: Tick) -> EntityId`
- `archive_entity(&mut self, id: EntityId, tick: Tick) -> Result<(), WorldError>`
- `purge_entity(&mut self, id: EntityId) -> Result<(), WorldError>` — also calls `components.remove_all(id)`
- `is_alive(&self, id: EntityId) -> bool`
- `is_archived(&self, id: EntityId) -> bool`
- `entity_meta(&self, id: EntityId) -> Option<&EntityMeta>`
- `entity_kind(&self, id: EntityId) -> Option<EntityKind>` — convenience

### 3. Component CRUD methods (validated)

These wrap `ComponentTables` with entity existence checks:

- `insert_component_name(&mut self, id: EntityId, name: Name) -> Result<(), WorldError>` — errors if entity not alive or already has component.
- `get_component_name(&self, id: EntityId) -> Option<&Name>` — returns None if entity dead/archived or missing component.
- `get_component_name_mut(&mut self, id: EntityId) -> Option<&mut Name>` — restricted mutable access.
- `remove_component_name(&mut self, id: EntityId) -> Result<Option<Name>, WorldError>` — errors if entity not found.
- `has_component_name(&self, id: EntityId) -> bool`

Same pattern for each component type (agents, places). Use the internal macro from E03ENTSTO-003 to generate these.

### 4. Read-only topology access

- `topology(&self) -> &Topology` — read-only reference to the topology.

### 5. Constructor

- `World::new(topology: Topology) -> Self` — creates world with given topology and empty allocator/tables.

### 6. Register module in `lib.rs`

Add `pub mod world;` and re-export `World`.

## Files to Touch

- `crates/worldwake-core/src/world.rs` (new)
- `crates/worldwake-core/src/lib.rs` (modify — add module + re-exports)

## Out of Scope

- Query API (entities_with, multi-component intersection) — E03ENTSTO-005.
- Factory/archetype helpers (create_agent, create_place) — E03ENTSTO-006.
- Event journaling or event emission — E06.
- Components defined by E04/E05 — those epics add their tables.
- World serialization — E03ENTSTO-007.

## Acceptance Criteria

### Tests That Must Pass

1. **Create + alive check**: `create_entity` returns a valid id, `is_alive` returns true.
2. **Archive + checks**: after archive, `is_alive` is false, `is_archived` is true.
3. **Purge cleans components**: after inserting components + archiving + purging, all components are gone.
4. **Component CRUD round-trip**: insert name, get it back, update via mut, remove it.
5. **Insert on dead entity errors**: inserting a component on an archived entity returns `WorldError::ArchivedEntity`.
6. **Insert duplicate errors**: inserting the same component type twice returns `WorldError::DuplicateComponent`.
7. **Get on missing returns None**: getting a component that was never inserted returns `None`.
8. **No public fields**: `World` has no `pub` fields — all access via methods.
9. **Topology accessible**: `world.topology()` returns a valid reference.
10. **Existing suite**: `cargo test -p worldwake-core` continues green.

### Invariants

1. No public fields on `World` — enforced by compilation (fields are private).
2. No direct external mutation of component tables.
3. Every mutation path is a single method call — narrow enough for E06 journaling.
4. Ordinary simulation code cannot receive broad mutable access to all tables.
5. Entity existence is validated before any component operation.

## Test Plan

### New Tests

In `crates/worldwake-core/src/world.rs`:

- `create_entity_returns_alive_id`
- `archive_entity_marks_non_live`
- `purge_cleans_components`
- `component_crud_roundtrip`
- `insert_on_archived_entity_errors`
- `insert_duplicate_component_errors`
- `get_missing_component_returns_none`
- `remove_missing_component_returns_none`
- `topology_accessible`
- `world_new_starts_empty`

### Commands

```bash
cargo test -p worldwake-core world
cargo clippy --workspace && cargo test --workspace
```
