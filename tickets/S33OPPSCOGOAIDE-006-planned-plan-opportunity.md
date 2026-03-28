# S33OPPSCOGOAIDE-006: Add OpportunityKey to PlannedPlan

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: Yes — `PlannedPlan` struct shape changes
**Deps**: S33OPPSCOGOAIDE-002

## Problem

`PlannedPlan` currently only carries `goal: GoalKey`. After `S33OPPSCOGOAIDE-002`, candidate generation and blocker matching are already opportunity-scoped, but the selected plan still sheds that identity at the plan object boundary. The plan runtime therefore cannot carry a canonical concrete opportunity through execution, persistence, and debugging.

The shared abstraction boundary under audit is:

- opportunity-scoped candidate identity entering search
- persisted/selected `PlannedPlan` identity leaving search

## Assumption Reassessment (2026-03-28)

1. `PlannedPlan` in `crates/worldwake-ai/src/planner_ops.rs` still stores desire identity but no concrete `OpportunityKey`. That remains the live gap.
2. `IntentionFrame` in `crates/worldwake-core/src/intention_frame.rs` is intentionally desire-scoped and should stay that way. This ticket must not broaden frame identity to opportunity scope.
3. Save/load support is not already done. Once `PlannedPlan` grows a new field, `S33OPPSCOGOAIDE-008` must own the format bump and persistence verification.
4. Archived `S33OPPSCOGOAIDE-005` already removed the temporary first-per-`GoalKey` planning gate. This ticket should not reopen planning admission behavior; it should pick up the remaining identity seam at the `PlannedPlan` boundary only. Candidate-local snapshot scope already landed in archived `S33OPPSCOGOAIDE-010`, and this ticket should preserve that boundary rather than revisit it.

## Architecture Check

1. The canonical transport path after this change is: candidate opportunity -> searched opportunity -> `PlannedPlan.opportunity`. Carrying opportunity separately in runtime side state would recreate a duplicate lawful path for the same fact.
2. `goal` remains because desire identity is still real and still used by `IntentionFrame`. `opportunity` is not an alias for `goal`; it is the concrete execution identity that was previously missing.
3. No backward-compatibility shims.

## Verification Layers

1. `PlannedPlan.opportunity` matches the searched opportunity -> focused unit test.
2. Desire continuity remains on `GoalKey` -> focused unit test or runtime assertion proving `IntentionFrame` behavior does not change.
3. Persistence impact is acknowledged but deferred -> no local save/load assertions here beyond compile/runtime integration.

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

### 2. Populate it at plan construction

When plan search returns a `PlannedPlan`, populate `opportunity` from the concrete `GroundedGoal` being searched, not by reconstructing it later from unrelated runtime state.

### 3. Thread the field through selection/runtime consumers

Update any code that stores, clones, compares, or traces `PlannedPlan` so the new field is preserved. The ticket should not redefine higher-level behavior; it should make the concrete opportunity explicit wherever the plan already flows.

## Files to Touch

- `crates/worldwake-ai/src/planner_ops.rs` (modify — add `opportunity` field to `PlannedPlan`)
- `crates/worldwake-ai/src/search.rs` and/or the plan-construction site (modify — populate `opportunity`)
- `crates/worldwake-ai/src/plan_selection.rs` (modify if it copies or inspects `PlannedPlan`)
- `crates/worldwake-ai/src/decision_runtime.rs` (modify if runtime storage or helper methods need the new field)

## Out of Scope

- `IntentionFrame` changes (spec says none — frame persists on `GoalKey`)
- Exhaustion re-keying (`S33OPPSCOGOAIDE-004`)
- Ranked opportunity admission (`S33OPPSCOGOAIDE-005`, completed)
- Planning snapshot isolation (already delivered by archived `S33OPPSCOGOAIDE-010`)
- Save/load version bump (S33OPPSCOGOAIDE-008)
- Decision trace changes (S33OPPSCOGOAIDE-007)

## Acceptance Criteria

### Tests That Must Pass

1. `PlannedPlan.opportunity` correctly reflects the searched opportunity's `OpportunityKey`.
2. A plan for `AcquireCommodity(Apple)` at orchard carries the orchard opportunity rather than only the desire key.
3. `IntentionFrame` continuity still keys off `GoalKey`, not `OpportunityKey`.
4. Existing suite: `cargo test -p worldwake-ai`
5. Existing suite: `cargo clippy --workspace`

### Invariants

1. `PlannedPlan.goal == PlannedPlan.opportunity.goal_key` always.
2. `IntentionFrame` is not widened; it remains desire-scoped.
3. No second side-channel carries the selected opportunity once this field exists.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/search.rs` or `planner_ops.rs` — `planned_plan_carries_searched_opportunity`
2. Existing plan/runtime tests updated to assert `IntentionFrame` continuity still follows `GoalKey`
3. Any `PlannedPlan` equality/serde/clone tests updated for the new field as needed

### Commands

1. `cargo test -p worldwake-ai -- planned_plan`
2. `cargo test -p worldwake-ai -- plan_selection`
3. `cargo clippy --workspace`
4. `cargo test --workspace`
