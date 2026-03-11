# E13DECARC-015: Reactive interrupt evaluation

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: None — AI-layer logic
**Deps**: E13DECARC-006, E13DECARC-008, E13DECARC-009

## Problem

Interrupt evaluation runs each tick but replanning does not. The interrupt system must decide whether the AI should drop its current plan/action and mark itself dirty for a fresh decision pass, while respecting the action framework's `Interruptibility`.

In the current codebase, higher-priority and same-class-switch interrupt decisions should not re-derive ranking logic independently. That logic already exists in E13's ranking and plan-selection work. This ticket should add a small, pure interrupt-policy layer that consumes ranked candidates plus current-plan state, rather than inventing a second switching system.

## Assumption Reassessment (2026-03-11)

1. `Interruptibility` in `worldwake-sim` is `NonInterruptible | InterruptibleWithPenalty | FreelyInterruptible` — confirmed.
2. There is no existing `crates/worldwake-ai/src/interrupts.rs` stub. This ticket must create the module instead of "filling in" an empty file.
3. `GoalPriorityClass`, `GroundedGoal`, and `RankedGoal` already exist in `crates/worldwake-ai/src/goal_model.rs`.
4. The switch-margin policy already exists in `crates/worldwake-ai/src/plan_selection.rs`; this ticket should reuse or extract that comparison logic instead of duplicating it.
5. `UtilityProfile` and `BlockedIntentMemory` live in `worldwake-core`, not `worldwake-ai`.
6. Derived pressures from E13DECARC-006 are already consumed by candidate generation/ranking. For interrupt policy, the clean seam is ranked candidates plus current-plan validity, not a second copy of raw-pressure classification.
7. `ActionInstance`/`ActionDef.interruptibility` exist in `worldwake-sim`, but actual cancellation/abort remains out of scope here; this ticket only decides whether AI should request replanning.

## Architecture Check

1. Interrupt evaluation stays cheap and pure. It returns a decision; it does not perform planning or mutate world state.
2. Decision-loop integration remains in E13DECARC-016. This ticket only provides the reusable policy primitive and tests.
3. `NonInterruptible` is absolute for voluntary E13 interrupts.
4. `InterruptibleWithPenalty` is limited to critical survival/danger or invalid-plan interrupts.
5. `FreelyInterruptible` may switch for a higher-priority challenger, or a same-class challenger that clears the existing switch margin.
6. Opportunity interrupts remain Phase-2 narrow: loot only, and only when no self-care or danger candidate is `Medium+`.
7. To avoid second-system drift, interrupt comparison must share the same switching policy used by plan selection.

## What to Change

### 1. Create `crates/worldwake-ai/src/interrupts.rs`

```rust
pub fn evaluate_interrupt(
    runtime: &AgentDecisionRuntime,
    current_action_interruptibility: Interruptibility,
    ranked_candidates: &[RankedGoal],
    plan_valid: bool,
    budget: &PlanningBudget,
) -> InterruptDecision

pub enum InterruptDecision {
    NoInterrupt,
    InterruptForReplan { trigger: InterruptTrigger },
}

pub enum InterruptTrigger {
    CriticalSurvival,
    CriticalDanger,
    HigherPriorityGoal,
    SuperiorSameClassPlan,
    PlanInvalid,
    OpportunisticLoot,
}
```

Notes:
- This API intentionally consumes already-ranked candidates. That keeps interrupt policy aligned with the current goal model and avoids recomputing priority logic from raw pressures.
- `plan_valid` is an explicit input because plan invalidation is already defined by plan revalidation work; the interrupt layer should not guess validity from indirect signals.

### 2. Reuse the current switch-policy logic instead of cloning it

Extract or share the "higher class wins / same class needs switch margin" comparison currently embedded in `plan_selection.rs`, then use the same rule for interrupt evaluation.

### 3. Implement interrupt rules per interruptibility level

**`NonInterruptible`**: Never interrupted by E13.

**`InterruptibleWithPenalty`**: interrupt only for:
- `Critical` danger or self-care
- Current plan is invalid

