# E03ENTSTO-005: Deterministic World Query API

**Status**: ✅ COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: None
**Deps**: E03ENTSTO-004 (World struct with component CRUD)

## Problem

Simulation systems need deterministic world-level read APIs for iterating entity ids and authoritative component data without reaching into allocator or table internals.

The original ticket assumed a generic query surface such as `entities_with<T>()` and `entities_with_component<T>()`. That does **not** match the current architecture or the E03 spec direction:

- `World` intentionally exposes explicit typed methods, not a runtime-erased or faux-generic query layer.
- `Place` data is topology-owned in `Topology`, not stored in `ComponentTables`.
- The current Phase 1 component tables in `worldwake-core` are `Name` and `AgentData`.
- Deterministic ordering already exists at the storage layer via `BTreeMap`; the missing piece is a world-level query API that composes allocator state with typed tables cleanly.

This ticket therefore covers explicit deterministic world queries for entity metadata plus the currently implemented component tables. It does **not** introduce a generic ECS-style query abstraction.

## Why This Direction Is Better Than The Original Assumption

Adding generic query APIs now would cut against the explicit typed-table architecture E03 is trying to protect. That would increase surface area, encourage aliasing between storage concepts, and make later event journaling harder to reason about.

The cleaner architecture is:

- allocator owns entity lifecycle and ordered entity identity iteration
- component tables own per-table deterministic storage iteration
- `World` owns the public query contract that joins those concerns into authoritative, live-only read APIs

That keeps the read surface narrow, robust, and extensible as more typed tables are added in later tickets.

## What To Change

### 1. Add deterministic entity iteration to `World`

In `crates/worldwake-core/src/world.rs`, add:

```rust
/// All live entity ids in sorted order.
pub fn entities(&self) -> impl Iterator<Item = EntityId> + '_

/// All registered entity ids, including archived ones, in sorted order.
pub fn all_entities(&self) -> impl Iterator<Item = EntityId> + '_

/// All live entities of a specific kind, sorted.
pub fn entities_of_kind(&self, kind: EntityKind) -> impl Iterator<Item = EntityId> + '_

/// Number of live entities.
pub fn entity_count(&self) -> usize
```

Notes:

- `entities()` should delegate to allocator-owned ordering.
- `all_entities()` should include archived entities but not purged ones.
- topology-owned place entities must participate correctly in these queries because they are registered in the allocator at `World::new`.

### 2. Add explicit per-component world queries for the current tables

Add world-level read methods for the component tables that exist today:

```rust
/// Live entities with a Name component, sorted by EntityId.
pub fn entities_with_name(&self) -> impl Iterator<Item = EntityId> + '_

/// Live (EntityId, &Name) pairs, sorted by EntityId.
pub fn query_name(&self) -> impl Iterator<Item = (EntityId, &Name)> + '_

/// Count of live entities with a Name component.
pub fn count_with_name(&self) -> usize

/// Live entities with AgentData, sorted by EntityId.
pub fn entities_with_agent_data(&self) -> impl Iterator<Item = EntityId> + '_

/// Live (EntityId, &AgentData) pairs, sorted by EntityId.
pub fn query_agent_data(&self) -> impl Iterator<Item = (EntityId, &AgentData)> + '_

/// Count of live entities with AgentData.
pub fn count_with_agent_data(&self) -> usize
```

Notes:

- These should be world methods, not direct table exposure.
- Archived entities must be excluded even if stale component rows somehow existed.
- No mutable query API is added here.

### 3. Add explicit multi-component intersection for the currently meaningful case

Add:

```rust
/// Live entities that have both Name and AgentData, sorted by EntityId.
pub fn entities_with_name_and_agent_data(&self) -> impl Iterator<Item = EntityId> + '_

/// Live tuples for entities that have both components, sorted by EntityId.
pub fn query_name_and_agent_data(&self) -> impl Iterator<Item = (EntityId, &Name, &AgentData)> + '_
```

Implementation rule:

- Drive the intersection from the smaller relevant table when practical, but preserve global `EntityId` ordering in the final iterator.
- Do not add a generic intersection builder in this ticket.

### 4. Document filtered-query behavior through tests

No special filtering API is needed. Standard iterator adaptors on these deterministic iterators preserve relative order.

