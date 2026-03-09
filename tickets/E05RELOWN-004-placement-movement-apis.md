# E05RELOWN-004: Placement and movement mutation APIs

**Status**: PENDING
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: Yes — new methods on `World`
**Deps**: E05RELOWN-002 (RelationTables with physical storage)

## Problem

Raw insertion into relation maps would bypass legality checks. The spec requires controlled mutation helpers that enforce placement invariants: single effective location, acyclic containment, and cascading `LocatedIn` updates when containers move.

## Assumption Reassessment (2026-03-09)

1. `World` has `relations: RelationTables` after E05RELOWN-002 — assumed
2. `World::ensure_alive(id)` exists and returns `Result<&EntityMeta, WorldError>` — confirmed
3. `WorldError::ContainmentCycle` variant exists — confirmed
4. `Container` component on `EntityKind::Container` entities — confirmed
5. `Topology` stores places as `EntityId` — confirmed; places are valid `LocatedIn` targets

## Architecture Check

1. All four placement APIs are methods on `World`, not free functions, because they need access to both `relations` and `allocator`/`components`
2. `set_ground_location` sets `LocatedIn` and clears any `ContainedBy` — an entity on the ground is not in a container
3. `put_into_container` sets `ContainedBy`, then derives `LocatedIn` from the container's effective place (walking the container chain up to a ground entity)
4. `remove_from_container` clears `ContainedBy` but entity must be given a new ground location (or the caller must provide one)
5. `move_container_subtree` updates the container's `LocatedIn` and recursively updates all descendants' `LocatedIn`
6. Cycle detection: before inserting `ContainedBy(entity, container)`, walk from `container` upward through `ContainedBy` chain; if `entity` is found, reject with `ContainmentCycle`

## What to Change

### 1. Add placement helpers to `World` in `world.rs`

```rust
pub fn set_ground_location(&mut self, entity: EntityId, place: EntityId) -> Result<(), WorldError>
```
- Validate entity is alive
- Validate place is a `Place` entity
- Remove any existing `ContainedBy` for entity (and update reverse index)
- Set `LocatedIn(entity, place)` (and update reverse index `entities_at`)

```rust
pub fn put_into_container(&mut self, entity: EntityId, container: EntityId) -> Result<(), WorldError>
```
- Validate both alive
- Validate container has `Container` component
- Cycle check: walk `ContainedBy` chain from `container` upward; reject if `entity` found
- Validate container admission policy (allowed_commodities, allows_unique_items, allows_nested_containers) + capacity check via `load` module
- Set `ContainedBy(entity, container)` + reverse index
- Derive effective place from container chain, set `LocatedIn(entity, effective_place)` + reverse index

```rust
pub fn remove_from_container(&mut self, entity: EntityId) -> Result<(), WorldError>
```
- Validate entity is alive and currently in a container
- Remove `ContainedBy` entry + reverse index
- Entity retains its current `LocatedIn` (caller must subsequently `set_ground_location` or `put_into_container` to relocate)

```rust
pub fn move_container_subtree(&mut self, container: EntityId, new_place: EntityId) -> Result<(), WorldError>
```
- Validate container alive, new_place is a `Place`
- Set `LocatedIn(container, new_place)`
- Recursively collect all descendants via `contents_of` reverse index
- Update `LocatedIn` for every descendant to `new_place`

## Files to Touch

- `crates/worldwake-core/src/world.rs` (modify — add placement methods)

## Out of Scope

- Ownership/possession APIs (E05RELOWN-005)
- Query helpers like `effective_place()`, `entities_effectively_at()` (E05RELOWN-006)
- Reservation API (E05RELOWN-007)
- Social relation mutation (E05RELOWN-008)
- Event emission for moves (E06)
- Transit/travel state (E06+)

## Acceptance Criteria

### Tests That Must Pass

1. `set_ground_location` places entity at a place; subsequent call moves it (old reverse index cleared)
2. `put_into_container` sets both `ContainedBy` and `LocatedIn` (inherited from container)
3. `put_into_container` rejects containment cycle (A in B, B in A)
4. `put_into_container` rejects self-containment
5. `put_into_container` rejects non-container target (entity without `Container` component)
6. `put_into_container` rejects if container admission policy denies the entity type
7. `put_into_container` rejects if container capacity would be exceeded
8. `remove_from_container` clears `ContainedBy` but retains `LocatedIn`
9. `remove_from_container` fails if entity is not in a container
10. `move_container_subtree` updates `LocatedIn` for container and all nested contents recursively
11. Deep nesting (3+ levels) correctly propagates `LocatedIn` changes
12. Existing suite: `cargo test -p worldwake-core`

### Invariants

1. After any placement operation, every affected entity has exactly one `LocatedIn` entry (spec 9.4)
2. Containment graph remains acyclic after any `put_into_container` call (spec 9.18)
3. `entities_at` reverse index stays consistent with `located_in` forward map
4. `contents_of` reverse index stays consistent with `contained_by` forward map

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/world.rs` (extend inline `#[cfg(test)]`) — placement API tests covering happy paths, error cases, cycle detection, deep nesting, reverse index consistency

### Commands

1. `cargo test -p worldwake-core world`
2. `cargo clippy --workspace && cargo test --workspace`
