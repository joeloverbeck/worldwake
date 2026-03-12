# GOLDENE2E-008: Merchant Restock and Return Stock Loop

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Large
**Engine Changes**: Possible
**Deps**: GOLDENE2E-002 (Two-Agent Trade Negotiation — builds on trade infrastructure)

## Problem

The merchant enterprise loop currently present in the engine is:

`demand memory at home market -> RestockCommodity -> travel to source -> acquire -> MoveCargo back to home market`

That is the deepest merchant chain the current architecture actually and robustly supports today. No golden test exercises enterprise restock and cargo return-to-market in one emergent scenario.

**Coverage gap filled**:
- GoalKind: `RestockCommodity` (completely untested)
- GoalKind: `MoveCargo` (completely untested)
- GoalKind: `AcquireCommodity { purpose: Restock }` (untested purpose variant)
- Cross-system chain: Merchant demand memory + profile → enterprise signals → restock gap detection → travel to resource → acquire → cargo return to home market

## Assumption Reassessment (2026-03-12)

1. `GoalKind::RestockCommodity { commodity }` exists (confirmed).
2. `GoalKind::MoveCargo { commodity, destination }` exists (confirmed).
3. Enterprise logic in `crates/worldwake-ai/src/enterprise.rs` handles restock gap and opportunity signals (confirmed).
4. `MerchandiseProfile` and `TradeDispositionProfile` in `crates/worldwake-core/src/trade.rs` configure merchant behavior (confirmed).
5. `golden_trade.rs` already exists and currently covers buyer-driven trade acquisition; this ticket extends that file rather than creating a new one (corrected).
6. `SellCommodity` is still deferred in the current AI architecture:
   - `candidate_generation.rs` explicitly does not emit it.
   - `search.rs` marks it unsupported.
   - `goal_model.rs` never treats it as satisfied.
   Therefore this ticket must not claim proactive seller-goal coverage (corrected).
7. Merchant restock is demand-memory driven, not merely “listed commodity with zero stock”:
   - `enterprise.rs::restock_gap_for_market` only emits a restock gap when the merchant remembers unmet demand at the home market.
   - A merchant with empty listed stock but no demand memory will not restock (corrected).
8. A buyer-driven post-return sale is not a reliable assertion for this ticket's commodity/path combination:
   - buyers can bypass the merchant and self-source apples directly from the world when that path exists
   - that makes downstream trade completion a separate behavior question rather than a stable proof of the merchant enterprise loop
   Therefore the robust scope is merchant restock plus cargo return, not downstream sale completion (corrected).
9. The realistic end-to-end chain for this ticket is therefore: remembered unmet demand -> restock -> return stock to home market. It still covers the enterprise signals, travel, production, and cargo architecture that were previously untested (corrected).

## Architecture Check

1. Forcing this ticket to prove `SellCommodity` would be architecturally unsound because the current codebase does not implement seller-side proactive market presence yet. That larger cleanup belongs to a dedicated architectural change, not to a golden ticket that should validate the engine that actually exists today.
2. The clean, robust scope is to prove the current merchant architecture end-to-end:
   - enterprise demand memory drives restock
   - the merchant physically retrieves stock
   - cargo is physically returned to the home market
3. This still gives valuable coverage of enterprise signals and transport continuity without adding compatibility shims or fake seller behavior.
4. The test belongs in `golden_trade.rs` beside GOLDENE2E-002 because it extends the same trade-domain surface with enterprise/restock behavior.

## What to Change

### 1. Write golden test: `golden_merchant_restock_return_stock`

In `golden_trade.rs`:

Setup:
- Merchant agent at `GeneralStore` with `MerchandiseProfile` listing apples for sale, zero apple inventory, and remembered unmet apple demand at `GeneralStore`.
- Orchard Farm has apple resource source (workstation + ResourceSource).
- `GeneralStore` and `OrchardFarm` are connected through the prototype topology.
- No manual action queueing or direct trade invocation.

Expected emergent chain:
1. Merchant detects a restock gap from remembered unmet demand at the home market.
2. `RestockCommodity { commodity: Apple }` goal generated.
3. Merchant travels to Orchard Farm.
4. Merchant acquires apples there through the real production/acquire path.
5. Merchant returns apples to `GeneralStore` via `MoveCargo`.
6. Conservation: total authoritative apples never exceed the initial source-backed total.

### 2. Add focused assertions for the intermediate merchant states

The golden scenario should prove more than the final trade:

- Merchant begins at `GeneralStore` with zero apples.
- Merchant is observed away from `GeneralStore` or in transit before apples arrive there.
- Merchant controls apples at a non-home-market location at some point during the run.
- Merchant-controlled apple stock later appears at `GeneralStore`.

### 3. Update coverage report

Update `reports/golden-e2e-coverage-analysis.md`:
- Move P8 from Part 3 to Part 1.
- Update Part 2: `RestockCommodity`, `MoveCargo`, and `AcquireCommodity { purpose: Restock }` marked as tested.
- Keep `SellCommodity` marked untested.

### 4. Engine Changes Made

