# S94COMRELCAN-003: Rewrite S93 golden budget-exhaustion tests as regression guards

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: None — test-only changes
**Deps**: archive/tickets/S94COMRELCAN-001.md, archive/tickets/S94COMRELCAN-002.md

## Problem

After the commodity-relevance filter lands (tickets 001+002), the 6 active S93 golden tests that assert `BudgetExhausted` should now find plans within budget. The 6 ignored tests that were waiting for the fix can be un-ignored. All 12 tests need to be converted from budget-exhaustion proofs into regression guards that ensure these specific scenarios never regress back to budget exhaustion.

## Assumption Reassessment (2026-04-11)

1. `crates/worldwake-ai/tests/golden_budget_exhaustion_snapshots.rs` exists with 12 tests: 6 active stale-exhaustion proofs and 6 ignored `*_found_after_fix` tests. One active test (`kael_treat_wounds_vara_at_dusty_trail_budgets_exhaust`) now already accepts `BudgetExhausted` or `FrontierExhausted` after ticket `002` reduced candidate pressure.
2. Active tests: `merchant_vara_water_at_thornwall_budgets_exhaust`, `guard_theron_water_at_thornwall_budgets_exhaust`, `merchant_vara_apple_at_dusty_trail_budgets_exhaust`, `kael_water_at_thornwall_late_game_budgets_exhaust`, `merchant_vara_treat_wounds_at_dusty_trail_budgets_exhaust`, `kael_treat_wounds_vara_at_dusty_trail_budgets_exhaust`. Their current expectation surface is no longer uniform pure-`BudgetExhausted`, so this ticket owns normalizing them into honest post-filter regression guards.
3. Ignored tests: `merchant_vara_water_at_thornwall_found_after_fix`, `guard_theron_water_at_thornwall_found_after_fix`, `merchant_vara_apple_at_dusty_trail_found_after_fix`, `kael_water_at_thornwall_late_game_found_after_fix`, `merchant_vara_treat_wounds_at_dusty_trail_found_after_fix`, `kael_treat_wounds_vara_at_dusty_trail_found_after_fix`.
4. Test 3 (`merchant_vara_apple_at_dusty_trail`): Apples are at Eldergrove Forest (2 hops away) and the agent doesn't know about Eldergrove. The commodity filter reduces irrelevant candidates but the plan may remain infeasible under belief constraints. Spec allows asserting the correct failure mode if it still budget-exhausts.
5. S93 candidate count baselines (from spec): 1483, 2085, 2511, 2657, 5739, 4151. Post-filter, candidate counts should be 60-90% lower per spec design goals.

## Architecture Check

1. Converting budget-exhaustion proofs to regression guards is the natural lifecycle: S93 proved the problem exists, S94 fixes it, the tests evolve to prevent regression. No backward-compatibility shims — the old assertions are replaced, not wrapped.
2. Each test keeps its exact snapshot setup (same beliefs, entities, cognitive profiles) — only the assertions change. This preserves the test's value as a specific-scenario regression guard.

## Verification Layers

1. Plans found within budget for commodity goals → `search_plan` returns `Found` (not `BudgetExhausted`) in each test
2. Candidate counts reduced from S93 baselines → expansion count assertions below budget limits
3. Plans contain commodity-relevant actions → plan action chain validation
4. Goal postconditions met (for phase 2 tests) → thirst/hunger/wounds state verification
5. Single-layer ticket (test-only) — no cross-system verification needed

## What to Change

### 1. Convert 6 active `_budgets_exhaust` tests to regression guards

In `crates/worldwake-ai/tests/golden_budget_exhaustion_snapshots.rs`:

For each of the 6 active tests:
- Replace the stale exhaustion assertion with the honest post-filter contract for that scenario: `Found` where the scenario is now solvable, or the correct residual `FrontierExhausted` / `BudgetExhausted` contract where the scenario remains unsolved
- Add assertion that expansion count is below the budget limit (e.g., `< 224` or `< 300` depending on the agent's cognitive profile)
- Add assertion that the returned plan contains commodity-relevant actions
- Keep the exact same snapshot setup unchanged
- Rename tests from `*_budgets_exhaust` to `*_finds_plan` or similar to reflect new semantics

### 2. Un-ignore and verify 6 phase 2 tests

For each of the 6 ignored tests:
- Remove `#[ignore = "..."]` attribute
- Verify the test's existing assertions (plan `Found`, action chain execution, goal postcondition)
- If any test needs adjustment to match the actual plan shape produced by the filter, update accordingly

### 3. Handle Test 3 (merchant_vara_apple_at_dusty_trail)

If `merchant_vara_apple_at_dusty_trail` still budget-exhausts or frontier-exhausts after the commodity filter:
- Convert the `_budgets_exhaust` test to assert the correct failure mode (`BudgetExhausted` with significantly fewer candidates than the S93 baseline of 2511, or `FrontierExhausted`)
- Convert the `_found_after_fix` test to assert the correct failure mode with documentation explaining the scenario is infeasible under belief constraints (agent doesn't know about Eldergrove Forest)
- Add a comment documenting why this scenario is genuinely infeasible, not a filter bug

## Files to Touch

- `crates/worldwake-ai/tests/golden_budget_exhaustion_snapshots.rs` (modify) — all 12 tests

## Out of Scope

- Modifying snapshot setups (beliefs, entities, cognitive profiles) — tests keep exact same scenarios
- Adding new test scenarios beyond the existing 12
- Changing `CognitiveProfile` or `ExecutionBudget` parameters
- Modifying the commodity-relevance filter itself (tickets 001+002)

## Acceptance Criteria

### Tests That Must Pass

1. All 12 tests in `golden_budget_exhaustion_snapshots.rs` pass with 0 ignored
2. At least 10 of 12 tests assert `Found` (the known infeasible pair and any other still-unsolved residual scenario may instead assert the correct post-filter failure mode)
3. Candidate counts in regression guard tests show significant reduction from S93 baselines
4. Decision traces for affected goals show `CommodityIrrelevant` filter entries
5. Existing suite: `cargo test --workspace`

### Invariants

1. Zero ignored tests in `golden_budget_exhaustion_snapshots.rs`
2. Each test's snapshot setup is unchanged from S93 — same beliefs, same entities, same cognitive profiles
3. Regression guard tests assert `Found` with expansion count below budget limit
4. Phase 2 tests verify goal postconditions (thirst/hunger decreases, wounds treated)

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_budget_exhaustion_snapshots.rs` — modify all 12 tests: 6 converted from budget-exhaustion proofs to regression guards, 6 un-ignored and verified

### Commands

1. `cargo test -p worldwake-ai golden_budget_exhaustion` — all 12 tests, 0 ignored
2. `cargo clippy --workspace --all-targets -- -D warnings` — clean
3. `cargo test --workspace` — full regression
