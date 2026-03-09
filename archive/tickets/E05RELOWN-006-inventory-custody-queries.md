# E05RELOWN-006: Inventory and custody query helpers

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — new read-only helper methods on `World`
**Deps**: E05RELOWN-004 (placement APIs populate relations), E05RELOWN-005 (ownership/possession APIs)

## Problem

Downstream systems (trade, combat, perception, AI) need efficient read-only queries over the relation layer. The spec lists eight specific query helpers that must be provided, and the current code already maintains the forward and reverse indices those helpers should read from.

## Assumption Reassessment (2026-03-09)

1. `RelationTables` already has the required physical forward and reverse indices: `located_in`/`entities_at`, `contained_by`/`contents_of`, `owned_by`/`property_of`, and `possessed_by`/`possessions_of`.
2. E05RELOWN-004 already maintains `LocatedIn` as the authoritative effective-place index for both ground and nested entities. Nested entities do not require recomputing their place from the container chain on every read.
3. E05RELOWN-004 also updates descendant `LocatedIn` rows when containers move, so these query helpers should reuse that invariant instead of duplicating movement logic.
4. E05RELOWN-005 already provides mutation APIs plus `can_exercise_control`; this ticket is only about exposing read helpers for ownership and possession.
5. Public `World` query APIs in this crate expose the live world view. `World::archive_entity` now removes the archived entity's own relation rows immediately, but rejects archival while live dependents still anchor through target-side relations such as containment, possession, ownership, office holding, or social reverse indices.
6. `entities_effectively_at(place)` must include nested contents because `LocatedIn` points at the top-level place even for contained entities.
7. `ground_entities_at(place)` must exclude nested contents and return only entities whose effective place is `place` and whose `ContainedBy` relation is absent.

## Architecture Check

1. All query methods remain `&self` methods on `World`; they are pure reads.
2. The implementation should live with the existing domain-specific APIs:
   placement/inventory helpers in `crates/worldwake-core/src/world/placement.rs`
   ownership/custody helpers in `crates/worldwake-core/src/world/ownership.rs`
   `world.rs` should only continue to expose the public `impl World` surface and tests.
3. `effective_place(entity)` should read the authoritative `located_in` row for live entities. It should not walk the containment chain as its primary behavior because that would duplicate state derivation already enforced by placement mutations and could mask index drift.
4. `entities_effectively_at(place)` should use `entities_at`, filtered to live entities.
5. `ground_entities_at(place)` should derive from `entities_at(place)` and exclude entities present in `contained_by`.
6. `direct_container`, `direct_contents_of`, and `recursive_contents_of` should use `contained_by`/`contents_of`, filtered to live entities and preserving deterministic order from the underlying `BTree*` structures.
7. `owner_of` and `possessor_of` should read from `owned_by` and `possessed_by`, returning `None` when either the subject entity or the related owner/holder is not live.

## What to Change

### 1. Add read-only query methods on `World`

```rust
pub fn effective_place(&self, entity: EntityId) -> Option<EntityId>
pub fn direct_container(&self, entity: EntityId) -> Option<EntityId>
pub fn direct_contents_of(&self, container: EntityId) -> Vec<EntityId>
pub fn recursive_contents_of(&self, container: EntityId) -> Vec<EntityId>
pub fn entities_effectively_at(&self, place: EntityId) -> Vec<EntityId>
pub fn ground_entities_at(&self, place: EntityId) -> Vec<EntityId>
pub fn owner_of(&self, entity: EntityId) -> Option<EntityId>
pub fn possessor_of(&self, entity: EntityId) -> Option<EntityId>
```

All return deterministic results in ascending `EntityId` order via `BTreeMap`/`BTreeSet` iteration order.

### 2. Keep the implementation aligned with current architecture

- Add placement/inventory query helpers to `crates/worldwake-core/src/world/placement.rs`
- Add ownership/custody query helpers to `crates/worldwake-core/src/world/ownership.rs`
- Reuse existing relation tables and placement invariants; do not add alias indices or recomputed caches

## Files to Touch

- `crates/worldwake-core/src/world/placement.rs` (modify — add placement/inventory read helpers)
- `crates/worldwake-core/src/world/ownership.rs` (modify — add ownership/custody read helpers)
- `crates/worldwake-core/src/world.rs` (modify — extend inline tests)

## Out of Scope

- Mutation APIs (already in E05RELOWN-004, -005)
- Reservation queries (E05RELOWN-007)
- Social relation queries (E05RELOWN-008)
- New relation storage or alias caches
- Event-based change tracking

## Acceptance Criteria

### Tests That Must Pass

