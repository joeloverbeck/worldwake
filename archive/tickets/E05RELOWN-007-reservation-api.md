# E05RELOWN-007: Reservation API

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — new `World` reservation methods in the existing `world` submodule layout
**Deps**: Archived tickets `E05RELOWN-001` and `E05RELOWN-002` are already implemented on `main`

## Problem

Facility slots, carts, beds, unique items, and single-use resources need temporary exclusive claims via reservations. The spec requires `try_reserve`, `release_reservation`, and `reservations_for` with deterministic conflict checking and stable reservation IDs.

## Assumption Reassessment (2026-03-09)

1. `RelationTables` already has `reservations: BTreeMap<ReservationId, ReservationRecord>`, `reservations_by_entity`, and `next_reservation_id` from archived ticket `E05RELOWN-002` — confirmed
2. `TickRange::overlaps()` already exists from archived ticket `E05RELOWN-001` — confirmed
3. `WorldError::ConflictingReservation { entity }` exists — confirmed
4. `ReservationRecord` already stores `id`, `entity`, `reserver`, and `range` — confirmed
5. `World` relation APIs are already split into `crates/worldwake-core/src/world/*.rs` helpers (`placement.rs`, `ownership.rs`, `relation_mutation.rs`) rather than being implemented directly in the root `world.rs` file — confirmed
6. There is no dedicated `WorldError` variant for a missing reservation id today — confirmed; this ticket must define the failure mode instead of assuming one exists

## Architecture Check

1. The existing architecture already has the right storage shape: reservation rows live in `RelationTables` with a primary table plus a reverse index; the missing piece is a legality API on `World`
2. New reservation behavior should follow the current `World` organization by landing in a dedicated `world` submodule rather than adding more unrelated logic directly into the monolithic `world.rs`
3. `try_reserve` should use the entity index for deterministic conflict checks, but public query results should remain reservation-id ordered and free of aliasing
4. `release_reservation` must remove by `ReservationId` from both the main table and the entity index so relation storage remains internally consistent
5. All operations must stay deterministic and serializable; no time-based or random elements

## Proposed Architecture Rationale

This change is more beneficial than the current architecture.

1. The current architecture has deterministic reservation storage but no authoritative reservation mutation API, so downstream systems would otherwise be pushed toward raw-table mutation and ad hoc conflict checks
2. Adding the API on `World` preserves the intended boundary: legality checks live at the world layer, while `RelationTables` stays focused on storage
3. A dedicated reservation submodule is cleaner and more extensible than growing `world.rs` further; it matches the direction already established by placement and ownership helpers
4. No compatibility aliases or parallel APIs should be introduced. If callers break once the authoritative API exists, they should be updated to use it

## What to Change

### 1. Add reservation methods to `World` in a reservation-focused `world` submodule

```rust
pub fn try_reserve(
    &mut self,
    entity: EntityId,
    reserver: EntityId,
    range: TickRange,
) -> Result<ReservationId, WorldError>
```
- Validate both entities alive
- Check active reservations for `entity` deterministically; if any `TickRange` overlaps, return `ConflictingReservation`
- Assign `next_reservation_id`, increment counter
- Insert `ReservationRecord` into main table and entity index
- Return the new `ReservationId`

```rust
pub fn release_reservation(&mut self, reservation_id: ReservationId) -> Result<(), WorldError>
```
- Look up reservation by ID
- Remove from main table and entity index
- Return `WorldError::InvalidOperation` if the reservation id does not exist

```rust
pub fn reservations_for(&self, entity: EntityId) -> Vec<ReservationRecord>
```
- Return all active reservations for the entity, sorted by reservation ID
- Match existing query-helper ergonomics: for missing or archived entities, return an empty vec rather than an error

## Files to Touch

- `crates/worldwake-core/src/world.rs` (modify — register reservation submodule)
- `crates/worldwake-core/src/world/reservations.rs` (new — reservation API implementation)

## Out of Scope

- Reservation expiry/cleanup (handled by scheduler in E08)
- Reservation-aware action preconditions (E07)
- Event emission for reservation changes (E06)
- Capacity-based reservation (only entity-level exclusivity for now)

## Acceptance Criteria

### Tests That Must Pass

1. `try_reserve` succeeds for an unreserved entity and returns a valid `ReservationId`
2. `try_reserve` succeeds for non-overlapping time windows on the same entity
3. `try_reserve` fails with `ConflictingReservation` for overlapping windows (spec T04)
4. Adjacent windows `[5,10)` and `[10,15)` do NOT conflict (half-open semantics)
5. Reservation IDs are monotonically increasing
6. `release_reservation` removes the reservation; subsequent `try_reserve` for that window succeeds
7. `release_reservation` returns `WorldError::InvalidOperation` for a nonexistent reservation ID
8. `reservations_for` returns empty vec for unreserved entity
9. `reservations_for` returns all active reservations in deterministic order
10. Reservation tables survive bincode round-trip with populated data
11. Existing suite: `cargo test -p worldwake-core`

### Invariants

1. No overlapping reservations for the same entity and time window (spec 9.8)
2. Reservation IDs are stable and monotonic
3. `reservations` and `reservations_by_entity` indices remain consistent

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/world.rs` (extend inline `#[cfg(test)]`) — reservation API tests covering happy paths, conflicts, adjacency, release, deterministic ordering, archived/missing query behavior, and round-trip

### Commands

1. `cargo test -p worldwake-core reservation`
2. `cargo test -p worldwake-core`
3. `cargo clippy --workspace --all-targets -- -D warnings`
4. `cargo test --workspace`
5. `cargo fmt --check`

## Outcome

- Completion date: 2026-03-09
- Actual changes:
  - Added authoritative `World` reservation APIs in `crates/worldwake-core/src/world/reservations.rs`
  - Registered the new reservation submodule from `crates/worldwake-core/src/world.rs`
  - Enforced deterministic overlap checking, monotonic reservation ids, and index cleanup on release
  - Added focused reservation tests for overlap rejection, half-open adjacency, release/re-reserve behavior, archived or missing entity handling, deterministic ordering, and bincode round-trip behavior
- Deviations from original plan:
  - Corrected the ticket first because reservation storage and supporting id/range types were already implemented
  - Scoped the implementation to a dedicated `world` submodule instead of adding methods directly inside the root `world.rs`, which better matches the existing architecture
  - Defined nonexistent reservation release as `WorldError::InvalidOperation` because the codebase does not currently have a dedicated missing-reservation error variant
- Verification results:
  - `cargo test -p worldwake-core reservation`
  - `cargo test -p worldwake-core`
  - `cargo clippy --workspace --all-targets -- -D warnings`
  - `cargo test --workspace`
  - `cargo fmt --check`
