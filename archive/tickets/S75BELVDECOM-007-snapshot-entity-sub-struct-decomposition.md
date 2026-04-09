# S75BELVDECOM-007: SnapshotEntity domain sub-struct decomposition

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes — SnapshotEntity struct layout in worldwake-ai
**Deps**: archive/tickets/S75BELVDECOM-002-extract-entity-profile-belief-views.md, archive/tickets/S75BELVDECOM-003-extract-spatial-temporal-belief-views.md, archive/tickets/S75BELVDECOM-004-extract-inventory-facility-belief-views.md, archive/tickets/S75BELVDECOM-005-extract-combat-economic-belief-views.md, archive/tickets/S75BELVDECOM-006-extract-social-political-belief-views.md

## Problem

After all sub-traits are extracted (tickets 001-006), SnapshotEntity remains a flat struct with ~44 fields. This ticket reorganizes those fields into domain sub-structs that mirror the 11 sub-traits, making the RuntimeBeliefView ↔ SnapshotEntity projection relationship explicit and auditable.

## Assumption Reassessment (2026-04-09)

1. `SnapshotEntity` was still the remaining flat planner mirror after tickets 002-006, and the live read surface included not just `planning_snapshot.rs` / `planning_state.rs` but also direct field reads in `goal_model.rs`, `planner_ops.rs`, and `search/candidates.rs`.
2. `SnapshotLifecycle` and `SnapshotActionFlags` had already been dissolved before this ticket executed. The live remaining shape debt was the flat `SnapshotEntity` itself.
3. All direct `SnapshotEntity` accesses remained inside `crates/worldwake-ai/` because the type is `pub(crate)`, so the blast radius stayed intra-crate.

## Architecture Check

1. Struct-of-structs layout preserves cache locality for sequential field access within a domain. Rust lays out inner structs inline (no heap indirection). This is cleaner than the flat 44-field struct because it groups fields by the sub-trait they serve, making the projection auditable.
2. No backward-compatibility shims. The flat struct is replaced, not wrapped. P12 compliance: no causal paths change; only struct layout.

## Verification Layers

1. SnapshotEntity field access correctness -> `cargo build --workspace` (compile-time proof all accesses updated)
2. Planning behavior unchanged -> `cargo test -p worldwake-ai` (all golden and unit tests pass)
3. Single-crate ticket — SnapshotEntity is `pub(crate)` in worldwake-ai, so blast radius is contained.

## What to Change

### 1. Define domain sub-structs

In `crates/worldwake-ai/src/planning_snapshot.rs`, define 11 sub-structs:

```rust
pub(crate) struct SnapshotEntityCore { /* kind, alive, dead, incapacitated, ... */ }
pub(crate) struct SnapshotSpatial { /* effective_place, in_transit_state */ }
pub(crate) struct SnapshotInventory { /* direct_container, direct_possessions, commodity_quantities, ... */ }
pub(crate) struct SnapshotCombat { /* wounds, hostile_targets, visible_hostiles, combat_profile, courage, ... */ }
pub(crate) struct SnapshotSocial { /* (if any fields cached for social queries) */ }
pub(crate) struct SnapshotEconomic { /* demand_memory, merchandise_profile, has_sale_listing, ... */ }
pub(crate) struct SnapshotPolitical { /* record_data, office_data */ }
pub(crate) struct SnapshotTemporal { /* reservation_ranges, facility_queue */ }
pub(crate) struct SnapshotProfiles { /* homeostatic_needs, drive_thresholds, metabolism_profile, ... */ }
pub(crate) struct SnapshotFacility { /* workstation_tag, stock_storage_policy, resource_source, has_production_job */ }
pub(crate) struct SnapshotControl { /* controllable_by_actor, has_control */ }
```

Each sub-struct derives `Clone, Debug, Eq, PartialEq` to match SnapshotEntity.

### 2. Replace flat SnapshotEntity with composed struct

