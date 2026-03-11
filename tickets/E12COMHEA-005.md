# E12COMHEA-005: Constraint, Precondition, and DurationExpr extensions

**Status**: PENDING
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — worldwake-sim action_semantics module
**Deps**: E07 (action_semantics.rs)

## Problem

The action semantics enums need new variants to support combat actions:
- `Constraint::ActorNotIncapacitated` and `Constraint::ActorNotDead` for actor liveness checks
- `Precondition::TargetAlive`, `TargetDead`, `TargetIsAgent` for target validation
- `DurationExpr::Indefinite` for Defend (runs until cancelled)
- `DurationExpr::CombatWeapon` for Attack (resolves from weapon profile)

## Assumption Reassessment (2026-03-11)

1. `Constraint` has 8 variants including `ActorAlive` — confirmed. `ActorNotIncapacitated` and `ActorNotDead` are distinct from `ActorAlive`.
2. `Precondition` has 14 variants — confirmed. New target variants follow the `u8` index pattern.
3. `DurationExpr` has 5 variants — confirmed. `Indefinite` is conceptually new (no tick count).
4. `DurationExpr::resolve_for()` returns `Result<u32, String>` — `Indefinite` needs special handling (cannot resolve to a finite tick count).
5. All enum types derive `Copy + Clone + Eq + Ord + Hash + Debug + Serialize + Deserialize` — confirmed.
6. Tests use `ALL_CONSTRAINTS`, `ALL_PRECONDITIONS`, `ALL_DURATION_EXPRS` arrays for exhaustive coverage — must be updated.

## Architecture Check

1. `ActorNotDead` is separate from `ActorAlive` because alive means "entity exists and is an agent", while not-dead means "no `DeadAt` component". They are independent checks.
2. `ActorNotIncapacitated` checks wound load against `CombatProfile.incapacitation_threshold` — this is a derived check at validation time.
3. `DurationExpr::Indefinite` means "action runs until cancelled". `resolve_for()` should return a sentinel or error since indefinite actions don't resolve to a finite count — follow whatever pattern the scheduler uses for indefinite durations.
4. `DurationExpr::CombatWeapon` resolves by checking actor's weapon payload → weapon profile duration, falling back to `CombatProfile.unarmed_attack_ticks`.

## What to Change

### 1. Add Constraint variants

```rust
ActorNotIncapacitated,
ActorNotDead,
```

### 2. Add Precondition variants

```rust
TargetAlive(u8),
TargetDead(u8),
TargetIsAgent(u8),
```

### 3. Add DurationExpr variants

```rust
Indefinite,
CombatWeapon,
```

### 4. Update `DurationExpr::fixed_ticks()`

Return `None` for `Indefinite` and `CombatWeapon`.

### 5. Update `DurationExpr::resolve_for()`

- `Indefinite`: return special sentinel value (e.g., `u32::MAX`) or `Err` — document the convention
- `CombatWeapon`: resolve from actor's action payload weapon → `CombatWeaponProfile.attack_duration_ticks`, falling back to `CombatProfile.unarmed_attack_ticks`

### 6. Update test arrays

Add new variants to `ALL_CONSTRAINTS`, `ALL_PRECONDITIONS`, `ALL_DURATION_EXPRS`.

## Files to Touch

- `crates/worldwake-sim/src/action_semantics.rs` (modify)

## Out of Scope

- Validation logic for new Constraint/Precondition variants in start_gate (E12COMHEA-007)
- Action definitions that use these variants (E12COMHEA-010/011/012/013)
- CombatProfile or DeadAt component definitions (E12COMHEA-002)
- Wound helper functions (E12COMHEA-006)

## Acceptance Criteria

### Tests That Must Pass

1. All new Constraint variants round-trip through bincode
2. All new Precondition variants round-trip through bincode
3. `DurationExpr::Indefinite` round-trips through bincode
4. `DurationExpr::CombatWeapon` round-trips through bincode
5. `DurationExpr::Indefinite.fixed_ticks()` returns `None`
6. `DurationExpr::CombatWeapon.fixed_ticks()` returns `None`
7. All new types satisfy `Copy + Clone + Eq + Ord + Hash + Debug + Serialize + Deserialize`
8. Existing suite: `cargo test -p worldwake-sim`

### Invariants

1. All enum variants derive the same trait set as existing variants
2. No `f32`/`f64` anywhere
3. Precondition target indices use `u8`
4. All exhaustive match arms updated

## Test Plan

### New/Modified Tests

1. `crates/worldwake-sim/src/action_semantics.rs` — update `ALL_*` arrays, add bincode roundtrip, `fixed_ticks` coverage

### Commands

1. `cargo test -p worldwake-sim -- action_semantics`
2. `cargo test --workspace && cargo clippy --workspace`
