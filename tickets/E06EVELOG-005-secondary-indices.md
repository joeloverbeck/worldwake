# E06EVELOG-005: EventLog Secondary Indices (Actor, Place, Tag)

**Status**: PENDING
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — extends `EventLog` in `worldwake-sim`
**Deps**: E06EVELOG-004 (EventLog with tick index exists)

## Problem

The event log needs secondary indices by actor, place, and tag for efficient querying. These are required by the spec and are essential for cause-chain traversal (E06EVELOG-008), agent perception (E14), and completeness verification (E06EVELOG-009).

## Assumption Reassessment (2026-03-09)

1. `EventLog` exists from E06EVELOG-004 with `events: Vec<EventRecord>`, `next_id`, and `by_tick` index — prerequisite
2. `EventLog::emit` now accepts a pending event payload and assigns `EventId` internally; index maintenance still happens only inside `emit`
3. `EventRecord` has `actor_id: Option<EntityId>`, `place_id: Option<EntityId>`, `tags: BTreeSet<EventTag>` — prerequisite (E06EVELOG-003)
4. `EntityId` and `EventId` exist in `worldwake-core::ids` — confirmed
5. `EventTag` exists from E06EVELOG-001 — prerequisite
6. Project convention: `BTreeMap` for ordered indices — confirmed

## Architecture Check

1. Three new indices mirror the existing `by_tick` pattern: `BTreeMap<Key, Vec<EventId>>`
2. The `emit` method is the single point of index maintenance — indices are updated atomically with event insertion
3. Actor and place indices only insert when the respective field is `Some`
4. Tag index inserts one entry per tag in the event's `tags` set
5. All indices use `BTreeMap` for deterministic iteration order

## What to Change

### 1. Extend `EventLog` struct in `crates/worldwake-sim/src/event_log.rs`

Add fields:
```rust
by_actor: BTreeMap<EntityId, Vec<EventId>>,
by_place: BTreeMap<EntityId, Vec<EventId>>,
by_tag: BTreeMap<EventTag, Vec<EventId>>,
```

### 2. Update `emit` to maintain new indices

After appending the event and updating `by_tick`:
- If `record.actor_id` is `Some(actor)`, push event ID into `by_actor[actor]`
- If `record.place_id` is `Some(place)`, push event ID into `by_place[place]`
- For each tag in `record.tags`, push event ID into `by_tag[tag]`

### 3. Add query methods

- `events_by_actor(&self, actor: EntityId) -> &[EventId]`
- `events_by_place(&self, place: EntityId) -> &[EventId]`
- `events_by_tag(&self, tag: EventTag) -> &[EventId]`

Each returns an empty slice if no events match.

### 4. Update serialization

Ensure new fields are included in `Serialize`/`Deserialize` derives. Update any existing round-trip tests.

## Files to Touch

- `crates/worldwake-sim/src/event_log.rs` (modify — add index fields, update emit, add query methods)

## Out of Scope

- Reverse cause index (E06EVELOG-008)
- Composite queries (e.g. actor + tick range) — not needed yet
- Index compaction or pagination
- EventLog hash stability (separate concern)

## Acceptance Criteria

### Tests That Must Pass

1. `events_by_actor` returns correct event IDs for events with that actor
2. `events_by_actor` returns empty slice for actors with no events
3. Events with `actor_id: None` do not appear in any actor index entry
4. `events_by_place` returns correct event IDs for events at that place
5. `events_by_place` returns empty slice for places with no events
6. Events with `place_id: None` do not appear in any place index entry
7. `events_by_tag` returns correct event IDs for events with that tag
8. Events with multiple tags appear in each tag's index
9. All indices maintain emission order within each key
10. `EventLog` bincode round-trip preserves all indices
11. Existing E06EVELOG-004 tests still pass
12. Existing suite: `cargo test --workspace`

### Invariants

1. Indices are consistent with stored events (no phantom or missing entries)
2. Index order matches emission order within each key
3. Deterministic iteration: `BTreeMap` keys are always ordered

## Test Plan

### New/Modified Tests

1. `crates/worldwake-sim/src/event_log.rs` — actor index queries, place index queries, tag index queries, None-field exclusion, multi-tag indexing, round-trip with populated indices

### Commands

1. `cargo test -p worldwake-sim`
2. `cargo clippy --workspace --all-targets -- -D warnings`
3. `cargo test --workspace`
4. `cargo fmt --check`
