# S115AGEMAN-011: resumed buyer purchase still aborts after live payload refresh

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes — mixed AI trade-payload synthesis / trade negotiation completion boundary after seller-return revival.
**Deps**: [archive/tickets/S115AGEMAN-010](../archive/tickets/S115AGEMAN-010.md), [archive/tickets/S115AGEMAN-009](../archive/tickets/S115AGEMAN-009.md), [archive/tickets/S115AGEMAN-008](../archive/tickets/S115AGEMAN-008.md)

## Problem

`S115AGEMAN-010` fixed the earlier lower-layer contradiction: resumed pending-repair trade plans now refresh from the current live affordance instead of restoring stale payloads verbatim. But the broader resumed-purchase ending is still false. A post-fix merchant-selling probe showed the buyer now carries the refreshed live trade payload (`offered_quantity = Quantity(2)`), retries `trade` after seller return, aborts again, and re-parks the `AcquireCommodity(Bread)` goal into `pending` with no bread acquired.

## Assumption Reassessment (2026-04-22)

1. Buyer-side pending parking / revival already passes in `crates/worldwake-ai/tests/golden_merchant_selling.rs::merchant_return_revives_pending_purchase_agenda_entry`, and seller-side relist after return already passes in `crates/worldwake-ai/tests/golden_merchant_selling.rs::seller_return_restores_displayed_listing_after_pending_revival`.
2. `S115AGEMAN-010` already corrected the earlier AI/runtime seam: `resume_pending_repair_plan(...)` refreshes resumed trade payloads from the live affordance, and `revalidate_best_effort_payload_override_step(...)` no longer treats stale explicit payload variants as valid.
3. The shared abstraction boundary now under audit is narrower and later: `GoalKind::AcquireCommodity` trade-step payload synthesis in `crates/worldwake-ai/src/goal_model.rs::build_payload_override(...)` versus the live trade-opening-offer / negotiation patience contract in `crates/worldwake-systems/src/trade_actions.rs::{enumerate_trade_payloads, derive_opening_offer, urgency_modulated_deadline, tick_trade}`.
4. The motivating merchant-selling probe is mixed-layer. The buyer revives and reaches a refreshed `trade` payload, but the authoritative trade action still aborts before commit. The intended invariant is now “after seller return and live payload refresh, the buyer can lawfully complete the resumed purchase,” not merely “the buyer retries trade.”
5. The failed post-fix probe established concrete live facts that this ticket must preserve: `trade_aborted = true`, `pending_after_return = true`, `bread_qty = Quantity(0)`, and the refreshed current plan carried `offered_quantity = Quantity(2)` before the abort.
6. This ticket must not reopen the already-fixed buyer pending-revival seam, seller relist seam, or the explicit-payload revalidation hardening from `S115AGEMAN-010` unless reassessment proves the remaining abort still depends on one of those surfaces.

## Architecture Check

1. The fix should land at the narrowest truthful contract that makes the refreshed resumed trade abort: either trade-step payload synthesis chooses an opening offer that cannot satisfy the live negotiation budget, or the authoritative negotiation contract is too impatient for the refreshed offer to converge.
2. No compatibility shim should preserve both “seller-return resumed purchase aborts after refreshed offer” and “seller-return resumed purchase completes” as parallel supported contracts.

## Verification Layers

1. The refreshed resumed plan carries the exact intended opening offer into the trade step -> focused AI/search/runtime proof at the payload-synthesis seam.
2. The authoritative trade action either converges and commits or aborts for a precisely identified reason -> focused `worldwake-systems` trade harness proof.
3. Seller-return relist and buyer-side revival remain intact while the later completion fix lands -> existing `seller_return_restores_displayed_listing_after_pending_revival` golden and full `golden_merchant_selling` suite.
4. Extend merchant-selling golden coverage to actual resumed purchase completion only if the live branch becomes truthful; otherwise keep proof at the strongest lower layer and narrow the ticket again.

## What to Change

### 1. Bind the refreshed resumed offer to the exact authoritative abort boundary

Trace the refreshed resumed buyer trade from the live offer (`Quantity(2)` today) into the exact authoritative trade negotiation boundary that still aborts instead of committing.

### 2. Land the narrowest truthful production fix

Fix whichever exact seam owns the false completion contract: `GoalKind::AcquireCommodity` trade payload synthesis, the trade affordance opening-offer derivation, negotiation patience, or another directly evidenced boundary.

### 3. Extend the golden only if the live branch supports it

If the broader resumed story becomes truthful, extend `crates/worldwake-ai/tests/golden_merchant_selling.rs` to prove actual resumed trade completion after seller return. If live evidence disproves that stronger ending again, rewrite this ticket before closeout instead of forcing the golden through.

## Files to Touch

- `crates/worldwake-ai/src/goal_model.rs` (modify if AI-side trade payload synthesis owns the remaining abort)
- `crates/worldwake-systems/src/trade_actions.rs` (modify if the authoritative opening-offer / negotiation contract owns the remaining abort)
- `crates/worldwake-ai/tests/golden_merchant_selling.rs` (modify only if the stronger completion story becomes truthful)

## Out of Scope

- Reopening seller-side authoritative relist logic already fixed by `S115AGEMAN-009`
- Reopening buyer-side pending parking / revival logic already fixed by `S115AGEMAN-008`
- Reopening explicit-payload revalidation / resume refresh hardening already fixed by `S115AGEMAN-010`

## Acceptance Criteria

### Tests That Must Pass

1. Focused proof at the exact opening-offer / negotiation / commit seam that currently aborts after seller return.
2. Existing seller-side relist golden remains green.
3. Existing suite: `cargo test -p worldwake-ai --test golden_merchant_selling`

### Invariants

1. Once seller-side relist and buyer-side revival have restored a lawful resumed trade attempt, the buyer-side trade payload and authoritative negotiation contract do not strand the goal in an abort-and-repark loop.
2. The resumed completion path uses real seller/facility/listing/trade state, not ad hoc reseeding or compatibility shims.

## Test Plan

### New/Modified Tests

1. Focused lower-layer proof at the exact remaining resumed-trade abort seam.
2. `crates/worldwake-ai/tests/golden_merchant_selling.rs` — extend to actual resumed trade completion only if the live branch now supports that contract.

### Commands

1. `cargo test -p worldwake-ai --test golden_merchant_selling -- --list`
2. `cargo test -p worldwake-ai <focused-selector>`
3. `cargo test -p worldwake-ai --test golden_merchant_selling`
