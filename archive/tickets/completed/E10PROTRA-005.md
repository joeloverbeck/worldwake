# E10PROTRA-005: WorkstationMarker component + ProductionJob component in worldwake-core

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — two new authoritative component registrations
**Deps**: `archive/tickets/completed/E10PROTRA-001.md` (completed shared-schema extraction for `WorkstationTag` and `RecipeId`)

## Problem

Two related components are needed for production concurrency and work-in-progress tracking:

1. **WorkstationMarker**: A component on `Facility` entities that tags them as a specific type of workstation. Concurrency at a place is derived from the number of unreserved matching workstation entities — not from an abstract slot count (Principle 3).

2. **ProductionJob**: Persistent work-in-progress state. When a job starts, inputs are staged, and progress accumulates. If interrupted, the job persists — partial work and staged inputs do not silently disappear. This replaces "partial progress lost" hand-waving with traceable state.

## Assumption Reassessment (2026-03-10)

1. `WorkstationTag` and `RecipeId` already exist in `crates/worldwake-core/src/production.rs` from `archive/tickets/completed/E10PROTRA-001.md` — confirmed.
2. `EntityKind::Facility` exists — confirmed.
3. The reservation system exists in `relations.rs` — workstation reservation will use the existing reservation infrastructure.
4. `Container` already exists as an authoritative component on `EntityKind::Container`, so `ProductionJob.staged_inputs_container` can point at a real container entity rather than an abstract buffer.
5. `crates/worldwake-core/src/component_schema.rs` is the single authoritative declaration point for ECS components, and its macro fanout affects `component_tables.rs`, `world.rs`, `delta.rs`, and `world_txn.rs`.
6. `component_schema.rs` now also drives transaction-layer simple-component setters/clearers through `select_txn_simple_set_components!`; this ticket must wire both components into `WorldTxn`, not just the world tables.
7. Workspace verification currently includes a systems-level exact `ComponentKind::ALL` expectation in `crates/worldwake-systems/tests/e09_needs_integration.rs`; adding authoritative components requires updating that mirror too.
8. `ProductionJob` should live on the workstation entity (`EntityKind::Facility`), but the component should not also store a duplicate `workstation: EntityId` field. The owning entity already provides that identity, and duplicating it would create unnecessary drift risk.

## Architecture Check

