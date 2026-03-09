# E08TIMSCHREP-003: InputEvent, InputKind, and InputQueue

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — new types in `worldwake-sim`
**Deps**: E07 (action framework — `ActionDefId`, `ActionInstanceId`, `EntityId`), archived E08TIMSCHREP-001 (`SystemId` / `SystemManifest`), archived E08TIMSCHREP-002 (`DeterministicRng`)

## Problem

The scheduler needs a deterministic input queue that orders all external inputs (human and AI action requests, cancellations, control switches) by `(scheduled_tick, sequence_no)`. Without explicit input ordering, same-tick inputs could resolve nondeterministically, breaking replay (Spec 9.2).

## Assumption Reassessment (2026-03-09)

1. `ActionDefId` and `ActionInstanceId` already exist in `crates/worldwake-sim/src/action_ids.rs` and already carry the deterministic trait set this ticket needs — confirmed.
2. `EntityId` and `Tick` already exist in `crates/worldwake-core/src/ids.rs`, and both already derive `Ord`, `Serialize`, and `Deserialize` — confirmed.
3. Archived E08TIMSCHREP-001 already established a closed `SystemId` and immutable `SystemManifest`, and archived E08TIMSCHREP-002 already added a deterministic RNG wrapper. This ticket should stay aligned with that pattern: small, explicit scheduler-owned data structures with deterministic iteration and serialization.
4. No input queue, input event, or controller-binding event surface exists yet in `worldwake-sim` — confirmed by repository search.
5. The current `worldwake-sim` architecture already treats authoritative ordered state as owned collections (`BTreeMap` for active actions, immutable manifest slices for system order). A queue design that leaves `sequence_no` allocation underspecified would be weaker than the surrounding architecture.

## Architecture Check

1. `InputEvent` should stay a flat data record with `scheduled_tick`, `sequence_no`, and `kind`. This is the scheduler/replay log shape the spec already implies.
2. `sequence_no` should be globally monotonic for the lifetime of the queue, not "per tick or globally". A single global counter is simpler to serialize, impossible to reset accidentally, and gives replay a stable total insertion order without inventing extra allocation rules later.
3. `InputQueue` should own `next_sequence_no: u64` plus deterministic per-tick buckets, rather than a loose "BTreeMap or sorted Vec" implementation note. The clean shape here is:

```rust
pub struct InputQueue {
    next_sequence_no: u64,
    events_by_tick: BTreeMap<Tick, Vec<InputEvent>>,
}
```

This preserves cheap per-tick drain/peek operations while keeping storage order deterministic and future scheduler integration straightforward.
4. `InputEvent` should derive `Ord` / `PartialOrd` rather than hand-writing an ordering that ignores `kind`. With a globally unique `sequence_no`, derived ordering is consistent with equality and still yields the required `(tick, sequence_no)` ordering semantics.
5. Inputs remain requests, not world mutations. `InputKind` may describe requested scheduler work, but it must not smuggle state deltas or direct world edits.

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

Derive: `Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize`.

### 2. New type: `InputEvent`

```rust
pub struct InputEvent {
    pub scheduled_tick: Tick,
    pub sequence_no: u64,
    pub kind: InputKind,
}
```

Derive: `Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize`.

Design constraint:
- `sequence_no` is globally unique within a queue instance, so derived ordering is valid and yields the desired deterministic order.

### 3. New type: `InputQueue`

A container that:
- Accepts new `InputEvent` entries
- Assigns monotonically increasing global `sequence_no`
- Drains all events for a given tick in `(tick, sequence_no)` order
- Is fully serializable

Recommended surface:

```rust
pub struct InputQueue {
    next_sequence_no: u64,
    events_by_tick: BTreeMap<Tick, Vec<InputEvent>>,
}

impl InputQueue {
    pub fn new() -> Self;
    pub fn enqueue(&mut self, tick: Tick, kind: InputKind) -> &InputEvent;
    pub fn drain_tick(&mut self, tick: Tick) -> Vec<InputEvent>;
    pub fn peek_tick(&self, tick: Tick) -> &[InputEvent];
    pub fn is_empty(&self) -> bool;
    pub fn next_sequence_no(&self) -> u64;
}
```

Behavior requirements:
- `enqueue()` appends into the bucket for `tick` and returns the queued event so tests can assert exact sequence allocation.
- `peek_tick()` returns an empty slice when the tick has no events.
- `drain_tick()` removes the bucket entirely after returning its events.
- No queue logic may rely on hash-map iteration order or post-hoc sorting to recover determinism.
- `next_sequence_no()` is part of the serialized authoritative state because save/load and replay must preserve future allocation order.

