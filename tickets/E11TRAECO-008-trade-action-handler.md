# E11TRAECO-008: Implement Trade Action Handler

## Summary
Implement the trade action handler that executes negotiation/exchange actions. When a trade action completes, the handler evaluates the candidate bundle, transfers ownership of goods and payment if mutually accepted, appends `LotOperation::Traded` provenance, and emits trade events.

## Dependencies
- E11TRAECO-001 (LotOperation::Traded)
- E11TRAECO-006 (TradeActionPayload)
- E11TRAECO-007 (evaluate_trade_bundle)
- E11TRAECO-002, 003, 004 (trade components in core)

## Files to Touch
- `crates/worldwake-systems/src/trade.rs` — **new file**: trade action handler
- `crates/worldwake-systems/src/trade_actions.rs` — **new file**: trade action definition registration
- `crates/worldwake-systems/src/lib.rs` — add `pub mod trade;` and `pub mod trade_actions;`

## Out of Scope
- Trade system tick (DemandMemory aging, unmet demand recording) — E11TRAECO-009
- Substitute demand logic — E11TRAECO-010
- Merchant restock planning — E11TRAECO-011
- BeliefView trait extensions (if needed, make a sub-task)
- Combat system, needs system, production system changes
- GOAP/AI integration (E13)

## Implementation Details

### Trade Action Definition
Register a `Trade` action def with:
- `DurationExpr::Fixed(negotiation_round_ticks)` from initiator's `TradeDispositionProfile`
- `interruptibility: FreelyInterruptible`
- `visibility: VisibilitySpec::SamePlace`
- Preconditions:
  - `Precondition::ActorAlive`
  - `Precondition::TargetAtActorPlace(0)` — co-location with counterparty
- Commit conditions: both parties co-located, seller possesses goods, buyer possesses payment

### Handler Logic (at action commit)
1. Extract `TradeActionPayload` from the completing action
2. Verify co-location of buyer and seller
3. Verify seller still possesses offered goods
4. Verify buyer still possesses offered payment
5. Call `evaluate_trade_bundle` for both sides
6. If both accept:
   a. For partial lots, call `World::split_lot` first
   b. Transfer `possessed_by` and `owned_by` relations for goods (seller -> buyer)
   c. Transfer `possessed_by` and `owned_by` relations for payment (buyer -> seller)
   d. Append `ProvenanceEntry` with `LotOperation::Traded` to each transferred lot
7. Emit trade event with: participants, commodity, quantity, offered amounts, success/failure, failure reason

### Conservation
Trade operates through ownership relation changes only. No lots are created or destroyed. `split_lot` maintains the conservation invariant for partial trades.

## Acceptance Criteria

### Tests That Must Pass
- `cargo test -p worldwake-systems` — all existing tests pass
- New test: successful trade transfers goods and coin with conservation verified
- New test: trade requires co-location (fails if parties not at same place)
- New test: cannot sell goods not in possession
- New test: cannot pay with coin not in possession
- New test: trade appends `LotOperation::Traded` provenance to transferred lots
- New test: partial lot trade uses `split_lot` and conserves total quantity
- New test: trade emits event with correct participants, commodity, quantity, success
- New test: failed negotiation emits event with failure reason
- New test: negotiation fails if either side rejects the bundle
- New test: `verify_conservation` passes after trade

### Invariants That Must Remain True
- Conservation (9.5): ownership transfer, not creation/destruction
- No negative stocks (9.6)
- Ownership transfer requires valid possession chain (9.7)
- Principle 6: negotiation consumes time (action duration from TradeDispositionProfile)
- Principle 7: all information is local (co-located agents only)
- Append-only event log — trade events committed, never mutated
- Determinism — no HashMap, no floats, no wall-clock time
- `cargo clippy --workspace` clean
