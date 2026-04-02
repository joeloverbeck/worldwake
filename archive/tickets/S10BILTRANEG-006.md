# S10BILTRANEG-006: Golden supply chain tests

**Status**: ✅ COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: None
**Deps**: S10BILTRANEG-004, S10BILTRANEG-005

## Problem

The full supply chain golden test (merchant restocks → consumer trades → consumer eats) has been blocked since the trade system lacked price negotiation. The golden_supply_chain.rs file header (lines 14-19) explicitly states: "The full end-to-end chain (restock → trade → consumption in one simulation) is blocked on `specs/S10-bilateral-trade-negotiation.md`." With tickets 001-005 implementing the negotiation protocol, this test can now be created to validate the complete emergent supply chain.

## Assumption Reassessment (2026-04-02)

1. `golden_supply_chain.rs` at `crates/worldwake-ai/tests/golden_supply_chain.rs` exists (1622 lines). Contains helper functions `run_merchant_restock_with_traces` and `run_consumer_trade_with_traces` but no full supply chain test function.
2. `PlanningBudget::default()` at `crates/worldwake-ai/src/budget.rs:20-37` has `max_node_expansions: 224`, `beam_width: 8`. The spec requires the test to pass with these defaults.
3. `default_trade_disposition()` at `golden_supply_chain.rs:41-49` uses `initial_offer_bias: pm(500)`, `concession_rate: pm(100)`, `negotiation_round_ticks: nz(4)`, `demand_memory_retention_ticks: 48`, `market_presence_ticks: nz(30)`. After ticket 001, this must also include `rejection_escalation_rate`.
4. The live merchant contract is facility-based, not place-based. Any new full-chain golden must create a real merchant facility and set `MerchandiseProfile.home_facility` to that facility entity, not to `GeneralStore` directly.
5. `enterprise_trade_disposition()` at line ~51-56 overrides only `demand_memory_retention_ticks: 240`. The merchant uses this — with enterprise_weight: pm(900), the merchant should require a negotiated price > 1 coin in the full-chain scenario.
6. Golden tests require `PerceptionProfile` on agents that need to observe post-production output. The existing restock helper already follows that pattern.
7. `hash_world` and `hash_event_log` are used for deterministic replay verification (existing pattern in other golden tests).
8. `verify_authoritative_conservation` and `verify_live_lot_conservation` are the conservation check functions used in existing golden tests.
9. The file currently contains helper runners but no `#[test]` entry points. This ticket must add actual golden tests, not just more dormant helpers.
10. The test must validate: (a) merchant restocks from orchard into home facility custody, (b) consumer observes merchant stock at the store, (c) consumer negotiates and trades at price > 1, (d) consumer eats to reduce hunger, (e) conservation holds throughout.

## Architecture Check

1. The golden test follows the established pattern: setup world with agents/places/items → run simulation for N ticks → assert decision traces and world state. No new test infrastructure needed.
2. The replay test follows the existing pattern: run twice with same seed, compare `hash_world` and `hash_event_log`.
3. No backward-compatibility shims. This is a new test, not a modification of existing ones.

## Verification Layers

1. Consumer trades at negotiated price > 1 coin -> action trace (TradeAgreed observation with quantity > 1)
2. Full supply chain completes within tick budget -> golden E2E (simulation completes without hanging)
3. Conservation holds -> authoritative world state (`verify_authoritative_conservation`, `verify_live_lot_conservation`)
4. Deterministic replay -> state hash comparison (`hash_world`, `hash_event_log`)
5. Existing segment tests still pass -> regression (run_merchant_restock_with_traces, run_consumer_trade_with_traces)

## What to Change

### 1. Create `test_full_supply_chain` golden test

Write a `#[test]` function that:

1. Sets up a world with:
   - Orchard place (with apple resource source)
   - General Store place (market)
   - Road connecting them
   - Merchant agent at General Store with `enterprise_trade_disposition()`, `enterprise_weight: pm(900)`, `PerceptionProfile`, and a real merchant facility whose entity id is stored in `MerchandiseProfile.home_facility`
   - Consumer agent at General Store with `default_trade_disposition()`, hunger need, coins, `PerceptionProfile`
   - Apple trees/resources at orchard

2. Runs the simulation with `PlanningBudget::default()` (224 expansions, beam width 8) for a sufficient tick budget (e.g., 200-500 ticks).

