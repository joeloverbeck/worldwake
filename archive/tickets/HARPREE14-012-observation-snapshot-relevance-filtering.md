# HARPREE14-012: Observation snapshot relevance filtering

**Status**: ✅ COMPLETED
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes -- behavioral change in replanning triggers
**Deps**: None
**Spec Reference**: HARDENING-PRE-E14.md, HARDEN-B04

## Problem

`observation_snapshot_changed()` in `agent_tick.rs` compares the full commodity signature across all commodity kinds for every active goal. This means an agent pursuing a non-commodity goal such as Sleep will replan when an unrelated inventory change occurs (for example, gaining a coin from trade). The replanning is wasted work and can cause goal churn.

## Assumption Reassessment (2026-03-11)

1. `observation_snapshot_changed()` exists in `crates/worldwake-ai/src/agent_tick.rs` and feeds the read-phase dirtiness calculation -- confirmed.
2. It compares full commodity signatures, but also compares place, needs, wounds, and unique-item signatures -- confirmed.
3. `AgentDecisionRuntime` already stores the last observation snapshot; this ticket does not require a runtime schema change -- confirmed.
4. `worldwake-ai` already has a goal-specific extension point in `GoalKindPlannerExt` (`crates/worldwake-ai/src/goal_model.rs`). Relevance filtering should extend that existing contract instead of introducing a second semantics layer -- confirmed.

## Architecture Check

1. Filtering by goal-relevant commodities is consistent with Principle 7 (Locality) and Principle 3 (Concrete State) -- agents should only react to locally relevant changes.
2. The cleanest implementation is to extend `GoalKindPlannerExt` with commodity relevance for observation filtering. Introducing a separate `GoalSemantics` trait or standalone mapping would duplicate existing goal-specific logic.
3. This ticket should stay narrowly scoped to commodity signature relevance only. Place, needs, wounds, and unique-item signatures remain global dirtiness triggers.
4. The "always dirty if no plan" behavior must be preserved -- filtering only applies when a current goal/plan exists.

## What to Change

### 1. Add goal-relevant commodity filtering for snapshot comparison

When comparing commodity signatures, only include commodities relevant to the current goal/plan. The mapping should be derived from the actual `GoalKind` variants that exist today:
- `ConsumeOwnedCommodity`, `AcquireCommodity`, `SellCommodity`, `RestockCommodity`, `MoveCargo`: only the goal commodity
- `ProduceCommodity`: the recipe input and output commodities
- `Sleep`, `Relieve`, `Wash`, `ReduceDanger`, `Heal`, `LootCorpse`, `BuryCorpse`: no commodity-driven snapshot dirtiness

### 2. Define relevance through existing goal-model semantics

Add a `GoalKindPlannerExt` method that returns the relevant commodities for observation filtering. The implementation may take `RecipeRegistry` so `ProduceCommodity` can derive its relevance from the registered recipe.

### 3. Preserve "always dirty if no plan" behavior

The filtering ONLY applies when a plan is active. If there's no active plan, the existing "always dirty" behavior continues unchanged.

### 4. Add targeted unit tests

Add unit tests around `refresh_runtime_for_read_phase()` / snapshot dirtiness that prove irrelevant commodity changes do not dirty a runtime for a non-commodity goal, while relevant changes still do.

## Files to Touch

- `crates/worldwake-ai/src/agent_tick.rs` (modify)
- `crates/worldwake-ai/src/goal_model.rs` (modify)

## Out of Scope

- Adding new observation dimensions beyond commodity filtering
- Changing the "always dirty if no plan" logic
- Modifying `GoalKind` enum or adding new goal types
- Changes to planning or search logic
- Filtering unique-item snapshot comparison by goal
- Introducing a second semantics trait or compatibility alias for goal relevance

## Acceptance Criteria

### Tests That Must Pass

1. New test: `test_irrelevant_commodity_change_does_not_trigger_replan` -- commodity change irrelevant to current goal does not cause replanning
2. New test: `test_relevant_commodity_change_triggers_replan` -- commodity change relevant to current goal does trigger replanning
3. New test: `test_no_plan_always_dirty` -- "always dirty if no plan" behavior preserved
4. Golden e2e passes. If fewer replans change the deterministic hash, record the new expected hashes and explain the reason in the archive outcome.
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
2. `crates/worldwake-ai/tests/golden_e2e.rs` -- verify whether the reduced replanning changes deterministic hash expectations

### Commands

1. `cargo test -p worldwake-ai agent_tick` (targeted)
2. `cargo test -p worldwake-ai --test golden_e2e` (verify behavior)
3. `cargo test --workspace`
4. `cargo clippy --workspace`

## Outcome

- Completed: 2026-03-12
- Actual changes:
  - Extended `GoalKindPlannerExt` with goal-specific observed commodity relevance.
  - Updated `agent_tick` snapshot dirtiness to compare filtered commodity signatures for the active goal while leaving place, needs, wounds, and unique items unchanged.
  - Added unit tests for irrelevant commodity changes, relevant commodity changes, no-plan dirtiness, and goal-level commodity relevance coverage.
- Deviations from original plan:
  - No `GoalSemantics` trait or standalone relevance mapper was introduced; the existing `GoalKindPlannerExt` contract was the cleaner extension point.
  - `decision_runtime.rs` did not need changes because the stored snapshot shape was already sufficient.
  - Golden hashes did not need updating; the existing golden e2e passed unchanged.
- Verification:
  - `cargo test -p worldwake-ai agent_tick`
  - `cargo test -p worldwake-ai goal_model`
  - `cargo test -p worldwake-ai --test golden_e2e`
  - `cargo test --workspace`
  - `cargo clippy --workspace`
