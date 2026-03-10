# E11TRAECO-006: Add `Trade(TradeActionPayload)` Variant to `ActionPayload`

## Summary
Define `TradeActionPayload` and add a `Trade(TradeActionPayload)` variant to the `ActionPayload` enum in `worldwake-sim`. This is the data payload carried by trade negotiation actions.

## Dependencies
- None (ActionPayload already exists with Harvest/Craft variants)

## Files to Touch
- `crates/worldwake-sim/src/action_payload.rs` — add `TradeActionPayload` struct and `Trade` variant to `ActionPayload`

## Out of Scope
- Trade action definition registration (E11TRAECO-007)
- Trade action handler (E11TRAECO-008)
- Valuation logic (E11TRAECO-007)
- Component changes in worldwake-core
- BeliefView changes

## Implementation Details

```rust
use worldwake_core::{CommodityKind, EntityId, Quantity};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct TradeActionPayload {
    pub counterparty: EntityId,
    pub offered_commodity: CommodityKind,
    pub offered_quantity: Quantity,
    pub requested_commodity: CommodityKind,
    pub requested_quantity: Quantity,
}
```

Add `Trade(TradeActionPayload)` to `ActionPayload` enum after `Craft`.

Update existing tests:
- `action_payload_satisfies_required_traits` must also cover `TradeActionPayload`
- Add bincode roundtrip test for `ActionPayload::Trade`

## Acceptance Criteria

### Tests That Must Pass
- `cargo test -p worldwake-sim` — all existing tests pass
- New test: `TradeActionPayload` satisfies `Clone + Eq + Debug + Serialize + DeserializeOwned`
- New test: `ActionPayload::Trade(...)` bincode roundtrip
- Existing `ActionPayload::None`, `Harvest`, `Craft` tests still pass

### Invariants That Must Remain True
- `ActionPayload` default remains `None`
- All existing variants unchanged
- `cargo clippy --workspace` clean
