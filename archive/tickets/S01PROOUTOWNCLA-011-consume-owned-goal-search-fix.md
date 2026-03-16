# S01PROOUTOWNCLA-011: Fix ConsumeOwnedCommodity planner search to prefer pick_up over harvest

**Status**: ✅ COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — AI planner goal model, search candidate filtering, golden test corrections
**Deps**: S01PROOUTOWNCLA-004 (introduced the regression), S01PROOUTOWNCLA-007 (authoritative pickup validation), S01PROOUTOWNCLA-008 (belief affordance filtering)

## Problem

When actor-owned production output exists on the ground, the GOAP planner's `ConsumeOwnedCommodity` goal always selects Harvest as its first step instead of pick_up, because the search immediately returns the first `ProgressBarrier` terminal it finds (search.rs:110-113). Both Harvest and MoveCargo (pick_up) are ProgressBarrier terminals for `ConsumeOwnedCommodity`, and the search returns whichever it encounters first — which is consistently Harvest.

This means agents with local owned stock endlessly harvest more instead of picking up and eating what they already have. The bug was introduced by S01PROOUTOWNCLA-004 (harvest commit ownership), which made production output actor-owned. Before -004, output was unowned, `can_exercise_control` failed for unowned lots, `local_controlled_commodity_evidence` returned None, so `ConsumeOwnedCommodity` was never emitted — agents used `AcquireCommodity` instead, which worked correctly.

Additionally, S01PROOUTOWNCLA-007 (authoritative pickup validation) exposes two related golden test regressions:
1. `golden_exclusive_queue_contention` — agents waste ticks on impossible pickups because belief-layer affordance filtering (-008) is not yet implemented
2. `golden_materialized_output_theft` — test expects theft via lawful `pick_up`, which -007 correctly blocks

## Assumption Reassessment (2026-03-16)

1. `CONSUME_OPS` at `goal_model.rs:63-71` includes Harvest, Craft, Trade, QueueForFacilityUse, MoveCargo, Consume, Travel — confirmed
2. `is_progress_barrier` at `goal_model.rs:529-535` treats Trade, Harvest, Craft, and MoveCargo as barriers for `ConsumeOwnedCommodity` — confirmed
3. `search_plan` at `search.rs:110-113` returns the first terminal successor immediately without exploring non-terminal alternatives — confirmed
4. `apply_planner_step` for `ConsumeOwnedCommodity` does NOT update hypothetical state for MoveCargo (falls through to no-op at `goal_model.rs:488`) — confirmed. This means pick_up → eat cannot be found as a 2-step GoalSatisfied plan within a single search
5. `ConsumeOwnedCommodity` and `AcquireCommodity` share the same `GoalKey` structure (`(Some(commodity), None, None)`) but differ in `GoalKind` — confirmed at `goal.rs:83-84`
6. `local_controlled_commodity_evidence` at `candidate_generation.rs:1143-1161` correctly finds actor-owned ground lots — confirmed via debug output
7. `golden_materialized_output_theft_forces_replan` at `golden_production.rs` expects a "Thief" agent to pick up Crafter's owned bread — this behavior is now correctly blocked by -007's ownership check and by the spec's Deliverable 9 ("unauthorized acquisition must not piggyback on lawful pick_up")
8. `seed_agent_with_recipes` at `golden_harness/mod.rs:288-321` did not set a `PerceptionProfile`, causing agents to be invisible to the perception system — fixed in -007 by adding a default profile

## Architecture Check

1. **Remove acquisition ops from CONSUME_OPS**: `ConsumeOwnedCommodity` is "I have local owned stock — pick up and eat." Including Harvest/Craft/Trade/QueueForFacilityUse is architecturally wrong because it conflates acquisition with consumption. When local stock runs out, candidate generation should stop emitting `ConsumeOwnedCommodity` and emit `AcquireCommodity` instead — that transition is already handled correctly by `local_controlled_commodity_evidence` returning None.

2. **MoveCargo barrier is correct**: The planner can't model pick_up's effect on possession in hypothetical state (`apply_planner_step` is a no-op for MoveCargo). So pick_up must remain a ProgressBarrier, with the agent replanning after execution to find the possessed apple and plan Consume. This is the same pattern used by `AcquireCommodity`.

3. **No backwards-compatibility shims**: The fix removes ops, doesn't add aliases or fallback paths.

4. **Theft test rewrite reflects intended architecture**: The spec explicitly states (Deliverable 9) that unauthorized acquisition must not piggyback on lawful `pick_up`. The theft test must be rewritten to match the ownership model — the thief should NOT eat crafter's bread, and should instead fall back to alternative food sources.

## What to Change

### 1. Narrow `CONSUME_OPS` to consumption-only ops in `goal_model.rs`

Remove Harvest, Craft, Trade, and QueueForFacilityUse from `CONSUME_OPS`. Keep only Consume, Travel, and MoveCargo:

```rust
const CONSUME_OPS: &[PlannerOpKind] = &[
    PlannerOpKind::Consume,
    PlannerOpKind::Travel,
    PlannerOpKind::MoveCargo,
];
```

Rationale: `ConsumeOwnedCommodity` should only plan steps that convert local owned stock into consumption. Acquisition ops belong exclusively in `AcquireCommodity`.

### 2. Update `is_progress_barrier` for `ConsumeOwnedCommodity`

With acquisition ops removed, only MoveCargo needs to be a barrier:

```rust
GoalKind::ConsumeOwnedCommodity { .. } => {
    step.op_kind == PlannerOpKind::MoveCargo
}
```

### 3. Update `consume_goal_relevant_ops_include_consumption_and_access_paths` test

The existing unit test at `goal_model.rs` asserts that `ConsumeOwnedCommodity` relevant ops include Travel and Consume. Update to also assert MoveCargo is included and that Harvest/Craft/Trade are NOT included.

