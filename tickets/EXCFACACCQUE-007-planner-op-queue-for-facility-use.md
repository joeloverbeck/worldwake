# EXCFACACCQUE-007 — `QueueForFacilityUse` Planner Op + Semantics

**Spec sections**: §10
**Crates**: `worldwake-ai`

## Summary

Add a `QueueForFacilityUse` planner operation kind with blocking-barrier semantics. This teaches the GOAP planner that exclusive facility operations require joining a queue first, and that the plan must suspend until a grant materializes.

## Deliverables

### 1. Add `QueueForFacilityUse` to `PlannerOpKind`

In `crates/worldwake-ai/src/planner_ops.rs`, add a new variant:

```rust
QueueForFacilityUse {
    facility: EntityId,
    intended_action: ActionDefId,
}
```

### 2. Define `PlannerOpSemantics` for queue op

Register semantics for the new op:
- **Barriers**: This op IS a barrier — it represents "join queue and wait for grant"
- **Mid-plan viability**: The op is viable mid-plan (agents can queue while pursuing other goals)
- **Goal relevance**: Relevant to any goal that requires an exclusive facility operation
- **Blocking barrier**: After this step, the plan is suspended until `has_facility_grant` returns true

### 3. Planning state effects

When the planner expands this op during search:
- Call `planning_state.simulate_queue_join(facility, actor, intended_action)`
- The planner treats subsequent exclusive-action steps as requiring `has_facility_grant` to be true
- If the agent already has a grant (snapshot shows granted), skip the queue step entirely

### 4. Integration with search

In `crates/worldwake-ai/src/search.rs`, ensure that:
- `QueueForFacilityUse` nodes are expandable during best-first search
- The blocking barrier means the planner does not expand past this node unless a grant is assumed
- The planner can produce partial plans that end with "waiting for grant" as a valid plan state

## Files to Touch

- `crates/worldwake-ai/src/planner_ops.rs` — add `QueueForFacilityUse` variant + semantics
- `crates/worldwake-ai/src/search.rs` — handle new op in plan search expansion
- `crates/worldwake-ai/src/goal_model.rs` — map exclusive-facility goals through queue op

## Out of Scope

- Candidate generation (EXCFACACCQUE-008)
- Ranking or decision runtime (EXCFACACCQUE-009)
- Failure handling (EXCFACACCQUE-010)
- The `queue_for_facility_use` action itself (EXCFACACCQUE-002)
- Planning snapshot/state infrastructure (EXCFACACCQUE-006 — assumed complete)

## Acceptance Criteria

### Tests that must pass
- Unit test: planner generates `QueueForFacilityUse` step before exclusive-action step when agent has no grant
- Unit test: planner skips `QueueForFacilityUse` step when agent already has a matching grant
- Unit test: planner skips `QueueForFacilityUse` step when agent is already queued at the facility
- Unit test: `QueueForFacilityUse` correctly simulates queue-join on planning state
- Unit test: plan with `QueueForFacilityUse` followed by exclusive action is valid
- Unit test: plan search respects blocking barrier — does not expand exclusive action without grant
- `cargo test --workspace` — no regressions

### Invariants that must remain true
- Planner op uses `ActionDefId` to specify the intended operation (no parallel enum)
- Blocking barrier semantics are consistent with existing travel barriers
- No omniscient queue discovery — only colocated facilities are considered
- Plan search remains deterministic
- No abstract fairness scoring in plan evaluation
