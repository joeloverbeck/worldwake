# EXCFACACCQUE-008 — Candidate Generation Updates for Queue Routing

**Spec sections**: §11
**Crates**: `worldwake-ai`

## Summary

Update candidate generation so that autonomous agents route exclusive facility use through the queue/grant path instead of directly emitting exclusive action requests.

## Deliverables

### 1. Update candidate generation for exclusive facilities

In `crates/worldwake-ai/src/candidate_generation.rs`, modify the logic that generates goal candidates involving exclusive facility actions (harvest, craft):

**New routing rules:**
- If actor already has a matching grant at the facility → emit the direct exclusive action (harvest/craft) as a candidate
- Else if actor is already queued at the facility → do NOT emit duplicate `queue_for_facility_use` actions; the agent is already waiting
- Else if the facility is locally visible, has `ExclusiveFacilityPolicy`, and the use is legal in principle → emit `queue_for_facility_use` with the appropriate `ActionDefId`

### 2. Remove direct exclusive action emission without grant

The current pattern where multiple agents repeatedly emit the same exclusive action request from the same snapshot must be replaced. Autonomous agents must go through the queue path for exclusive facilities.

**Exception**: Human-controlled agents (`ControlSource::Human`) may bypass the queue if the player explicitly requests it (or this can be deferred to a future ticket — document the decision).

### 3. Affordance integration

Ensure `get_affordances()` in `crates/worldwake-sim/src/affordance_query.rs` includes `queue_for_facility_use` in the affordance list for facilities with `ExclusiveFacilityPolicy`. The affordance should carry the `ActionDefId` of each exclusive operation available at that facility.

## Files to Touch

- `crates/worldwake-ai/src/candidate_generation.rs` — modify exclusive facility candidate logic
- `crates/worldwake-sim/src/affordance_query.rs` — include queue affordances for exclusive facilities

## Out of Scope

- Ranking updates (EXCFACACCQUE-009)
- Decision runtime (EXCFACACCQUE-009)
- Failure handling (EXCFACACCQUE-010)
- Planner ops (EXCFACACCQUE-007 — assumed complete)
- The `queue_for_facility_use` action handler (EXCFACACCQUE-002)
- Human player queue bypass (may be future work)

## Acceptance Criteria

### Tests that must pass
- Unit test: agent without grant and not queued generates `queue_for_facility_use` candidate for colocated exclusive facility
- Unit test: agent with active grant generates direct exclusive action candidate (not queue action)
- Unit test: agent already queued does NOT generate duplicate `queue_for_facility_use` candidate
- Unit test: non-exclusive facilities do not generate queue candidates
- Unit test: multiple exclusive operation types at the same facility generate separate queue candidates (one per ActionDefId)
- Unit test: facility without `ExclusiveFacilityPolicy` still generates direct action candidates (no regression for non-exclusive facilities)
- Unit test: affordance list for exclusive facility includes `queue_for_facility_use`
- `cargo test --workspace` — no regressions

### Invariants that must remain true
- Autonomous agents never directly emit exclusive action candidates without a grant
- Queue candidates reference `ActionDefId`, not a parallel taxonomy
- Only colocated facilities produce candidates (locality preserved)
- Candidate generation remains deterministic
- No duplicate queue entries — at most one queue candidate per facility per agent
