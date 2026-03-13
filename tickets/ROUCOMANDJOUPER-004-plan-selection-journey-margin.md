# ROUCOMANDJOUPER-004: Controller-Level Journey Switch Margin Policy

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — compute and apply an effective journey switch margin at the controller layer
**Deps**: ROUCOMANDJOUPER-003

## Problem

Journey commitment should influence all same-class reprioritization decisions, not only idle plan replacement.

Today the controller has two separate decision paths that use goal-switch margin semantics:

- `select_best_plan()` while the agent is idle and choosing the next plan
- `evaluate_interrupt()` while the agent has an active action and is deciding whether to abandon it for a challenger

If ticket 004 only overrides the margin in `select_best_plan()`, active-action interrupt decisions will still use the default global margin and journey commitment will be inconsistent.

## Assumption Reassessment (2026-03-13)

1. After ticket 003, `compare_goal_switch()` accepts an explicit `Permille` margin instead of `&PlanningBudget` — completed.
2. `select_best_plan()` is called from `AgentTickDriver`, and `evaluate_interrupt()` is also called from `AgentTickDriver` via `handle_active_action_phase()` — confirmed by reading `agent_tick.rs`.
3. `select_best_plan()` and `evaluate_interrupt()` currently still receive enough information to apply only the default global margin, not a controller-computed effective margin — confirmed.
4. `AgentDecisionRuntime` has `has_active_journey()` after ticket 002 — confirmed.
5. `TravelDispositionProfile` has `route_replan_margin: Permille` after ticket 001 — confirmed.
6. `BeliefView` currently exposes `metabolism_profile()`, `trade_disposition_profile()`, and `combat_profile()`, but not `travel_disposition_profile()` — confirmed.

## Architecture Check

1. The cleanest approach is to compute one effective switch margin at the controller/orchestration layer and pass that explicit `Permille` into every goal-switching consumer for this tick.
2. `select_best_plan()` and `evaluate_interrupt()` should both depend on explicit margin input, not on `PlanningBudget`, because they are making policy decisions, not choosing the policy source.
3. The effective margin should be derived once from `(runtime.has_active_journey(), view.travel_disposition_profile(agent), budget.switch_margin_permille)` so idle replanning and active-action interruption stay consistent.
4. Alternative: add an `Option<Permille>` override only to `select_best_plan()`. Rejected — this duplicates policy boundaries and leaves `evaluate_interrupt()` behind.
5. Alternative: pass `TravelDispositionProfile` directly into decision helpers. Rejected — low-level helpers should consume only the scalar policy value they need.
6. No backwards-compatibility aliasing or shims.

## What to Change

### 1. Add a controller-level effective margin helper

In `agent_tick.rs` or an adjacent controller-policy module, add:

```rust
fn effective_goal_switch_margin(
    view: &dyn BeliefView,
    agent: EntityId,
    runtime: &AgentDecisionRuntime,
    budget: &PlanningBudget,
) -> Permille
```

Behavior:
- If `runtime.has_active_journey()` is `true` and the agent has a `TravelDispositionProfile`, return `profile.route_replan_margin`.
- Otherwise return `budget.switch_margin_permille`.

### 2. Change `select_best_plan()` to take explicit `Permille`

```rust
pub fn select_best_plan(
    candidates: &[RankedGoal],
    plans: &[(GoalKey, Option<PlannedPlan>)],
    current: &AgentDecisionRuntime,
    switch_margin: Permille,
) -> Option<PlannedPlan>
```

Remove the `PlanningBudget` parameter entirely. `select_best_plan()` only needs the effective margin.

### 3. Change `evaluate_interrupt()` to take explicit `Permille`

```rust
pub fn evaluate_interrupt(
    runtime: &AgentDecisionRuntime,
    current_action_interruptibility: Interruptibility,
    ranked_candidates: &[RankedGoal],
    plan_valid: bool,
    switch_margin: Permille,
) -> InterruptDecision
```

Remove the `PlanningBudget` parameter entirely. `evaluate_interrupt()` uses the budget today only as an indirect source for switch margin.

