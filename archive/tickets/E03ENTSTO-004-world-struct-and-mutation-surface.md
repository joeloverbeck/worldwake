# E03ENTSTO-004: World Struct and Mutation Surface

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: None
**Deps**: E03ENTSTO-002 (EntityAllocator), E03ENTSTO-003 (ComponentTables), E02 (Topology)

## Problem

The `World` struct should become the authoritative boundary that owns entity lifecycle, non-topological component storage, and topology access. All fields must be private so persistent mutation stays narrow enough for E06 journaling.

The original ticket assumed `World::new(topology)` could simply wrap an empty allocator around an existing `Topology` and then expose CRUD for names, agents, and places. That does not match the current codebase:

1. `Topology` already owns authoritative `Place` payloads keyed by `EntityId`.
2. `ComponentTables` currently owns only non-topological component tables (`Name`, `AgentData`).
3. `build_prototype_world()` already manufactures place ids directly inside topology, so leaving the allocator empty would create split authority where topology references entity ids unknown to the lifecycle store.
4. Duplicating `Place` into `ComponentTables` here would make the architecture worse, not better.

This ticket therefore focuses on introducing `World` as the single mutation boundary while reconciling topology-owned place ids into allocator metadata instead of introducing a second authoritative place store.

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

### 2. Constructor and topology/entity reconciliation

- `World::new(topology: Topology) -> Result<Self, WorldError>`
- Construction must register every place id already present in `topology` as a live `EntityKind::Place` entity in allocator metadata.
- Construction must not duplicate `Place` payloads outside topology.
- If topology cannot be reconciled into allocator state cleanly, construction must fail instead of silently creating split authority.

This is the key architectural correction in scope for this ticket.

### 3. Entity lifecycle methods

- `create_entity(&mut self, kind: EntityKind, tick: Tick) -> EntityId`
- `archive_entity(&mut self, id: EntityId, tick: Tick) -> Result<(), WorldError>`
- `purge_entity(&mut self, id: EntityId) -> Result<(), WorldError>`:
  also calls `components.remove_all(id)`
- `is_alive(&self, id: EntityId) -> bool`
- `is_archived(&self, id: EntityId) -> bool`
- `entity_meta(&self, id: EntityId) -> Option<&EntityMeta>`
- `entity_kind(&self, id: EntityId) -> Option<EntityKind>`

Place entities seeded from topology must participate in lifecycle reads the same way as any other entity. Purging topology-owned places is out of scope unless the operation can be rejected cleanly.

### 4. Component CRUD methods (validated)

These wrap `ComponentTables` with entity validation:

- `insert_component_name(&mut self, id: EntityId, name: Name) -> Result<(), WorldError>`
- `get_component_name(&self, id: EntityId) -> Option<&Name>`
- `get_component_name_mut(&mut self, id: EntityId) -> Option<&mut Name>`
- `remove_component_name(&mut self, id: EntityId) -> Result<Option<Name>, WorldError>`
- `has_component_name(&self, id: EntityId) -> bool`

Apply the same pattern to each registered non-topological component type (currently `AgentData`). Do not introduce `Place` component CRUD in this ticket.

For robustness, component insertion should reject obvious kind/component mismatches:

- `AgentData` insertion requires `EntityKind::Agent`
- `Name` remains valid for any named entity kind

### 5. Read-only topology access

- `topology(&self) -> &Topology`

### 6. Register module in `lib.rs`

Add `pub mod world;` and re-export `World`.

## Files to Touch

- `crates/worldwake-core/src/world.rs` (new)
- `crates/worldwake-core/src/lib.rs` (modify)
- supporting minimal updates in `allocator.rs` or `topology.rs` only if needed to reconcile topology-owned place ids into `World`

## Out of Scope

- Query API (entities_with, multi-component intersection) — E03ENTSTO-005
- Factory/archetype helpers such as `create_agent` — E03ENTSTO-006
- Topology mutation helpers that add/remove places or edges after `World` construction
- Moving `Place` out of `Topology` or duplicating it into `ComponentTables`
- Event journaling or event emission — E06
- Components defined by E04/E05
- World serialization — E03ENTSTO-007

## Acceptance Criteria

### Tests That Must Pass

1. **Create + alive check**: `create_entity` returns a valid id and `is_alive` returns true.
2. **Topology seeding**: `World::new(topology)` makes every topology place id visible as a live `EntityKind::Place` entity.
3. **Archive + checks**: after archive, `is_alive` is false and `is_archived` is true.
4. **Purge cleans components**: after inserting components, archiving, and purging a non-topological entity, all registered components are gone.
5. **Component CRUD round-trip**: insert name, get it back, update via mut, remove it.
6. **Insert on dead entity errors**: inserting a component on an archived entity returns `WorldError::ArchivedEntity`.
7. **Insert duplicate errors**: inserting the same component type twice returns `WorldError::DuplicateComponent`.
8. **Get on missing returns None**: getting a component that was never inserted returns `None`.
9. **Kind validation**: inserting `AgentData` on a non-agent entity returns an error.
10. **No public fields**: `World` has no `pub` fields.
11. **Topology accessible**: `world.topology()` returns a valid reference.
12. **Existing suite**: `cargo test -p worldwake-core` continues green.

### Invariants

1. No public fields on `World`.
2. No direct external mutation of component tables.
3. Every persistent mutation path is a single method call.
4. Ordinary simulation code cannot receive broad mutable access to all tables.
5. Entity existence is validated before any component operation.
6. `Topology` remains the sole authoritative owner of `Place` payloads.
7. Topology place ids are reconciled into entity metadata during `World` construction.

## Test Plan

### New Tests

In `crates/worldwake-core/src/world.rs`:

- `world_new_registers_topology_places_as_live_entities`
- `create_entity_returns_alive_id`
- `archive_entity_marks_non_live`
- `purge_cleans_components`
- `component_crud_roundtrip`
- `insert_on_archived_entity_errors`
- `insert_duplicate_component_errors`
- `insert_agent_data_on_non_agent_errors`
- `get_missing_component_returns_none`
- `remove_missing_component_returns_none`
- `topology_accessible`
- `world_new_starts_with_empty_non_topological_component_tables`

### Commands

```bash
cargo test -p worldwake-core world
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

## Outcome

- **Completed**: 2026-03-09
- **What actually changed**:
  - Added `crates/worldwake-core/src/world.rs` with a private-field `World` façade over `EntityAllocator`, `ComponentTables`, and `Topology`.
  - Made `World::new(topology)` reconcile topology-owned place ids into allocator metadata so topology entities are visible through the lifecycle API instead of living outside it.
  - Added validated CRUD wrappers for the current non-topological component set: `Name` and `AgentData`.
  - Added kind validation for `AgentData` and guarded topology-owned places from archive/purge operations that would otherwise split topology from entity lifecycle authority.
  - Added a read-only `Topology::place_ids()` iterator and allocator seeding support for pre-existing entity ids.
- **Deviation from original plan**:
  - `World::new(topology)` returns `Result<Self, WorldError>` instead of constructing blindly. This prevents silently accepting inconsistent topology/entity authority.
  - Did not add `Place` component CRUD. `Topology` remains the sole authoritative owner of `Place`.
  - Added explicit guards against archiving or purging topology-owned places until a later ticket defines coordinated topology mutation semantics.
- **Verification results**:
  - `cargo test -p worldwake-core world`
  - `cargo test -p worldwake-core`
  - `cargo clippy --workspace --all-targets -- -D warnings`
  - `cargo test --workspace`
