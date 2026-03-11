# E12COMHEA-005: Constraint, Precondition, and DurationExpr extensions

**Status**: ✅ COMPLETED
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

1. `E12COMHEA-001` through `E12COMHEA-004` are already complete and archived. `CombatWeaponRef`, `CombatProfile`, `DeadAt`, `Sword`/`Bow`, `CombatWeaponProfile`, and `ActionPayload::Combat`/`Loot` already exist in the codebase, so this ticket must use the live types rather than treating them as future work.
2. `Constraint` currently has 8 variants including `ActorAlive` — confirmed. `ActorNotIncapacitated` and `ActorNotDead` are distinct additions rather than aliases.
3. `Precondition` currently has 14 variants — confirmed. New target variants should continue the existing `u8` target-index convention.
4. `DurationExpr` currently has 5 variants and `DurationExpr::resolve_for()` returns `Result<u32, String>` — confirmed. The action engine still models active action duration as a finite `remaining_ticks: u32`, and `start_gate` also derives finite reservation windows from that duration.
5. `DurationExpr::resolve_for()` currently receives only `world`, `actor`, and `targets`. That is insufficient for `DurationExpr::CombatWeapon`, because weapon choice is carried by `ActionPayload::Combat`, not by targets or actor state. A clean implementation therefore requires passing the effective action payload into duration resolution.
6. `DurationExpr::Indefinite` cannot be implemented honestly as a finite tick count within the current action lifecycle. Using a sentinel such as `u32::MAX` would hide a real architectural gap and couple Defend to fake scheduler semantics. For this ticket, `Indefinite` should be representable at the type level, but resolution must fail explicitly until a later ticket introduces first-class non-finite action durations.
7. All enum types derive `Copy + Clone + Eq + Ord + Hash + Debug + Serialize + Deserialize` today — confirmed and must remain true.
8. `crates/worldwake-sim/src/action_semantics.rs` already contains exhaustive `ALL_CONSTRAINTS`, `ALL_PRECONDITIONS`, and `ALL_DURATION_EXPRS` coverage, plus `resolve_for()` and `fixed_ticks()` tests. Those tests must be extended rather than replaced.

## Architecture Check

1. `ActorNotDead` is a worthwhile separate constraint. The existing `ActorAlive` check is about entity liveness/existence in the current world model, while combat death is modeled explicitly through `DeadAt`. Keeping both checks explicit makes later action definitions easier to read and avoids overloading one semantic with two meanings.
2. `ActorNotIncapacitated` belongs in action semantics even though its validation lands in E12COMHEA-007. Incapacitation is a derived action-start rule, not a stored flag.
3. `DurationExpr::CombatWeapon` is more beneficial than hardcoding combat timings into handlers or action definitions. Duration should remain declarative in the action model and derive from concrete state: the chosen combat payload plus authoritative combat/weapon profiles.
4. The clean design for `CombatWeapon` is payload-driven duration resolution. Inferring the weapon from targets, from inventory, or from action definition IDs would create hidden coupling and make future weapon-bearing actions harder to extend.
5. The current architecture does not yet support indefinite actions as a first-class lifecycle concept. The robust long-term fix is to introduce an explicit action-duration model in the scheduler/action instance layer rather than encoding “forever” as an arbitrary tick count. That larger lifecycle change is outside this ticket.

## Revised Scope

Implement the action-semantics extensions that fit the current engine cleanly, and make the unsupported part explicit rather than faking it:

1. Add `ActorNotIncapacitated` and `ActorNotDead` to `Constraint`.
2. Add `TargetAlive(u8)`, `TargetDead(u8)`, and `TargetIsAgent(u8)` to `Precondition`.
3. Add `Indefinite` and `CombatWeapon` to `DurationExpr`.
4. Keep `fixed_ticks()` returning `None` for both new duration expressions.
5. Refactor `DurationExpr::resolve_for()` to accept the effective `ActionPayload` so `CombatWeapon` can resolve cleanly from payload + authoritative profiles.
6. Make `DurationExpr::CombatWeapon` resolve from `ActionPayload::Combat { weapon }`, using `CommodityKind::spec().combat_weapon_profile.attack_duration_ticks` for armed attacks and `CombatProfile.unarmed_attack_ticks` for unarmed attacks.
7. Make `DurationExpr::Indefinite` return an explicit error documenting that first-class indefinite action lifecycle support has not landed yet.
8. Update existing action-semantic and start-gate tests to cover the new enum variants, payload-aware duration resolution, and the explicit `Indefinite` failure mode.

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

### 5. Refactor `DurationExpr::resolve_for()`

- Extend the signature to accept the effective `ActionPayload`
- `Indefinite`: return `Err(...)` with a clear message that indefinite durations are not yet supported by the finite action lifecycle
- `CombatWeapon`: resolve from combat payload weapon → weapon profile duration, falling back to actor `CombatProfile.unarmed_attack_ticks` for `CombatWeaponRef::Unarmed`

### 6. Update start gate call sites

- `start_gate` must pass the effective payload (`payload_override` if present, otherwise the action definition payload) into `resolve_for()`