## Files to Touch

- `crates/worldwake-sim/src/input_event.rs` (new)
- `crates/worldwake-sim/src/input_queue.rs` (new)
- `crates/worldwake-sim/src/lib.rs` (modify — add modules + re-exports)

## Out of Scope

- Processing/interpreting input events in the tick loop (that's E08TIMSCHREP-006)
- AI planner generating `RequestAction` inputs (Phase 2, E13)
- CLI/UI capturing human inputs (E21)
- The `ControllerState` type (E08TIMSCHREP-004)
- Replay recording or save/load of the full simulation root (later E08 tickets)

## Acceptance Criteria

### Tests That Must Pass

1. `InputKind` satisfies `Clone + Debug + Eq + Ord + Serialize + Deserialize`
2. `InputEvent` satisfies `Clone + Debug + Eq + Ord + Serialize + Deserialize`
3. `InputEvent` ordering: `(tick=3, seq=0) < (tick=3, seq=1) < (tick=5, seq=0)`
4. `InputQueue::new()` starts empty with `next_sequence_no() == 0`
5. `InputQueue::enqueue` assigns monotonically increasing global `sequence_no`, including when enqueuing multiple events for the same tick and later returning to an earlier tick
6. `InputQueue::peek_tick` returns events for one tick without exposing events from other ticks
7. `InputQueue::drain_tick` returns events in `(tick, sequence_no)` order
8. `InputQueue::drain_tick` removes drained events — second drain returns empty
9. Events for tick N are not returned when draining tick M (M != N)
10. `InputQueue` bincode round-trip preserves queued events, per-tick ordering, and `next_sequence_no`
11. `InputKind` bincode round-trip covers each variant
12. Existing suite: `cargo test -p worldwake-sim`

### Invariants

1. Input ordering is deterministic — no `HashMap` / `HashSet` in queue internals
2. Inputs are requests, not mutations — `InputKind` contains no `StateDelta` or direct world mutation payload
3. `sequence_no` is gap-free and globally monotonic within the queue's lifetime
4. Draining one tick must not perturb ordering or allocation state for other ticks

## Test Plan

### New/Modified Tests

1. `crates/worldwake-sim/src/input_event.rs` — type traits, ordering, and per-variant bincode round-trips
2. `crates/worldwake-sim/src/input_queue.rs` — empty-state behavior, cross-tick sequence allocation, per-tick peek/drain isolation, and serialization preserving future sequence allocation

### Commands

1. `cargo test -p worldwake-sim input_event`
2. `cargo test -p worldwake-sim input_queue`
3. `cargo clippy --workspace --all-targets -- -D warnings`
4. `cargo test --workspace`

## Outcome

- Completed: 2026-03-09
- Changed vs. corrected plan:
  - Added `crates/worldwake-sim/src/input_event.rs` with `InputKind` and `InputEvent` as serializable scheduler records.
  - Added `crates/worldwake-sim/src/input_queue.rs` with a deterministic `InputQueue` storing `next_sequence_no` plus per-tick buckets in `BTreeMap<Tick, Vec<InputEvent>>`.
  - Re-exported `InputKind`, `InputEvent`, and `InputQueue` from `crates/worldwake-sim/src/lib.rs`.
  - Added focused unit coverage for trait bounds, event ordering, per-variant bincode round-trips, cross-tick sequence allocation, per-tick peek/drain isolation, and queue serialization preserving future allocation state.
- Deviations from the original ticket version:
  - Tightened `sequence_no` from “per tick or globally” to globally monotonic only. This is cleaner, simpler to serialize, and better aligned with replay/save-load requirements.
  - Replaced the ticket’s loose storage suggestion (`BTreeMap<(Tick, u64), InputEvent>` or sorted `Vec`) with explicit per-tick buckets plus a global counter. That keeps tick-local drain/peek cheap without leaving ordering recovery to sorting.
  - Derived `Ord` / `PartialOrd` for `InputKind` as well so `InputEvent` can derive a total ordering cleanly and consistently.
- Verification:
  - `cargo test -p worldwake-sim input_event`
  - `cargo test -p worldwake-sim input_queue`
  - `cargo test -p worldwake-sim`
  - `cargo clippy --workspace --all-targets -- -D warnings`
  - `cargo test --workspace`
