# E12COMHEA-010: Attack action definition + handler

**Status**: ✅ COMPLETED
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: Yes — `worldwake-systems` combat action registration/handler, plus small `worldwake-core` / `worldwake-sim` runtime changes to expose active combat stance through authoritative state and make deterministic RNG available to action handlers
**Deps**: `archive/tickets/completed/E12COMHEA-001.md`, `archive/tickets/completed/E12COMHEA-002.md`, `archive/tickets/completed/E12COMHEA-003.md`, `archive/tickets/completed/E12COMHEA-004.md`, `archive/tickets/completed/E12COMHEA-005.md`, `archive/tickets/completed/E12COMHEA-006.md`, `archive/tickets/completed/E12COMHEA-011.md`, `specs/E12-combat-health.md`, `tickets/E12COMHEA-000-index.md`

## Problem

The repo now has the shared wound schema, combat profile data, weapon profiles, combat payloads, wound helpers, Defend registration, wound progression, and death handling. What is still missing is the actual Attack action:

1. an `ActionDef` for Attack
2. a combat hit-resolution path that converts authoritative state into new `Wound` values
3. a handler that appends those wounds and emits a public combat event
4. tests proving deterministic, profile-driven outcomes

## Assumption Reassessment (2026-03-11)

1. This is not a greenfield combat ticket anymore. `crates/worldwake-systems/src/combat.rs` already implements:
   - `combat_system()`
   - wound progression
   - death detection / `DeadAt`
   - `register_defend_action()`
2. The original dependency references were stale. `E12COMHEA-001` through `E12COMHEA-006` and `E12COMHEA-011` are already completed and live under `archive/tickets/completed/`.
3. Action registration in this codebase is owned by the systems module that defines the action. Current patterns are:
   - `register_trade_action()` in `crates/worldwake-systems/src/trade_actions.rs`
   - `register_harvest_actions()` / `register_craft_actions()` in `crates/worldwake-systems/src/production_actions.rs`
   - `register_defend_action()` in `crates/worldwake-systems/src/combat.rs`
   This ticket should follow that pattern instead of editing generic sim registries directly.
4. `ActionDefRegistry` and `ActionHandlerRegistry` do exist in `worldwake-sim`, but there is no global central registration file for concrete actions. The original file list naming `crates/worldwake-sim/src/action_def_registry.rs` and `crates/worldwake-sim/src/action_handler_registry.rs` as edit targets was incorrect.
5. `DurationExpr::CombatWeapon`, `Constraint::ActorNotIncapacitated`, `Constraint::ActorNotDead`, `Precondition::TargetAlive`, and `Precondition::TargetIsAgent` already exist and are tested in `worldwake-sim`.
6. `CombatActionPayload { target, weapon }` already exists and `DurationExpr::CombatWeapon` already resolves from it.
7. Weapon duration/severity data already exists in `CommodityKind::spec().combat_weapon_profile` for `Sword` and `Bow`, and unarmed fallbacks already exist on `CombatProfile`.
8. A basic architectural mismatch remains: Attack hit resolution is supposed to check whether the target is actively Defending, but current action handlers only receive `&ActionDef`, `&ActionInstance`, and `&mut WorldTxn`. They do not receive scheduler `active_actions`, so Attack cannot currently inspect Defend's presence-in-active-actions design from `E12COMHEA-011`.
9. A second architectural mismatch also exists: Attack hit resolution is supposed to be deterministic but body-part / hit variation is RNG-driven, and current generic action handlers have no access to `DeterministicRng`.
10. Because of items 8 and 9, the original ticket overstated what can be implemented cleanly without:
   - exposing Defend's effective state through authoritative/shared state, and
   - threading deterministic RNG through the generic action runtime instead of inventing combat-only randomness plumbing
11. `crates/worldwake-systems/src/lib.rs` currently exports only `register_defend_action()` from the combat module. If Attack lands as a peer combat action, the export surface likely changes too.

## Architecture Check

### What Is Better Than The Current Architecture

1. Attack registration belongs in `crates/worldwake-systems/src/combat.rs`, next to Defend and the combat system. That keeps combat action ownership localized and matches the current architecture.
2. Hit resolution should remain profile-driven and wound-driven. It should derive wound severity/bleed from `CombatProfile`, `CombatWeaponProfile`, fatigue, and existing wound load rather than introducing health bars, damage numbers, or magic constants.
3. Wounds should continue to be appended directly to `WoundList`. That is still the cleanest carrier for downstream progression, incapacitation, death, healing, and evidence.

### Current Architectural Gap

1. Defend's current effect is represented implicitly by the action remaining active in the scheduler.
2. Attack's handler cannot read scheduler active actions, only authoritative world state via `WorldTxn`.
3. That means the current Defend representation is not consumable by Attack in a state-mediated way.
4. The spec intent is still correct, but the current implementation seam is not.