### 7. Update test arrays and targeted tests

Add new variants to `ALL_CONSTRAINTS`, `ALL_PRECONDITIONS`, `ALL_DURATION_EXPRS`.

## Files to Touch

- `crates/worldwake-sim/src/action_semantics.rs` (modify)
- `crates/worldwake-sim/src/start_gate.rs` (modify)

## Out of Scope

- Validation logic for new Constraint/Precondition variants in start_gate (E12COMHEA-007)
- Action definitions that use these variants (E12COMHEA-010/011/012/013)
- CombatProfile or DeadAt component definitions (already completed in E12COMHEA-002)
- Wound helper functions (E12COMHEA-006)
- Introducing first-class indefinite action lifecycle support in `ActionInstance`, `tick_action`, or reservation handling
- Making Defend runnable end-to-end; that still depends on a later lifecycle change beyond this ticket's corrected scope

## Acceptance Criteria

### Tests That Must Pass

1. All new Constraint variants round-trip through bincode
2. All new Precondition variants round-trip through bincode
3. `DurationExpr::Indefinite` round-trips through bincode
4. `DurationExpr::CombatWeapon` round-trips through bincode
5. `DurationExpr::Indefinite.fixed_ticks()` returns `None`
6. `DurationExpr::CombatWeapon.fixed_ticks()` returns `None`
7. `DurationExpr::CombatWeapon` resolves unarmed duration from `CombatProfile.unarmed_attack_ticks`
8. `DurationExpr::CombatWeapon` resolves commodity weapon duration from `CommodityKind::spec().combat_weapon_profile.attack_duration_ticks`
9. `DurationExpr::CombatWeapon` fails with a clear error for non-combat payloads or missing required profile data
10. `DurationExpr::Indefinite` fails with a clear error instead of fabricating a finite duration
11. `start_gate` passes the effective payload into duration resolution so payload-driven durations work at action start
12. All new types satisfy `Copy + Clone + Eq + Ord + Hash + Debug + Serialize + Deserialize`
13. Existing suite: `cargo test -p worldwake-sim`

### Invariants

1. All enum variants derive the same trait set as existing variants
2. No `f32`/`f64` anywhere
3. Precondition target indices use `u8`
4. All exhaustive match arms updated
5. No fake finite sentinel is introduced for indefinite durations

## Test Plan

### New/Modified Tests

1. `crates/worldwake-sim/src/action_semantics.rs` — update `ALL_*` arrays, add bincode roundtrip, `fixed_ticks`, payload-aware `CombatWeapon` resolution, and explicit `Indefinite` resolution failure coverage
2. `crates/worldwake-sim/src/start_gate.rs` — add coverage showing payload overrides are used when resolving `DurationExpr::CombatWeapon`

### Commands

1. `cargo test -p worldwake-sim -- action_semantics`
2. `cargo test -p worldwake-sim -- start_gate`
3. `cargo test --workspace`
4. `cargo clippy --workspace`

## Outcome

- Outcome amended: 2026-03-11
- Completion date: 2026-03-11
- What actually changed:
  - corrected the ticket assumptions and scope before implementation so it matched the real action engine, archived E12 prerequisites, and the current `ActionPayload` design
  - added `Constraint::ActorNotIncapacitated` and `Constraint::ActorNotDead`
  - added `Precondition::TargetAlive`, `TargetDead`, and `TargetIsAgent`
  - added `DurationExpr::Indefinite` and `DurationExpr::CombatWeapon`
  - refactored `DurationExpr::resolve_for()` to accept the effective `ActionPayload`, which lets combat duration resolve from the actual chosen weapon instead of inferred state
  - implemented `DurationExpr::CombatWeapon` using `ActionPayload::Combat`, `CombatProfile.unarmed_attack_ticks`, and `CommodityKind::spec().combat_weapon_profile.attack_duration_ticks`
  - kept `DurationExpr::Indefinite` explicit but unresolved, returning a clear error instead of inventing a fake finite duration
  - updated `start_gate` to pass the effective payload into duration resolution
  - extended action-semantics and start-gate tests, including payload-override duration coverage
  - updated exhaustive `Constraint`/`Precondition` match sites in affordance and authoritative validation to compile conservatively until E12COMHEA-007 lands
- Deviations from original plan:
  - rejected the original sentinel-duration idea for `Indefinite`; the current scheduler still only models finite `remaining_ticks`, so fabricating an “infinite” `u32` would have encoded a false lifecycle abstraction
  - strengthened existing duration test coverage beyond the original ticket by fixing the previously non-exhaustive `ALL_DURATION_EXPRS` set, which had omitted `ActorTradeDisposition`
- Verification results:
  - `cargo test -p worldwake-sim -- action_semantics` passed
  - `cargo test -p worldwake-sim -- start_gate` passed
  - `cargo test -p worldwake-sim` passed
  - `cargo test --workspace` passed
  - `cargo clippy --workspace` passed
  - Post-archival refinement: the action runtime now has first-class indefinite duration support via an explicit `ActionDuration` model, so `DurationExpr::Indefinite` resolves successfully and indefinite actions can remain active until completion or cancellation
