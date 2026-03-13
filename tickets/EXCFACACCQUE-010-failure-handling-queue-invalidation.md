# EXCFACACCQUE-010 — Failure Handling: Queue Invalidation as Explicit Blocker

**Spec sections**: §8, §10 (failure handling)
**Crates**: `worldwake-ai`

## Summary

Update AI failure handling so that queue-head invalidation (from `QueueHeadFailed` events) becomes an explicit blocker in the agent's `BlockedIntentMemory`, replacing the old pattern of repeated reservation collisions for exclusive facilities.

## Deliverables

### 1. Recognize queue failure events

In `crates/worldwake-ai/src/failure_handling.rs`:
- When processing `QueueHeadFailed` events (emitted by `facility_queue_system`), create a `BlockedIntent` entry for the specific `(facility, ActionDefId)` combination
- The blocked intent should carry a meaningful barrier description (e.g., "facility lost required workstation" or "facility destroyed")
- Set appropriate `blocked_until` expiration based on the permanence of the failure

### 2. Replace reservation-collision pattern

For exclusive facility actions, the old failure path was:
1. Agent tries to start harvest → reservation fails → plan fails → repeated replanning

The new failure path is:
1. Agent joins queue → waits for grant → gets grant → starts harvest normally
2. OR: Queue head fails permanently → `QueueHeadFailed` event → blocked intent created → agent stops targeting this facility

Update `handle_plan_failure()` to recognize that exclusive-facility plan failures should reference queue/grant state, not reservation collisions.

### 3. Grant expiry as soft failure

When a grant expires because the agent did not use it (detected via `QueueGrantExpired` event):
- This is NOT a permanent blocker — the agent may re-queue
- Log the expiry in failure context for diagnostic purposes
- Do NOT create a `BlockedIntent` — the agent can rejoin the queue

## Files to Touch

- `crates/worldwake-ai/src/failure_handling.rs` — handle queue failure events, update exclusive-facility failure logic

## Out of Scope

- `facility_queue_system` event emission (EXCFACACCQUE-003 — assumed complete)
- Queue types (EXCFACACCQUE-001)
- Belief views (EXCFACACCQUE-005)
- Candidate generation or ranking (EXCFACACCQUE-008, 009)
- `BlockedIntentMemory` data structure changes (existing structure should suffice)

## Acceptance Criteria

### Tests that must pass
- Unit test: `QueueHeadFailed` event creates a `BlockedIntent` for the specific facility + ActionDefId
- Unit test: blocked intent from queue failure has appropriate `blocked_until` expiration
- Unit test: `QueueGrantExpired` event does NOT create a blocked intent
- Unit test: agent with blocked intent for facility X does not re-emit queue candidates for facility X (until expiration)
- Unit test: agent with expired blocked intent can re-emit queue candidates
- Unit test: permanent impossibility (facility destroyed) creates long-duration or permanent blocked intent
- `cargo test --workspace` — no regressions

### Invariants that must remain true
- Queue failures create explicit, observable blocked intents — not silent retry loops
- Grant expiry is soft — agent can rejoin queue
- Permanent impossibility is hard — agent stops targeting the facility
- Blocked intent uses existing `BlockedIntentMemory` infrastructure (no new data structures)
- Failure handling reads events through belief view, not world state directly
