# GOLDE2E-008: Blocked Facility Use Avoidance in Planner

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Possible — `blocked_facility_uses` filter in planning state may be incomplete
**Deps**: None (blocked intent and planning state exist from E13)

## Problem

After a failed queue+execute cycle at facility A, the planner should avoid re-queueing at facility A via `blocked_facility_uses` in `PlanningState`. If an alternative facility B exists, the agent should route there. This avoidance behavior is untested end-to-end.

## Report Reference

Backlog item **P-NEW-8** in `reports/golden-e2e-coverage-analysis.md` (Tier 2, composite score 4).

## Assumption Reassessment (2026-03-13)

1. `PlanningState` may contain `blocked_facility_uses: BTreeSet<(EntityId, ActionDefId)>` — verify.
2. `candidate_uses_blocked_facility_use()` filter may exist in the planning/search module — verify.
3. The topology can support two facilities with the same workstation tag at different locations.
4. Blocked intent memory populates `blocked_facility_uses` during planning — verify the pipeline.

## Architecture Check

1. Facility avoidance should be driven by `BlockedIntentMemory` → `PlanningState` population, not special-case planner logic.
2. The planner should discover facility B through standard search, not through a dedicated "alternative facility" lookup.

## Engine-First Mandate

If implementing this e2e suite reveals that `blocked_facility_uses`, the `candidate_uses_blocked_facility_use()` filter, or the blocked-intent-to-planning-state pipeline is incomplete or architecturally unsound — do NOT patch around it. Instead, design and implement a comprehensive architectural solution. Document any engine changes in the ticket outcome.

## What to Change

### 1. Verify/implement blocked facility use pipeline

Ensure the full chain exists: failed queue attempt → blocked intent → `PlanningState::blocked_facility_uses` → planner avoids facility A → routes to facility B.

### 2. New golden test in `golden_production.rs`

**Setup**: Two exclusive facilities (A and B) with the same workstation tag at different locations. Agent fails at facility A (e.g., queue patience timeout or structural failure). Facility B is reachable.

**Assertions**:
- Agent attempts to use facility A and fails (queue timeout, grant expiry, or structural failure).
- Blocked intent is recorded for facility A.
- On replanning, the planner avoids facility A and routes to facility B.
- Agent successfully uses facility B.

## Files to Touch

- `crates/worldwake-ai/tests/golden_production.rs` (modify)
- `crates/worldwake-ai/tests/golden_harness/mod.rs` (modify, if helpers needed)
- `crates/worldwake-ai/src/planning_state.rs` (modify, if `blocked_facility_uses` missing)
- `crates/worldwake-ai/src/search.rs` (modify, if filter missing)
- Engine files TBD if pipeline is incomplete

## Out of Scope

- Blocked facility TTL expiry (intent memory TTL handles this)
- More than two alternative facilities
- Non-exclusive facility avoidance

## Acceptance Criteria

### Tests That Must Pass

1. `golden_blocked_facility_use_avoidance` — agent avoids failed facility and routes to alternative
2. Existing suite: `cargo test -p worldwake-ai golden_`
3. Full workspace: `cargo test --workspace`

### Invariants

1. All behavior is emergent — no manual plan injection
2. Facility avoidance flows through the standard blocked intent pipeline
3. Conservation holds throughout

## Post-Implementation

After implementing this suite, update `reports/golden-e2e-coverage-analysis.md`:
- Add the new scenario to Part 1 (Proven Emergent Scenarios)
- Remove P-NEW-8 from the Part 3 backlog
- Update Part 4 summary statistics

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_production.rs::golden_blocked_facility_use_avoidance` — proves planner facility avoidance

### Commands

1. `cargo test -p worldwake-ai golden_blocked_facility_use`
2. `cargo test --workspace && cargo clippy --workspace`
