# E12COMHEA-001: Extend Wound with bleed_rate_per_tick, add CombatWeaponRef + WoundCause::Combat

**Status**: ✅ COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — worldwake-core wounds module
**Deps**: E09NEEMET-001 (shared wound schema), E09NEEMET-006 (deprivation wound creation), `specs/E12-combat-health.md`, `tickets/E12COMHEA-000-index.md`

## Problem

The existing `Wound` schema is still the E09 deprivation-only version. It lacks the per-wound bleeding state that E12 wound progression needs, and `WoundCause` still cannot represent combat provenance. This ticket is the schema-breaking extension that lets later combat and wound-progression tickets operate on one shared, explicit wound carrier rather than inventing parallel combat-only damage state.

## Assumption Reassessment (2026-03-11)

1. `Wound` struct exists in `crates/worldwake-core/src/wounds.rs` with fields: `body_part`, `cause`, `severity`, `inflicted_at` — confirmed.
2. `WoundCause` only has `Deprivation(DeprivationKind)` — confirmed.
3. `WoundList` already registered as a component on `EntityKind::Agent` — confirmed.
4. There is only one production wound-construction site today: `crates/worldwake-systems/src/needs.rs` creates deprivation wounds. The other `Wound { ... }` call sites currently in the tree are test fixtures and serialization samples spread across `worldwake-core`, `worldwake-sim`, and `worldwake-systems` — they must still be updated because this ticket intentionally breaks the schema.
5. The original ticket understated test impact. Adding a field to `Wound` touches not just `wounds.rs` tests, but every fixture that round-trips `WoundList` or embeds a sample wound in component/world serialization coverage.
6. The original ticket referenced prior tickets as `E12COMHEA-0001/0002`; those files do not exist. The relevant planning reference is `tickets/E12COMHEA-000-index.md`. The relevant wound/deprivation precedent is `archive/tickets/completed/E09NEEMET-001.md` and `archive/tickets/E09NEEMET-006.md`.
7. `CommodityKind` does not yet include `Sword` or `Bow`; those arrive in `E12COMHEA-003`. This ticket may introduce `CombatWeaponRef::Commodity(CommodityKind)` as planned schema, but it must not assume weapon commodities already exist at runtime.
8. The codebase already has `UniqueItemKind::Weapon`, which means the broader weapon model is not fully unified yet. That architectural tension should be noted, but resolving item taxonomy is outside this ticket.

## Architecture Check

### What Is Beneficial To Add Now

1. Adding `bleed_rate_per_tick` directly to `Wound` is the right architecture. Bleeding is wound state, not a derived global score and not a parallel combat-only component.
2. Extending `WoundCause` with `Combat { attacker, weapon }` is also the right direction. Combat provenance belongs on the wound itself so later death-cause tracing and healing logic remain state-driven.
3. `CombatWeaponRef` belongs alongside `WoundCause` in `wounds.rs` because it is wound provenance, not authoritative inventory state.

### What This Ticket Should Not Pretend To Solve

1. The existing item model already contains `UniqueItemKind::Weapon`, while the E12 spec plans combatable `CommodityKind` weapons in `E12COMHEA-003`. This ticket should not attempt to reconcile that larger item-architecture question.
2. Because this ticket is schema-first, it should introduce the combat wound provenance types without forcing any aliasing or compatibility layer around the current item model.

## Revised Scope

Implement the E12 wound-schema extension cleanly and update every required compile/test site affected by the intentional break:

1. Add `bleed_rate_per_tick: Permille` to `Wound`.
2. Add `CombatWeaponRef`.
3. Add `WoundCause::Combat { attacker, weapon }`.
4. Update the E09 deprivation wound constructor to pass `Permille(0)`.
5. Update all affected serialization fixtures and tests so the new schema is covered everywhere it is already treated as first-class state.

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

Update:

- the E09 deprivation wound constructor in `crates/worldwake-systems/src/needs.rs`
- all `Wound` sample/fixture values used by serialization, component-table, world, verification, and trade-valuation tests

Every non-bleeding wound introduced by this ticket should pass `Permille(0)`.

## Files to Touch

