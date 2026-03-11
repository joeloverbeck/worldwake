# E12COMHEA-011: Defend action definition + handler

**Status**: ✅ COMPLETED
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: Yes — worldwake-sim (action def) + worldwake-systems (handler)
**Deps**: E12COMHEA-005 (DurationExpr::Indefinite, Constraint extensions)

## Problem

The Defend action is an indefinite-duration stance that boosts the agent's effective `guard_skill` by `CombatProfile.defend_bonus`. Hit resolution (E12COMHEA-010) checks whether the target has an active Defend action to apply the bonus.

## Assumption Reassessment (2026-03-11)

1. `DurationExpr::Indefinite` exists after E12COMHEA-005, and the action runtime now supports first-class `ActionDuration::Indefinite` instead of a fake countdown.
2. `Interruptibility::FreelyInterruptible` already exists — confirmed.
3. `ActionPayload::None` is the default — Defend has no special payload.
4. Hit resolution in E12COMHEA-010 will need to check active actions for Defend on the target — this is a read from the scheduler's active action list, state-mediated per Principle 12.
5. Active actions are queryable through the scheduler/action execution context.

## Architecture Check

1. Defend is a "passive" action — no handler effect on start/tick. Its presence in active actions is what matters.
2. The handler can be a no-op or minimal (just maintain the action state). The guard_skill bonus is applied by the hit resolution function in E12COMHEA-010 when it detects an active Defend action on the target.
3. `ActionPayload::None` — no combat-specific payload needed.
4. Registration belongs in the combat systems module, which owns combat action definitions and handlers. It does not need a special-case slot in generic sim registries.

## What to Change

### 1. Define Defend ActionDef

- Constraints: `ActorAlive`, `ActorNotDead`, `ActorNotIncapacitated`, `ActorNotInTransit`, `ActorHasControl`
- Targets: none
- Preconditions: `ActorAlive`
- Duration: `DurationExpr::Indefinite`
- Interruptibility: `FreelyInterruptible`
- Payload: `ActionPayload::None`

### 2. Expose Defend registration through the combat systems module

### 3. Implement Defend handler

Minimal handler — Defend's effect is passive (checked by hit resolution). Handler may simply maintain the action without producing events.

### 4. Register the handler alongside the action definition in the combat systems module

## Files to Touch

- `crates/worldwake-systems/src/combat.rs` (modify — Defend action def + handler + registration helper)
- `crates/worldwake-systems/src/lib.rs` (modify — export registration helper)

## Out of Scope

- Attack action (E12COMHEA-010)
- Hit resolution logic that reads Defend status (E12COMHEA-010 responsibility)
- Loot/Heal actions (E12COMHEA-012/013)
- AI deciding when to defend (E13)
- Combat hit resolution that consumes active Defend state (E12COMHEA-010 responsibility)

## Acceptance Criteria

### Tests That Must Pass

1. `DurationExpr::Indefinite` keeps Defend running until cancelled
2. Defend action starts successfully for alive, non-incapacitated agent
3. Defend action is `FreelyInterruptible` — can be cancelled by the agent
4. Defend action rejected for dead agent (constraint `ActorNotDead`)
5. Defend action rejected for incapacitated agent (constraint `ActorNotIncapacitated`)
6. Defend is visible at Place (public visibility)
7. Defend action has `ActionPayload::None`
8. Existing suite: `cargo test --workspace`

### Invariants

1. Defend does not modify any world state on its own — passive action
2. Defend produces no items, consumes no items (conservation intact)
3. Indefinite duration does not cause scheduler errors or infinite loops

## Test Plan

### New/Modified Tests

1. `crates/worldwake-systems/src/combat.rs` — Defend action lifecycle tests

### Commands

1. `cargo test -p worldwake-systems -- combat`
2. `cargo test --workspace && cargo clippy --workspace`

## Outcome

- Outcome amended: 2026-03-11
- Completion date: 2026-03-11
- What actually changed:
  - added `register_defend_action()` in `crates/worldwake-systems/src/combat.rs`
  - defined Defend as an indefinite, freely interruptible, no-target, no-payload action with the intended combat/liveness constraints
  - implemented Defend so it projects `CombatStance::Defending` into authoritative state on start and clears that stance on abort/termination
  - exported the registration helper from `crates/worldwake-systems/src/lib.rs`
  - added lifecycle tests covering affordance availability, indefinite runtime behavior, cancellation, and dead/incapacitated rejection
- Deviations from original plan:
  - registration was implemented as a combat-module helper rather than by hardwiring Defend into generic sim registries, which keeps combat action ownership localized and more extensible
  - the ticket now relies on the first-class indefinite duration lifecycle rather than the earlier placeholder design
  - the original archived outcome described Defend as passive scheduler presence only; after E12COMHEA-010, Defend's authoritative effect carrier is the combat stance component instead
- Verification results:
  - `cargo test -p worldwake-systems -- combat` passed
  - `cargo test --workspace` passed
  - `cargo clippy --workspace` passed