### 4. Rewrite `golden_materialized_output_theft_forces_replan` test

The test currently expects the Thief to pick up and eat Crafter's owned bread before the Crafter uses an orchard fallback. Under ownership semantics, this is correctly blocked. Rewrite to verify:
- Crafter crafts bread (actor-owned, on ground)
- Thief CANNOT pick up crafter's bread (ownership check blocks)
- Crafter picks up and eats own bread OR falls back to orchard
- Thief independently seeks food via harvest at a different source

The specific milestones and assertions need redesigning to match the new ownership reality. The test name should be updated to reflect the new behavior (e.g., `golden_owned_output_prevents_unauthorized_taking`).

### 5. Adjust `golden_exclusive_queue_contention` tick budget or assertions

With -008 (belief affordance filtering) implemented, agents won't waste ticks on impossible pickups. If -008 is not yet implemented when this ticket runs, increase the tick budget from 150 to accommodate the ownership friction. If -008 IS implemented, the test may pass without changes.

## Files to Touch

- `crates/worldwake-ai/src/goal_model.rs` (modify — narrow CONSUME_OPS, update barrier logic)
- `crates/worldwake-ai/tests/golden_production.rs` (modify — rewrite theft test, adjust contention test)

## Out of Scope

- Belief-based affordance filtering for ownership (S01PROOUTOWNCLA-008)
- `apply_planner_step` hypothetical state modeling for MoveCargo (would be a larger planner architecture change)
- E17 theft actions (separate epic)
- Search algorithm changes to prefer GoalSatisfied terminals over ProgressBarrier terminals (optimization, not required)

## Acceptance Criteria

### Tests That Must Pass

1. `ConsumeOwnedCommodity` goal plans pick_up (not harvest) when local owned stock exists
2. After pick_up barrier, replanned `ConsumeOwnedCommodity` finds Consume step for possessed lot
3. When no local owned stock exists, `AcquireCommodity` is emitted (not `ConsumeOwnedCommodity`)
4. Theft test verifies ownership prevents unauthorized pickup
5. Capacity-constrained pickup golden test completes harvest → pick_up → eat cycle
6. Queue contention golden test completes within tick budget
7. Existing suite: `cargo test --workspace`

### Invariants

1. `ConsumeOwnedCommodity` never plans acquisition steps (Harvest, Craft, Trade)
2. `AcquireCommodity` handles all acquisition paths (harvest/craft/trade → pick_up)
3. Candidate generation correctly transitions between `ConsumeOwnedCommodity` and `AcquireCommodity` based on local controlled stock presence
4. No unauthorized pickup via lawful `pick_up` action
5. Search prefers shortest barrier-free path when available within `ConsumeOwnedCommodity`

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/goal_model.rs` — update `consume_goal_relevant_ops_include_consumption_and_access_paths` to assert Harvest/Craft/Trade are excluded from CONSUME_OPS
2. `crates/worldwake-ai/tests/golden_production.rs` — rewrite `golden_materialized_output_theft_forces_replan` for ownership semantics
3. `crates/worldwake-ai/tests/golden_production.rs` — verify `golden_capacity_constrained_ground_lot_pickup` completes the full cycle
4. `crates/worldwake-ai/tests/golden_production.rs` — verify `golden_exclusive_queue_contention_uses_queue_grants_and_rotates_first_turns` passes

### Commands

1. `cargo test -p worldwake-ai --test golden_production golden_capacity_constrained`
2. `cargo test -p worldwake-ai --test golden_production golden_materialized_output_theft`
3. `cargo test -p worldwake-ai --test golden_production golden_exclusive_queue_contention`
4. `cargo test -p worldwake-ai`
5. `cargo test --workspace`
6. `cargo clippy --workspace`

## Outcome

**Completion date**: 2026-03-16

**What changed**:
- `goal_model.rs`: Narrowed `CONSUME_OPS` from 7 ops to 3 (Consume, Travel, MoveCargo) — removed acquisition ops (Harvest, Craft, Trade, QueueForFacilityUse) that belong to `AcquireCommodity`. Added MoveCargo barrier check for `ConsumeOwnedCommodity` above the `is_materialization_barrier` guard.
- `search.rs`: Added GoalSatisfied preference over ProgressBarrier for `ConsumeOwnedCommodity` searches — when both eat (GoalSatisfied) and pick_up (ProgressBarrier) terminals coexist, the search now prefers eating possessed stock over picking up more ground stock.
- `golden_production.rs`: Added `production_perception_profile()` (observation_fidelity=1000) to all production test setups that use `ProductionOutputOwner::Actor`, enabling agents to observe harvest output through the perception system.
- Updated 4 search unit tests and 1 goal_model unit test to reflect new barrier semantics.

**Deviations from plan**:
- Queue contention tick budget increase was NOT needed.
- Root cause was deeper than originally specified: the `is_materialization_barrier` guard (line 512) silently blocked MoveCargo from being a barrier for ConsumeOwnedCommodity, AND the search's terminal sort treated GoalSatisfied and ProgressBarrier equally, causing agents to loop on pick_up instead of eating.
- Production test agents lacked `PerceptionProfile`, causing newly created harvest output to be invisible to the belief view.
- Theft test WAS rewritten after the -007 ownership check was restored: renamed to `golden_materialized_output_ownership_prevents_theft`, updated milestones (CrafterAteBread, ThiefUsedOrchard), gave thief harvest recipe and world beliefs so it can independently acquire food via the orchard.

Outcome amended: 2026-03-16

**Verification**: `cargo test --workspace` (all pass except 2 pre-existing trade test failures unrelated to this ticket), `cargo clippy --workspace` (clean).