### 4. Add `travel_disposition_profile()` to `BeliefView`

Add a trait accessor following the existing profile pattern:

```rust
fn travel_disposition_profile(&self, agent: EntityId) -> Option<TravelDispositionProfile>;
```

Implement it in `OmniscientBeliefView` to read from `world.get_travel_disposition_profile(agent)`.

### 5. Update `AgentTickDriver` call sites

In both controller paths:

- Compute `let switch_margin = effective_goal_switch_margin(&view, agent, runtime, budget);`
- Pass `switch_margin` to `select_best_plan()`
- Pass the same `switch_margin` to `evaluate_interrupt()`

This preserves a single source of truth for journey commitment policy.

### 6. Update existing tests

- `plan_selection.rs` tests should pass explicit `Permille` values.
- `interrupts.rs` tests should pass explicit `Permille` values.
- Add regression tests proving the same journey margin affects both idle replacement and freely interruptible same-class interruption.

## Files to Touch

- `crates/worldwake-ai/src/plan_selection.rs` (modify — accept explicit switch margin, remove `PlanningBudget` dependency)
- `crates/worldwake-ai/src/interrupts.rs` (modify — accept explicit switch margin, remove `PlanningBudget` dependency)
- `crates/worldwake-ai/src/agent_tick.rs` (modify — compute one effective switch margin and pass it to both decision paths)
- `crates/worldwake-sim/src/belief_view.rs` (modify — add trait method)
- `crates/worldwake-sim/src/omniscient_belief_view.rs` (modify — implement trait method)

## Out of Scope

- `compare_goal_switch()` signature change (ticket 003 — completed)
- Journey temporal field lifecycle (when to set/clear `journey_established_at`) (tickets 005, 006)
- `TravelDispositionProfile` definition (ticket 001)
- Debug surface (ticket 007)
- Changes to `worldwake-systems`
- Blocked-intent integration (ticket 006)

## Acceptance Criteria

### Tests That Must Pass

1. Existing `plan_selection.rs` regression tests still pass with explicit default margin input.
2. Existing `interrupts.rs` regression tests still pass with explicit default margin input.
3. New test: when the effective margin is `Permille(300)`, a same-class challenger with +15% motive improvement does NOT replace the current plan.
4. New test: when the effective margin is `Permille(300)`, a same-class challenger with +35% motive improvement DOES replace the current plan.
5. New test: when the effective margin is `Permille(300)`, a freely interruptible same-class challenger with +15% improvement does NOT trigger interruption.
6. New test: when the effective margin is `Permille(300)`, a freely interruptible same-class challenger with +35% improvement DOES trigger interruption.
7. New test: higher priority class still wins regardless of effective margin.
8. Existing suite: `cargo test -p worldwake-ai`
9. Existing suite: `cargo clippy --workspace`

### Invariants

1. Idle replanning and active-action interruption use the same effective switch margin policy.
2. Higher priority class always overrides margin (unchanged).
3. Same-goal replanning is unaffected by margin (same goal = always accept refresh).
4. Deterministic tie-breaking is unaffected.
5. No new cross-crate coupling beyond the existing `worldwake-ai` → `worldwake-sim` → `worldwake-core` dependency chain.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/plan_selection.rs` — update existing tests to pass explicit margin
2. `crates/worldwake-ai/src/interrupts.rs` — update existing tests to pass explicit margin
3. `crates/worldwake-ai/src/plan_selection.rs` — new test: `higher_effective_margin_raises_plan_switch_threshold`
4. `crates/worldwake-ai/src/interrupts.rs` — new test: `higher_effective_margin_raises_interrupt_switch_threshold`
5. `crates/worldwake-ai/src/agent_tick.rs` or dedicated controller test module — new test: active journey uses `route_replan_margin` consistently across both decision paths

### Commands

1. `cargo test -p worldwake-ai plan_selection`
2. `cargo test -p worldwake-ai interrupts`
3. `cargo test -p worldwake-ai agent_tick`
4. `cargo test -p worldwake-ai`
5. `cargo clippy --workspace`
