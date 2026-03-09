# E06EVELOG-003: StateDelta Wrapper and EventRecord Struct

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — new types in `worldwake-sim`
**Deps**: E06EVELOG-002 (delta types and WitnessData exist)

## Problem

Individual delta types need a unified wrapper (`StateDelta`) so event records can hold a heterogeneous ordered list of mutations. `EventRecord` is the core data structure of the append-only event log — it ties together cause, actor, targets, deltas, visibility, witnesses, and tags into a single immutable record.

## Assumption Reassessment (2026-03-09)

1. `EntityDelta`, `ComponentDelta`, `RelationDelta`, `QuantityDelta`, `ReservationDelta`, `ComponentValue`, and `RelationValue` already exist in `crates/worldwake-sim/src/delta.rs` from archived ticket `archive/tickets/E06EVELOG-002-witness-data-and-delta-types.md` — confirmed
2. `CauseRef`, `EventTag`, and `VisibilitySpec` already exist in `crates/worldwake-sim/src/cause.rs`, `event_tag.rs`, and `visibility.rs` — confirmed
3. `WitnessData` already exists in `crates/worldwake-sim/src/witness.rs` — confirmed
4. `EventId`, `Tick`, and `EntityId` are public `worldwake-core` ids with the required serde and ordering traits — confirmed
5. `BTreeSet` is already the deterministic ordered-set convention used by this codebase and by `WitnessData` — confirmed
6. The current file layout centralizes delta-schema types in `crates/worldwake-sim/src/delta.rs`; creating a separate `state_delta.rs` would duplicate that ownership boundary and fragment the event payload model — corrected scope
7. `EventRecord`, `EventLog`, and `WorldTxn` do not yet exist in `worldwake-sim` — confirmed
8. `cargo test -p worldwake-sim` passes before this ticket, so this change starts from a green foundation — confirmed

## Architecture Check

1. `StateDelta` should live in `crates/worldwake-sim/src/delta.rs` with the concrete delta families it wraps. That keeps the event mutation schema in one place, avoids cross-module aliasing, and makes future schema expansion less brittle.
2. `EventRecord` should be a dedicated higher-level type in `crates/worldwake-sim/src/event_record.rs` because it composes existing event foundation types into the immutable append-only payload consumed by later `EventLog` and `WorldTxn` work.
3. `EventRecord` should expose a canonical constructor that sorts and deduplicates `target_ids` while preserving `state_deltas` order. This encodes the stable-target invariant once instead of relying on every future caller to remember it.
4. `tags` remains `BTreeSet<EventTag>` for deterministic iteration, and `witness_data` remains the previously established deterministic wrapper type.
5. `StateDelta` must preserve the richer typed payloads from E06EVELOG-002 unchanged. It is only a heterogeneous wrapper layer; it must not collapse deltas into strings, ids-only payloads, or any other lossy representation.

## What to Change

### 1. Extend `crates/worldwake-sim/src/delta.rs`

Add `StateDelta` enum alongside the existing delta families:
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

Also add `EventRecord::new(...) -> Self` that canonicalizes `target_ids` into stable sorted, deduplicated order before storing the record.

### 3. Register modules in `crates/worldwake-sim/src/lib.rs`

Add modules and re-export types.

## Files to Touch

- `crates/worldwake-sim/src/delta.rs` (modify — add `StateDelta` and tests)
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
10. `EventRecord::new` stores `target_ids` in stable sorted, deduplicated order
11. Existing suite: `cargo test --workspace`

### Invariants

1. `state_deltas` preserves mutation order within the event (spec: delta order as committed)
2. `tags` uses `BTreeSet` for deterministic iteration (determinism invariant)
3. `event_id` is just data here — monotonicity is enforced by `EventLog` (E06EVELOG-004)
4. `target_ids` canonicalization happens at `EventRecord` construction so later emit/commit paths inherit one stable rule instead of duplicating ad hoc sorting logic

## Test Plan

### New/Modified Tests

1. `crates/worldwake-sim/src/delta.rs` — add `StateDelta` wrapper coverage for variant wrapping, pattern matching, and bincode round-trip
Rationale: verifies the heterogeneous event payload layer preserves the richer E06EVELOG-002 delta schema without introducing a lossy abstraction boundary.
2. `crates/worldwake-sim/src/event_record.rs` — construction, edge cases (empty deltas/targets), target canonicalization, field access, and bincode round-trip with realistic data
Rationale: verifies the event payload shape now exists and that it encodes the stable-target invariant centrally instead of relying on future callers.

### Commands

1. `cargo test -p worldwake-sim`
2. `cargo clippy --workspace --all-targets -- -D warnings`
3. `cargo test --workspace`
4. `cargo fmt --check`

## Outcome

- Completion date: 2026-03-09
- What actually changed:
  - Added `StateDelta` to `crates/worldwake-sim/src/delta.rs` so the heterogeneous event-delta wrapper lives with the concrete delta schema it owns.
  - Added `crates/worldwake-sim/src/event_record.rs` with `EventRecord` and `EventRecord::new`, which canonicalizes `target_ids` into stable sorted, deduplicated order while preserving `state_deltas` order.
  - Registered and re-exported `StateDelta` and `EventRecord` from `crates/worldwake-sim/src/lib.rs`.
  - Added focused unit coverage for `StateDelta` wrapping/round-trip behavior and for `EventRecord` construction, edge cases, canonicalization, and serialization order stability.
- Deviations from original plan:
  - Corrected the ticket before implementation because the original file plan was architecturally stale. `StateDelta` was added to the existing `delta.rs` instead of a new `state_delta.rs` file to keep all event-delta schema in one ownership boundary.
  - Strengthened the original design by adding `EventRecord::new` so the stable `target_ids` invariant is enforced centrally instead of being duplicated across future callers.
- Verification results:
  - `cargo test -p worldwake-sim` passed
  - `cargo fmt --check` passed
  - `cargo clippy --workspace --all-targets -- -D warnings` passed
  - `cargo test --workspace` passed
