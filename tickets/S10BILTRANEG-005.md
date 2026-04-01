# S10BILTRANEG-005: Negotiation protocol in trade action lifecycle

**Status**: PENDING
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: Yes — `worldwake-systems` (trade action handlers: start, tick, commit, abort)
**Deps**: S10BILTRANEG-001, S10BILTRANEG-002, S10BILTRANEG-003

## Problem

The trade action lifecycle currently skips negotiation entirely: `tick_trade` is a no-op returning `ActionProgress::Continue`, and `commit_trade` evaluates a fixed-price bundle at completion. This means every trade is evaluated at whatever price was set in the payload (currently always 1:1). With variable-price payloads from ticket 004, the lifecycle must execute multi-round negotiation where both agents derive offers from concrete state, concede over time, and agree or walk away.

## Assumption Reassessment (2026-04-02)

1. `tick_trade` at `trade_actions.rs:164-171` is a no-op: `fn tick_trade(...) -> Result<ActionProgress, ActionError> { Ok(ActionProgress::Continue) }`.
2. `start_trade` at `trade_actions.rs:152-161` validates the bundle context and returns `Ok(Some(ActionState::Empty))`. It must be changed to return `ActionState::Trade { .. }` with initial negotiation state.
3. `commit_trade` at `trade_actions.rs:173-183` calls `validate_trade_bundle_context` which includes `ensure_bundle_accepted` — the mutual acceptance check at commit time. With negotiation, `ensure_bundle_accepted` is no longer needed at commit; acceptance was determined during rounds.
4. `abort_trade` at `trade_actions.rs:186-194` is a no-op. It must record failed negotiation outcomes in `DemandMemory`.
5. `execute_trade_transfers` at `trade_actions.rs:473-510` handles the physical lot-based transfer. It takes the `TradeActionPayload` which contains `offered_quantity`. With negotiation, the agreed price may differ from the payload's `offered_quantity` (the opening offer). The transfer must use `agreed_price` from `ActionState::Trade`, not `payload.offered_quantity`.
6. `ActionState::Trade` from ticket 001 provides `round`, `initiator_role`, `initiator_last_offer`, `responder_last_offer`, `agreed_price`.
7. `WorldTxn` is available during `tick_trade` — the established pattern is that action execution uses authoritative state (not belief views). This is consistent with all other action handlers.
8. `ActionProgress::Continue` vs `ActionProgress::Done` — `tick_trade` returns `Continue` to keep ticking or the action framework handles completion via duration expiry. For negotiation, early termination on agreement or walkaway must use the existing action abort/complete machinery.
9. Seller occupancy during trade: the seller is a target of the active trade action. The action system prevents a second buyer from starting a trade with the same occupied seller — existing contention mechanism, no new logic needed.
10. Authoritative-to-AI Impact Rule: `commit_trade` changes validation (removing `ensure_bundle_accepted`). Downstream: `handle_plan_failure` in `agent_tick.rs` will see different abort reasons (negotiation walkaway vs. bundle rejection). The planner replans on abort, so walkaway is handled gracefully by the existing replan-on-failure path.

## Architecture Check

1. The alternating-offer protocol is a well-known game-theoretic mechanism (Rubinstein 1982) that guarantees convergence under monotonic concession. It is more architecturally sound than the current "evaluate fixed bundle at commit" because it creates duration-bearing negotiation rounds (Principle 8) and produces granular aftermath on failure (Principle 10).
2. Using `ActionState::Trade` for negotiation progress keeps transient state within the action lifecycle — no new components or persistent storage needed. The state exists only for the action's duration.
3. No backward-compatibility shims. `ensure_bundle_accepted` in commit is removed. The old fixed-price evaluation path is replaced, not wrapped.

## Verification Layers

1. Negotiation converges when buyer reservation > seller reservation -> focused integration test (construct agents with overlapping zones, run negotiation rounds, assert agreed_price exists)
2. Negotiation fails when buyer reservation < seller reservation -> focused integration test (assert walkaway)
3. `commit_trade` executes at `agreed_price`, not `payload.offered_quantity` -> action trace verification
4. `abort_trade` records DemandObservation with correct reason -> event-log / authoritative state verification
5. Each tick advances negotiation round by 1 -> action trace / state inspection
6. Monotonic concession holds across rounds -> focused unit test (sequence of tick calls)
7. Seller occupancy prevents concurrent negotiations -> action lifecycle verification (start_trade fails for second buyer)

## What to Change

### 1. Modify `start_trade`

Instead of returning `ActionState::Empty`, initialize `ActionState::Trade`:
```rust
Ok(Some(ActionState::Trade {
    round: 0,
    initiator_role: TradeRole::Buyer,
    initiator_last_offer: Some(payload.offered_quantity),
    responder_last_offer: None,
    agreed_price: None,
}))
```

Keep the existing `validate_trade_bundle_context` call for initial validation (ensure co-location, lot exists, etc.), but skip the `ensure_bundle_accepted` portion during start — acceptance is determined during negotiation.

