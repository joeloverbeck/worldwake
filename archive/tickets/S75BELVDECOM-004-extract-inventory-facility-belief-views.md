# S75BELVDECOM-004: Extract InventoryBeliefView + FacilityBeliefView sub-traits

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes — RuntimeBeliefView trait decomposition, SnapshotActionFlags partial dissolution
**Deps**: archive/tickets/S75BELVDECOM-001-extract-control-belief-view.md, archive/tickets/S75BELVDECOM-002-extract-entity-profile-belief-views.md, archive/tickets/S75BELVDECOM-003-extract-spatial-temporal-belief-views.md

## Problem

Extract InventoryBeliefView (13 methods: possession, commodity, items, load, recipes) and FacilityBeliefView (6 methods: workstation, storage, resource sources, production) from RuntimeBeliefView. This batch also migrates `has_production_job` out of `SnapshotActionFlags` into the facility domain.

## Assumption Reassessment (2026-04-08)

1. `RuntimeBeliefView` already composes `ControlBeliefView + EntityBeliefView + ProfileBeliefView + SpatialBeliefView + TemporalBeliefView`, so this ticket is the next live trait-domain split rather than a fresh decomposition baseline.
2. InventoryBeliefView methods confirmed from the live `RuntimeBeliefView` surface (13): `direct_possessions`, `commodity_quantity`, `locally_observed_commodity_quantity`, `item_lot_commodity`, `item_lot_consumable_profile`, `direct_container`, `direct_possessor`, `carry_capacity`, `load_of_entity`, `knows_recipe`, `recipe_definition`, `unique_item_count`, `known_recipes`.
3. FacilityBeliefView methods confirmed from the live `RuntimeBeliefView` surface (6): `workstation_tag`, `stock_storage_policy`, `resource_source`, `has_production_job`, `matching_workstations_at`, `resource_sources_at`.
4. The live production ownership for these methods currently still sits on `RuntimeBeliefView` in `crates/worldwake-sim/src/per_agent_belief_view.rs` and `crates/worldwake-ai/src/planning_state.rs`; the same mock/UFCS/trait-import fallout pattern from 002 and 003 still applies.
5. `SnapshotActionFlags` in `crates/worldwake-ai/src/planning_snapshot.rs` still carries `has_production_job` alongside the two control booleans. This ticket should remove only the facility field, leaving the control flags in place until the later snapshot decomposition ticket.

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

## Outcome

Completed on 2026-04-08.

`InventoryBeliefView` and `FacilityBeliefView` now own the extracted inventory/facility reads in `crates/worldwake-sim/src/belief_view.rs`, `RuntimeBeliefView` composes them as supertraits, production impl ownership is split in `crates/worldwake-sim/src/per_agent_belief_view.rs` and `crates/worldwake-ai/src/planning_state.rs`, and `has_production_job` moved from `SnapshotActionFlags` onto `SnapshotEntity` in `crates/worldwake-ai/src/planning_snapshot.rs`. The remaining AI/sim/systems mock fallout, UFCS fallout, and golden harness/test-module trait-import fallout were migrated to the new trait boundary without adding shims.

Deviation from original plan: the ticket remained structurally scoped, but the actual fallout surface was broader than the initial "all 16 test mock files" shorthand and included additional AI test modules plus helper-method/import fallout in golden and test-only surfaces. No production-scope expansion beyond the ticket's domain boundary was required.

Verification passed with:

1. `cargo build --workspace`
2. `cargo test --workspace`
3. `cargo clippy --workspace --all-targets -- -D warnings`
