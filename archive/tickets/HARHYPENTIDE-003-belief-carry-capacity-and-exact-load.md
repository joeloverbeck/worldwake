# HARHYPENTIDE-003: Belief/snapshot carry-capacity and exact load support

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — planning snapshot/state and belief surface (`worldwake-ai`, `worldwake-sim`)
**Deps**: HARHYPENTIDE-002 (PlanningEntityRef needed for method signatures)
**Spec Reference**: HARDENING-hypothetical-entity-identity.md, Section B

## Problem

The planner cannot compute exact carry-fit math because it has no visibility into carry capacity or per-entity load. Without this data, the planner cannot determine whether a `pick_up` will fully move a lot, partially split it, or fail — making exact partial pickup impossible.

## Assumption Reassessment (2026-03-12)

1. `PlanningSnapshot` in `crates/worldwake-ai/src/planning_snapshot.rs` does not store carry capacity or per-entity load data — confirmed.
2. `PlanningState` already has `PlanningEntityRef`, `HypotheticalEntityId`, and hypothetical lot registry support from HARHYPENTIDE-002 — confirmed. This ticket must build on that model, not reintroduce raw-`EntityId` assumptions.
3. Authoritative carry capacity is currently stored as `CarryCapacity(LoadUnits)` in `worldwake-core` and consumed by `worldwake-systems/src/inventory.rs::remaining_capacity()` — confirmed.
4. Authoritative load accounting exists in `worldwake-core/src/load.rs` and `worldwake-systems/src/inventory.rs`, and actor carry math is recursive over possessed entities plus nested container contents — confirmed. The ticket's original "sum of direct possessions" assumption was too weak.
5. `BeliefView` currently has no carry-capacity or per-entity-load methods, and `PlanningSnapshot::build()` can only populate data exposed by `BeliefView` — confirmed. Extending `BeliefView` is required for snapshot population; the original out-of-scope note was incorrect.
6. `apply_pick_up_transition()` still moves the full lot today; exact partial pickup remains blocked on planner-visible carry math — confirmed, but the transition rework itself stays in HARHYPENTIDE-006.

## Architecture Check

1. Carry capacity should be stored in the snapshot because it is concrete authoritative state; remaining capacity must stay derived.
2. The snapshot should store authoritative per-entity intrinsic load, not a lot-only field. That keeps the planner aligned with current authoritative semantics for lots, unique items, and zero-load non-items without encoding item-shape special cases into the planner surface.
3. Remaining carry capacity in `PlanningState` must mirror authoritative recursive carried-load semantics across possessions and nested container contents, including hypothetical lots introduced by HARHYPENTIDE-002.
4. `BeliefView` must grow the minimal concrete methods needed to populate this snapshot data. That is an architectural requirement, not an optional convenience.
5. This work extends existing snapshot/state structures; no new component tables or planner-only cargo score abstractions are justified.

## What to Change

### 1. Extend `BeliefView` with concrete carry/load accessors

In `crates/worldwake-sim/src/belief_view.rs`, add:

```rust
fn carry_capacity(&self, entity: EntityId) -> Option<LoadUnits>;
fn load_of_entity(&self, entity: EntityId) -> Option<LoadUnits>;
```

Implement them for:

- `OmniscientBeliefView`
- `PlanningState`
- test belief stubs touched by compilation

These are concrete state accessors, not planner-specific helpers.

### 2. Extend `SnapshotEntity` with carry/load fields

In `crates/worldwake-ai/src/planning_snapshot.rs`, add to `SnapshotEntity`:

```rust
pub carry_capacity: Option<LoadUnits>,
pub intrinsic_load: LoadUnits,
```

Populate during `build_planning_snapshot` from `BeliefView::carry_capacity()` and `BeliefView::load_of_entity()`.

### 3. Add `PlanningEntityRef`-aware load methods to `PlanningState`

```rust
pub fn carry_capacity_ref(&self, entity: PlanningEntityRef) -> Option<LoadUnits>
pub fn load_of_entity_ref(&self, entity: PlanningEntityRef) -> Option<LoadUnits>
pub fn remaining_carry_capacity_ref(&self, agent: PlanningEntityRef) -> Option<LoadUnits>
```

- `carry_capacity_ref`: for authoritative refs, read snapshot; for hypothetical refs, return `None` unless a future ticket introduces hypothetical carriers.
- `load_of_entity_ref`: for authoritative refs, read snapshot `intrinsic_load`; for hypothetical lots, compute from commodity kind and quantity in overrides using `load_per_unit()`. For non-item entities, return their intrinsic load if known or `LoadUnits(0)` when they are concrete non-items.
- `remaining_carry_capacity_ref`: compute capacity minus recursive carried load using the same possession/container traversal shape as `worldwake-systems/src/inventory.rs`.