### 2. Implement active `tick_trade`

Replace the no-op with negotiation round logic:

```
1. Read ActionState::Trade from instance.state.
2. Determine whose turn: even rounds = initiator, odd rounds = responder.
3. The responding agent:
   a. Compute own reservation price from WorldTxn state (using buyer_reservation_price or seller_reservation_price).
   b. Read the current offer (initiator_last_offer or responder_last_offer from the previous round).
   c. If offer meets or exceeds own reservation → set agreed_price, return Continue (commit will finalize).
   d. Compute own effective deadline via urgency_modulated_deadline.
   e. If round >= own deadline → walk away: return ActionError (abort path).
   f. Compute counter-offer via generate_offer using own concession curve.
   g. Enforce monotonic concession: buyer counter-offer ≥ previous offer, seller counter-offer ≤ previous ask.
   h. Store counter-offer in state, increment round.
4. Return ActionProgress::Continue.
```

The responder's `TradeDispositionProfile` is read from `WorldTxn` via `txn.get_component_trade_disposition_profile(counterparty)`.

### 3. Modify `commit_trade`

1. Read `ActionState::Trade` from instance.
2. If `agreed_price` is `Some(price)`:
   a. Create a modified `TradeActionPayload` (or adjust the transfer call) to use `agreed_price` instead of `payload.offered_quantity`.
   b. Call `execute_trade_transfers` with the agreed price.
   c. Record `DemandObservation` with reason `TradeAgreed` and quantity = agreed price for both buyer and seller.
3. If `agreed_price` is `None` (deadline reached without agreement):
   a. Return an error to trigger abort.

Remove the `ensure_bundle_accepted` call from `validate_trade_bundle_context` for the commit path (or make it conditional — only called when no negotiation state exists, for backward safety during transition). Actually, per Principle 28, remove it cleanly.

### 4. Modify `abort_trade`

Record failed negotiation outcomes in both agents' `DemandMemory`:
- Buyer: `DemandObservation` with reason `WantedToBuyButTooExpensive`, counterparty = seller.
- Seller: `DemandObservation` with reason `WantedToSellButNoBuyer`.

Use `txn.get_component_demand_memory` to read current memory, append the new observation, and write back via the appropriate component setter.

### 5. Adjust `execute_trade_transfers` call

The transfer function uses `payload.offered_quantity` for the coin amount. With negotiation, the agreed price differs from the opening offer. Either:
- Pass `agreed_price` as an override parameter, or
- Modify the payload before calling (construct a local copy with updated `offered_quantity = agreed_price`).

The second approach is cleaner — it keeps `execute_trade_transfers` unchanged.

## Files to Touch

- `crates/worldwake-systems/src/trade_actions.rs` (modify — `start_trade`, `tick_trade`, `commit_trade`, `abort_trade`, adjust transfer call)

## Out of Scope

- Affordance generation changes (ticket 004)
- Golden test creation (ticket 006)
- Changes to `evaluate_trade_bundle` or `trade_valuation.rs`
- Multi-commodity bundle negotiation
- Changes to `PlanningBudget` defaults

## Acceptance Criteria

### Tests That Must Pass

1. Negotiation converges: two agents with overlapping reservation prices reach agreement within deadline
2. Negotiation fails: two agents with non-overlapping reservations walk away
3. `agreed_price` is used for transfer, not `payload.offered_quantity`
4. Monotonic concession holds: buyer offers never decrease, seller asks never increase across rounds
5. Each tick advances round counter by 1
6. `abort_trade` records `WantedToBuyButTooExpensive` for buyer and `WantedToSellButNoBuyer` for seller
7. `commit_trade` records `TradeAgreed` with correct price for both agents
8. Conservation invariant holds: total coins + commodities before = total after
9. All existing golden tests pass: `cargo test -p worldwake-ai`
10. Full suite: `cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings`

### Invariants

1. Conservation: coins and commodities are neither created nor destroyed by negotiation (transfer is zero-sum).
2. Determinism: same seed, same inputs → same negotiation outcome (no floats, no wall-clock time, seeded RNG not used in protocol).
3. Monotonic concession: enforced per round, preventing oscillation.
4. Agent symmetry (Principle 19): human-controlled agent with same `TradeDispositionProfile` negotiates identically to AI-controlled.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-systems/src/trade_actions.rs` (new `#[cfg(test)] mod negotiation_tests`) — integration tests for the full negotiation lifecycle (start → tick × N → commit or abort)
2. `crates/worldwake-systems/src/trade_actions.rs` (modify existing tests if any reference the old no-op tick_trade or fixed-price commit)

### Commands

1. `cargo test -p worldwake-systems -- negotiation` — targeted negotiation tests
2. `cargo test -p worldwake-systems -- trade` — all trade tests
3. `cargo test -p worldwake-ai` — golden tests (regression check)
4. `cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings` — full suite
