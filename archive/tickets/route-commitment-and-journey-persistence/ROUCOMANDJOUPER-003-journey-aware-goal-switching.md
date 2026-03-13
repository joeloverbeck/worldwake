# ROUCOMANDJOUPER-003: Journey-Aware Goal Switching Margin Override

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — make goal-switch margin explicit at the API boundary
**Deps**: ROUCOMANDJOUPER-001, ROUCOMANDJOUPER-002

## Problem

`compare_goal_switch()` currently reads `budget.switch_margin_permille` internally. That makes the comparison helper own caller policy it should not own. Journey-aware switching needs the comparison to accept an explicit margin so caller-specific policy can stay outside `goal_switching.rs`.

## Assumption Reassessment (2026-03-13)

1. `compare_goal_switch()` in `goal_switching.rs` takes `&PlanningBudget` and extracts `budget.switch_margin_permille` for the `clears_switch_margin()` call — confirmed.
2. `clears_switch_margin()` compares `new_score` vs `current_score + required_increase` where `required_increase` is derived from `budget.switch_margin_permille.value()` — confirmed.
3. `compare_goal_switch()` is `pub(crate)` and has two production call sites today: `plan_selection.rs` and `interrupts.rs` — confirmed. The original ticket incorrectly scoped the change to plan selection only.
4. `Permille` has a `.value()` method returning the inner `u16` — confirmed by usage in the margin math.
5. `TravelDispositionProfile::route_replan_margin` already exists as a `Permille` in [`crates/worldwake-core/src/travel_disposition.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/travel_disposition.rs) via completed ticket 001 — confirmed.
6. `AgentDecisionRuntime` already has journey temporal fields and `has_active_journey()` via archived ticket 002 — confirmed, but this ticket still should not decide when a journey override applies.

## Architecture Check

1. The cleanest approach is still to change `compare_goal_switch()` to accept `margin: Permille` directly instead of `&PlanningBudget`. Margin comparison is a pure policy-free operation; callers own the policy for which margin to use.
2. Because `interrupts.rs` is also a caller, this ticket should update both production call sites to pass `budget.switch_margin_permille` explicitly and preserve current behavior everywhere.
3. Journey-aware margin selection remains a caller concern. It belongs in the orchestration layer that knows whether the agent is on an active journey, not inside `goal_switching.rs`.
4. Alternative: pass an `Option<Permille>` override. Rejected — it bakes caller fallback policy into a low-level helper.
5. No backwards-compatibility aliasing or shims.

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

Update both production call sites:

- In `plan_selection.rs`, pass `budget.switch_margin_permille` directly for now so behavior stays unchanged until the journey-aware caller policy lands.
- In `interrupts.rs`, also pass `budget.switch_margin_permille` directly so freely interruptible same-class switching keeps the current behavior.

### 4. Update tests in `goal_switching.rs`

All tests pass `&PlanningBudget::default()` — change to pass `PlanningBudget::default().switch_margin_permille` directly.

### 5. Preserve existing caller behavior with regression coverage

Keep current plan-selection and interrupt behavior unchanged by retaining their existing tests and adding explicit margin-focused unit coverage in `goal_switching.rs`.

## Files to Touch

- `crates/worldwake-ai/src/goal_switching.rs` (modify — change signature + body + tests)
- `crates/worldwake-ai/src/plan_selection.rs` (modify — update call site)
- `crates/worldwake-ai/src/interrupts.rs` (modify — update call site only)

## Out of Scope

- Journey-aware margin selection in plan selection or interrupt policy (follow-up caller work) — this ticket only makes the margin explicit at the helper boundary
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
7. Existing interrupt tests pass with the updated call site.
8. Existing suite: `cargo test -p worldwake-ai`
9. Existing suite: `cargo clippy --workspace`

### Invariants

1. Higher priority class always wins regardless of margin (unchanged behavior).
2. Same-class switching still requires the challenger to exceed the specified margin.
3. No new crate dependencies introduced.
4. The function remains `pub(crate)`.
5. No caller behavior changes occur in this ticket; both existing production callers continue to use `budget.switch_margin_permille`.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/goal_switching.rs` — update all existing tests to pass `Permille` instead of `&PlanningBudget`
2. `crates/worldwake-ai/src/goal_switching.rs` — new test: `higher_margin_makes_switching_harder`
3. `crates/worldwake-ai/src/goal_switching.rs` — new test: `zero_margin_allows_any_improvement`
4. `crates/worldwake-ai/src/plan_selection.rs` — existing regression tests cover unchanged caller behavior after the signature update
5. `crates/worldwake-ai/src/interrupts.rs` — existing regression tests cover unchanged caller behavior after the signature update

### Commands

1. `cargo test -p worldwake-ai goal_switching`
2. `cargo test -p worldwake-ai plan_selection`
3. `cargo test -p worldwake-ai interrupts`
4. `cargo test -p worldwake-ai`
5. `cargo clippy --workspace`

## Outcome

- Completion date: 2026-03-13
- What actually changed:
  - Changed `compare_goal_switch()` and its internal margin helper to accept an explicit `Permille` instead of reading `PlanningBudget` internally.
  - Updated both production callers, `plan_selection.rs` and `interrupts.rs`, to pass `budget.switch_margin_permille` explicitly so existing runtime behavior stays unchanged.
  - Added direct unit coverage for explicit-margin edge cases in `goal_switching.rs`, including higher-margin and zero-margin behavior.
  - Corrected the ticket before implementation to reflect the real caller graph and the existing locations of ticket 001/002 artifacts.
- Deviations from original plan:
  - The original ticket incorrectly scoped the API change to `plan_selection.rs`; the actual code change also had to touch `interrupts.rs`.
  - No journey-aware override policy landed here. That remains a follow-up caller/orchestration concern; this ticket only made the margin explicit at the helper boundary.
- Verification results:
  - `cargo test -p worldwake-ai goal_switching` ✅
  - `cargo test -p worldwake-ai plan_selection` ✅
  - `cargo test -p worldwake-ai interrupts` ✅
  - `cargo test -p worldwake-ai` ✅
  - `cargo clippy --workspace --all-targets -- -D warnings` ✅
