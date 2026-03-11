# E12COMHEA-003: Sword/Bow CommodityKind variants + CombatWeaponProfile

**Status**: PENDING
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — worldwake-core items module
**Deps**: E04 (CommodityKind, TradeCategory, CommodityKindSpec)

## Problem

The item system needs weapon commodities (`Sword`, `Bow`) and a `CombatWeaponProfile` struct so that weapon properties (wound severity, bleed rate, attack duration) are data-driven rather than hardcoded. Each weapon commodity returns its profile via `CommodityKind::combat_weapon_profile()`.

## Assumption Reassessment (2026-03-11)

1. `CommodityKind` exists in `crates/worldwake-core/src/items.rs` with 8 variants — confirmed.
2. `TradeCategory::Weapon` already exists in the enum — confirmed.
3. `CommodityKindSpec` has fields: `trade_category`, `per_unit_load`, `consumable_profile`, `body_cost_per_tick` — confirmed.
4. Each `CommodityKind` variant has a `spec()` method returning `&'static CommodityKindSpec` — confirmed.
5. Weapons are not consumable — `consumable_profile: None`.

## Architecture Check

1. `CombatWeaponProfile` is a simple data struct, not a component. It lives alongside `CommodityKindSpec` in items.rs.
2. Adding `combat_weapon_profile` as an `Option<CombatWeaponProfile>` field on `CommodityKindSpec` follows the existing pattern (like `consumable_profile`).
3. Alternatively, `combat_weapon_profile()` can be a method on `CommodityKind` directly. Follow whichever pattern is cleaner.

## What to Change

### 1. Add `CombatWeaponProfile` struct

```rust
pub struct CombatWeaponProfile {
    pub base_wound_severity: Permille,
    pub base_bleed_rate: Permille,
    pub attack_duration_ticks: NonZeroU32,
}
```

### 2. Add `Sword` and `Bow` to `CommodityKind`

Both with `TradeCategory::Weapon`.

### 3. Add `combat_weapon_profile` to `CommodityKindSpec` or as method

Add `combat_weapon_profile: Option<CombatWeaponProfile>` field on `CommodityKindSpec`, or add `combat_weapon_profile() -> Option<CombatWeaponProfile>` method on `CommodityKind`. Ensure non-weapon commodities return `None`.

### 4. Define weapon profiles for Sword and Bow

Provide concrete `Permille` and `NonZeroU32` values (per Principle 11 — profile-driven, not magic numbers). Values should be reasonable defaults that can be tuned later.

### 5. Update all exhaustive matches on CommodityKind

Any existing `match` statements on `CommodityKind` must include the new variants.

## Files to Touch

- `crates/worldwake-core/src/items.rs` (modify — add variants, struct, profiles)
- Any file with exhaustive `match` on `CommodityKind` (modify — add new arms)

## Out of Scope

- CombatWeaponRef enum (E12COMHEA-001 — already in wounds.rs)
- CombatProfile component (E12COMHEA-002)
- Attack action handler (E12COMHEA-010)
- Hit resolution logic
- Armor or damage mitigation
- Per-unit load values for weapons (use reasonable defaults; tuning is future work)

## Acceptance Criteria

### Tests That Must Pass

1. `CommodityKind::Sword` and `CommodityKind::Bow` exist and have `TradeCategory::Weapon`
2. `CombatWeaponProfile` derives `Copy + Clone + Debug + Eq + PartialEq + Serialize + Deserialize`
3. `Sword.spec().combat_weapon_profile` (or equivalent) returns `Some(CombatWeaponProfile { ... })`
4. `Bow.spec().combat_weapon_profile` returns `Some(CombatWeaponProfile { ... })`
5. Non-weapon commodities (Apple, Bread, etc.) return `None` for combat weapon profile
6. `CommodityKind::Sword` and `CommodityKind::Bow` round-trip through bincode
7. Existing commodity tests still pass
8. Existing suite: `cargo test -p worldwake-core`

### Invariants

1. No `f32`/`f64` — `Permille` and `NonZeroU32` only
2. All `CommodityKind` match arms remain exhaustive (no `_ =>` wildcards in authoritative code)
3. Weapon profiles are static data, not stored per-entity

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/items.rs` — weapon variant existence, profile access, trait bounds

### Commands

1. `cargo test -p worldwake-core -- items`
2. `cargo test --workspace && cargo clippy --workspace`
