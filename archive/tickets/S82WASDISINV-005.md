# S82WASDISINV-005: Implement GoalKindPlannerExt for FreeCarryCapacity

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes — new trait method implementations for FreeCarryCapacity
**Deps**: S82WASDISINV-004

## Problem

The planner still cannot search for, evaluate, or apply `FreeCarryCapacity` goals correctly. `S82WASDISINV-004` already made the dispatch identity, relevant op, and progress-barrier wiring live for `DropItem`, but the remaining `GoalKindPlannerExt` semantics are still inert or over-broad: the goal does not yet expose waste as its relevant observed commodity, never reports satisfied, has no in-place relevant place guidance, and currently treats every operator as available. In addition, `PlanningState` still inherits the default `ProfileBeliefView::disposal_profile()` = `None`, so threshold-driven satisfaction checks would silently lose the actor's disposal profile during planning.

## Assumption Reassessment (2026-04-10)

1. `GoalKindPlannerExt` trait at `goal_model.rs:37-87` requires 11 methods: `ranked_goal_provenance_family`, `relevant_op_kinds`, `relevant_observed_commodities`, `build_payload_override`, `apply_planner_step`, `is_progress_barrier`, `is_satisfied`, `goal_relevant_places`, `prerequisite_places`, `matches_binding`, `candidate_is_available`. Verified against current trait definition.
2. `S82WASDISINV-004` already made `ranked_goal_provenance_family()`, `relevant_op_kinds()`, and `is_progress_barrier()` live indirectly through `GoalDispatchKey::FreeCarryCapacity.declaration()`: `provenance_family = None`, `relevant_ops = [PlannerOpKind::DropItem]`, and `progress_barrier_ops = [PlannerOpKind::DropItem]`. This ticket must not reintroduce those branches as bespoke logic.
3. `PlannerTransitionKind::PutDownGroundLot` handles the hypothetical state effect at `planner_ops.rs:324` — removes item from possession, places on ground.
4. `CommodityKind::Waste` exists at `items.rs:20`.
5. `build_payload_override()` already returns `Ok(None)` for `drop_item` under the existing fallback path because the action payload is `ActionPayload::None`. `prerequisite_places()` already returns empty for unhandled goals, and `matches_binding()` already treats `PlannerOpKind::DropItem` as an auxiliary op that passes binding checks. This ticket now owns the remaining live semantic delta, not duplicate no-op logic.
6. `apply_hypothetical_transition()` in `planner_ops.rs:311-350` applies `GoalModelFallback` first, then `apply_put_down_transition()` for `PlannerTransitionKind::PutDownGroundLot`. That means `GoalKind::FreeCarryCapacity` does not need a bespoke `apply_planner_step()` branch unless it must add extra goal-specific hypothetical state beyond the shared put-down transition.
7. `PlanningState` implements `ProfileBeliefView` at `planning_state.rs:1181-1201`, but currently exposes `homeostatic_needs`, `drive_thresholds`, and `metabolism_profile` only. It does not override `disposal_profile()`, so planner-side reads currently fall back to the trait default `None` from `belief_view.rs:193-198`.
8. This is a planner-driven ticket. The live `GoalKind` under test is `FreeCarryCapacity`. The operator surface is `PlannerOpKind::DropItem` with `PlannerTransitionKind::PutDownGroundLot`, and the threshold/provenance input is `ProfileBeliefView::disposal_profile()` on both the runtime belief view and `PlanningState`.

## Architecture Check

1. Standard pattern: keep declaration-owned behavior in the dispatch table and implement only the remaining live semantic methods in `GoalKindPlannerExt`, plus planner-state profile parity needed for those semantics. This avoids duplicating dispatch truth across two codepaths.
2. `is_satisfied` checks hypothetical load vs. threshold — derived computation, never stored (P3, P27). The threshold must come through the same belief/profile surface on both runtime views and `PlanningState`.
3. No backward-compatibility shims.

## Verification Layers

