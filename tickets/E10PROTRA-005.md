# E10PROTRA-005: WorkstationMarker component + ProductionJob component in worldwake-core

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — two new authoritative component registrations
**Deps**: E10PROTRA-001 (WorkstationTag and RecipeId must exist)

## Problem

Two related components are needed for production concurrency and work-in-progress tracking:

1. **WorkstationMarker**: A component on `Facility` entities that tags them as a specific type of workstation. Concurrency at a place is derived from the number of unreserved matching workstation entities — not from an abstract slot count (Principle 3).

2. **ProductionJob**: Persistent work-in-progress state. When a job starts, inputs are staged, and progress accumulates. If interrupted, the job persists — partial work and staged inputs do not silently disappear. This replaces "partial progress lost" hand-waving with traceable state.

## Assumption Reassessment (2026-03-10)

1. `WorkstationTag` and `RecipeId` will exist after E10PROTRA-001 — confirmed dependency.
2. `EntityKind::Facility` exists — confirmed.
3. The reservation system exists in `relations.rs` — workstation reservation will use the existing reservation infrastructure.
4. `ProductionJob` tracks: recipe_id, worker, workstation, staged_inputs_container, progress_ticks. The `staged_inputs_container` is an `EntityId` pointing to a `Container` entity.
5. `ProductionJob` goes on the workstation entity (`EntityKind::Facility`) — this naturally enforces "one job per workstation" when combined with reservation.

## Architecture Check

1. `WorkstationMarker(WorkstationTag)` is a thin wrapper component. "Available workstations at a place" is a derived read-model (query unreserved Facility entities with matching WorkstationMarker at the same location).
2. `ProductionJob` on a Facility entity is cleaner than a standalone job entity — no new EntityKind needed, and the one-to-one relationship between workstation and active job is enforced by the component system.
3. Both are authoritative stored state per Principle 3.
4. Grouping is justified: both are production-domain Facility components.

## What to Change

### 1. Add to `crates/worldwake-core/src/production.rs`

```rust
/// Marks a Facility entity as a workstation of a specific type.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub struct WorkstationMarker(pub WorkstationTag);
impl Component for WorkstationMarker {}

/// Persistent work-in-progress state on a workstation.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ProductionJob {
    pub recipe_id: RecipeId,
    pub worker: EntityId,
    pub workstation: EntityId,
    pub staged_inputs_container: EntityId,
    pub progress_ticks: u32,
}
impl Component for ProductionJob {}
```

### 2. Register both in `component_schema.rs`

Both restricted to `EntityKind::Facility`.

### 3. Schema fanout + exports

Update `delta.rs`, `component_tables.rs`, `world.rs`, `lib.rs`.

## Files to Touch

- `crates/worldwake-core/src/production.rs` (modify — add WorkstationMarker, ProductionJob)
- `crates/worldwake-core/src/component_schema.rs` (modify — add 2 component registrations)
- `crates/worldwake-core/src/lib.rs` (modify — re-exports)
- `crates/worldwake-core/src/component_tables.rs` (modify — schema fanout)
- `crates/worldwake-core/src/world.rs` (modify — generated API tests)
- `crates/worldwake-core/src/delta.rs` (modify — component inventory coverage)

## Out of Scope

- Job start/complete/abandon logic (E10PROTRA-008, E10PROTRA-009)
- Reservation logic for workstations (handled by existing relation reservation APIs)
- Staged container creation (handled by action logic in systems crate)
- RecipeDefinition/RecipeRegistry (E10PROTRA-006)
- AI awareness of workstations (E13)

## Acceptance Criteria

### Tests That Must Pass

1. `WorkstationMarker` can be inserted/retrieved/removed on Facility entities through the `World` API.
2. `WorkstationMarker` insertion is rejected for non-Facility kinds (Agent, Place, etc.).
3. `ProductionJob` can be inserted/retrieved/removed on Facility entities through the `World` API.
4. `ProductionJob` insertion is rejected for non-Facility kinds.
5. Both round-trip through bincode.
6. `ProductionJob` correctly stores all five fields (recipe_id, worker, workstation, staged_inputs_container, progress_ticks).
7. `ComponentKind::ALL` and `ComponentValue` coverage include both new components.
8. Existing suite: `cargo test -p worldwake-core`

### Invariants

1. Facility-only components.
2. Authoritative stored state — "available workstations" and "job progress percentage" are derived.
3. `ProductionJob.progress_ticks` is a raw `u32` counter, not a `Permille` — progress percentage is derived.
4. No floating-point types.
5. No `HashMap`/`HashSet`.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/production.rs` — construction, serialization, trait bounds for both types
2. `crates/worldwake-core/src/component_tables.rs` — table CRUD for both
3. `crates/worldwake-core/src/world.rs` — kind-restricted insertion + wrong-kind rejection for both
4. `crates/worldwake-core/src/delta.rs` — component inventory coverage

### Commands

1. `cargo test -p worldwake-core`
2. `cargo clippy --workspace --all-targets -- -D warnings`
3. `cargo test --workspace`