**`FreelyInterruptible`**: May interrupt for:
- Higher-priority survival/danger/heal challengers
- Same-class challenger exceeding switch margin
- Strictly local opportunistic loot ONLY when no self-care or danger goal is `Medium+`

### 4. Export the module from `crates/worldwake-ai/src/lib.rs`

## Files to Touch

- `crates/worldwake-ai/src/interrupts.rs` (new)
- `crates/worldwake-ai/src/plan_selection.rs` (modify if switch logic is extracted/shared)
- `crates/worldwake-ai/src/lib.rs` (export new module)

## Out of Scope

- Actually cancelling the action or emitting simulation-side interrupt/abort requests — E13DECARC-016
- Full replanning logic
- Agent tick integration / dirty-flag mutation wiring — E13DECARC-016
- Determining current action interruptibility from scheduler state — E13DECARC-016
- Human-controlled agents

## Acceptance Criteria

### Tests That Must Pass

1. `NonInterruptible` action -> `NoInterrupt` even with a critical danger challenger
2. `InterruptibleWithPenalty` + Critical danger -> `InterruptForReplan(CriticalDanger)`
3. `InterruptibleWithPenalty` + High danger -> `NoInterrupt`
4. `InterruptibleWithPenalty` + invalid plan -> `InterruptForReplan(PlanInvalid)`
5. `FreelyInterruptible` + higher priority survival goal -> `InterruptForReplan(HigherPriorityGoal)`
6. `FreelyInterruptible` + opportunistic loot with no Medium+ needs -> `InterruptForReplan(OpportunisticLoot)`
7. `FreelyInterruptible` + opportunistic loot with Medium hunger -> `NoInterrupt` (loot suppressed)
8. `FreelyInterruptible` + same-class challenger below switch margin -> `NoInterrupt`
9. `FreelyInterruptible` + same-class challenger at/above switch margin -> `InterruptForReplan(SuperiorSameClassPlan)`
10. Existing suite: `cargo test --workspace`

### Invariants

1. Interrupt evaluation is pure and does NOT perform replanning directly
2. `NonInterruptible` is absolute — E13 never overrides it
3. Opportunity interrupts are narrow — only loot, only when no Medium+ needs
4. Higher-priority / switch-margin comparisons reuse the same rule as plan selection
5. No global state queries or `&World` access in `worldwake-ai`

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/interrupts.rs` — tests per interruptibility level with ranked-candidate scenarios
2. `crates/worldwake-ai/src/plan_selection.rs` — adjusted only if shared switch-policy extraction changes coverage shape

### Commands

1. `cargo test -p worldwake-ai`
2. `cargo test --workspace`
3. `cargo clippy --workspace`

## Outcome

Completed: 2026-03-11

What actually changed:
- Added `crates/worldwake-ai/src/interrupts.rs` with a pure interrupt-policy primitive that evaluates `Interruptibility`, ranked challengers, plan validity, and Phase 2 loot restrictions.
- Added `crates/worldwake-ai/src/goal_switching.rs` so interrupt evaluation and plan selection share the same higher-priority and switch-margin comparison rule.
- Updated `crates/worldwake-ai/src/plan_selection.rs` to reuse the shared switch-policy helper instead of carrying a duplicated margin implementation.
- Exported the new interrupt API from `crates/worldwake-ai/src/lib.rs`.
- Added focused interrupt and switch-policy tests covering critical danger, invalid-plan interrupts, same-class margin behavior, loot suppression, and protection against higher-priority enterprise thrash.

Deviations from original plan:
- The original ticket assumed an existing `interrupts.rs` stub and outdated `Uninterruptible` naming; both assumptions were corrected first.
- The final interrupt API consumes ranked candidates and explicit plan validity rather than re-deriving pressure/ranking logic inside the interrupt layer.
- `No action running` handling was removed from this ticket's acceptance surface because that belongs to agent tick integration in E13DECARC-016, not to the pure interrupt-policy primitive.

Verification:
- `cargo test -p worldwake-ai`
- `cargo test --workspace`
- `cargo clippy --workspace`
