# E11TRAECO-007: Implement `evaluate_trade_bundle` Valuation Helper

## Summary
Implement the `evaluate_trade_bundle` function in `worldwake-sim`. This is the bilateral trade valuation function that determines whether a candidate exchange bundle is acceptable to a given agent. It lives in `worldwake-sim` (not `worldwake-ai`) because it needs `&dyn BeliefView`.

## Dependencies
- E11TRAECO-003 (DemandMemory types)
- E11TRAECO-002 (MerchandiseProfile)

## Files to Touch
- `crates/worldwake-sim/src/trade_valuation.rs` — **new file**: `evaluate_trade_bundle`, `TradeAcceptance`
- `crates/worldwake-sim/src/lib.rs` — add `pub mod trade_valuation;` and re-exports

## Out of Scope
- Trade action handler (E11TRAECO-008)
- Trade system tick (E11TRAECO-009)
- Substitute demand (E11TRAECO-010)
- Merchant restock logic (E11TRAECO-011)
- Component registration (done in core tickets)
- `BeliefView` trait changes (if any needed, see E11TRAECO-008)
- No global base price, no scarcity multiplier table, no market score

## Implementation Details

```rust
use worldwake_core::{
    CommodityKind, DemandMemory, EntityId, HomeostaticNeeds, MerchandiseProfile,
    Quantity, WoundList,
};
use crate::BeliefView;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TradeAcceptance {
    Accept,
    Reject { reason: TradeRejectionReason },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TradeRejectionReason {
    PostTradeStateWorse,
    InsufficientPayment,
    NoNeed,
}

pub fn evaluate_trade_bundle(
    actor: EntityId,
    belief: &dyn BeliefView,
    needs: Option<&HomeostaticNeeds>,
    wounds: Option<&WoundList>,
    current_coin: Quantity,
    offered: &[(CommodityKind, Quantity)],
    received: &[(CommodityKind, Quantity)],
    local_alternatives: &[(EntityId, CommodityKind, Quantity)],
    demand_memory: Option<&DemandMemory>,
) -> TradeAcceptance
```

Valuation must:
- Compare concrete post-trade state vs current state for the actor
- Use physiological needs (if present) to increase value of survival goods
- Consider local alternatives (more sellers = less willingness to accept bad deal)
- Consider current inventory abundance/shortage
- `Option`-wrapped `needs`/`wounds`: agents without these have no physiological urgency
- **No global base price, no scarcity table, no market score**

## Acceptance Criteria

### Tests That Must Pass
- `cargo test -p worldwake-sim` — all existing tests pass
- New test: agent accepts trade when post-trade state improves
- New test: agent rejects trade when post-trade state is worse
- New test: agent without `HomeostaticNeeds` can still evaluate (no panic, treats as no urgency)
- New test: higher physiological need makes buyer accept worse offers for survival goods
- New test: presence of local alternative sellers changes acceptance threshold
- New test: presence of local alternative buyers changes seller acceptance
- New test: `TradeAcceptance` and `TradeRejectionReason` are `Clone + Debug + Eq`

### Invariants That Must Remain True
- No global price table, no scarcity multiplier, no hidden market state
- Valuation is purely from one agent's perspective using believed state
- No `f32`/`f64` in valuation logic
- `cargo clippy --workspace` clean
