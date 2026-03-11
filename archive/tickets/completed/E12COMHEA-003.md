# E12COMHEA-003: Sword/Bow commodity weapons and weapon taxonomy unification

**Status**: ✅ COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — `worldwake-core` items taxonomy, load tables, and affected world/core tests
**Deps**: E04 (`CommodityKind`, `TradeCategory`, `UniqueItemKind`), E12COMHEA-001 (`CombatWeaponRef`)

## Problem

E12 wants combatable weapons to be stackable commodities with explicit weapon profiles. The current codebase still has a parallel generic `UniqueItemKind::Weapon` path, which creates two competing representations for “weapon”:

- unique-item taxonomy says a weapon is a singular unique item
- combat wound provenance already points toward commodity-backed weapons
- E12’s combat spec expects `Sword` and `Bow` as commodity kinds

That split is not a useful abstraction boundary. It is duplicated taxonomy with no clear semantic distinction, and it will force later combat, loot, trade, and inventory code to answer “which weapon path is authoritative?” every time.

## Assumption Reassessment (2026-03-11)

1. `CommodityKind` currently has 8 variants and no weapon commodities — confirmed.
2. `TradeCategory::Weapon` already exists — confirmed.
3. `CommodityKindSpec` currently carries `trade_category`, `physical_profile`, and `consumable_profile` — confirmed.
4. `UniqueItemKind` still contains `Weapon`, but current usages are limited to core tests and generic unique-item examples. No production gameplay flow depends on a distinct unique-weapon architecture today.
5. `CombatWeaponRef::Commodity(CommodityKind)` already exists from E12COMHEA-001, which means the wound/combat provenance model is already leaning toward commodity weapons.
6. The previous version of this ticket understated the architecture issue by treating weapon commodities as an additive feature. In reality, the cleaner change is unification: add commodity weapons and remove `UniqueItemKind::Weapon`.

## Architecture Check

### Why Change The Current Architecture

1. Commodity weapons are the better fit for Phase 2. Swords and bows are durable trade goods with stable profiles and straightforward conserved quantities; they do not need a separate unique-item taxonomy just to be weapons.
2. Removing `UniqueItemKind::Weapon` is cleaner than keeping both. Keeping both would force future code to support aliasing or branching behavior for identical concepts, which violates the repo’s no-backward-compatibility rule.
3. `CombatWeaponProfile` belongs in the commodity catalog, not in per-entity state. Weapon characteristics are catalog data just like consumable characteristics.

### Guardrails

1. This ticket should not invent a compatibility bridge between unique weapons and commodity weapons.
2. If a future design genuinely needs singular named relic weapons, that should be a different concept layered on top of the combat commodity model, not a duplicate generic `Weapon` kind.
3. This ticket should stay focused on taxonomy and static profiles, not combat resolution logic.

## Revised Scope

1. Add `Sword` and `Bow` to `CommodityKind`.
2. Add `CombatWeaponProfile` to commodity spec data.
3. Expose combat weapon profile access through `CommodityKindSpec`.
4. Remove `UniqueItemKind::Weapon` entirely.
5. Update all affected tests and exhaustive matches to the unified taxonomy.
6. Replace legacy unique-weapon fixtures with the nearest appropriate remaining unique item kind.

## What to Change

### 1. Add `CombatWeaponProfile`

Add a catalog data struct in `items.rs`:

```rust
pub struct CombatWeaponProfile {
    pub base_wound_severity: Permille,
    pub base_bleed_rate: Permille,
    pub attack_duration_ticks: NonZeroU32,
}
```

Derive `Copy + Clone + Debug + Eq + PartialEq + Serialize + Deserialize`.

### 2. Extend `CommodityKind`

Add:

```rust
Sword,
Bow,
```

Both should map to `TradeCategory::Weapon`.

### 3. Extend `CommodityKindSpec`

Add:

```rust
pub combat_weapon_profile: Option<CombatWeaponProfile>,
```

This keeps combat catalog data co-located with the commodity catalog rather than adding a parallel method table elsewhere.

### 4. Define catalog profiles

