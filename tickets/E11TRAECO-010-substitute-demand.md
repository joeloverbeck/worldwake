# E11TRAECO-010: Implement Substitute Demand Logic

## Summary
When a preferred good is unavailable or a negotiation fails, a buyer with `SubstitutePreferences` inspects other available goods at the current place and may buy a substitute if the exchange is beneficial. An agent without `SubstitutePreferences` does not consider substitutes.

## Dependencies
- E11TRAECO-005 (SubstitutePreferences component)
- E11TRAECO-007 (evaluate_trade_bundle)
- E11TRAECO-008 (trade action handler — substitute demand extends it)

## Files to Touch
- `crates/worldwake-systems/src/trade.rs` — add substitute demand evaluation within trade handler flow
- `crates/worldwake-sim/src/trade_valuation.rs` — may need a helper to rank substitutes against valuation

## Out of Scope
- `SubstitutePreferences` struct definition (E11TRAECO-005)
- Trade action handler core logic (E11TRAECO-008)
- Merchant restock (E11TRAECO-011)
- DemandMemory aging (E11TRAECO-009)
- Component registration
- GOAP integration (E13)

## Implementation Details

When a buyer's preferred trade fails (counterparty lacks stock, or negotiation rejected):
1. Check if buyer has `SubstitutePreferences` component
2. If not, stop — no substitute demand
3. Look up the `TradeCategory` of the failed commodity
4. Walk the buyer's `SubstitutePreferences.preferences[category]` list in order (index 0 = most preferred)
5. For each substitute commodity, check if any co-located seller has it available
6. For each available substitute, call `evaluate_trade_bundle` to check if the exchange is beneficial
7. If beneficial, the buyer may initiate a trade for the substitute
8. Record `DemandObservation` with appropriate reason for the original failed commodity

The substitute search order must be deterministic (Vec iteration + BTreeMap keying ensures this).

## Acceptance Criteria

### Tests That Must Pass
- `cargo test -p worldwake-systems` — all existing tests pass
- New test: substitute demand activates when preferred good is unavailable
- New test: substitute demand activates when negotiation for preferred good is rejected
- New test: substitutes are tried in preference order (index 0 first)
- New test: agent without `SubstitutePreferences` does not consider substitutes
- New test: substitute demand only checks co-located sellers (locality)
- New test: substitute must pass `evaluate_trade_bundle` acceptance — bad substitutes rejected

### Invariants That Must Remain True
- Principle 7: all information is local (co-located agents only)
- Substitute search is deterministic (BTreeMap + Vec ordering)
- No global market state consulted
- Conservation maintained through substitute trades
- `cargo clippy --workspace` clean
