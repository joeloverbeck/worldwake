# E11TRAECO-007: Implement `evaluate_trade_bundle` Valuation Helper

**Status**: COMPLETED

## Summary
Implement a narrow trade-valuation helper in `worldwake-sim` that evaluates a candidate bundle from one actor's perspective using concrete held goods, concrete physiological pressure, wounds, remembered unmet demand, coin on hand, and visible local alternatives. It belongs in `worldwake-sim` because the evaluation depends on `&dyn BeliefView`.

This ticket is not a negotiation system and not a pricing engine. It should add a bounded helper that later trade action logic can call at commit time.

## Dependencies
- Archived `E11TRAECO-002` (`MerchandiseProfile`)
- Archived `E11TRAECO-003` (`DemandMemory`)
- Archived `E11TRAECO-004` (`TradeDispositionProfile`)
- Archived `E11TRAECO-006` (`TradeActionPayload`)

## Assumption Reassessment (2026-03-11)

1. The original ticket assumed several trade prerequisites were still missing. That is stale:
   - `crates/worldwake-core/src/trade.rs` already exists and already defines `MerchandiseProfile`, `DemandMemory`, `TradeDispositionProfile`, and `SubstitutePreferences`.
   - `LotOperation::Traded` already exists in `worldwake-core`.
   - `TradeActionPayload` and `ActionPayload::Trade` already exist in `worldwake-sim`.
2. There is currently no trade valuation module in `worldwake-sim`. This ticket is still needed, but its blast radius is smaller and cleaner than the original wording implied.
3. `BeliefView` is narrower than the original ticket assumed. Today it exposes:
   - liveness, kind, effective place
   - direct possessions and commodity quantities
   - item-lot commodity / consumable profile
   - local co-location facts and other action-affordance queries
   It does **not** expose market quotes, counterparty intent, merchant profiles, or any richer trade belief model.
4. The original ticket imported `MerchandiseProfile` into the proposed API but did not include it in the function signature. That was internally inconsistent. Given the current `BeliefView` surface and this ticket's scope, the clean correction is to remove `MerchandiseProfile` from this helper's promised inputs instead of adding another parameter prematurely.
5. The original acceptance criteria claimed the helper could model both:
   - alternative sellers affecting a buyer's willingness
   - alternative buyers affecting a seller's willingness
   The current `local_alternatives: &[(EntityId, CommodityKind, Quantity)]` shape can support the first cleanly by representing visible alternative accessible supply for a commodity. It does **not** encode buyer/seller role, quoted bundles, or counterparty demand, so the second claim was unsupported and has to be removed from this ticket.
6. The original wording also implied a broad bilateral post-trade utility engine. That would be a larger architectural surface than the current codebase can support cleanly. The robust move here is a trade-specific helper with explicit, inspectable inputs rather than a generic hidden scoring subsystem.

## Architecture Check

1. Keeping trade valuation in `worldwake-sim` is stronger than pushing it into `worldwake-ai`. The helper depends on `BeliefView`, and negotiation/action commit belongs to simulation runtime, not planner-specific logic.
2. The helper should remain trade-specific, not become a generic utility framework. A generic utility engine at this stage would be a premature abstraction with invented semantics not yet backed by the rest of the architecture.
3. The helper should evaluate concrete state only:
   - accessible commodity quantities from belief
   - current physiological pressure from `HomeostaticNeeds`
   - wound burden from `WoundList`
   - remembered unmet demand from `DemandMemory`
   - visible local alternative supply for the same commodity
   - current coin reserve
4. No global price table, no scarcity multiplier table, no floating-point weights, and no hidden market object are acceptable.
5. The clean long-term architecture is still to let negotiation enumerate candidate bundles and call this helper from each participant's perspective. That is better than embedding ad hoc valuation logic inside the eventual action handler.
6. A likely future refinement, once trade action handling lands, is to introduce a richer trade-observation/read-model type for local opportunities instead of the current raw `(EntityId, CommodityKind, Quantity)` tuples. This ticket should not guess that shape early.

## Scope Correction

This ticket should:

1. Add `crates/worldwake-sim/src/trade_valuation.rs`.
2. Define `TradeAcceptance`, `TradeRejectionReason`, and `evaluate_trade_bundle`.
3. Re-export the helper and types from `crates/worldwake-sim/src/lib.rs`.
4. Add focused unit tests around concrete valuation behavior.

This ticket should not:

1. Change `BeliefView`.
2. Change `TradeActionPayload` or action registration.
3. Implement the negotiation action handler or bundle search.
4. Implement merchant restock logic or substitute demand.
5. Introduce a global market state, price index, or compatibility wrapper.

## Files to Touch
- `crates/worldwake-sim/src/trade_valuation.rs` — new module
- `crates/worldwake-sim/src/lib.rs` — module export / re-exports

## Out of Scope
- Trade action handler (`E11TRAECO-008`)
- Trade system tick (`E11TRAECO-009`)
- Substitute demand (`E11TRAECO-010`)
- Merchant restock logic (`E11TRAECO-011`)
- Component registration in `worldwake-core`
- `BeliefView` trait changes