Give `Sword` and `Bow` explicit:

- load
- trade category
- absent consumable profile
- present combat weapon profile

All non-weapon commodities must return `combat_weapon_profile: None`.

### 5. Remove `UniqueItemKind::Weapon`

Delete the variant and update:

- `UniqueItemKind::ALL`
- `UniqueItemKind::spec()`
- tests and fixtures that still use generic unique weapons

No alias, no deprecated wrapper, no fallback mapping.

## Files to Touch

- `crates/worldwake-core/src/items.rs` (modify)
- `crates/worldwake-core/src/load.rs` (modify — fixture expectations)
- `crates/worldwake-core/src/world.rs` (modify — unique-item fixtures/tests)
- `crates/worldwake-core/src/component_tables.rs` (modify — unique-item fixture)
- `crates/worldwake-core/src/wounds.rs` (modify — combat wound sample should use a real weapon commodity)
- `crates/worldwake-core/src/lib.rs` (modify — re-export `CombatWeaponProfile`)

## Out of Scope

- Attack/defend/heal action handlers
- Combat hit resolution
- Armor or mitigation systems
- Any scheduler or wound progression logic
- Adding unique relic-weapon semantics

## Acceptance Criteria

### Tests That Must Pass

1. `CommodityKind::Sword` and `CommodityKind::Bow` exist and have `TradeCategory::Weapon`.
2. `CombatWeaponProfile` satisfies the required trait bounds and round-trips through bincode.
3. `Sword.spec().combat_weapon_profile` returns `Some(...)`.
4. `Bow.spec().combat_weapon_profile` returns `Some(...)`.
5. Non-weapon commodities return `combat_weapon_profile: None`.
6. `UniqueItemKind::Weapon` no longer exists.
7. Core load/item/world tests pass after replacing the old unique-weapon fixtures.
8. `cargo test -p worldwake-core` passes.
9. `cargo test --workspace` passes.
10. `cargo clippy --workspace --all-targets` passes.

### Invariants

1. No `f32`/`f64`; use `Permille` and `NonZeroU32`.
2. Combat weapon stats remain catalog data, not entity components.
3. There is only one generic weapon taxonomy after this change: commodity weapons.
4. No compatibility aliases or dual-path weapon handling is introduced.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/items.rs`
   - commodity variant list and spec coverage
   - combat weapon profile coverage
   - removal of unique generic weapon variant
2. `crates/worldwake-core/src/load.rs`
   - commodity load expectations for sword/bow
   - updated unique-item load fixtures
3. `crates/worldwake-core/src/world.rs`
   - updated unique-item creation/query fixtures to use remaining unique kinds
4. `crates/worldwake-core/src/component_tables.rs`
   - updated unique-item fixture values
5. `crates/worldwake-core/src/wounds.rs`
   - combat wound sample uses a real weapon commodity

### Commands

1. `cargo test -p worldwake-core -- items`
2. `cargo test -p worldwake-core`
3. `cargo test --workspace`
4. `cargo clippy --workspace --all-targets`

## Outcome

- Completion date: 2026-03-11
- What actually changed:
  - corrected the ticket first so it captured the real architectural problem: duplicate generic weapon taxonomies
  - added `CommodityKind::Sword` and `CommodityKind::Bow`
  - added `CombatWeaponProfile` and wired it into `CommodityKindSpec` as catalog data
  - removed `UniqueItemKind::Weapon` entirely instead of preserving a parallel path
  - updated legacy unique-weapon fixtures across core tests to use appropriate remaining unique-item kinds
  - updated combat wound samples to reference a real weapon commodity instead of a non-weapon placeholder
- Deviations from original plan:
  - expanded scope beyond “add two commodity variants” because the additive approach would have preserved a duplicated weapon model
  - kept the cleanup limited to taxonomy/catalog/test fallout; no combat action or scheduler behavior was pulled in
- Verification results:
  - `cargo test -p worldwake-core -- items` passed
  - `cargo test -p worldwake-core` passed
  - `cargo test --workspace` passed
  - `cargo clippy --workspace --all-targets` passed
