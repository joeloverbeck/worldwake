# S06COMOPPVAL-005: Integrate shared layer into evaluate_trade_bundle and reservation prices

**Status**: ✅ COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — `worldwake-sim` (belief_view.rs + per_agent_belief_view.rs recipe-definition surface, action execution context/runtime recipe access, trade_valuation.rs recipe-aware shared valuation), `worldwake-systems` (trade_actions.rs — recipe-aware reservation and negotiation valuation)
**Deps**: S06COMOPPVAL-003, S06COMOPPVAL-004

## Problem

`evaluate_trade_bundle` currently computes commodity value using only direct channels (survival, treatment, demand, coin). It does not consider indirect recipe value — a merchant will sell their last firewood for 1 coin even though firewood + grain → bread worth 3 coins of hunger relief. Post-S10, this function is called during negotiation rounds via `evaluate_for_participant`. The reservation price functions (`buyer_reservation_price`, `seller_reservation_price`) also lack recipe awareness. This ticket wires the shared `commodity_opportunity_score` into both valuation paths.

## Assumption Reassessment (2026-04-02)

1. `evaluate_trade_bundle` at `crates/worldwake-sim/src/trade_valuation.rs` takes `(actor, belief, needs, wounds, current_coin, offered, received, local_alternatives, demand_memory)`. It does NOT take `RecipeRegistry`, but it already receives `&dyn RuntimeBeliefView`, which is the correct shared read boundary for S06 if the belief surface can materialize recipe definitions by `RecipeId`.
2. `GoalBeliefView`/`RuntimeBeliefView` currently expose `knows_recipe` and `known_recipes`, but not recipe definitions. `commodity_opportunity_score` currently avoids that gap only because callers still pass `&RecipeRegistry` directly.
3. `PerAgentBeliefView` currently wraps `&World` plus beliefs/runtime state, but not `&RecipeRegistry`. Trade affordance generation and AI affordance/planning paths use that view directly, so recipe-aware valuation cannot become live until the sim runtime lawfully carries recipe definitions into `PerAgentBeliefView`.
4. Active trade negotiation (`tick_trade`) runs inside action-handler callbacks that currently receive only `WorldTxn`. `WorldTxn` cannot expose `RecipeRegistry` from `worldwake-core`, so the live missing substrate is also on the sim-side action execution context.
5. `buyer_reservation_price` and `seller_reservation_price` in `trade_actions.rs` were added by S10. They compute reservation from needs, wounds, stock, demand — but not recipes. Adding recipe awareness means these functions should consume the same belief-facing commodity-opportunity layer as `evaluate_trade_bundle`, not a second bespoke recipe path.
4. `trade_bundle_is_mutually_accepted` was removed in S10. The only caller of `evaluate_trade_bundle` from `trade_actions.rs` is `evaluate_for_participant`.
5. Seller-side consequence (Deliverable 8): a seller who needs firewood for a bread recipe will have higher reservation price for firewood — this emerges naturally when `seller_reservation_price` considers indirect recipe value via the shared layer.
6. Authoritative-to-AI Impact Rule: this changes `evaluate_trade_bundle` (valuation function used during negotiation). Impact: sellers may now reject trades they previously accepted (retaining recipe inputs). Buyers may now value recipe inputs higher (willing to pay more). Both affect negotiation convergence. Golden tests must verify.

## Architecture Check

1. The clean approach is to extend the belief-facing read model with recipe-definition lookup by `RecipeId`, while also widening the sim runtime context so `PerAgentBeliefView` and action execution can lawfully materialize that data from the authoritative `RecipeRegistry`.
2. This preserves Principle 14 and the S06 design goal: trade valuation stays on the same belief-facing contract as AI instead of introducing a special authoritative recipe side channel in `trade_actions`.
3. The required runtime widening belongs in `worldwake-sim`, where `TickStepServices` and other sim contexts already carry `&RecipeRegistry`. That is cleaner than pushing recipe definitions into persistent world state.
4. The shared `commodity_opportunity_score` replaces the inline `snapshot()` valuation logic in `trade_valuation.rs`, eliminating the AI-trade split.
5. No backward-compatibility shims. The old direct-only valuation path is replaced, not wrapped. All callers are updated.

## Verification Layers

1. Seller refuses to sell last firewood when it closes a valuable bread recipe -> focused integration test
2. Buyer willing to pay more for recipe input than for non-recipe commodity -> focused integration test
3. Reservation prices account for indirect recipe value -> focused unit test
4. Conservation invariants still hold after recipe-aware trades -> authoritative world state verification
5. Existing golden tests pass -> regression (cargo test -p worldwake-ai)

## What to Change

### 1. Extend the belief-facing recipe read surface

Add a recipe-definition read keyed by `RecipeId` on the shared belief boundary used by valuation:

```rust
fn recipe_definition(&self, recipe: RecipeId) -> Option<RecipeDefinition>;
```

Implement it on:
- `GoalBeliefView`
- `RuntimeBeliefView`
- `PerAgentBeliefView`
- relevant runtime/fixture belief-view test stubs

This allows commodity-opportunity and trade valuation to remain belief-facing without duplicating recipe logic in handlers.

### 2. Widen the sim runtime context that builds belief views and runs actions

Thread `&RecipeRegistry` through the existing sim-side runtime surfaces that already sit above `worldwake-core`:

- `PerAgentBeliefView` constructors used by live affordance/planning paths
- action execution context used by handler start/tick/commit callbacks

