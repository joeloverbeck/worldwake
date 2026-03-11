# HARPREE14-012: Observation snapshot relevance filtering

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes -- behavioral change in replanning triggers
**Deps**: HARPREE14-010 (benefits from GoalSemantics trait, soft dep)
**Spec Reference**: HARDENING-PRE-E14.md, HARDEN-B04

## Problem

`observation_snapshot_changed()` in `agent_tick.rs` (lines 475-481) compares the full commodity signature across ALL commodity kinds. This means an agent pursuing a Sleep goal will replan when an unrelated commodity changes in their inventory (e.g., gaining a coin from trade). The replanning is wasted work and can cause goal thrashing.

## Assumption Reassessment (2026-03-11)

1. `observation_snapshot_changed()` exists at line 475 of `agent_tick.rs` -- confirmed
2. It compares full commodity signatures -- confirmed
3. The function is called to decide whether to trigger replanning -- confirmed

## Architecture Check

1. Filtering by goal-relevant commodities is consistent with Principle 7 (Locality) and Principle 3 (Concrete State) -- agents should only react to locally relevant changes.
2. If HARPREE14-010 is done, the `GoalSemantics` trait could declare relevant observation dimensions. Otherwise, a simpler per-`GoalKind` relevance function works.
3. The "always dirty if no plan" behavior must be preserved -- filtering only applies when a plan is active.

## What to Change

### 1. Add goal-relevant commodity filtering

When comparing commodity signatures, only include commodities relevant to the current goal/plan. For example:
- Eat goal: only food commodities
- Drink goal: only drink commodities
- Sleep goal: no commodities (sleep doesn't depend on inventory)
- Trade goal: commodities involved in the trade
- Produce goal: input and output commodities of the recipe

### 2. Define relevance mapping

If HARPREE14-010 is done: add a `relevant_commodities()` method to `GoalSemantics`.
If not: add a standalone function `goal_relevant_commodities(goal: &GoalKind) -> Option<Vec<CommodityKind>>` where `None` means "all commodities are relevant."

### 3. Preserve "always dirty if no plan" behavior

The filtering ONLY applies when a plan is active. If there's no active plan, the existing "always dirty" behavior continues unchanged.

### 4. Add unit test

Test that commodity changes irrelevant to the current goal do NOT trigger replanning.

## Files to Touch

- `crates/worldwake-ai/src/agent_tick.rs` (modify)
- `crates/worldwake-ai/src/decision_runtime.rs` (modify -- if observation snapshot struct needs changes)

## Out of Scope

- Adding new observation dimensions beyond commodity filtering
- Changing the "always dirty if no plan" logic
- Modifying `GoalKind` enum or adding new goal types
- Changes to planning or search logic

## Acceptance Criteria

### Tests That Must Pass

1. New test: `test_irrelevant_commodity_change_does_not_trigger_replan` -- commodity change irrelevant to current goal does not cause replanning
2. New test: `test_relevant_commodity_change_triggers_replan` -- commodity change relevant to current goal does trigger replanning
3. New test: `test_no_plan_always_dirty` -- "always dirty if no plan" behavior preserved
4. Golden e2e passes -- **NOTE: hashes MAY change** because agents may replan fewer times. If hashes change, document the new expected hashes.
5. Conservation and death/loot outcomes equivalent even if tick counts differ
6. `cargo test --workspace` passes
7. `cargo clippy --workspace` -- no new warnings

### Invariants

1. "Always dirty if no plan" behavior preserved
2. Agents still replan when relevant commodities change
3. Conservation invariants maintained
4. Determinism maintained (same inputs -> same outputs)

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/agent_tick.rs` (test module) -- new relevance filtering tests
2. `crates/worldwake-ai/tests/golden_e2e.rs` -- may need updated hash expectations

### Commands

1. `cargo test -p worldwake-ai agent_tick` (targeted)
2. `cargo test -p worldwake-ai --test golden_e2e` (verify behavior)
3. `cargo test --workspace`
4. `cargo clippy --workspace`
