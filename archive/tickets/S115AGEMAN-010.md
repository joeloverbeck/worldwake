# S115AGEMAN-010: resumed pending-repair trade should refresh live payload variants before retry

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes — AI plan-revalidation / pending-repair resumption boundary for explicit payload-variant steps.
**Deps**: [archive/tickets/S115AGEMAN-009](./S115AGEMAN-009.md), [archive/tickets/S115AGEMAN-008](./S115AGEMAN-008.md)

## Problem

`S115AGEMAN-009` restored the seller-side authoritative relist seam: after seller departure prunes a displayed lot's `SaleListing`, seller return now restores that listing lawfully. Reassessment of the still-false resumed-purchase story showed an earlier live contradiction before the later `InsufficientPayment` abort: the buyer-side pending-repair path could restore a failed `trade` plan with a stale payload, and AI revalidation treated explicit payload-variant steps as valid even when the current live affordance had moved to a different concrete payload. This ticket owns that lower-layer runtime/AI contract. The broader “resumed purchase completes after seller return” ending remains a separate follow-up seam.

## Assumption Reassessment (2026-04-22)

1. Buyer-side pending parking / revival already passes in `crates/worldwake-ai/tests/golden_merchant_selling.rs::merchant_return_revives_pending_purchase_agenda_entry`, and seller-side relist after return already passes in `crates/worldwake-ai/tests/golden_merchant_selling.rs::seller_return_restores_displayed_listing_after_pending_revival` plus focused `crates/worldwake-systems/src/trade.rs::tests::displayed_listing_returns_when_seller_returns_to_market`.
2. The shared abstraction boundary under audit is the AI/runtime seam that restores a failed `Trade` step from `runtime.pending_repair_context.failed_plan` back into `runtime.current_plan`, plus the `revalidate_best_effort_payload_override_step` fallback in `crates/worldwake-ai/src/plan_revalidation.rs`.
3. The first live contradiction was not yet the later negotiation abort. `resume_pending_repair_plan(...)` in `crates/worldwake-ai/src/agent_tick/planning.rs` could clone a failed trade plan back into `current_plan` without rebinding explicit payload variants from the current affordance, while `revalidate_best_effort_payload_override_step(...)` could still treat that stale payload as valid whenever the payload validator accepted it.
4. Focused lower-layer proof now exists for both corrected seams: `plan_revalidation::tests::explicit_trade_payload_variants_require_exact_affordance_match` proves explicit payload-variant actions no longer survive revalidation through validator-only fallback, and `agent_tick::planning::tests::resume_pending_repair_plan_restores_failed_trade_plan_when_counterparty_trigger_revives` proves resumed trade plans refresh their payload from the live affordance before re-entering `current_plan`.
5. A post-fix golden probe still disproves the stronger completion ending. After seller return, the buyer now carries the refreshed live offer (`offered_quantity = Quantity(2)`), retries `trade`, aborts again, and re-parks the goal into `pending` with no bread acquired. That later trade-opening-offer / negotiation-completion seam is split to follow-up ticket `S115AGEMAN-011`.

## Architecture Check

1. The landed fix hardens the first false boundary directly: explicit payload-variant plans must track the current live affordance, not any merely validator-legal stale payload.
2. No compatibility shim or golden-only scaffolding was added. The old failed trade plan is refreshed in-place from the live affordance, and revalidation refuses validator-only fallback when explicit payload variants exist.

## Verification Layers

1. Explicit payload-variant steps no longer survive revalidation via validator-only fallback -> `crates/worldwake-ai/src/plan_revalidation.rs::tests::explicit_trade_payload_variants_require_exact_affordance_match`.
2. Pending-repair resumption refreshes the failed trade step from the live affordance before restoring `current_plan` -> `crates/worldwake-ai/src/agent_tick/planning.rs::tests::resume_pending_repair_plan_restores_failed_trade_plan_when_counterparty_trigger_revives`.
3. Seller-side relist and buyer-side revival remain intact while this lower-layer fix lands -> existing `crates/worldwake-ai/tests/golden_merchant_selling.rs::seller_return_restores_displayed_listing_after_pending_revival` plus the full `golden_merchant_selling` suite.
4. The later resumed trade completion seam remains owned by follow-up `tickets/S115AGEMAN-011.md`, not overclaimed here.

