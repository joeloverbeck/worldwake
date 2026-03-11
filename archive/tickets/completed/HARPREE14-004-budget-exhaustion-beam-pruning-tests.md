# HARPREE14-004: Budget exhaustion and beam pruning tests

**Status**: ✅ COMPLETED
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
5. `beam_width` is currently applied by truncating each expanded node's sorted successor list before those successors are pushed onto the frontier; the implementation does **not** maintain a globally bounded frontier -- confirmed
6. Because of that implementation, asserting "frontier size respects beam width" would test the wrong contract. The stable externally visible contract is which branches survive pruning and whether search still terminates within the configured expansion budget -- confirmed

## Architecture Check

1. Pure test addition. No production code changes needed.
2. Tests should target externally observable planner behavior, not internal container size. That is cleaner and more durable than baking the current frontier representation into the test suite.
3. Testing that narrow beams can prune away a viable but lower-priority branch is more beneficial than testing that a greedy beam still succeeds. The former proves the architectural tradeoff the beam introduces; the latter only proves one happy path.
4. No backward-compatibility aliases or helper APIs are warranted here. If the search architecture changes later, the tests should continue to describe behavior, not implementation trivia.

## What to Change

### 1. Test `beam_width=1` prunes away a slower viable branch

Build a branching scenario where the cheapest immediate successor is a dead end, while a slightly slower sibling successor leads to a valid plan. Verify:

- `beam_width=1` returns `None` because the viable branch is pruned
- a wider beam keeps the viable branch and finds the plan

This is a stronger and more truthful regression check than "greedy search still succeeds."

### 2. Test `beam_width` pruning keeps only the top-ranked successors from an expansion

Build a scenario with more viable sibling successors than the beam allows and verify the returned plan changes in the expected deterministic way when the beam widens. The assertion should be about which branch survives, not about the internal frontier length.

### 3. Test interaction: small `max_node_expansions` with large `beam_width`

Verify `search_plan()` returns `None` once `max_node_expansions` is exhausted even in a high-branching scenario where `beam_width` is large enough to keep many successors alive.

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

1. New test: `test_beam_width_1_prunes_viable_slower_branch` -- a narrow beam can prune away the only viable branch
2. New test: `test_beam_width_widening_keeps_more_successors` -- widening the beam changes which sibling branches survive pruning in a deterministic scenario
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

1. `crates/worldwake-ai/src/search.rs` (test module) -- four new tests covering observable beam pruning behavior and budget edge cases

### Commands

1. `cargo test -p worldwake-ai search` (targeted)
2. `cargo test --workspace`
3. `cargo clippy --workspace`

## Outcome

Completion date: 2026-03-11

What actually changed:
- Corrected the ticket scope before implementation so it matches the real search architecture: `beam_width` prunes per-node successor sets, not the global frontier.
- Added four search tests covering observable pruning outcomes and budget edge cases in `crates/worldwake-ai/src/search.rs`.
- Verified that the current planner implementation already satisfies the corrected ticket without production code changes.

Deviations from original plan:
- Did not add a test asserting that the frontier size never exceeds `beam_width`; reassessment showed that would encode a false architectural assumption about the current search implementation.
- Replaced the original "greedy search still finds a plan" emphasis with stronger regression coverage that proves narrow beams can prune away viable slower branches and that widening the beam deterministically restores them.
- Kept the change set test-only because the search architecture itself is already clean enough for this hardening step. The right improvement here was coverage, not algorithm churn.

Verification results:
- `cargo test -p worldwake-ai search` passed.
- `cargo test --workspace` passed.
- `cargo clippy --workspace` passed.
