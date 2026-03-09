# E04ITECON-001: CommodityKind enum and TradeCategory enum

**Status**: PENDING
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: None
**Deps**: E03 (entity store — completed)

## Problem

The item system needs a stackable commodity taxonomy before lots can be defined. Without `CommodityKind`, there is no way to distinguish apples from grain or enforce type-safe lot operations. `TradeCategory` provides a lightweight bridge between bulk and unique items for later trade/pricing logic.

## Assumption Reassessment (2026-03-09)

1. `worldwake-core/src/lib.rs` exists and already re-exports types — confirmed
2. `Quantity` newtype already exists in `numerics.rs` — confirmed
3. No existing `CommodityKind` or `TradeCategory` types — confirmed via grep
4. The crate uses `BTreeSet`/`BTreeMap` (never `HashMap`/`HashSet`) for authoritative state — confirmed

## Architecture Check

1. Pure data enums with derived traits; no logic, no dependencies beyond `serde`
2. Placed in a new `items.rs` module to keep `components.rs` focused on ECS-attached data

## What to Change

### 1. Create `crates/worldwake-core/src/items.rs`

Define:

```rust
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Serialize, Deserialize)]
pub enum CommodityKind {
    Apple,
    Grain,
    Bread,
    Water,
    Firewood,
    Medicine,
    Coin,
    Waste,
}
```

And:

```rust
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Serialize, Deserialize)]
pub enum TradeCategory {
    Food,
    Water,
    Fuel,
    Medicine,
    Coin,
    SimpleTool,
    Weapon,
    Waste,
}
```

Provide `CommodityKind::trade_category(&self) -> TradeCategory` mapping so commodity kinds can be categorized for pricing.

### 2. Register module in `lib.rs`

Add `pub mod items;` and re-export `CommodityKind` and `TradeCategory`.

## Files to Touch

- `crates/worldwake-core/src/items.rs` (new)
- `crates/worldwake-core/src/lib.rs` (modify — add module + re-exports)

## Out of Scope

- `ItemLot` component struct (E04ITECON-003)
- `UniqueItem` / `UniqueItemKind` (E04ITECON-004)
- Lot algebra (split/merge) (E04ITECON-006)
- Container capacity or load accounting
- Trade pricing logic (future epic)
- Any ECS component registration

## Acceptance Criteria

### Tests That Must Pass

1. All 8 `CommodityKind` variants round-trip through bincode
2. All 8 `TradeCategory` variants round-trip through bincode
3. `CommodityKind` ordering is deterministic (sorted array equals itself after sort)
4. `TradeCategory` ordering is deterministic
5. `CommodityKind::trade_category()` maps each variant to the expected category
6. Both types satisfy `Copy + Clone + Eq + Ord + Hash + Debug + Serialize + DeserializeOwned`
7. Existing suite: `cargo test -p worldwake-core`

### Invariants

1. No `HashMap` or `HashSet` usage in authoritative state
2. All enum variants match the spec's Section 4.3 minimum goods catalog exactly
3. `Waste` is a first-class commodity, not excluded from conservation (spec 7.1)

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/items.rs` (inline `#[cfg(test)]` module) — trait bounds, bincode round-trips, ordering, trade_category mapping

### Commands

1. `cargo test -p worldwake-core items`
2. `cargo clippy --workspace && cargo test --workspace`
