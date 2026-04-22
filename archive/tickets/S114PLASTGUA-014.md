# S114PLASTGUA-014: Seller-backed displayed sale stock with known container detail must still route through `Trade`

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: Yes — `worldwake-ai` search filtering for seller-backed displayed sale lots
**Deps**: `specs/S114-plan-step-guards.md`, `archive/tickets/S114PLASTGUA-010.md`, `archive/tickets/S114PLASTGUA-011.md`, `archive/tickets/S114PLASTGUA-013.md`

## Problem

`archive/tickets/S114PLASTGUA-013.md` fixed the no-custody-detail case for remote listed sale lots, but the live runtime still had one remaining planner/search contradiction on the path to the deferred S114 golden. When the buyer knew a displayed sale lot entity strongly enough for `PerAgentBeliefView` / `PlanningState` to expose its direct display-container detail, `AcquireCommodity(SelfConsume)` could still admit `MoveCargo` root candidates against that same seller-backed lot instead of keeping the opportunity trade-only. That left the deferred guard-breach golden without a truthful planner substrate whenever the agent knew both the seller-backed listing and the lot’s container detail.

## Assumption Reassessment (2026-04-22)

1. The originally drafted ticket claimed pure golden/E2E ownership. Live focused reproduction disproved that boundary: before any truthful guard-breach golden could land, the planner still needed one more production-side filter for seller-backed displayed lots with known container detail.
2. `archive/tickets/S114PLASTGUA-013.md` remains correct for the narrower “remote listed sale lot without custody detail” case. The remaining gap only appears when the buyer also knows the lot entity closely enough for `direct_container()` to return the authoritative display container.
3. Exact shared boundary under audit: `PerAgentBeliefView` / `PlanningState` economic read surface (`seller_for_sale_lot`) versus `worldwake-ai` root-candidate admission in `crates/worldwake-ai/src/search/candidates.rs`.
4. Strongest honest proof surface for the newly exposed bug is focused planner/search coverage, not a golden. The contradiction appears before runtime guard revalidation: the selected branch stayed `MoveCargo` whenever a seller-backed displayed lot also exposed container detail.
5. Live `GoalKind` under test is `AcquireCommodity { commodity: Bread, purpose: SelfConsume }`. The exact operator family under audit is the planner’s `MoveCargo` versus `Trade` root-candidate surface for seller-backed displayed sale stock.
6. Existing focused search proof already covered the no-custody-detail branch: `search_returns_travel_then_trade_barrier_for_remote_listed_sale_lot_without_custody_detail`.
7. A new focused search proof was required for the live missing case: remote listed sale lot, seller-backed, and known display-container detail.
8. Focused autonomous golden repro after the search fix still disproved the drafted end-to-end stale-window. The buyer first selects a remote travel-then-trade branch, but arrival remains a progress barrier; after the seller departs, the live runtime replans through `BlockingFact::TooExpensive` rather than emitting the guard-breach `ExpectationMismatch` payload. That remaining proof problem is now split out to `tickets/S114PLASTGUA-015.md`.
9. Mismatch + correction: this ticket no longer owns the full guard-breach golden from the original draft. It truthfully owns the final production routing fix required before any future golden can prove that scenario.

## Architecture Check

1. The clean fix is to reuse the existing seller-backed sale-lot contract (`seller_for_sale_lot`) and make search treat those lots as trade-only for acquisition-style goals, rather than inventing another carrier or weakening the proof to accept `MoveCargo`.
2. No backward-compatibility aliasing or new substrate was introduced. The fix narrows the already-live search filter at the exact boundary where seller-backed displayed stock and loose cargo must stop sharing the same operator family.

## Verification Layers

1. Seller-backed displayed lot with known container detail no longer admits a `MoveCargo` path for acquisition -> focused search test
2. Existing no-custody-detail remote listed-lot routing remains `Travel -> Trade` -> focused search test
3. Existing merchant-selling golden still passes with the narrowed search filter -> focused golden test
4. The disproved autonomous stale-window is tracked explicitly as follow-up ticket ownership, not hidden as silent fallout -> active ticket/spec update

## What to Change

### 1. Filter seller-backed displayed lots out of `MoveCargo` search admission

