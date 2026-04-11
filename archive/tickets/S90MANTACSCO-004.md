# S90MANTACSCO-004: Tests for D3

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: None
**Deps**: archive/tickets/S90MANTACSCO-003.md

## Problem

D3's production substrate is now live, but it still lacks the focused planner-level threshold assertion that proves candidate-count overflow returns `BudgetExhausted` at the intended search boundary. D1's focused exploration tests landed in S90MANTACSCO-001 during implementation, and D2's strategic-classification plus fail-fast coverage landed in S90MANTACSCO-002.

## Assumption Reassessment (2026-04-11)

1. Test file confirmed at `crates/worldwake-ai/src/search/tests.rs`. Existing S88/S89 tests live here.
2. `GroundedGoal` at `goal_model.rs:2095-2100` has `evidence_places: BTreeSet<EntityId>` and `evidence_entities: BTreeSet<EntityId>`. Test construction requires populating these fields.
3. `PlanSearchResult::BudgetExhausted { expansions_used: u16 }` at `mod.rs:170` remains the intended D3 assertion surface.
4. Reassessment correction: `CognitiveProfile::max_candidates_per_expansion` now exists on the live branch and 003 is archived. The only remaining owned delta is the focused `search_plan` threshold proof.

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

## Outcome

Completion date: 2026-04-11

Implemented the remaining D3 focused coverage by adding `search_candidate_safety_valve_triggers_at_threshold` in `crates/worldwake-ai/src/search/tests.rs`.

What actually changed:
1. Added a focused `search_plan`-level test that sets `max_candidates_per_expansion = 0` on the live `CognitiveProfile` boundary
2. The test uses a minimal hungry-consume scenario with at least one post-filter candidate so the safety valve deterministically returns `PlanSearchResult::BudgetExhausted { expansions_used: 1 }`

Deviation from the original ticket draft:
1. Reassessment narrowed the live scope to test-only work because `archive/tickets/S90MANTACSCO-003.md` had already landed the D3 substrate
2. The focused threshold assertion uses `0` rather than the ticket’s illustrative `5`; the contract under test is the threshold boundary, not a specific numeric demo

Verification completed:
1. `cargo test -p worldwake-ai -- search_candidate_safety_valve`
2. `cargo test -p worldwake-ai`
3. `cargo clippy --workspace --all-targets -- -D warnings`

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/search/tests.rs::search_candidate_safety_valve_triggers_at_threshold` — confirms safety valve

### Commands

1. `cargo test -p worldwake-ai -- search_candidate_safety_valve`
2. `cargo test -p worldwake-ai`
3. `cargo clippy --workspace --all-targets -- -D warnings`
