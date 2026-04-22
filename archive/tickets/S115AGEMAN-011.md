# S115AGEMAN-011: resumed buyer purchase still aborts after live payload refresh

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes — mixed AI trade-payload synthesis / trade negotiation completion boundary after seller-return revival.
**Deps**: [archive/tickets/S115AGEMAN-010](./S115AGEMAN-010.md), [archive/tickets/S115AGEMAN-009](./S115AGEMAN-009.md), [archive/tickets/S115AGEMAN-008](./S115AGEMAN-008.md)

## Problem

`S115AGEMAN-010` fixed the earlier lower-layer contradiction: resumed pending-repair trade plans now refresh from the current live affordance instead of restoring stale payloads verbatim. Reassessment then showed the next still-live contradiction at the buyer opening-offer seam: planner fallback and live affordance synthesis could still underbid one-tick trade attempts after seller return. This ticket owns that narrower offer-synthesis alignment, not the broader resumed-purchase completion story.

## Assumption Reassessment (2026-04-22)

1. Buyer-side pending parking / revival already passes in `crates/worldwake-ai/tests/golden_merchant_selling.rs::merchant_return_revives_pending_purchase_agenda_entry`, and seller-side relist after return already passes in `crates/worldwake-ai/tests/golden_merchant_selling.rs::seller_return_restores_displayed_listing_after_pending_revival`.
2. `S115AGEMAN-010` already corrected the earlier AI/runtime seam: `resume_pending_repair_plan(...)` refreshes resumed trade payloads from the live affordance, and `revalidate_best_effort_payload_override_step(...)` no longer treats stale explicit payload variants as valid.
3. The shared abstraction boundary under audit narrowed to opening-offer synthesis parity: `GoalKind::AcquireCommodity` trade-step payload synthesis in [`crates/worldwake-ai/src/goal_model.rs`](../../crates/worldwake-ai/src/goal_model.rs) versus the live buyer opening-offer derivation in [`crates/worldwake-systems/src/trade_actions.rs`](../../crates/worldwake-systems/src/trade_actions.rs). The earlier `resume_pending_repair_plan(...)` payload-refresh seam already landed in `S115AGEMAN-010`.
4. Focused live proof now shows a real narrow fix inside that boundary. The revived buyer plan and the planner fallback both need to use the same one-tick-aware opening offer as the systems affordance query; otherwise resumed and synthesized trade steps underbid the live negotiation contract before runtime execution even starts.
5. The stronger resumed-completion ending is still false after that narrow fix. A focused golden probe showed the revived current plan now carries `offered_quantity = Quantity(3)`, but the authoritative `trade` action still aborts with buyer-side `InsufficientPayment` and no post-return `EventTag::Trade` commit.
6. The still-false premise is no longer the stale opening-offer seam this ticket owned. Follow-up ticket `S115AGEMAN-012` now owns the later authoritative pricing / seller-reservation contradiction exposed after this alignment.
7. This ticket must not reopen the already-fixed buyer pending-revival seam, seller relist seam, or the explicit-payload revalidation hardening from `S115AGEMAN-010` unless reassessment proves the remaining abort still depends on one of those surfaces.

## Architecture Check

1. The fix landed at the narrowest truthful contract that the current evidence proved false: buyer-side opening-offer synthesis must match the live affordance/runtime opening-offer math when `negotiation_round_ticks` leaves no room for concession.
2. No compatibility shim should preserve both “seller-return resumed purchase aborts after refreshed offer” and “seller-return resumed purchase completes” as parallel supported contracts.

## Verification Layers

1. Planner fallback `TradeActionPayload` synthesis uses the same opening offer as the live affordance path -> focused `goal_model` unit proof.
2. One-tick trade affordances surface the reservation-price opening offer, and that offer commits when the authoritative overlap really exists -> focused `worldwake-systems` trade harness proofs.
3. Seller-return relist and buyer-side revival remain intact while the narrower offer-synthesis fix lands -> existing `seller_return_restores_displayed_listing_after_pending_revival` golden and full `golden_merchant_selling` suite.
4. The stronger resumed-purchase completion golden remains out of scope until a later ticket proves that broader end state truthfully.

## What to Change

### 1. Bind the refreshed resumed offer to the exact authoritative abort boundary

