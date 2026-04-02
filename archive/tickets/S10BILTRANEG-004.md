# S10BILTRANEG-004: Modified enumerate_trade_payloads with variable-price offers

**Status**: ✅ COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — `worldwake-systems` (affordance generation, payload override validator)
**Deps**: S10BILTRANEG-001, S10BILTRANEG-002, S10BILTRANEG-003

## Problem

`enumerate_trade_payloads` at `crates/worldwake-systems/src/trade_actions.rs:81-127` hardcodes every trade offer as `offered_quantity: Quantity(1), requested_quantity: Quantity(1)`. This means agents can never offer more than 1 coin for a commodity, causing infinite rejection loops when sellers value their goods above 1 coin. The affordance generation must compute belief-derived opening offers using reservation prices and `TradeDispositionProfile` parameters.

## Assumption Reassessment (2026-04-02)

1. `enumerate_trade_payloads` at `trade_actions.rs:81-127` receives `&dyn RuntimeBeliefView`. It iterates `profile.sale_kinds` → `listed_sale_lots_at(place, commodity)` → individual lots. Each lot gets a `TradeActionPayload` with `sale_lot: EntityId`. This lot-based iteration pattern must be preserved.
2. `trade_bundle_is_mutually_accepted` at `trade_actions.rs:324-354` is the pre-filter that will be removed. It calls `evaluate_trade_bundle` for both buyer and seller from the buyer's belief view. Removing it means payloads are emitted whenever the buyer can afford ≥1, not only when both sides would accept the fixed 1:1 price.
3. `RuntimeBeliefView` exposes `homeostatic_needs`, `commodity_quantity`, `trade_disposition_profile`, `wounds`, `demand_memory`, `listed_sale_lots_at`, `seller_for_sale_lot`, `merchandise_profile`, `entities_at`, `entity_kind` — all needed for the rewrite.
4. The existing `wounds_for(view, actor)` helper at `trade_actions.rs:385-388` constructs `Option<WoundList>` from the view's `Vec<Wound>`. The existing `demand_memory_for(view, actor)` helper at `trade_actions.rs:390-396` constructs `Option<DemandMemory>`. Both are reusable.
5. Authoritative-to-AI Impact Rule check: `enumerate_trade_payloads` is the affordance payload function. Changes here affect what payloads the planner can synthesize. The payload override validator (currently not registered for trade actions — `register_trade_action` at line 20-25 chains `.with_affordance_payloads` but not `.with_payload_override_validator`) must be added or the planner's synthesized payloads will silently fail revalidation in `plan_revalidation.rs`.
6. `DemandObservationReason::WantedToBuyButTooExpensive` is the variant used to count prior rejections. `DemandObservation.counterparty` is `Option<EntityId>` — filtering by counterparty and commodity is straightforward.

## Architecture Check

1. Removing the mutual acceptance pre-filter aligns with Principle 16 (ignorance is first-class). The buyer doesn't know the seller's reservation and shouldn't pretend to. Failed negotiations produce learning (Principle 10).
2. Computing opening offers from belief state preserves Principle 14 (belief-only planning). The reservation price functions from ticket 002 accept belief-derived inputs.
3. Adding `with_payload_override_validator` ensures planner-synthesized payloads are validated before execution, closing the revalidation gap identified during reassessment.
4. No backward-compatibility shims. `trade_bundle_is_mutually_accepted` is deleted, not deprecated.

## Verification Layers

1. Variable-price payloads are generated -> focused unit/integration test (affordance enumeration returns payloads with offered_quantity > 1 when urgency is high)
2. No payloads generated when buyer has 0 coins -> focused unit test
3. Opening offer respects initial_offer_bias + rejection history -> focused unit test
4. Payload override validator accepts valid variable prices, rejects invalid -> focused unit test
5. `trade_bundle_is_mutually_accepted` is fully removed -> grep verification (no remaining call sites)

## What to Change

### 1. Add `rejection_count_for` helper

```rust
fn rejection_count_for(
    view: &dyn RuntimeBeliefView,
    actor: EntityId,
    counterparty: EntityId,
    commodity: CommodityKind,
) -> u32
```

Counts `DemandObservation` entries in the actor's demand memory with reason `WantedToBuyButTooExpensive`, matching counterparty and commodity.

### 2. Rewrite `enumerate_trade_payloads`

