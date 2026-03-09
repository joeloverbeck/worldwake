# E05RELOWN-002: RelationTables struct with physical relation storage

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — new module `relations.rs`, World struct gains `relations` field
**Deps**: E05RELOWN-001 (ReservationId, FactId, TickRange)

## Problem

The world needs deterministic, typed storage for the five physical relations (LocatedIn, ContainedBy, PossessedBy, OwnedBy, ReservedBy). Without explicit ordered tables and reverse indices, relation queries are impossible and invariants like T01 (unique placement) cannot be enforced.

## Assumption Reassessment (2026-03-09)

1. `World` struct has fields `allocator`, `components`, `topology` — confirmed; `relations` must be added
2. `ComponentTables` uses `BTreeMap<EntityId, T>` pattern — confirmed; relation tables follow the same deterministic pattern
3. `WorldError` already has `ContainmentCycle`, `ConflictingReservation` variants — confirmed
4. `EntityId`, `ReservationId`, `TickRange` will exist after E05RELOWN-001
5. `Container` component exists on `EntityKind::Container` entities — confirmed

## Architecture Check

1. Relations are NOT components — they are separate ordered tables with forward+reverse indices stored in a dedicated `RelationTables` struct
2. `RelationTables` is a peer to `ComponentTables` inside `World`, not nested inside it
3. All maps use `BTreeMap`/`BTreeSet` for determinism
4. Reservation records include a monotonic `next_reservation_id` counter for stable IDs

## What to Change

### 1. Create `crates/worldwake-core/src/relations.rs`

Define `ReservationRecord`:
```rust
pub struct ReservationRecord {
    pub id: ReservationId,
    pub entity: EntityId,
    pub reserver: EntityId,
    pub range: TickRange,
}
```

Define `RelationTables`:
```rust
pub struct RelationTables {
    // Physical placement
    pub(crate) located_in: BTreeMap<EntityId, EntityId>,        // entity → place
    pub(crate) entities_at: BTreeMap<EntityId, BTreeSet<EntityId>>, // place → entities (reverse)

    // Containment
    pub(crate) contained_by: BTreeMap<EntityId, EntityId>,      // entity → container
    pub(crate) contents_of: BTreeMap<EntityId, BTreeSet<EntityId>>, // container → entities (reverse)

    // Possession
    pub(crate) possessed_by: BTreeMap<EntityId, EntityId>,      // entity → holder
    pub(crate) possessions_of: BTreeMap<EntityId, BTreeSet<EntityId>>, // holder → entities (reverse)

    // Ownership
    pub(crate) owned_by: BTreeMap<EntityId, EntityId>,          // entity → owner
    pub(crate) property_of: BTreeMap<EntityId, BTreeSet<EntityId>>, // owner → entities (reverse)

    // Reservations
    pub(crate) reservations: BTreeMap<ReservationId, ReservationRecord>,
    pub(crate) reservations_by_entity: BTreeMap<EntityId, BTreeSet<ReservationId>>,
    pub(crate) next_reservation_id: u64,
}
```

All fields derive `Clone, Debug, Eq, PartialEq, Serialize, Deserialize`. Provide `Default` impl.

### 2. Add `relations` field to `World` struct

In `world.rs`, add `relations: RelationTables` to the `World` struct and initialize it in `World::new()`.

### 3. Register module in `lib.rs`

Add `pub mod relations;` and re-export `RelationTables` and `ReservationRecord`.

## Files to Touch

- `crates/worldwake-core/src/relations.rs` (new)
- `crates/worldwake-core/src/world.rs` (modify — add `relations` field)
- `crates/worldwake-core/src/lib.rs` (modify — add module + re-exports)

## Out of Scope

- Social relations (E05RELOWN-003)
- Placement/movement mutation APIs (E05RELOWN-004)
- Ownership/possession mutation APIs (E05RELOWN-005)
- Query helpers (E05RELOWN-006)
- Reservation API logic (E05RELOWN-007)
- Any invariant enforcement logic — this ticket is storage only

## Acceptance Criteria

### Tests That Must Pass

1. `RelationTables::default()` creates empty tables
2. `RelationTables` round-trips through bincode
3. `ReservationRecord` satisfies `Clone + Debug + Eq + Serialize + DeserializeOwned`
4. `World` with `RelationTables` still round-trips through bincode (existing world serialization tests must still pass)
5. Existing suite: `cargo test -p worldwake-core`

### Invariants

1. No `HashMap` or `HashSet` in any relation table
2. All relation storage is deterministic and serializable
3. Existing `World` API and tests remain unchanged

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/relations.rs` (inline `#[cfg(test)]`) — default construction, bincode round-trip, trait bounds
2. `crates/worldwake-core/src/world.rs` — existing world tests must still pass (world now includes empty relations)

### Commands

1. `cargo test -p worldwake-core relations`
2. `cargo clippy --workspace && cargo test --workspace`
