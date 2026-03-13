# EXCFACACCQUE-010 — Queue Invalidation Handling as Runtime + Planning Constraint

**Status**: COMPLETED
**Spec sections**: §8, §10 (failure handling)
**Crates**: `worldwake-ai`, `worldwake-core`

## Summary

Reassessed against the current codebase:
- `QueueHeadFailed` and `QueueGrantExpired` events are emitted by `crates/worldwake-systems/src/facility_queue.rs`, but AI planning/failure handling does not consume queue events through `BeliefView`.
- `crates/worldwake-ai/src/failure_handling.rs` is step-failure oriented. Queue-head invalidation is usually **not** an in-flight action failure; it is a later queue-state transition after the `queue_for_facility_use` progress barrier has already completed.
- `BlockedIntentMemory` currently blocks at the `GoalKey` level. That is too coarse to express "avoid facility X for intended action Y, but still consider other ways to satisfy the same goal."

The clean architecture is therefore:
- observe queue invalidation from the AI runtime's local facility-access state transitions, not by teaching `failure_handling.rs` to read queue events
- persist the result as blocker state on the agent
- make planning/search treat a blocked `(facility, intended_action)` pair as unavailable, without suppressing the whole goal family

This keeps the behavior local, explicit, and extensible without introducing a parallel alias path or reusing reservation-collision logic for a queue-specific failure.

## Assumption Reassessment (2026-03-13)

1. `QueueHeadFailed` / `QueueGrantExpired` **do exist** in the event log, emitted by the queue system.
2. AI does **not** currently read queue events through `BeliefView`, and the ticket was wrong to assume it should be implemented that way in this phase.
3. `handle_plan_failure()` still classifies exclusive production failures via generic production conditions such as reservation conflicts and busy workstations, but queue-head invalidation is not primarily a `handle_plan_failure()` concern.
4. `BlockedIntentMemory` does **not** currently support the ticket's claimed facility-specific suppression behavior. Its existing `GoalKey`-level filtering would over-block alternate acquisition/production paths for the same goal.
5. The real gap is runtime/planning coordination:
   - the runtime can observe local facility queue/grant state changes
   - the planner/search layer does not yet have a way to exclude one blocked exclusive facility use while continuing to plan other valid paths for the same goal

## Deliverables

### 1. Detect queue invalidation in the runtime layer

In `crates/worldwake-ai/src/agent_tick.rs` and/or `decision_runtime.rs`:
- track the runtime's queued exclusive-facility intents across ticks
- detect the specific local state transition:
  - previously queued at facility `F`
  - now neither queued nor granted at `F`
  - actor is still in the same local context
- treat that transition as permanent queue-head invalidation, not as soft grant expiry

This is the authoritative signal for "my queued exclusive facility use became impossible."

### 2. Persist a facility-scoped exclusive-use blocker

Use the existing blocked-intent persistence path, but extend it so the blocker can retain:
- related facility entity
- intended exclusive action

This blocker must be precise enough to mean:
- "do not plan `intended_action` against facility `F` until this blocker expires or is cleared"

It must **not** mean:
- "give up on the entire goal forever"
- "disable all other facilities or other acquisition paths"

### 3. Make planning/search respect the blocked facility-use pair

In the planning/search path (`planning_snapshot.rs`, `planning_state.rs`, `search.rs`, or adjacent files as needed):
- surface active blocked `(facility, intended_action)` pairs into planning state
- filter queue/direct exclusive candidates that target a blocked pair
- keep other facilities and other non-exclusive paths available for the same goal

This is the key architectural correction versus the original ticket text. Goal-level candidate suppression alone is not robust enough.

### 4. Keep grant expiry soft

When the runtime observes:
- previously granted at facility `F`
- now no longer granted

do **not** create a hard blocker solely from that transition.

Grant expiry remains a soft failure:
- the agent may re-queue
- other goals may continue interleaving normally

## Files To Touch

- `crates/worldwake-ai/src/agent_tick.rs` — runtime transition detection and blocker recording
- `crates/worldwake-ai/src/decision_runtime.rs` — runtime memory for queued facility intents if needed
- `crates/worldwake-ai/src/planning_snapshot.rs` — expose blocked facility-use constraints to planning
- `crates/worldwake-ai/src/planning_state.rs` — query blocked facility-use constraints during search
- `crates/worldwake-ai/src/search.rs` — filter blocked exclusive facility candidates
- `crates/worldwake-core/src/blocked_intent.rs` — extend blocked intent precision only if needed for the above

## Out Of Scope

- Rewriting the queue system or changing queue event emission
- Adding a separate queue-event belief pipeline
- Global fairness or starvation scoring
- Broad refactors of unrelated blocker types
- Backward-compatibility shims for the old reservation-collision mental model

## Acceptance Criteria

### Tests that must pass
- Unit test: same-place queued membership loss records a blocker for the blocked exclusive facility use
- Unit test: grant expiry / grant disappearance does **not** record a hard blocker
- Unit test: blocked `(facility, intended_action)` pair is filtered from planning/search
- Unit test: another valid facility or alternate path for the same goal remains plannable
- Unit test: blocker persistence does not require queue-event ingestion through `BeliefView`
- `cargo test -p worldwake-ai`
- `cargo test -p worldwake-systems facility_queue -- --nocapture`
- `cargo clippy --workspace --all-targets -- -D warnings`

### Invariants that must remain true
- Queue invalidation is observed from local authoritative state transitions, not from omniscient ad hoc queries
- Grant expiry is soft
- Permanent queue-head invalidation becomes explicit blocker state
- Planning excludes a blocked facility/action pair without suppressing the whole goal family
- No compatibility layer reintroduces reservation-collision logic as the primary exclusive-facility failure model

## Outcome

- Completion date: 2026-03-13
- What actually changed:
  - Re-scoped the work away from `failure_handling.rs` as the primary implementation point and into the AI runtime + planning layers.
  - Extended `BlockedIntent` with `related_action` and added `BlockingFact::ExclusiveFacilityUnavailable` so queue invalidation can be represented as a facility/action-specific blocker.
  - Updated runtime observation handling to detect same-place queued-entry loss, persist the new blocker, and keep grant loss soft.
  - Extended planning snapshot/search filtering so blocked `(facility, intended_action)` pairs are excluded without suppressing the whole goal family.
  - Added targeted tests for queue invalidation blocker recording, soft grant loss, blocked facility/action filtering, and alternate-facility planning.
- Deviations from original plan:
  - Did not implement queue-event ingestion through `BeliefView`.
  - Did not center the change in `crates/worldwake-ai/src/failure_handling.rs`; that layer remains step-failure oriented and was only updated minimally for the new blocker type.
  - Did not use goal-level blocked-intent suppression for this feature, because it would have over-blocked alternate valid paths.
- Verification results:
  - `cargo test -p worldwake-ai`
  - `cargo test -p worldwake-systems facility_queue -- --nocapture`
  - `cargo test --workspace`
  - `cargo clippy --workspace --all-targets -- -D warnings`