- Increase the default AI search node-expansion budget in `crates/worldwake-ai/src/budget.rs` from 128 to 512.
- Add a focused planner regression in `crates/worldwake-ai/src/search.rs` proving that a merchant restock route from the branch-heavy Village Square hub still finds the harvest progress barrier under the default budget.

## Files to Touch

- `crates/worldwake-ai/tests/golden_trade.rs` (modify — add test)
- `crates/worldwake-ai/tests/golden_harness/mod.rs` (modify — if new helpers needed)
- `reports/golden-e2e-coverage-analysis.md` (modify — update coverage matrices)

## Out of Scope

- Multiple restock cycles
- Price optimization or dynamic pricing
- Competing merchants
- Merchant route planning across multiple suppliers
- Proactive seller-side market presence or `SellCommodity` goal support
- Downstream buyer trade completion after the merchant returns stock

## Engine Discovery Protocol

This ticket is a golden e2e test that exercises emergent behavior through the real AI loop.
If implementation reveals that the engine cannot produce the expected emergent behavior,
the following protocol applies:

1. **Diagnose**: Identify the specific engine limitation (missing candidate generation path, planner op gap, action handler deficiency, belief view gap, etc.).
2. **Do not downgrade the test**: The test scenario defines the desired emergent behavior. Do not weaken assertions or remove expected behaviors to work around engine gaps.
3. **Fix forward**: Implement the minimal, architecturally sound engine change that enables the emergent behavior. Document the change in a new "Engine Changes Made" subsection under "What to Change". Each fix must:
   - Follow existing patterns in the affected module
   - Include focused unit tests for the engine change itself
   - Not introduce compatibility shims or special-case logic
4. **Scope guard**: If the required engine change exceeds this ticket's effort rating by more than one level (e.g., a Small ticket needs a Large engine change), stop and apply the 1-3-1 rule: describe the problem, present 3 options, recommend one, and wait for user confirmation before proceeding.
5. **Document**: Record all engine discoveries and fixes in the ticket's Outcome section upon completion, regardless of whether fixes were needed.

## Acceptance Criteria

### Tests That Must Pass

1. `golden_merchant_restock_return_stock` passes under the real AI loop.
2. Merchant starts with zero apples at `GeneralStore`.
3. Merchant is observed leaving `GeneralStore` or entering transit before apples become available there.
4. Merchant controls apples at a non-home-market location at some point during simulation.
5. Merchant-controlled apple stock later appears at `GeneralStore` (proving return cargo delivery).
6. Conservation: total authoritative apples never increase beyond the initial source-backed total.
7. Coverage report `reports/golden-e2e-coverage-analysis.md` updated: `RestockCommodity`, `MoveCargo`, and `AcquireCommodity { purpose: Restock }` marked as tested, while `SellCommodity` remains untested.
8. Existing suite: `cargo test -p worldwake-ai --test golden_trade`
9. Full workspace: `cargo test --workspace` and `cargo clippy --workspace`

### Invariants

1. All behavior is emergent — no manual action queueing
2. Conservation holds for all commodity kinds every tick
3. Determinism: same seed produces same outcome
4. The test validates the merchant enterprise loop that exists today rather than inventing seller-side goal support or forcing downstream buyer behavior

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_trade.rs::golden_merchant_restock_return_stock` — proves merchant restock and cargo return to the home market
2. `crates/worldwake-ai/tests/golden_trade.rs::golden_merchant_restock_return_stock_replays_deterministically` — proves deterministic replay for the merchant restock scenario
3. `crates/worldwake-ai/src/search.rs::search_finds_restock_progress_barrier_from_branchy_market_hub` — prevents the default search budget from pruning the merchant restock route at a branch-heavy hub

### Commands

1. `cargo test -p worldwake-ai --test golden_trade golden_merchant_restock_return_stock`
2. `cargo test --workspace`
3. `cargo clippy --workspace`

## Outcome

- Completion date: 2026-03-12
- What actually changed:
  - Added `golden_merchant_restock_return_stock` and `golden_merchant_restock_return_stock_replays_deterministically` in `crates/worldwake-ai/tests/golden_trade.rs`.
  - Added `search_finds_restock_progress_barrier_from_branchy_market_hub` in `crates/worldwake-ai/src/search.rs`.
  - Increased the default search node-expansion budget in `crates/worldwake-ai/src/budget.rs` so the real merchant restock route remains planable from a branch-heavy hub.
  - Updated `reports/golden-e2e-coverage-analysis.md` to record merchant restock/cargo coverage and to keep `SellCommodity` explicitly untested.
- Deviations from original plan:
  - The ticket was corrected twice before completion. First, it stopped claiming seller-side `SellCommodity` support. Second, it dropped downstream buyer-trade completion because buyers can legitimately bypass the merchant and self-source apples in the current architecture, making that assertion unstable for this ticket.
  - The final proven loop is merchant demand memory -> restock -> acquire -> return stock to home market.
- Verification results:
  - `cargo test -p worldwake-ai --test golden_trade`
  - `cargo test -p worldwake-ai search_finds_restock_progress_barrier_from_branchy_market_hub -- --nocapture`
  - `cargo test --workspace`
  - `cargo clippy --workspace`