### Recommended Direction

1. Expose the effective Defend state through authoritative/shared state instead of requiring Attack to inspect scheduler internals. That is cleaner, easier to test, and more aligned with Principle 12 than widening every action handler to read the scheduler.
2. Extend the generic action runtime so handlers can consume `DeterministicRng` directly. Randomness in actions is not combat-specific, so the runtime seam should be fixed generically once instead of papered over locally.

## Revised Scope

This ticket now covers two layers:

1. implement Attack itself
2. add a first-class combat-stance representation so Defend is readable from authoritative state
3. add deterministic RNG access to generic action handlers so Attack can resolve hits cleanly

Concretely:

1. Define and register an Attack `ActionDef` in the combat module.
2. Implement deterministic hit resolution for armed and unarmed attacks.
3. Implement the Attack handler and combat-event emission.
4. Add or strengthen tests for affordance/start-gate behavior, deterministic wound application, weapon-profile-driven outcomes, and public event emission.
5. Add a first-class combat stance component in `worldwake-core` and make Defend own its lifecycle explicitly.
6. Thread deterministic RNG through the action runtime (`start_action` / `tick_action` / abort path / scheduler wiring) so handlers can consume randomness without special cases.

## What to Change

### 1. Define Attack ActionDef

Attack should be registered from the combat module, following the existing `register_defend_action()` pattern.

Target/action shape:

- Constraints:
  - `ActorAlive`
  - `ActorNotDead`
  - `ActorNotIncapacitated`
  - `ActorNotInTransit`
  - `ActorHasControl`
- Targets:
  - one `EntityAtActorPlace { kind: EntityKind::Agent }`
- Preconditions:
  - `ActorAlive`
  - `TargetExists(0)`
  - `TargetAtActorPlace(0)`
  - `TargetAlive(0)`
  - `TargetIsAgent(0)`
- Duration:
  - `DurationExpr::CombatWeapon`
- Interruptibility:
  - `FreelyInterruptible`
- Payload:
  - `ActionPayload::Combat(CombatActionPayload { target, weapon })`
- Visibility:
  - `VisibilitySpec::SamePlace`

### 2. Register Attack through the combat systems module

Add a combat-module helper analogous to `register_defend_action()`, and export it from `crates/worldwake-systems/src/lib.rs` if downstream tests/integration harnesses need it.

### 3. Implement hit resolution

Implement a deterministic function that derives one new wound per resolved hit from:

- attacker's `CombatProfile.attack_skill`
- target's effective guard skill
- weapon profile (`CombatWeaponProfile`) or unarmed profile (`CombatProfile`)
- target and/or attacker fatigue from `HomeostaticNeeds`
- existing wound load / wound penalties from `WoundList`
- `DeterministicRng`

The current ticket should not invent health or damage abstractions. Output remains concrete `Wound` values.

### 4. Implement Attack handler

The handler should:

1. validate payload/target consistency
2. read attacker/target authoritative state
3. resolve the wound outcome deterministically
4. append new wounds to target `WoundList`
5. emit a public combat event at the place of combat
6. record `WoundCause::Combat { attacker, weapon }`

### 5. Add a first-class combat stance component

Add an authoritative combat-state component in `worldwake-core` for active stance/effect state. For this ticket, it at least needs to represent `Defending`.

Defend should:

1. set the stance on action start
2. keep it while active
3. clear it on abort/termination

Attack should then read the stance from `WorldTxn`/world state when computing effective guard skill.

### 6. Add deterministic RNG access to action handlers

Extend the generic action runtime so action handlers can consume `DeterministicRng` directly.

This should be threaded through:

1. action handler function signatures
2. `ActionExecutionAuthority` / scheduler action runtime
3. `start_action`
4. `tick_action`
5. abort/failure termination paths
6. tick-step scheduler wiring

This is the clean architecture because randomness is a general action-runtime concern, not an Attack special case.

## Files to Touch

- `crates/worldwake-core/src/combat.rs` (modify — add combat stance type and tests)
- `crates/worldwake-core/src/component_schema.rs` (modify — register combat stance component)
- `crates/worldwake-core/src/lib.rs` (modify — export combat stance type)
- `crates/worldwake-sim/src/action_handler.rs` (modify — RNG-aware handler signatures)
- `crates/worldwake-sim/src/action_execution.rs` (modify — thread RNG through action execution authority)
- `crates/worldwake-sim/src/start_gate.rs` (modify — RNG-aware start path)
- `crates/worldwake-sim/src/tick_action.rs` (modify — RNG-aware tick/commit path)
- `crates/worldwake-sim/src/interrupt_abort.rs` and/or `crates/worldwake-sim/src/action_termination.rs` (modify — RNG-aware abort path)
- `crates/worldwake-sim/src/scheduler.rs` (modify — pass RNG into action runtime)
- `crates/worldwake-sim/src/tick_step.rs` (modify — supply RNG to action runtime calls)
- `crates/worldwake-systems/src/combat.rs` (modify — Defend stance lifecycle, Attack definition, Attack handler, tests)
- `crates/worldwake-systems/src/lib.rs` (modify if Attack registration helper is exported)

