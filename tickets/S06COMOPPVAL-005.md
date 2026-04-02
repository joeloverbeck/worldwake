# S06COMOPPVAL-005: Integrate shared layer into evaluate_trade_bundle and reservation prices

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — `worldwake-sim` (trade_valuation.rs signature change), `worldwake-systems` (trade_actions.rs — pass RecipeRegistry through)
**Deps**: S06COMOPPVAL-003, S06COMOPPVAL-004

## Problem

`evaluate_trade_bundle` currently computes commodity value using only direct channels (survival, treatment, demand, coin). It does not consider indirect recipe value — a merchant will sell their last firewood for 1 coin even though firewood + grain → bread worth 3 coins of hunger relief. Post-S10, this function is called during negotiation rounds via `evaluate_for_participant`. The reservation price functions (`buyer_reservation_price`, `seller_reservation_price`) also lack recipe awareness. This ticket wires the shared `commodity_opportunity_score` into both valuation paths.

## Assumption Reassessment (2026-04-02)

1. `evaluate_trade_bundle` at `crates/worldwake-sim/src/trade_valuation.rs` takes `(actor, belief, needs, wounds, current_coin, offered, received, local_alternatives, demand_memory)`. It does NOT take `RecipeRegistry`. Signature must be extended.
2. `evaluate_for_participant` at `crates/worldwake-systems/src/trade_actions.rs` calls `evaluate_trade_bundle`. It has access to `WorldTxn` which can provide the `RecipeRegistry` (need to verify how the registry is accessible during tick execution).
3. `buyer_reservation_price` and `seller_reservation_price` in `trade_actions.rs` were added by S10. They compute reservation from needs, wounds, stock, demand — but not recipes. Adding recipe awareness means these functions also need `RecipeRegistry` access.
4. `trade_bundle_is_mutually_accepted` was removed in S10. The only caller of `evaluate_trade_bundle` from `trade_actions.rs` is `evaluate_for_participant`.
5. Seller-side consequence (Deliverable 8): a seller who needs firewood for a bread recipe will have higher reservation price for firewood — this emerges naturally when `seller_reservation_price` considers indirect recipe value via the shared layer.
6. Authoritative-to-AI Impact Rule: this changes `evaluate_trade_bundle` (valuation function used during negotiation). Impact: sellers may now reject trades they previously accepted (retaining recipe inputs). Buyers may now value recipe inputs higher (willing to pay more). Both affect negotiation convergence. Golden tests must verify.

## Architecture Check

1. Extending `evaluate_trade_bundle` to accept `&RecipeRegistry` is the clean approach — the registry is a static read-only structure, not mutable state. No AI crate dependency is introduced.
2. The shared `commodity_opportunity_score` replaces the inline `snapshot()` valuation logic in `trade_valuation.rs`, eliminating the AI-trade split.
3. No backward-compatibility shims. The old signature is replaced, not wrapped. All callers are updated.

## Verification Layers

1. Seller refuses to sell last firewood when it closes a valuable bread recipe -> focused integration test
2. Buyer willing to pay more for recipe input than for non-recipe commodity -> focused integration test
3. Reservation prices account for indirect recipe value -> focused unit test
4. Conservation invariants still hold after recipe-aware trades -> authoritative world state verification
5. Existing golden tests pass -> regression (cargo test -p worldwake-ai)

## What to Change

### 1. Extend `evaluate_trade_bundle` signature

Add `recipes: &RecipeRegistry` parameter:

```rust
pub fn evaluate_trade_bundle(
    actor: EntityId,
    belief: &dyn RuntimeBeliefView,
    needs: Option<&HomeostaticNeeds>,
    wounds: Option<&WoundList>,
    current_coin: Quantity,
    offered: &[(CommodityKind, Quantity)],
    received: &[(CommodityKind, Quantity)],
    local_alternatives: &[(EntityId, CommodityKind, Quantity)],
    demand_memory: Option<&DemandMemory>,
    recipes: &RecipeRegistry,  // NEW
) -> TradeAcceptance
```

### 2. Replace internal snapshot valuation with shared layer

Inside `evaluate_trade_bundle`, replace the inline `snapshot()` → `survival_score` / `wound_score` / `demand_score` computation with calls to `commodity_opportunity_score` for each commodity in the holdings/received/offered sets. The `indirect_recipe_score` channel now contributes to the bundle comparison.

### 3. Update `evaluate_for_participant` in `trade_actions.rs`

Pass `RecipeRegistry` through to `evaluate_trade_bundle`. The registry should be accessible via the simulation state during tick execution (verify the exact access pattern — likely through `WorldTxn` or a context parameter).

### 4. Update reservation price functions

Extend `buyer_reservation_price` and `seller_reservation_price` to accept `&RecipeRegistry` and `&dyn GoalBeliefView` (or equivalent). Use `commodity_opportunity_score` to include indirect recipe value in the reservation computation.

### 5. Update all callers

Grep for all call sites of `evaluate_trade_bundle` and update them to pass the recipe registry. This includes:
- `evaluate_for_participant` in `trade_actions.rs`
- `trade_bundle_is_mutually_accepted` equivalent (if any remain post-S10)
- Any test helpers that call `evaluate_trade_bundle`

## Files to Touch

- `crates/worldwake-sim/src/trade_valuation.rs` (modify — extend signature, delegate to shared layer)
- `crates/worldwake-systems/src/trade_actions.rs` (modify — pass RecipeRegistry to evaluate_for_participant and reservation functions)
- Any other callers of `evaluate_trade_bundle` found via grep (modify)

## Out of Scope

- AI ranking replacement (ticket 006)
- Golden integration tests (ticket 007)
- Changes to `commodity_opportunity.rs` itself (tickets 003, 004)
- Multi-commodity bundle negotiation

## Acceptance Criteria

### Tests That Must Pass

1. Seller with firewood + grain + reachable mill rejects selling firewood at low price (recipe retention)
2. Buyer values firewood higher when it closes a bread recipe (willingness to pay rises)
3. Agent without `CommodityValuationProfile` still evaluates using direct channels only (graceful degradation)
4. Conservation invariants hold: `verify_authoritative_conservation`, `verify_live_lot_conservation`
5. All existing golden tests pass: `cargo test -p worldwake-ai`
6. Full suite: `cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings`

### Invariants

1. No AI crate dependency in `worldwake-sim` or `worldwake-systems`.
2. Trade acceptance remains bilateral and bundle-based — no global market computation.
3. Seller and buyer both use the same shared commodity-opportunity layer (Design Goal 2).
4. All valuation is deterministic — no floats, no hash-order.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-sim/src/trade_valuation.rs` (modify existing tests) — update to pass `RecipeRegistry`
2. `crates/worldwake-systems/src/trade_actions.rs` (new/modify) — integration tests for recipe-aware reservation prices and negotiation

### Commands

1. `cargo test -p worldwake-sim -- trade_valuation` — targeted valuation tests
2. `cargo test -p worldwake-systems -- trade` — trade action tests
3. `cargo test -p worldwake-ai` — golden tests (regression)
4. `cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings` — full suite