1. `WorkstationMarker(WorkstationTag)` is a thin wrapper component. "Available workstations at a place" is a derived read-model (query unreserved Facility entities with matching WorkstationMarker at the same location).
2. `ProductionJob` on a Facility entity is cleaner than a standalone job entity right now: no new `EntityKind` is needed, and the one-to-one relationship between workstation and active job is enforced by the component system.
3. Because the job is stored on the workstation entity, `ProductionJob.workstation` would be redundant authoritative state. The cleaner long-lived architecture is to derive the workstation from the host entity and only store the data that cannot be derived elsewhere.
4. Both are authoritative stored state per Principle 3.
5. Both should participate in the standard typed `WorldTxn` component-delta pipeline now. Deferring that would force later E10 craft/harvest work to invent an ad hoc mutation path for workstation/job state.
6. Grouping is justified: both are production-domain Facility components with the same schema surface and lifecycle.

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
    pub staged_inputs_container: EntityId,
    pub progress_ticks: u32,
}
impl Component for ProductionJob {}
```

### 2. Register both in `component_schema.rs`

Both restricted to `EntityKind::Facility`.

### 3. Schema fanout + exports

Update `delta.rs`, `component_tables.rs`, `world.rs`, `world_txn.rs`, and `lib.rs`.

### 4. Workspace-wide schema expectation

Update any exact `ComponentKind::ALL` expectations outside `worldwake-core` that intentionally mirror the authoritative schema inventory.

## Files to Touch

- `crates/worldwake-core/src/production.rs` (modify — add WorkstationMarker, ProductionJob)
- `crates/worldwake-core/src/component_schema.rs` (modify — add 2 component registrations)
- `crates/worldwake-core/src/lib.rs` (modify — re-exports)
- `crates/worldwake-core/src/component_tables.rs` (modify — schema fanout)
- `crates/worldwake-core/src/world.rs` (modify — generated API tests)
- `crates/worldwake-core/src/delta.rs` (modify — component inventory coverage)
- `crates/worldwake-core/src/world_txn.rs` (modify — transaction-layer setter/clearer coverage)
- `crates/worldwake-systems/tests/e09_needs_integration.rs` (modify only if needed — authoritative schema expectation)

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
6. `ProductionJob` correctly stores its four authoritative fields (`recipe_id`, `worker`, `staged_inputs_container`, `progress_ticks`).
7. `ComponentKind::ALL` and `ComponentValue` coverage include both new components.
8. `WorldTxn::set_component_workstation_marker(...)` and `WorldTxn::set_component_production_job(...)` record typed `ComponentDelta::Set` values and update the world on commit.
9. `WorldTxn::clear_component_workstation_marker(...)` and `WorldTxn::clear_component_production_job(...)` record typed `ComponentDelta::Removed` values and update the world on commit.
10. Existing suite: `cargo test -p worldwake-core`

### Invariants

1. Facility-only components.
2. Authoritative stored state — "available workstations" and "job progress percentage" are derived.
3. `ProductionJob.progress_ticks` is a raw `u32` counter, not a `Permille` — progress percentage is derived.
4. No floating-point types.
5. No `HashMap`/`HashSet`.
6. No duplicate `workstation` field inside `ProductionJob`; workstation identity is derived from the owning entity.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/production.rs` — construction, serialization, trait bounds for both types
2. `crates/worldwake-core/src/component_tables.rs` — table CRUD for both
3. `crates/worldwake-core/src/world.rs` — kind-restricted insertion + wrong-kind rejection for both
4. `crates/worldwake-core/src/delta.rs` — component inventory coverage
5. `crates/worldwake-core/src/world_txn.rs` — typed set/clear delta coverage for both
6. `crates/worldwake-systems/tests/e09_needs_integration.rs` — authoritative schema expectation, if impacted

### Commands

1. `cargo test -p worldwake-core`
2. `cargo clippy --workspace --all-targets -- -D warnings`
3. `cargo test --workspace`

## Outcome

- Completion date: 2026-03-10
- What actually changed:
  - Added `WorkstationMarker(WorkstationTag)` and `ProductionJob` to `crates/worldwake-core/src/production.rs`.
  - Registered both as Facility-only authoritative components in the shared schema and re-exported them from `worldwake-core`.
  - Extended schema fanout through `component_tables.rs`, `world.rs`, `delta.rs`, and `world_txn.rs`.
  - Updated the systems-level exact `ComponentKind::ALL` expectation in `crates/worldwake-systems/tests/e09_needs_integration.rs`.
  - Added focused tests for serialization, table CRUD, world kind enforcement, delta inventory coverage, and typed transaction set/clear behavior.
- Deviations from original plan:
  - Corrected the ticket first to match the current architecture: this work also needed `world_txn.rs` coverage and the downstream schema mirror test, not just the core schema fanout files originally listed.
  - Removed the proposed `workstation: EntityId` field from `ProductionJob`. Because the component lives on the workstation entity, storing the workstation id again would duplicate authoritative state and create avoidable drift risk.
  - Kept the workstation/job model as Facility-owned components rather than introducing a separate job entity. At the current scale that is the cleaner, more robust architecture: one workstation, one active job, enforced directly by the component system without another entity layer.
- Verification results:
  - `cargo test -p worldwake-core workstation_marker` ✅
  - `cargo test -p worldwake-core production_job` ✅
  - `cargo test -p worldwake-core` ✅
  - `cargo test -p worldwake-systems authoritative_schema_includes_expected_shared_and_e09_components_and_fields` ✅
  - `cargo clippy --workspace --all-targets -- -D warnings` ✅
  - `cargo test --workspace` ✅
