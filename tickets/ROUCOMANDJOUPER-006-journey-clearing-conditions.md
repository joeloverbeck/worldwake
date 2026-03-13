# ROUCOMANDJOUPER-006: Journey Clearing Conditions and Blocked-Intent Integration

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — journey clearing logic in agent_tick.rs and failure_handling.rs
**Deps**: ROUCOMANDJOUPER-001, ROUCOMANDJOUPER-002, ROUCOMANDJOUPER-005

## Problem

Journey temporal fields must be cleared under specific conditions. Without explicit clearing, agents could remain "committed" to unreachable destinations, dead agents could retain journey state, or agents could be stuck in infinite blocked-leg loops. Additionally, when the blocked-leg patience threshold is exceeded, the journey should be recorded in `BlockedIntentMemory` so the agent doesn't immediately re-commit to the same failed route.

## Assumption Reassessment (2026-03-13)

1. `AgentDecisionRuntime` has `journey_established_at`, `journey_last_progress_tick`, `consecutive_blocked_leg_ticks`, `has_active_journey()`, and `clear_journey_fields()` after ticket 002 — assumed complete.
2. `TravelDispositionProfile::blocked_leg_patience_ticks` is a `NonZeroU32` (ticket 001) — assumed complete.
3. `BlockedIntentMemory::record()` takes a `BlockedIntent` and replaces existing entries for the same goal — confirmed.
4. `BlockingFact` enum in `worldwake-core::blocked_intent` has existing variants like `NoKnownPath`, `TargetGone`, etc. — confirmed.
5. `handle_plan_failure()` in `failure_handling.rs` already clears `runtime.current_plan` and records blocked intents — confirmed.
6. Death is tracked via `DeadAt` component and checked via `view.is_dead()` — confirmed.
7. Plan replacement for any reason (goal switch, replan) goes through `select_best_plan()` which replaces `runtime.current_plan` — confirmed.

## Architecture Check

1. Journey clearing is distributed across existing control flow points: plan selection (goal switch), failure handling (blockage), and agent tick (death/incapacitation). This follows the existing pattern where runtime state management happens at the decision point, not in a separate cleanup pass.
2. The `BlockingFact` enum may need a new variant for journey-specific blockage (e.g., `LegRepeatedlyBlocked`) or can reuse `NoKnownPath`. The spec says to use existing blocked-intent infrastructure — evaluate whether existing variants suffice or a new one is needed.
3. No backwards-compatibility aliasing or shims.

## What to Change

### 1. Clear journey fields on plan replacement (goal switch)

In `agent_tick.rs`, when `select_best_plan()` returns a plan for a different goal than the current one, and the new plan is adopted:

```rust
if new_plan.goal != runtime.current_goal.unwrap_or(new_plan.goal) {
    runtime.clear_journey_fields();
}
```

This covers:
- "A higher-priority challenger beats the current commitment by the agent's `route_replan_margin`"
- "The plan is replaced for any reason (goal switch, replan)"

Note: same-goal replanning (refreshed plan for same goal) does NOT clear journey fields (ticket 005 already handles this).

### 2. Clear journey fields on blocked-leg patience exhaustion

In `agent_tick.rs`, after incrementing `consecutive_blocked_leg_ticks` (from ticket 005):

```rust
if let Some(profile) = view.travel_disposition_profile(agent) {
    if runtime.consecutive_blocked_leg_ticks >= profile.blocked_leg_patience_ticks.get() {
        // Record blocked intent
        if let Some(goal_key) = runtime.current_goal {
            blocked_memory.record(BlockedIntent {
                goal_key,
                blocking_fact: BlockingFact::NoKnownPath, // or new variant
                related_entity: None,
                related_place: derive_next_leg_target(runtime),
                observed_tick: current_tick,
                expires_tick: current_tick + u64::from(budget.structural_block_ticks),
            });
        }
        // Clear journey and plan
        runtime.clear_journey_fields();
        runtime.current_plan = None;
        runtime.materialization_bindings.clear();
        runtime.dirty = true;
    }
}
```

### 3. Clear journey fields on death/incapacitation

In `agent_tick.rs`, at the top of the per-agent tick (before any planning):

```rust
if view.is_dead(agent) || view.is_incapacitated(agent) {
    runtime.clear_journey_fields();
    // (existing dead-agent handling continues)
}
```

