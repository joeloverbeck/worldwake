# S75BELVDECOM-002: Extract EntityBeliefView + ProfileBeliefView sub-traits

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes — RuntimeBeliefView trait decomposition, SnapshotLifecycle dissolution
**Deps**: archive/tickets/S75BELVDECOM-001-extract-control-belief-view.md

## Problem

Continue RuntimeBeliefView decomposition by extracting EntityBeliefView (8 methods: lifecycle queries) and ProfileBeliefView (5 methods: needs/metabolism/preferences). This batch also dissolves `SnapshotLifecycle` into `SnapshotEntityCore` since `alive`/`dead`/`incapacitated` are EntityBeliefView fields.

## Assumption Reassessment (2026-04-08)

1. `ControlBeliefView` was already extracted by `S75BELVDECOM-001`; live `RuntimeBeliefView` in `crates/worldwake-sim/src/belief_view.rs` still owns the entity/profile methods this ticket targets.
2. Entity-domain methods still on `RuntimeBeliefView` are `is_alive`, `is_dead`, `is_incapacitated`, `entity_kind`, `corpse_entities_at`, `bandit_flee_wound_threshold`, `bandit_camp_establishment_ticks`, and `locally_observed_is_dead`.
3. Profile-domain methods still on `RuntimeBeliefView` are `homeostatic_needs`, `drive_thresholds`, `metabolism_profile`, `preference_profile`, and `utility_profile`.
4. `SnapshotLifecycle` still exists in `crates/worldwake-ai/src/planning_snapshot.rs`, and `PlanningState` still reads `snapshot.lifecycle.*`; those fields should move onto `SnapshotEntity` in this ticket.

## Architecture Check

1. Same supertrait pattern as 001. EntityBeliefView and ProfileBeliefView are added as additional supertrait bounds on RuntimeBeliefView. No backward-compatibility shims.
2. Dissolving SnapshotLifecycle into SnapshotEntityCore is a preparation step for ticket 007's full sub-struct decomposition — it ensures the lifecycle fields are already in the right domain grouping.

## Outcome

Completed on 2026-04-08.

- `EntityBeliefView` and `ProfileBeliefView` now own the extracted methods, and `RuntimeBeliefView` composes them as supertraits.
- Production impls were split in `PerAgentBeliefView` and `PlanningState`.
- `SnapshotLifecycle` was dissolved into flat `SnapshotEntity` fields.
- Touched test/mock belief views were migrated to the new sub-trait split.

Deviations from original plan:

- Verification exposed a real regression where the production bandit policy entity methods had fallen back to default `None` after the split. The implemented fix restored those reads in `PerAgentBeliefView` and threaded flee/establishment policy through `PlanningSnapshot` and `PlanningState`.

Verification results:

- Passed `cargo fmt --all`
- Passed `cargo test --workspace`
- Passed `cargo clippy --workspace --all-targets -- -D warnings`

## Verification Layers

1. Entity lifecycle queries -> golden tests (golden_soak, golden_resilience exercise `is_alive`, `is_dead` every tick)
2. Profile queries -> golden tests exercise `homeostatic_needs`, `metabolism_profile` via needs-driven planning
3. SnapshotLifecycle dissolution -> `cargo build` (compile-time proof all field accesses updated)

## What to Change

### 1. Define EntityBeliefView and ProfileBeliefView sub-traits

In `crates/worldwake-sim/src/belief_view.rs`, define both traits with their method signatures moved from RuntimeBeliefView.

### 2. Add supertrait bounds

```rust
pub trait RuntimeBeliefView: ControlBeliefView + EntityBeliefView + ProfileBeliefView {
```

Remove the 13 methods from RuntimeBeliefView's body.

### 3. Update all 18 impl blocks

Split the 8 entity methods and 5 profile methods into separate `impl EntityBeliefView for T` and `impl ProfileBeliefView for T` blocks for each implementor.

### 4. Dissolve SnapshotLifecycle

In `crates/worldwake-ai/src/planning_snapshot.rs`, move the 3 `SnapshotLifecycle` fields (`alive`, `dead`, `incapacitated`) into the main `SnapshotEntity` struct (or a new `SnapshotEntityCore` sub-struct if 007 hasn't created it yet). Remove the `SnapshotLifecycle` struct and the `lifecycle` field on `SnapshotEntity`. Update all construction sites and field accesses (e.g., `entity.lifecycle.alive` → `entity.alive`).

### 5. Export new sub-traits

Add `EntityBeliefView` and `ProfileBeliefView` to worldwake-sim's exports.

## Files to Touch

- `crates/worldwake-sim/src/belief_view.rs` (modify)
- `crates/worldwake-sim/src/lib.rs` (modify — exports)
- `crates/worldwake-sim/src/per_agent_belief_view.rs` (modify)
- `crates/worldwake-ai/src/planning_state.rs` (modify)
- `crates/worldwake-ai/src/planning_snapshot.rs` (modify — impl blocks + SnapshotLifecycle dissolution)
- All 16 test mock files (modify — split impl blocks)

## Out of Scope

- Other domain sub-trait extractions (Spatial, Combat, etc.)
- Full SnapshotEntity sub-struct decomposition (ticket 007)
- GoalBeliefView changes (ticket 008)

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test --workspace`
2. `cargo clippy --workspace --all-targets -- -D warnings`

### Invariants

1. `&dyn RuntimeBeliefView` remains usable at all existing call sites.
2. No behavioral change — method bodies moved, not modified.
3. `SnapshotLifecycle` struct no longer exists after this ticket.

## Test Plan

### New/Modified Tests

1. Added `per_agent_belief_view::tests::bandit_policy_entity_methods_read_from_authoritative_faction_policy`.
2. Added `planning_state::tests::planning_state_preserves_bandit_policy_queries_from_snapshot`.

### Commands

1. `cargo fmt --all`
2. `cargo test --workspace`
3. `cargo clippy --workspace --all-targets -- -D warnings`
