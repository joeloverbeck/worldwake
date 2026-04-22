# S115AGEMAN-009: seller return should restore lawful market-selling state after buyer-side agenda revival

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes — authoritative trade listing validity sync for displayed merchant stock.
**Deps**: [archive/tickets/S115AGEMAN-008](./S115AGEMAN-008.md), [archive/tickets/S115AGEMAN-005](./S115AGEMAN-005.md), [archive/tickets/S115AGEMAN-007](./S115AGEMAN-007.md)

## Problem

After `S115AGEMAN-008`, a buyer whose locally bound purchase goal was parked to `pending` now revives back into committed/current-plan state when the seller returns. But the broader drafted story still does not complete: on the live branch, seller departure prunes the displayed lot's `SaleListing`, and seller return does not automatically restore that lawful market-facing listing state.

## Assumption Reassessment (2026-04-22)

1. `crates/worldwake-ai/tests/golden_merchant_selling.rs` now proves the buyer-side boundary truthfully with `merchant_return_revives_pending_purchase_agenda_entry`, and that test intentionally stops at revived `committed/current_plan` rather than trade completion.
2. The earlier pre-failure seam is still covered by `remote_branch_selection_reaches_local_trade_binding_before_merchant_departure`, which proves the buyer can reach a concrete local `trade` binding before seller departure.
3. The shared boundary under audit for this follow-up is no longer buyer agenda lifecycle. That boundary now passes in `agent_tick/planning.rs`. The remaining live question is seller-side market presence / listing readiness after authoritative departure and return.
4. The intended invariant is narrower than “seller is back at the same place.” A seller return must also restore the concrete seller-side state that makes displayed stock sale-visible again: seller presence plus lawful `SaleListing` validity for displayed stock at the controlled facility.
5. This ticket is mixed-layer: authoritative seller/facility/listing state in the merchant-selling substrate must line up with the AI/runtime path that resumes the revived buyer plan.
6. Reassessment from `S115AGEMAN-008` classified this as a separate bug, not a required consequence of the buyer-side pending-revival fix, so it needs its own focused ticket and truthful proof.
7. Focused live follow-up during implementation showed the stronger drafted `...and trade completes` ending is still false even after listing restoration: the revived buyer later reaches a `trade` abort with `TradeBundleRejected { reason: InsufficientPayment }`. That later mixed-layer completion seam is split to follow-up ticket `S115AGEMAN-010`.

## Architecture Check

1. The fix should restore the seller-side authoritative state transition that makes resumed trade lawful, rather than adding buyer-side special cases or test-only reseeding.
2. No compatibility shim should preserve both “seller returned but not market-ready” and “seller returned and lawfully sellable” as parallel contracts.

## Verification Layers

1. Seller departure/return prunes then restores `SaleListing` for displayed stock at the authoritative trade seam -> focused `worldwake-systems` unit proof.
2. Buyer-side pending revival remains intact while the seller-side relist fix lands -> existing `merchant_return_revives_pending_purchase_agenda_entry` golden.
3. Seller return restores displayed-lot listing after buyer-side pending revival without test-only reseeding -> focused merchant-selling golden.
4. The stronger resumed-trade completion seam remains owned by follow-up `S115AGEMAN-010`, not overclaimed here.

## What to Change

### 1. Reassess the seller-side return contract

Trace what concrete merchant-selling state is missing after seller departure and return: listing continuity, facility/display readiness, staff-market re-entry, or another authoritative prerequisite.

### 2. Restore the lawful seller-side path

Implement the narrowest production fix so displayed stock regains `SaleListing` when seller return makes that listing lawfully valid again, without buyer-side special cases or test-only scaffolding.

### 3. Extend the golden boundary truthfully

Add or revise merchant-selling golden coverage so the strongest honest seller-side seam is proved: pending revival remains green, and seller return restores the displayed-lot listing. Split any still-false later trade-completion story to a follow-up ticket instead of overclaiming it here.

## Files to Touch

- `crates/worldwake-ai/tests/golden_merchant_selling.rs` (modify)
- `crates/worldwake-systems/src/trade.rs` (modify)

## Out of Scope

- Reopening buyer-side pending parking / revival logic already landed in `S115AGEMAN-008`
- Observer-only reporting tweaks

## Acceptance Criteria

### Tests That Must Pass

1. Focused `worldwake-systems` proof that seller return restores `SaleListing` for displayed stock after departure.
2. Existing buyer-side revival seam remains green.
3. Existing suite: `cargo test -p worldwake-ai --test golden_merchant_selling`

### Invariants

1. Seller return does not leave displayed seller stock stranded in a `Displayed`-but-unlisted state once listing validity is lawfully restored.
2. The fix restores seller/facility/listing state directly, without buyer-side reseeding, compatibility shims, or duplicate market-state carriers.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-systems/src/trade.rs` — add focused authoritative proof that a displayed lot regains `SaleListing` when the seller returns.
2. `crates/worldwake-ai/tests/golden_merchant_selling.rs` — add seller-return golden proof at the truthful relist seam after buyer-side pending revival.

### Commands

1. `cargo test -p worldwake-systems trade::tests::displayed_listing_returns_when_seller_returns_to_market -- --exact`
2. `cargo test -p worldwake-ai --test golden_merchant_selling merchant_return_revives_pending_purchase_agenda_entry -- --exact`
3. `cargo test -p worldwake-ai --test golden_merchant_selling seller_return_restores_displayed_listing_after_pending_revival -- --exact`
4. `cargo test -p worldwake-ai --test golden_merchant_selling`
5. `cargo test -p worldwake-systems`
6. `cargo fmt --all`

## Outcome

Completed on 2026-04-22.

- `trade_system_tick` now synchronizes displayed-stock `SaleListing` state in both directions: invalid listings are pruned, and displayed lots that become lawfully sale-valid again regain `SaleListing` on seller return.
- Added focused `worldwake-systems` proof for the authoritative relist seam.
- Added merchant-selling golden coverage that proves seller return restores the displayed listing after buyer-side pending revival.

## Deviations

- The drafted stronger `...and the revived buyer trade completes` ending is still false on the live branch after the seller-side relist fix. The buyer later reaches a `trade` abort with `TradeBundleRejected { reason: InsufficientPayment }`.
- Follow-up ticket `S115AGEMAN-010` now owns that remaining resumed trade-completion seam.

## Verification Result

- Passed `cargo test -p worldwake-systems trade::tests::displayed_listing_returns_when_seller_returns_to_market -- --exact`
- Passed `cargo test -p worldwake-ai --test golden_merchant_selling merchant_return_revives_pending_purchase_agenda_entry -- --exact`
- Passed `cargo test -p worldwake-ai --test golden_merchant_selling seller_return_restores_displayed_listing_after_pending_revival -- --exact`
- Passed `cargo test -p worldwake-ai --test golden_merchant_selling`
- Passed `cargo test -p worldwake-systems`
- Passed `cargo fmt --all`
