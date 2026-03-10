# E09NEEMET-004: CommodityConsumableProfile and BodyCostPerTick metadata types

**Status**: PENDING
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — extend commodity data, new action metadata type
**Deps**: E09NEEMET-003 (needs `Permille` usage patterns established)

## Problem

Consumable effects must come from commodity data, not hardcoded action logic. Additionally, every long-running action that strains the body needs deterministic per-tick body costs. This ticket adds the `CommodityConsumableProfile` to commodity specs and the `BodyCostPerTick` struct for action metadata.

## Assumption Reassessment (2026-03-10)

1. `CommodityKind::spec()` returns `CommodityKindSpec` containing a `CommodityPhysicalProfile` — confirmed in `items.rs`.
2. `CommodityKindSpec` currently has: `trade_category: TradeCategory`, `physical_profile: CommodityPhysicalProfile`.
3. `CommodityPhysicalProfile` currently has: `load_per_unit: LoadUnits`.
4. The `spec()` method is `const fn` — the consumable profile must also be `const`-compatible.
5. No `BodyCostPerTick` type exists yet.

## Architecture Check

1. Adding `CommodityConsumableProfile` as an `Option` field on `CommodityKindSpec` keeps backward compatibility — non-consumables return `None`.
2. `BodyCostPerTick` is a standalone struct in `worldwake-core` so both `worldwake-sim` (action framework) and `worldwake-systems` (metabolism system) can reference it.
3. Both types use `Permille` — consistent with no-float policy.

## What to Change

### 1. Add `CommodityConsumableProfile` to `items.rs`

```rust
pub struct CommodityConsumableProfile {
    pub consumption_ticks_per_unit: NonZeroU32,
    pub hunger_relief_per_unit: Permille,
    pub thirst_relief_per_unit: Permille,
    pub bladder_fill_per_unit: Permille,
}
```

### 2. Extend `CommodityKindSpec` in `items.rs`

Add `pub consumable_profile: Option<CommodityConsumableProfile>` to `CommodityKindSpec`.

Update `CommodityKind::spec()` match arms:
- `Apple`: hunger relief high, thirst relief moderate, bladder low
- `Bread`: hunger relief high, thirst relief zero, bladder low
- `Grain`: hunger relief moderate, thirst relief zero, bladder low
- `Water`: hunger relief zero, thirst relief high, bladder high
- `Firewood`, `Medicine`, `Coin`, `Waste`: `None` (not consumable for needs purposes)

### 3. Add `BodyCostPerTick` to a suitable location in `worldwake-core`

```rust
pub struct BodyCostPerTick {
    pub hunger_delta: Permille,
    pub thirst_delta: Permille,
    pub fatigue_delta: Permille,
    pub dirtiness_delta: Permille,
}
```

Include `BodyCostPerTick::zero()` for actions with no body cost.

This can go in `needs.rs` or a new `body_cost.rs` module — the important thing is it's in `worldwake-core`.

## Files to Touch

- `crates/worldwake-core/src/items.rs` (modify — add `CommodityConsumableProfile`, extend `CommodityKindSpec`, update `spec()`)
- `crates/worldwake-core/src/needs.rs` or `crates/worldwake-core/src/body_cost.rs` (modify or new — add `BodyCostPerTick`)
- `crates/worldwake-core/src/lib.rs` (modify — re-exports if new module)

## Out of Scope

- Consumption action handlers (E09NEEMET-007)
- How `BodyCostPerTick` is attached to action instances (E09NEEMET-005 reads it)
- Trade category or pricing changes (E11)
- New commodity kinds

## Acceptance Criteria

### Tests That Must Pass

1. `CommodityKind::Apple.spec().consumable_profile` is `Some(...)` with positive hunger relief.
2. `CommodityKind::Water.spec().consumable_profile` is `Some(...)` with positive thirst relief and high bladder fill.
3. `CommodityKind::Firewood.spec().consumable_profile` is `None`.
4. `CommodityKind::Waste.spec().consumable_profile` is `None`.
5. All food commodities have non-zero `consumption_ticks_per_unit`.
6. `BodyCostPerTick::zero()` has all deltas at `Permille(0)`.
7. Both types round-trip through bincode serialization.
8. `CommodityKind::spec()` remains `const fn` (or explain why not and adjust).
9. Existing suite: `cargo test -p worldwake-core`

### Invariants

1. Consumable effects are data-driven from commodity spec, never hardcoded in action logic.
2. No floating-point types.
3. Non-consumable commodities return `None` for consumable profile.
4. `consumption_ticks_per_unit` is `NonZeroU32` — no zero-duration consumption.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/items.rs` (unit tests) — consumable profile for each commodity kind
2. `crates/worldwake-core/src/needs.rs` or `body_cost.rs` — BodyCostPerTick construction, zero, serialization

### Commands

1. `cargo test -p worldwake-core`
2. `cargo clippy --workspace`
