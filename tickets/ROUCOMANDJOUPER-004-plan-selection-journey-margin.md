# ROUCOMANDJOUPER-004: Plan Selection Integration with Journey Margin

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — modify `select_best_plan()` to use per-agent journey margin
**Deps**: ROUCOMANDJOUPER-003

## Problem

`select_best_plan()` always passes the global `budget.switch_margin_permille` to `compare_goal_switch()`. When an agent has an active journey (plan with remaining Travel steps and `journey_established_at` is `Some`), the agent's `TravelDispositionProfile::route_replan_margin` should be used instead, making committed travelers harder (or easier, per-agent) to divert.

## Assumption Reassessment (2026-03-13)

1. `select_best_plan()` takes `(candidates, plans, current: &AgentDecisionRuntime, budget: &PlanningBudget)` — confirmed.
2. After ticket 003, `compare_goal_switch()` accepts a `Permille` margin instead of `&PlanningBudget` — assumed complete.
3. `AgentDecisionRuntime` has `has_active_journey()` after ticket 002 — assumed complete.
4. `TravelDispositionProfile` has `route_replan_margin: Permille` after ticket 001 — assumed complete.
5. `select_best_plan()` is called from `AgentTickDriver` where the belief view and agent's profile are accessible — confirmed by reading `agent_tick.rs`.

## Architecture Check

1. The cleanest approach: add an `Option<Permille>` parameter to `select_best_plan()` representing the journey margin override. When `Some`, use it instead of `budget.switch_margin_permille`. The caller (`AgentTickDriver`) determines whether the agent has an active journey and has a `TravelDispositionProfile`, and passes the override accordingly.
2. Alternative: pass `TravelDispositionProfile` directly. Rejected — `select_best_plan` shouldn't know about travel-specific types; it just needs a margin value.
3. Alternative: pass the entire `AgentDecisionRuntime` and do journey detection inside. Rejected — it already receives `current: &AgentDecisionRuntime` but adding profile lookup here would require belief view access, mixing concerns.
4. No backwards-compatibility aliasing or shims.

## What to Change

### 1. Add `journey_margin_override` parameter to `select_best_plan()`

```rust
pub fn select_best_plan(
    candidates: &[RankedGoal],
    plans: &[(GoalKey, Option<PlannedPlan>)],
    current: &AgentDecisionRuntime,
    budget: &PlanningBudget,
    journey_margin_override: Option<Permille>,
) -> Option<PlannedPlan>
```

In the body, when calling `compare_goal_switch()`, pass:
```rust
let margin = journey_margin_override.unwrap_or(budget.switch_margin_permille);
```

### 2. Update call site in `AgentTickDriver`

In `agent_tick.rs` where `select_best_plan()` is called, determine the margin override:

```rust
let journey_margin = if runtime.has_active_journey() {
    view.travel_disposition_profile(agent)
        .map(|profile| profile.route_replan_margin)
} else {
    None
};
```

Pass `journey_margin` as the last argument to `select_best_plan()`.

### 3. Add `travel_disposition_profile()` to `BeliefView` trait (if not already present)

Check whether `BeliefView` already has a `travel_disposition_profile()` accessor. If not, add one following the pattern of existing profile accessors (`combat_profile()`, `trade_disposition_profile()`, `metabolism_profile()`):

```rust
fn travel_disposition_profile(&self, agent: EntityId) -> Option<TravelDispositionProfile>;
```

Implement it in `OmniscientBeliefView` to read from `world.get_travel_disposition_profile(agent)`.

### 4. Update existing `select_best_plan` tests

All existing tests pass `None` for the new parameter to preserve existing behavior.

## Files to Touch

- `crates/worldwake-ai/src/plan_selection.rs` (modify — add parameter, use in margin selection)
- `crates/worldwake-ai/src/agent_tick.rs` (modify — compute and pass journey margin override)
- `crates/worldwake-sim/src/belief_view.rs` (modify — add trait method if missing)
- `crates/worldwake-sim/src/omniscient_belief_view.rs` (modify — implement trait method if missing)

## Out of Scope

- `compare_goal_switch()` signature change (ticket 003 — assumed complete)
- Journey temporal field lifecycle (when to set/clear `journey_established_at`) (tickets 005, 006)
- `TravelDispositionProfile` definition (ticket 001)
- Debug surface (ticket 007)
- Changes to `worldwake-systems`
- Blocked-intent integration (ticket 006)

## Acceptance Criteria

### Tests That Must Pass

1. Existing test `selection_prefers_higher_priority_class_before_cost` — passes with `journey_margin_override: None`.
2. Existing test `same_class_replacement_requires_switch_margin` — passes with `journey_margin_override: None`.
3. New test: with `journey_margin_override: Some(Permille(300))`, a same-class challenger with +15% motive improvement does NOT switch (would switch with default 100 permille margin).
4. New test: with `journey_margin_override: Some(Permille(300))`, a same-class challenger with +35% motive improvement DOES switch.
5. New test: with `journey_margin_override: Some(Permille(0))`, any motive improvement triggers a switch.
6. New test: higher priority class always wins regardless of `journey_margin_override` value.
7. Existing suite: `cargo test -p worldwake-ai`
8. Existing suite: `cargo clippy --workspace`

### Invariants

1. When `journey_margin_override` is `None`, behavior is identical to pre-ticket behavior.
2. Higher priority class always overrides margin (unchanged).
3. Same-goal replanning is unaffected by margin (same goal = always accept refresh).
4. Deterministic tie-breaking is unaffected.
5. No new cross-crate coupling beyond the existing `worldwake-ai` → `worldwake-sim` → `worldwake-core` dependency chain.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/plan_selection.rs` — update all existing tests to pass `None` for journey margin
2. `crates/worldwake-ai/src/plan_selection.rs` — new test: `journey_margin_override_raises_switching_threshold`
3. `crates/worldwake-ai/src/plan_selection.rs` — new test: `journey_margin_override_exceeded_allows_switch`
4. `crates/worldwake-ai/src/plan_selection.rs` — new test: `journey_margin_override_does_not_affect_priority_class_switch`
5. `crates/worldwake-ai/src/plan_selection.rs` — new test: `zero_journey_margin_override_allows_any_improvement`

### Commands

1. `cargo test -p worldwake-ai plan_selection`
2. `cargo test -p worldwake-ai`
3. `cargo clippy --workspace`
