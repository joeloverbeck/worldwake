# ROUCOMANDJOUPER-005: Journey Field Advancement on Arrival and Blockage Tracking

**Status**: ✅ COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — journey lifecycle hooks in agent_tick.rs
**Deps**: ROUCOMANDJOUPER-002, ROUCOMANDJOUPER-004

## Problem

The journey temporal fields added in ticket 002 exist on `AgentDecisionRuntime` but are never set or advanced. This ticket implements the establishment and advancement lifecycle:
- Setting `journey_established_at` when selecting a multi-hop travel-led plan.
- Updating `journey_last_progress_tick` and resetting `consecutive_blocked_leg_ticks` on successful leg completion.
- Incrementing `consecutive_blocked_leg_ticks` when the next leg cannot start.

## Assumption Reassessment (2026-03-13)

1. `AgentDecisionRuntime` already has `journey_established_at`, `journey_last_progress_tick`, `consecutive_blocked_leg_ticks`, `has_active_journey()`, `remaining_travel_steps()`, and `clear_journey_fields()` after ticket 002 — confirmed.
2. `PlannedPlan` already exposes plan-level travel helpers (`remaining_travel_steps_from`, `has_remaining_travel_steps_from`, `terminal_travel_destination`) after the ticket-002 follow-up refinement — confirmed.
3. `AgentTickDriver` manages per-agent `AgentDecisionRuntime` instances in `runtime_by_agent: BTreeMap<EntityId, AgentDecisionRuntime>` and already computes a controller-level effective journey switch margin after ticket 004 — confirmed.
4. Plan adoption happens in `agent_tick.rs` inside `plan_and_validate_next_step()`, where `select_best_plan()` returns a `PlannedPlan` and the driver assigns it to `runtime.current_plan` — confirmed.
5. Successful step completion happens in `reconcile_in_flight_state()`, which clears `step_in_flight`, applies any materialization bindings, and then calls `advance_completed_step()` — confirmed.
6. The current controller treats a step that cannot validate or resolve targets this tick as a full plan failure via `handle_current_step_failure()`. That means blocked-leg tracking cannot be implemented as a passive counter layered on top of the existing failure path; travel-step start blockage needs an explicit recoverable branch in `agent_tick.rs` — confirmed.
7. `PlannedStep::op_kind` still identifies Travel steps via `PlannerOpKind::Travel` — confirmed.

## Architecture Check

1. Journey establishment and advancement belong in `agent_tick.rs` because that is where plan selection results are applied, action completion is processed, and step-start validation decisions are made. No new module is needed.
2. No backwards-compatibility aliasing or shims.
3. The blockage counter increment should happen only for recoverable failure to start the current Travel step this tick. Generic plan failure is the wrong seam because it tears down `current_plan`, clears materialization continuity, and records a blocked intent immediately; that architecture would make `consecutive_blocked_leg_ticks` meaningless. This ticket should instead add a dedicated travel-step blockage path that preserves the current commitment for later tickets to clear deliberately.
4. Route-inspection helpers should continue to live on `PlannedPlan`, not as ad hoc free functions in `agent_tick.rs`. `AgentDecisionRuntime` should delegate to plan helpers, but plan-derived facts such as "remaining Travel steps from index N" belong with the plan data structure.
5. Same-goal replanning should preserve `journey_established_at`. Re-establishing a journey on every refreshed same-destination plan would turn commitment lifetime into a plan-allocation artifact instead of a semantic lifecycle signal.

## What to Change

### 1. Set `journey_established_at` on travel-led plan adoption

After `select_best_plan()` returns a new plan and it is assigned to `runtime.current_plan`:

```rust
// After plan assignment:
if new_plan.has_remaining_travel_steps_from(0) {
    if runtime.journey_established_at.is_none() {
        runtime.journey_established_at = Some(current_tick);
        runtime.consecutive_blocked_leg_ticks = 0;
    }
} else {
    runtime.clear_journey_fields();
}
```

Key: only set `journey_established_at` if it's not already set (preserves existing journey across same-goal replanning). Leave `journey_last_progress_tick` unchanged on adoption; it represents completed-leg progress, not commitment establishment. If the new plan has no Travel steps, clear journey state.

### 2. Advance journey fields on successful leg completion

When a Travel step completes (action finishes, `current_step_index` advances past a `PlannerOpKind::Travel` step):

```rust
runtime.journey_last_progress_tick = Some(current_tick);
runtime.consecutive_blocked_leg_ticks = 0;
```

This hook belongs in the step advancement logic of `agent_tick.rs`, after confirming the completed step was a Travel step.

### 3. Increment blockage counter on recoverable failed leg start

When the agent has an active journey and the current step is Travel but the action cannot be started this tick (precondition failure, affordance mismatch, target resolution failure, etc.), do not go through the generic `handle_current_step_failure()` path. Instead:

```rust
runtime.consecutive_blocked_leg_ticks += 1;
runtime.dirty = true;
```

Behavioral requirements:
- Preserve `runtime.current_goal` and the journey temporal fields.
- Drop the stale `current_plan` and reset `current_step_index`/materialization bindings so the next tick replans instead of retrying an already-invalid concrete route step forever.
- Do not record a `BlockedIntent` yet. Patience exhaustion and blocked-intent integration belong to ticket 006.
- Do not clear journey fields here. This ticket is the "set/advance/track" side of the lifecycle.

