# E05RELOWN-002: RelationTables struct with physical relation storage

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — new module `relations.rs`, World struct gains `relations` field
**Deps**: Archived ticket `E05RELOWN-001` is already implemented; no remaining code dependency beyond current `main`

## Problem

The world needs deterministic, typed storage for the five physical relations (LocatedIn, ContainedBy, PossessedBy, OwnedBy, ReservedBy). Without explicit ordered tables and reverse indices, relation queries are impossible and invariants like T01 (unique placement) cannot be enforced.

## Assumption Reassessment (2026-03-09)

1. `World` struct has fields `allocator`, `components`, `topology` — confirmed; `relations` must be added
2. `ComponentTables` uses `BTreeMap<EntityId, T>` pattern — confirmed; relation tables follow the same deterministic pattern
3. `WorldError` already has `ContainmentCycle`, `ConflictingReservation` variants — confirmed
4. `EntityId`, `ReservationId`, `FactId`, and `TickRange` already exist in `ids.rs` — confirmed; this ticket must integrate with those existing types instead of reintroducing or reshaping them
5. `Container` component exists and `World::create_container` currently pairs it with `EntityKind::Container` — confirmed; future legality checks should validate the component, not trust the kind alone
6. `worldwake-core` already exposes a `RelationRecord` marker trait in `traits.rs` — confirmed; relation-side records added here should satisfy that contract
7. `World::purge_entity` currently removes allocator state and component rows only — confirmed; adding relation storage without relation cleanup would leave stale authoritative rows behind

## Architecture Check

1. Relations are NOT components — they are separate ordered tables with forward+reverse indices stored in a dedicated `RelationTables` struct
2. `RelationTables` is a peer to `ComponentTables` inside `World`, not nested inside it
3. All maps use `BTreeMap`/`BTreeSet` for determinism
4. Reservation records include a monotonic `next_reservation_id` counter for stable IDs
5. Storage-only is still the right split for this ticket, but `World` integration must include relation teardown on entity purge so the authoritative world boundary stays internally consistent from the moment the field is introduced

## Proposed Architecture Rationale

This change is more beneficial than the current architecture.

1. The current architecture has no authoritative relation layer, so Phase 1 legality rules from the E05 spec cannot be represented directly and later APIs would be forced either to bolt raw maps onto `World` ad hoc or to smuggle physical state through components that do not model relations cleanly
2. A dedicated `RelationTables` peer to `ComponentTables` keeps the model extensible: physical relations, social relations, and reservation records all share deterministic storage conventions without collapsing into an untyped relation bag
3. The ticket should not introduce aliasing or compatibility shims. If downstream code breaks once `World` gains relation state, that breakage should be fixed in follow-up implementation rather than papered over here
4. The one architectural risk is orphaned relation data during purge. That is small to solve now and expensive to discover later, so cleanup belongs in this ticket's integration scope

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
Also implement `RelationRecord` for `ReservationRecord`.

### 2. Add `relations` field to `World` struct

In `world.rs`, add `relations: RelationTables` to the `World` struct and initialize it in `World::new()`.

### 3. Integrate relation cleanup into `World::purge_entity`

When an entity is purged, remove any relation rows and reverse-index entries involving that entity. Even though mutation APIs land in later tickets, introducing authoritative relation storage without purge cleanup would leave `World` internally inconsistent.

### 4. Register module in `lib.rs`

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
3. `ReservationRecord` satisfies `RelationRecord` and explicit `Eq + PartialEq + Serialize + DeserializeOwned` bounds
4. `World` with `RelationTables` still round-trips through bincode (existing world serialization tests must still pass)
5. `World::purge_entity` removes relation rows involving the purged entity
6. Existing suite: `cargo test -p worldwake-core`

### Invariants

1. No `HashMap` or `HashSet` in any relation table
2. All relation storage is deterministic and serializable
3. `World` cannot retain stale relation rows for purged entities
4. Existing public `World` API remains minimal; this ticket adds storage, not raw public mutation entrypoints

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/relations.rs` (inline `#[cfg(test)]`) — default construction, bincode round-trip, trait bounds
2. `crates/worldwake-core/src/world.rs` — world serialization tests still pass with empty relations
3. `crates/worldwake-core/src/world.rs` — purge cleanup test covering relation teardown for a populated `relations` field

### Commands

1. `cargo test -p worldwake-core relations`
2. `cargo clippy --workspace && cargo test --workspace`

## Outcome

- Completion date: 2026-03-09
- Actual changes:
  - Added `crates/worldwake-core/src/relations.rs` with deterministic physical relation storage and `ReservationRecord`
  - Integrated `RelationTables` into `World` and crate exports
  - Added relation teardown during `World::purge_entity` so the world boundary cannot retain stale authoritative relation rows
  - Added focused relation-storage and purge-cleanup tests
- Deviations from original plan:
  - The ticket assumptions were corrected first because `E05RELOWN-001` was already implemented and the repo already exposed `RelationRecord`
  - `World` integration scope was widened slightly to include purge cleanup; without that, storage-only integration would have introduced stale-state risk
- Verification results:
  - `cargo test -p worldwake-core`
  - `cargo fmt --check`
  - `cargo clippy --workspace --all-targets -- -D warnings`
  - `cargo test --workspace`