This keeps recipe access in `worldwake-sim`, where `RecipeRegistry` already belongs, instead of trying to push it through `WorldTxn`.

### 3. Replace internal snapshot valuation with shared layer

Inside `evaluate_trade_bundle`, replace the inline `snapshot()` → `survival_score` / `wound_score` / `demand_score` computation with calls to `commodity_opportunity_score` for each commodity in the holdings/received/offered sets. The `indirect_recipe_score` channel now contributes through the belief-facing recipe-definition surface rather than a direct registry parameter.

### 4. Update `evaluate_for_participant` in `trade_actions.rs`

Keep `evaluate_for_participant` belief-local: it already builds `PerAgentBeliefView::from_world(actor, txn)`. Once that view exposes recipe definitions, no handler-signature widening is needed.

### 5. Update reservation price functions

Extend `buyer_reservation_price` and `seller_reservation_price` to accept the live belief-facing context they need to call `commodity_opportunity_score` and include indirect recipe value without introducing a second recipe-valuation path.

### 6. Update all callers

Grep for all call sites of `evaluate_trade_bundle` and reservation helpers and update them to the new belief-surface contract. This includes:
- `evaluate_for_participant` in `trade_actions.rs`
- trade affordance generation in `enumerate_trade_payloads`
- negotiation-round reservation lookup in `reservation_price_for_actor`
- test helpers and stubs that implement `RuntimeBeliefView`

## Files to Touch

- `crates/worldwake-sim/src/belief_view.rs` (modify — recipe-definition read surface and forwarding)
- `crates/worldwake-sim/src/per_agent_belief_view.rs` (modify — implement recipe-definition lookup)
- `crates/worldwake-sim/src/action_execution.rs` (modify — carry recipe registry in action execution context)
- `crates/worldwake-sim/src/action_handler.rs` (modify — pass execution context through start/tick/commit callbacks)
- `crates/worldwake-sim/src/start_gate.rs` and `crates/worldwake-sim/src/tick_action.rs` (modify — forward recipe-aware execution context)
- `crates/worldwake-sim/src/trade_valuation.rs` (modify — extend signature, delegate to shared layer)
- `crates/worldwake-sim/src/commodity_opportunity.rs` (modify — consume recipe definitions from belief instead of direct registry parameter)
- `crates/worldwake-systems/src/trade_actions.rs` (modify — recipe-aware reservation and negotiation valuation through belief views)
- Any belief-view or trade-valuation test stubs touched by the new trait method (modify)

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

1. `crates/worldwake-sim/src/belief_view.rs` / `per_agent_belief_view.rs` (modify existing tests) — verify recipe-definition forwarding and lookup
2. `crates/worldwake-sim/src/trade_valuation.rs` (modify existing tests) — recipe-aware bundle evaluation through the belief surface
3. `crates/worldwake-systems/src/trade_actions.rs` (new/modify) — integration tests for recipe-aware reservation prices and negotiation

### Commands

1. `cargo test -p worldwake-sim trade_valuation -- --nocapture` — targeted valuation tests
2. `cargo test -p worldwake-sim belief_view -- --nocapture` — belief-surface forwarding tests
3. `cargo test -p worldwake-systems -- trade` — trade action tests
4. `cargo test -p worldwake-ai` — golden tests (regression)
5. `cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings` — full suite

## Outcome

- **Completed**: 2026-04-02
- **What changed**:
  - extended the shared belief-facing recipe surface in `crates/worldwake-sim/src/belief_view.rs` and `crates/worldwake-sim/src/per_agent_belief_view.rs` so runtime-backed belief views can materialize recipe definitions by `RecipeId`
  - widened the sim execution substrate in `crates/worldwake-sim/src/action_execution.rs`, `crates/worldwake-sim/src/action_handler.rs`, `crates/worldwake-sim/src/start_gate.rs`, `crates/worldwake-sim/src/tick_action.rs`, and `crates/worldwake-sim/src/tick_step.rs` so action handlers and live runtime belief views lawfully carry `RecipeRegistry`
  - replaced the old direct-only trade bundle valuation in `crates/worldwake-sim/src/trade_valuation.rs` with the shared `commodity_opportunity_score` path, including indirect recipe value
  - integrated recipe-aware reservation and negotiation valuation into `crates/worldwake-systems/src/trade_actions.rs`, keeping buyer and seller reasoning on the same shared commodity-opportunity layer
  - updated the AI runtime belief path in `crates/worldwake-ai/src/agent_tick/mod.rs` and related agent-tick modules so planner/runtime evaluation sees the same recipe-aware belief surface
  - updated affected sim, systems, and AI test harnesses to the widened execution-context signature
- **Deviations from original plan**:
  - `What to Change` item 4 turned out to be stale: `evaluate_for_participant` could not stay on the old substrate by itself because active negotiation also needed recipe access at handler tick/commit time, so the correct fix widened the sim action execution context rather than trying to push `RecipeRegistry` through `WorldTxn`
  - the focused negotiation monotonicity test in `crates/worldwake-systems/src/trade_actions.rs` needed recalibrated input profiles so it still proved multi-round concession under the stronger recipe-aware valuation contract
- **Verification results**:
  - `cargo test -p worldwake-sim trade_valuation -- --nocapture`
  - `cargo test -p worldwake-sim`
  - `cargo test -p worldwake-systems -- trade`
  - `cargo test -p worldwake-ai`
  - `cargo test --workspace`
  - `cargo clippy --workspace --all-targets -- -D warnings`
