# E16DPOLPLAN-006: Integration tests — planner finds Bribe/Threaten plans

**Status**: ✅ COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: None
**Deps**: E16DPOLPLAN-005, E16DPOLPLAN-022, E16DPOLPLAN-023, E16DPOLPLAN-024, E16DPOLPLAN-025

## Problem

Unit tests verify state transitions but don't prove the GOAP search actually selects Bribe/Threaten in realistic multi-step planning scenarios.

## Assumption Reassessment (2026-03-18)

1. `search_plan()` is the GOAP search entry point in `crates/worldwake-ai/src/search.rs` — confirmed
2. `CLAIM_OFFICE_OPS` includes Travel, Bribe, Threaten, DeclareSupport — confirmed
3. Plans are sequences of `PlannerOpKind` steps — confirmed
4. `PlanningBudget` controls search depth and expansion limits — confirmed

## Architecture Check

1. Integration tests use realistic belief views and snapshots to test full search pipeline
2. Tests verify plan content (which ops appear), not just plan existence

## What to Change

### 1. Add integration tests

4 integration tests in `goal_model.rs` or `crates/worldwake-ai/tests/`:
1. **Planner selects Bribe plan**: agent at jurisdiction with goods, bribable target, vacant office. **A competitor agent must be present** with existing support (e.g., self-declared) so the actor faces competition and DeclareSupport alone would produce a tie (ProgressBarrier), motivating the planner to select Bribe for a winning coalition (GoalSatisfied). -> plan contains `PlannerOpKind::Bribe` + `DeclareSupport`
2. **Planner selects Threaten plan**: agent at jurisdiction, high attack_skill, low-courage target. **A competitor agent must be present** with existing support so the planner is motivated to select Threaten rather than relying on DeclareSupport alone. -> plan contains `Threaten`
3. **Planner selects Travel + Bribe**: agent NOT at jurisdiction but has goods. **A competitor agent must be present at the jurisdiction** with existing support. -> plan starts with `Travel` then includes `Bribe` + `DeclareSupport`
4. **Planner rejects Threaten against high-courage**: target courage exceeds attack_skill -> Threaten NOT in plan. Planner falls back to Bribe (if goods available) or DeclareSupport with ProgressBarrier terminal kind.

## Files to Touch

- `crates/worldwake-ai/src/goal_model.rs` (modify — test module) or `crates/worldwake-ai/tests/` (new integration test file)

## Out of Scope

- Golden E2E tests (separate tickets)
- Changes to production code
- Authoritative action execution
- Testing action handlers

## Acceptance Criteria

### Tests That Must Pass

1. All 4 integration tests pass
2. Plan for test 1 contains `PlannerOpKind::Bribe` step
3. Plan for test 2 contains `PlannerOpKind::Threaten` step
4. Plan for test 3 contains `Travel` before `Bribe`
5. Plan for test 4 does NOT contain `Threaten`
6. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. Plans are valid sequences of operations that the planner could execute
2. Bribe plans always include commodity cost deduction
3. Threaten plans only appear when attack_skill > target courage

## Test Plan

### New/Modified Tests

1. `planner_selects_bribe_plan`
2. `planner_selects_threaten_plan`
3. `planner_selects_travel_then_bribe`
4. `planner_rejects_threaten_against_high_courage`

### Commands

1. `cargo test -p worldwake-ai`
2. `cargo clippy --workspace`

## Dependency Chain Note

This ticket's test scenarios depend on the coalition-aware planner changes from E16DPOLPLAN-022 (coalition-aware terminal kind), E16DPOLPLAN-023 (deferred ProgressBarrier semantics), E16DPOLPLAN-024 (solo DeclareSupport GoalSatisfied in uncontested scenarios), and E16DPOLPLAN-025 (deferred ProgressBarrier for tied coalitions). Without these, the planner would not correctly distinguish contested vs. uncontested scenarios, and tests requiring competitor-motivated Bribe/Threaten selection would not produce the expected plan structures.

## Outcome

- **Completion date**: 2026-03-18
- **What changed**: Added 4 integration tests to `crates/worldwake-ai/src/goal_model.rs` test module: `planner_selects_bribe_plan`, `planner_selects_threaten_plan`, `planner_selects_travel_then_bribe`, `planner_rejects_threaten_against_high_courage`.
- **Deviations**: Rivals are placed at a different location than the actor in all tests. Without this, the planner shortcuts by bribing/threatening the rival directly (1-step plan), bypassing the intended multi-step coalition-building. This is correct planner behavior but defeats the test's purpose of verifying multi-step Bribe/Threaten plans.
- **Verification**: `cargo test -p worldwake-ai` (all pass), `cargo clippy --workspace` (clean).