This keeps travel blockage as an explicit controller lifecycle state instead of collapsing it into generic plan failure, while still avoiding stale-plan livelocks.

### 4. Reuse the existing `PlannedPlan` route-inspection helpers

Use the plan-level helpers instead of introducing a new local helper:

```rust
plan.has_remaining_travel_steps_from(from_index)
plan.remaining_travel_steps_from(from_index)
```

## Files to Touch

- `crates/worldwake-ai/src/agent_tick.rs` (modify — add journey establishment, travel-step advancement, and recoverable blockage hooks)
- `crates/worldwake-ai/src/planner_ops.rs` (read existing plan-level route-inspection helpers; extend only if lifecycle work needs an additional plan-derived query)

## Out of Scope

- `TravelDispositionProfile` component shape (ticket 001 — already landed, not modified here)
- Goal switching margin policy changes (tickets 003, 004 — already landed, not modified here)
- Journey clearing conditions (ticket 006 — this ticket only handles the "set" and "advance" side)
- When to clear journey fields on goal switch, death, etc. (ticket 006)
- Debug surface (ticket 007)
- Blocked-intent integration on patience exhaustion (ticket 006)
- Changes to `worldwake-core`, `worldwake-sim`, or `worldwake-systems`

## Acceptance Criteria

### Tests That Must Pass

1. Adopting a plan whose remaining steps include at least one Travel op sets `journey_established_at` to the current tick.
2. Adopting a plan with 0 remaining Travel steps leaves `journey_established_at` as `None`.
3. Same-goal replanning with another travel-led plan does NOT reset `journey_established_at` if it was already set.
4. Completing a Travel step updates `journey_last_progress_tick` to the completion tick.
5. Completing a Travel step resets `consecutive_blocked_leg_ticks` to 0.
6. When the current Travel step cannot start through the recoverable controller path, `consecutive_blocked_leg_ticks` increments by 1 for that tick, the stale plan is dropped for replanning, and the journey temporal state remains intact.
7. Recoverable blocked-leg tracking leaves the current commitment intact for ticket 006 to clear later on patience exhaustion.
8. Existing suite: `cargo test -p worldwake-ai`
9. Existing suite: `cargo clippy --workspace --all-targets -- -D warnings`

### Invariants

1. Journey fields are only set on `AgentDecisionRuntime`, never stored as authoritative world state.
2. Plan adoption with non-Travel plans clears journey fields.
3. The authoritative travel model (adjacent-place, per-leg) is not modified.
4. Determinism is preserved — all journey field updates are driven by deterministic tick/plan/step state.
5. Recoverable blocked-leg tracking does not introduce a second failure-memory mechanism or route cache.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/agent_tick.rs` — test: `travel_led_plan_adoption_sets_journey_established_at`
2. Test: `non_travel_plan_adoption_clears_journey_fields`
3. Test: `same_goal_replan_preserves_journey_established_at`
4. Test: `travel_leg_completion_updates_progress_tick_and_resets_blocked_counter`
5. Test: `recoverable_blocked_travel_step_increments_consecutive_blocked_ticks_and_forces_replan`

### Commands

1. `cargo test -p worldwake-ai agent_tick`
2. `cargo test -p worldwake-ai`
3. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

- Completion date: 2026-03-13
- What actually changed:
  - Added journey-establishment updates when a travel-led plan is adopted in `crates/worldwake-ai/src/agent_tick.rs`.
  - Added travel-leg completion updates so successful Travel steps stamp `journey_last_progress_tick` and reset `consecutive_blocked_leg_ticks`.
  - Added a recoverable blocked-travel-step path that increments `consecutive_blocked_leg_ticks`, preserves the journey commitment fields, and drops the stale concrete plan so the next tick replans instead of livelocking on an invalid route step.
  - Added targeted `agent_tick` unit coverage for travel-led adoption, same-goal replan preservation, non-travel clearing, travel-leg progress updates, and recoverable blockage handling.
  - Updated the golden multi-leg reprioritization scenario to make low-commitment travel disposition explicit now that journey state is actually established during travel.
- Deviations from original plan:
  - The original ticket assumed blocked-leg tracking could sit on top of the existing generic plan-failure path. That was incorrect in the current architecture because generic failure tears down the plan immediately. The ticket was corrected before implementation so blocked travel-step starts take a dedicated recoverable controller path instead.
  - The original draft also implied setting `journey_last_progress_tick` on plan adoption. The implemented version keeps that field reserved for completed-leg progress only.
  - Recoverable blockage now drops the stale concrete plan while preserving the journey lifecycle state. This was a deliberate refinement discovered during verification to avoid stale-route livelocks at intermediate places.
- Verification results:
  - `cargo test -p worldwake-ai agent_tick` ✅
  - `cargo test -p worldwake-ai --test golden_ai_decisions golden_goal_switching_during_multi_leg_travel -- --nocapture` ✅
  - `cargo test -p worldwake-ai` ✅
  - `cargo clippy --workspace --all-targets -- -D warnings` ✅
