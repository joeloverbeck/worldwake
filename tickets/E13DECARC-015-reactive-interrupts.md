# E13DECARC-015: Reactive interrupt evaluation

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: None — AI-layer logic
**Deps**: E13DECARC-006, E13DECARC-008, E13DECARC-009

## Problem

Interrupt evaluation runs each tick but replanning does not. The interrupt system checks whether the current action should be interrupted based on danger, homeostatic pressures, pain, wounds, and the action's interruptibility level. Interrupts must respect the `Interruptibility` enum from the action framework.

## Assumption Reassessment (2026-03-11)

1. `Interruptibility` enum in `worldwake-sim`: `FreelyInterruptible`, `InterruptibleWithPenalty`, `Uninterruptible` — confirmed (spec uses `NonInterruptible` but codebase uses `Uninterruptible`; will use codebase name).
2. `ActionInstance` has access to its `ActionDef` which has `interruptibility` — confirmed.
3. Derived pressures (pain, danger) from E13DECARC-006.
4. Priority classes from E13DECARC-008.
5. `GoalPriorityClass` with `Ord` from E13DECARC-004.
6. `PlanningBudget.switch_margin_permille` from E13DECARC-009.

## Architecture Check

1. Interrupt evaluation is cheap (runs every tick) — it does NOT trigger full replanning.
2. It sets `runtime.dirty = true` when an interrupt is warranted, which triggers replanning on the next decision pass.
3. `Uninterruptible` actions are never voluntarily interrupted by E13.
4. `InterruptibleWithPenalty` only for Critical danger/self-care or invalid plan.
5. `FreelyInterruptible` allows more liberal interruption for higher-priority goals.
6. Opportunity interrupts (loot) are intentionally narrow in Phase 2.

## What to Change

### 1. Implement interrupt evaluation in `worldwake-ai/src/interrupts.rs`

```rust
pub fn evaluate_interrupt(
    view: &dyn BeliefView,
    agent: EntityId,
    runtime: &AgentDecisionRuntime,
    current_action_interruptibility: Option<Interruptibility>,
    utility: &UtilityProfile,
    budget: &PlanningBudget,
) -> InterruptDecision

pub enum InterruptDecision {
    NoInterrupt,
    InterruptForReplan { reason: InterruptReason },
}

pub enum InterruptReason {
    CriticalSurvival,
    CriticalDanger,
    HigherPriorityGoal,
    PlanInvalid,
    SuperiorSameClassPlan,
    OpportunisticLoot,
}
```

### 2. Implement interrupt rules per interruptibility level

**`Uninterruptible`**: Never interrupted by E13.

**`InterruptibleWithPenalty`**: Interrupt only for:
- `Critical` danger or self-care
- Current plan is invalid

**`FreelyInterruptible`**: May interrupt for:
- Higher priority-class survival/danger/heal goals
- Same-class superior plan exceeding switch margin
- Strictly local opportunistic loot ONLY when no self-care or danger goal is `Medium+`

### 3. Integrate derived pressures

Use `derive_pain_pressure()`, `derive_danger_pressure()`, and `classify_band()` from E13DECARC-006 to determine current urgency levels.

## Files to Touch

- `crates/worldwake-ai/src/interrupts.rs` (modify — was empty stub)

## Out of Scope

- Actually cancelling the action (emitting `InputKind::CancelAction`) — E13DECARC-016
- Full replanning logic — E13DECARC-012
- Agent tick integration — E13DECARC-016
- Interrupt evaluation for human-controlled agents (human agents don't use AI interrupts)

## Acceptance Criteria

### Tests That Must Pass

1. `Uninterruptible` action -> `NoInterrupt` even with Critical danger
2. `InterruptibleWithPenalty` + Critical danger -> `InterruptForReplan(CriticalDanger)`
3. `InterruptibleWithPenalty` + High danger -> `NoInterrupt`
4. `InterruptibleWithPenalty` + invalid plan -> `InterruptForReplan(PlanInvalid)`
5. `FreelyInterruptible` + higher priority survival goal -> `InterruptForReplan(HigherPriorityGoal)`
6. `FreelyInterruptible` + opportunistic loot with no Medium+ needs -> `InterruptForReplan(OpportunisticLoot)`
7. `FreelyInterruptible` + opportunistic loot with Medium hunger -> `NoInterrupt` (loot suppressed)
8. `FreelyInterruptible` + same-class plan below switch margin -> `NoInterrupt`
9. No action running (None interruptibility) -> always eligible for planning
10. Existing suite: `cargo test --workspace`

### Invariants

1. Interrupt evaluation does NOT trigger replanning directly — only sets dirty flag
2. `Uninterruptible` is absolute — E13 never overrides it
3. Opportunity interrupts are narrow — only loot, only when no Medium+ needs
4. No global state queries — all through `BeliefView`

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/interrupts.rs` — tests per interruptibility level with various pressure scenarios

### Commands

1. `cargo test -p worldwake-ai`
2. `cargo test --workspace`
3. `cargo clippy --workspace`