3. Asserts:
   - Merchant traveled to orchard and returned with apples (via decision traces)
   - Consumer observed merchant's apples at the store (via belief/perception or resulting local trade selection)
   - Consumer initiated trade with merchant
   - Trade completed at an agreed price (via `TradeAgreed` observation in consumer's demand memory, quantity > 1)
   - Consumer ate an apple (hunger decreased)
   - Conservation holds: `verify_authoritative_conservation`, `verify_live_lot_conservation`

### 2. Create `test_full_supply_chain_replay` golden test

Write a `#[test]` function that:
1. Runs `test_full_supply_chain` logic twice with the same seed.
2. Compares `hash_world` and `hash_event_log` between runs.
3. Asserts both are identical (deterministic replay).

### 3. Correct the stale supply-chain setup to live merchant architecture

- add or reuse a local helper to create a merchant facility in `golden_supply_chain.rs`
- store that facility entity in `MerchandiseProfile.home_facility` for the full-chain merchant
- update the stale file header comment so it no longer claims the end-to-end chain is still blocked once the new scenario lands

### 4. Verify the existing helper runners still work as supporting coverage

The existing `run_merchant_restock_with_traces` and `run_consumer_trade_with_traces` helpers should continue to represent valid segment coverage, but they should not be promoted into duplicate top-level goldens unless they prove a distinct contract from the new full-chain scenario.

## Files to Touch

- `crates/worldwake-ai/tests/golden_supply_chain.rs` (modify — add `test_full_supply_chain`, `test_full_supply_chain_replay`, update disposition helpers for `rejection_escalation_rate` if not already done in ticket 001)
- `crates/worldwake-ai/tests/golden_supply_chain.rs` (modify — add a local merchant-facility helper or equivalent setup so the new scenario uses facility-based home-market identity)

## Out of Scope

- New golden tests beyond the full supply chain (gap analysis is a separate concern)
- Changes to `PlanningBudget::default()`
- Changes to the negotiation protocol itself
- Segment test modifications (they should pass as-is)

## Acceptance Criteria

### Tests That Must Pass

1. `test_full_supply_chain` passes with `PlanningBudget::default()` — consumer successfully trades with merchant at mutually acceptable price
2. `test_full_supply_chain_replay` passes — deterministic replay preserved
3. The stale supply-chain file header no longer claims the full end-to-end chain is blocked on S10 once the scenario lands
4. All existing golden tests pass unchanged: `cargo test -p worldwake-ai` — no regressions
5. Conservation invariants hold throughout the full supply chain simulation
6. Full suite: `cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings`

### Invariants

1. `PlanningBudget::default()` is not modified (224 expansions, beam width 8).
2. Deterministic replay: same seed, same inputs → identical `hash_world` and `hash_event_log`.
3. Conservation: coins + commodities neither created nor destroyed.
4. Existing segment tests (`run_merchant_restock_with_traces`, `run_consumer_trade_with_traces`) continue to pass — the negotiation protocol does not break low-reservation-price trades.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_supply_chain.rs::test_full_supply_chain` — E2E golden test for full merchant restock → consumer trade → consumption chain
2. `crates/worldwake-ai/tests/golden_supply_chain.rs::test_full_supply_chain_replay` — deterministic replay verification

### Commands

1. `cargo test -p worldwake-ai --test golden_supply_chain -- test_full_supply_chain` — targeted new test
2. `cargo test -p worldwake-ai --test golden_supply_chain` — all supply chain golden tests
3. `cargo test -p worldwake-ai` — all AI golden tests (regression check)
4. `cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings` — full suite

## Outcome

Completed: 2026-04-02

What changed:
- added a local facility helper in `crates/worldwake-ai/tests/golden_supply_chain.rs` so the full-chain merchant setup uses the live facility-based `MerchandiseProfile.home_facility` contract
- added `Scenario 88` and its deterministic replay companion: `golden_full_supply_chain_negotiated_restock_to_consumption` and `golden_full_supply_chain_negotiated_restock_to_consumption_replays_deterministically`
- updated the stale file header in `golden_supply_chain.rs` so it no longer claims the full supply chain is still blocked on S10
- proved the full merchant restock -> negotiated trade -> consumption chain under `PlanningBudget::default()` with conservation assertions and a `TradeAgreed` price check above one coin

Deviations from original plan:
- the critical scenario calibration was local belief seeding, not production code: the merchant needed direct local home-facility beliefs at `GeneralStore` so the restock return could transition into lawful sell-side behavior before the consumer trade
- the existing segment runners remained as supporting helper coverage and were not promoted into duplicate top-level goldens

Verification results:
- `cargo test -p worldwake-ai --test golden_supply_chain -- golden_full_supply_chain_negotiated_restock_to_consumption --nocapture`
- `cargo test -p worldwake-ai --test golden_supply_chain`
- `cargo test -p worldwake-ai`
- `cargo test --workspace`
- `cargo clippy --workspace --all-targets -- -D warnings`