Trace the refreshed resumed buyer trade from the live offer into the exact opening-offer synthesis boundary that still underbids the current authoritative trade budget.

### 2. Land the narrowest truthful production fix

Fix the exact seam that live proof still showed false: align planner-fallback and affordance-side buyer opening-offer synthesis so one-tick trade attempts start at the lawful live reservation price instead of an underbidding opening offer.

### 3. Extend the golden only if the live branch supports it

Broader resumed-purchase completion is deferred. If live evidence still disproves that stronger ending after the opening-offer fix, rewrite the ticket to the narrower landed seam and split the remaining authoritative contradiction to a follow-up instead of forcing the golden through.

## Files to Touch

- `crates/worldwake-ai/src/goal_model.rs` (modify)
- `crates/worldwake-systems/src/trade_actions.rs` (modify)
- `tickets/S115AGEMAN-011.md` (modify)
- `tickets/S115AGEMAN-012.md` (new)

## Out of Scope

- Reopening seller-side authoritative relist logic already fixed by `S115AGEMAN-009`
- Reopening buyer-side pending parking / revival logic already fixed by `S115AGEMAN-008`
- Reopening explicit-payload revalidation / resume refresh hardening already fixed by `S115AGEMAN-010`

## Acceptance Criteria

### Tests That Must Pass

1. Focused proof that planner-fallback trade payload synthesis now matches the live opening-offer contract.
2. Existing seller-side relist golden remains green.
3. Existing suite: `cargo test -p worldwake-ai --test golden_merchant_selling`

### Invariants

1. Planner-fallback trade steps and live affordance payloads use the same buyer opening-offer contract; no stale hardcoded one-coin path remains at that boundary.
2. The revived trade payload still uses real seller/facility/listing/trade state, not ad hoc reseeding or compatibility shims.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/goal_model.rs` — prove planner-fallback trade payload synthesis uses the live opening offer.
2. `crates/worldwake-systems/src/trade_actions.rs` — prove one-tick affordances surface the reservation-price opening offer and that the same offer commits when overlap exists.
3. `None — the stronger resumed-completion golden remained false after the narrow fix, so no new golden landed in this ticket.`

### Commands

1. `cargo test -p worldwake-ai --lib goal_model::tests::acquire_goal_builds_trade_payload_override_from_goal_semantics -- --exact`
2. `cargo test -p worldwake-systems --lib trade_actions::tests::trade_affordance_uses_reservation_price_when_deadline_is_one_tick -- --exact`
3. `cargo test -p worldwake-systems --lib trade_actions::tests::single_tick_trade_commits_when_opening_offer_matches_overlap_price -- --exact`
4. `cargo test -p worldwake-ai --test golden_merchant_selling`

## Outcome

Completed on 2026-04-22.

- `crates/worldwake-systems/src/trade_actions.rs` now centralizes buyer opening-offer derivation in a reusable helper and makes one-tick affordances open at the actor's full live reservation price.
- `crates/worldwake-ai/src/goal_model.rs::build_payload_override(...)` now uses that same live opening-offer helper instead of synthesizing a stale fixed one-coin trade payload.
- The landed lower-layer seam is truthful: the revived/current trade plan now carries `offered_quantity = Quantity(3)` instead of underbidding the live one-tick trade contract.

## Deviations

- The drafted stronger “seller return resumed purchase completes” ending is still false after the opening-offer alignment. Focused golden repro showed repeated buyer-side `InsufficientPayment` aborts with no post-return trade commit even after the revived/current plan carried `offered_quantity = Quantity(3)`.
- Follow-up ticket [`S115AGEMAN-012`](../../tickets/S115AGEMAN-012.md) now owns that later authoritative pricing contradiction.

## Verification Result

- Passed `cargo test -p worldwake-ai --lib goal_model::tests::acquire_goal_builds_trade_payload_override_from_goal_semantics -- --exact`
- Passed `cargo test -p worldwake-systems --lib trade_actions::tests::trade_affordance_uses_reservation_price_when_deadline_is_one_tick -- --exact`
- Passed `cargo test -p worldwake-systems --lib trade_actions::tests::single_tick_trade_commits_when_opening_offer_matches_overlap_price -- --exact`
- Passed `cargo test -p worldwake-ai --test golden_merchant_selling`
