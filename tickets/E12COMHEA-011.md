# E12COMHEA-011: Defend action definition + handler

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: Yes — worldwake-sim (action def) + worldwake-systems (handler)
**Deps**: E12COMHEA-005 (DurationExpr::Indefinite, Constraint extensions)

## Problem

The Defend action is an indefinite-duration stance that boosts the agent's effective `guard_skill` by `CombatProfile.defend_bonus`. Hit resolution (E12COMHEA-010) checks whether the target has an active Defend action to apply the bonus.

## Assumption Reassessment (2026-03-11)

1. `DurationExpr::Indefinite` will exist after E12COMHEA-005 — runs until cancelled or interrupted.
2. `Interruptibility::FreelyInterruptible` already exists — confirmed.
3. `ActionPayload::None` is the default — Defend has no special payload.
4. Hit resolution in E12COMHEA-010 will need to check active actions for Defend on the target — this is a read from the scheduler's active action list, state-mediated per Principle 12.
5. Active actions are queryable through `SystemExecutionContext.active_actions`.

## Architecture Check

1. Defend is a "passive" action — no handler effect on start/tick. Its presence in active actions is what matters.
2. The handler can be a no-op or minimal (just maintain the action state). The guard_skill bonus is applied by the hit resolution function in E12COMHEA-010 when it detects an active Defend action on the target.
3. `ActionPayload::None` — no combat-specific payload needed.

## What to Change

### 1. Define Defend ActionDef

- Constraints: `ActorAlive`, `ActorNotDead`, `ActorNotIncapacitated`, `ActorNotInTransit`, `ActorHasControl`
- Targets: none
- Preconditions: `ActorAlive`
- Duration: `DurationExpr::Indefinite`
- Interruptibility: `FreelyInterruptible`
- Payload: `ActionPayload::None`

### 2. Register Defend in ActionDefRegistry

### 3. Implement Defend handler

Minimal handler — Defend's effect is passive (checked by hit resolution). Handler may simply maintain the action without producing events.

### 4. Register handler in ActionHandlerRegistry

## Files to Touch

- `crates/worldwake-sim/src/action_def_registry.rs` (modify — register Defend def)
- `crates/worldwake-systems/src/combat.rs` (modify — Defend handler)
- `crates/worldwake-sim/src/action_handler_registry.rs` (modify — register handler)

## Out of Scope

- Attack action (E12COMHEA-010)
- Hit resolution logic that reads Defend status (E12COMHEA-010 responsibility)
- Loot/Heal actions (E12COMHEA-012/013)
- AI deciding when to defend (E13)
- How Indefinite durations interact with scheduler tick — must be handled in E12COMHEA-005 or E12COMHEA-014

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
