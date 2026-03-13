# ROUCOMANDJOUPER-006: Blocked-Leg Patience Exhaustion and Journey Commitment Clearing

**Status**: ✅ COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — blocked-leg patience handling in `agent_tick.rs`
**Deps**: ROUCOMANDJOUPER-001, ROUCOMANDJOUPER-002, ROUCOMANDJOUPER-004, ROUCOMANDJOUPER-005, ROUCOMANDJOUPER-008, ROUCOMANDJOUPER-009

## Problem

Most journey-clearing behavior that this ticket originally proposed already exists after tickets 004, 005, 008, and 009:

- death cleanup already clears journey commitment in `process_agent()`
- generic plan failure already clears commitment in `handle_plan_failure()`
- committed-journey completion versus suspended-detour completion is already distinguished in `advance_completed_step()`
- plan adoption already distinguishes commitment refresh, suspension, and abandonment through `JourneyPlanRelation`

The remaining concrete gap is narrower: recoverable blocked travel legs only increment `consecutive_blocked_leg_ticks`, drop the stale concrete plan, and force replanning. They never consult `TravelDispositionProfile::blocked_leg_patience_ticks`, never clear the durable journey commitment when patience is exhausted, and never record a concrete blocker in `BlockedIntentMemory`. That leaves a committed journey able to thrash indefinitely on a locally impossible next leg.

## Assumption Reassessment (2026-03-13)

1. `AgentDecisionRuntime` now has a durable commitment anchor (`journey_committed_goal`, `journey_committed_destination`) plus `journey_commitment_state: Active | Suspended` after tickets 008 and 009 — confirmed.
2. `TravelDispositionProfile::blocked_leg_patience_ticks` exists as a `NonZeroU32` authoritative component on agents — confirmed.
3. `BlockedIntentMemory::record()` replaces any existing entry for the same `goal_key`, so patience exhaustion can reuse the existing blocker table without aliasing or a second cooldown mechanism — confirmed.
4. `handle_plan_failure()` already clears `current_plan`, clears journey commitment, clears materialization bindings, records a blocker, and marks the runtime dirty — confirmed. This ticket must not duplicate that generic failure path.
5. Death cleanup already happens at the top of `process_agent()`, where dead or non-alive agents have journey commitment, plan state, and bindings cleared before the tick exits — confirmed.
6. `advance_completed_step()` already preserves commitment for suspended-detour completion and clears it for true committed-journey completion — confirmed.
7. `update_journey_fields_for_adopted_plan()` already handles same-commitment refresh, suspension for local detours, and commitment replacement for abandonment — confirmed.
8. `handle_recoverable_travel_step_blockage()` currently increments `consecutive_blocked_leg_ticks`, drops `current_plan`, resets `current_step_index`, clears materialization bindings, and marks `dirty`, but it does not inspect `blocked_leg_patience_ticks` or record any blocker — confirmed. This is the actual missing behavior.

## Architecture Check

1. The clean architecture is to keep broad journey-clearing semantics where they already live today: death cleanup in `process_agent()`, generic failures in `handle_plan_failure()`, plan-relation handling in `update_journey_fields_for_adopted_plan()`, and terminal completion in `advance_completed_step()`. Re-implementing those clear paths in this ticket would be duplication, not hardening.
2. Patience exhaustion belongs on the recoverable blocked-leg seam itself, because that seam owns the counter and already distinguishes "temporarily blocked next leg" from generic plan failure.
3. Blocked-intent integration should continue using the existing `BlockedIntentMemory` infrastructure. No second cooldown table, no compatibility aliasing.
4. `BlockingFact::NoKnownPath` is precise enough for the current concrete symptom. A new enum variant for "repeated blockage" would be a weaker abstraction unless the controller starts distinguishing multiple concrete route blockers later.
5. The implementation should keep route state plan-derived. The only durable state to clear on exhaustion is the existing journey commitment anchor plus temporal counters, not a new route cache or alias layer.

## What to Change

### 1. Trigger commitment clearing only on blocked-leg patience exhaustion

Update the recoverable blocked-travel path in `agent_tick.rs` so it consults the actor's `TravelDispositionProfile` after incrementing `consecutive_blocked_leg_ticks`.

```rust
runtime.consecutive_blocked_leg_ticks += 1;

if runtime.consecutive_blocked_leg_ticks >= profile.blocked_leg_patience_ticks.get() {
    // record blocker, clear commitment, drop plan continuity
}
```

Behavioral requirements:
- below threshold: preserve the durable commitment anchor and keep the current recoverable replan behavior
- at threshold: record a blocker, clear the durable commitment anchor, and leave the runtime dirty for replanning away from the failed journey
- do not route patience exhaustion through `handle_plan_failure()` because this is not a generic step-failure path and does not need its broader blocking-fact derivation logic

### 2. Record a structural blocker on exhaustion

```rust
blocked_memory.record(BlockedIntent {
    goal_key,
    blocking_fact: BlockingFact::NoKnownPath,
    related_entity: None,
    related_place: next_leg_place,
    observed_tick: current_tick,
    expires_tick: current_tick + u64::from(budget.structural_block_ticks),
});
```

Use:
- `goal_key`: the committed/current goal being abandoned
- `related_place`: the authoritative target of the blocked travel step if it exists
- `blocking_fact`: `BlockingFact::NoKnownPath`

