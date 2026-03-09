# E06EVELOG-005: EventLog Secondary Indices (Actor, Place, Tag)

## Archive Amendment (2026-03-09)

The final authoritative `EventLog` implementation lives in `worldwake-core`, not `worldwake-sim`. This archived ticket records the intermediate plan before the journal ownership correction.

**Status**: ✅ COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — extends `EventLog` in `worldwake-sim`
**Deps**: E06EVELOG-004 (EventLog with tick index exists)

## Problem

The event log needs secondary indices by actor, place, and tag for efficient querying. These are required by the spec and are essential for cause-chain traversal (E06EVELOG-008), agent perception (E14), and completeness verification (E06EVELOG-009).

## Assumption Reassessment (2026-03-09)

1. `EventLog` already exists from E06EVELOG-004 with append-only `events: Vec<EventRecord>`, `next_id`, and `by_tick: BTreeMap<Tick, Vec<EventId>>` — confirmed in `crates/worldwake-sim/src/event_log.rs`
2. `EventLog::emit(&mut self, pending: PendingEvent) -> EventId` is already the only append surface and the only valid place to maintain secondary indices — confirmed
3. `PendingEvent` / `EventRecord` already carry `actor_id: Option<EntityId>`, `place_id: Option<EntityId>`, and `tags: BTreeSet<EventTag>` — confirmed in `crates/worldwake-sim/src/event_record.rs`
4. Existing `EventLog` tests already live inline in `crates/worldwake-sim/src/event_log.rs`; this ticket should extend that suite rather than introducing a parallel test module
5. `EntityId`, `EventId`, and `Tick` exist in `worldwake-core::ids` — confirmed
6. `EventTag` exists from E06EVELOG-001 and is ordered, so it is safe as a `BTreeMap` key — confirmed
7. Project convention remains `BTreeMap` for deterministic key order and `Vec<EventId>` for emission-order-preserving value storage — confirmed

## Architecture Check

1. Three new indices mirror the existing `by_tick` pattern: `BTreeMap<Key, Vec<EventId>>`
2. The `emit` method is the single point of index maintenance — indices are updated atomically with event insertion
3. Actor and place indices only insert when the respective field is `Some`
4. Tag index inserts one entry per tag in the event's `tags` set
5. All indices use `BTreeMap` for deterministic iteration order
6. Query methods should follow the existing `events_at_tick` API and return borrowed slices backed by internal index storage; this avoids unnecessary allocation and keeps the API consistent

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

## Files to Touch

- `crates/worldwake-sim/src/event_log.rs` (modify — add index fields, update `emit`, add query methods, extend existing tests)

## Out of Scope

- Reverse cause index (E06EVELOG-008)
- Composite queries (e.g. actor + tick range) — not needed yet
- Index compaction or pagination
- EventLog hash stability (separate concern)
- Generalizing indices behind a shared abstraction unless the implementation would otherwise repeat meaningful logic; this ticket should stay small and direct

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

1. `crates/worldwake-sim/src/event_log.rs` — actor index queries, place index queries, tag index queries, `None`-field exclusion, multi-tag indexing, empty-slice behavior for absent keys, and round-trip with populated secondary indices

### Commands

1. `cargo test -p worldwake-sim`
2. `cargo clippy --workspace --all-targets -- -D warnings`
3. `cargo test --workspace`
4. `cargo fmt --check`

## Outcome

- Outcome amended: 2026-03-09
- Completion date: 2026-03-09
- What actually changed: extended `crates/worldwake-core/src/event_log.rs` with deterministic secondary indices for actor, place, and tag; kept `emit(PendingEvent)` as the single append-and-index maintenance path; added borrowed-slice query methods for all three indices; extended the inline `event_log.rs` test suite to cover emission-order preservation, `None` exclusions, multi-tag indexing, empty-key behavior, and bincode round-tripping of populated secondary indices
- Deviations from original plan: corrected the ticket first so it matched the existing `PendingEvent`-based architecture and inline test layout; no broader abstraction layer was introduced because the direct `BTreeMap<Key, Vec<EventId>>` shape is currently the cleanest and most extensible fit for the codebase; the final ownership boundary moved into `worldwake-core`
- Verification results: `cargo test -p worldwake-core`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, `cargo fmt`, and `cargo fmt --check` passed