1. `effective_place` returns the place from `LocatedIn` for a directly placed live entity.
2. `effective_place` returns the same top-level place for a deeply nested live entity after placement helpers populate inherited `LocatedIn`.
3. `effective_place` returns `None` for an unplaced entity, missing entity, or archived entity.
4. `direct_container` returns `Some(container)` for contained live entities and `None` for ground, missing, or archived entities.
5. `direct_contents_of` returns only immediate live children, not nested grandchildren.
6. `recursive_contents_of` returns all live descendants at all nesting depths in deterministic order.
7. `recursive_contents_of` returns an empty vec for entities with no contents or non-live containers.
8. `entities_effectively_at(place)` includes both ground entities and nested contents for that place.
9. `ground_entities_at(place)` excludes nested contents and returns only entities directly on the ground at that place.
10. `owner_of` and `possessor_of` return correct live relations or `None` when the entity or related owner/holder is not live.
11. Moving a container via `move_container_subtree` is reflected in subsequent `effective_place`, `recursive_contents_of`, and `entities_effectively_at` queries.
12. Archived entities do not appear in any of the new public query helpers. Archival removes the archived entity's own rows immediately, and target-side archival is blocked until dependent live relations are cleared explicitly.
13. Existing suite: `cargo test -p worldwake-core`

### Invariants

1. All query results are deterministic.
2. Query methods are pure reads and do not mutate relation state.
3. `entities_effectively_at(place)` is a superset of `ground_entities_at(place)` for any place.
4. Query helpers read the authoritative relation tables already maintained by placement/ownership mutations; they do not recompute or shadow those indices.
5. Public query helpers expose the live world view only.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/world.rs` (extend inline `#[cfg(test)]`) — placement query helper tests for direct, nested, moved, empty, and archived cases
2. `crates/worldwake-core/src/world.rs` (extend inline `#[cfg(test)]`) — ownership/possession query helper tests for present, absent, and archived-related cases

### Commands

1. `cargo test -p worldwake-core world`
2. `cargo test -p worldwake-core`
3. `cargo clippy --workspace`
4. `cargo test --workspace`

## Outcome

- Completion date: 2026-03-09
- Actual changes:
  - Added public read-only `World` helpers for `effective_place`, `direct_container`, `direct_contents_of`, `recursive_contents_of`, `entities_effectively_at`, `ground_entities_at`, `owner_of`, and `possessor_of`
  - Implemented placement/inventory queries in `crates/worldwake-core/src/world/placement.rs` and ownership/custody queries in `crates/worldwake-core/src/world/ownership.rs` to stay aligned with the existing world submodule architecture
  - Kept the query layer anchored to the authoritative relation indices already maintained by placement and ownership mutations rather than recomputing placement state from container chains
  - Added focused coverage for nested containment queries, post-move query correctness, deterministic ordering, and live-only behavior across archive boundaries
- Deviations from original plan:
  - The ticket was corrected before implementation because `effective_place` should read `LocatedIn` as the authoritative source of truth, not re-derive placement by walking containment on every query
  - The ticket originally targeted `world.rs` directly; the implementation instead followed the cleaner existing split between placement and ownership submodules
  - Outcome amended: 2026-03-09
  - After implementation, archival semantics were strengthened further so `archive_entity` now removes outbound relation rows immediately but rejects archiving entities that still anchor live dependents; this removed the need for silent target-side detachment and made lifecycle transitions explicit
  - The lifecycle surface was further improved with typed `archive_dependencies` reporting, so callers can inspect deterministic archive blockers before mutating state instead of parsing precondition strings
  - A dedicated `prepare_entity_for_archive` lifecycle operation was added so callers can intentionally clear current archive blockers through one authoritative path before invoking strict archival
  - Archive preparation was then generalized into a policy-driven API with typed reports, so callers can choose which blocker classes may be auto-cleared and which must remain blocked
  - The policy surface was refined further into explicit archive resolution actions per dependency kind, making the lifecycle model extensible when future domains need alternatives beyond the current default detach/drop/revoke behaviors
  - Containment resolution was then widened to support multiple legal outcomes, including preserving nested subtrees intact or recursively spilling them into the current place, proving the lifecycle layer can express domain-specific resolution strategies instead of one hardcoded cleanup path
  - The lifecycle surface now has an explicit plan/apply split: callers can inspect a typed `ArchivePreparationPlan` before mutation, then execute it through `prepare_entity_for_archive*`, which returns a typed `ArchivePreparationReport` describing the resolutions actually applied
- Verification results:
  - `cargo fmt --all`
  - `cargo test -p worldwake-core archive_entity`
  - `cargo test -p worldwake-core archive`
  - `cargo test -p worldwake-core prepare_entity_for_archive`
  - `cargo test -p worldwake-core world`
  - `cargo test -p worldwake-core`
  - `cargo clippy --workspace`
  - `cargo test --workspace`
