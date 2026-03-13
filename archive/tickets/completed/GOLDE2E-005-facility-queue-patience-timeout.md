# GOLDE2E-005: Facility Queue Patience Timeout

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Required — queue patience exists, but authoritative abandonment is incomplete
**Deps**: None (facility queue infrastructure exists from EXCFACACCQUE tickets)

## Problem

Scenario 9 proves queue contention with successful rotation, but not the abandonment path. Reassessment shows the missing piece is not just test coverage: when an agent's queue patience expires, the current engine can mark the runtime dirty for replanning, but it does not authoritatively remove the agent from `FacilityUseQueue`. That means the intended failure/recovery path is architecturally incomplete and untested end to end.

## Report Reference

Backlog item **P-NEW-4** in `reports/golden-e2e-coverage-analysis.md` (Tier 2, composite score 4).

## Assumption Reassessment (2026-03-13)

1. `FacilityQueueDispositionProfile` already exists in `worldwake-core/src/facility_queue.rs` with `queue_patience_ticks: Option<NonZeroU32>`, and `BeliefView::facility_queue_patience_ticks()` exposes it.
2. `facility_queue_patience_exhausted()` already exists in `worldwake-ai/src/agent_tick.rs`, but today it only contributes to `runtime.dirty`; it does not remove the actor from the authoritative queue.
3. The blocked-facility replanning pipeline already exists:
   - queue disappearance without grant records `BlockingFact::ExclusiveFacilityUnavailable` in `handle_facility_queue_transitions()`;
   - `build_planning_snapshot_with_blocked_facility_uses()` feeds that into planner search;
   - search already filters blocked `(facility, intended_action)` pairs.
4. No current engine path calls `FacilityUseQueue::remove_actor()` for patience expiry, so "abandon queue and replan" is not currently a real world-state transition.
5. The golden harness does not seed `FacilityQueueDispositionProfile` by default, so this scenario needs explicit patience setup.
6. An alternative facility or acquisition path still must exist in the scenario topology for replanning to succeed after abandonment.

## Architecture Check

1. Queue patience should remain a profile-driven per-agent parameter, not a hardcoded constant.
2. Queue abandonment must be authoritative world state, not only a decision-runtime hint. If the actor is still in `FacilityUseQueue`, the architecture is wrong.
3. Replanning should reuse the existing blocked-facility pipeline instead of introducing a second alias path for "patience timeout."

## Engine-First Mandate

Reassessment already shows the architectural gap: queue patience tracking exists, but abandonment signaling is not authoritative. Do NOT patch around that in the golden test. Implement the engine change needed to make patience expiry remove the actor from the queue and feed the existing blocked-facility replanning path. Document any engine changes in the ticket outcome.

## What to Change

### 1. Complete the patience-expiry abandonment path

Ensure the full chain exists:
- `FacilityQueueDispositionProfile` supplies per-agent patience;
- the AI/runtime notices patience expiry;
- the actor is authoritatively removed from `FacilityUseQueue`;
- that removal records the existing blocked-facility signal;
- replanning avoids the abandoned facility and can choose an alternative path.

This should extend the current architecture rather than bypass it. The clean path is to reuse `FacilityUseQueue`, `handle_facility_queue_transitions()`, and blocked-facility planning support already in the codebase.

### 2. New golden test in `golden_production.rs`

**Setup**: Two facilities with exclusive policy. Agent joins queue at facility A, but another agent monopolizes the grant long enough for patience to expire. Facility B exists as a valid alternative.

**Assertions**:
- Agent joins queue at facility A.
- Agent's queue position does not improve within patience window.
- Agent is removed from the authoritative queue at facility A when patience expires.
- The abandonment flows through the existing blocked-facility mechanism rather than a special-case planner branch.
- Agent replans to facility B (or alternative acquisition path).

### 3. Add focused non-golden coverage for the engine invariant

The golden scenario should prove the end-to-end behavior, but the architectural gap also warrants focused tests around the queue-abandonment runtime path so the authoritative dequeue invariant is not only protected indirectly.