This is intentionally concrete and reuses the existing structural blocker TTL.

### 3. Keep all other clearing paths unchanged

Do not add new clearing code to:
- `handle_plan_failure()` in `failure_handling.rs`
- death cleanup in `process_agent()`
- plan adoption in `update_journey_fields_for_adopted_plan()`
- terminal completion in `advance_completed_step()`

Those paths already express the intended architecture and already have tests. This ticket should strengthen the missing blocked-leg exhaustion seam instead of reopening already-landed lifecycle code.

### 4. Add a small helper only if it reduces duplication

If the blocked-leg seam needs a helper, keep it local to `agent_tick.rs` and narrowly scoped, for example:

```rust
fn blocked_leg_target(step: &PlannedStep) -> Option<EntityId> {
    step.targets.first().copied().and_then(authoritative_target)
}
```

Do not introduce a generalized journey-clearing abstraction or a second failure subsystem.

## Files to Touch

- `crates/worldwake-ai/src/agent_tick.rs` (modify — add blocked-leg patience exhaustion + blocker recording)

## Out of Scope

- `TravelDispositionProfile` definition (ticket 001)
- Journey temporal field definitions (ticket 002)
- Goal switching margin policy implementation details (tickets 003, 004)
- Journey field advancement on arrival except for the patience threshold handoff from ticket 005
- Debug surface (ticket 007)
- New `BlockingFact` variants — reuse `NoKnownPath`
- Changes to `worldwake-core`
- Changes to `failure_handling.rs`
- Changes to `worldwake-sim` or `worldwake-systems`

## Acceptance Criteria

### Tests That Must Pass

1. When a recoverable blocked travel leg leaves `consecutive_blocked_leg_ticks < blocked_leg_patience_ticks`, the runtime keeps the journey commitment anchor intact, drops the stale concrete plan, and marks the runtime dirty for replanning.
2. When a recoverable blocked travel leg increments `consecutive_blocked_leg_ticks` to `blocked_leg_patience_ticks`, the runtime clears the journey commitment anchor and temporal fields.
3. Patience exhaustion records a `BlockedIntent` for the abandoned goal in `BlockedIntentMemory`.
4. The exhaustion blocker uses `BlockingFact::NoKnownPath`.
5. The exhaustion blocker uses `budget.structural_block_ticks` as its TTL.
6. The blocker records the blocked next-leg place when the travel step has an authoritative target.
7. Existing already-landed behaviors remain true:
   - death clears commitment immediately
   - generic plan failure clears commitment
   - same-commitment refresh preserves journey state
   - suspended-detour completion preserves commitment while true journey completion clears it
8. Existing suite: `cargo test -p worldwake-ai`
9. Existing suite: `cargo clippy --workspace --all-targets -- -D warnings`

### Invariants

1. Journey commitment is cleared on patience exhaustion only at the recoverable blocked-leg seam, not by a speculative cleanup pass.
2. Below-threshold blocked legs remain recoverable and do not erase the durable commitment anchor.
3. Blocked-intent recording on patience exhaustion reuses existing infrastructure — no second cooldown table.
4. The exhaustion reason is deterministic and tied to concrete state: repeated failure to start the current travel leg.
5. No route cache, compatibility shim, or alias layer is introduced.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/agent_tick.rs` — strengthen `recoverable_blocked_travel_step_increments_consecutive_blocked_ticks_and_forces_replan` or replace it with a below-threshold equivalent that asserts commitment is preserved and no blocker is recorded.
2. `crates/worldwake-ai/src/agent_tick.rs` — new test: `blocked_leg_patience_exhaustion_clears_commitment_and_records_blocker`
3. `crates/worldwake-ai/src/failure_handling.rs` — no new behavior required, but keep the existing failure-clearing regression passing as an architecture guard.

### Commands

1. `cargo test -p worldwake-ai agent_tick`
2. `cargo test -p worldwake-ai failure_handling`
3. `cargo test -p worldwake-ai`
4. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

- Completion date: 2026-03-13
- What actually changed:
  - Narrowed the ticket before implementation because the broader journey-clearing work had already landed in `agent_tick.rs` and `failure_handling.rs`.
  - Added blocked-leg patience exhaustion handling to the recoverable travel blockage seam in `crates/worldwake-ai/src/agent_tick.rs`.
  - When the blocked-leg counter reaches `TravelDispositionProfile::blocked_leg_patience_ticks`, the runtime now records `BlockingFact::NoKnownPath`, uses `budget.structural_block_ticks` for TTL, clears the durable journey commitment, and drops the stale plan for replanning.
  - Below the threshold, recoverable blocked travel still preserves the journey commitment anchor and only forces a replan.
- Deviations from original plan:
  - No changes were made to `failure_handling.rs`, death cleanup, plan adoption, or terminal completion because those paths were already correct in the current architecture.
  - No new `BlockingFact` variant or generalized clearing abstraction was introduced; `BlockingFact::NoKnownPath` was precise enough for repeated blocked-leg exhaustion.
- Verification results:
  - `cargo test -p worldwake-ai agent_tick` ✅
  - `cargo test -p worldwake-ai failure_handling` ✅
  - `cargo test -p worldwake-ai` ✅
  - `cargo clippy --workspace --all-targets -- -D warnings` ✅
