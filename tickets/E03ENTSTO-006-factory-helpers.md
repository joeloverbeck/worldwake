# E03ENTSTO-006: Factory / Archetype Helpers

**Status**: TODO
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: None
**Deps**: E03ENTSTO-004 (World struct with component CRUD)

## Problem

The spec requires convenience factory helpers like `create_agent`, `create_place`, `create_office` that are thin wrappers over entity creation + component insertion. They must not hide authoritative state changes — just reduce boilerplate for common entity archetypes.

## What to Change

### 1. Add factory methods to `World`

In `crates/worldwake-core/src/world.rs`, add:

```rust
/// Create a new agent entity with Name and AgentData components.
///
/// Thin wrapper: creates entity of kind Agent, inserts Name and AgentData.
pub fn create_agent(
    &mut self,
    name: &str,
    control_source: ControlSource,
    tick: Tick,
) -> Result<EntityId, WorldError>

/// Create a new place entity with Name and Place components.
///
/// Thin wrapper: creates entity of kind Place, inserts Name and Place.
/// Does NOT add the place to the topology — topology is managed separately.
pub fn create_place(
    &mut self,
    name: &str,
    place: Place,
    tick: Tick,
) -> Result<EntityId, WorldError>

/// Create a new office entity with Name component.
///
/// Thin wrapper: creates entity of kind Office, inserts Name.
pub fn create_office(
    &mut self,
    name: &str,
    tick: Tick,
) -> Result<EntityId, WorldError>

/// Create a new faction entity with Name component.
pub fn create_faction(
    &mut self,
    name: &str,
    tick: Tick,
) -> Result<EntityId, WorldError>
```

Each factory method:
1. Calls `self.create_entity(kind, tick)` to allocate the entity.
2. Inserts the appropriate components via the CRUD API.
3. Returns the new `EntityId`.
4. If any component insertion fails, the entity still exists but is in a partially-initialized state — the error is propagated.

## Files to Touch

- `crates/worldwake-core/src/world.rs` (modify — add factory methods)

## Out of Scope

- Container, ItemLot, UniqueItem, Facility factories — those depend on E04/E05 component types.
- Topology integration (adding the place to the graph) — topology is separate from the entity's Place component.
- Event emission — E06.
- Any validation beyond entity existence (e.g., "is this name unique?").

## Acceptance Criteria

### Tests That Must Pass

1. **create_agent**: returns a live entity with correct EntityKind, Name, and AgentData.
2. **create_place**: returns a live entity with correct EntityKind, Name, and Place component.
3. **create_office**: returns a live entity with EntityKind::Office and Name.
4. **create_faction**: returns a live entity with EntityKind::Faction and Name.
5. **Factory is thin wrapper**: the entity created by `create_agent` is indistinguishable from one created via manual `create_entity` + `insert_component_*` calls.
6. **Multiple agents**: creating multiple agents produces unique ids, each queryable.
7. **Existing suite**: `cargo test -p worldwake-core` continues green.

### Invariants

1. Factory helpers do not bypass the normal mutation API.
2. Factory helpers do not hide state changes — every mutation is visible through the same CRUD/query methods.
3. No special-case logic for any entity kind beyond setting the correct `EntityKind`.

## Test Plan

### New Tests

In `crates/worldwake-core/src/world.rs`:

- `create_agent_produces_correct_entity`
- `create_agent_components_queryable`
- `create_place_produces_correct_entity`
- `create_office_produces_correct_entity`
- `create_faction_produces_correct_entity`
- `factory_equivalent_to_manual_creation`
- `multiple_agents_unique_ids`

### Commands

```bash
cargo test -p worldwake-core world
cargo clippy --workspace && cargo test --workspace
```
