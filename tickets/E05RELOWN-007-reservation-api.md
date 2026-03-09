# E05RELOWN-007: Reservation API

**Status**: PENDING
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — new methods on `World`
**Deps**: E05RELOWN-001 (TickRange, ReservationId), E05RELOWN-002 (reservation storage in RelationTables)

## Problem

Facility slots, carts, beds, unique items, and single-use resources need temporary exclusive claims via reservations. The spec requires `try_reserve`, `release_reservation`, and `reservations_for` with deterministic conflict checking and stable reservation IDs.

## Assumption Reassessment (2026-03-09)

1. `RelationTables` has `reservations: BTreeMap<ReservationId, ReservationRecord>`, `reservations_by_entity`, and `next_reservation_id` after E05RELOWN-002 — assumed
2. `TickRange::overlaps()` exists after E05RELOWN-001 — assumed
3. `WorldError::ConflictingReservation { entity }` exists — confirmed
4. `ReservationRecord` stores `id`, `entity`, `reserver`, `range` — assumed from E05RELOWN-002

## Architecture Check

1. `try_reserve` checks all existing reservations for the same entity; if any overlap with the requested `TickRange`, returns `ConflictingReservation`
2. Reservation IDs are monotonically assigned from `next_reservation_id` counter in `RelationTables`
3. `release_reservation` removes by `ReservationId` from both the main table and the entity index
4. All operations are deterministic — no time-based or random elements

## What to Change

### 1. Add reservation methods to `World` in `world.rs`

```rust
pub fn try_reserve(
    &mut self,
    entity: EntityId,
    reserver: EntityId,
    range: TickRange,
) -> Result<ReservationId, WorldError>
```
- Validate both entities alive
- Check all existing reservations for `entity` — if any `TickRange` overlaps, return `ConflictingReservation`
- Assign `next_reservation_id`, increment counter
- Insert `ReservationRecord` into main table and entity index
- Return the new `ReservationId`

```rust
pub fn release_reservation(&mut self, reservation_id: ReservationId) -> Result<(), WorldError>
```
- Look up reservation by ID
- Remove from main table and entity index
- Return error if not found

```rust
pub fn reservations_for(&self, entity: EntityId) -> Vec<ReservationRecord>
```
- Return all active reservations for the entity, sorted by reservation ID

## Files to Touch

- `crates/worldwake-core/src/world.rs` (modify — add reservation methods)

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
7. `release_reservation` returns error for nonexistent reservation ID
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

1. `crates/worldwake-core/src/world.rs` (extend inline `#[cfg(test)]`) — reservation API tests covering happy paths, conflicts, adjacent windows, release, and round-trip

### Commands

1. `cargo test -p worldwake-core world`
2. `cargo clippy --workspace && cargo test --workspace`
