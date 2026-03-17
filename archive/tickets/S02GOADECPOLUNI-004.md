# S02GOADECPOLUNI-004: Migrate free interrupt evaluation to consume shared policy

**Status**: ✅ COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — interrupts.rs refactor (free interrupt path)
**Deps**: S02GOADECPOLUNI-001, S02GOADECPOLUNI-003

## Problem

`interrupt_freely()` in `interrupts.rs` hardcodes two goal-family-specific checks:
1. `matches!(kind, GoalKind::LootCorpse { .. })` — opportunistic loot with `no_medium_or_above_self_care_or_danger()` gate
2. `is_reactive_goal()` — determines if HigherPriorityGoal switch kind permits interrupt

These are the remaining goal-family policy branches in interrupts. Migrating them to `goal_family_policy()` completes the interrupt-side unification.

## Assumption Reassessment (2026-03-16)

1. `interrupt_freely()` at `interrupts.rs:81-151` first checks LootCorpse specifically (lines 89-97), then uses `is_reactive_goal()` in the switch_kind matching (lines 113-114, 141-142) — confirmed.
2. `is_reactive_goal()` at `interrupts.rs:246-249` returns true for `is_critical_survival_goal() || ReduceDanger || Heal` — confirmed.
3. `no_medium_or_above_self_care_or_danger()` at `interrupts.rs:251-269` scans ranked candidates for medium+ self-care/danger goals — confirmed.
4. The spec replaces `no_medium_or_above_self_care_or_danger()` with `DecisionContext.is_stressed_at_or_above(Medium)` — confirmed in spec Deliverable 6.
5. The spec notes these should be equivalent (same pressure → same classification) but golden tests verify — confirmed.
6. `BuryCorpse` gets `FreeInterruptRole::Normal`, NOT `Opportunistic` — confirmed in spec.

## Architecture Check

1. Replacing LootCorpse special-case with `FreeInterruptRole::Opportunistic` policy lookup generalizes the pattern. Any future opportunistic goal family just declares `Opportunistic` in policy.
2. Replacing `is_reactive_goal()` with `FreeInterruptRole::Reactive` check unifies the reactive/normal distinction in policy. The `HigherPriorityGoal` switch kind only permits interrupt when the challenger is Reactive; Normal goals require `SameClassMargin`.
3. `no_medium_or_above_self_care_or_danger()` is replaced by `DecisionContext.is_stressed_at_or_above(Medium)`. This is a semantic change from scanning ranked candidates to checking raw pressure classes. Golden tests validate equivalence.
4. No backwards-compatibility shims.

## What to Change

### 1. Replace LootCorpse special-case in `interrupt_freely()`

Replace the `if matches!(kind, GoalKind::LootCorpse { .. })` block with a policy-based check:
```rust
let policy = goal_family_policy(&challenger.grounded.key.kind);
if policy.free_interrupt == FreeInterruptRole::Opportunistic {
    return if !decision_context.is_stressed_at_or_above(GoalPriorityClass::Medium) {
        InterruptDecision::InterruptForReplan {
            trigger: InterruptTrigger::OpportunisticLoot,
        }
    } else {
        InterruptDecision::NoInterrupt
    };
}
```

### 2. Replace `is_reactive_goal()` checks in switch_kind matching

In both the relation-aware and non-relation-aware paths, replace:
```rust
GoalSwitchKind::HigherPriorityGoal if is_reactive_goal(&challenger.grounded.key.kind) =>
```
with:
```rust
GoalSwitchKind::HigherPriorityGoal
    if goal_family_policy(&challenger.grounded.key.kind).free_interrupt == FreeInterruptRole::Reactive =>
```

### 3. Thread `&DecisionContext` into `interrupt_freely()`

Pass the `DecisionContext` (already added to `evaluate_interrupt` signature in ticket 003) into `interrupt_freely()`.

### 4. Delete dead functions

- Delete `is_reactive_goal()`
- Delete `no_medium_or_above_self_care_or_danger()`

## Files to Touch

- `crates/worldwake-ai/src/interrupts.rs` (modify)

## Out of Scope

- Modifying `ranking.rs` (ticket 002)
- Modifying `agent_tick.rs` beyond what ticket 003 already changed
- Changing `interrupt_with_penalty()` (ticket 003)
- Changing `compare_goal_switch()` or `compare_relation_aware_goal_switch()`
- Changing `InterruptTrigger` enum variants
- Changes to `worldwake-core` or `worldwake-sim`

## Acceptance Criteria

### Tests That Must Pass

1. LootCorpse interrupts freely when no medium+ stress (via `DecisionContext.is_stressed_at_or_above(Medium)` returning false)
2. LootCorpse does NOT interrupt freely when stress is `>= Medium`
3. BuryCorpse does NOT get opportunistic interrupt (it has `Normal` role, not `Opportunistic`)
4. Self-care goals (Reactive) can interrupt via HigherPriorityGoal switch kind
5. ReduceDanger (Reactive) can interrupt via HigherPriorityGoal switch kind
6. Heal (Reactive) can interrupt via HigherPriorityGoal switch kind
7. EngageHostile (Normal) does NOT interrupt via HigherPriorityGoal — only via SameClassMargin
8. Enterprise goals (Normal) do NOT interrupt via HigherPriorityGoal — only via SameClassMargin
9. `is_reactive_goal()` function no longer exists in `interrupts.rs`
10. `no_medium_or_above_self_care_or_danger()` function no longer exists in `interrupts.rs`
11. All existing interrupt unit tests pass (behavioral equivalence)
12. All existing golden tests pass: `cargo test -p worldwake-ai`
13. `cargo clippy --workspace`

### Invariants

1. Free interrupt behavior is identical to pre-migration for all 17 goal families
2. `interrupts.rs` does not contain goal-family-specific branches — all family discrimination is via `goal_family_policy()`
3. Opportunistic interrupt gating uses `DecisionContext.is_stressed_at_or_above(Medium)`, not candidate scanning
4. Reactive interrupt routing uses `FreeInterruptRole::Reactive` check, not `is_reactive_goal()`

## Test Plan

### New/Modified Tests

1. Existing `interrupts.rs` tests — verify behavioral equivalence
2. Add test: BuryCorpse does NOT get opportunistic interrupt (explicitly distinguishes from LootCorpse)
3. Add test: Heal can interrupt via HigherPriorityGoal (Reactive) but NOT via penalty (covers the two-dimensional policy)

### Commands

1. `cargo test -p worldwake-ai interrupts`
2. `cargo test -p worldwake-ai` (includes golden tests)
3. `cargo test --workspace && cargo clippy --workspace`

## Outcome

- **Completion date**: 2026-03-17
- **What changed**: Replaced LootCorpse special-case with `FreeInterruptRole::Opportunistic` policy lookup, replaced `is_reactive_goal()` with `FreeInterruptRole::Reactive` policy lookup, replaced `no_medium_or_above_self_care_or_danger()` candidate scanning with `DecisionContext.is_stressed_at_or_above(Medium)`, threaded `DecisionContext` into `interrupt_freely()`, deleted both dead functions. Added 2 new tests (BuryCorpse not opportunistic, Heal reactive but no penalty).
- **Deviations**: `DecisionContext` passed by value (not reference) into `interrupt_freely()` per clippy pedantic (`trivially_copy_pass_by_ref`). If-not-else flipped per clippy pedantic.
- **Verification**: `cargo test --workspace` all pass, `cargo clippy --workspace` clean.
