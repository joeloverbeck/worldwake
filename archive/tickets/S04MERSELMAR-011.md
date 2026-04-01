# S04MERSELMAR-011: Golden integration tests for merchant selling market presence

**Status**: ✅ COMPLETED (partial — 5 of 12 scenarios; remaining 7 deferred to follow-up)
**Priority**: MEDIUM
**Effort**: Large
**Engine Changes**: None — test-only ticket
**Deps**: S04MERSELMAR-003, S04MERSELMAR-004, S04MERSELMAR-005, S04MERSELMAR-006, S04MERSELMAR-007, S04MERSELMAR-008, S04MERSELMAR-009, S04MERSELMAR-010

## Problem

The S04 spec defines 12 test scenarios that validate the merchant selling market-presence system end-to-end. These golden integration tests exercise the full pipeline: candidate generation, plan search, action execution, listing lifecycle, buyer discovery, trade against listed lots, dampening, and deterministic replay.

## Assumption Reassessment (2026-03-31)

1. Golden integration tests live in `crates/worldwake-ai/tests/golden_*.rs`. Confirmed — existing files include `golden_integration.rs`, `golden_trade.rs`, `golden_supply_chain.rs`, `golden_production.rs`, `golden_ai_decisions.rs`, `golden_combat.rs`, `golden_offices.rs`, etc.
2. Golden tests use the harness pattern: `GoldenHarness` or `IntegrationHarness` with full action registries, scheduler, and world setup. Confirmed.
3. Decision trace and action trace systems are available for verification (`h.driver.enable_tracing()`, `h.enable_action_tracing()`). Confirmed in CLAUDE.md.
4. `PerceptionProfile` is required on agents that need to observe post-production output. Confirmed in CLAUDE.md.
5. All 10 preceding tickets must be complete before these tests can be written — they depend on the full S04 feature surface.
6. The spec lists 12 specific test scenarios in the Tests section.
7. No adjacent contradictions found.

## Architecture Check

1. Golden tests exercise the full system stack without mocking intermediate layers. This is the strongest verification surface for cross-system behavior.
2. Each test scenario maps to a specific spec invariant, making the tests directly traceable to requirements.
3. No backwards-compatibility concerns — these are new tests.

## Verification Layers

1. Each scenario maps to a spec invariant:
   - buyer discovers listed lots (not unlisted stock) -> golden test + world state
   - merchant emits SellCommodity -> decision trace
   - staff_market lists/unlists -> action trace + world state
   - buyer trades against listed lot -> action trace + world state + conservation
   - unlisted stock invisible -> decision trace (no trade candidate)
   - seller departure invalidates listing -> world state
   - dead seller invalidates listing -> world state
   - blocked-intent dampening -> decision trace (no re-emission)
   - MoveCargo + SellCommodity chain -> action trace sequence
   - demand memory ranking -> decision trace (motive comparison)
   - planning state determinism -> planning snapshot comparison
   - deterministic replay -> replay verification

## What to Change

### 1. Create `golden_merchant_selling.rs` test file

New file `crates/worldwake-ai/tests/golden_merchant_selling.rs` containing golden integration tests for the S04 spec scenarios.

### 2. Implement test scenarios

Each test follows the golden harness pattern with full world setup:

