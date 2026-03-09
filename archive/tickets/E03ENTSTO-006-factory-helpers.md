# E03ENTSTO-006: Factory / Archetype Helpers

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: None
**Deps**: E03ENTSTO-004 (World struct with component CRUD)

## Problem

The spec calls for convenience factory helpers, but the current code and tests only support a clean implementation for non-topological entities.

The original ticket still overreached by keeping `create_place` in scope. That does not match the current architecture:

1. `Topology` is the sole authoritative owner of `Place` payloads.
2. `World` reconciles topology-owned place ids into entity metadata only at construction time.
3. `World` does not yet own any topology mutation API for adding places after construction.
4. `Place` already carries `name`, so a `create_place` helper would also force an unresolved decision about whether `Name` is redundant for places.

Adding `create_place` now would either introduce duplicate authority or force a broader topology-mutation design than this ticket can justify. This ticket should therefore focus on thin, atomic factory helpers for non-topological archetypes only.

## Assumptions Reassessed

1. `Name` and `AgentData` are the only non-topological authoritative component tables currently available in `ComponentTables`.
2. `Place` is owned by `Topology`, not `ComponentTables`, and that split is intentional.
3. `World` already provides the narrow public mutation API needed to build non-topological factory helpers without exposing internal tables.
4. There is no existing world-owned topology mutation API for adding a place after construction, and inventing one here would expand the mutation surface beyond a thin helper ticket.
5. For non-topological factories, partial creation is an architectural bug. If a helper fails after allocating an entity, it should roll back rather than leave half-populated entities behind.

## What to Change

### 1. Add non-topological factory methods to `World`

In `crates/worldwake-core/src/world.rs`, add:

```rust
/// Create a new agent entity with Name and AgentData components.
pub fn create_agent(
    &mut self,
    name: &str,
    control_source: ControlSource,
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
4. Behave atomically: if inserting a required component fails, do not leave a live entity behind with a partial archetype.

`create_place` is explicitly deferred from this ticket. A future ticket can add it only after `World` owns the relevant topology mutation path and the codebase decides whether place names live only in `Topology::Place` or also in `Name`.

## Files to Touch

- `crates/worldwake-core/src/world.rs`

## Out of Scope

- `create_place` and any topology-owned place creation path
- Container, ItemLot, UniqueItem, Facility factories
- Introducing a duplicate `Place` component table
- Broad topology editing APIs
- Event emission — E06
- Name uniqueness rules or higher-level game rules
- Resolving the long-term `Name` versus `Place.name` model for place entities

## Acceptance Criteria

### Tests That Must Pass

1. **create_agent**: returns a live entity with correct `EntityKind`, `Name`, and `AgentData`.
2. **create_office**: returns a live entity with `EntityKind::Office` and `Name`.
3. **create_faction**: returns a live entity with `EntityKind::Faction` and `Name`.
4. **Factory is thin wrapper**: `create_agent` is behaviorally equivalent to manual `create_entity` plus component insertion.
5. **Atomic failure handling**: a failed helper call does not leave a live entity with a partially applied archetype.
6. **Single authority preserved**: no second authoritative place store or topology mutation shortcut is introduced.
7. **Existing suite**: `cargo test -p worldwake-core` continues green.

### Invariants

1. Factory helpers do not bypass the normal world mutation API.
2. Factory helpers do not hide state changes.
3. `Topology` remains the sole authoritative owner of `Place`.
4. No special-case aliasing or duplicate place storage is introduced.
5. Failed helper calls do not leave live partially initialized entities behind.

## Test Plan

### New Tests

In `crates/worldwake-core/src/world.rs`:

- `create_agent_produces_correct_entity`
- `create_agent_components_queryable`
- `create_office_produces_correct_entity`
- `create_faction_produces_correct_entity`
- `factory_equivalent_to_manual_creation`
- `multiple_agents_unique_ids`
- `factory_failure_rolls_back_allocated_entity`

### Commands

```bash
cargo test -p worldwake-core world
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

## Outcome

- **Completed**: 2026-03-09
- **What actually changed**:
  - Narrowed the ticket scope to non-topological factories only after confirming that `Topology` remains the sole authority for `Place` and `World` still has no place-creation mutation API.
  - Added `World::create_agent`, `World::create_office`, and `World::create_faction` as thin wrappers over the existing world mutation surface.
  - Implemented internal rollback so failed factory initialization does not leave live partially initialized entities behind.
  - Added coverage for the new factories, query visibility, manual-equivalence behavior, unique ids, and rollback on failure.
- **Deviation from original plan**:
  - Did not implement `create_place`. That would currently force either duplicate authority or a broader topology-mutation design that belongs in a separate ticket.
  - Did not touch `Topology` because the clean implementation path for this ticket stayed entirely inside `World`.
- **Verification results**:
  - `cargo test -p worldwake-core world`
  - `cargo test -p worldwake-core`
  - `cargo clippy --workspace --all-targets -- -D warnings`
  - `cargo test --workspace`
