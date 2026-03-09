# E06EVELOG-004: EventLog Storage and Core API

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — new `EventLog` struct in `worldwake-sim`
**Deps**: E06EVELOG-003 (EventRecord and StateDelta exist)

## Problem

The append-only event log is the causal source of truth for the simulation. It needs storage, monotonic ID assignment, the `emit` method that enforces append-only semantics, and basic retrieval (`get`, `events_at_tick`).

## Assumption Reassessment (2026-03-09)

1. `EventRecord` exists from E06EVELOG-003 — prerequisite
2. `EventId(pub u64)` exists in `worldwake-core::ids` — confirmed
3. `Tick` exists in `worldwake-core::ids` — confirmed
4. Project convention: `BTreeMap` for ordered indices — confirmed
5. No event log exists yet in `worldwake-sim` — confirmed

## Architecture Check

1. `EventLog` owns a `Vec<EventRecord>` — events are stored in insertion order, which equals ID order since IDs are monotonic
2. `next_id: EventId` tracks the next assignable ID, starting at `EventId(0)`
3. `emit` takes an `EventRecord` and enforces: the record's `event_id` must equal `next_id` (caller sets it from the log's `next_event_id()` accessor)
4. `get(id)` returns `Option<&EventRecord>` via direct index into the Vec (since IDs are gapless and zero-based, `id.0` is the Vec index)
5. `events_at_tick` uses a secondary index — this ticket creates only the tick index; actor/place/tag indices are E06EVELOG-005

## What to Change

### 1. Create `crates/worldwake-sim/src/event_log.rs`

Define `EventLog` struct:
```rust
pub struct EventLog {
    events: Vec<EventRecord>,
    next_id: EventId,
    by_tick: BTreeMap<Tick, Vec<EventId>>,
}
```

Implement:
- `EventLog::new() -> Self` — empty log with `next_id = EventId(0)`
- `next_event_id(&self) -> EventId` — returns the next assignable ID without consuming it
- `emit(&mut self, record: EventRecord) -> Result<EventId, EventLogError>`:
  - Validates `record.event_id == self.next_id`
  - Appends to `events` vec
  - Updates `by_tick` index
  - Increments `next_id`
  - Returns the assigned `EventId`
- `get(&self, id: EventId) -> Option<&EventRecord>` — O(1) lookup by index
- `events_at_tick(&self, tick: Tick) -> &[EventId]` — returns slice from tick index (empty slice if none)
- `len(&self) -> usize`
- `is_empty(&self) -> bool`

### 2. Define `EventLogError` in `crates/worldwake-sim/src/event_log.rs`

```rust
pub enum EventLogError {
    IdMismatch { expected: EventId, got: EventId },
}
```

### 3. Register in `crates/worldwake-sim/src/lib.rs`

Add module and re-export.

## Files to Touch

- `crates/worldwake-sim/src/event_log.rs` (new)
- `crates/worldwake-sim/src/lib.rs` (modify — register module, re-export)

## Out of Scope

- Secondary indices by actor, place, tag (E06EVELOG-005)
- Cause chain traversal (E06EVELOG-008)
- WorldTxn integration (E06EVELOG-006, E06EVELOG-007)
- Completeness verification (E06EVELOG-009)
- Save/load serialization of EventLog (E08)
- Event log hashing (can be added later; deterministic replay already ensures identical sequences)

## Acceptance Criteria

### Tests That Must Pass

1. Fresh `EventLog` has `next_event_id() == EventId(0)` and `is_empty() == true`
2. `emit` succeeds and returns the correct `EventId`
3. After emitting N events, `len() == N` and `next_event_id() == EventId(N)`
4. Event IDs are sequential and gapless: emitting events 0, 1, 2 produces IDs 0, 1, 2
5. `emit` rejects an `EventRecord` whose `event_id` does not match `next_event_id()` (IdMismatch error)
6. `get(EventId(0))` returns the first emitted event
7. `get` returns `None` for IDs beyond the log length
8. `events_at_tick` returns correct event IDs for a given tick
9. `events_at_tick` returns empty slice for ticks with no events
10. Multiple events at the same tick are returned in emission order
11. Append-only: no API exists to modify or remove events after emission
12. `EventLog` satisfies `Clone + Debug + Serialize + Deserialize`
13. `EventLog` survives bincode round-trip with populated data
14. Existing suite: `cargo test --workspace`

### Invariants

1. Event IDs are monotonic and gapless (spec requirement)
2. Event log is append-only — no mutation/deletion API (spec 5.6)
3. `by_tick` index is consistent with stored events

## Test Plan

### New/Modified Tests

1. `crates/worldwake-sim/src/event_log.rs` — fresh log state, sequential emission, ID mismatch rejection, get by ID, tick index queries, multiple events per tick, round-trip serialization

### Commands

1. `cargo test -p worldwake-sim`
2. `cargo clippy --workspace --all-targets -- -D warnings`
3. `cargo test --workspace`
4. `cargo fmt --check`
