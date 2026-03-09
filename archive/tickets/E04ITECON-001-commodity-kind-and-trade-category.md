# E04ITECON-001: CommodityKind enum and TradeCategory enum

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: None
**Deps**: E03 (entity store — completed)

## Problem

The item system needs a stackable commodity taxonomy before lots can be defined. Without `CommodityKind`, there is no way to distinguish apples from grain or enforce type-safe lot operations. `TradeCategory` provides a lightweight bridge between bulk and unique items for later trade/pricing logic.

## Assumption Reassessment (2026-03-09)

1. `worldwake-core/src/lib.rs` exists and already re-exports types — confirmed
2. `Quantity` newtype already exists in `numerics.rs` — confirmed, but this ticket should not couple taxonomy enums to lot-count wrappers yet
3. No existing `CommodityKind` or `TradeCategory` types — confirmed via grep
4. The crate uses focused source modules with inline unit tests for pure domain types (`entity.rs`, `numerics.rs`) — confirmed
5. The crate enforces deterministic-authoritative-state policy through `tests/policy.rs` and already depends on `serde` + `bincode` — confirmed

## Architecture Check

1. Pure item-domain enums belong in a dedicated `items.rs` module, not in `components.rs`, because they are not ECS-attached data yet
2. This ticket should stay taxonomy-only: no `ItemLot`, no `Quantity` wrapper decisions, no component registration, no world factories
3. Expose canonical variant lists in the module so tests and future item code reuse one source of truth instead of duplicating enum inventories
4. Keep the commodity-to-category bridge local to `CommodityKind`; later unique-item work can add its own bridge without pretending tools or weapons are stackable

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

Also provide:

```rust
impl CommodityKind {
    pub const ALL: [Self; 8] = [/* all variants in declaration order */];
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

Also provide:

```rust
impl TradeCategory {
    pub const ALL: [Self; 8] = [/* all variants in declaration order */];
}
```

Provide `CommodityKind::trade_category(self) -> TradeCategory` as a `const fn` so commodity kinds can be categorized without introducing alias enums or runtime lookup tables.

### 2. Register module in `lib.rs`

Add `pub mod items;` and re-export `CommodityKind` and `TradeCategory`.

## Files to Touch

- `crates/worldwake-core/src/items.rs` (new)
- `crates/worldwake-core/src/lib.rs` (modify — add module + re-exports)

## Out of Scope

- Switching lot quantities from `u32` to `Quantity` in later tickets
- `ItemLot` component struct (E04ITECON-003)
- `UniqueItem` / `UniqueItemKind` (E04ITECON-004)
- Lot algebra (split/merge) (E04ITECON-006)
- Container capacity or load accounting
- Trade pricing logic (future epic)
- Any ECS component registration

## Acceptance Criteria

### Tests That Must Pass

1. `CommodityKind::ALL` contains all 8 variants in declaration order and each variant round-trips through bincode
2. `TradeCategory::ALL` contains all 8 variants in declaration order and each variant round-trips through bincode
3. `CommodityKind` ordering is deterministic (`CommodityKind::ALL` remains unchanged after reverse + sort)
4. `TradeCategory` ordering is deterministic (`TradeCategory::ALL` remains unchanged after reverse + sort)
5. `CommodityKind::trade_category()` maps each variant to the expected category
6. Both types satisfy `Copy + Clone + Eq + Ord + Hash + Debug + Serialize + DeserializeOwned`
7. Existing suite: `cargo test -p worldwake-core`

### Invariants

1. No `HashMap` or `HashSet` usage in authoritative state
2. All enum variants match the spec's Section 4.3 minimum goods catalog exactly
3. `Waste` is a first-class commodity, not excluded from conservation (spec 7.1)

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/items.rs` (inline `#[cfg(test)]` module) — canonical variant lists, trait bounds, bincode round-trips, ordering, trade-category mapping

### Commands

1. `cargo test -p worldwake-core items`
2. `cargo clippy --workspace && cargo test --workspace`

## Outcome

- Completion date: 2026-03-09
- What actually changed:
  - Added `crates/worldwake-core/src/items.rs` with `CommodityKind` and `TradeCategory`
  - Added canonical `ALL` variant lists for both enums
  - Added `CommodityKind::trade_category(self) -> TradeCategory` as a `const fn`
  - Re-exported both enums from `worldwake-core::lib`
  - Added inline unit coverage for trait bounds, canonical variant inventories, bincode round-trips, deterministic ordering, and category mapping
- Deviations from original plan:
  - Strengthened the design to include canonical `ALL` arrays so tests and future item code share a single source of truth
  - Explicitly kept `Quantity` and ECS concerns out of scope to avoid coupling taxonomy work to later lot/component decisions
- Verification results:
  - `cargo test -p worldwake-core items` passed
  - `cargo test -p worldwake-core` passed
  - `cargo clippy --workspace --all-targets -- -D warnings` passed
  - `cargo test --workspace` passed
