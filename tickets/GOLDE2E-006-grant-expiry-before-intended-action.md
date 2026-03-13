# GOLDE2E-006: Grant Expiry Before Intended Action

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Possible — re-queue after grant expiry may not be wired into the AI loop
**Deps**: None (grant expiry logic exists in `facility_queue_system`)

## Problem

Scenario 9 shows grants being used promptly. This gap tests what happens when a grant expires before the planner schedules the intended action (e.g., because of goal-switching delay). The agent must re-queue or replan, and the queue must advance correctly after the expired grant is cleared.

## Report Reference

Backlog item **P-NEW-5** in `reports/golden-e2e-coverage-analysis.md` (Tier 2, composite score 4).

## Assumption Reassessment (2026-03-13)

1. `GrantedFacilityUse::expires_at` exists and `expire_stale_grant()` runs in `facility_queue_system`.
2. `QueueGrantExpired` event tag exists in `worldwake-core/src/event_tag.rs`.
3. The AI runtime must handle the case where a grant expires while the agent is doing something else — verify the replan/re-queue path exists.
4. The golden harness can set short grant expiry windows via `ExclusiveFacilityPolicy::grant_hold_ticks`.

## Architecture Check

1. Grant expiry → re-queue should flow through the standard plan failure / replanning path.
2. No special-case handling; the facility queue system clears the grant, and the AI observes the state change on its next decision cycle.

## Engine-First Mandate

If implementing this e2e suite reveals that post-grant-expiry replanning, re-queue behavior, or the interaction between grant expiry events and the AI decision runtime is incomplete or architecturally unsound — do NOT patch around it. Instead, design and implement a comprehensive architectural solution. Document any engine changes in the ticket outcome.

## What to Change

### 1. New golden test in `golden_production.rs`

**Setup**: Agent at a facility with exclusive policy and very short grant expiry (e.g., 2 ticks). Configure the agent so that after receiving the grant, a competing need forces a goal switch before the intended action starts. The grant expires during the interruption.

**Assertions**:
- Agent queues for facility, receives grant.
- A competing need causes goal switch before the exclusive action starts.
- Grant expires (`QueueGrantExpired` event emitted).
- Agent re-queues or replans to complete the original goal.

## Files to Touch

- `crates/worldwake-ai/tests/golden_production.rs` (modify)
- `crates/worldwake-ai/tests/golden_harness/mod.rs` (modify, if helpers needed)
- Engine files TBD if re-queue after expiry path is incomplete

## Out of Scope

- Grant renewal mechanics
- Multiple concurrent grant expirations
- Patience timeout (separate ticket GOLDE2E-005)

## Acceptance Criteria

### Tests That Must Pass

1. `golden_grant_expiry_before_intended_action` — agent's grant expires, agent re-queues or replans
2. Existing suite: `cargo test -p worldwake-ai golden_`
3. Full workspace: `cargo test --workspace`

### Invariants

1. All behavior is emergent — no manual grant manipulation after setup
2. At most one active grant per facility at any time
3. Conservation holds throughout

## Post-Implementation

After implementing this suite, update `reports/golden-e2e-coverage-analysis.md`:
- Add the new scenario to Part 1 (Proven Emergent Scenarios)
- Remove P-NEW-5 from the Part 3 backlog
- Update Part 4 summary statistics

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_production.rs::golden_grant_expiry_before_intended_action` — proves grant expiry recovery

### Commands

1. `cargo test -p worldwake-ai golden_grant_expiry`
2. `cargo test --workspace && cargo clippy --workspace`
