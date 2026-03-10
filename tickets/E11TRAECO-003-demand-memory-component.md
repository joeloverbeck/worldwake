# E11TRAECO-003: Add `DemandMemory`, `DemandObservation`, and `DemandObservationReason`

## Summary
Define demand memory types in `crates/worldwake-core/src/trade.rs` and register `DemandMemory` as a component on `EntityKind::Agent`. These types record concrete missed demand and sale opportunities.

## Dependencies
- E11TRAECO-002 (trade.rs module must exist)

## Files to Touch
- `crates/worldwake-core/src/trade.rs` — add `DemandMemory`, `DemandObservation`, `DemandObservationReason`
- `crates/worldwake-core/src/lib.rs` — re-export new types
- `crates/worldwake-core/src/component_schema.rs` — register `DemandMemory` on `EntityKind::Agent`
- `crates/worldwake-core/src/component_tables.rs` — add `DemandMemory` import and storage

## Out of Scope
- Aging/pruning logic (E11TRAECO-009)
- Trade system tick (E11TRAECO-009)
- Recording observations from failed trades (E11TRAECO-009)
- `MerchandiseProfile` (E11TRAECO-002)
- `TradeDispositionProfile` (E11TRAECO-004)
- `SubstitutePreferences` (E11TRAECO-005)

## Implementation Details

```rust
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct DemandMemory {
    pub observations: Vec<DemandObservation>,
}

impl Component for DemandMemory {}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct DemandObservation {
    pub commodity: CommodityKind,
    pub quantity: Quantity,
    pub place: EntityId,
    pub tick: Tick,
    pub counterparty: Option<EntityId>,
    pub reason: DemandObservationReason,
}

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Serialize, Deserialize)]
pub enum DemandObservationReason {
    WantedToBuyButNoSeller,
    WantedToBuyButSellerOutOfStock,
    WantedToBuyButTooExpensive,
    WantedToSellButNoBuyer,
}
```

`Vec<DemandObservation>` is correct here because append-order is meaningful (chronological). `DemandObservationReason` uses `Copy` since it's a simple enum.

## Acceptance Criteria

### Tests That Must Pass
- `cargo test -p worldwake-core` — all existing tests pass
- New test: `DemandMemory` satisfies `Clone + Eq + Debug + Serialize + DeserializeOwned`
- New test: `DemandObservation` bincode roundtrip
- New test: `DemandObservationReason` satisfies `Copy + Clone + Eq + Ord + Hash + Debug + Serialize + DeserializeOwned`
- New test: component table insert/get/remove/has cycle for `DemandMemory`

### Invariants That Must Remain True
- Component is only registerable on `EntityKind::Agent`
- Existing component registrations unchanged
- `cargo clippy --workspace` clean
