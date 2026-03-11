# E12COMHEA-007: Constraint/Precondition validation for new variants in start_gate

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — worldwake-sim start_gate / action_validation
**Deps**: E12COMHEA-005 (new Constraint/Precondition/DurationExpr variants), E12COMHEA-002 (CombatProfile, DeadAt)

## Problem

The new `Constraint` and `Precondition` variants added in E12COMHEA-005 need validation logic in the action start gate so that `ActorNotIncapacitated`, `ActorNotDead`, `TargetAlive`, `TargetDead`, and `TargetIsAgent` are actually enforced when actions attempt to start. Without this, the variants exist but are never checked.

## Assumption Reassessment (2026-03-11)

1. Constraint validation happens in `action_validation.rs` — need to verify exact file and function name.
2. `start_gate.rs` checks preconditions before allowing an action to start — confirmed pattern exists.
3. `ActorNotDead` checks for absence of `DeadAt` component on actor.
4. `ActorNotIncapacitated` checks wound_load < incapacitation_threshold (needs wound helpers from E12COMHEA-006).
5. `TargetAlive(u8)` checks target lacks `DeadAt`.
6. `TargetDead(u8)` checks target has `DeadAt` (used by Loot action).
7. `TargetIsAgent(u8)` checks target's `EntityKind` is `Agent`.

## Architecture Check

1. Follows existing validation patterns exactly — each Constraint/Precondition variant gets a match arm in the validation function.
2. `ActorNotIncapacitated` needs access to `WoundList` and `CombatProfile` — these are read from World.
3. This ticket does NOT implement `DurationExpr::Indefinite` or `CombatWeapon` resolution in the scheduler — that's covered by E12COMHEA-005's `resolve_for()` updates and E12COMHEA-008/014.

## What to Change

### 1. Add Constraint validation arms

In the constraint validation function:
- `ActorNotDead`: check `world.get_component_dead_at(actor).is_none()`
- `ActorNotIncapacitated`: check wound_load < incapacitation_threshold using wound helpers

### 2. Add Precondition validation arms

In the precondition validation function:
- `TargetAlive(idx)`: check target lacks `DeadAt`
- `TargetDead(idx)`: check target has `DeadAt`
- `TargetIsAgent(idx)`: check target's EntityKind == Agent

## Files to Touch

- `crates/worldwake-sim/src/action_validation.rs` (modify)
- `crates/worldwake-sim/src/start_gate.rs` (modify, if separate from action_validation)

## Out of Scope

- Constraint/Precondition enum definitions (E12COMHEA-005)
- Wound helper functions (E12COMHEA-006)
- CombatProfile/DeadAt definitions (E12COMHEA-002)
- Action definitions that use these constraints (E12COMHEA-010/011/012/013)
- Scheduler changes (E12COMHEA-008)
- DurationExpr resolution for Indefinite/CombatWeapon (handled in E12COMHEA-005)

## Acceptance Criteria

### Tests That Must Pass

1. Action with `ActorNotDead` constraint fails to start when actor has `DeadAt`
2. Action with `ActorNotDead` constraint succeeds when actor lacks `DeadAt`
3. Action with `ActorNotIncapacitated` constraint fails when wound_load >= incapacitation_threshold
4. Action with `ActorNotIncapacitated` constraint succeeds when wound_load < incapacitation_threshold
5. Action with `TargetAlive(0)` precondition fails when target has `DeadAt`
6. Action with `TargetAlive(0)` precondition succeeds when target lacks `DeadAt`
7. Action with `TargetDead(0)` precondition fails when target lacks `DeadAt`
8. Action with `TargetDead(0)` precondition succeeds when target has `DeadAt`
9. Action with `TargetIsAgent(0)` precondition fails for non-Agent target
10. Action with `TargetIsAgent(0)` precondition succeeds for Agent target
11. Existing suite: `cargo test -p worldwake-sim`

### Invariants

1. All existing constraint/precondition validations unchanged
2. No false positives: constraints only reject when the condition is actually violated
3. Agents without `CombatProfile` are treated as if they have wound_capacity=0 (or validation fails gracefully)

## Test Plan

### New/Modified Tests

1. `crates/worldwake-sim/src/action_validation.rs` or `start_gate.rs` — targeted tests for each new variant

### Commands

1. `cargo test -p worldwake-sim -- start_gate`
2. `cargo test -p worldwake-sim -- action_validation`
3. `cargo test --workspace && cargo clippy --workspace`
