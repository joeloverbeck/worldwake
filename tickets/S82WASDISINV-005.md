# S82WASDISINV-005: Implement GoalKindPlannerExt for FreeCarryCapacity

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes — new trait method implementations for FreeCarryCapacity
**Deps**: S82WASDISINV-004

## Problem

The planner still cannot search for, evaluate, or apply `FreeCarryCapacity` goals. `S82WASDISINV-001` already landed compile-safe inert `FreeCarryCapacity` handling in some `GoalKindPlannerExt` match sites so the shared enum variant compiles, but those branches do not provide real disposal semantics. This ticket adds the live implementations needed for disposal plans.

## Assumption Reassessment (2026-04-10)

1. `GoalKindPlannerExt` trait at `goal_model.rs:37-87` requires 11 methods: `ranked_goal_provenance_family`, `relevant_op_kinds`, `relevant_observed_commodities`, `build_payload_override`, `apply_planner_step`, `is_progress_barrier`, `is_satisfied`, `goal_relevant_places`, `prerequisite_places`, `matches_binding`, `candidate_is_available`. Verified against current trait definition.
2. `S82WASDISINV-001` already added compile-safe inert `FreeCarryCapacity` handling to some `GoalKindPlannerExt` match sites (`relevant_observed_commodities`, `is_satisfied`, `goal_relevant_places`, `matches_binding`). This ticket now owns replacing inert handling with real disposal semantics across all 11 methods, not merely adding missing exhaustive arms.
3. `PlannerTransitionKind::PutDownGroundLot` handles the hypothetical state effect at `planner_ops.rs:324` — removes item from possession, places on ground.
4. `CommodityKind::Waste` exists at `items.rs:20`.
5. This is a planner-driven ticket. The live `GoalKind` under test is `FreeCarryCapacity`. The operator surface is `PlannerOpKind::DropItem` with `PlannerTransitionKind::PutDownGroundLot`.

## Architecture Check

1. Standard pattern: replace the existing inert scaffolding and fill the remaining `GoalKindPlannerExt` methods with live disposal semantics. Each method already has arms for the other GoalKind variants.
2. `is_satisfied` checks hypothetical load vs. threshold — derived computation, never stored (P3, P27).
3. No backward-compatibility shims.

## Verification Layers

1. `relevant_op_kinds` returns `[DropItem]` -> focused unit test
2. `is_satisfied` returns true when hypothetical load < threshold -> focused unit test
3. `apply_planner_step` removes item from hypothetical possession -> focused unit test
4. `goal_relevant_places` returns agent's current place -> focused unit test
5. `matches_binding` returns true for DropItem op -> focused unit test
6. Single-layer ticket (planner model) — verified via focused unit tests on PlanningState

## What to Change

### 1. ranked_goal_provenance_family

Return `None` — no drive or danger provenance.

### 2. relevant_op_kinds

Return `&[PlannerOpKind::DropItem]`.

### 3. relevant_observed_commodities

Return `Some(BTreeSet::from([CommodityKind::Waste]))`.

### 4. build_payload_override

Return `Ok(None)` — `ActionPayload::None`, no override needed.

### 5. apply_planner_step

For `PlannerOpKind::DropItem`, the existing `PlannerTransitionKind::PutDownGroundLot` transition handler already removes the target from hypothetical possession. Verify this is invoked correctly for the new op kind.

### 6. is_progress_barrier

Return `true` when `step.op_kind == PlannerOpKind::DropItem`.

### 7. is_satisfied

Check that the agent's hypothetical load is below the `capacity_strain_threshold`. If no `DisposalProfile` is available via belief view, satisfied when any item has been dropped (load decreased).

### 8. goal_relevant_places

Return the agent's current believed place — dropping happens in-place.

### 9. prerequisite_places

Return empty vec — no travel prerequisite.

### 10. matches_binding

Return `true` when `op_kind == PlannerOpKind::DropItem`.

### 11. candidate_is_available

Return `true` when `op_kind == PlannerOpKind::DropItem` and the agent has waste items in hypothetical possession.

## Files to Touch

- `crates/worldwake-ai/src/goal_model.rs` (modify — 11 match arms)

## Out of Scope

- Ranking integration (ticket 006)
- Candidate generation (ticket 007)
- Golden tests (ticket 008)

## Acceptance Criteria

### Tests That Must Pass

1. `relevant_op_kinds` for `FreeCarryCapacity` returns `[DropItem]`
2. `is_satisfied` returns `false` when load exceeds threshold, `true` when below
3. `apply_planner_step` with DropItem reduces hypothetical possession
4. `goal_relevant_places` returns agent's place
5. `prerequisite_places` returns empty
6. `matches_binding` is true for DropItem, false for other ops
7. `candidate_is_available` is true when waste in hypothetical possession
8. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. All 11 GoalKindPlannerExt methods have live `FreeCarryCapacity` semantics (no inert placeholder branches left on the canonical disposal path)
2. `cargo clippy --workspace --all-targets -- -D warnings` passes

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/goal_model.rs` (test module) — focused tests for each of the 11 methods with FreeCarryCapacity

### Commands

1. `cargo test -p worldwake-ai`
2. `cargo clippy --workspace --all-targets -- -D warnings`
