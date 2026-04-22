# S115AGEMAN-012: seller-return resumed purchase completes after seller frustration stops inflating reservation price

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes — authoritative trade reservation / buyer-budget overlap after seller-return revival.
**Deps**: [archive/tickets/S115AGEMAN-011](./S115AGEMAN-011.md), [archive/tickets/S115AGEMAN-010](./S115AGEMAN-010.md), [archive/tickets/S115AGEMAN-009](./S115AGEMAN-009.md), [archive/tickets/S115AGEMAN-008](./S115AGEMAN-008.md)

## Problem

`S115AGEMAN-011` aligned buyer opening-offer synthesis across planner fallback and live affordance enumeration. After that fix, the revived/current `AcquireCommodity(Bread)` trade step carries `offered_quantity = Quantity(3)` after seller return, but the authoritative `trade` action still re-entered repeated buyer-side `InsufficientPayment` aborts instead of reaching the eventual resumed purchase commit.

## Assumption Reassessment (2026-04-22)

1. Buyer-side pending parking / revival already passes in `crates/worldwake-ai/tests/golden_merchant_selling.rs::merchant_return_revives_pending_purchase_agenda_entry`, and seller-side relist after return already passes in `crates/worldwake-ai/tests/golden_merchant_selling.rs::seller_return_restores_displayed_listing_after_pending_revival`.
2. `S115AGEMAN-010` already fixed the earlier stale-payload seam: resumed trade plans refresh from the live affordance before re-entering `current_plan`.
3. `S115AGEMAN-011` already fixed the next lower layer: `crates/worldwake-ai/src/goal_model.rs::build_payload_override(...)` and `crates/worldwake-systems/src/trade_actions.rs` now agree on a revived/current opening offer of `Quantity(3)` for the motivating bread purchase.
4. The shared abstraction boundary under audit narrowed to seller-side remembered-demand accumulation inside `crates/worldwake-systems/src/trade_actions.rs::{remembered_demand_pressure, seller_reservation_price, seller_reservation_price_for_view, tick_trade}`.
5. Focused runtime proof showed the later abort loop was real but narrower than the ticket draft suggested: seller-side `DemandObservationReason::WantedToSellButNoBuyer` entries were being counted as positive demand pressure for seller reservation pricing, so each failed retry could keep seller reservation elevated even though those observations describe missing buyers rather than buyer demand.
6. The motivating merchant-selling probe remained mixed-layer but became truthful after that narrow reservation fix. The revived/current plan still carries `offered_quantity = Quantity(3)` and `requested_quantity = Quantity(1)`, and the resumed purchase now eventually commits after seller return at that live three-coin unit-purchase price instead of remaining stuck in a permanent abort loop.
7. This ticket must not reopen seller relist, buyer pending revival, or payload-refresh parity unless reassessment proves the remaining behavior still depends on one of those already-landed seams.

## Architecture Check

1. The fix landed at the narrowest truthful authoritative pricing seam that the live branch proved false: seller-side `WantedToSellButNoBuyer` frustration memory must not inflate remembered demand pressure for reservation pricing.
2. No compatibility shim should preserve both “seller-return resumed purchase aborts after live three-coin offer” and “seller-return resumed purchase commits” as parallel supported contracts.

## Verification Layers

1. Seller-side no-buyer frustration memory no longer raises reservation pricing above an already overlapping offer -> focused `worldwake-systems` trade proof.
2. The revived/current plan still carries the corrected three-coin opening offer into the trade step, and the resumed purchase now eventually commits after seller return -> focused merchant-selling golden.
3. Seller-return relist and buyer-side revival remain intact while the later authoritative pricing fix lands -> existing `seller_return_restores_displayed_listing_after_pending_revival` golden and full `golden_merchant_selling` suite.

## What to Change

### 1. Bind the revived three-coin offer to the exact authoritative rejection boundary

Trace the revived/current buyer trade from `offered_quantity = Quantity(3)` into the exact seller-reservation boundary that still emitted `InsufficientPayment`.

### 2. Land the narrowest truthful production fix

