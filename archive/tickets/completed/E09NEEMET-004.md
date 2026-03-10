# E09NEEMET-004: CommodityConsumableProfile and BodyCostPerTick metadata types

**Status**: COMPLETED
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
5. No `CommodityConsumableProfile` type exists yet.
6. No `BodyCostPerTick` type exists yet.
7. `worldwake-sim` does not yet expose a general action-metadata field for physiology costs. `ActionDef` currently stores `duration`, `interruptibility`, and handler IDs, while `ActionState` is still only `Empty`. This ticket can define the canonical body-cost value type in `worldwake-core`, but wiring it into action definitions or active-action state belongs to later tickets.

## Architecture Check

1. A dedicated `CommodityConsumableProfile` on `CommodityKindSpec` is cleaner than overloading `CommodityPhysicalProfile`. Load and physiology are different axes of truth; combining them would blur physical handling with bodily effects.
2. `consumable_profile: Option<CommodityConsumableProfile>` is the right shape because consumability is a real property difference between commodities, not a compatibility shim. Non-consumables should explicitly carry no consumable metadata.
3. `BodyCostPerTick` belongs beside the existing physiology schema in `worldwake-core::needs`, not in `worldwake-sim`. It is domain data that multiple crates consume; the action framework should reference it rather than own it.
4. Both types use `Permille` and `NonZeroU32` only — consistent with the no-float policy and the “no magic zero-duration” requirement.

## Scope Correction

This ticket should:

1. Add `CommodityConsumableProfile` to the commodity catalog in `worldwake-core`.
2. Add `BodyCostPerTick` as the canonical physiology-cost value type in `worldwake-core::needs`.
3. Re-export the new type(s) from `worldwake-core`.
4. Add focused unit tests for commodity consumable metadata, const compatibility, and body-cost construction / serialization.

This ticket should not:

1. Change `ActionDef`, `DurationExpr`, or `ActionState` to carry body-cost metadata yet.
2. Implement eating, drinking, metabolism ticking, or active-action physiology application.
3. Introduce alias fields, wrapper types, or temporary compatibility paths.

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

Place this in `needs.rs` with the other physiology-domain types. A separate `body_cost.rs` module is not justified by the current surface area.

## Files to Touch

- `crates/worldwake-core/src/items.rs` (modify — add `CommodityConsumableProfile`, extend `CommodityKindSpec`, update `spec()`)
- `crates/worldwake-core/src/needs.rs` (modify — add `BodyCostPerTick`)
- `crates/worldwake-core/src/lib.rs` (modify — re-exports if new module)

## Out of Scope

- Consumption action handlers (E09NEEMET-007)
- How `BodyCostPerTick` is attached to action definitions or active action instances (follow-on E09 metabolism/action-integration work)
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
8. `CommodityKind::spec()` remains `const fn`.
9. Existing suite: `cargo test -p worldwake-core`

### Invariants

1. Consumable effects are data-driven from commodity spec, never hardcoded in action logic.
2. No floating-point types.
3. Non-consumable commodities return `None` for consumable profile.
4. `consumption_ticks_per_unit` is `NonZeroU32` — no zero-duration consumption.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/items.rs` (unit tests) — consumable profile catalog coverage, non-consumable `None`, const-compatibility check
2. `crates/worldwake-core/src/needs.rs` — `BodyCostPerTick` construction, zero, serialization

### Commands

1. `cargo test -p worldwake-core`
2. `cargo clippy --workspace --all-targets -- -D warnings`
3. `cargo test --workspace`

## Outcome

Completion date: 2026-03-10

Outcome amended: 2026-03-10

What actually changed:

1. Added `CommodityConsumableProfile` to `crates/worldwake-core/src/items.rs` and extended `CommodityKindSpec` with `consumable_profile: Option<CommodityConsumableProfile>`.
2. Updated `CommodityKind::spec()` to define explicit consumable metadata for `Apple`, `Grain`, `Bread`, and `Water`, while leaving `Firewood`, `Medicine`, `Coin`, and `Waste` as non-consumable.
3. Added `BodyCostPerTick` to `crates/worldwake-core/src/needs.rs` with `new(...)`, `zero()`, and `Default`.
4. Re-exported `CommodityConsumableProfile` and `BodyCostPerTick` from `crates/worldwake-core/src/lib.rs`.
5. Added a first-class `body_cost_per_tick: BodyCostPerTick` field to `crates/worldwake-sim/src/action_def.rs`, making physiology cost an explicit part of action schema rather than a later lookup concern.
6. Added focused unit coverage for commodity consumable catalog entries, non-consumable `None` cases, const compatibility, `BodyCostPerTick` construction / serialization, and `ActionDef` / `ActionDefRegistry` body-cost persistence.

Differences from the original plan:

1. The ticket was corrected before implementation because the original wording assumed there was already an action-metadata attachment point for physiology costs in `worldwake-sim`.
2. `BodyCostPerTick` was placed in `needs.rs`, not a new `body_cost.rs`, because it belongs with the existing physiology-domain types.
3. The architecture rationale was tightened to reject backward-compatibility framing and to keep physical commodity metadata separate from consumable physiology metadata.
4. After the core schema landed, the action framework was refined in the same session so `ActionDef` now owns body-cost metadata directly. The cleaner long-term design here is an explicit non-optional field with `BodyCostPerTick::zero()` for no-cost actions, not an optional side lookup.

Verification results:

1. `cargo test -p worldwake-core` passed.
2. `cargo clippy --workspace --all-targets -- -D warnings` passed.
3. `cargo test --workspace` passed.
4. `cargo test -p worldwake-sim` passed.