- `crates/worldwake-core/src/wounds.rs` (modify)
- `crates/worldwake-systems/src/needs.rs` (modify — production deprivation wound creation)
- `crates/worldwake-core/src/component_tables.rs` (modify — fixture updates)
- `crates/worldwake-core/src/delta.rs` (modify — fixture updates)
- `crates/worldwake-core/src/verification.rs` (modify — fixture updates)
- `crates/worldwake-core/src/world.rs` (modify — fixture/sample updates)
- `crates/worldwake-sim/src/trade_valuation.rs` (modify — fixture updates)
- `crates/worldwake-systems/tests/e09_needs_integration.rs` or existing systems/core tests that assert wound payloads (modify if needed for the new field)

## Out of Scope

- CombatProfile component (E12COMHEA-002)
- DeadAt component (E12COMHEA-002)
- CombatWeaponProfile struct (E12COMHEA-003)
- Sword/Bow commodity variants (E12COMHEA-003)
- Wound helper functions (E12COMHEA-006)
- Any action definitions or handlers
- Resolving the broader `CommodityKind` weapon vs `UniqueItemKind::Weapon` architecture

## Acceptance Criteria

### Tests That Must Pass

1. `CombatWeaponRef` satisfies `Copy + Clone + Eq + Ord + Hash + Serialize + Deserialize`
2. `WoundCause::Combat` variant round-trips through bincode
3. `Wound` with `bleed_rate_per_tick > 0` round-trips through bincode
4. `Wound` with `bleed_rate_per_tick = Permille(0)` round-trips through bincode
5. Existing `WoundList` bincode round-trip test still passes (updated for new field)
6. Deprivation wounds and combat wounds can coexist in the same `WoundList`
7. Existing core/world/component fixture coverage that serializes or stores `WoundList` still passes after the schema change
8. Existing E09 deprivation tests still pass with explicit zero bleed-rate wounds
9. Existing suite: `cargo test -p worldwake-core`
10. Relevant systems coverage: `cargo test -p worldwake-systems`
11. Full suite: `cargo test --workspace`
12. `cargo clippy --workspace`

### Invariants

1. All enum types derive `Copy + Clone + Eq + Ord + Hash + Serialize + Deserialize`
2. No `f32`/`f64` anywhere — `Permille` only
3. `WoundCause` remains `Copy` (all variants must be `Copy`)
4. Existing E09 deprivation wound creation compiles and works with the added field
5. No compatibility shims, alias enums, or duplicate combat-only wound carriers are introduced

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/wounds.rs` — extend trait/serialization coverage for `CombatWeaponRef`, `WoundCause::Combat`, zero/non-zero bleed-rate wounds, and mixed-cause `WoundList`
2. Existing fixture-based tests in `worldwake-core`, `worldwake-sim`, and `worldwake-systems` — update expectations and constructors to include `bleed_rate_per_tick`

### Commands

1. `cargo test -p worldwake-core -- wounds`
2. `cargo test -p worldwake-systems`
3. `cargo test --workspace`
4. `cargo clippy --workspace`

## Outcome

- Completion date: 2026-03-11
- What actually changed:
  - corrected the ticket assumptions and scope before implementation so it matched the real wound/deprivation code and test surface
  - extended `Wound` with `bleed_rate_per_tick`, added `CombatWeaponRef`, and added `WoundCause::Combat { attacker, weapon }`
  - updated the production deprivation wound constructor to emit explicit zero-bleed wounds
  - updated all affected wound fixtures and serialization samples across `worldwake-core` and `worldwake-sim`
  - strengthened wound and deprivation tests to cover combat-cause serialization, zero/non-zero bleed-rate round-trips, mixed-cause `WoundList` values, and the invariant that E09 deprivation wounds are non-bleeding
- Deviations from original plan:
  - expanded the file-impact scope beyond `wounds.rs` and the needs system because the schema break also affected core/sim fixture coverage
  - kept the broader `CommodityKind` weapon versus `UniqueItemKind::Weapon` architecture issue explicitly out of scope; this ticket only adds wound provenance types
- Verification results:
  - `cargo test -p worldwake-core -- wounds` passed
  - `cargo test -p worldwake-systems` passed
  - `cargo test --workspace` passed
  - `cargo clippy --workspace --all-targets` passed
