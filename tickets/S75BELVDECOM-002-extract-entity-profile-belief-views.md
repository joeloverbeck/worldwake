# S75BELVDECOM-002: Extract EntityBeliefView + ProfileBeliefView sub-traits

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes — RuntimeBeliefView trait decomposition, SnapshotLifecycle dissolution
**Deps**: S75BELVDECOM-001

## Problem

Continue RuntimeBeliefView decomposition by extracting EntityBeliefView (8 methods: lifecycle queries) and ProfileBeliefView (5 methods: needs/metabolism/preferences). This batch also dissolves `SnapshotLifecycle` into `SnapshotEntityCore` since `alive`/`dead`/`incapacitated` are EntityBeliefView fields.

## Assumption Reassessment (2026-04-08)

1. EntityBeliefView methods confirmed on RuntimeBeliefView: `is_alive`, `is_dead`, `is_incapacitated`, `entity_kind`, `corpse_entities_at`, `bandit_flee_wound_threshold`, `bandit_camp_establishment_ticks`, `locally_observed_is_dead`.
2. ProfileBeliefView methods confirmed: `homeostatic_needs`, `drive_thresholds`, `metabolism_profile`, `preference_profile`, `utility_profile`.
3. `SnapshotLifecycle` at `planning_snapshot.rs:220` has 3 fields: `alive`, `dead`, `incapacitated`. These correspond to EntityBeliefView methods and should migrate into the entity domain.

## Architecture Check

1. Same supertrait pattern as 001. EntityBeliefView and ProfileBeliefView are added as additional supertrait bounds on RuntimeBeliefView. No backward-compatibility shims.
2. Dissolving SnapshotLifecycle into SnapshotEntityCore is a preparation step for ticket 007's full sub-struct decomposition — it ensures the lifecycle fields are already in the right domain grouping.

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

1. None — pure structural refactor. Existing golden and unit tests are the behavior proof.

### Commands

1. `cargo build --workspace`
2. `cargo test --workspace`
3. `cargo clippy --workspace --all-targets -- -D warnings`
