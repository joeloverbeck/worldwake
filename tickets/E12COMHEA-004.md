# E12COMHEA-004: ActionPayload Combat + Loot variants

**Status**: PENDING
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — worldwake-sim action_payload module
**Deps**: E07 (ActionPayload enum), E12COMHEA-001 (CombatWeaponRef)

## Problem

The `ActionPayload` enum needs `Combat(CombatActionPayload)` and `Loot(LootActionPayload)` variants so that combat and looting actions can carry their target/weapon data.

## Assumption Reassessment (2026-03-11)

1. `ActionPayload` exists in `crates/worldwake-sim/src/action_payload.rs` with `None`, `Harvest`, `Craft`, `Trade` — confirmed.
2. Each variant has a typed accessor (`as_harvest()`, `as_craft()`, `as_trade()`) — confirmed, pattern must be followed.
3. `ActionPayload` derives `Clone, Debug, Default, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize` — confirmed.
4. `CombatWeaponRef` will exist in `worldwake-core::wounds` after E12COMHEA-001.
5. `EntityId` is in `worldwake-core`.

## Architecture Check

1. Following the existing pattern exactly: new payload structs + new enum variants + new typed accessors.
2. All existing `match` arms on `ActionPayload` must be updated (exhaustive matches).

## What to Change

### 1. Add `CombatActionPayload` struct

```rust
pub struct CombatActionPayload {
    pub target: EntityId,
    pub weapon: CombatWeaponRef,
}
```

### 2. Add `LootActionPayload` struct

```rust
pub struct LootActionPayload {
    pub target: EntityId,
}
```

### 3. Add variants to `ActionPayload`

```rust
Combat(CombatActionPayload),
Loot(LootActionPayload),
```

### 4. Add typed accessors

```rust
pub const fn as_combat(&self) -> Option<&CombatActionPayload> { ... }
pub const fn as_loot(&self) -> Option<&LootActionPayload> { ... }
```

### 5. Update all exhaustive matches on ActionPayload

All existing `as_*` methods must include the new variants in their non-matching arms.

## Files to Touch

- `crates/worldwake-sim/src/action_payload.rs` (modify)
- Any file with exhaustive `match` on `ActionPayload` (modify — add new arms)

## Out of Scope

- HealActionPayload — Heal action uses existing Medicine consumable patterns; if a dedicated payload is needed, it will be added in E12COMHEA-013
- CombatWeaponRef definition (E12COMHEA-001)
- Action definitions or handlers (E12COMHEA-010/011/012/013)
- Hit resolution logic

## Acceptance Criteria

### Tests That Must Pass

1. `CombatActionPayload` satisfies `Clone + Debug + Eq + Ord + Serialize + Deserialize`
2. `LootActionPayload` satisfies `Clone + Debug + Eq + Ord + Serialize + Deserialize`
3. `ActionPayload::Combat(...)` round-trips through bincode
4. `ActionPayload::Loot(...)` round-trips through bincode
5. `as_combat()` returns `Some` for Combat variant, `None` for all others
6. `as_loot()` returns `Some` for Loot variant, `None` for all others
7. Existing accessor tests still pass (updated for new variants in non-matching arms)
8. `ActionPayload::default()` remains `ActionPayload::None`
9. Existing suite: `cargo test -p worldwake-sim`

### Invariants

1. All payload types derive the same trait set as existing payloads
2. No `f32`/`f64` in payload structs
3. All `ActionPayload` matches remain exhaustive

## Test Plan

### New/Modified Tests

1. `crates/worldwake-sim/src/action_payload.rs` — trait bounds, bincode roundtrip, accessor coverage

### Commands

1. `cargo test -p worldwake-sim -- action_payload`
2. `cargo test --workspace && cargo clippy --workspace`
