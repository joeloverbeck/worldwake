# S04MERSELMAR-009: Trade commit rules for `sale_lot` validity

**Status**: ✅ COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — trade commit handler validation and lot transfer logic
**Deps**: S04MERSELMAR-005, S04MERSELMAR-008

## Problem

The trade commit handler must validate that the `sale_lot` referenced in `TradeActionPayload` is still valid at commit time and handle listing state correctly during lot transfer. Without these checks, trades could execute against lots that are no longer listed, no longer possessed by the seller, or no longer co-located.

## Assumption Reassessment (2026-03-31)

1. Trade commit handler `commit_trade` in `crates/worldwake-systems/src/trade_actions.rs` currently validates: buyer and seller co-located, seller has sufficient stock of `requested_commodity`, buyer has sufficient payment. Confirmed by reading the handler.
2. Lot splitting logic already exists — partial-lot trades split the lot and transfer a portion. Confirmed in `trade_actions.rs`.
3. `LotOperation::Traded` provenance is already appended on trade. Confirmed.
4. `SaleListing` must be removed from transferred lot portions. If the seller retains a remainder that is still local and possessed, it may remain listed.
5. Listing cleanup system (ticket 005) handles edge cases, but the trade commit itself must handle the normal flow: remove listing from transferred portion.
6. `TradeActionPayload` will have `sale_lot: EntityId` field after ticket 008.
7. No adjacent contradictions found.

## Architecture Check

1. Adding `sale_lot` validity checks to the trade commit handler is the natural place — the handler already validates other preconditions at commit time. This keeps all commit-time validation co-located.
2. Removing `SaleListing` from transferred portions during commit is correct — the new owner is not the seller, so the listing is meaningless on their lot.
3. Retaining `SaleListing` on the seller's remainder (if any) is correct — the seller is still actively staffing the market.
4. No backwards-compatibility shims.

## Verification Layers

1. Trade fails if `sale_lot` no longer exists -> focused unit test (abort with clear reason)
2. Trade fails if `sale_lot` no longer has `SaleListing` -> focused unit test
3. Trade fails if seller no longer possesses `sale_lot` -> focused unit test
4. Trade fails if seller and buyer no longer co-located -> existing test (already validated)
5. `SaleListing` removed from transferred lot -> focused unit test + world state check
6. `SaleListing` preserved on seller remainder -> focused unit test + world state check
7. Provenance and conservation preserved -> existing trade tests

## What to Change

### 1. Add `sale_lot` validity checks to `commit_trade`

At commit time, before executing the transfer, validate:
- `sale_lot` entity still exists (not archived)
- `sale_lot` still has `SaleListing` component
- seller still directly possesses `sale_lot`
- seller still has enough quantity in the lot (existing quantity check, but now against the specific lot)
- buyer and seller still co-located (existing check)
- bilateral bundle valuation still accepts (existing check)

If any check fails, abort the trade with an appropriate `ActionError`.

### 2. Update lot transfer to handle `SaleListing`

When the trade commits and lot transfer occurs:
- If the entire lot transfers to the buyer: remove `SaleListing` from it
- If the lot is split (partial quantity trade):
  - The transferred portion (new lot going to buyer): do NOT attach `SaleListing`
  - The seller's remainder lot: preserve `SaleListing` if the remainder still meets listing conditions (still possessed, still at market, commodity still in `sale_kinds`)

### 3. Derive `requested_commodity` from `sale_lot`

In the commit handler, derive `requested_commodity` from `world.item_lot(payload.sale_lot).commodity` instead of reading a now-removed payload field. (Structural change from ticket 008, but the derivation logic lives here.)

## Files to Touch

- `crates/worldwake-systems/src/trade_actions.rs` (modify — commit validation, listing transfer logic)

## Out of Scope

- `TradeActionPayload` struct changes (ticket 008 — must be done first)
- Listing cleanup for non-trade edge cases (ticket 005)
- Buyer-side evidence generation (ticket 008)
- `staff_market` action handler (ticket 003)
- Valuation changes beyond existing bilateral bundle checks

## Acceptance Criteria

### Tests That Must Pass

1. Trade against a listed lot succeeds and buyer receives correct quantity
2. Trade fails (aborts) if `sale_lot` no longer has `SaleListing`
3. Trade fails if seller no longer possesses `sale_lot`
4. Trade fails if `sale_lot` entity was archived
5. `SaleListing` is removed from the transferred lot (whole or split-off portion)
6. `SaleListing` is preserved on the seller's remainder lot after a partial trade
7. `LotOperation::Traded` provenance is appended as before
8. Conservation invariant holds: `verify_live_lot_conservation` passes after trade
9. Existing suite: `cargo test -p worldwake-systems`

### Invariants

1. Trades only execute against lots with valid `SaleListing` at commit time
2. Transferred lots never retain `SaleListing` — the buyer is not the seller
3. Conservation is preserved exactly as before
4. Bilateral valuation remains the acceptance mechanism — `SaleListing` is a prerequisite, not a price

## Test Plan

### New/Modified Tests

1. `crates/worldwake-systems/src/trade_actions.rs` — focused test: trade succeeds against listed lot
2. `crates/worldwake-systems/src/trade_actions.rs` — focused test: trade fails when listing removed between start and commit
3. `crates/worldwake-systems/src/trade_actions.rs` — focused test: trade fails when seller lost possession between start and commit
4. `crates/worldwake-systems/src/trade_actions.rs` — focused test: SaleListing correctly handled on split lots
5. Existing trade tests updated for new payload structure

### Commands

1. `cargo test -p worldwake-systems -- trade_action`
2. `cargo clippy --workspace && cargo test --workspace`

## Outcome

- **Completion date**: 2026-04-01
- **What changed**:
  - Added `SaleLotNotListed` and `SaleLotNotPossessedBySeller` variants to `ActionAbortRequestReason`
  - `validate_trade_bundle_context` now checks sale_lot has `SaleListing` and seller possesses it before proceeding
  - `transfer_trade_lot` removes `SaleListing` from lots transferred to the buyer
  - `map_handler_abort_reason` in failure_handling.rs maps new variants to `SellerOutOfStock`
  - Existing `partial_lot_trade_splits_and_preserves_conservation` test updated to add `SaleListing` to replacement lot
  - Golden trade test assertions widened to accept new sale-lot-specific failure reasons
  - 4 new focused tests: listing-removed abort, possession-lost abort, listing removal on transfer, listing preservation on split remainder
- **Deviations from original plan**:
  - Deliverable 3 (derive `requested_commodity` from `sale_lot`) was already done by ticket 008; skipped
  - Golden trade test needed assertion updates for new failure reason priority
- **Verification**: `cargo clippy --workspace --all-targets -- -D warnings` clean, `cargo test --workspace` all tests pass
