# S75BELVDECOM-004: Extract InventoryBeliefView + FacilityBeliefView sub-traits

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes — RuntimeBeliefView trait decomposition, SnapshotActionFlags partial dissolution
**Deps**: S75BELVDECOM-001

## Problem

Extract InventoryBeliefView (13 methods: possession, commodity, items, load, recipes) and FacilityBeliefView (6 methods: workstation, storage, resource sources, production) from RuntimeBeliefView. This batch also migrates `has_production_job` out of `SnapshotActionFlags` into the facility domain.

## Assumption Reassessment (2026-04-08)

1. InventoryBeliefView methods confirmed (13): `direct_possessions`, `commodity_quantity`, `locally_observed_commodity_quantity`, `item_lot_commodity`, `item_lot_consumable_profile`, `direct_container`, `direct_possessor`, `carry_capacity`, `load_of_entity`, `knows_recipe`, `recipe_definition`, `unique_item_count`, `known_recipes`.
2. FacilityBeliefView methods confirmed (6): `workstation_tag`, `stock_storage_policy`, `resource_source`, `has_production_job`, `matching_workstations_at`, `resource_sources_at`.
3. `SnapshotActionFlags` at `planning_snapshot.rs:213` has `has_production_job` (FacilityBeliefView), `controllable_by_actor`, `has_control` (ControlBeliefView). After this ticket, only the 2 control fields remain in SnapshotActionFlags (or they were already moved by ticket 001 if it addressed them).

## Architecture Check

1. Same supertrait pattern. No backward-compatibility shims.
2. Migrating `has_production_job` from SnapshotActionFlags prepares for the full sub-struct decomposition in ticket 007.

## Verification Layers

1. Inventory queries -> golden tests exercise commodity/possession queries via trade, production, and crafting planning
2. Facility queries -> golden tests exercise workstation matching and resource source queries via production planning
3. Compile-time proof -> `cargo build --workspace`

## What to Change

### 1. Define InventoryBeliefView and FacilityBeliefView sub-traits

Move 13 inventory and 6 facility method signatures from RuntimeBeliefView.

### 2. Add supertrait bounds and remove methods from RuntimeBeliefView

### 3. Update all 18 impl blocks

### 4. Migrate `has_production_job` from SnapshotActionFlags

In `planning_snapshot.rs`, move the `has_production_job` field from `SnapshotActionFlags` to a top-level field on `SnapshotEntity` (it will move into `SnapshotFacility` sub-struct in ticket 007). Update all `entity.action_flags.has_production_job` accesses to `entity.has_production_job`.

### 5. Export new sub-traits

## Files to Touch

- `crates/worldwake-sim/src/belief_view.rs` (modify)
- `crates/worldwake-sim/src/lib.rs` (modify — exports)
- `crates/worldwake-sim/src/per_agent_belief_view.rs` (modify)
- `crates/worldwake-ai/src/planning_state.rs` (modify)
- `crates/worldwake-ai/src/planning_snapshot.rs` (modify — impl blocks + SnapshotActionFlags partial dissolution)
- All 16 test mock files (modify)

## Out of Scope

- Other domain sub-trait extractions
- Full SnapshotEntity sub-struct decomposition (ticket 007)
- GoalBeliefView changes (ticket 008)

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test --workspace`
2. `cargo clippy --workspace --all-targets -- -D warnings`

### Invariants

1. `&dyn RuntimeBeliefView` usable at all existing call sites.
2. No behavioral change.

## Test Plan

### New/Modified Tests

1. None — pure structural refactor.

### Commands

1. `cargo build --workspace`
2. `cargo test --workspace`
3. `cargo clippy --workspace --all-targets -- -D warnings`
