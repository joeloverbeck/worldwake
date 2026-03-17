# S09TRAAWAPLASEA-005: Enable full supply chain golden tests with default budget

**Status**: NOT IMPLEMENTED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: None — test configuration changes only
**Deps**: S09TRAAWAPLASEA-003 (A* heuristic), S09TRAAWAPLASEA-004 (travel pruning)

## Problem

The full 3-agent supply chain golden tests (`test_full_supply_chain` and `test_full_supply_chain_replay`) in `golden_supply_chain.rs` are currently `#[ignore]`d because the plan search exhausts its budget at hub nodes. With A* heuristic and travel pruning in place (tickets 001-004), these tests should pass with the default `PlanningBudget` (512 expansions, beam width 8). This ticket removes the `#[ignore]` annotations and reduces the budget from 1024 back to default.

## Assumption Reassessment (2026-03-17)

1. `test_full_supply_chain` and `test_full_supply_chain_replay` are both `#[ignore]`d in `crates/worldwake-ai/tests/golden_supply_chain.rs` — confirmed.
2. The combined test currently uses `max_node_expansions: 1024` (line 616) — confirmed. This was a workaround for the budget exhaustion.
3. `PlanningBudget::default()` has `max_node_expansions: 512` (budget.rs:5-29) — confirmed.
4. The segment tests (merchant restock, consumer trade) already pass with default budget — confirmed.
5. `test_full_supply_chain_replay` validates deterministic replay by running the simulation twice and comparing event logs — the replay invariant must hold after the search changes.

## Architecture Check

1. This is purely a test configuration change — no production code is modified.
2. If the tests fail, it means tickets 001-004 did not sufficiently improve search efficiency, and those tickets need revisiting — not this one.

## What to Change

### 1. Remove `#[ignore]` from both tests

Remove the `#[ignore]` attribute from `test_full_supply_chain` and `test_full_supply_chain_replay`.

### 2. Replace custom budget with default

In the combined test setup, replace the custom `PlanningBudget { max_node_expansions: 1024, .. }` with `PlanningBudget::default()`. If the budget is constructed inline, change it to use `PlanningBudget::default()` or remove the override entirely.

### 3. Update comments

Remove or update comments referencing budget exhaustion as a blocking issue (e.g., lines 611-616 and the header comment at lines 14-16 about SUPPLYCHAINFIX-001 blocking the full chain).

## Files to Touch

- `crates/worldwake-ai/tests/golden_supply_chain.rs` (modify — remove `#[ignore]`, change budget, update comments)

## Out of Scope

- Modifying `search.rs`, `planning_snapshot.rs`, or `goal_model.rs` (tickets 001-004)
- Adding new golden tests
- Changing `PlanningBudget` default values
- Modifying any production code
- Changing other golden test files (combat, production, trade, etc.)
- Budget auto-scaling (explicitly out of scope per spec)

## Acceptance Criteria

### Tests That Must Pass

1. `test_full_supply_chain` passes with `PlanningBudget::default()` (512 expansions, beam width 8).
2. `test_full_supply_chain_replay` passes — deterministic replay is preserved after the search algorithm changes.
3. All other golden tests continue to pass: `cargo test -p worldwake-ai` — no regressions.
4. `cargo test --workspace` — no regressions across all crates.
5. `cargo clippy --workspace` — clean.

### Invariants

1. `PlanningBudget::default()` is not modified — the budget remains 512 expansions, beam width 8.
2. Deterministic replay holds — same seed, same inputs produce identical event logs.
3. All existing (non-ignored) golden tests continue to pass with unchanged behavior.
4. The full supply chain test exercises: merchant restock (travel to source, harvest, travel back) → merchant trade with consumer → consumer consumption — the complete 3-agent economic loop.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_supply_chain.rs` — un-ignore `test_full_supply_chain` and `test_full_supply_chain_replay`, reduce budget to default

### Commands

1. `cargo test -p worldwake-ai golden_supply_chain`
2. `cargo test -p worldwake-ai` (all golden tests)
3. `cargo test --workspace && cargo clippy --workspace`

## Outcome

**Date**: 2026-03-17
**Result**: Not implemented — investigation revealed the test failure is not caused by budget exhaustion (resolved by S09 tickets 001-004) but by a missing price negotiation mechanism in the trade system. The merchant correctly rejects the consumer's fixed 1-coin offer as `InsufficientPayment` given its `enterprise_weight: pm(900)` and scarce stock. Perception (E14) works correctly — the consumer observes the merchant's return. The plan search works correctly with default budget. The blocker is that `enumerate_trade_payloads` hardcodes all offers at `Quantity(1)` with no mechanism for bidding higher. A new spec `specs/S10-bilateral-trade-negotiation.md` was created to address this architectural gap. The `#[ignore]` comments on both tests were updated to reference S10 as the blocker.
