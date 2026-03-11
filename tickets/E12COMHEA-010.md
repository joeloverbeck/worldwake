# E12COMHEA-010: Attack action definition + handler

**Status**: PENDING
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: Yes — worldwake-sim (action def) + worldwake-systems (handler)
**Deps**: E12COMHEA-001 (CombatWeaponRef, WoundCause::Combat), E12COMHEA-002 (CombatProfile), E12COMHEA-003 (CombatWeaponProfile, Sword/Bow), E12COMHEA-004 (CombatActionPayload), E12COMHEA-005 (Constraint/Precondition/DurationExpr extensions), E12COMHEA-006 (wound helpers)

## Problem

The Attack action is the core combat action. It requires:
1. An `ActionDef` with appropriate constraints, preconditions, and duration
2. A handler that performs hit resolution and appends wounds to the target
3. Registration in the action def registry and handler registry

## Assumption Reassessment (2026-03-11)

1. `ActionDef` struct exists with `actor_constraints`, `targets`, `preconditions`, `duration`, `body_cost_per_tick`, `interruptibility`, `payload`, `handler` — confirmed.
2. `ActionDefRegistry` and `ActionHandlerRegistry` exist for registration — confirmed.
3. Hit resolution must be deterministic given RNG state (Principle 6) — confirmed.
4. Duration comes from weapon profile (`CombatWeaponProfile.attack_duration_ticks`) or `CombatProfile.unarmed_attack_ticks`.
5. Combat uses `DeterministicRng` from worldwake-sim.

## Architecture Check

1. Attack ActionDef:
   - Constraints: `ActorAlive`, `ActorNotDead`, `ActorNotIncapacitated`, `ActorNotInTransit`, `ActorHasControl`
   - Targets: one target at actor's place (Agent)
   - Preconditions: `TargetAtActorPlace(0)`, `TargetAlive(0)`, `TargetIsAgent(0)`
   - Duration: `DurationExpr::CombatWeapon`
   - Interruptibility: `FreelyInterruptible`
   - Payload: `ActionPayload::Combat(CombatActionPayload { target, weapon })`
2. Hit resolution: inputs are attacker's `attack_skill`, target's `guard_skill` (boosted if Defend active), weapon profile, fatigue, wound penalties. Output is one or more `Wound` values.
3. Handler emits combat event visible at the Place.

## What to Change

### 1. Define Attack ActionDef

Create the `ActionDef` for Attack with all constraints, preconditions, targets, and duration as specified.

### 2. Register Attack in ActionDefRegistry

### 3. Implement hit resolution function

Pure function: `(attacker_profile, target_profile, weapon_profile, rng, context) -> Vec<Wound>`

Inputs:
- `CombatProfile.attack_skill`
- Target's `CombatProfile.guard_skill` (+ `defend_bonus` if Defend active)
- Weapon's `CombatWeaponProfile` (or unarmed from `CombatProfile`)
- Fatigue from `HomeostaticNeeds` (state-mediated read)
- Existing wound penalties from `WoundList`
- `DeterministicRng`

Output: `Vec<Wound>` to append to target's `WoundList`.

### 4. Implement Attack action handler

- Read attacker and target state from World
- Call hit resolution
- Append resulting wounds to target's `WoundList` via WorldTxn
- Emit combat event (public at Place, all co-located agents as witnesses)

### 5. Register handler in ActionHandlerRegistry

## Files to Touch

- `crates/worldwake-sim/src/action_def_registry.rs` (modify — register Attack def)
- `crates/worldwake-systems/src/combat.rs` (modify — hit resolution + attack handler)
- `crates/worldwake-sim/src/action_handler_registry.rs` (modify — register handler)

## Out of Scope

- Defend action (E12COMHEA-011)
- Loot action (E12COMHEA-012)
- Heal action (E12COMHEA-013)
- Wound progression (E12COMHEA-009)
- Death detection (E12COMHEA-008)
- Armor/mitigation (explicitly deferred per spec)
- Route combat (explicitly deferred per spec)
- AI deciding when to attack (E13)

## Acceptance Criteria

### Tests That Must Pass

1. Combat resolves deterministically with same RNG state
2. New wounds append to target's `WoundList` with correct `WoundCause::Combat { attacker, weapon }`
3. Wounds have correct `body_part` (randomly selected from RNG)
4. Wound severity derives from weapon profile, not hardcoded
5. Unarmed attacks use `CombatProfile.unarmed_wound_severity` and `unarmed_bleed_rate`
6. Armed attacks (Sword) use `CombatWeaponProfile.base_wound_severity` and `base_bleed_rate`
7. Cannot attack dead agents (precondition `TargetAlive`)
8. Cannot attack if incapacitated (constraint `ActorNotIncapacitated`)
9. Cannot attack if dead (constraint `ActorNotDead`)
10. Cannot attack agents in transit (constraint `ActorNotInTransit`)
11. Target must be co-located at same Place
12. Different `CombatProfile` values produce different outcomes
13. Durations derive from weapon profiles, not hardcoded constants
14. `CombatWeaponRef::Commodity(Sword)` produces different wound profile than `Unarmed`
15. Combat event is emitted and visible at Place
16. Existing suite: `cargo test --workspace`

### Invariants

1. Principle 6: deterministic outcomes given RNG state
2. Principle 11: per-agent profiles, no magic numbers
3. Principle 12: reads HomeostaticNeeds from shared state (state-mediated)
4. No stored health component — wounds only
5. Conservation: no items created or destroyed by attack

## Test Plan

### New/Modified Tests

1. `crates/worldwake-systems/src/combat.rs` — hit resolution unit tests, handler tests

### Commands

1. `cargo test -p worldwake-systems -- combat`
2. `cargo test --workspace && cargo clippy --workspace`
