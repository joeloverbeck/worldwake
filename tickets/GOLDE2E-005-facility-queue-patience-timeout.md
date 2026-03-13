# GOLDE2E-005: Facility Queue Patience Timeout

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Possible — `FacilityQueueDispositionProfile` and patience-based abandonment may be incomplete
**Deps**: None (facility queue infrastructure exists from EXCFACACCQUE tickets)

## Problem

Scenario 9 proves queue contention with successful rotation, but not the abandonment path. When an agent's queue position never improves and their patience expires, they should abandon the queue and replan to an alternative. This failure/recovery path is untested.

## Report Reference

Backlog item **P-NEW-4** in `reports/golden-e2e-coverage-analysis.md` (Tier 2, composite score 4).

## Assumption Reassessment (2026-03-13)

1. `FacilityQueueDispositionProfile` may exist with `queue_patience_ticks` — verify.
2. `FacilityUseQueue` and `facility_queue_system()` exist in `worldwake-systems/src/facility_queue.rs`.
3. The AI planner must be able to handle queue abandonment and replan (possibly via `handle_plan_failure()`).
4. An alternative facility or acquisition path must exist in the topology for the replan to succeed.

## Architecture Check

1. Queue patience should be a profile-driven parameter, not a hardcoded constant.
2. Abandonment should flow through the standard plan failure / blocked intent path.

## Engine-First Mandate

If implementing this e2e suite reveals that queue patience tracking, abandonment signaling, or post-abandonment replanning are incomplete or architecturally unsound — do NOT patch around it. Instead, design and implement a comprehensive architectural solution that makes facility queue patience clean, robust, and extensible. Document any engine changes in the ticket outcome.

## What to Change

### 1. Verify/implement patience tracking

Ensure `FacilityQueueDispositionProfile` (or equivalent) tracks patience and the AI runtime handles patience expiry → queue abandonment → replan.

### 2. New golden test in `golden_production.rs`

**Setup**: Two facilities with exclusive policy. Agent joins queue at facility A, but another agent holds the grant indefinitely (or grant keeps being renewed). Agent's patience expires. Facility B exists as an alternative.

**Assertions**:
- Agent joins queue at facility A.
- Agent's queue position does not improve within patience window.
- Agent abandons queue at facility A.
- Agent replans to facility B (or alternative acquisition path).

## Files to Touch

- `crates/worldwake-ai/tests/golden_production.rs` (modify)
- `crates/worldwake-ai/tests/golden_harness/mod.rs` (modify, if helpers needed)
- Engine files TBD if patience tracking is incomplete

## Out of Scope

- Multi-facility queue balancing algorithms
- Queue priority ordering beyond FIFO
- Grant expiry (separate ticket GOLDE2E-006)

## Acceptance Criteria

### Tests That Must Pass

1. `golden_facility_queue_patience_timeout` — agent abandons queue after patience expires, replans to alternative
2. Existing suite: `cargo test -p worldwake-ai golden_`
3. Full workspace: `cargo test --workspace`

### Invariants

1. All behavior is emergent — no manual queue manipulation
2. Patience is profile-driven, not hardcoded
3. Conservation holds throughout

## Post-Implementation

After implementing this suite, update `reports/golden-e2e-coverage-analysis.md`:
- Add the new scenario to Part 1 (Proven Emergent Scenarios)
- Remove P-NEW-4 from the Part 3 backlog
- Update Part 4 summary statistics

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_production.rs::golden_facility_queue_patience_timeout` — proves patience-based abandonment

### Commands

1. `cargo test -p worldwake-ai golden_facility_queue_patience`
2. `cargo test --workspace && cargo clippy --workspace`