### 4. Ensure `load_per_unit` is accessible from `worldwake-ai`

`load_per_unit()` is in `worldwake-core/src/load.rs` which is a dependency of `worldwake-ai`. Verify the function is public and usable. No new dependency needed.

## Files to Touch

- `crates/worldwake-ai/src/planning_snapshot.rs` (modify — add carry capacity and intrinsic load fields to `SnapshotEntity`, populate in `build_planning_snapshot`)
- `crates/worldwake-ai/src/planning_state.rs` (modify — add `carry_capacity_ref`, `load_of_entity_ref`, `remaining_carry_capacity_ref` methods)
- `crates/worldwake-sim/src/belief_view.rs` (modify — add carry/load accessors)
- `crates/worldwake-sim/src/omniscient_belief_view.rs` (modify — expose authoritative carry/load values)

## Out of Scope

- Exact pickup transition logic (that is HARHYPENTIDE-006)
- Changes to `worldwake-core/src/load.rs` (already has what we need)
- Changing the authoritative storage model for carry capacity (for example replacing `CarryCapacity` with container-derived semantics). If that architecture should change, it needs its own follow-up ticket/spec update.

## Acceptance Criteria

### Tests That Must Pass

1. `SnapshotEntity` populated with correct `carry_capacity` from world state.
2. `SnapshotEntity` populated with correct `intrinsic_load` for authoritative entities, including item lots and non-item zero-load entities.
3. `carry_capacity_ref(Authoritative(agent))` returns the agent's capacity.
4. `load_of_entity_ref(Authoritative(entity))` returns the authoritative intrinsic load from snapshot data.
5. `load_of_entity_ref(Hypothetical(h))` computes load from commodity kind and quantity overrides.
6. `remaining_carry_capacity_ref` returns capacity minus total recursive carried load, including nested contents and hypothetical possessions.
7. Full-fit detection: lot load <= remaining capacity.
8. Partial-fit detection: lot load > remaining capacity, per-unit load <= remaining capacity.
9. Zero-fit detection: per-unit load > remaining capacity.
10. Existing suite: `cargo test --workspace`
11. Existing lint: `cargo clippy --workspace`

### Invariants

1. Carry capacity is concrete stored state in the snapshot, not derived at query time.
2. Remaining capacity is always derived, never stored.
3. Authoritative intrinsic load remains sourced from the same authoritative load semantics as `worldwake-core`; hypothetical lot load uses the same `load_per_unit()` table as authoritative code.
4. No floating-point math — all load values are `LoadUnits(u32)`.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/planning_snapshot.rs` — snapshot correctly captures carry capacity and intrinsic load.
2. `crates/worldwake-ai/src/planning_state.rs` — full-fit, partial-fit, and zero-fit carry math tests; remaining capacity with nested and hypothetical possessions.
3. `crates/worldwake-sim/src/omniscient_belief_view.rs` — carry/load accessors expose authoritative values.

### Commands

1. `cargo test -p worldwake-ai planning_snapshot`
2. `cargo test -p worldwake-ai planning_state`
3. `cargo test --workspace`
4. `cargo clippy --workspace`

## Outcome

- Completion date: 2026-03-12
- What actually changed:
  - Added `carry_capacity()` and `load_of_entity()` to `BeliefView`.
  - Populated `PlanningSnapshot` with concrete `carry_capacity` and authoritative `intrinsic_load`.
  - Added `PlanningState::{carry_capacity_ref, load_of_entity_ref, remaining_carry_capacity_ref}` with recursive carried-load traversal across possessions, nested contents, and hypothetical lots.
  - Added focused tests in `planning_snapshot`, `planning_state`, and `omniscient_belief_view`.
- Deviations from original plan:
  - Replaced the proposed lot-only `lot_load` field with a general `intrinsic_load` field because the planner must mirror authoritative load semantics for lots, unique items, containers, and zero-load non-items without special-casing item shapes.
  - Extended `BeliefView` after reassessment; snapshot population could not be implemented cleanly without widening the concrete belief surface.
  - Preserved current authoritative `CarryCapacity` semantics instead of rewriting carry capacity onto container-derived state. The hardening spec's container wording does not match the current runtime and should be reconciled separately if the architecture is meant to change.
- Verification results:
  - `cargo test -p worldwake-ai planning_snapshot`
  - `cargo test -p worldwake-ai planning_state`
  - `cargo test -p worldwake-sim omniscient_belief_view`
  - `cargo test --workspace`
  - `cargo clippy --workspace`
