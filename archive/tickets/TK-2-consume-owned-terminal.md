# TK-2: ConsumeOwnedCommodity Terminal Condition Treats Possession as Goal Satisfaction

**Status**: COMPLETED
**Priority**: P1
**Crate(s)**: `worldwake-ai`
**Source**: Simulation observer report (2026-04-06) + GT-1 root-cause analysis

## Assumption Reassessment

1. Ticket says the `ConsumeOwnedCommodity` terminal condition treats commodity possession as goal satisfaction. Live code in `crates/worldwake-ai/src/goal_model.rs` already checks the relevant need against `DriveThresholds::{hunger,thirst}.medium()`. Correction applied: root cause reframed to the live mismatch between candidate emission at the `low` threshold and goal satisfaction at the `medium` threshold. Safe because this is a factual branch-state correction.

2. Ticket says the `GoalSatisfied` terminal must include the actual consume step because possession alone short-circuits planning. Live search in `crates/worldwake-ai/src/search/mod.rs` returns a zero-step `GoalSatisfied` plan whenever `GoalKind::is_satisfied()` is already true at the root node. Correction applied: the owned fix surface is the `ConsumeOwnedCommodity` satisfaction boundary, plus the golden assertions that currently encode the stale diagnosis. Safe because it keeps the ticket scoped to the same planner contract and affected tests.

3. Ticket says Scenario 84 in `golden_merchant_selling.rs` should tighten to require `staff_market`. Live `SellCommodity` satisfaction in `crates/worldwake-ai/src/goal_model.rs` is listing-based: the goal is satisfied once the merchant is at the home market and has listed sale lots there. Correction applied: Scenario 84 now tightens to require the merchant to reach home and get the bread lot listed for sale there, not specifically to start `staff_market`. Safe because it matches the current authoritative goal contract.

## Problem

When an agent owns a consumable commodity (e.g., Apple) and has the corresponding need elevated into the planner's low-urgency band, the planner can still select `ConsumeOwnedCommodity(Apple)` and find `GoalSatisfied` with 0 steps. Candidate generation emits the goal once the need crosses the `low` threshold, but the terminal condition already treats the goal as satisfied when the need is below the `medium` threshold. For needs in the `[low, medium)` band, the planner therefore considers the goal already met without generating an `eat` or `drink` step. The agent never actually consumes.

### Trace Evidence

```
[tick 0] PLAN: selected=ConsumeOwnedCommodity [ConsumeOwnedCommodity { commodity: Apple }],
  source=SearchSelection,
  selected_plan=GoalSatisfied[steps=0, next_index=None, next_step=none],
  candidates=2, plans_found=2
```

This repeats for all 200 ticks. The planner "succeeds" every tick but generates no action steps.

### Impact

- Agent with owned food never eats despite hunger pressure
- Agent with owned water never drinks despite thirst pressure
- `golden_thirst_driven_acquisition` works because the agent starts with thirst=0 and the need climbs past the threshold — the existing test doesn't exercise the "already owns commodity + already needs it" path
- This was the root cause of Guard Theron being idle for 1024 ticks in the simulation observer run

## Root Cause

The `ConsumeOwnedCommodity` goal is emitted once hunger/thirst crosses the corresponding `DriveThresholds::* .low()` band in `candidate_generation.rs`, but the A* root-node terminal check uses `GoalKind::is_satisfied()` from `goal_model.rs`, which currently clears the goal when the need is below `DriveThresholds::* .medium()`. That mismatch makes any low-band-but-sub-medium owned-consumption goal look complete before the planner adds an `eat`/`drink` step.

The semantic gap is between "this need is urgent enough to plan around now" and "this need is already relieved enough to stop planning around." Those boundaries must align for `ConsumeOwnedCommodity`, so the zero-step root success disappears while possession-for-later remains distinct from active consumption.

## Acceptance Criteria

1. The `#[ignore]` test `golden_fallback_to_addressable_need_when_top_need_unsatisfiable` in `crates/worldwake-ai/tests/golden_ai_decisions.rs` passes (remove `#[ignore]`)
2. An agent that owns a consumable commodity AND has the corresponding need elevated actually dispatches the consume action (eat/drink)
3. All existing golden tests continue to pass (especially `golden_thirst_driven_acquisition`)
4. The fix correctly distinguishes "possession for future consumption" from "consumption to satisfy current need"
5. Scenario 84 in `golden_merchant_selling.rs` tightens to assert that the remote merchant reaches the home market and gets the bread lot listed for sale there

## Investigation Pointers

- Terminal condition logic: search for the `ConsumeOwnedCommodity` match arm in the A* terminal/goal-satisfied check (likely in `crates/worldwake-ai/src/search.rs` or `goal_model.rs`)
- Compare with how `golden_thirst_driven_acquisition` succeeds — that test starts with thirst=0 so the need climbs past the threshold AFTER giving water, meaning the goal is generated fresh when the need is high
- The `GoalSatisfied` terminal must include the actual consume step (PlannerOpKind::Consume) when the agent has the commodity but the need is still elevated

## Outcome

Completed on 2026-04-06.

- Aligned `ConsumeOwnedCommodity` planner satisfaction with the live low-band emission contract, using the commodity's real `CommodityConsumableProfile` instead of hard-coded single-need checks.
- Updated hypothetical consume state so planner simulation matches runtime `eat`/`drink` effects for multi-relief consumables such as Apple.
- Re-enabled the fallback golden and tightened Scenario 84 to assert the live sell-goal contract: the merchant reaches home and gets the bread lot listed for sale there.
- Bounded deviation from the original ticket wording: the root cause was not commodity possession alone, and Scenario 84 does not lawfully require `staff_market` start under the current `SellCommodity` contract.

## Deviations

- Reassessment showed a separate low-band zero-step loop for `Relieve` and `Sleep` once TK-2's consume path began working. That follow-up is tracked separately in `tickets/TK-3-self-care-low-band-terminal.md`.

## Verification Result

- Passed `cargo test -p worldwake-ai --test golden_ai_decisions golden_fallback_to_addressable_need_when_top_need_unsatisfiable -- --exact`
- Passed `cargo test -p worldwake-ai --test golden_ai_decisions golden_thirst_driven_acquisition -- --exact`
- Passed `cargo test -p worldwake-ai --test golden_merchant_selling move_cargo_then_sell_commodity_plan_shape -- --exact`
- Passed `cargo test -p worldwake-ai`
- Passed `cargo clippy --workspace --all-targets -- -D warnings`
