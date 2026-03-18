# E16DPOLPLAN-002: Add `courage` to `SnapshotEntity` + expose through `PlanningState`

**Status**: ✅ COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — planning snapshot, planning state belief view impl
**Deps**: E16DPOLPLAN-001

## Problem

The planner evaluates Threaten viability during `apply_planner_step` but `SnapshotEntity` doesn't capture `courage`, so `PlanningState::courage()` cannot return it.

## Assumption Reassessment (2026-03-18)

1. `SnapshotEntity` is in `crates/worldwake-ai/src/planning_snapshot.rs` with fields like `combat_profile: Option<CombatProfile>` — confirmed
2. `build_snapshot_entity()` populates snapshot fields from belief view — confirmed
3. `PlanningState` implements `RuntimeBeliefView` in `crates/worldwake-ai/src/planning_state.rs` — confirmed
4. `PlanningState::combat_profile()` reads from snapshot — confirmed as pattern to follow

## Architecture Check

1. Adding `courage: Option<Permille>` to `SnapshotEntity` follows existing pattern for profile-gated fields
2. `PlanningState` impl reads from snapshot, no new state tracking needed

## What to Change

### 1. `SnapshotEntity` — add field

Add `pub(crate) courage: Option<Permille>` after `combat_profile`.
Add `courage: None` to `Default` impl.

### 2. `build_snapshot_entity()` — populate

Add `courage: view.courage(entity)` to the builder.

### 3. `PlanningState` RuntimeBeliefView impl — add method

```rust
fn courage(&self, agent: EntityId) -> Option<Permille> {
    self.snapshot.entity(agent).and_then(|e| e.courage)
}
```

## Files to Touch

- `crates/worldwake-ai/src/planning_snapshot.rs` (modify)
- `crates/worldwake-ai/src/planning_state.rs` (modify)

## Out of Scope

- Changes to `RuntimeBeliefView` trait (done in E16DPOLPLAN-001)
- Changes to `PlannerOpKind` match arms
- Golden tests

## Acceptance Criteria

### Tests That Must Pass

1. Build `SnapshotEntity` with `courage = Some(Permille::new(500))`, verify `PlanningState::courage()` returns `Some(Permille(500))`
2. Verify `courage()` returns `None` for entities not in snapshot
3. Verify `courage()` returns `None` for entities in snapshot without `UtilityProfile`
4. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. `SnapshotEntity::default().courage == None`
2. No new `PlanningState` override fields — courage is read-only from snapshot

## Test Plan

### New/Modified Tests

1. Unit test in `planning_snapshot.rs` or `planning_state.rs` — verify courage round-trip through snapshot

### Commands

1. `cargo test -p worldwake-ai`
2. `cargo clippy --workspace`

## Outcome

- **Completion date**: 2026-03-18
- **What changed**:
  - `planning_snapshot.rs`: Added `Permille` import, `courage: Option<Permille>` field to `SnapshotEntity`, `None` in `Default` impl, `courage: view.courage(entity)` in `build_snapshot_entity()`
  - `planning_state.rs`: Added `courage()` method to `PlanningState`'s `RuntimeBeliefView` impl reading from snapshot. Added `courages` field + impl to test `StubBeliefView`. Added unit test `courage_round_trips_through_snapshot_and_planning_state` covering all 3 acceptance criteria.
- **Deviations**: `PlanningState::courage()` uses `self.snapshot.entities.get(&agent).and_then(|snapshot| snapshot.courage)` instead of `self.snapshot.entity(agent).and_then(|e| e.courage)` — matches the exact pattern used by `combat_profile()` which accesses the entities map directly.
- **Verification**: `cargo clippy --workspace` clean, all worldwake-ai tests pass (403+).
