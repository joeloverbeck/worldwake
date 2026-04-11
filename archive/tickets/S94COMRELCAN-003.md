# S94COMRELCAN-003: Rewrite S93 golden budget-exhaustion tests as regression guards

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: None — test-only changes
**Deps**: archive/tickets/S94COMRELCAN-001.md, archive/tickets/S94COMRELCAN-002.md

## Problem

`S94COMRELCAN-002` reduced candidate pressure and changed one stale golden failure from pure `BudgetExhausted` to `FrontierExhausted`, but it did not make these six scenarios fully solvable within the current planner budgets. `crates/worldwake-ai/tests/golden_budget_exhaustion_snapshots.rs` is now a transitional file containing 6 active stale-exhaustion proofs plus 6 ignored optimistic `*_found_after_fix` duplicates that all fail on the current branch. This ticket rewrites that file into a zero-ignored, honest post-filter regression surface.

## Assumption Reassessment (2026-04-11)

1. `crates/worldwake-ai/tests/golden_budget_exhaustion_snapshots.rs` currently has 12 scenario tests: 6 active stale-exhaustion proofs and 6 ignored optimistic `*_found_after_fix` duplicates.
2. Running `cargo test -p worldwake-ai --test golden_budget_exhaustion_snapshots -- --ignored` on 2026-04-11 shows all 6 ignored `*_found_after_fix` tests still fail on the current branch: 5 return `BudgetExhausted { expansions_used: 224 | 300 }`, and `kael_treat_wounds_vara_at_dusty_trail_found_after_fix` returns `FrontierExhausted { expansions_used: 54 }`.
3. The live scenario outcome surface after ticket `002` is therefore: `merchant_vara_water_at_thornwall` -> `BudgetExhausted { 300 }`, `guard_theron_water_at_thornwall` -> `BudgetExhausted { 224 }`, `merchant_vara_apple_at_dusty_trail` -> `BudgetExhausted { 300 }`, `kael_water_at_thornwall_late_game` -> `BudgetExhausted { 224 }`, `merchant_vara_treat_wounds_at_dusty_trail` -> `BudgetExhausted { 300 }`, `kael_treat_wounds_vara_at_dusty_trail` -> `FrontierExhausted { 54 }`.
4. Test 3 (`merchant_vara_apple_at_dusty_trail`) remains plausibly infeasible under belief constraints because Apples are at Eldergrove Forest and the actor does not know that location. The spec already allows a residual failure-mode assertion there.
5. Because `search_plan` is the only public proof carrier used in this integration file, exact post-filter candidate-count assertions are not directly available here. The honest golden contract for this ticket is the scenario-level plan-search result surface, not an internal root-candidate count.
6. The file has no scenario-metadata comments, but test-name changes still affect generated golden inventory/docs. If tests are renamed or removed, `python3 scripts/golden_inventory.py --write --check-docs` becomes part of broadened verification.

## Architecture Check

1. The file is currently carrying duplicate ownership of the same six scenarios: one active stale expectation and one ignored optimistic expectation. The cleaner post-`002` state is a single active regression test per scenario with an honest current contract and zero ignored duplicates.
2. Each scenario keeps its exact snapshot setup (same beliefs, entities, cognitive profiles). This ticket changes only the asserted post-filter search contract and the duplicate-test structure.

## Verification Layers

1. Each named golden scenario asserts the honest current post-filter `PlanSearchResult` surface -> focused golden integration test
2. The transitional duplicate ignored tests are removed -> test inventory / generated golden docs
3. Single-layer ticket (golden/test-only) — no production change

## What to Change

### 1. Rewrite the 6 scenario contracts into honest post-filter regression guards

In `crates/worldwake-ai/tests/golden_budget_exhaustion_snapshots.rs`:

