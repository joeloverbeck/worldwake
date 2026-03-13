# EXCFACACCQUE-006 — Planning Snapshot + Planning State Queue/Grant Support

**Spec sections**: §9, §10 (planning infrastructure)
**Crates**: `worldwake-ai`, `worldwake-sim`

## Summary

Extend the planning snapshot and mutable planning state to cache and simulate facility queue/grant state. The GOAP planner needs to know whether an agent has a grant, is queued, or needs to join a queue — and to simulate the effects of queue-related actions during plan search.

## Deliverables

### 1. Planning snapshot — cache queue/grant state

In `crates/worldwake-ai/src/planning_snapshot.rs`, extend `SnapshotEntity` (or the facility snapshot) to capture:
- Whether the planning agent has an active grant at each colocated facility (and for which `ActionDefId`)
- Whether the planning agent is currently queued at each colocated facility (and position)

This is read from `BeliefView::facility_queue_position` and `BeliefView::facility_grant` during snapshot construction.

### 2. Planning state — simulate queue effects

In `crates/worldwake-ai/src/planning_state.rs`, add the ability to:
- Track hypothetical "agent has joined queue" state during plan search
- Track hypothetical "agent has received grant" state during plan search
- Check whether a grant exists for `(actor, ActionDefId)` at a given facility

This enables the planner to reason about the queue-join → wait → grant → exclusive-action sequence.

### 3. Query methods on planning state

Add methods that the planner ops will call:
- `has_facility_grant(&self, facility: EntityId, actor: EntityId, action_def: ActionDefId) -> bool`
- `is_queued_at_facility(&self, facility: EntityId, actor: EntityId) -> bool`
- `simulate_queue_join(&mut self, facility: EntityId, actor: EntityId, action_def: ActionDefId)`
- `simulate_grant_received(&mut self, facility: EntityId, actor: EntityId, action_def: ActionDefId)`
- `simulate_grant_consumed(&mut self, facility: EntityId, actor: EntityId)`

## Files to Touch

- `crates/worldwake-ai/src/planning_snapshot.rs` — add queue/grant fields to snapshot
- `crates/worldwake-ai/src/planning_state.rs` — add simulation methods for queue state

## Out of Scope

- Planner ops that use these methods (EXCFACACCQUE-007)
- Candidate generation (EXCFACACCQUE-008)
- Ranking or decision runtime (EXCFACACCQUE-009)
- BeliefView trait changes (EXCFACACCQUE-005 — assumed complete)
- Actual queue system logic (EXCFACACCQUE-003)

## Acceptance Criteria

### Tests that must pass
- Unit test: snapshot correctly captures that agent has a grant at facility X for ActionDefId Y
- Unit test: snapshot correctly captures that agent is queued at position 2 at facility X
- Unit test: snapshot correctly captures no queue/grant state when none exists
- Unit test: `simulate_queue_join` sets queued state; subsequent `is_queued_at_facility` returns true
- Unit test: `simulate_grant_received` sets grant state; subsequent `has_facility_grant` returns true
- Unit test: `simulate_grant_consumed` clears grant state; subsequent `has_facility_grant` returns false
- Unit test: planning state is independent of snapshot (hypothetical overrides work correctly)
- `cargo test --workspace` — no regressions

### Invariants that must remain true
- Planning snapshot is immutable after construction
- Planning state mutations are hypothetical — they never modify world state
- All queue/grant queries use `ActionDefId` matching (not position-based)
- Deterministic behavior — same snapshot produces same planning state
- No global facility iteration — queries are per-facility