Fix the exact seam that live proof still showed false: exclude seller-side `WantedToSellButNoBuyer` frustration observations from seller remembered-demand pressure so reservation pricing only reflects actual buyer demand.

### 3. Extend the golden only if the live branch supports it

Extend `crates/worldwake-ai/tests/golden_merchant_selling.rs` to prove actual resumed trade completion after seller return once the reservation fix lands.

## Files to Touch

- `crates/worldwake-systems/src/trade_actions.rs` (modify if the authoritative reservation / acceptance contract owns the remaining abort)
- `crates/worldwake-ai/tests/golden_merchant_selling.rs` (modify)
- `tickets/S115AGEMAN-012.md` (modify)

## Out of Scope

- Reopening seller-side authoritative relist logic already fixed by `S115AGEMAN-009`
- Reopening buyer-side pending parking / revival logic already fixed by `S115AGEMAN-008`
- Reopening stale-payload refresh / explicit-payload revalidation hardening already fixed by `S115AGEMAN-010`
- Reopening opening-offer synthesis parity already fixed by `S115AGEMAN-011`

## Acceptance Criteria

### Tests That Must Pass

1. Focused proof that seller-side `WantedToSellButNoBuyer` frustration memory no longer inflates reservation pricing above an already overlapping offer.
2. Existing seller-side relist golden remains green.
3. Existing suite: `cargo test -p worldwake-ai --test golden_merchant_selling`

### Invariants

1. Seller reservation pricing reflects actual buyer-demand observations, not seller-side “no buyer” frustration records.
2. Any resumed completion path still uses real seller/facility/listing/trade state, not ad hoc reseeding or compatibility shims.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-systems/src/trade_actions.rs` — prove seller-side no-buyer frustration memory does not raise reservation pricing above an already overlapping trade.
2. `crates/worldwake-ai/tests/golden_merchant_selling.rs` — prove the revived three-coin unit purchase eventually commits after seller return.

### Commands

1. `cargo test -p worldwake-systems --lib trade_actions::tests::seller_no_buyer_memory_does_not_raise_reservation_above_overlapping_price -- --exact`
2. `cargo test -p worldwake-systems --lib trade_actions::tests::single_tick_trade_commits_when_opening_offer_matches_overlap_price -- --exact`
3. `cargo test -p worldwake-ai --test golden_merchant_selling seller_return_completes_resumed_purchase_after_live_offer_refresh -- --exact`
4. `cargo test -p worldwake-ai --test golden_merchant_selling`

## Outcome

Completed on 2026-04-22.

- `crates/worldwake-systems/src/trade_actions.rs::remembered_demand_pressure(...)` now counts only actual buyer-demand observations when seller reservation pricing is derived, excluding seller-side `WantedToSellButNoBuyer` frustration records from reservation inflation.
- Added focused regression `trade_actions::tests::seller_no_buyer_memory_does_not_raise_reservation_above_overlapping_price` so the authoritative reservation seam stays pinned.
- Added merchant-selling golden `seller_return_completes_resumed_purchase_after_live_offer_refresh`, which proves the revived purchase now eventually commits after seller return at the live three-coin unit-purchase price.

## Deviations

- The resumed purchase does not immediately commit on seller return. The revived plan still requests `Quantity(1)` bread for `Quantity(3)` coin, and the trade only commits once older positive demand memory ages out of the seller's retention window.
- Because the live committed purchase remains a one-bread unit purchase rather than a full-lot buy, the golden now proves eventual resumed completion at the truthful unit-purchase contract instead of a same-tick full-lot commit.

## Verification Result

- Passed `cargo test -p worldwake-systems --lib trade_actions::tests::seller_no_buyer_memory_does_not_raise_reservation_above_overlapping_price -- --exact`
- Passed `cargo test -p worldwake-systems --lib trade_actions::tests::single_tick_trade_commits_when_opening_offer_matches_overlap_price -- --exact`
- Passed `cargo test -p worldwake-ai --test golden_merchant_selling seller_return_completes_resumed_purchase_after_live_offer_refresh -- --exact`
- Passed `cargo test -p worldwake-ai --test golden_merchant_selling`
