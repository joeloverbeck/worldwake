# E06EVELOG-003: StateDelta Wrapper and EventRecord Struct

**Status**: PENDING
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — new types in `worldwake-sim`
**Deps**: E06EVELOG-002 (delta types and WitnessData exist)

## Problem

Individual delta types need a unified wrapper (`StateDelta`) so event records can hold a heterogeneous ordered list of mutations. `EventRecord` is the core data structure of the append-only event log — it ties together cause, actor, targets, deltas, visibility, witnesses, and tags into a single immutable record.

## Assumption Reassessment (2026-03-09)

1. `EntityDelta`, `ComponentDelta`, `RelationDelta`, `QuantityDelta`, `ReservationDelta` exist from E06EVELOG-002 — prerequisite
2. `CauseRef`, `EventTag`, `VisibilitySpec` exist from E06EVELOG-001 — prerequisite
3. `WitnessData` exists from E06EVELOG-002 — prerequisite
4. `EventId`, `Tick`, `EntityId` exist in `worldwake-core::ids` — confirmed
5. `BTreeSet` is the project convention for ordered sets — confirmed
6. E06EVELOG-002 now defines rich typed payloads including `ComponentValue`, `RelationValue`, and full `ReservationRecord`-backed reservation deltas — prerequisite

## Architecture Check

1. `StateDelta` is a flat enum wrapping the five delta families — this preserves delta ordering within an event (spec requirement: "event records preserve delta order as committed by the transaction")
2. `EventRecord` fields match the spec exactly: event_id, tick, cause, actor_id, target_ids, place_id, state_deltas, visibility, witness_data, tags
3. `target_ids` is `Vec<EntityId>` stored in sorted order when ordering is not semantically meaningful (spec requirement)
4. `tags` is `BTreeSet<EventTag>` for deterministic iteration
5. `StateDelta` must preserve the richer typed payloads from E06EVELOG-002 unchanged. It is a wrapper layer only, not a place to collapse deltas into strings or partial identifiers.

## What to Change

### 1. Create `crates/worldwake-sim/src/state_delta.rs`

Define `StateDelta` enum:
- `Entity(EntityDelta)`
- `Component(ComponentDelta)`
- `Relation(RelationDelta)`
- `Quantity(QuantityDelta)`
- `Reservation(ReservationDelta)`

### 2. Create `crates/worldwake-sim/src/event_record.rs`

Define `EventRecord` struct:
```rust
pub struct EventRecord {
    pub event_id: EventId,
    pub tick: Tick,
    pub cause: CauseRef,
    pub actor_id: Option<EntityId>,
    pub target_ids: Vec<EntityId>,
    pub place_id: Option<EntityId>,
    pub state_deltas: Vec<StateDelta>,
    pub visibility: VisibilitySpec,
    pub witness_data: WitnessData,
    pub tags: BTreeSet<EventTag>,
}
```

### 3. Register modules in `crates/worldwake-sim/src/lib.rs`

Add modules and re-export types.

## Files to Touch

- `crates/worldwake-sim/src/state_delta.rs` (new)
- `crates/worldwake-sim/src/event_record.rs` (new)
- `crates/worldwake-sim/src/lib.rs` (modify — register modules, re-export)

## Out of Scope

- EventLog storage and indexing (E06EVELOG-004, E06EVELOG-005)
- WorldTxn that produces EventRecords (E06EVELOG-006, E06EVELOG-007)
- Cause chain traversal (E06EVELOG-008)
- Completeness verification (E06EVELOG-009)
- Hashing or content-addressed event IDs (events use monotonic sequential IDs)

## Acceptance Criteria

### Tests That Must Pass

1. `StateDelta` wraps all five delta families and is pattern-matchable
2. `EventRecord` constructs with all required fields
3. `EventRecord` with empty `state_deltas` is valid (e.g. system tick events)
4. `EventRecord` with empty `target_ids` is valid (e.g. bootstrap events)
5. `EventRecord.tags` maintains deterministic ordering via `BTreeSet`
6. `StateDelta` satisfies `Clone + Eq + Debug + Serialize + Deserialize`
7. `EventRecord` satisfies `Clone + Eq + Debug + Serialize + Deserialize`
8. Both types survive bincode round-trip with populated data, including `ComponentValue`, `RelationValue`, and full `ReservationRecord` payloads
9. `state_deltas` order is preserved through serialization (Vec order stability)
10. Existing suite: `cargo test --workspace`

### Invariants

1. `state_deltas` preserves mutation order within the event (spec: delta order as committed)
2. `tags` uses `BTreeSet` for deterministic iteration (determinism invariant)
3. `event_id` is just data here — monotonicity is enforced by `EventLog` (E06EVELOG-004)

## Test Plan

### New/Modified Tests

1. `crates/worldwake-sim/src/state_delta.rs` — variant wrapping, pattern matching, bincode round-trip
2. `crates/worldwake-sim/src/event_record.rs` — construction, edge cases (empty deltas/targets), field access, bincode round-trip with realistic data

### Commands

1. `cargo test -p worldwake-sim`
2. `cargo clippy --workspace --all-targets -- -D warnings`
3. `cargo test --workspace`
4. `cargo fmt --check`
