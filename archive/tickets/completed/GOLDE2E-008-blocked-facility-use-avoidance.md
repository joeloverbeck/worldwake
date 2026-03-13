# GOLDE2E-008: Blocked Facility Use Avoidance in Planner

**Status**: ✅ COMPLETED
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: None expected
**Deps**: None

## Problem

The original ticket assumed blocked exclusive-facility avoidance was still a gap in either the engine or the golden suite. That assumption is no longer correct.

The current codebase already has the intended architecture and behavior coverage:
- `PlanningSnapshot` already carries `blocked_facility_uses: BTreeSet<(EntityId, ActionDefId)>`.
- `build_planning_snapshot_with_blocked_facility_uses()` already derives that set from `BlockedIntentMemory`.
- `candidate_uses_blocked_facility_use()` already filters blocked `(facility, intended_action)` pairs during search.
- `handle_facility_queue_transitions()` already records `BlockingFact::ExclusiveFacilityUnavailable` when a same-place queue membership disappears without a grant.
- `golden_facility_queue_patience_timeout` already proves the real behavior-level recovery path: queue at facility A, abandon it, record the blocker, route to facility B, and satisfy hunger there.

What is missing is not implementation. The stale part is the ticket and the report backlog entry.

## Report Reference

Backlog item **P-NEW-8** in `reports/golden-e2e-coverage-analysis.md` (Tier 2, composite score 4). This ticket resolves that stale backlog item by correcting the documentation and coverage analysis rather than by adding a redundant new golden test.

## Assumption Reassessment (2026-03-13)

1. `blocked_facility_uses` exists today, but it lives in `PlanningSnapshot` and is queried through `PlanningState`; it is not a missing planning-state feature.
2. `candidate_uses_blocked_facility_use()` already exists in `crates/worldwake-ai/src/search.rs`, and focused search tests already cover both filtering the blocked facility and retaining alternate facility paths.
3. The blocked-intent pipeline is already wired end to end:
   - authoritative queue abandonment occurs;
   - same-place queue disappearance records `ExclusiveFacilityUnavailable`;
   - snapshot construction projects blocked `(facility, action)` pairs into planning;
   - planner search excludes the blocked facility/action pair while leaving other valid paths intact.
4. The golden harness already supports the required topology shape: two exclusive facilities with the same workstation tag at different locations.
5. `crates/worldwake-ai/tests/golden_production.rs::golden_facility_queue_patience_timeout` already provides the behavior-level proof this ticket originally requested.

## Architecture Check

1. The current architecture is cleaner than the ticket assumed: blocked facility avoidance is expressed through the existing blocked-intent memory pipeline, not a special-case "pick another facility" branch.
2. Adding another golden test for the same behavior would mostly duplicate `golden_facility_queue_patience_timeout` and the focused planner/runtime tests without improving extensibility.
3. The robust long-term design is to keep:
   - authoritative queue state in the world;
   - blockage memory in `BlockedIntentMemory`;
   - planner visibility in `PlanningSnapshot`;
   - search filtering generic over blocked `(facility, action)` pairs.

## Engine-First Mandate

If future work reveals a real architectural problem here, the fix should still land in the existing blocked-intent pipeline rather than in a bespoke alternative-facility lookup or compatibility alias. This ticket does not justify engine changes today.

## What to Change

### 1. Correct the ticket scope

Record that blocked exclusive-facility avoidance is already implemented and already covered at both focused-test and golden-test layers.

### 2. Remove the stale backlog item from the report

Update `reports/golden-e2e-coverage-analysis.md` so it no longer lists P-NEW-8 as missing golden coverage.

### 3. Strengthen the existing golden proof without duplicating behavior

If a minimal improvement is warranted, prefer a replay/determinism companion for the existing patience-timeout scenario over adding a second behavior-duplicate golden scenario.

## Files to Touch

- `crates/worldwake-ai/tests/golden_production.rs` (modify)
- `reports/golden-e2e-coverage-analysis.md` (modify)
- `tickets/GOLDE2E-008-blocked-facility-use-avoidance.md` (modify, then archive)

## Out of Scope

- Adding a second golden scenario that restates behavior already proven by `golden_facility_queue_patience_timeout`
- Changing blocked-facility planning architecture without a demonstrated deficiency
- Adding special-case alternative-facility search logic

## Acceptance Criteria

### Tests That Must Pass

1. Focused planner/runtime tests for blocked facility avoidance pass.
2. Existing golden suite in `worldwake-ai` passes.
3. Full workspace tests pass.
4. `cargo clippy --workspace` passes.
5. `reports/golden-e2e-coverage-analysis.md` no longer lists P-NEW-8 as a missing golden scenario.

### Invariants

1. No backward-compatibility shims or alias paths are introduced.
2. Blocked facility avoidance remains expressed through the standard blocked-intent pipeline.
3. Golden tests remain focused on distinct emergent behaviors rather than duplicate proofs.

## Post-Implementation

Archive this ticket with an Outcome section documenting what was actually changed after reassessment.

## Test Plan

### New/Modified Tests

- `crates/worldwake-ai/tests/golden_production.rs::golden_facility_queue_patience_timeout_replays_deterministically`

### Commands

1. `cargo test -p worldwake-ai golden_facility_queue_patience_timeout`
2. `cargo test -p worldwake-ai golden_facility_queue_patience_timeout_replays_deterministically`
3. `cargo test -p worldwake-ai golden_`
4. `cargo test --workspace`
5. `cargo clippy --workspace`

## Outcome

### Completion date

2026-03-13

### What actually changed

- Corrected the ticket assumptions to match the current blocked-facility architecture and test suite.
- Removed the stale P-NEW-8 backlog item from `reports/golden-e2e-coverage-analysis.md`.
- Added a deterministic replay companion for the existing patience-timeout golden scenario in `crates/worldwake-ai/tests/golden_production.rs`.
- Kept engine architecture unchanged because the requested behavior was already implemented and already covered.

### Deviations from the original plan

- The original plan proposed adding a new behavior-level golden scenario and possibly engine changes.
- Reassessment showed that would duplicate existing coverage:
  - focused tests already prove blocker recording and planner filtering;
  - `golden_facility_queue_patience_timeout` already proves the behavior-level alternative-facility recovery path.
- The resolved scope became documentation/report correction plus one replay-strengthening test on the existing canonical scenario.

### Verification results

- `cargo test -p worldwake-ai golden_facility_queue_patience_timeout`
- `cargo test -p worldwake-ai golden_facility_queue_patience_timeout_replays_deterministically`
- `cargo test -p worldwake-ai golden_`
- `cargo test --workspace`
- `cargo clippy --workspace`
