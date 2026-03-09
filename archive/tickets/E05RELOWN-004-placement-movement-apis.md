# E05RELOWN-004: Placement and movement mutation APIs

**Status**: COMPLETED
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
5. `Topology` stores places as `EntityId` and `Topology::place(place_id)` is the authoritative way to validate `LocatedIn` targets — confirmed; this ticket should validate against topology membership, not just `EntityKind::Place`
6. `load::{load_of_entity, current_container_load, remaining_container_capacity}` already exist — confirmed; container-capacity checks should reuse this module rather than duplicate load math inside `World`
7. There are currently no public placement/movement mutation APIs on `World` — confirmed; inline tests in `world.rs` only touch `relations` directly for purge coverage, so this ticket must add the first legal mutation path for placement

## Architecture Check

1. All four placement APIs are methods on `World`, not free functions, because they need access to both `relations` and `allocator`/`components`
2. The public methods should be thin wrappers over small private `World` helpers that own forward/reverse-index mutation. The codebase already has paired indices (`located_in`/`entities_at`, `contained_by`/`contents_of`), so duplicating row-edit logic across four public methods would be brittle and make later E05 tickets harder to extend cleanly
3. `set_ground_location` sets `LocatedIn` and clears any `ContainedBy` — an entity on the ground is not in a container
4. `put_into_container` sets `ContainedBy`, then derives `LocatedIn` from the container's effective place by walking the existing container chain up to a topology place
5. `remove_from_container` clears `ContainedBy` but entity retains its current effective `LocatedIn`; callers can then either leave it grounded at that effective place or move it elsewhere with another placement helper
6. If the moved entity is itself a container, `set_ground_location` and `put_into_container` must also update `LocatedIn` for the whole descendant subtree; otherwise nested contents would keep a stale effective place and violate spec 9.4
7. `move_container_subtree` updates the container's `LocatedIn` and recursively updates all descendants' effective `LocatedIn`
8. Cycle detection: before inserting `ContainedBy(entity, container)`, walk from `container` upward through the `ContainedBy` chain; if `entity` is found, reject with `ContainmentCycle`
9. Container admission checks should be centralized in one helper and enforced from `put_into_container`: commodity allow-list, unique-item permission, nested-container permission, and capacity via `load.rs`

## What to Change

### 1. Add placement helpers to `World` in `world.rs`

```rust
pub fn set_ground_location(&mut self, entity: EntityId, place: EntityId) -> Result<(), WorldError>
```
- Validate entity is alive
- Validate `self.topology.place(place).is_some()`
- Remove any existing `ContainedBy` for entity (and update reverse index)
- Set `LocatedIn(entity, place)` (and update reverse index `entities_at`)
- If `entity` has a `Container` component, propagate the new effective place to all descendants

```rust
pub fn put_into_container(&mut self, entity: EntityId, container: EntityId) -> Result<(), WorldError>
```
- Validate both alive
- Validate container has `Container` component
- Cycle check: walk `ContainedBy` chain from `container` upward; reject if `entity` found
- Validate container admission policy (allowed_commodities, allows_unique_items, allows_nested_containers) + capacity check via `load` module
- Set `ContainedBy(entity, container)` + reverse index
- Derive effective place from container chain, set `LocatedIn(entity, effective_place)` + reverse index
- If `entity` has a `Container` component, propagate the inherited effective place to all descendants

```rust
pub fn remove_from_container(&mut self, entity: EntityId) -> Result<(), WorldError>
```
- Validate entity is alive and currently in a container
- Remove `ContainedBy` entry + reverse index
- Entity retains its current `LocatedIn` (caller must subsequently `set_ground_location` or `put_into_container` to relocate)

```rust
pub fn move_container_subtree(&mut self, container: EntityId, new_place: EntityId) -> Result<(), WorldError>
```
- Validate container alive, container has `Container` component, and `self.topology.place(new_place).is_some()`
- Set `LocatedIn(container, new_place)`
- Recursively collect all descendants via `contents_of` reverse index
- Update `LocatedIn` for every descendant to `new_place`

### 2. Add private legality helpers in `world.rs`

These are implementation details, not new public API, but they belong in scope because they make the public methods robust and reusable for later E05 tickets.

- `require_place(place) -> Result<(), WorldError>` or equivalent
- relation-row helpers that update forward and reverse indices together for `LocatedIn` and `ContainedBy`
- `effective_place_from_container(container) -> Result<EntityId, WorldError>` or equivalent
- `validate_container_admission(entity, container) -> Result<(), WorldError>` or equivalent
- descendant collection helper used by `move_container_subtree`

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
10. `set_ground_location` rejects non-place targets by checking topology membership
11. Moving a container with `set_ground_location` updates `LocatedIn` for the whole descendant subtree
12. Moving a container with `put_into_container` updates `LocatedIn` for the whole descendant subtree
13. `move_container_subtree` updates `LocatedIn` for container and all nested contents recursively
14. Deep nesting (3+ levels) correctly propagates `LocatedIn` changes
15. Existing suite: `cargo test -p worldwake-core`

### Invariants

1. After any placement operation, every affected entity has exactly one `LocatedIn` entry (spec 9.4)
2. Containment graph remains acyclic after any `put_into_container` call (spec 9.18)
3. `entities_at` reverse index stays consistent with `located_in` forward map
4. `contents_of` reverse index stays consistent with `contained_by` forward map
5. Placement legality is enforced through one shared mutation path, not open-coded row edits per public method

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/world.rs` (extend inline `#[cfg(test)]`) — placement API tests covering happy paths, error cases, cycle detection, deep nesting, reverse index consistency, and topology-backed place validation

### Commands

1. `cargo test -p worldwake-core world`
2. `cargo clippy --workspace && cargo test --workspace`

## Outcome

- Completion date: 2026-03-09
- Actual changes:
  - Added `World::{set_ground_location, put_into_container, remove_from_container, move_container_subtree}` in `crates/worldwake-core/src/world.rs`
  - Added private legality helpers for topology-backed place validation, paired forward/reverse relation updates, container admission checks, containment-cycle checks, effective-place derivation, and descendant collection
  - Extracted the placement legality layer into internal module `crates/worldwake-core/src/world/placement.rs` so later E05 relation work can reuse the same boundary without further bloating `world.rs`
  - Ensured container moves propagate `LocatedIn` changes to descendant subtrees not just through `move_container_subtree`, but also when a container is grounded or reparented into another container
  - Added focused placement and movement tests in `crates/worldwake-core/src/world.rs`
- Deviations from original plan:
  - The ticket was corrected first to use `Topology::place` as the authoritative place validator instead of relying on `EntityKind::Place`
  - The implementation scope was widened slightly to require shared private helpers so placement legality is enforced through one mutation path rather than duplicated row edits
  - The ticket originally underspecified container-subtree propagation for `set_ground_location` and `put_into_container`; that scope gap was corrected and implemented
- Verification results:
  - `cargo test -p worldwake-core`
  - `cargo clippy --workspace --all-targets -- -D warnings`
  - `cargo test --workspace`
