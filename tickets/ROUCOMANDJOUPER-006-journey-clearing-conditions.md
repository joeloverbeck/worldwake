# ROUCOMANDJOUPER-006: Journey Clearing Conditions and Blocked-Intent Integration

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — journey clearing logic in `agent_tick.rs` and `failure_handling.rs`
**Deps**: ROUCOMANDJOUPER-001, ROUCOMANDJOUPER-002, ROUCOMANDJOUPER-004, ROUCOMANDJOUPER-005

## Problem

Journey temporal fields must be cleared under specific conditions. Without explicit clearing, agents could remain committed to unreachable destinations, dead agents could retain journey state, or agents could be stuck in infinite blocked-leg loops. Additionally, when the blocked-leg patience threshold is exceeded, the journey should be recorded in `BlockedIntentMemory` so the agent does not immediately re-commit to the same failed route.

After the controller-policy cleanup in ticket 004, reprioritization can originate from both idle plan replacement and active-action interruption, so journey clearing needs to stay consistent across both paths.

## Assumption Reassessment (2026-03-13)

1. `AgentDecisionRuntime` has `journey_established_at`, `journey_last_progress_tick`, `consecutive_blocked_leg_ticks`, `has_active_journey()`, and `clear_journey_fields()` after ticket 002 — assumed complete.
2. `TravelDispositionProfile::blocked_leg_patience_ticks` is a `NonZeroU32` (ticket 001) — assumed complete.
3. `BlockedIntentMemory::record()` takes a `BlockedIntent` and replaces existing entries for the same goal — confirmed.
4. `BlockingFact` in `worldwake-core::blocked_intent` already has concrete route-failure-ish variants like `NoKnownPath`, `TargetGone`, and related blockers — confirmed.
5. `handle_plan_failure()` in `failure_handling.rs` already clears `runtime.current_plan` and records blocked intents — confirmed.
6. Death is tracked via `DeadAt` and checked via `view.is_dead()` — confirmed.
7. Reprioritization can originate from both idle plan replacement (`select_best_plan()`) and active-action interruption (`evaluate_interrupt()`), but both paths are orchestrated in `agent_tick.rs` — confirmed.
8. After ticket 004, controller-level switch-margin policy is computed outside low-level decision helpers and shared across both paths — this ticket should rely on that shared boundary rather than duplicating path-specific clearing logic.

## Architecture Check

1. Journey clearing belongs in existing control-flow points: commitment replacement, failure handling, patience exhaustion, terminal completion, and death cleanup. A separate cleanup pass would be less explicit and harder to reason about.
2. Clearing semantics should not care whether reprioritization came from idle selection or active interruption. If the current commitment is replaced, journey lifecycle should clear through shared controller code rather than duplicated path-specific rules.
3. Blocked-intent integration should continue using the existing `BlockedIntentMemory` infrastructure. No second cooldown table.
4. The `BlockingFact` enum may need a new variant for repeated-leg blockage, but only if existing concrete variants are not precise enough.
5. No backwards-compatibility aliasing or shims.

## What to Change

### 1. Clear journey fields on commitment replacement

In `agent_tick.rs`, when the controller replaces the current commitment with a different-goal commitment, clear journey state through shared orchestration code:

```rust
if new_plan.goal != runtime.current_goal.unwrap_or(new_plan.goal) {
    runtime.clear_journey_fields();
}
```

This covers:
- "A higher-priority challenger beats the current commitment by the agent's `route_replan_margin`"
- "The plan is replaced for any reason (goal switch, replan)"

This logic should be reached regardless of whether the challenger won during idle plan selection or after an interrupt-driven replan.

### 2. Clear journey fields on blocked-leg patience exhaustion

In `agent_tick.rs`, after incrementing `consecutive_blocked_leg_ticks` (from ticket 005):

```rust
if let Some(profile) = view.travel_disposition_profile(agent) {
    if runtime.consecutive_blocked_leg_ticks >= profile.blocked_leg_patience_ticks.get() {
        if let Some(goal_key) = runtime.current_goal {
            blocked_memory.record(BlockedIntent {
                goal_key,
                blocking_fact: BlockingFact::NoKnownPath, // or a more precise existing/new variant
                related_entity: None,
                related_place: derive_next_leg_target(runtime),
                observed_tick: current_tick,
                expires_tick: current_tick + u64::from(budget.structural_block_ticks),
            });
        }
        runtime.clear_journey_fields();
        runtime.current_plan = None;
        runtime.materialization_bindings.clear();
        runtime.dirty = true;
    }
}
```