1. **buyer_discovers_listed_lots_not_unlisted_stock**: Setup merchant with listed + unlisted lots. Verify buyer AcquireCommodity evidence includes only listed lots.
2. **merchant_emits_sell_commodity_at_home_market**: Setup merchant at home_market with unlisted stock. Verify SellCommodity candidate emitted via decision trace.
3. **staff_market_lists_on_start_unlists_on_complete**: Run staff_market action. Verify SaleListing added on start, removed on commit via action trace + world state.
4. **buyer_trades_against_listed_lot**: Full trade cycle: merchant lists, buyer discovers, trade executes. Verify correct lot transfer and conservation.
5. **unlisted_stock_not_sellable**: Merchant with stock but no SaleListing. Verify buyer cannot discover or trade.
6. **seller_departure_invalidates_listing**: Merchant lists then travels away. Verify listing pruned within one tick.
7. **dead_seller_invalidates_listing**: Merchant lists then dies. Verify listing pruned within one tick.
8. **blocked_intent_dampens_relisting**: Unproductive staff_market cycle followed by candidate check. Verify SellCommodity suppressed.
9. **move_cargo_then_sell_commodity**: Merchant with off-site stock. Verify plan: MoveCargo -> Travel -> StaffMarket sequence.
10. **demand_memory_raises_sell_ranking**: Compare SellCommodity motive with and without demand memory. Verify boost without overpowering self-care.
11. **planning_state_preserves_listing_determinism**: Verify planning snapshot and planning state produce identical listed-lot visibility.
12. **deterministic_replay_preserves_listing_behavior**: Run scenario, save, replay, verify identical state hashes.

### 3. Register in test inventory

Update `docs/generated/golden-scenario-map.md` and `docs/generated/golden-e2e-inventory.md` if the golden inventory scripts auto-detect new test files.

## Files to Touch

- `crates/worldwake-ai/tests/golden_merchant_selling.rs` (new — full golden test suite)
- `docs/generated/golden-scenario-map.md` (modify — if manual update needed)
- `docs/generated/golden-e2e-inventory.md` (modify — if manual update needed)

## Out of Scope

- Any implementation changes — this is test-only
- Focused unit tests for individual components (covered by tickets 001-010)
- Performance testing or soak tests
- CLI integration tests

## Acceptance Criteria

### Tests That Must Pass

1. All 12 golden scenarios pass
2. Decision traces confirm correct candidate generation and goal selection
3. Action traces confirm correct action lifecycle (start, commit, abort)
4. World state checks confirm SaleListing attach/detach correctness
5. Conservation invariants hold across all trade scenarios
6. Deterministic replay reproduces identical results
7. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. Golden tests exercise full system stack — no mocked intermediate layers
2. Each test maps to a specific spec scenario — complete coverage of S04 test list
3. Tests are deterministic — same seed produces same results
4. Tests use `PerceptionProfile` on agents that need to observe events

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_merchant_selling.rs` — 12 golden integration tests covering all S04 spec test scenarios

### Commands

1. `cargo test -p worldwake-ai -- golden_merchant_selling`
2. `cargo clippy --workspace && cargo test --workspace`
3. `python3 scripts/golden_inventory.py --write --check-docs`

## Outcome

- **Completion date**: 2026-04-01
- **What changed**:
  - Created `crates/worldwake-ai/tests/golden_merchant_selling.rs` with 5 core golden tests + 2 replay companions
  - Tests: staff_market lists/unlists, buyer trades listed lot, unlisted stock invisible, blocked intent dampens relisting, deterministic replay
  - Fixed planner search gate: removed `SellCommodity` from `unsupported_goal` in `search/candidates.rs`
  - Added `StaffMarket` payload override in `goal_model.rs:build_payload_override`
  - Added `payload_override_is_valid` for `staff_market` handler registration
  - Updated golden inventory via `golden_inventory.py`
- **Deviations from original plan**:
  - Only 5 of 12 scenarios implemented (per user-approved option 2 to avoid context degradation). Remaining 7: seller departure, dead seller, move cargo chain, demand ranking, planning state determinism, buyer discovery evidence, replay variant.
  - Discovered and fixed a planner infrastructure gap: `SellCommodity` was hardcoded as `unsupported_goal` in search, and `staff_market` lacked a payload override validator. Both were required for the AI to autonomously plan and execute `SellCommodity` → `StaffMarket`.
- **Verification**: `cargo clippy --workspace --all-targets -- -D warnings` clean, `cargo test --workspace` all tests pass
