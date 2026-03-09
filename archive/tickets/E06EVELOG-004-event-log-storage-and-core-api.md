# E06EVELOG-004: EventLog Storage and Core API

**Status**: ✅ COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — new `EventLog` struct in `worldwake-sim`
**Deps**: E06EVELOG-003 (EventRecord and StateDelta exist)

## Problem

The append-only event log is the causal source of truth for the simulation. It needs storage, monotonic ID assignment, the `emit` method that enforces append-only semantics, and basic retrieval (`get`, `events_at_tick`).

## Assumption Reassessment (2026-03-09)

1. E06EVELOG-003 is already complete and archived; `CauseRef`, `EventTag`, `VisibilitySpec`, `WitnessData`, `StateDelta`, and `EventRecord` already exist in `worldwake-sim`
2. `EventId(pub u64)` exists in `worldwake-core::ids` — confirmed
3. `Tick` exists in `worldwake-core::ids` — confirmed
4. Project convention: `BTreeMap` for ordered indices — confirmed
5. `worldwake-sim` already depends on `serde` and `bincode`, and the existing E06 primitives already round-trip through serialization tests
6. No `event_log` module or `EventLog` type exists yet in `worldwake-sim` — confirmed

## Architecture Check

1. `EventLog` owns a `Vec<EventRecord>` as the single authoritative append-only store; insertion order equals ID order because IDs are monotonic and gapless
2. `next_id: EventId` tracks the next assignable ID, starting at `EventId(0)`
3. `emit` remains the only append surface, but the more robust shape is for it to accept a pending event payload and assign `event_id` internally; callers should not reserve IDs themselves
4. `get(id)` returns `Option<&EventRecord>` via direct index into the Vec (since IDs are gapless and zero-based, `id.0` is the Vec index)
5. `events_at_tick` uses a secondary index; this ticket creates only the tick index, while actor/place/tag indices stay in E06EVELOG-005
6. `EventRecord::new` already normalizes unordered `target_ids`; this ticket must not expand into record-shape changes unless the new log API makes one strictly necessary

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
- `emit(&mut self, pending_event: PendingEvent) -> EventId`:
  - Assigns the next gapless `EventId` internally
  - Appends to `events` vec
  - Updates `by_tick` index
  - Increments `next_id`
  - Returns the assigned `EventId`
- `get(&self, id: EventId) -> Option<&EventRecord>` — O(1) lookup by index
- `events_at_tick(&self, tick: Tick) -> &[EventId]` — returns slice from tick index (empty slice if none)
- `len(&self) -> usize`
- `is_empty(&self) -> bool`

### 2. Define a pending-event payload in `crates/worldwake-sim/src/event_record.rs`

Use a pre-append event payload without `event_id`, then have the log materialize the final `EventRecord` with its assigned ID.

### 3. Register in `crates/worldwake-sim/src/lib.rs`

Add module and re-export.

## Files to Touch

- `crates/worldwake-sim/src/event_log.rs` (new)
- `crates/worldwake-sim/src/lib.rs` (modify — register module, re-export)

## Out of Scope

- Reworking `EventRecord` construction or ownership semantics
- Secondary indices by actor, place, tag (E06EVELOG-005)
- Cause chain traversal (E06EVELOG-008)
- WorldTxn integration (E06EVELOG-006, E06EVELOG-007)
- Completeness verification (E06EVELOG-009)
- Save/load serialization of EventLog (E08)
- Event log hashing (can be added later; deterministic replay already ensures identical sequences)

## Acceptance Criteria

### Tests That Must Pass

1. Fresh `EventLog` is empty and `get(EventId(0)) == None`
2. `emit` succeeds and returns the correct `EventId`
3. After emitting N events, `len() == N`
4. Event IDs are sequential and gapless: emitting three pending events produces stored records with IDs 0, 1, 2
5. `emit` assigns the `event_id` inside the stored `EventRecord`
6. `get(EventId(0))` returns the first emitted event
7. `get` returns `None` for IDs beyond the log length
8. `events_at_tick` returns correct event IDs for a given tick
9. `events_at_tick` returns empty slice for ticks with no events
10. Multiple events at the same tick are returned in emission order
11. Append-only: no API exists to modify or remove events after emission
12. `EventLog` satisfies `Clone + Debug + Serialize + Deserialize`
13. `EventLog` survives bincode round-trip with populated data
14. Existing `EventRecord` tests continue to pass unchanged; this ticket must not duplicate or replace their coverage
15. Existing suite: `cargo test --workspace`

### Invariants

1. Event IDs are monotonic and gapless (spec requirement)
2. Event log is append-only — no mutation/deletion API (spec 5.6)
3. `by_tick` index is consistent with stored events

## Test Plan

### New/Modified Tests

1. `crates/worldwake-sim/src/event_log.rs` — fresh log state, assigned sequential emission, stored ID assignment, get by ID, tick index queries, multiple events per tick, append-only API surface, round-trip serialization

### Commands

1. `cargo test -p worldwake-sim`
2. `cargo clippy --workspace --all-targets -- -D warnings`
3. `cargo test --workspace`
4. `cargo fmt --check`

## Outcome

- Outcome amended: 2026-03-09
- Completion date: 2026-03-09
- What actually changed: added `worldwake_sim::EventLog` with append-only `Vec<EventRecord>` storage, internal gapless `EventId` assignment during append, tick indexing, O(1) lookup by `EventId`, a `PendingEvent` payload type, and public exports from `worldwake-sim`
- Deviations from original plan: corrected the ticket assumptions first because E06EVELOG-003 had already landed, then refined the append API so callers no longer supply event IDs; this is the cleaner long-term shape for `WorldTxn` and future indices
- Verification results: `cargo test -p worldwake-sim`, `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, and `cargo test --workspace` all passed
