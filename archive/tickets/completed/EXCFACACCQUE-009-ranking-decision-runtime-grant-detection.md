# EXCFACACCQUE-009 — Decision Runtime Grant Detection + Queue Patience Replan

**Status**: COMPLETED
**Spec sections**: §10 (wait-for-grant gap), §12
**Crates**: `worldwake-ai`

## Summary

Reassessed against the current codebase:
- Exclusive-facility planning is already queue-aware in `search.rs`: without a grant, exclusive goals route through `queue_for_facility_use`; with a matching grant, the planner emits the direct exclusive action.
- Plan selection already prefers the directly executable path when candidate priority/motive ties because shorter plans win before deterministic tie-breaks.
- `queue_patience_ticks` already exists on `FacilityQueueDispositionProfile` in `worldwake-core`, not on `UtilityProfile`.

The remaining gap is in the AI runtime: it does not currently observe queue/grant state changes as part of its per-tick runtime snapshot, so grant arrival does not mark the runtime dirty, and queue patience is not evaluated against authoritative queue membership.

## Assumption Reassessment (2026-03-13)

1. `search.rs` already routes exclusive-facility goals through `queue_for_facility_use` when no matching grant is active, and skips that barrier when a matching grant exists.
2. `plan_selection.rs` already prefers directly executable shorter plans after goal priority and motive ties; no new ranking layer was required for grant-vs-queue differentiation.
3. `queue_patience_ticks` already exists on `FacilityQueueDispositionProfile` in `worldwake-core`; the original ticket text was incorrect to attribute it to `UtilityProfile`.
4. The real runtime gap was in observation tracking: `AgentDecisionRuntime` snapshotting previously ignored facility queue/grant state entirely.

## Deliverables

### 1. Decision runtime — grant arrival detection

In `crates/worldwake-ai/src/agent_tick.rs` and/or `decision_runtime.rs`:
- Extend the runtime observation snapshot so it notices relevant local queue/grant changes
- Use `BeliefView::facility_grant(facility)` for grant detection rather than direct world reads
- If the actor receives a matching grant after previously being queued without one, mark the runtime dirty so the next plan selection pass can adopt the directly executable exclusive action

### 2. Queue patience replanning

In the decision runtime:
- Read `queue_patience_ticks` from `FacilityQueueDispositionProfile`
- If an agent has been queued beyond its configured patience threshold and still has no grant, mark the runtime dirty for replanning
- The replan may cause the agent to abandon the queue and pursue other goals
- The agent does NOT need to explicitly "leave the queue" — departure from the place (if replanning leads elsewhere) triggers automatic pruning by `facility_queue_system`

### 3. Non-exclusive interleaving

While waiting for a grant, agents should be free to pursue other interruptible goals (eat, drink, rest). Queue membership persists regardless of what action the agent is performing. The decision runtime must not treat queue membership as "busy" state.

## Files to Touch

- `crates/worldwake-ai/src/decision_runtime.rs` — runtime snapshot fields if needed
- `crates/worldwake-ai/src/agent_tick.rs` — integrate queue/grant observation and patience checks into per-tick runtime refresh
- `crates/worldwake-ai/src/search.rs` / `crates/worldwake-ai/src/plan_selection.rs` — read-only confirmation only unless runtime changes reveal a real gap

## Out of Scope

- Ranking-layer motive changes unless runtime work exposes a real plan-selection deficiency
- Planner ops (spec path already implemented in current code)
- Search-layer queue routing verification (already implemented in current code)
- Failure handling (EXCFACACCQUE-010)
- `facility_queue_system` (EXCFACACCQUE-003)
- Queue/core data model changes unless required for a narrowly scoped runtime read

## Acceptance Criteria

### Tests that must pass
- Unit test: decision runtime detects grant arrival and sets plan dirty flag
- Unit test: dirty plan triggers replan within the same or next tick
- Unit test: agent queued beyond `queue_patience_ticks` from `FacilityQueueDispositionProfile` triggers replan
- Unit test: agent with `queue_patience_ticks = None` never triggers patience-based replan
- Unit test: agent in queue performing non-exclusive action (eat) retains queue membership
- Unit test: agent with grant that replans to a different goal does NOT immediately start the exclusive action (grant expires naturally via EXCFACACCQUE-003)
- `cargo test --workspace` — no regressions

### Invariants that must remain true
- No abstract fairness score or starvation score introduced
- Queue membership does not block non-exclusive actions
- Grant detection is belief-safe (reads through BeliefView, not world state directly)
- Goal ranking remains goal-level and deterministic for the same snapshot
- Replan triggers are observable (plan dirty flag) not silent

## Outcome

What changed versus the original plan:
- Did not modify `ranking.rs`. After reassessment, goal ranking was already the wrong layer for this behavior; the planner/search path and plan-selection tie-breaks already handled grant-vs-queue execution cleanly.
- Added facility-access observation tracking to `AgentDecisionRuntime` and `agent_tick.rs` so queue/grant changes participate in the runtime dirtiness snapshot.
- Extended `BeliefView` / `OmniscientBeliefView` with queue join tick and queue patience accessors so grant detection and patience checks stay on the belief/read-model path.
- Added runtime tests covering grant-arrival dirtying, direct-harvest replanning after grant arrival, patience exhaustion, missing patience configuration, and queue persistence during a non-exclusive action.
- Added the minimal clippy cleanup required to keep workspace linting green.
