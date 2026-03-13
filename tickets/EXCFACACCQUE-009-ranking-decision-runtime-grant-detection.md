# EXCFACACCQUE-009 — Ranking + Decision Runtime Grant Detection + Replan Trigger

**Spec sections**: §10 (wait-for-grant gap), §12
**Crates**: `worldwake-ai`

## Summary

Update ranking to treat grant possession as higher-motive than merely being queued, and queue-join as a valid progress step. Update the decision runtime to detect grant arrival each tick and trigger a replan when a grant materializes.

## Deliverables

### 1. Ranking updates

In `crates/worldwake-ai/src/ranking.rs`:
- When scoring goal candidates that involve exclusive facility use:
  - Agent already has a matching grant → treat as higher motive (the operation is immediately actionable)
  - Agent is queued but no grant → treat queue membership as valid progress, not a failure state
  - Agent needs to join queue → treat as normal access path (lower priority than having a grant, but valid)
- No abstract fairness bonus. No starvation score. No round-robin logic.

### 2. Decision runtime — grant arrival detection

In `crates/worldwake-ai/src/decision_runtime.rs` (or `agent_tick.rs`):
- Each tick, for agents who are queued at an exclusive facility and whose current goal involves that facility:
  - Check `BeliefView::facility_grant(facility)` to see if the agent now has a grant
  - If grant was received since last check → mark current plan as dirty → trigger replan
  - On replan, the harvest/craft action becomes directly executable since the agent has a grant

### 3. Queue patience replanning

In the decision runtime, also check:
- If agent has been queued for longer than `queue_patience_ticks` (from `UtilityProfile`) → trigger a replan
- The replan may cause the agent to abandon the queue and pursue other goals
- The agent does NOT need to explicitly "leave the queue" — departure from the place (if replanning leads elsewhere) triggers automatic pruning by `facility_queue_system`

### 4. Non-exclusive interleaving

While waiting for a grant, agents should be free to pursue other interruptible goals (eat, drink, rest). Queue membership persists regardless of what action the agent is performing. The decision runtime must not treat queue membership as "busy" state.

## Files to Touch

- `crates/worldwake-ai/src/ranking.rs` — update scoring for queue/grant states
- `crates/worldwake-ai/src/decision_runtime.rs` — grant detection, patience timeout, replan triggers
- `crates/worldwake-ai/src/agent_tick.rs` — possibly integrate grant-check into per-tick driver

## Out of Scope

- Planner ops (EXCFACACCQUE-007 — assumed complete)
- Search-layer queue routing verification (EXCFACACCQUE-008 — assumed complete)
- Failure handling (EXCFACACCQUE-010)
- `facility_queue_system` (EXCFACACCQUE-003)
- Queue types or belief views (EXCFACACCQUE-001, 005)

## Acceptance Criteria

### Tests that must pass
- Unit test: goal with active grant ranks higher than same goal where agent is merely queued
- Unit test: goal requiring queue-join ranks as valid progress (not penalized vs. unrelated goals)
- Unit test: decision runtime detects grant arrival and sets plan dirty flag
- Unit test: dirty plan triggers replan within the same or next tick
- Unit test: agent queued beyond `queue_patience_ticks` triggers replan
- Unit test: agent with `queue_patience_ticks = None` never triggers patience-based replan
- Unit test: agent in queue performing non-exclusive action (eat) retains queue membership
- Unit test: agent with grant that replans to a different goal does NOT immediately start the exclusive action (grant expires naturally via EXCFACACCQUE-003)
- `cargo test --workspace` — no regressions

### Invariants that must remain true
- No abstract fairness score or starvation score introduced
- Queue membership does not block non-exclusive actions
- Grant detection is belief-safe (reads through BeliefView, not world state directly)
- Ranking remains deterministic for the same snapshot
- Replan triggers are observable (plan dirty flag) not silent