## Implementation Details

```rust
use worldwake_core::{DemandMemory, EntityId, HomeostaticNeeds, Quantity, WoundList};
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
    offered: &[(worldwake_core::CommodityKind, Quantity)],
    received: &[(worldwake_core::CommodityKind, Quantity)],
    local_alternatives: &[(EntityId, worldwake_core::CommodityKind, Quantity)],
    demand_memory: Option<&DemandMemory>,
) -> TradeAcceptance
```

Required behavior for this ticket:

1. Compare current accessible state against post-trade accessible state from one actor's perspective.
2. Treat `local_alternatives` as visible alternative accessible supply for a commodity at the current place. This can reduce the marginal benefit of receiving more of that commodity because the actor can already pursue those alternatives.
3. Use `HomeostaticNeeds` when present to make survival goods concretely more valuable under higher pressure.
4. Use `WoundList` when present so medicine can matter under concrete injury pressure.
5. Use `DemandMemory` when present so remembered unmet demand can make otherwise non-consumable stock worth acquiring.
6. Agents without `HomeostaticNeeds` or `WoundList` must still evaluate deterministically without panicking.
7. The acceptance rule for this ticket is strict improvement: accept only when the post-trade valuation is better than the current valuation from this actor's perspective.

## Acceptance Criteria

### Tests That Must Pass
- `cargo test -p worldwake-sim` — all existing tests pass
- New test: actor accepts trade when the post-trade state is concretely better
- New test: actor rejects trade when the post-trade state is worse
- New test: actor without `HomeostaticNeeds` can still evaluate without panic
- New test: high physiological need makes an otherwise bad survival-good trade acceptable where a no-need actor rejects it
- New test: local alternative accessible supply can make a bad offer unacceptable
- New test: `DemandMemory` can make a non-consumable good worth acquiring
- New test: `TradeAcceptance` and `TradeRejectionReason` satisfy `Clone + Debug + Eq`

### Invariants That Must Remain True
- No global price table, no scarcity multiplier table, no hidden market state
- Valuation is one-actor, belief-facing logic in `worldwake-sim`
- No `f32`/`f64`
- No compatibility aliasing or duplicate valuation path
- `cargo clippy --workspace --all-targets -- -D warnings` clean

## Test Plan

### New/Modified Tests
1. `crates/worldwake-sim/src/trade_valuation.rs` — trait bounds for public valuation enums
2. `crates/worldwake-sim/src/trade_valuation.rs` — accept when survival pressure is improved by the bundle
3. `crates/worldwake-sim/src/trade_valuation.rs` — reject when the bundle worsens a more urgent concrete state
4. `crates/worldwake-sim/src/trade_valuation.rs` — no-needs agents evaluate without panic
5. `crates/worldwake-sim/src/trade_valuation.rs` — local alternatives reduce marginal value of an offered commodity
6. `crates/worldwake-sim/src/trade_valuation.rs` — remembered unmet demand raises value for otherwise non-consumable stock

### Commands
1. `cargo test -p worldwake-sim trade_valuation`
2. `cargo test -p worldwake-sim`
3. `cargo clippy --workspace --all-targets -- -D warnings`
4. `cargo test --workspace`

## Outcome

- Completion date: 2026-03-11
- What actually changed:
  - Added [`crates/worldwake-sim/src/trade_valuation.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/trade_valuation.rs) with `evaluate_trade_bundle`, `TradeAcceptance`, and `TradeRejectionReason`.
  - Re-exported the valuation helper and public enums from [`crates/worldwake-sim/src/lib.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/lib.rs).
  - Implemented valuation as a concrete accessible-state comparison over:
    - survival-goods relief from held plus visible alternative local supply
    - wound pressure versus accessible medicine
    - remembered unmet demand coverage
    - coin reserve
  - Added focused tests for acceptance, rejection, no-needs behavior, alternative local supply, wound-driven medicine value, demand-memory value, trait bounds, and impossible bundle rejection.
- Deviations from original plan:
  - The ticket was corrected before implementation because the original assumptions about missing trade schema, missing trade payload work, and richer `BeliefView` support were stale.
  - `MerchandiseProfile` was intentionally removed from the helper contract for now. The original ticket mentioned it, but the proposed API never accepted it, and forcing it into this helper before the rest of the trade stack needs it would have been premature surface area.
  - The original “alternative buyers affect seller acceptance” claim was removed because the current `local_alternatives` tuple shape only cleanly models visible alternative supply, not buyer-side demand or quoted offers.
  - The helper uses a bounded trade-specific valuation snapshot instead of introducing a generic utility engine. That is the cleaner long-term architecture at the current phase boundary.
- Verification results:
  - `cargo test -p worldwake-sim trade_valuation` passed.
  - `cargo test -p worldwake-sim` passed.
  - `cargo clippy --workspace --all-targets -- -D warnings` passed.
  - `cargo test --workspace` passed.