### 4. Clear journey fields on control loss

If the agent's `ControlSource` changes to `None` or is removed from the AI driver:

```rust
runtime.clear_journey_fields();
```

### 5. Clear journey fields when destination goal is satisfied

When `current_plan` completes (all steps executed, terminal kind is `GoalSatisfied`):

```rust
runtime.clear_journey_fields();
```

### 6. Integrate journey clearing into `handle_plan_failure()`

In `failure_handling.rs`, add journey field clearing when a plan fails:

```rust
pub fn handle_plan_failure(
    context: &PlanFailureContext<'_>,
    runtime: &mut AgentDecisionRuntime,
    blocked_memory: &mut BlockedIntentMemory,
    budget: &PlanningBudget,
) {
    runtime.current_plan = None;
    runtime.materialization_bindings.clear();
    runtime.clear_journey_fields(); // <-- ADD THIS
    // ... rest unchanged
}
```

### 7. Helper: derive next leg target

```rust
fn derive_next_leg_target(runtime: &AgentDecisionRuntime) -> Option<EntityId> {
    let plan = runtime.current_plan.as_ref()?;
    let step = plan.steps.get(runtime.current_step_index)?;
    if step.op_kind == PlannerOpKind::Travel {
        step.targets.first().copied().and_then(authoritative_target)
    } else {
        None
    }
}
```

## Files to Touch

- `crates/worldwake-ai/src/agent_tick.rs` (modify — add clearing hooks at goal switch, patience exhaustion, death, plan completion)
- `crates/worldwake-ai/src/failure_handling.rs` (modify — add `clear_journey_fields()` call)

## Out of Scope

- `TravelDispositionProfile` definition (ticket 001)
- Journey temporal field definitions (ticket 002)
- Goal switching margin override (tickets 003, 004)
- Journey field advancement on arrival (ticket 005 — that's the "set" side; this is the "clear" side)
- Debug surface (ticket 007)
- New `BlockingFact` variants beyond what's needed — reuse existing variants where possible
- Changes to `worldwake-core` (except potentially a new `BlockingFact` variant if `NoKnownPath` is insufficient)
- Changes to `worldwake-sim` or `worldwake-systems`

## Acceptance Criteria

### Tests That Must Pass

1. Goal switch to a different goal clears `journey_established_at`, `journey_last_progress_tick`, and `consecutive_blocked_leg_ticks`.
2. Same-goal replanning does NOT clear journey fields.
3. When `consecutive_blocked_leg_ticks >= blocked_leg_patience_ticks`, journey fields are cleared AND a `BlockedIntent` is recorded for the goal.
4. When `consecutive_blocked_leg_ticks < blocked_leg_patience_ticks`, journey fields are NOT cleared.
5. Death clears journey fields immediately.
6. Incapacitation clears journey fields immediately.
7. Plan completion (all steps done, goal satisfied) clears journey fields.
8. `handle_plan_failure()` clears journey fields along with the plan.
9. After patience-exhaustion clearing, the goal is blocked in `BlockedIntentMemory` with a concrete blocking fact and appropriate TTL.
10. Existing suite: `cargo test -p worldwake-ai`
11. Existing suite: `cargo clippy --workspace`

### Invariants

1. Journey fields are always cleared on death — no zombie journeys.
2. Journey fields are always cleared when the plan is cleared — no orphan journey state without a plan.
3. Blocked-intent recording on patience exhaustion uses existing infrastructure — no second cooldown table.
4. The clearing reason is deterministic and tied to concrete state (not heuristic).
5. No backwards-compatibility shims.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/agent_tick.rs` or dedicated test module — test: `goal_switch_clears_journey_fields`
2. Test: `same_goal_replan_preserves_journey_fields`
3. Test: `patience_exhaustion_clears_journey_and_records_blocked_intent`
4. Test: `patience_not_yet_exhausted_preserves_journey`
5. Test: `death_clears_journey_fields`
6. Test: `plan_completion_clears_journey_fields`
7. Test: `handle_plan_failure_clears_journey_fields`

### Commands

1. `cargo test -p worldwake-ai agent_tick`
2. `cargo test -p worldwake-ai failure_handling`
3. `cargo test -p worldwake-ai`
4. `cargo clippy --workspace`
