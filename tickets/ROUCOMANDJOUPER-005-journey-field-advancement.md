# ROUCOMANDJOUPER-005: Journey Field Advancement on Arrival and Blockage Tracking

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — journey lifecycle hooks in agent_tick.rs
**Deps**: ROUCOMANDJOUPER-002

## Problem

The journey temporal fields added in ticket 002 exist on `AgentDecisionRuntime` but are never set or advanced. This ticket implements the establishment and advancement lifecycle:
- Setting `journey_established_at` when selecting a multi-hop travel-led plan.
- Updating `journey_last_progress_tick` and resetting `consecutive_blocked_leg_ticks` on successful leg completion.
- Incrementing `consecutive_blocked_leg_ticks` when the next leg cannot start.

## Assumption Reassessment (2026-03-13)

1. `AgentDecisionRuntime` has `journey_established_at`, `journey_last_progress_tick`, `consecutive_blocked_leg_ticks`, and `has_active_journey()` after ticket 002 — assumed complete.
2. `AgentTickDriver` manages per-agent `AgentDecisionRuntime` instances in `runtime_by_agent: BTreeMap<EntityId, AgentDecisionRuntime>` — confirmed.
3. Plan selection happens in `agent_tick.rs` where the driver calls `select_best_plan()` and assigns the result to `runtime.current_plan` — confirmed by reading the file.
4. Travel leg completion is detected when an action with `PlannerOpKind::Travel` completes and the agent's `current_step_index` advances — confirmed by reading agent tick flow.
5. The `PlannedStep::op_kind` field identifies Travel steps via `PlannerOpKind::Travel` — confirmed.

## Architecture Check

1. Journey establishment and advancement belong in `agent_tick.rs` because that's where plan selection results are applied and action completion is processed. No new module needed.
2. No backwards-compatibility aliasing or shims.
3. The blockage counter increment should happen when the agent has an active journey but cannot start the next Travel step. The exact detection mechanism depends on how `agent_tick.rs` handles "step not started this tick" — this needs careful integration with the existing replan/retry flow.

## What to Change

### 1. Set `journey_established_at` on plan adoption

After `select_best_plan()` returns a new plan and it is assigned to `runtime.current_plan`:

```rust
// After plan assignment:
if plan_has_remaining_travel_steps(&new_plan, 0) {
    if runtime.journey_established_at.is_none() {
        runtime.journey_established_at = Some(current_tick);
        runtime.journey_last_progress_tick = Some(current_tick);
        runtime.consecutive_blocked_leg_ticks = 0;
    }
} else {
    runtime.clear_journey_fields();
}
```

Key: only set `journey_established_at` if it's not already set (preserves existing journey across same-goal replanning). If the new plan has no Travel steps, clear journey state.

### 2. Advance journey fields on successful leg completion

When a Travel step completes (action finishes, `current_step_index` advances past a `PlannerOpKind::Travel` step):

```rust
runtime.journey_last_progress_tick = Some(current_tick);
runtime.consecutive_blocked_leg_ticks = 0;
```

This hook belongs in the step advancement logic of `agent_tick.rs`, after confirming the completed step was a Travel step.

### 3. Increment blockage counter on failed leg start

When the agent has an active journey and the next step is Travel but the action cannot be started this tick (precondition failure, affordance mismatch, etc.):

```rust
runtime.consecutive_blocked_leg_ticks += 1;
```

### 4. Add helper: `plan_has_remaining_travel_steps`

A free function or method on `PlannedPlan`:

```rust
fn plan_has_remaining_travel_steps(plan: &PlannedPlan, from_index: usize) -> bool {
    plan.steps[from_index..]
        .iter()
        .any(|step| step.op_kind == PlannerOpKind::Travel)
}
```

## Files to Touch

- `crates/worldwake-ai/src/agent_tick.rs` (modify — add journey establishment, advancement, and blockage hooks)
- `crates/worldwake-ai/src/decision_runtime.rs` (modify — add `plan_has_remaining_travel_steps` helper if placed here)

## Out of Scope

- `TravelDispositionProfile` component (ticket 001)
- Goal switching margin override (tickets 003, 004)
- Journey clearing conditions (ticket 006 — this ticket only handles the "set" and "advance" side)
- When to clear journey fields on goal switch, death, etc. (ticket 006)
- Debug surface (ticket 007)
- Blocked-intent integration on patience exhaustion (ticket 006)
- Changes to `worldwake-core`, `worldwake-sim`, or `worldwake-systems`

## Acceptance Criteria

### Tests That Must Pass

1. Adopting a plan with 2+ Travel steps sets `journey_established_at` to the current tick.
2. Adopting a plan with 0 Travel steps leaves `journey_established_at` as `None`.
3. Adopting a plan with 1 Travel step (single-leg, not multi-hop) sets `journey_established_at` only if the step is not the final goal-satisfying step (i.e., there are more steps after it). Alternatively, any plan with Travel steps counts — spec says "current plan has remaining Travel steps." Clarify: the spec says `journey_established_at` is set when selecting a "travel-led plan" — a plan whose remaining steps include Travel ops.
4. Re-adopting the same goal with a refreshed plan does NOT reset `journey_established_at` if it was already set.
5. Completing a Travel step updates `journey_last_progress_tick` to the completion tick.
6. Completing a Travel step resets `consecutive_blocked_leg_ticks` to 0.
7. When the next Travel step cannot start, `consecutive_blocked_leg_ticks` increments by 1 per tick.
8. Existing suite: `cargo test -p worldwake-ai`
9. Existing suite: `cargo clippy --workspace`

### Invariants

1. Journey fields are only set on `AgentDecisionRuntime`, never stored as authoritative world state.
2. Plan adoption with non-Travel plans clears journey fields.
3. The authoritative travel model (adjacent-place, per-leg) is not modified.
4. Determinism is preserved — all journey field updates are driven by deterministic tick/plan/step state.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/agent_tick.rs` or a new test module — test: `multi_hop_plan_adoption_sets_journey_established_at`
2. Test: `non_travel_plan_adoption_clears_journey_fields`
3. Test: `same_goal_replan_preserves_journey_established_at`
4. Test: `travel_leg_completion_updates_progress_tick_and_resets_blocked_counter`
5. Test: `blocked_next_leg_increments_consecutive_blocked_ticks`

### Commands

1. `cargo test -p worldwake-ai agent_tick`
2. `cargo test -p worldwake-ai`
3. `cargo clippy --workspace`
