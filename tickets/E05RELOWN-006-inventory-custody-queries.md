# E05RELOWN-006: Inventory and custody query helpers

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — new query methods on `World`
**Deps**: E05RELOWN-004 (placement APIs populate relations), E05RELOWN-005 (ownership/possession APIs)

## Problem

Downstream systems (trade, combat, perception, AI) need efficient read-only queries over the relation layer. The spec lists eight specific query helpers that must be provided.

## Assumption Reassessment (2026-03-09)

1. `RelationTables` has all physical relation forward+reverse maps after E05RELOWN-002 — assumed
2. Placement/movement APIs (E05RELOWN-004) and ownership APIs (E05RELOWN-005) populate these maps — assumed
3. `entities_effectively_at(place)` must include nested contents — per spec, "this relation is what `entities_effectively_at(place)` queries"
4. `ground_entities_at(place)` excludes nested contents — per spec distinction

## Architecture Check

1. All query methods are `&self` methods on `World` — no mutation
2. `effective_place` walks the `ContainedBy` chain upward to find the ground-level entity's `LocatedIn` place
3. `entities_effectively_at` uses the `entities_at` reverse index (which already includes nested entities because `put_into_container` sets `LocatedIn` for nested items too)
4. `ground_entities_at` returns entities in `entities_at` that are NOT in `contained_by` (i.e., directly on the ground)
5. `recursive_contents_of` performs BFS/DFS over `contents_of` reverse index

## What to Change

### 1. Add query methods to `World` in `world.rs`

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

All return deterministic results (sorted by `EntityId` via `BTreeMap`/`BTreeSet` iteration order).

## Files to Touch

- `crates/worldwake-core/src/world.rs` (modify — add query methods)

## Out of Scope

- Mutation APIs (already in E05RELOWN-004, -005)
- Reservation queries (E05RELOWN-007)
- Social relation queries (E05RELOWN-008)
- Performance optimization (BFS vs DFS for recursive_contents_of)
- Event-based change tracking

## Acceptance Criteria

### Tests That Must Pass

1. `effective_place` returns the ground place for a directly-placed entity
2. `effective_place` returns the ground place for a deeply nested entity (item in crate in storehouse at place)
3. `effective_place` returns `None` for an entity with no placement
4. `direct_container` returns `Some(container)` for contained entities, `None` for ground entities
5. `direct_contents_of` returns only immediate children, not nested grandchildren
6. `recursive_contents_of` returns all descendants at all nesting depths
7. `recursive_contents_of` returns empty vec for entity with no contents
8. `entities_effectively_at(place)` includes both ground entities and nested contents
9. `ground_entities_at(place)` excludes nested contents (only entities directly located at the place without a container)
10. `owner_of` and `possessor_of` return correct values or `None`
11. Moving a container via `move_container_subtree` is reflected in subsequent `effective_place` and `entities_effectively_at` queries
12. Existing suite: `cargo test -p worldwake-core`

### Invariants

1. All query results are deterministic (consistent ordering from BTreeMap/BTreeSet)
2. Query methods are pure reads — no mutation
3. `entities_effectively_at` ⊇ `ground_entities_at` for any place

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/world.rs` (extend inline `#[cfg(test)]`) — query helper tests with various nesting scenarios, empty cases, and cross-reference with placement mutations

### Commands

1. `cargo test -p worldwake-core world`
2. `cargo clippy --workspace && cargo test --workspace`