### 3. Clear journey fields on death and other concrete terminal invalidations

In `agent_tick.rs`, at the top of the per-agent tick, clear journey state when the controller can already observe a concrete terminal invalidation such as death:

```rust
if view.is_dead(agent) {
    runtime.clear_journey_fields();
    // existing dead-agent handling continues
}
```

Only add incapacity/control-loss branches if the current controller runtime actually retains stale journey state across those transitions. Do not add speculative cleanup paths that current code cannot trigger or observe concretely.

### 4. Clear journey fields when destination goal is satisfied

When `current_plan` completes and the terminal outcome abandons the journey commitment:

```rust
runtime.clear_journey_fields();
```

This should happen alongside the existing plan cleanup in step advancement / terminal handling.

### 5. Integrate journey clearing into `handle_plan_failure()`

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
    runtime.clear_journey_fields();
    // ... rest unchanged
}
```

### 6. Helper: derive next leg target

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

- `crates/worldwake-ai/src/agent_tick.rs` (modify — add clearing hooks at commitment replacement, patience exhaustion, death, and plan completion)
- `crates/worldwake-ai/src/failure_handling.rs` (modify — add `clear_journey_fields()` call)

## Out of Scope

- `TravelDispositionProfile` definition (ticket 001)
- Journey temporal field definitions (ticket 002)
- Goal switching margin policy implementation details (tickets 003, 004)
- Journey field advancement on arrival (ticket 005 — that is the "set/advance" side; this is the "clear" side)
- Debug surface (ticket 007)
- New `BlockingFact` variants beyond what is necessary — reuse existing variants where possible
- Changes to `worldwake-core` unless a more precise `BlockingFact` is genuinely required
- Changes to `worldwake-sim` or `worldwake-systems`

## Acceptance Criteria

### Tests That Must Pass

1. Reprioritization to a different goal clears `journey_established_at`, `journey_last_progress_tick`, and `consecutive_blocked_leg_ticks`, regardless of whether the change originated from idle selection or interrupt-driven replanning.
2. Same-goal replanning does NOT clear journey fields.
3. When `consecutive_blocked_leg_ticks >= blocked_leg_patience_ticks`, journey fields are cleared AND a `BlockedIntent` is recorded for the goal.
4. When `consecutive_blocked_leg_ticks < blocked_leg_patience_ticks`, journey fields are NOT cleared.
5. Death clears journey fields immediately.
6. Plan completion (all steps done, goal satisfied) clears journey fields.
7. `handle_plan_failure()` clears journey fields along with the plan.
8. After patience-exhaustion clearing, the goal is blocked in `BlockedIntentMemory` with a concrete blocking fact and appropriate TTL.
9. Existing suite: `cargo test -p worldwake-ai`
10. Existing suite: `cargo clippy --workspace`

### Invariants

1. Journey fields are always cleared on death — no zombie journeys.
2. Journey fields are always cleared when the current commitment is abandoned or the plan is cleared — no orphan journey state without a plan.
3. Blocked-intent recording on patience exhaustion uses existing infrastructure — no second cooldown table.
4. Reprioritization-triggered clearing does not diverge between idle and active-action paths.
5. The clearing reason is deterministic and tied to concrete state, not heuristic.
6. No backwards-compatibility shims.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/agent_tick.rs` or dedicated test module — test: `idle_goal_switch_clears_journey_fields`
2. Test: `interrupt_driven_reprioritization_clears_journey_fields_after_replan`
3. Test: `same_goal_replan_preserves_journey_fields`
4. Test: `patience_exhaustion_clears_journey_and_records_blocked_intent`
5. Test: `patience_not_yet_exhausted_preserves_journey`
6. Test: `death_clears_journey_fields`
7. Test: `plan_completion_clears_journey_fields`
8. Test: `handle_plan_failure_clears_journey_fields`

### Commands

1. `cargo test -p worldwake-ai agent_tick`
2. `cargo test -p worldwake-ai failure_handling`
3. `cargo test -p worldwake-ai`
4. `cargo clippy --workspace`