```rust
pub(crate) struct SnapshotEntity {
    pub entity: SnapshotEntityCore,
    pub spatial: SnapshotSpatial,
    pub inventory: SnapshotInventory,
    pub combat: SnapshotCombat,
    pub social: SnapshotSocial,
    pub economic: SnapshotEconomic,
    pub political: SnapshotPolitical,
    pub temporal: SnapshotTemporal,
    pub profiles: SnapshotProfiles,
    pub facility: SnapshotFacility,
    pub control: SnapshotControl,
}
```

### 3. Update all field accesses

Find-and-replace all `entity.field_name` accesses to use the domain prefix (e.g., `entity.effective_place` → `entity.spatial.effective_place`). Since SnapshotEntity is `pub(crate)`, all accesses are within worldwake-ai:

- `planning_snapshot.rs` — snapshot construction and RuntimeBeliefView impl for PlanningState
- `planning_state.rs` — PlanningState reads from snapshot
- Other worldwake-ai files that construct or read SnapshotEntity fields

### 4. Update SnapshotEntityFilter if needed

`SnapshotEntityFilter` at `planning_snapshot.rs:54` may need corresponding updates if it references SnapshotEntity field names.

## Files to Touch

- `crates/worldwake-ai/src/planning_snapshot.rs` (modify — define sub-structs, restructure SnapshotEntity, update construction + impl)
- `crates/worldwake-ai/src/planning_state.rs` (modify — update field accesses in RuntimeBeliefView impl)
- Any other worldwake-ai files that directly access SnapshotEntity fields (grep for `SnapshotEntity` to identify)

## Out of Scope

- Sub-trait definitions (already done in tickets 001-006)
- GoalBeliefView changes (ticket 008)
- Cross-crate changes (SnapshotEntity is crate-internal)

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-ai` — all golden and unit tests pass
2. `cargo test --workspace`
3. `cargo clippy --workspace --all-targets -- -D warnings`

### Invariants

1. Every SnapshotEntity sub-struct field corresponds to exactly one sub-trait method.
2. No SnapshotEntity field is orphaned (not mapped to a sub-trait).
3. No behavioral change — struct layout only.

## Test Plan

### New/Modified Tests

1. None — pure structural refactor. Golden tests (golden_soak, golden_resilience, golden_production, etc.) serve as comprehensive behavior proofs.

### Commands

1. `cargo build --workspace`
2. `cargo test -p worldwake-ai`
3. `cargo test --workspace`
4. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

Completed on 2026-04-09.

- Implemented the `SnapshotEntity` decomposition in `crates/worldwake-ai/src/planning_snapshot.rs` by replacing the flat planner mirror with domain-shaped sub-structs: `SnapshotEntityCore`, `SnapshotSpatial`, `SnapshotInventory`, `SnapshotCombat`, `SnapshotSocial`, `SnapshotEconomic`, `SnapshotPolitical`, `SnapshotTemporal`, `SnapshotProfiles`, `SnapshotFacility`, and `SnapshotControl`. `SnapshotEntity` now composes those sub-structs inline, and `build_snapshot_entity` populates them directly from the belief-view domain boundaries established by tickets 002-006.
- Updated all live planner-side field reads to the new domain prefixes in `crates/worldwake-ai/src/planning_state.rs`, plus the remaining direct snapshot readers in `crates/worldwake-ai/src/goal_model.rs`, `crates/worldwake-ai/src/planner_ops.rs`, and `crates/worldwake-ai/src/search/candidates.rs`. Snapshot-focused tests in `crates/worldwake-ai/src/planning_snapshot.rs` and planner-state tests were migrated to the new nested layout.
- Deviation from the original ticket wording: the live read surface was broader than the initial file list, and `SnapshotActionFlags` had already been removed before this ticket executed, so implementation was carried through the real remaining access sites instead of the older assumption set.

No behavior or planner data content changed; the ticket stayed a shape-only refactor.

## Verification

1. `cargo fmt --all`
2. `cargo build --workspace`
3. `cargo test -p worldwake-ai`
4. `cargo test --workspace`
5. `cargo clippy --workspace --all-targets -- -D warnings`