## Files to Touch

- `crates/worldwake-ai/tests/golden_production.rs` (modify)
- `crates/worldwake-ai/tests/golden_harness/mod.rs` (modify, if helpers needed)
- `crates/worldwake-ai/src/agent_tick.rs` (modify and/or add focused tests)
- Engine files that own authoritative queue state transitions, if needed

## Out of Scope

- Multi-facility queue balancing algorithms
- Queue priority ordering beyond FIFO
- Grant expiry (separate ticket GOLDE2E-006)

## Acceptance Criteria

### Tests That Must Pass

1. Focused tests prove patience expiry removes the actor from the authoritative queue and records the standard blocked-facility signal.
2. `golden_facility_queue_patience_timeout` proves the full end-to-end abandonment and replanning path.
3. Existing suite: `cargo test -p worldwake-ai golden_`
4. Full workspace: `cargo test --workspace`
5. `cargo clippy --workspace`

### Invariants

1. All behavior is emergent after setup — no manual queue manipulation during the scenario
2. Patience is profile-driven, not hardcoded
3. Queue abandonment is authoritative world state, not just runtime dirtiness
4. Conservation holds throughout

## Post-Implementation

After implementing this suite, update `reports/golden-e2e-coverage-analysis.md`:
- Add the new scenario to Part 1 (Proven Emergent Scenarios)
- Remove P-NEW-4 from the Part 3 backlog
- Update Part 4 summary statistics

## Test Plan

### New/Modified Tests

1. Focused `worldwake-ai` tests around queue-patience abandonment/runtime integration
2. `crates/worldwake-ai/tests/golden_production.rs::golden_facility_queue_patience_timeout` — proves patience-based abandonment and alternative replanning

### Commands

1. `cargo test -p worldwake-ai facility_queue_patience`
2. `cargo test -p worldwake-ai golden_facility_queue_patience`
3. `cargo test --workspace`
4. `cargo clippy --workspace`

## Outcome

### Completion date

2026-03-13

### What actually changed

- Corrected the ticket scope before implementation: queue patience already existed as profile/read-path data, but authoritative queue abandonment did not.
- Added authoritative patience-expiry dequeue logic in `worldwake-ai/src/agent_tick.rs` so expired waiters are removed from `FacilityUseQueue` before the read-phase snapshot, allowing the existing blocked-facility transition path to observe a real queue disappearance.
- Added focused coverage in `worldwake-ai/src/agent_tick.rs` for both invariants:
  - patience expiry removes the actor from authoritative queue state;
  - the resulting disappearance records the standard `ExclusiveFacilityUnavailable` blocker.
- Added `golden_facility_queue_patience_timeout` in `crates/worldwake-ai/tests/golden_production.rs`, proving end-to-end recovery through an alternative facility.
- Added `set_queue_patience(...)` to the golden harness and filled a missing public mutation hole by exposing `WorldTxn::set_component_facility_queue_disposition_profile(...)` in `worldwake-core/src/world_txn.rs`.
- Updated `reports/golden-e2e-coverage-analysis.md` to promote the scenario into the proven suite and remove the stale backlog item.

### Deviations from the original plan

- The original ticket assumed this might be "test only" if patience already existed. Reassessment showed the engine path was incomplete, so the correct implementation was an engine fix plus tests.
- The work also uncovered an API gap: the queue-patience component was readable through beliefs but not conveniently writable through the public transaction surface. Fixing that was necessary for clean end-to-end setup.

### Verification results

- `cargo test -p worldwake-ai abandon_expired_facility_queues_removes_actor_from_authoritative_queue`
- `cargo test -p worldwake-ai abandoned_queue_then_records_standard_exclusive_facility_blocker`
- `cargo test -p worldwake-ai golden_facility_queue_patience_timeout`
- `cargo test -p worldwake-ai golden_`
- `cargo test --workspace`
- `cargo clippy --workspace`
