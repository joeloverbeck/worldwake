# E16DPOLPLAN-006: Integration tests — planner finds Bribe/Threaten plans

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: None
**Deps**: E16DPOLPLAN-005

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
1. **Planner selects Bribe plan**: agent at jurisdiction with goods, bribable target, vacant office -> plan contains `PlannerOpKind::Bribe` + `DeclareSupport`
2. **Planner selects Threaten plan**: agent at jurisdiction, high attack_skill, low-courage target -> plan contains `Threaten`
3. **Planner selects Travel + Bribe**: agent NOT at jurisdiction but has goods -> plan starts with `Travel` then includes `Bribe` + `DeclareSupport`
4. **Planner rejects Threaten against high-courage**: target courage exceeds attack_skill -> Threaten NOT in plan

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
