# HARPREE14-004: Budget exhaustion and beam pruning tests

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: None
**Deps**: None (Wave 1, independent)
**Spec Reference**: HARDENING-PRE-E14.md, HARDEN-D02

## Problem

The search module has tests for `max_plan_depth` and `max_node_expansions` exhaustion, but does not test `beam_width` pruning behavior or the interaction between budget parameters. These are important edge cases for the GOAP planner.

## Assumption Reassessment (2026-03-11)

1. `PlanningBudget` has `beam_width`, `max_plan_depth`, `max_node_expansions` fields -- confirmed
2. `search_plan()` uses these budget parameters to control search -- confirmed
3. Existing tests cover `max_plan_depth` and `max_node_expansions` -- confirmed
4. No tests for `beam_width` pruning behavior exist -- confirmed

## Architecture Check

1. Pure test addition. No production code changes needed.
2. Tests exercise existing budget enforcement logic from new angles.

## What to Change

### 1. Test `beam_width=1` forces greedy search

Set `beam_width=1`, verify that only the single best successor survives at each expansion step. The search should still find a plan if one exists greedily.

### 2. Test `beam_width` pruning discards lower-priority successors

Set a small `beam_width` (e.g., 2) in a scenario with many branching affordances, verify the frontier never exceeds the beam width.

### 3. Test interaction: small `max_node_expansions` with large `beam_width`

Verify the search terminates within `max_node_expansions` even when `beam_width` is large and many successors are generated.

### 4. Test `max_plan_depth=0` edge case

Verify `search_plan()` returns `None` immediately when `max_plan_depth=0`.

## Files to Touch

- `crates/worldwake-ai/src/search.rs` (modify -- add tests to test module)

## Out of Scope

- Changing search algorithm behavior
- Modifying `PlanningBudget` fields or defaults
- Changing `SearchNode` or frontier management logic
- Any production code changes

## Acceptance Criteria

### Tests That Must Pass

1. New test: `test_beam_width_1_greedy_search` -- beam_width=1 still finds a valid plan
2. New test: `test_beam_width_prunes_low_priority` -- frontier size respects beam_width
3. New test: `test_max_expansions_with_large_beam` -- terminates within budget
4. New test: `test_max_plan_depth_0_returns_none` -- immediate None return
5. All existing search tests pass unchanged
6. `cargo clippy --workspace` -- no new warnings

### Invariants

1. No production code changes
2. Existing search behavior unaffected
3. Golden e2e hashes identical

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/search.rs` (test module) -- four new tests covering beam pruning and budget edge cases

### Commands

1. `cargo test -p worldwake-ai search` (targeted)
2. `cargo test --workspace`
3. `cargo clippy --workspace`
