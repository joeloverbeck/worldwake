# S02GOADECPOLUNI-003: Migrate penalty interrupt evaluation to consume shared policy

**Status**: PENDING
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — interrupts.rs refactor (penalty path)
**Deps**: S02GOADECPOLUNI-001

## Problem

`interrupt_with_penalty()` in `interrupts.rs` hardcodes `is_critical_survival_goal()` and a separate `ReduceDanger` match to determine which goals can interrupt penalty-protected actions. This duplicates the self-care/danger classification that `goal_family_policy()` now declares authoritatively.

## Assumption Reassessment (2026-03-16)

1. `interrupt_with_penalty()` at `interrupts.rs:63-79` checks `is_critical_survival_goal()` (returns CriticalSurvival trigger) and `ReduceDanger` (returns CriticalDanger trigger), else NoInterrupt — confirmed.
2. `is_critical_survival_goal()` at `interrupts.rs:232-243` matches ConsumeOwnedCommodity, AcquireCommodity(SelfConsume), Sleep, Relieve, Wash — confirmed.
3. Both checks require `challenger.priority_class == Critical` — confirmed at line 64.
4. The spec's `PenaltyInterruptEligibility::WhenCritical { trigger }` maps exactly to these two branches — confirmed in spec Deliverable 6.

## Architecture Check

1. Replacing the two-branch `if/else if` with a single policy lookup + match is simpler and extensible. Future penalty-eligible goals just add a `WhenCritical` entry in `goal_family_policy()`.
2. No backwards-compatibility shims. `is_critical_survival_goal()` is deleted outright.

## What to Change

### 1. Add `&DecisionContext` parameter to `evaluate_interrupt()`

The `DecisionContext` parameter is needed for ticket 004 (free interrupt migration). Adding it now avoids a second signature change. This ticket does not use it in the penalty path but threads it through.

### 2. Replace `interrupt_with_penalty()` body

Replace:
```rust
fn interrupt_with_penalty(challenger: &RankedGoal) -> InterruptDecision {
    if challenger.priority_class != GoalPriorityClass::Critical {
        return InterruptDecision::NoInterrupt;
    }
    if is_critical_survival_goal(&challenger.grounded.key.kind) { ... }
    else if matches!(challenger.grounded.key.kind, GoalKind::ReduceDanger) { ... }
    else { NoInterrupt }
}
```
With:
```rust
fn interrupt_with_penalty(challenger: &RankedGoal) -> InterruptDecision {
    if challenger.priority_class != GoalPriorityClass::Critical {
        return InterruptDecision::NoInterrupt;
    }
    let policy = goal_family_policy(&challenger.grounded.key.kind);
    match policy.penalty_interrupt {
        PenaltyInterruptEligibility::WhenCritical { trigger } => {
            InterruptDecision::InterruptForReplan { trigger }
        }
        PenaltyInterruptEligibility::Never => InterruptDecision::NoInterrupt,
    }
}
```

### 3. Delete `is_critical_survival_goal()`

Remove the function entirely from `interrupts.rs`.

## Files to Touch

- `crates/worldwake-ai/src/interrupts.rs` (modify)
- `crates/worldwake-ai/src/lib.rs` (modify — update `evaluate_interrupt` re-export if signature changed)
- `crates/worldwake-ai/src/agent_tick.rs` (modify — pass placeholder `DecisionContext` to `evaluate_interrupt()` call)

## Out of Scope

- Migrating `interrupt_freely()` (ticket 004)
- Removing `is_reactive_goal()` or `no_medium_or_above_self_care_or_danger()` (ticket 004)
- Building `DecisionContext` properly in agent_tick (ticket 005)
- Modifying `ranking.rs`
- Changes to `worldwake-core` or `worldwake-sim`

## Acceptance Criteria

### Tests That Must Pass

1. Critical self-care goals (ConsumeOwnedCommodity, AcquireCommodity(SelfConsume), Sleep, Relieve, Wash) at Critical priority interrupt penalty-protected actions with CriticalSurvival trigger
2. ReduceDanger at Critical priority interrupts penalty-protected actions with CriticalDanger trigger
3. Heal at Critical priority does NOT interrupt penalty-protected actions
4. EngageHostile at Critical priority does NOT interrupt penalty-protected actions
5. Enterprise goals at Critical priority do NOT interrupt penalty-protected actions
6. Non-Critical priority goals never interrupt penalty-protected actions
7. `is_critical_survival_goal()` function no longer exists in `interrupts.rs`
8. All existing interrupt unit tests pass
9. All existing golden tests pass: `cargo test -p worldwake-ai`
10. `cargo clippy --workspace`

### Invariants

1. Penalty interrupt behavior is identical to pre-migration for all 17 goal families
2. `interrupts.rs` does not contain goal-family-specific penalty logic — it delegates to `goal_family_policy()`
3. The `priority_class == Critical` gate remains (policy eligibility + critical class = interrupt)

## Test Plan

### New/Modified Tests

1. Existing `interrupts.rs` penalty tests — verify behavioral equivalence
2. Add test: Heal at Critical does not interrupt penalty (explicitly covers the Heal ≠ penalty-eligible distinction)

### Commands

1. `cargo test -p worldwake-ai interrupts`
2. `cargo test -p worldwake-ai` (includes golden tests)
3. `cargo test --workspace && cargo clippy --workspace`
