# HARCARGOACON-005: Enable MoveCargo goal satisfaction and search support

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — worldwake-ai (goal_model, search modules)
**Deps**: HARCARGOACON-001 (new MoveCargo variant), HARCARGOACON-002 (BeliefView helpers), HARCARGOACON-003 (restock_gap_at_destination)

## Problem

Two blockers prevent `MoveCargo` from functioning as a real planner goal:

1. `goal_model.rs:333` — `is_satisfied()` returns `false` permanently for `MoveCargo`
2. `search.rs:236-241` — `unsupported_goal()` rejects `MoveCargo` before search even begins

Both must be fixed for cargo plans to be discovered and completed.

## Assumption Reassessment (2026-03-12)

1. `is_satisfied` at `goal_model.rs:333` has `MoveCargo { .. }` in the `=> false` arm — confirmed
2. `unsupported_goal` at `search.rs:236-241` matches `MoveCargo { .. }` — confirmed
3. `apply_planner_step` at `goal_model.rs:258` — `MoveCargo` falls through to `_ => state` (intentional no-op per spec Section D.4) — confirmed
4. `is_progress_barrier` at `goal_model.rs:277-280` — `MoveCargo` falls through to `_ => false` — confirmed; spec says to consider whether `pick_up` should be a barrier
5. `restock_gap_at_destination` will exist after HARCARGOACON-003 — per dependency

## Architecture Check

1. Satisfaction uses `restock_gap_at_destination` — destination-aware, not just "agent owns commodity somewhere"
2. Removing from `unsupported_goal` is a one-line change with no side effects beyond enabling search
3. No special progress-barrier needed for lot splitting — the goal key is stable across splits

## What to Change

### 1. Fix `is_satisfied` for `MoveCargo` (goal_model.rs)

Replace the `MoveCargo { .. } => false` arm with:

```rust
GoalKind::MoveCargo { commodity, destination } => {
    restock_gap_at_destination(state, actor, *destination, *commodity).is_none()
}
```

This means: satisfied when the destination has enough controlled stock to meet observed demand.

Note: `is_satisfied` receives `&PlanningState` which implements `BeliefView`, so `restock_gap_at_destination` can be called directly.

### 2. Remove `MoveCargo` from `unsupported_goal` (search.rs)

Change:
```rust
GoalKind::SellCommodity { .. } | GoalKind::MoveCargo { .. } | GoalKind::BuryCorpse { .. }
```
To:
```rust
GoalKind::SellCommodity { .. } | GoalKind::BuryCorpse { .. }
```

### 3. Confirm `apply_planner_step` no-op is correct (goal_model.rs)

`MoveCargo` falls through to `_ => state` at line 258. This is intentional: state transitions for cargo happen via `PickUpGroundLot`/`PutDownGroundLot` transition kinds in `planner_ops.rs`, not via goal-level step application. Add a code comment documenting this.

### 4. Decide `is_progress_barrier` for `MoveCargo` (goal_model.rs)

Per spec Section D.5: `pick_up` under `MoveCargo` can split lots (creating materialization). However, the new goal identity survives lot splits, and satisfaction is checked via destination stock, not lot identity. Therefore `MoveCargo` does NOT need a progress barrier — the default `false` is correct. Add a code comment explaining why.

## Files to Touch

- `crates/worldwake-ai/src/goal_model.rs` (modify — satisfaction logic, comments)
- `crates/worldwake-ai/src/search.rs` (modify — remove from unsupported)

## Out of Scope

- Changing `apply_planner_step` behavior for `MoveCargo` (confirmed no-op is correct)
- Changing `is_progress_barrier` behavior for `MoveCargo` (confirmed `false` is correct)
- Modifying planner_ops.rs `PlannerOpKind::MoveCargo` semantics
- Adding new search heuristics or candidate filtering for cargo
- Modifying candidate generation (HARCARGOACON-004)
- Agent tick continuity tests (HARCARGOACON-006)

## Acceptance Criteria

### Tests That Must Pass

1. New test: `MoveCargo` satisfaction returns `true` when destination stock meets demand
2. New test: `MoveCargo` satisfaction returns `false` when destination stock is below demand
3. New test: `MoveCargo` satisfaction returns `true` even when agent has stock elsewhere (destination-aware)
4. New test: `MoveCargo` is no longer rejected by `unsupported_goal()`
5. New test: cargo search can find a plan (pick_up → travel) for a simple delivery scenario
6. Existing search tests continue to pass
7. `cargo test --workspace` and `cargo clippy --workspace` pass

### Invariants

1. `MoveCargo` satisfaction is destination-aware — uses `restock_gap_at_destination`, not global `commodity_quantity`
2. `MoveCargo` is searchable — not in `unsupported_goal()` list
3. `apply_planner_step` for `MoveCargo` remains a no-op (state transitions via `PlannerOpKind`)
4. `is_progress_barrier` for `MoveCargo` remains `false` (goal key survives lot splits)
5. `SellCommodity` and `BuryCorpse` remain unsupported (not changed by this ticket)

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/goal_model.rs` — `move_cargo_satisfied_when_destination_stocked`, `move_cargo_not_satisfied_when_destination_understocked`, `move_cargo_satisfaction_is_destination_local`
2. `crates/worldwake-ai/src/search.rs` — `move_cargo_is_not_unsupported`, `cargo_search_finds_pickup_travel_plan`

### Commands

1. `cargo test -p worldwake-ai goal_model`
2. `cargo test -p worldwake-ai search`
3. `cargo test --workspace`
4. `cargo clippy --workspace`