## Out of Scope

- Loot action (`E12COMHEA-012`)
- Heal action (`E12COMHEA-013`)
- AI deciding when to attack (`E13`)
- Route/ranged spatial combat beyond same-place targeting
- armor/mitigation systems
- any backward-compatibility shim between old and new combat state exposure

## Acceptance Criteria

### Tests That Must Pass

1. Attack action definition is registered from the combat module and has the intended combat constraints/preconditions.
2. Attack duration resolves from weapon profile or `CombatProfile.unarmed_attack_ticks`, not hardcoded constants.
3. Attack deterministically produces the same wound result for the same seed/state.
4. New wounds append to target `WoundList` with `WoundCause::Combat { attacker, weapon }`.
5. Wound severity and bleed rate derive from weapon/unarmed profiles, not hardcoded constants.
6. `CombatWeaponRef::Commodity(Sword)` and `CombatWeaponRef::Unarmed` produce distinct wound profiles.
7. Dead, incapacitated, in-transit, or non-co-located attacks are rejected by the action framework.
8. Combat event emission is public at the place of combat.
9. Defend projects its active effect through authoritative state rather than requiring scheduler introspection.
10. Attack reads that stance state and applies the intended guard bonus.
11. Generic action handlers can consume deterministic RNG from the runtime without any combat-specific backdoor.
12. Relevant targeted suites pass.
13. `cargo test --workspace` passes.
14. `cargo clippy --workspace --all-targets` passes.

### Invariants

1. Deterministic outcomes given identical seed and state.
2. No stored health component or damage alias is introduced.
3. No magic-number combat timing or severity constants outside per-agent/per-weapon profiles.
4. No compatibility wrapper or alias path is introduced for old combat-state exposure.
5. Attack reads shared/authoritative state directly; it does not introduce cross-system calls.
6. Randomness is provided through the generic action runtime, not through a combat-only side channel.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-systems/src/combat.rs`
   - Attack action definition/registration coverage
   - Defend stance lifecycle coverage
   - Attack start-gate rejection coverage
   - deterministic hit-resolution coverage
   - weapon/unarmed wound-profile coverage
   - event emission coverage
   - Defend interaction coverage through authoritative stance state
2. `crates/worldwake-sim/src/start_gate.rs`, `crates/worldwake-sim/src/tick_action.rs`, and abort-path tests
   - action runtime still behaves correctly after RNG threading

### Commands

1. `cargo test -p worldwake-systems -- combat`
2. `cargo test -p worldwake-sim -- action_semantics`
3. `cargo test -p worldwake-sim -- start_gate`
4. `cargo test --workspace`
5. `cargo clippy --workspace --all-targets`

## Outcome

- Completion date: 2026-03-11
- What actually changed:
  - added first-class `CombatStance::Defending` in `worldwake-core` and registered it through authoritative component schema / txn mutation surfaces
  - updated Defend so its effect is explicit authoritative stance state, set on start and cleared on abort/termination
  - threaded `DeterministicRng` through generic action handlers, execution authority, scheduler runtime, and abort/termination paths in `worldwake-sim`
  - implemented Attack registration, deterministic hit resolution, and wound application in `crates/worldwake-systems/src/combat.rs`
  - added a full Attack lifecycle test covering payload-driven weapon duration, wound append, and public combat event emission
  - updated test harnesses and schema assertions across systems/sim to match the new runtime and component surface
- Deviations from original plan:
  - hit resolution was refactored into typed attacker/target/context inputs instead of a long scalar argument list so the combat boundary stays cleaner under clippy and future extension
  - Defend no longer relies on scheduler active-action presence as its authoritative effect carrier; stance is now world state, which is cleaner and more extensible for future combat actions
  - RNG plumbing became a generic action-runtime concern rather than a combat-local special case
- Verification results:
  - `cargo test -p worldwake-systems combat -- --nocapture` passed
  - `cargo test -p worldwake-sim -- start_gate --nocapture` passed
  - `cargo test -p worldwake-systems --test e09_needs_integration -- --nocapture` passed
  - `cargo test --workspace` passed
  - `cargo clippy --workspace` passed
  - `cargo clippy --workspace --all-targets` passed
