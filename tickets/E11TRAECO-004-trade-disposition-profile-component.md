# E11TRAECO-004: Add `TradeDispositionProfile` Component

## Summary
Define `TradeDispositionProfile` in `crates/worldwake-core/src/trade.rs` and register it as a component on `EntityKind::Agent`. This component controls per-agent negotiation style and demand memory retention.

## Dependencies
- E11TRAECO-002 (trade.rs module must exist)

## Files to Touch
- `crates/worldwake-core/src/trade.rs` — add `TradeDispositionProfile` struct
- `crates/worldwake-core/src/lib.rs` — re-export `TradeDispositionProfile`
- `crates/worldwake-core/src/component_schema.rs` — register on `EntityKind::Agent`
- `crates/worldwake-core/src/component_tables.rs` — add import and storage

## Out of Scope
- `MerchandiseProfile` (E11TRAECO-002)
- `DemandMemory` (E11TRAECO-003)
- `SubstitutePreferences` (E11TRAECO-005)
- Negotiation logic or valuation
- DemandMemory aging logic (E11TRAECO-009)

## Implementation Details

```rust
use std::num::NonZeroU32;
use crate::Permille;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct TradeDispositionProfile {
    pub negotiation_round_ticks: NonZeroU32,
    pub initial_offer_bias: Permille,
    pub concession_rate: Permille,
    pub demand_memory_retention_ticks: u32,
}

impl Component for TradeDispositionProfile {}
```

Fields:
- `negotiation_round_ticks`: Duration of a negotiation action in ticks.
- `initial_offer_bias`: 0 = generous, 1000 = maximally aggressive.
- `concession_rate`: Permille conceded per round.
- `demand_memory_retention_ticks`: How long DemandObservations are retained (dampener for demand-memory feedback loop).

## Acceptance Criteria

### Tests That Must Pass
- `cargo test -p worldwake-core` — all existing tests pass
- New test: `TradeDispositionProfile` satisfies `Clone + Eq + Debug + Serialize + DeserializeOwned`
- New test: bincode roundtrip
- New test: component table insert/get/remove/has cycle

### Invariants That Must Remain True
- Component only registerable on `EntityKind::Agent`
- Uses `Permille` (not `f32`/`f64`) for ratio fields
- Uses `NonZeroU32` for `negotiation_round_ticks` (cannot be zero)
- Existing component registrations unchanged
- `cargo clippy --workspace` clean