Use the already-live `seller_for_sale_lot` read surface during search candidate filtering so `AcquireCommodity`-family goals stop admitting `MoveCargo` root candidates against lots that are still seller-backed displayed sale stock.

### 2. Add focused proof for the known-container-detail case

Extend search coverage with the exact case that the original S114-013 proof did not cover: remote listed sale lot, seller-backed, and known display-container detail.

### 3. Split the remaining golden work into a new owner

Record the still-disproved autonomous stale-window truthfully and create `tickets/S114PLASTGUA-015.md` for the hybrid/local trade-step guard-breach proof seam.

## Files to Touch

- `crates/worldwake-ai/src/search/candidates.rs` (modify — filter seller-backed displayed lots out of `MoveCargo` search admission)
- `crates/worldwake-ai/src/search/tests.rs` (modify — focused regression for known display-container detail)
- `tickets/S114PLASTGUA-014.md` (modify — reassessed scope and closeout)
- `tickets/S114PLASTGUA-015.md` (new — follow-up for the remaining hybrid/autonomous golden proof problem)
- `specs/S114-plan-step-guards.md` (modify — point deferred validation item 12 at the new owner)

## Out of Scope

- Landing the final merchant-departure guard-breach golden in `golden_merchant_selling.rs`
- New guard kinds, payload fields, or discrepancy-taxonomy changes
- Reworking travel progress-barrier semantics for remote acquisition plans

## Acceptance Criteria

### Tests That Must Pass

1. Remote seller-backed displayed sale stock with known container detail routes through `Travel -> Trade`, not `MoveCargo`
2. Existing remote listed-lot routing without custody detail stays green
3. Existing merchant-selling golden coverage stays green for the local listed-lot trade path
4. A new active ticket exists for the still-disproved autonomous/hybrid guard-breach golden seam

### Invariants

1. Seller-backed displayed sale stock and loose cargo must not share the same `MoveCargo` root-candidate surface for acquisition-style goals
2. This ticket must not overstate a golden contract that the live runtime still disproves

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/search/tests.rs` — prove remote displayed sale stock with known container detail still selects `Trade`
2. `None — the originally drafted golden was reassessed out of this ticket and moved to a follow-up owner`

### Commands

1. `cargo test -p worldwake-ai --lib search::tests::search_returns_travel_then_trade_barrier_for_remote_displayed_sale_lot_with_container_detail -- --exact`
2. `cargo test -p worldwake-ai --lib search::tests::search_returns_travel_then_trade_barrier_for_remote_listed_sale_lot_without_custody_detail -- --exact`
3. `cargo test -p worldwake-ai --test golden_merchant_selling buyer_trades_against_listed_lot -- --exact`

## Outcome

Completed on 2026-04-22.

- `crates/worldwake-ai/src/search/candidates.rs` now filters seller-backed displayed sale lots out of `MoveCargo` admission for acquisition-style goals. When the target lot is still listed for sale by another seller, search keeps that opportunity on the `Trade` path instead of treating known display-container detail as permission to plan a cargo pickup.
- Added focused search coverage for the previously missing case: remote listed sale lot with known display-container detail still returns a `Travel -> Trade` barrier.
- Reassessed the originally drafted golden honestly: after this routing fix, the full autonomous stale-window in `golden_merchant_selling` is still disproved by the live progress-barrier / replanning shape, so the remaining proof work was extracted to `tickets/S114PLASTGUA-015.md` instead of forcing a false green test.

## Deviations

- Draft ticket 014 was authored as a golden-only slice. Live reassessment showed one more production search fix was still required before any truthful golden could exist, so the ticket was widened to land that narrow fix and to hand off the remaining golden proof seam explicitly.

## Verification Result

- Passed `cargo test -p worldwake-ai --lib search::tests::search_returns_travel_then_trade_barrier_for_remote_displayed_sale_lot_with_container_detail -- --exact`
- Passed `cargo test -p worldwake-ai --lib search::tests::search_returns_travel_then_trade_barrier_for_remote_listed_sale_lot_without_custody_detail -- --exact`
- Passed `cargo test -p worldwake-ai --test golden_merchant_selling buyer_trades_against_listed_lot -- --exact`
