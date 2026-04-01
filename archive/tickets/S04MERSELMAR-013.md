# S04MERSELMAR-013: Nonzero baseline enterprise motive for merchants with stock but no demand memory

**Status**: ✅ COMPLETED
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: Yes — enterprise motive signal in AI crate
**Deps**: S04MERSELMAR-007

## Problem

`market_signal_for_place` in `enterprise.rs` hard-returns `Permille(0)` when `relevant_demand_quantity` is 0. This makes demand memory a de facto gating condition for `SellCommodity`, contradicting the S04 spec invariant: "Demand memory is a ranking signal, not a gating condition — merchants can sell without demand memory." A freshly spawned merchant with stock at `home_market` cannot autonomously sell until demand memory is externally seeded, violating Principle 1 (Maximal Emergence) and Principle 6 (World Runs Without Observers).

## Assumption Reassessment (2026-04-01)

1. `market_signal_for_place` at `crates/worldwake-ai/src/enterprise.rs:54` checks `if demand == 0 { return Permille::new_unchecked(0); }`. Confirmed.
2. `enterprise_score` at `crates/worldwake-ai/src/ranking.rs:964` calls `opportunity_signal` → `market_signal_for_place`. When signal is 0, `score_product(enterprise_weight, 0) = 0`. Confirmed.
3. `SellCommodity` ranking uses `enterprise_score` (ranking.rs:501). With 0 motive, it never wins selection against any other goal with nonzero motive. Confirmed.
4. The S04 spec (Section 13) states: "Demand memory is a ranking signal, not a gating condition — merchants can sell without demand memory."
5. GoalKind under test: `SellCommodity { commodity }`. Operator surface: `PlannerOpKind::StaffMarket`. Affordance: `staff_market` (untargeted, `StaffMarketPayload`). All wired after S04MERSELMAR-011.
6. No adjacent contradictions. `RestockCommodity` also uses `enterprise_score` but its candidate generation already requires demand memory, so the 0-signal behavior is consistent there. `SellCommodity` candidate generation does NOT require demand memory — it only requires unlisted local stock.

## Architecture Check

1. The fix should provide a nonzero baseline signal when the merchant has stock of the commodity at the market, even with no demand memory. The baseline should be lower than a demand-memory-boosted signal so demand memory still functions as a ranking boost. A simple approach: when `demand == 0` but `stock > 0`, return a small baseline (e.g., `Permille(100)`) instead of `Permille(0)`. This preserves the existing demand-boost arithmetic while allowing first-time selling.
2. No backwards-compatibility shims.

## Verification Layers

1. SellCommodity motive nonzero without demand memory -> focused ranking test
2. SellCommodity motive higher with demand memory than without -> focused ranking comparison test
3. Merchant autonomously starts staff_market without demand memory -> golden test (modify existing `golden_merchant_selling.rs` setup)
4. Single-layer ticket (AI ranking); authoritative action framework not modified.

## What to Change

### 1. Provide nonzero baseline in `market_signal_for_place`

In `crates/worldwake-ai/src/enterprise.rs`, modify `market_signal_for_place` to return a nonzero baseline signal when `demand == 0` but the merchant has stock of the commodity. The baseline should be a modest fraction of `Permille::MAX` (e.g., `Permille(100)`) to keep demand-memory-boosted signals strictly higher.

### 2. Add focused ranking test

In `crates/worldwake-ai/src/ranking.rs` tests, add a test that verifies `SellCommodity` has nonzero motive when the merchant has stock but no demand memory.

## Files to Touch

- `crates/worldwake-ai/src/enterprise.rs` (modify — baseline signal when demand is 0 but stock > 0)
- `crates/worldwake-ai/src/ranking.rs` (modify — add focused test)

## Out of Scope

- `RestockCommodity` baseline (its candidate generation already gates on demand memory — consistent)
- Demand memory creation or aging
- Valuation changes

## Acceptance Criteria

### Tests That Must Pass

1. `SellCommodity` motive is nonzero when merchant has stock but no demand memory
2. `SellCommodity` motive is higher with demand memory than without (demand memory is still a boost)
3. Merchant can autonomously start `staff_market` in a golden test without seeded demand memory
4. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. Demand memory remains a ranking signal — higher demand memory produces higher motive
2. Zero stock still produces zero motive (no stock = nothing to sell)
3. Enterprise goals never overpower survival-class goals

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/ranking.rs` — focused test: SellCommodity motive nonzero without demand memory
2. `crates/worldwake-ai/src/ranking.rs` — focused test: demand memory boosts motive above baseline

### Commands

1. `cargo test -p worldwake-ai -- ranking`
2. `cargo test -p worldwake-ai -- enterprise`
3. `cargo clippy --workspace && cargo test --workspace`

## Outcome

- **Completion date**: 2026-04-01
- **What changed**:
  - Added `STOCK_PRESENT_BASELINE` constant (`Permille(100)`) in `enterprise.rs`
  - `market_signal_for_place` returns the baseline when demand=0 but stock>0, instead of hard-returning 0
  - 3 new focused tests: baseline with stock, zero without stock, demand exceeds baseline
- **Deviations from original plan**: None — implementation matched ticket exactly
- **Verification**: `cargo clippy --workspace --all-targets -- -D warnings` clean, `cargo test --workspace` all tests pass
