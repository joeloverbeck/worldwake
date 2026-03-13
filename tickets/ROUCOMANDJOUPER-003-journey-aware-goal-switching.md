# ROUCOMANDJOUPER-003: Journey-Aware Goal Switching Margin Override

**Status**: PENDING
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — modify `compare_goal_switch()` signature and logic
**Deps**: ROUCOMANDJOUPER-001, ROUCOMANDJOUPER-002

## Problem

`compare_goal_switch()` always uses `budget.switch_margin_permille` as the switching threshold. During an active journey, the agent's per-profile `TravelDispositionProfile::route_replan_margin` should replace the global margin, making committed travelers harder to divert mid-journey (or easier, depending on the agent's disposition).

## Assumption Reassessment (2026-03-13)

1. `compare_goal_switch()` in `goal_switching.rs` takes `&PlanningBudget` and extracts `budget.switch_margin_permille` for the `clears_switch_margin()` call — confirmed.
2. `clears_switch_margin()` compares `new_score` vs `current_score + required_increase` where `required_increase` is derived from `budget.switch_margin_permille.value()` — confirmed.
3. The function is `pub(crate)` and called from `plan_selection.rs` — confirmed.
4. `Permille` has a `.value()` method returning the inner `u16` — confirmed by usage in the margin math.
5. `TravelDispositionProfile::route_replan_margin` is a `Permille` (ticket 001) — per spec.

## Architecture Check

1. The cleanest approach: change `compare_goal_switch()` to accept `margin: Permille` directly instead of `&PlanningBudget`. The caller (plan selection) can decide which margin to pass based on journey state. This keeps margin logic in plan selection where journey awareness lives, and keeps goal switching pure/simple.
2. Alternative: pass an `Option<Permille>` override. Rejected — adds optional-override complexity when the caller can simply pass the right value.
3. No backwards-compatibility aliasing or shims.

## What to Change

### 1. Change `compare_goal_switch()` to accept `Permille` instead of `&PlanningBudget`

Before:
```rust
pub(crate) fn compare_goal_switch(
    current_class: GoalPriorityClass,
    current_motive: Option<u32>,
    challenger_class: GoalPriorityClass,
    challenger_motive: u32,
    budget: &PlanningBudget,
) -> Option<GoalSwitchKind>
```

After:
```rust
pub(crate) fn compare_goal_switch(
    current_class: GoalPriorityClass,
    current_motive: Option<u32>,
    challenger_class: GoalPriorityClass,
    challenger_motive: u32,
    switch_margin: Permille,
) -> Option<GoalSwitchKind>
```

### 2. Update `clears_switch_margin()` to accept `Permille` directly

Before: `fn clears_switch_margin(new_score: u32, current_score: u32, budget: &PlanningBudget) -> bool`
After: `fn clears_switch_margin(new_score: u32, current_score: u32, margin: Permille) -> bool`

The body changes from `budget.switch_margin_permille.value()` to `margin.value()`.

### 3. Update all call sites

In `plan_selection.rs`, the call to `compare_goal_switch()` changes from passing `budget` to passing the appropriate `Permille`. For now (before ticket 004 adds journey awareness), pass `budget.switch_margin_permille` to preserve existing behavior.

### 4. Update tests in `goal_switching.rs`

All tests pass `&PlanningBudget::default()` — change to pass `PlanningBudget::default().switch_margin_permille` directly.

## Files to Touch

- `crates/worldwake-ai/src/goal_switching.rs` (modify — change signature + body + tests)
- `crates/worldwake-ai/src/plan_selection.rs` (modify — update call site)

## Out of Scope

- Journey-aware margin selection in plan selection (ticket 004) — this ticket only changes the interface
- `TravelDispositionProfile` definition (ticket 001 — assumed complete)
- Journey temporal fields (ticket 002 — assumed complete)
- Any changes to `worldwake-core`, `worldwake-sim`, or `worldwake-systems`
- Adding `TravelDispositionProfile` to `BeliefView`

## Acceptance Criteria

### Tests That Must Pass

1. `challenger_with_higher_priority_always_switches` — still passes (higher priority bypasses margin).
2. `same_class_switch_requires_margin` — still passes with explicit `Permille(100)` instead of budget.
3. `same_class_switch_without_current_motive_is_disallowed` — still passes.
4. New test: passing a higher `Permille` (e.g., `Permille(300)`) makes it harder to switch — challenger with +20% improvement fails to switch with 300 permille margin but succeeds with 100 permille margin.
5. New test: passing `Permille(0)` means any improvement triggers a switch.
6. Existing plan selection tests pass with the updated call site.
7. Existing suite: `cargo test -p worldwake-ai`
8. Existing suite: `cargo clippy --workspace`

### Invariants

1. Higher priority class always wins regardless of margin (unchanged behavior).
2. Same-class switching still requires the challenger to exceed the specified margin.
3. No new crate dependencies introduced.
4. The function remains `pub(crate)`.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/goal_switching.rs` — update all existing tests to pass `Permille` instead of `&PlanningBudget`
2. `crates/worldwake-ai/src/goal_switching.rs` — new test: `higher_margin_makes_switching_harder`
3. `crates/worldwake-ai/src/goal_switching.rs` — new test: `zero_margin_allows_any_improvement`

### Commands

1. `cargo test -p worldwake-ai goal_switching`
2. `cargo test -p worldwake-ai plan_selection`
3. `cargo test -p worldwake-ai`
4. `cargo clippy --workspace`
