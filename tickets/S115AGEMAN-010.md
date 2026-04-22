# S115AGEMAN-010: revived buyer trade still aborts after seller-side listing restoration

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes — mixed trade-action / AI-resumption boundary after seller-side relist.
**Deps**: [archive/tickets/S115AGEMAN-009](../archive/tickets/S115AGEMAN-009.md), [archive/tickets/S115AGEMAN-008](../archive/tickets/S115AGEMAN-008.md)

## Problem

`S115AGEMAN-009` restored the seller-side authoritative relist seam: after seller departure prunes a displayed lot's `SaleListing`, seller return now restores that listing lawfully. But the broader drafted story is still false on the live branch. After buyer-side pending revival and seller-side relist, the revived buyer reaches `trade` and then aborts with `TradeBundleRejected { reason: InsufficientPayment }` instead of completing the resumed purchase.

## Assumption Reassessment (2026-04-22)

1. Buyer-side pending parking / revival already passes in `crates/worldwake-ai/tests/golden_merchant_selling.rs::merchant_return_revives_pending_purchase_agenda_entry`.
2. Seller-side authoritative relist after return now passes in `crates/worldwake-ai/tests/golden_merchant_selling.rs::seller_return_restores_displayed_listing_after_pending_revival` plus focused `crates/worldwake-systems/src/trade.rs::tests::displayed_listing_returns_when_seller_returns_to_market`.
3. The remaining shared boundary under audit is no longer listing validity. It is the later mixed-layer handoff from revived buyer `trade` resumption into the live trade negotiation / completion path.
4. Focused golden instrumentation on the live branch showed the buyer does retry `trade` after relist, and the failure is not a `StartFailed` path. The action aborts later with `ExternalAbort { kind: HandlerRequested { reason: TradeBundleRejected { ... InsufficientPayment }}}`.
5. This ticket must not reopen the seller-side listing fix or buyer-side pending trigger unless reassessment proves the remaining abort is caused by one of those seams rather than by the later trade payload / negotiation / resumption contract.

## Architecture Check

1. The fix should land at the first runtime or trade-action boundary that makes the resumed trade completion contract false, not by loosening the golden with more scaffolding or by reseeding state after seller return.
2. No compatibility shim should preserve both “revived buyer retried trade but aborted” and “revived buyer completes resumed trade” as parallel supported contracts.

## Verification Layers

1. The first failing resumed-trade boundary is identified precisely (`request resolution`, `trade` start, negotiation abort, or commit transfer) -> focused lower-layer proof at the owning seam.
2. Seller-side relist remains intact while the later trade fix lands -> existing `seller_return_restores_displayed_listing_after_pending_revival` golden.
3. If the broader resumed story becomes truthful, a merchant-selling golden may extend to actual trade completion after seller return; otherwise the ticket must narrow again before closeout.

## What to Change

### 1. Identify the first resumed-trade failure seam

Trace the revived buyer's post-relist `trade` attempt and bind the failure to the exact runtime/trade-action symbol and contract that currently aborts with `InsufficientPayment`.

### 2. Land the narrowest truthful production fix

Fix the owning mixed-layer seam so the resumed buyer trade can complete lawfully after seller-side relist, without undoing the seller relist fix or adding test-only reseeding.

### 3. Extend the golden only if the live branch supports it

If the later completion story becomes truthful, extend `golden_merchant_selling` to prove actual resumed trade completion. If live evidence disproves that stronger ending again, rewrite this ticket before closeout instead of forcing the golden through.

## Files to Touch

- `crates/worldwake-ai/tests/golden_merchant_selling.rs` (modify)
- `crates/worldwake-systems/src/trade_actions.rs` (modify if the abort lives in trade negotiation / commit)
- `crates/worldwake-ai/src/*` (modify only if reassessment proves the remaining seam is runtime-owned)

## Out of Scope

- Reopening seller-side authoritative relist logic already fixed by `S115AGEMAN-009`
- Reopening buyer-side pending parking / revival logic already fixed by `S115AGEMAN-008`

## Acceptance Criteria

### Tests That Must Pass

1. Focused proof at the first failing resumed-trade seam.
2. Existing seller-side relist golden remains green.
3. Existing suite: `cargo test -p worldwake-ai --test golden_merchant_selling`

### Invariants

1. Once seller-side relist has restored lawful listing state, the revived buyer's resumed trade path does not abort on a false mixed-layer contract.
2. The resumed completion path uses real seller/facility/listing/trade state, not ad hoc reseeding or compatibility shims.

## Test Plan

### New/Modified Tests

1. Focused lower-layer test at the first failing resumed-trade seam — prove the real abort/commit boundary.
2. `crates/worldwake-ai/tests/golden_merchant_selling.rs` — extend to actual resumed trade completion only if the live branch now supports that contract.

### Commands

1. `cargo test -p worldwake-ai --test golden_merchant_selling -- --list`
2. `cargo test -p worldwake-ai <focused-selector>`
3. `cargo test -p worldwake-ai --test golden_merchant_selling`
