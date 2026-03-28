# S33OPPSCOGOAIDE-006: Add OpportunityKey to PlannedPlan and update plan search iteration

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: Yes — PlannedPlan struct change, build_candidate_plans iteration update
**Deps**: S33OPPSCOGOAIDE-001, S33OPPSCOGOAIDE-002

## Problem

`PlannedPlan` currently only carries `goal: GoalKey`. After opportunity-scoped planning, the plan must also record which `OpportunityKey` it was searched for. This enables `record_exhausted_goals()` (S33OPPSCOGOAIDE-004) to record exhaustion under the correct opportunity, and allows the decision runtime to track which opportunity is currently being executed.

## Assumption Reassessment (2026-03-28)

1. `PlannedPlan` at `crates/worldwake-ai/src/planner_ops.rs:782-830` has fields `{ goal, steps, total_estimated_ticks, terminal_kind }`. No `opportunity` field exists. Confirmed.
2. `PlannedPlan` is constructed in `search.rs` when a plan is found. The goal field is set from the search target.
3. `PlannedPlan` is consumed in `plan_selection.rs::select_best_plan()` and stored in `AgentDecisionRuntime`.
4. `IntentionFrame` at `crates/worldwake-core/src/intention_frame.rs:131-152` persists on `goal: GoalKey`. Per spec, IntentionFrame does NOT change — it remains at desire level. Frame continuity is maintained when opportunity switches within the same desire.
5. `PlannedPlan` is serialized (via serde) as part of `AgentDecisionRuntime` — save/load must handle the new field (S33OPPSCOGOAIDE-008).

## Architecture Check

1. Adding `opportunity: OpportunityKey` to `PlannedPlan` is the minimal structural change. The alternative — carrying opportunity as separate runtime state — would decouple plan identity from its opportunity, making debugging harder.
2. No backward-compatibility shims. The `goal` field is retained for desire-level identity (used by IntentionFrame matching). The `opportunity` field is purely additive.

## Verification Layers

1. `PlannedPlan.opportunity` matches searched opportunity → focused unit test: plan found for specific anchor, field reflects it.
2. IntentionFrame continuity on tactic switch → focused unit test: frame persists when `PlannedPlan.opportunity` changes but `PlannedPlan.goal` stays the same.
3. Single-layer ticket (struct field addition + construction site updates).

## What to Change

### 1. Add `opportunity: OpportunityKey` to `PlannedPlan`

In `crates/worldwake-ai/src/planner_ops.rs`:

```rust
pub struct PlannedPlan {
    pub goal: GoalKey,
    pub opportunity: OpportunityKey,  // NEW
    pub steps: Vec<PlannedStep>,
    pub total_estimated_ticks: u32,
    pub terminal_kind: PlanTerminalKind,
}
```

### 2. Update `PlannedPlan` construction in `search.rs`

When `search_plan()` finds a plan and constructs `PlannedPlan`, populate the `opportunity` field from the `GroundedGoal` that was searched.

### 3. Update `build_candidate_plans()` to pass opportunity through

The iteration in `build_candidate_plans()` already has access to `RankedGoal.grounded` (which carries `key` and `anchor` after S33OPPSCOGOAIDE-002). Construct `OpportunityKey { goal_key: grounded.key, anchor: grounded.anchor }` and pass it to plan construction.

### 4. Update `plan_selection.rs` if it constructs or accesses `PlannedPlan` fields

`select_best_plan()` selects from plan search results — ensure the `opportunity` field flows through correctly.

## Files to Touch

- `crates/worldwake-ai/src/planner_ops.rs` (modify — add `opportunity` field to `PlannedPlan`)
- `crates/worldwake-ai/src/search.rs` (modify — populate `opportunity` on `PlannedPlan` construction)
- `crates/worldwake-ai/src/agent_tick/planning.rs` (modify — pass opportunity to plan construction)
- `crates/worldwake-ai/src/plan_selection.rs` (modify — if it accesses `PlannedPlan` fields directly)

## Out of Scope

- `IntentionFrame` changes (spec says none — frame persists on `GoalKey`)
- Exhaustion re-keying (S33OPPSCOGOAIDE-004)
- Post-rank dedup (S33OPPSCOGOAIDE-005)
- Save/load version bump (S33OPPSCOGOAIDE-008)
- Decision trace changes (S33OPPSCOGOAIDE-007)

## Acceptance Criteria

### Tests That Must Pass

1. `PlannedPlan.opportunity` correctly reflects the searched opportunity's `OpportunityKey`.
2. Plan for `AcquireCommodity(Apple)` at orchard carries `OpportunityKey { ..., Place(orchard) }`.
3. Frame persists when plan switches from orchard-opportunity to market-opportunity (same `GoalKey`).
4. Existing suite: `cargo test -p worldwake-ai`
5. Existing suite: `cargo clippy --workspace`

### Invariants

1. `PlannedPlan.goal == PlannedPlan.opportunity.goal_key` always (consistency).
2. `IntentionFrame` is NOT modified — it persists on `GoalKey` only.
3. `PlannedPlan` serialization includes the new `opportunity` field.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/search.rs` or `planner_ops.rs` — `test_planned_plan_carries_opportunity` — plan carries correct OpportunityKey.
2. `crates/worldwake-ai/src/agent_tick/` — `test_frame_continuity_on_tactic_switch` — IntentionFrame persists when opportunity changes within same GoalKey.
3. Existing plan search tests updated to verify `opportunity` field.

### Commands

1. `cargo test -p worldwake-ai -- planned_plan`
2. `cargo test -p worldwake-ai -- plan_selection`
3. `cargo clippy --workspace && cargo test --workspace`
