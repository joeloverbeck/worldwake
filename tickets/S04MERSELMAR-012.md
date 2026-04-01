# S04MERSELMAR-012: Remaining golden integration tests for merchant selling

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: None — test-only ticket
**Deps**: S04MERSELMAR-011

## Problem

S04MERSELMAR-011 implemented 5 of 12 golden integration tests for the merchant selling market-presence system. The remaining 7 scenarios cover buyer discovery evidence, autonomous SellCommodity emission, listing invalidation on seller departure/death, multi-step MoveCargo+SellCommodity chains, demand memory ranking verification, and planning state determinism.

## What to Change

### 1. Add remaining test scenarios to `golden_merchant_selling.rs`

All tests use the existing shared setup helpers (`seed_merchant`, `seed_buyer`) from S04MERSELMAR-011.

1. **buyer_discovers_listed_lots_not_unlisted_stock**: Setup merchant with two lots of the same commodity — one listed, one not. Seed buyer beliefs. Verify buyer's AcquireCommodity candidate evidence references only the listed lot (via decision trace).

2. **merchant_emits_sell_commodity_at_home_market**: Setup merchant at home_market with unlisted stock and demand memory. Verify SellCommodity candidate is emitted via decision trace (not just that staff_market starts — prove the candidate generation step directly).

3. **seller_departure_invalidates_listing**: Merchant starts staff_market (listing attached). Then manually move merchant to a different place. Run one tick of the trade system. Verify SaleListing is removed from the lot within one tick of departure.

4. **dead_seller_invalidates_listing**: Merchant starts staff_market (listing attached). Then kill the merchant (apply a fatal wound or archive the agent). Run one tick. Verify SaleListing is removed from the lot within one tick of death.

5. **move_cargo_then_sell_commodity**: Merchant at a non-home-market place with stock. Home market is a different place. Verify plan search finds a multi-step plan: Travel (to home_market) → StaffMarket. Alternatively: merchant has stock at a remote location, verify MoveCargo → Travel → StaffMarket sequence via decision trace plan shape.

6. **demand_memory_raises_sell_ranking**: Create two merchants with identical profiles except one has demand memory and one does not. Compare SellCommodity motive scores via decision trace ranking. Verify the merchant with demand memory has a higher motive. Verify neither overpowers a critical self-care goal (hunger at critical threshold).

7. **planning_state_preserves_listing_determinism**: Verify that planning snapshot and planning state produce identical listed-lot visibility for the same merchant scenario. This can be tested by running the same scenario twice with the same seed and asserting identical plan search results.

## Files to Touch

- `crates/worldwake-ai/tests/golden_merchant_selling.rs` (modify — add 7 test scenarios)

## Out of Scope

- Any implementation changes — this is test-only
- The 5 tests already implemented in S04MERSELMAR-011
- Inventory script fixes (separate concern)

## Acceptance Criteria

### Tests That Must Pass

1. All 7 new golden scenarios pass
2. All 5 existing golden merchant selling tests continue to pass
3. Decision traces confirm correct candidate generation and goal selection
4. World state checks confirm SaleListing invalidation on departure/death
5. Conservation invariants hold across all scenarios
6. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. Golden tests exercise full system stack — no mocked intermediate layers
2. Each test maps to a specific spec scenario
3. Tests are deterministic — same seed produces same results
4. Tests use `PerceptionProfile` on agents that need to observe events

## Test Plan

### New Tests

1. `crates/worldwake-ai/tests/golden_merchant_selling.rs` — 7 additional golden integration tests

### Commands

1. `cargo test -p worldwake-ai --test golden_merchant_selling`
2. `cargo clippy --workspace && cargo test --workspace`
