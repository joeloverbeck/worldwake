# E08TIMSCHREP-004: ControllerState type

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — new type in worldwake-sim
**Deps**: `ControlSource` in worldwake-core, `EntityId` in worldwake-core

## Problem

The simulation needs a serializable scheduler-owned record of which entity, if any, is currently bound to human input. That binding must survive save/load and replay, and `SwitchControl` events need a deterministic landing point that is separate from world mutation. Without a dedicated `ControllerState`, later E08 work has no authoritative place to restore or compare human-control routing.

## Assumption Reassessment (2026-03-09)

1. `ControlSource` enum (`Human | Ai | None`) exists in `worldwake-core::control` — confirmed
2. `EntityId` exists in `worldwake-core::ids` — confirmed
3. No `ControllerState` type exists yet — confirmed
4. `InputKind::SwitchControl { from, to }` already exists in `worldwake-sim::input_event` — confirmed
5. The current codebase still uses per-agent `ControlSource` in world data, and `WorldKnowledgeView::has_control` currently treats both `Human` and `Ai` as "has control" — confirmed
6. No scheduler or tick-step implementation exists yet, so there is currently no code path that reconciles world-level `ControlSource` with scheduler-level control binding
7. Spec 9.12 (player symmetry): simulation code may not branch on "is_player" — `ControllerState` is for input routing only, not simulation legality

## Architecture Check

1. `ControllerState` is a small struct tracking `controlled_entity: Option<EntityId>` — the single human-controlled agent slot
2. `ControllerState` is intentionally passive state. It does not validate world existence, liveness, or mutate agent components because that requires scheduler/world context that belongs in later tickets.
3. It does NOT duplicate per-agent `ControlSource` storage. `ControllerState` answers a different question: "which entity receives human input this tick?"
4. Control switches are requested via `InputKind::SwitchControl`, but this ticket only provides the data type and its local invariants. Tick-loop application remains in E08TIMSCHREP-006.
5. The current architecture has a known mismatch: world-facing code still infers "has control" from `ControlSource`, while E08 introduces a separate human-input binding. This ticket does not resolve that mismatch; it creates the scheduler-side primitive needed to resolve it cleanly later.

## Scope Correction

1. This ticket delivers a value object for human-input binding, not a complete control-transfer system.
2. It does not update `ControlSource`, `WorldKnowledgeView`, affordance logic, or CLI switching behavior.
3. It does not guarantee that `controlled_entity` points to a live or human-labeled entity. That validation belongs where `SwitchControl` is applied with access to `World`.
4. It should stay small and durable so later scheduler/save-load/replay tickets can compose it without rework.

## What to Change

### 1. New type: `ControllerState`

```rust
pub struct ControllerState {
    controlled_entity: Option<EntityId>,
}
```

Derive: `Clone, Debug, Eq, PartialEq, Serialize, Deserialize`.

Implementation notes:
- `controlled_entity` stays private
- accessors should be read-only
- no world references, callbacks, traits, or aliases
- no convenience API that mutates agent `ControlSource`

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
- Resolving the existing `WorldKnowledgeView::has_control` / scheduler-binding split
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
3. `ControllerState` contains no world-derived validation rules; it is serializable scheduler state, not a world policy object

## Architectural Rationale

This change is more beneficial than relying on the current architecture alone. Today, `ControlSource` lives on agents and some code reads it as a generic "has control" flag, but that does not give the scheduler a replayable, saveable answer to the narrower question of which entity is receiving human input right now. A dedicated `ControllerState` keeps human binding explicit, deterministic, and independent from affordance legality.

The cleaner long-term architecture is:
- `ControlSource` remains the per-agent input-source classification used by world/AI-facing code
- `ControllerState` is the sole scheduler-owned human-input binding
- the tick loop becomes the place that keeps those layers coherent when control is switched or lost

That separation is cleaner, more extensible, and easier to reason about than encoding the active human binding indirectly through agent components alone.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-sim/src/controller_state.rs` (inline `#[cfg(test)]`) — construction, switching, mismatched `from`, clearing, serialization

### Commands

1. `cargo test -p worldwake-sim controller_state`
2. `cargo clippy --workspace && cargo test --workspace`

## Outcome

Implemented the planned `ControllerState` and `ControlError` in `worldwake-sim`, with serialization and local switch-validation tests.

Compared to the original ticket wording, the delivered scope was clarified and kept narrower: this work intentionally did not update world-level `ControlSource` handling or resolve the existing `WorldKnowledgeView::has_control` split. That architectural reconciliation remains the responsibility of the later tick-step/control-transfer work.
