# E08TIMSCHREP-004: ControllerState type

**Status**: PENDING
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — new type in worldwake-sim
**Deps**: `ControlSource` in worldwake-core, `EntityId` in worldwake-core

## Problem

The simulation needs to track which entity is currently human-controlled and manage control-binding changes deterministically. `ControllerState` owns this mapping so that save/load and replay can restore the exact control configuration. Without it, `SwitchControl` input events have nowhere to land, and the controlled-agent mortality invariant (Spec 9.21) cannot be enforced.

## Assumption Reassessment (2026-03-09)

1. `ControlSource` enum (`Human | Ai | None`) exists in `worldwake-core::control` — confirmed
2. `EntityId` exists in `worldwake-core::ids` — confirmed
3. No `ControllerState` type exists yet — confirmed
4. Spec 9.12 (player symmetry): simulation code may not branch on "is_player" — `ControllerState` is for input capture only, not simulation legality

## Architecture Check

1. `ControllerState` is a small struct tracking `controlled_entity: Option<EntityId>` — the single human-controlled agent slot
2. It does NOT store per-agent `ControlSource` — that's already a component in the ECS. `ControllerState` is the scheduler-level binding that says "which entity gets human input this tick"
3. Control switches are applied via `InputKind::SwitchControl`, not direct mutation

## What to Change

### 1. New type: `ControllerState`

```rust
pub struct ControllerState {
    controlled_entity: Option<EntityId>,
}
```

Methods:
- `new() -> Self` — starts with no controlled entity
- `with_entity(entity: EntityId) -> Self`
- `controlled_entity(&self) -> Option<EntityId>`
- `switch_control(&mut self, from: Option<EntityId>, to: Option<EntityId>) -> Result<(), ControlError>` — validates `from` matches current, then sets `to`
- `clear(&mut self)` — sets to `None`

### 2. New type: `ControlError`

Simple enum for control switch failures:
- `MismatchedFrom { expected: Option<EntityId>, actual: Option<EntityId> }`

Derive: `Clone, Debug, Eq, PartialEq`.

## Files to Touch

- `crates/worldwake-sim/src/controller_state.rs` (new)
- `crates/worldwake-sim/src/lib.rs` (modify — add module + re-export)

## Out of Scope

- Processing `SwitchControl` inputs in the tick loop (E08TIMSCHREP-006)
- Updating per-agent `ControlSource` components in the World (E08TIMSCHREP-006 will coordinate)
- CLI/UI for switching control (E21)
- What happens when the controlled entity dies (E08TIMSCHREP-006 enforces Spec 9.21)

## Acceptance Criteria

### Tests That Must Pass

1. `ControllerState` satisfies `Clone + Debug + Eq + PartialEq + Serialize + Deserialize`
2. `ControllerState::new()` starts with `controlled_entity() == None`
3. `ControllerState::with_entity(e)` starts with `controlled_entity() == Some(e)`
4. `switch_control(None, Some(e))` succeeds when currently `None`
5. `switch_control(Some(e), None)` succeeds when currently `Some(e)`
6. `switch_control(Some(wrong), Some(new))` fails with `MismatchedFrom` when current != `wrong`
7. `clear()` sets to `None` unconditionally
8. Bincode round-trip preserves state
9. Existing suite: `cargo test -p worldwake-sim`

### Invariants

1. At most one entity is controlled at any time
2. `ControllerState` is for input routing only — simulation rules never read it for branching (Spec 9.12)

## Test Plan

### New/Modified Tests

1. `crates/worldwake-sim/src/controller_state.rs` (inline `#[cfg(test)]`) — construction, switching, error cases, serialization

### Commands

1. `cargo test -p worldwake-sim controller_state`
2. `cargo clippy --workspace && cargo test --workspace`
