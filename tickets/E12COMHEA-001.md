# E12COMHEA-001: Extend Wound with bleed_rate_per_tick, add CombatWeaponRef + WoundCause::Combat

**Status**: PENDING
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — worldwake-core wounds module
**Deps**: E09 (WoundList/Wound already exist)

## Problem

The existing `Wound` struct lacks a `bleed_rate_per_tick` field needed for combat wounds that bleed. The `WoundCause` enum only has `Deprivation` — combat wounds need their own cause variant referencing the attacker and weapon. A new `CombatWeaponRef` enum is needed to distinguish unarmed vs commodity-based weapons.

## Assumption Reassessment (2026-03-11)

1. `Wound` struct exists in `crates/worldwake-core/src/wounds.rs` with fields: `body_part`, `cause`, `severity`, `inflicted_at` — confirmed.
2. `WoundCause` only has `Deprivation(DeprivationKind)` — confirmed.
3. `WoundList` already registered as a component on `EntityKind::Agent` — confirmed.
4. E09 deprivation wounds should pass `Permille(0)` for `bleed_rate_per_tick` — must update E09 deprivation code to include this field.

## Architecture Check

1. Adding a field to `Wound` is a breaking change to all construction sites. This is the correct approach since `bleed_rate_per_tick` is fundamental to wound progression. All existing `Wound` construction sites (E09 deprivation) must be updated to pass `Permille(0)`.
2. `CombatWeaponRef` lives in wounds.rs alongside `WoundCause` since it's part of the wound cause chain, not the item system.

## What to Change

### 1. Add `bleed_rate_per_tick` field to `Wound`

```rust
pub struct Wound {
    pub body_part: BodyPart,
    pub cause: WoundCause,
    pub severity: Permille,
    pub inflicted_at: Tick,
    pub bleed_rate_per_tick: Permille,  // NEW: 0 for non-bleeding wounds
}
```

### 2. Add `CombatWeaponRef` enum

```rust
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum CombatWeaponRef {
    Unarmed,
    Commodity(CommodityKind),
}
```

### 3. Extend `WoundCause` enum

```rust
pub enum WoundCause {
    Deprivation(DeprivationKind),
    Combat { attacker: EntityId, weapon: CombatWeaponRef },  // NEW
}
```

### 4. Fix all existing Wound construction sites

Update E09 deprivation wound creation to include `bleed_rate_per_tick: Permille(0)`.

## Files to Touch

- `crates/worldwake-core/src/wounds.rs` (modify)
- Any file in `crates/worldwake-systems/src/` that constructs `Wound` values (modify — add `bleed_rate_per_tick: Permille(0)`)

## Out of Scope

- CombatProfile component (E12COMHEA-002)
- DeadAt component (E12COMHEA-002)
- CombatWeaponProfile struct (E12COMHEA-003)
- Sword/Bow commodity variants (E12COMHEA-003)
- Wound helper functions (E12COMHEA-006)
- Any action definitions or handlers

## Acceptance Criteria

### Tests That Must Pass

1. `CombatWeaponRef` satisfies `Copy + Clone + Eq + Ord + Hash + Serialize + Deserialize`
2. `WoundCause::Combat` variant round-trips through bincode
3. `Wound` with `bleed_rate_per_tick > 0` round-trips through bincode
4. `Wound` with `bleed_rate_per_tick = Permille(0)` round-trips through bincode
5. Existing `WoundList` bincode round-trip test still passes (updated for new field)
6. Deprivation wounds and combat wounds can coexist in the same `WoundList`
7. Existing suite: `cargo test -p worldwake-core`
8. Full suite: `cargo test --workspace`

### Invariants

1. All enum types derive `Copy + Clone + Eq + Ord + Hash + Serialize + Deserialize`
2. No `f32`/`f64` anywhere — `Permille` only
3. `WoundCause` remains `Copy` (all variants must be `Copy`)
4. Existing E09 deprivation wound creation compiles and works with the added field

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/wounds.rs` — extend existing tests for new field and variants

### Commands

1. `cargo test -p worldwake-core -- wounds`
2. `cargo test --workspace && cargo clippy --workspace`
