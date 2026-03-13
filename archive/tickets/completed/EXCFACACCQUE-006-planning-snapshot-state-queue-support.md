**Status**: ✅ COMPLETED

# EXCFACACCQUE-006 — Planning Snapshot + Planning State Queue/Grant Support

**Spec sections**: §9, §10 (planning infrastructure)
**Crates**: `worldwake-ai`, `worldwake-sim`
**Depends on**: implemented queue/grant groundwork from `EXCFACACCQUE-001` through `EXCFACACCQUE-005`

## Summary

Extend the planning snapshot and mutable planning state to cache and simulate facility queue/grant state. The GOAP planner needs to know whether an agent has a grant, is queued, or needs to join a queue — and to simulate the effects of queue-related actions during plan search.

## Assumptions Reassessed

- `BeliefView::facility_queue_position` and `BeliefView::facility_grant` already exist and are covered in `worldwake-sim`; that portion of the stack is no longer future work. This ticket must consume those APIs, not redefine them.
- `PlanningState` already implements `BeliefView`, but today it hardcodes both facility queue methods to `None`. There is an explicit AI regression test documenting that gap.
- The real missing work is entirely in `worldwake-ai`: `PlanningSnapshot` does not capture facility queue/grant reads yet, and `PlanningState` has no hypothetical overlay for queue/grant transitions.
- The original ticket overstated the scope of hypothetical simulation. `PlanningState` is a single-actor search state, not a generic multi-agent world simulator. The correct architecture is actor-scoped queue/grant overlays for the planning actor, while preserving per-facility read-only visibility of the current active grant.
- Full queue membership for arbitrary third parties is not available through the current belief API, and inventing it here would create fake knowledge. This ticket must not fabricate a broader queue model than the planner can justify from local belief inputs.

## Architecture Check

- The clean architecture is: immutable snapshot of locally known facility queue/grant facts, plus a lightweight hypothetical overlay in `PlanningState`. That matches the existing snapshot/overlay pattern and keeps planning mutations separate from world state.
- Queue/grant simulation should be actor-scoped because the planner reasons about one actor's next actions. Adding generic multi-actor hypothetical queue mutation now would look extensible, but it would actually be ungrounded because the snapshot does not include full queue membership.
- No compatibility layer or alias API should be introduced. `PlanningState` should stop returning hardcoded `None` and should instead answer from snapshot data plus overlay state.

## Deliverables

### 1. Planning snapshot — cache queue/grant state

In `crates/worldwake-ai/src/planning_snapshot.rs`, extend the facility-facing snapshot data to capture:
- The planning actor's queue position at each included facility, if known
- The current active facility grant, if one exists

This data is read from `BeliefView::facility_queue_position` and `BeliefView::facility_grant` during snapshot construction. The snapshot remains immutable after construction.

### 2. Planning state — simulate queue effects

In `crates/worldwake-ai/src/planning_state.rs`, add an actor-scoped hypothetical overlay for:
- "planning actor has joined this facility queue"
- "planning actor has received this facility grant"
- "planning actor has consumed/cleared this facility grant"

This enables the planner to reason about the queue-join → wait → grant → exclusive-action sequence without pretending to simulate unrelated actors' hidden queue state.

### 3. Query methods on planning state

Add direct actor-scoped helpers that planner ops can call:
- `has_actor_facility_grant(&self, facility: EntityId, action_def: ActionDefId) -> bool`
- `is_actor_queued_at_facility(&self, facility: EntityId) -> bool`
- `simulate_queue_join(self, facility: EntityId, action_def: ActionDefId) -> Self`
- `simulate_grant_received(self, facility: EntityId, action_def: ActionDefId) -> Self`
- `simulate_grant_consumed(self, facility: EntityId) -> Self`

`BeliefView` methods on `PlanningState` should also stop returning unconditional `None`; they should answer from the snapshot plus overlay. For actor-specific queue position queries, they are only authoritative for the planning actor and should remain conservative for other actors.

## Files to Touch

- `crates/worldwake-ai/src/planning_snapshot.rs` — add queue/grant fields to snapshot
- `crates/worldwake-ai/src/planning_state.rs` — add simulation methods for queue state

## Out of Scope

- Planner ops that use these methods (EXCFACACCQUE-007)
- Candidate generation (EXCFACACCQUE-008)
- Ranking or decision runtime (EXCFACACCQUE-009)
- BeliefView trait changes or omniscient queue/grant reads (EXCFACACCQUE-005 — already complete)
- Actual queue system logic (EXCFACACCQUE-003)
- Multi-actor hypothetical queue simulation beyond the planning actor

## Acceptance Criteria

### Tests that must pass
- Unit test: snapshot captures the planning actor's queue position at an included facility
- Unit test: snapshot captures the active facility grant when present
- Unit test: snapshot stores no queue/grant data when the facility has none
- Unit test: `simulate_queue_join` marks the planning actor as queued and clears any stale grant for that facility
- Unit test: `simulate_grant_received` sets a matching grant and removes queued state for that facility
- Unit test: `simulate_grant_consumed` clears the grant without mutating the immutable snapshot
- Unit test: `PlanningState` `BeliefView` queue/grant methods answer from snapshot data and hypothetical overrides
- Unit test: `PlanningState` remains conservative for non-planning-actor queue-position queries
- `cargo test -p worldwake-ai`
- `cargo test -p worldwake-sim`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace`

### Invariants that must remain true
- Planning snapshot is immutable after construction
- Planning state mutations are hypothetical — they never modify world state
- Grant checks use `ActionDefId` matching, not queue position heuristics
- Deterministic behavior — same snapshot plus same overlay transitions produce the same answers
- No global queue reconstruction — queries are per-facility and limited to belief-backed data

## Outcome

- Completed: 2026-03-13
- What actually changed:
  - `PlanningSnapshot` now caches per-facility queue/grant facts needed by the planner: the planning actor's observed queue position and the facility's current active grant.
  - `PlanningState` now answers facility queue/grant reads from snapshot data plus an actor-scoped hypothetical overlay instead of hardcoded `None`.
  - Added actor-scoped planning helpers for queue/grant semantics: queued-state detection, matching-grant detection, simulated queue join, simulated grant receipt, and simulated grant consumption.
  - Preserved conservative belief behavior for non-planning actors instead of inventing multi-actor hypothetical queue knowledge that the current belief API cannot justify.
  - Strengthened AI tests to cover snapshot capture, overlay transitions, conservative non-planning-actor behavior, and snapshot immutability under hypothetical grant consumption.
- Deviations from original plan:
  - The original ticket assumed generic `(facility, actor)` hypothetical queue simulation. That was corrected before implementation because `PlanningState` is a single-actor search state and the current belief surface does not expose full third-party queue membership.
  - `simulate_queue_join` intentionally does not fabricate a numeric queue position. It marks the actor as queued while leaving `facility_queue_position` conservative when the exact hypothetical position is unknown.
  - No `BeliefView` trait work or omniscient sim work was needed here; that had already landed in `EXCFACACCQUE-005`.
- Verification:
  - `cargo test -p worldwake-ai`
  - `cargo test -p worldwake-sim`
  - `cargo clippy --workspace --all-targets -- -D warnings`
  - `cargo test --workspace`