1. `relevant_observed_commodities` reports `Waste` rather than an empty set -> focused unit test
2. `is_satisfied` respects disposal threshold from planner-visible profile state, with fallback when profile missing -> focused unit test on `PlanningState`
3. `goal_relevant_places` returns the actor's current place for in-place disposal -> focused unit test
4. `candidate_is_available` only allows `DropItem` when hypothetical waste possession exists -> focused unit test on `PlanningState`
5. `PlanningState::disposal_profile()` exposes actor profile to planner-side semantics -> focused unit test
6. Single-layer ticket (planner model + planning-state parity) — verified via focused unit tests on `PlanningState`

## What to Change

### 1. relevant_observed_commodities

Return `Some(BTreeSet::from([CommodityKind::Waste]))`.

### 2. is_satisfied

Check that the agent's hypothetical load is below the `capacity_strain_threshold`. If no `DisposalProfile` is available via belief view, satisfied when any item has been dropped (load decreased).

### 3. goal_relevant_places

Return the agent's current believed place — dropping happens in-place.

### 4. candidate_is_available

Return `true` when `op_kind == PlannerOpKind::DropItem` and the agent has waste items in hypothetical possession.

### 5. PlanningState profile parity

In `crates/worldwake-ai/src/planning_state.rs`, implement `ProfileBeliefView::disposal_profile()` so planner-side reads see the actor's `DisposalProfile` instead of silently falling back to `None`.

## Files to Touch

- `crates/worldwake-ai/src/goal_model.rs` (modify)
- `crates/worldwake-ai/src/planning_state.rs` (modify)

## Out of Scope

- Ranking integration (ticket 006)
- Candidate generation (ticket 007)
- Golden tests (ticket 008)

## Acceptance Criteria

### Tests That Must Pass

1. `relevant_observed_commodities` for `FreeCarryCapacity` returns `{CommodityKind::Waste}`
2. `is_satisfied` returns `false` when load exceeds threshold and `true` when below
3. `is_satisfied` falls back to load reduction when disposal profile unavailable
4. `goal_relevant_places` returns the actor's place
5. `candidate_is_available` is true only for `DropItem` when waste is in hypothetical possession
6. `PlanningState::disposal_profile()` returns the actor profile
8. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. The canonical `FreeCarryCapacity` path has live planner semantics without duplicating dispatch/declaration-owned truth in bespoke goal-model branches
2. `cargo clippy --workspace --all-targets -- -D warnings` passes

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/goal_model.rs` (test module) — focused tests for waste-observed commodities, satisfaction, relevant places, and candidate availability
2. `crates/worldwake-ai/src/planning_state.rs` (test module) or `crates/worldwake-ai/src/goal_model.rs` planner-state-focused test helpers — verify `disposal_profile()` planner parity

### Commands

1. `cargo test -p worldwake-ai`
2. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

Completed: 2026-04-10

- Implemented live `FreeCarryCapacity` planner semantics in `crates/worldwake-ai/src/goal_model.rs` for:
  - `relevant_observed_commodities` -> `{CommodityKind::Waste}`
  - `is_satisfied` -> threshold-driven carried-load satisfaction with load-reduction fallback when no disposal profile is available
  - `goal_relevant_places` -> actor current place
  - `candidate_is_available` -> `DropItem` only when hypothetical waste possession exists
- Implemented planner-visible disposal-profile parity in `crates/worldwake-ai/src/planning_state.rs` by overriding `ProfileBeliefView::disposal_profile()`.
- Also updated `crates/worldwake-ai/src/planning_snapshot.rs` so disposal profiles are carried through snapshot construction and storage; without that snapshot path, planner-side profile reads would still fall back to `None`.

Deviations from original plan:

- The ticket was narrowed during implementation because `S82WASDISINV-004` had already made declaration-owned `FreeCarryCapacity` dispatch, relevant-op, and progress-barrier behavior live.
- The final parity fix required snapshot-layer storage in addition to the ticket's narrowed `PlanningState` accessor wording.

Verification results:

- `cargo test -p worldwake-ai free_carry_capacity_` -> passed
- `cargo test -p worldwake-ai planning_state_matches_runtime_duration_estimation_for_dynamic_duration_contract` -> passed
- `cargo test -p worldwake-ai` -> passed
- `cargo clippy --workspace --all-targets -- -D warnings` -> passed
