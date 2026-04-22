# S115AGEMAN-012: seller-return resumed purchase still aborts after live three-coin offer

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes — authoritative trade reservation / buyer-budget overlap after seller-return revival.
**Deps**: [archive/tickets/S115AGEMAN-011](../archive/tickets/S115AGEMAN-011.md), [archive/tickets/S115AGEMAN-010](../archive/tickets/S115AGEMAN-010.md), [archive/tickets/S115AGEMAN-009](../archive/tickets/S115AGEMAN-009.md), [archive/tickets/S115AGEMAN-008](../archive/tickets/S115AGEMAN-008.md)

## Problem

`S115AGEMAN-011` aligned buyer opening-offer synthesis across planner fallback and live affordance enumeration. After that fix, the revived/current `AcquireCommodity(Bread)` trade step now carries `offered_quantity = Quantity(3)` after seller return instead of the older underbidding offer. The broader resumed-purchase ending is still false: the authoritative `trade` action keeps aborting with buyer-side `InsufficientPayment`, emits no post-return `EventTag::Trade` commit, and re-parks the goal into `pending`.

## Assumption Reassessment (2026-04-22)

1. Buyer-side pending parking / revival already passes in `crates/worldwake-ai/tests/golden_merchant_selling.rs::merchant_return_revives_pending_purchase_agenda_entry`, and seller-side relist after return already passes in `crates/worldwake-ai/tests/golden_merchant_selling.rs::seller_return_restores_displayed_listing_after_pending_revival`.
2. `S115AGEMAN-010` already fixed the earlier stale-payload seam: resumed trade plans refresh from the live affordance before re-entering `current_plan`.
3. `S115AGEMAN-011` already fixed the next lower layer: `crates/worldwake-ai/src/goal_model.rs::build_payload_override(...)` and `crates/worldwake-systems/src/trade_actions.rs` now agree on a revived/current opening offer of `Quantity(3)` for the motivating bread purchase.
4. The shared abstraction boundary now under audit is later and authoritative: buyer budget / opening offer versus seller reservation and negotiation acceptance inside `crates/worldwake-systems/src/trade_actions.rs::{tick_trade, reservation_price_for_actor, seller_reservation_price_for_view}`.
5. The motivating merchant-selling probe remains mixed-layer. The revived current plan is now truthful at the payload seam, but the authoritative trade action still aborts before commit. The intended invariant is now “after seller return and live three-coin offer alignment, the resumed purchase can lawfully commit,” not merely “the revived plan carries the corrected offer.”
6. Focused golden evidence from the exploratory `seller_return_completes_resumed_purchase_after_live_offer_refresh` probe established the concrete live facts this ticket must preserve: the revived/current plan carried `offered_quantity = Quantity(3)`, no post-return `EventTag::Trade` commit occurred within the probe window, and the buyer saw repeated `TradeBundleRejected { reason: InsufficientPayment }` aborts after seller return.
7. This ticket must not reopen seller relist, buyer pending revival, or payload-refresh parity unless reassessment proves the later `InsufficientPayment` abort still depends on one of those already-landed seams.

## Architecture Check

1. The fix should land at the narrowest truthful authoritative pricing seam that still makes the revived three-coin trade abort: seller reservation arithmetic, remembered-demand accumulation, self-need pricing, negotiation acceptance timing, or another directly evidenced trade-runtime boundary.
2. No compatibility shim should preserve both “seller-return resumed purchase aborts after live three-coin offer” and “seller-return resumed purchase commits” as parallel supported contracts.

## Verification Layers

1. The revived/current plan still carries the corrected three-coin opening offer into the trade step -> focused AI/runtime proof at the revived-plan payload seam.
2. The authoritative trade action either commits or aborts for a precisely identified reservation/acceptance reason -> focused `worldwake-systems` trade harness proof.
3. Seller-return relist and buyer-side revival remain intact while the later authoritative pricing fix lands -> existing `seller_return_restores_displayed_listing_after_pending_revival` golden and full `golden_merchant_selling` suite.
4. Extend merchant-selling golden coverage to actual resumed purchase completion only if the live branch becomes truthful; otherwise keep proof at the strongest lower layer and narrow the ticket again.

## What to Change

### 1. Bind the revived three-coin offer to the exact authoritative rejection boundary

Trace the revived/current buyer trade from `offered_quantity = Quantity(3)` into the exact seller-reservation / acceptance boundary that still emits `InsufficientPayment`.

### 2. Land the narrowest truthful production fix

Fix whichever exact authoritative seam owns the still-false completion contract: seller reservation arithmetic, remembered-demand accumulation, self-need pricing, negotiation acceptance ordering, or another directly evidenced trade-runtime boundary.

### 3. Extend the golden only if the live branch supports it

If the broader resumed story becomes truthful, extend `crates/worldwake-ai/tests/golden_merchant_selling.rs` to prove actual resumed trade completion after seller return. If live evidence disproves that stronger ending again, rewrite this ticket before closeout instead of forcing the golden through.

## Files to Touch

- `crates/worldwake-systems/src/trade_actions.rs` (modify if the authoritative reservation / acceptance contract owns the remaining abort)
- `crates/worldwake-ai/tests/golden_merchant_selling.rs` (modify only if the stronger completion story becomes truthful)
- `tickets/S115AGEMAN-012.md` (modify)

## Out of Scope

- Reopening seller-side authoritative relist logic already fixed by `S115AGEMAN-009`
- Reopening buyer-side pending parking / revival logic already fixed by `S115AGEMAN-008`
- Reopening stale-payload refresh / explicit-payload revalidation hardening already fixed by `S115AGEMAN-010`
- Reopening opening-offer synthesis parity already fixed by `S115AGEMAN-011`

## Acceptance Criteria

### Tests That Must Pass

1. Focused proof at the exact authoritative reservation / acceptance seam that still aborts after seller return even with the revived three-coin offer.
2. Existing seller-side relist golden remains green.
3. Existing suite: `cargo test -p worldwake-ai --test golden_merchant_selling`

### Invariants

1. Once seller-side relist, buyer-side revival, and live three-coin offer alignment have restored a lawful resumed trade attempt, the authoritative trade contract does not strand the goal in an `InsufficientPayment` abort-and-repark loop unless that pricing outcome is itself the truthful designed result.
2. Any resumed completion path still uses real seller/facility/listing/trade state, not ad hoc reseeding or compatibility shims.

## Test Plan

### New/Modified Tests

1. Focused lower-layer proof at the exact remaining authoritative resumed-trade abort seam.
2. `crates/worldwake-ai/tests/golden_merchant_selling.rs` — extend to actual resumed trade completion only if the live branch now supports that contract.

### Commands

1. `cargo test -p worldwake-ai --test golden_merchant_selling -- --list`
2. `cargo test -p worldwake-systems <focused-selector>`
3. `cargo test -p worldwake-ai --test golden_merchant_selling`