Capture that contract with tests rather than adding a redundant wrapper.

## Files To Touch

- `crates/worldwake-core/src/allocator.rs` if needed for ordered all-entity iteration
- `crates/worldwake-core/src/world.rs`

## Assumptions Updated

- `World` already exists and already owns typed CRUD for `Name` and `AgentData`.
- `ComponentTables` already provides deterministic per-table iterators.
- `Topology` places are allocator-registered entities, not component-table rows.
- There is no existing generic query abstraction, and introducing one is out of scope for this ticket.

## Out Of Scope

- Generic `entities_with<T>()` or `entities_with_component<T>()`
- Query generation for future tables that do not exist yet
- `Place` component-table queries
- Mutable query APIs
- Factory helpers from E03ENTSTO-006
- Serialization work from E03ENTSTO-007
- Performance work beyond straightforward deterministic iteration

## Acceptance Criteria

### Tests That Must Pass

1. `entities()` yields live ids in sorted `EntityId` order.
2. `all_entities()` includes archived ids and excludes purged ids.
3. `entities_of_kind()` returns only live entities of the requested kind, including topology-owned places.
4. `entities_with_name()` returns only live entities that have `Name`.
5. `query_name()` returns sorted `(EntityId, &Name)` pairs.
6. `entities_with_agent_data()` returns only live entities that have `AgentData`.
7. `query_agent_data()` returns sorted `(EntityId, &AgentData)` pairs.
8. `entities_with_name_and_agent_data()` returns only live entities with both components.
9. `query_name_and_agent_data()` returns correct sorted tuples and excludes partial matches.
10. Filtering any of the returned iterators preserves relative order.
11. Empty queries return empty iterators without panic.
12. Count helpers report live-only totals.
13. Deterministic operation sequences produce identical query results.
14. Existing `worldwake-core` tests continue green.

### Invariants

1. Public query ordering is deterministic and derived from ordered storage.
2. World query methods return live entities unless the method explicitly says otherwise.
3. The public query surface remains explicit and typed.
4. No query method exposes broad mutable access.

## Test Plan

### New Or Updated Tests

In `crates/worldwake-core/src/world.rs`:

- `entities_returns_sorted_live_ids`
- `all_entities_includes_archived_but_not_purged`
- `entities_of_kind_filters_live_entities`
- `entities_with_name_returns_live_entities`
- `query_name_returns_sorted_pairs`
- `entities_with_agent_data_returns_live_entities`
- `query_agent_data_returns_sorted_pairs`
- `entities_with_name_and_agent_data_returns_intersection`
- `query_name_and_agent_data_returns_sorted_tuples`
- `filtering_query_preserves_relative_order`
- `empty_queries_are_safe`
- `count_helpers_report_live_totals`
- `query_results_are_deterministic_across_identical_sequences`

### Commands

```bash
cargo test -p worldwake-core world
cargo test -p worldwake-core
cargo clippy --workspace
cargo test --workspace
```

## Outcome

- Outcome amended: 2026-03-09
- Outcome amended: 2026-03-09
- Outcome amended: 2026-03-09
- Completion date: 2026-03-09
- What actually changed: added deterministic world-level entity iteration, live-only explicit query/count methods for `Name` and `AgentData`, explicit `Name + AgentData` intersection queries, and allocator support for ordered all-entity iteration including archived entities.
- Post-archive refinement: replaced duplicated per-component world query/count boilerplate with an internal macro that still generates explicit typed public methods. No public API or behavior changed.
- Post-archive refinement: collapsed the separate CRUD macro and query macro into a single per-component declaration macro in `World`, so each authoritative component now has one internal definition site for its full explicit API surface.
- Post-archive refinement: moved the authoritative component list into a shared schema macro consumed by both `ComponentTables` and `World`, so adding a new component now updates storage and world APIs from one declaration source.
- Deviations from original plan: did not add any generic `entities_with<T>()`-style API, and did not add `Place` component-table queries because that would conflict with the current explicit typed-table plus topology-owned-place architecture.
- Verification results: `cargo test -p worldwake-core world`, `cargo test -p worldwake-core`, `cargo clippy --workspace`, and `cargo test --workspace` all passed.