## What to Change

### 1. Tighten revalidation for explicit payload-variant steps

Do not let `revalidate_best_effort_payload_override_step(...)` treat a stale payload as valid when the live action family currently enumerates concrete payload variants through affordances.

### 2. Refresh pending-repair trade payloads from the live affordance on resume

When `resume_pending_repair_plan(...)` restores a failed trade plan after the counterparty revival trigger fires, rebind its explicit payload-variant steps from the current live affordance before restoring `current_plan`.

### 3. Split the still-false completion seam truthfully

If the broader resumed purchase still aborts after the lower-layer fix, record that evidence here and move the remaining opening-offer / negotiation-completion contradiction to a follow-up ticket instead of forcing a false golden.

## Files to Touch

- `crates/worldwake-ai/src/plan_revalidation.rs` (modify)
- `crates/worldwake-ai/src/agent_tick/planning.rs` (modify)
- `specs/S115-agenda-manager.md` (modify)
- `tickets/S115AGEMAN-010.md` (modify)
- `tickets/S115AGEMAN-011.md` (new)

## Out of Scope

- Reopening seller-side authoritative relist logic already fixed by `S115AGEMAN-009`
- Reopening buyer-side pending parking / revival logic already fixed by `S115AGEMAN-008`
- Forcing a false merchant-selling completion golden while the refreshed live offer still aborts

## Acceptance Criteria

### Tests That Must Pass

1. Focused proof that explicit payload-variant actions no longer revalidate through validator-only fallback when a different live affordance payload exists.
2. Focused proof that pending-repair trade resumption refreshes the failed plan payload from the current live affordance.
3. Existing seller-return relist golden remains green.
4. Existing suite: `cargo test -p worldwake-ai --test golden_merchant_selling`

### Invariants

1. Explicit payload-variant plans cannot keep executing against stale payloads solely because the payload validator still accepts them.
2. Pending-repair resumption reuses the live affordance payload rather than restoring a stale failed trade payload verbatim.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/plan_revalidation.rs` — prove explicit payload-variant steps require an exact live affordance match instead of validator-only fallback.
2. `crates/worldwake-ai/src/agent_tick/planning.rs` — prove resumed failed trade plans refresh to the live affordance payload before re-entering `current_plan`.

### Commands

1. `cargo test -p worldwake-ai --lib plan_revalidation::tests::explicit_trade_payload_variants_require_exact_affordance_match -- --exact`
2. `cargo test -p worldwake-ai --lib agent_tick::planning::tests::resume_pending_repair_plan_restores_failed_trade_plan_when_counterparty_trigger_revives -- --exact`
3. `cargo test -p worldwake-ai --test golden_merchant_selling seller_return_restores_displayed_listing_after_pending_revival -- --exact`
4. `cargo test -p worldwake-ai --test golden_merchant_selling`

## Outcome

Completed on 2026-04-22.

- `revalidate_best_effort_payload_override_step(...)` now rejects validator-only fallback for steps whose live action family enumerates explicit affordance payload variants.
- `resume_pending_repair_plan(...)` now refreshes resumed trade payloads from the current live affordance before restoring the failed plan into `runtime.current_plan`.
- Added focused unit coverage for both seams.

## Deviations

- The drafted stronger “seller returns and the resumed buyer completes the purchase” ending is still false after the lower-layer fix. A post-fix golden probe showed the buyer now carries the refreshed live offer (`Quantity(2)`), retries `trade`, aborts again, and re-parks the goal into `pending`.
- Follow-up ticket `tickets/S115AGEMAN-011.md` now owns that remaining opening-offer / negotiation-completion seam.

## Verification Result

- Passed `cargo test -p worldwake-ai --lib plan_revalidation::tests::explicit_trade_payload_variants_require_exact_affordance_match -- --exact`
- Passed `cargo test -p worldwake-ai --lib agent_tick::planning::tests::resume_pending_repair_plan_restores_failed_trade_plan_when_counterparty_trigger_revives -- --exact`
- Passed `cargo test -p worldwake-ai --test golden_merchant_selling seller_return_restores_displayed_listing_after_pending_revival -- --exact`
- Passed `cargo test -p worldwake-ai --test golden_merchant_selling`