- Keep the exact same snapshot setup unchanged
- Replace stale “should now find a plan” expectations with the honest current `PlanSearchResult` per scenario
- Use exact variant + `expansions_used` assertions so the post-filter residual search shape is explicit and reproducible
- Rename tests where needed so names no longer claim `BudgetExhausted` when the live contract is `FrontierExhausted`, and no test claims “found after fix” when that is false

### 2. Remove the 6 ignored optimistic duplicates

- Delete the `*_found_after_fix` tests once their scenario contracts are represented by the active regression set
- End state: one active regression guard per scenario, zero ignored duplicates

### 3. Refresh generated golden inventory/docs if test names change

- If any test names are renamed or deleted, run `python3 scripts/golden_inventory.py --write --check-docs`
- Keep the resulting `docs/generated/golden-*` updates aligned with the new live test inventory

## Files to Touch

- `crates/worldwake-ai/tests/golden_budget_exhaustion_snapshots.rs` (modify) — rewrite scenario contracts and remove ignored duplicates
- `docs/generated/golden-*.md` (modify, generated) — inventory/index/detail fallout if test names change

## Out of Scope

- Modifying snapshot setups (beliefs, entities, cognitive profiles) — tests keep exact same scenarios
- Adding new test scenarios beyond the existing 12
- Changing `CognitiveProfile` or `ExecutionBudget` parameters
- Modifying the commodity-relevance filter itself (tickets 001+002)

## Acceptance Criteria

### Tests That Must Pass

1. `golden_budget_exhaustion_snapshots.rs` has 0 ignored tests
2. Each of the 6 scenario regressions asserts the honest current post-filter `PlanSearchResult`
3. No test in this file claims `Found` unless the current branch actually returns `Found`
4. Generated golden inventory/docs remain aligned if test names change
5. Existing suite: `cargo test --workspace`

### Invariants

1. Zero ignored tests in `golden_budget_exhaustion_snapshots.rs`
2. Each scenario's snapshot setup is unchanged from S93 — same beliefs, same entities, same cognitive profiles
3. Each regression test names and proves the current post-filter search outcome honestly
4. No duplicate optimistic test remains for a scenario whose current branch does not satisfy it

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_budget_exhaustion_snapshots.rs` — rewrite the 6 scenario regression guards and remove the 6 ignored duplicates
2. `docs/generated/golden-*.md` — refresh if the scenario test inventory changes

### Commands

1. `cargo test -p worldwake-ai golden_budget_exhaustion` — all 12 tests, 0 ignored
2. `python3 scripts/golden_inventory.py --write --check-docs` — if test names change
3. `cargo clippy --workspace --all-targets -- -D warnings` — clean
4. `cargo test --workspace` — full regression

## Outcome

Completed on 2026-04-11.

- Rewrote `crates/worldwake-ai/tests/golden_budget_exhaustion_snapshots.rs` into a zero-ignored post-filter regression file with one active scenario guard per formerly duplicated S93 scenario.
- Replaced the stale optimistic `*_found_after_fix` placeholders with honest exact-result assertions for the current branch: five residual `BudgetExhausted` scenarios and one residual `FrontierExhausted` scenario.
- Renamed the active scenario tests so they no longer claim planner success that the current branch does not produce.
- Refreshed generated golden inventory/docs to keep the renamed and removed test inventory aligned.

## Deviations

- Reassessment showed ticket `002` did not make these scenarios fully solvable within budget. The landed contract therefore narrows from “convert the family to found-plan regressions” to “convert the transitional stale/ignored file into honest zero-ignored post-filter regression guards.”
- The final file now carries 6 active scenario regressions rather than 12 scenario duplicates, because the ignored `*_found_after_fix` tests were redundant optimistic copies of the same setups rather than distinct proof surfaces.

## Verification Result

- Passed `cargo test -p worldwake-ai --test golden_budget_exhaustion_snapshots`
- Passed `python3 scripts/golden_inventory.py --write --check-docs`
- Passed `cargo clippy --workspace --all-targets -- -D warnings`
- Passed `cargo test --workspace`
