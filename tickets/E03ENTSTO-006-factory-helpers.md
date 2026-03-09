# E03ENTSTO-006: Factory / Archetype Helpers

**Status**: TODO
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: None
**Deps**: E03ENTSTO-004 (World struct with component CRUD)

## Problem

The spec requires convenience factory helpers like `create_agent`, `create_place`, and `create_office` that are thin wrappers over authoritative world mutation. They should reduce boilerplate without creating hidden state transitions or duplicate authority.

The original ticket assumed `create_place` should allocate an entity and insert a `Place` component separately from topology. That no longer matches the current architecture:

1. `Topology` is already the sole authoritative owner of `Place` payloads.
2. `World` now seeds topology-owned place ids into entity metadata during construction rather than duplicating place storage.
3. A `create_place` helper that only inserts a component would recreate the split-authority bug E03ENTSTO-004 just corrected.

This ticket should therefore keep thin factory helpers for non-topological archetypes and define `create_place` as a coordinated topology-plus-entity operation, not a duplicate component insert.

## Assumptions Reassessed

1. `Name` and `AgentData` are the only non-topological component tables currently available in `ComponentTables`.
2. `Place` is not a `ComponentTables` entry and should not become one in this ticket.
3. `World` is now the authoritative lifecycle boundary, but topology-owned place creation still needs an explicit world-level API so later code does not mutate topology behind `World`'s back.
4. If place creation needs topology mutation support that `World` does not expose yet, this ticket should add the smallest coordinated mutation surface necessary rather than a parallel place store.

## What to Change

### 1. Add factory methods to `World`

In `crates/worldwake-core/src/world.rs`, add:

```rust
/// Create a new agent entity with Name and AgentData components.
pub fn create_agent(
    &mut self,
    name: &str,
    control_source: ControlSource,
    tick: Tick,
) -> Result<EntityId, WorldError>

/// Create a new place entity and register it in topology.
///
/// Thin wrapper: allocates an entity of kind Place, inserts a Name,
/// and adds the authoritative Place payload to topology using the same id.
pub fn create_place(
    &mut self,
    name: &str,
    place: Place,
    tick: Tick,
) -> Result<EntityId, WorldError>

/// Create a new office entity with Name.
pub fn create_office(
    &mut self,
    name: &str,
    tick: Tick,
) -> Result<EntityId, WorldError>

/// Create a new faction entity with Name.
pub fn create_faction(
    &mut self,
    name: &str,
    tick: Tick,
) -> Result<EntityId, WorldError>
```

### 2. Required behavior

Each factory helper should:

1. Call the narrow world mutation API instead of mutating fields directly.
2. Leave the resulting entity indistinguishable from one created manually through the same public world methods.
3. Preserve single-authority semantics for the resulting data.

For `create_place` specifically:

1. Allocate the entity through `World::create_entity(EntityKind::Place, tick)`.
2. Add the `Place` payload to `Topology` using the same `EntityId`.
3. Optionally insert `Name` only if keeping both `Name` and `Place.name` is still considered intentional by the codebase at that point.
4. If any step fails, do not leave topology and entity metadata disagreeing about whether the place exists.

If implementing `create_place` reveals that `Name` and `Place.name` are redundant and harmful, prefer tightening the ticket and API rather than cementing duplicated naming fields.

## Files to Touch

- `crates/worldwake-core/src/world.rs`
- `crates/worldwake-core/src/topology.rs` only if a minimal, coordinated world-owned topology insertion method is needed

## Out of Scope

- Container, ItemLot, UniqueItem, Facility factories
- Introducing a duplicate `Place` component table
- Broad topology editing APIs beyond the minimal surface needed for coordinated place creation
- Event emission — E06
- Name uniqueness rules or higher-level game rules

## Acceptance Criteria

### Tests That Must Pass

1. **create_agent**: returns a live entity with correct `EntityKind`, `Name`, and `AgentData`.
2. **create_place**: returns a live entity with correct `EntityKind`, and the resulting id is present in topology with the expected `Place` payload.
3. **create_office**: returns a live entity with `EntityKind::Office` and `Name`.
4. **create_faction**: returns a live entity with `EntityKind::Faction` and `Name`.
5. **Factory is thin wrapper**: `create_agent` is behaviorally equivalent to manual `create_entity` plus component insertion.
6. **Place creation preserves single authority**: no second authoritative place store is introduced.
7. **Failure does not split state**: failed place creation does not leave topology and entity metadata disagreeing.
8. **Existing suite**: `cargo test -p worldwake-core` continues green.

### Invariants

1. Factory helpers do not bypass the normal world mutation API.
2. Factory helpers do not hide state changes.
3. `Topology` remains the sole authoritative owner of `Place`.
4. No special-case aliasing or duplicate place storage is introduced.

## Test Plan

### New Tests

In `crates/worldwake-core/src/world.rs`:

- `create_agent_produces_correct_entity`
- `create_agent_components_queryable`
- `create_place_registers_entity_in_topology`
- `create_place_failure_does_not_split_topology_and_entity_state`
- `create_office_produces_correct_entity`
- `create_faction_produces_correct_entity`
- `factory_equivalent_to_manual_creation`
- `multiple_agents_unique_ids`

### Commands

```bash
cargo test -p worldwake-core world
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```
