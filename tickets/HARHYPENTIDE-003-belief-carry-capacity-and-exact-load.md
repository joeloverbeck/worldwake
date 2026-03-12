# HARHYPENTIDE-003: Belief/snapshot carry-capacity and exact load support

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — planning snapshot and belief surface (`worldwake-ai`)
**Deps**: HARHYPENTIDE-002 (PlanningEntityRef needed for method signatures)
**Spec Reference**: HARDENING-hypothetical-entity-identity.md, Section B

## Problem

The planner cannot compute exact carry-fit math because it has no visibility into carry capacity or per-entity load. Without this data, the planner cannot determine whether a `pick_up` will fully move a lot, partially split it, or fail — making exact partial pickup impossible.

## Assumption Reassessment (2026-03-12)

1. `PlanningSnapshot` in `crates/worldwake-ai/src/planning_snapshot.rs` does not store carry capacity or per-entity load data — confirmed.
2. `PlanningState` has no `carry_capacity` or `load_of_entity` methods — confirmed.
3. Authoritative carry capacity is stored as `CarryCapacity(LoadUnits)` component in `worldwake-core` — confirmed.
4. Load accounting exists in `worldwake-core/src/load.rs`: `load_per_unit()`, `load_of_lot()`, `load_of_entity()`, `remaining_container_capacity()` — confirmed.
5. `remaining_capacity()` in `worldwake-systems/src/inventory.rs` computes remaining carry capacity for an actor — confirmed.
6. `BeliefView` trait has no carry capacity or load methods — confirmed.

## Architecture Check

1. Adding carry capacity to the snapshot and derived load computation to `PlanningState` follows the stored-vs-derived distinction: capacity is stored in the snapshot, remaining capacity is derived.
2. The planner must compute load recursively over direct possessions, using the same `load_per_unit()` function from `worldwake-core/src/load.rs` to stay consistent with authoritative semantics.
3. This data extends the existing `SnapshotEntity` struct — no new component tables needed.

## What to Change

### 1. Extend `SnapshotEntity` with carry capacity

In `crates/worldwake-ai/src/planning_snapshot.rs`, add to `SnapshotEntity`:

```rust
pub carry_capacity: Option<LoadUnits>,
pub lot_load: Option<LoadUnits>,  // pre-computed load of this entity if it's an item lot
```

Populate during `build_planning_snapshot` from the world's `CarryCapacity` component and `load_of_entity()`.

### 2. Add `PlanningEntityRef`-aware load methods to `PlanningState`

```rust
pub fn carry_capacity_ref(&self, entity: PlanningEntityRef) -> Option<LoadUnits>
pub fn load_of_entity_ref(&self, entity: PlanningEntityRef) -> Option<LoadUnits>
pub fn remaining_carry_capacity_ref(&self, agent: PlanningEntityRef) -> Option<LoadUnits>
```

- `carry_capacity_ref`: for authoritative refs, read from snapshot; for hypothetical refs, return `None` (hypothetical lots don't have carry capacity).
- `load_of_entity_ref`: for authoritative refs, read snapshot `lot_load`; for hypothetical lots, compute from commodity kind and quantity in overrides using `load_per_unit()`.
- `remaining_carry_capacity_ref`: compute carry capacity minus sum of loads of all direct possessions (both authoritative and hypothetical).

### 3. Ensure `load_per_unit` is accessible from `worldwake-ai`

`load_per_unit()` is in `worldwake-core/src/load.rs` which is a dependency of `worldwake-ai`. Verify the function is public and usable. No new dependency needed.

## Files to Touch

- `crates/worldwake-ai/src/planning_snapshot.rs` (modify — add carry capacity and lot load fields to `SnapshotEntity`, populate in `build_planning_snapshot`)
- `crates/worldwake-ai/src/planning_state.rs` (modify — add `carry_capacity_ref`, `load_of_entity_ref`, `remaining_carry_capacity_ref` methods)

## Out of Scope

- Adding carry capacity to `BeliefView` trait (not needed — planner uses `PlanningState` methods directly)
- Exact pickup transition logic (that is HARHYPENTIDE-006)
- Changes to `worldwake-core/src/load.rs` (already has what we need)
- Changes to `worldwake-sim` or `worldwake-systems`

## Acceptance Criteria

### Tests That Must Pass

1. `SnapshotEntity` populated with correct `carry_capacity` from world state.
2. `SnapshotEntity` populated with correct `lot_load` for item lots.
3. `carry_capacity_ref(Authoritative(agent))` returns the agent's capacity.
4. `load_of_entity_ref(Authoritative(lot))` returns the lot's load.
5. `load_of_entity_ref(Hypothetical(h))` computes load from commodity kind and quantity overrides.
6. `remaining_carry_capacity_ref` returns capacity minus total carried load (both authoritative and hypothetical possessions).
7. Full-fit detection: lot load <= remaining capacity.
8. Partial-fit detection: lot load > remaining capacity, per-unit load <= remaining capacity.
9. Zero-fit detection: per-unit load > remaining capacity.
10. Existing suite: `cargo test --workspace`
11. Existing lint: `cargo clippy --workspace`

### Invariants

1. Carry capacity is concrete stored state in the snapshot, not derived at query time.
2. Remaining capacity is always derived, never stored.
3. Load computation uses the same `load_per_unit()` as authoritative code (consistency).
4. No floating-point math — all load values are `LoadUnits(u32)`.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/planning_snapshot.rs` — snapshot correctly captures carry capacity and lot load.
2. `crates/worldwake-ai/src/planning_state.rs` — full-fit, partial-fit, and zero-fit carry math tests; remaining capacity with hypothetical possessions.

### Commands

1. `cargo test -p worldwake-ai planning_snapshot`
2. `cargo test -p worldwake-ai planning_state`
3. `cargo test --workspace`
4. `cargo clippy --workspace`