Replace the current implementation with:
1. Validate counterparty, place, merchandise profile (same as current).
2. Check buyer has ≥1 coin (same as current).
3. Get `TradeDispositionProfile` for actor. Return empty if absent.
4. For each commodity in `profile.sale_kinds`:
   a. Compute `buyer_reservation_price` from belief state.
   b. If reservation < 1, skip (can't afford).
   c. Compute `derive_opening_offer` from reservation + bias + rejection history.
   d. For each `listed_sale_lots_at(place, commodity)` belonging to counterparty, emit a `TradeActionPayload` with `offered_quantity: opening_offer`.
5. Sort and dedup payloads (same as current).

Remove the call to `trade_bundle_is_mutually_accepted`.

### 3. Delete `trade_bundle_is_mutually_accepted`

Remove the function entirely. It is no longer called. If any other call sites exist (verify via grep), update them.

### 4. Register payload override validator

Add `.with_payload_override_validator(validate_trade_payload_override)` to the `ActionHandler::new` chain in `register_trade_action`.

Implement `validate_trade_payload_override`:
```rust
fn validate_trade_payload_override(
    _def: &ActionDef,
    actor: EntityId,
    _targets: &[EntityId],
    payload: &ActionPayload,
    view: &dyn RuntimeBeliefView,
) -> bool
```

Accept the payload if:
- `offered_quantity` ≥ 1
- `offered_quantity` ≤ buyer's coin balance (from belief view)
- `offered_commodity` is `CommodityKind::Coin`

## Files to Touch

- `crates/worldwake-systems/src/trade_actions.rs` (modify — rewrite `enumerate_trade_payloads`, add `rejection_count_for`, delete `trade_bundle_is_mutually_accepted`, add `validate_trade_payload_override`, register it in `register_trade_action`)

## Out of Scope

- Negotiation round logic in `tick_trade` (ticket 005)
- Commit/abort behavior changes (ticket 005)
- Golden test creation (ticket 006)
- Changes to `evaluate_trade_bundle` or `trade_valuation.rs`

## Acceptance Criteria

### Tests That Must Pass

1. `enumerate_trade_payloads` returns payloads with `offered_quantity > Quantity(1)` when buyer has high urgency and coins
2. `enumerate_trade_payloads` returns empty when buyer has 0 coins
3. `enumerate_trade_payloads` returns payloads even when the seller would reject the fixed 1:1 price (mutual acceptance filter removed)
4. `rejection_count_for` returns correct count for specific counterparty/commodity
5. `validate_trade_payload_override` accepts valid payloads and rejects `offered_quantity: Quantity(0)` or exceeding coin balance
6. `trade_bundle_is_mutually_accepted` no longer exists in the codebase
7. All existing golden tests pass: `cargo test -p worldwake-ai`
8. Full suite: `cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings`

### Invariants

1. Affordance generation uses belief views only (Principle 14) — no `WorldTxn` access.
2. Opening offers are deterministic given the same belief state.
3. The lot-based iteration pattern (`sale_kinds` → `listed_sale_lots_at` → individual lots) is preserved.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-systems/src/trade_actions.rs` (new `#[cfg(test)] mod affordance_tests`) — unit tests for rewritten `enumerate_trade_payloads`, `rejection_count_for`, `validate_trade_payload_override`

### Commands

1. `cargo test -p worldwake-systems -- affordance` — targeted affordance tests
2. `cargo test -p worldwake-systems -- trade` — all trade tests
3. `cargo test -p worldwake-ai` — golden tests (regression check)
4. `cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings` — full suite

## Outcome

**Completed**: 2026-04-02

Implemented variable-price trade affordance generation in `crates/worldwake-systems/src/trade_actions.rs`. `enumerate_trade_payloads` now derives buyer opening offers from belief-state reservation pricing, local alternatives, trade-disposition bias, and `WantedToBuyButTooExpensive` rejection history instead of hardcoding `Quantity(1)`. The stale `trade_bundle_is_mutually_accepted` affordance filter was removed, `rejection_count_for` was added, and the trade action now registers `validate_trade_payload_override` so planner-synthesized trade payloads revalidate against the live coin-budget boundary.

Focused affordance and override-validation tests were added in `trade_actions.rs`, and the only downstream fallout was a stale AI search-trace harness assumption in `crates/worldwake-ai/src/search/tests.rs`. That harness was updated to reflect the new admission contract: without a `TradeDispositionProfile`, trade root candidates are now omitted at affordance generation instead of surviving until duration estimation.

**Deviations from original plan**:
1. The implementation touched `crates/worldwake-ai/src/search/tests.rs` in addition to the planned `trade_actions.rs` surface because the AI regression was a stale trace expectation, not a production contradiction.
2. The old `demand_memory_for` helper remained in place but is now explicitly marked as staged/unused after this ticket because the new affordance path reads rejection observations directly.

**Verification results**:
1. `cargo test -p worldwake-systems trade_affordance_ -- --nocapture`
2. `cargo test -p worldwake-systems rejection_count_for -- --nocapture`
3. `cargo test -p worldwake-systems trade_payload_override_validator -- --nocapture`
4. `cargo test -p worldwake-systems -- trade`
5. `cargo test -p worldwake-ai search_trace_omits_trade_root_candidate_without_trade_disposition_profile -- --nocapture`
6. `cargo test -p worldwake-ai`
7. `cargo test --workspace`
8. `cargo clippy --workspace --all-targets -- -D warnings`
