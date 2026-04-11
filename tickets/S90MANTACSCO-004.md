# S90MANTACSCO-004: Tests for D3

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: None
**Deps**: S90MANTACSCO-003

## Problem

D3 introduces a candidate count safety valve that still needs focused test coverage to prevent future regressions. D1's focused exploration tests landed in S90MANTACSCO-001 during implementation, and D2's strategic-classification plus fail-fast coverage now lands in S90MANTACSCO-002.

## Assumption Reassessment (2026-04-11)

1. Test file confirmed at `crates/worldwake-ai/src/search/tests.rs`. Existing S88/S89 tests live here.
2. `GroundedGoal` at `goal_model.rs:2095-2100` has `evidence_places: BTreeSet<EntityId>` and `evidence_entities: BTreeSet<EntityId>`. Test construction requires populating these fields.
3. `PlanSearchResult::BudgetExhausted { expansions_used: u16 }` at `mod.rs:170` remains the intended D3 assertion surface.
4. `CognitiveProfile::max_candidates_per_expansion` does not exist yet; it remains owned by 003.

## Architecture Check

1. Focused unit tests on `from_strategic_step` and `search_plan` are the correct verification surface for planner-internal changes. No need for golden tests since no world state is affected.
2. No backwards-compatibility shims.

## Verification Layers

1. Safety valve → focused unit test: `search_plan` returns `BudgetExhausted` when candidate count exceeds threshold
2. Single-layer ticket: all tests are planner-internal focused coverage

## What to Change

### 1. `search_candidate_safety_valve_triggers_at_threshold`

**File**: `crates/worldwake-ai/src/search/tests.rs`

Set `max_candidates_per_expansion` to a low value (e.g., 5). Create a scenario with more candidates than the threshold. Assert `BudgetExhausted` is returned.

## Files to Touch

- `crates/worldwake-ai/src/search/tests.rs` (modify)

## Out of Scope

- Golden/E2E tests (planner-internal changes don't need world-level coverage)
- Observer re-runs (verification step in spec, not a test)
- Testing non-S90 behaviors

## Acceptance Criteria

### Tests That Must Pass

1. `search_candidate_safety_valve_triggers_at_threshold`
2. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. The new safety-valve test passes once D3 lands
2. Existing S88/S89/S90 D1-D2 tests pass unchanged
3. No omniscient queries in test setup — tests use `PlanningSnapshot`/`PlanningState` belief surface

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/search/tests.rs::search_candidate_safety_valve_triggers_at_threshold` — confirms safety valve

### Commands

1. `cargo test -p worldwake-ai -- search_candidate_safety_valve`
2. `cargo test -p worldwake-ai`
3. `cargo clippy --workspace --all-targets -- -D warnings`
