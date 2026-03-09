# E08TIMSCHREP-003: InputEvent, InputKind, and InputQueue

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — new types in worldwake-sim
**Deps**: E07 (action framework — `ActionDefId`, `ActionInstanceId`, `EntityId`)

## Problem

The scheduler needs a deterministic input queue that orders all external inputs (human and AI action requests, cancellations, control switches) by `(scheduled_tick, sequence_no)`. Without explicit input ordering, same-tick inputs could resolve nondeterministically, breaking replay (Spec 9.2).

## Assumption Reassessment (2026-03-09)

1. `ActionDefId` and `ActionInstanceId` exist in `worldwake-sim::action_ids` — confirmed
2. `EntityId` exists in `worldwake-core::ids` — confirmed
3. `Tick` exists in `worldwake-core::ids` with `Ord` — confirmed
4. No input queue or input event types exist yet — confirmed

## Architecture Check

1. `InputEvent` is a flat struct with `(scheduled_tick, sequence_no, kind)` — deterministic ordering by derived `Ord` on `(Tick, u64)`
2. `InputQueue` uses `BTreeMap<(Tick, u64), InputEvent>` or sorted `Vec` — guarantees deterministic drain order
3. Inputs request actions; they never directly mutate world state — this maintains simulation authority (Spec 9.1)

## What to Change

### 1. New type: `InputKind`

```rust
pub enum InputKind {
    RequestAction {
        actor: EntityId,
        def_id: ActionDefId,
        targets: Vec<EntityId>,
    },
    CancelAction {
        actor: EntityId,
        action_instance_id: ActionInstanceId,
    },
    SwitchControl {
        from: Option<EntityId>,
        to: Option<EntityId>,
    },
}
```

Derive: `Clone, Debug, Eq, PartialEq, Serialize, Deserialize`.

### 2. New type: `InputEvent`

```rust
pub struct InputEvent {
    pub scheduled_tick: Tick,
    pub sequence_no: u64,
    pub kind: InputKind,
}
```

Derive: `Clone, Debug, Eq, PartialEq, Serialize, Deserialize`. Implement `Ord` and `PartialOrd` based on `(scheduled_tick, sequence_no)`.

### 3. New type: `InputQueue`

A container that:
- Accepts new `InputEvent` entries
- Assigns monotonically increasing `sequence_no` per tick (or globally)
- Drains all events for a given tick in `(tick, sequence_no)` order
- Is fully serializable

Methods:
- `enqueue(&mut self, tick: Tick, kind: InputKind)` — auto-assigns next `sequence_no`
- `drain_tick(&mut self, tick: Tick) -> Vec<InputEvent>` — returns events for that tick in order, removing them
- `peek_tick(&self, tick: Tick) -> &[InputEvent]` or iterator
- `is_empty(&self) -> bool`

## Files to Touch

- `crates/worldwake-sim/src/input_event.rs` (new)
- `crates/worldwake-sim/src/input_queue.rs` (new)
- `crates/worldwake-sim/src/lib.rs` (modify — add modules + re-exports)

## Out of Scope

- Processing/interpreting input events in the tick loop (that's E08TIMSCHREP-006)
- AI planner generating `RequestAction` inputs (Phase 2, E13)
- CLI/UI capturing human inputs (E21)
- The `ControllerState` type (E08TIMSCHREP-004)

## Acceptance Criteria

### Tests That Must Pass

1. `InputKind` satisfies `Clone + Debug + Eq + Serialize + Deserialize`
2. `InputEvent` satisfies `Clone + Debug + Eq + Ord + Serialize + Deserialize`
3. `InputEvent` ordering: `(tick=3, seq=0) < (tick=3, seq=1) < (tick=5, seq=0)`
4. `InputQueue::enqueue` assigns monotonically increasing `sequence_no`
5. `InputQueue::drain_tick` returns events in `(tick, sequence_no)` order
6. `InputQueue::drain_tick` removes drained events — second drain returns empty
7. Events for tick N are not returned when draining tick M (M ≠ N)
8. `InputQueue` bincode round-trip preserves all queued events and ordering
9. `InputKind` bincode round-trip for each variant
10. Existing suite: `cargo test -p worldwake-sim`

### Invariants

1. Input ordering is deterministic — no `HashMap` in queue internals
2. Inputs are requests, not mutations — `InputKind` contains no `StateDelta`
3. `sequence_no` is gap-free within the queue's lifetime

## Test Plan

### New/Modified Tests

1. `crates/worldwake-sim/src/input_event.rs` — type traits, ordering, bincode
2. `crates/worldwake-sim/src/input_queue.rs` — enqueue, drain, ordering, serialization

### Commands

1. `cargo test -p worldwake-sim input_event`
2. `cargo test -p worldwake-sim input_queue`
3. `cargo clippy --workspace && cargo test --workspace`
