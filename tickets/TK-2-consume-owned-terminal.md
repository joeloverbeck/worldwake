# TK-2: ConsumeOwnedCommodity Terminal Condition Treats Possession as Goal Satisfaction

**Priority**: P1
**Crate(s)**: `worldwake-ai`
**Source**: Simulation observer report (2026-04-06) + GT-1 root-cause analysis

## Problem

When an agent owns a consumable commodity (e.g., Apple) and has the corresponding need elevated (hunger=400+), the planner selects `ConsumeOwnedCommodity(Apple)` as the goal but finds `GoalSatisfied` with 0 steps. The terminal condition checks "does the agent possess this commodity?" which is true — so the planner considers the goal already met without generating an `eat` action step. The agent never actually eats.

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

The `ConsumeOwnedCommodity` goal's terminal condition in the A* search checks whether the agent possesses the target commodity. When the agent already possesses it, the search immediately returns `GoalSatisfied` at the root node (0 steps). The `eat`/`drink` action is never added as a plan step because the goal appears already satisfied.

The semantic gap: "owning the commodity" is not the same as "having consumed it to satisfy the need." The goal should require the consumption action as a mandatory terminal step, or the terminal condition should check need satisfaction rather than possession.

## Acceptance Criteria

1. The `#[ignore]` test `golden_fallback_to_addressable_need_when_top_need_unsatisfiable` in `crates/worldwake-ai/tests/golden_ai_decisions.rs` passes (remove `#[ignore]`)
2. An agent that owns a consumable commodity AND has the corresponding need elevated actually dispatches the consume action (eat/drink)
3. All existing golden tests continue to pass (especially `golden_thirst_driven_acquisition`)
4. The fix correctly distinguishes "possession for future consumption" from "consumption to satisfy current need"
5. Scenario 84 in `golden_merchant_selling.rs` can be tightened to assert `saw_staff_market` (currently asserts only travel + arrival due to this same terminal issue blocking `staff_market` dispatch)

## Investigation Pointers

- Terminal condition logic: search for the `ConsumeOwnedCommodity` match arm in the A* terminal/goal-satisfied check (likely in `crates/worldwake-ai/src/search.rs` or `goal_model.rs`)
- Compare with how `golden_thirst_driven_acquisition` succeeds — that test starts with thirst=0 so the need climbs past the threshold AFTER giving water, meaning the goal is generated fresh when the need is high
- The `GoalSatisfied` terminal must include the actual consume step (PlannerOpKind::Consume) when the agent has the commodity but the need is still elevated
