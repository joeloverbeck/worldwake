# S01PROOUTOWNCLA-003: Extend can_exercise_control() for institutional delegation

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — ownership control logic
**Deps**: None (uses existing `factions_of()` and `offices_held_by()`)

## Problem

`can_exercise_control()` only checks direct `actor == owner`. Faction members cannot pick up faction-owned production output, and office holders cannot pick up office-owned output. Institutional delegation is required for the production ownership model to work with factions and offices.

## Assumption Reassessment (2026-03-15)

1. `can_exercise_control()` at `ownership.rs:148-179` checks: container chain → possession → direct ownership (unpossessed) → error — confirmed
2. `factions_of(member)` at `social.rs:60` returns `Vec<EntityId>` of factions — confirmed
3. `offices_held_by(holder)` at `social.rs:205` returns `Vec<EntityId>` of offices — confirmed
4. `owner_of(entity)` at `ownership.rs:7` returns `Option<EntityId>` — confirmed
5. Both `factions_of()` and `offices_held_by()` are methods on `World` — confirmed

## Architecture Check

1. Adding two checks after the direct ownership check is minimal and follows the existing control flow pattern
2. Both checks only apply to unpossessed entities (preserving "possession overrides ownership" invariant)
3. Uses existing relation tables — zero new data structures
4. Faction/office checks are order-independent (both are checked; first success returns Ok)

## What to Change

### 1. Extend `can_exercise_control()` in `ownership.rs`

After the direct ownership check (line 164-168) and before the "possessed by someone else" check, add:

```rust
// Faction membership delegation: actor is member of the owning faction
if let Some(owner) = self.relations.owned_by.get(&entity) {
    if !self.relations.possessed_by.contains_key(&entity) {
        let actor_factions = self.factions_of(actor);
        if actor_factions.contains(owner) {
            return Ok(());
        }
        let actor_offices = self.offices_held_by(actor);
        if actor_offices.contains(owner) {
            return Ok(());
        }
    }
}
```

Note: The exact placement must be after the direct ownership check succeeds/fails but before the final error. The entity must be unpossessed for institutional delegation to apply.

## Files to Touch

- `crates/worldwake-core/src/world/ownership.rs` (modify — extend `can_exercise_control()`)

## Out of Scope

- Adding new relation types or data structures
- Belief-based `can_control()` changes (S01PROOUTOWNCLA-006, -008)
- Pickup validation changes (S01PROOUTOWNCLA-007, -008)
- Contested ownership or contested offices (E16b scope)
- Theft semantics (E17 scope)

## Acceptance Criteria

### Tests That Must Pass

1. `can_exercise_control()` succeeds for faction member on faction-owned, unpossessed entity
2. `can_exercise_control()` succeeds for office holder on office-owned, unpossessed entity
3. `can_exercise_control()` rejects non-member on faction-owned entity
4. `can_exercise_control()` rejects non-holder on office-owned entity
5. `can_exercise_control()` still rejects faction member if entity is possessed by someone else
6. `can_exercise_control()` still succeeds for direct ownership (regression)
7. `can_exercise_control()` still succeeds for possession (regression)
8. Vacant office means no one can exercise control on office-owned entity
9. Existing suite: `cargo test -p worldwake-core`

### Invariants

1. Possession still overrides ownership — if entity is possessed, only possessor controls it
2. Direct ownership still works — no regression
3. Faction membership is checked via existing `factions_of()` relation query
4. Office holding is checked via existing `offices_held_by()` relation query
5. Container chain traversal still applies before ownership checks

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/world/ownership.rs` test module — faction delegation, office delegation, non-member rejection, non-holder rejection, possession override, vacant office

### Commands

1. `cargo test -p worldwake-core can_exercise_control`
2. `cargo test -p worldwake-core`
3. `cargo clippy --workspace`

## Outcome

- **Completion date**: 2026-03-16
- **What changed**:
  - Extended `can_exercise_control()` in `crates/worldwake-core/src/world/ownership.rs` with two new delegation checks for unpossessed entities: faction membership via `factions_of()` and office holding via `offices_held_by()`
  - Added 8 new tests in `crates/worldwake-core/src/world.rs` covering faction delegation, office delegation, non-member/non-holder rejection, possession override, direct ownership/possession regression, and vacant office semantics
- **Deviations from plan**: None
- **Verification**: 670 tests pass in worldwake-core (8 new), clippy clean
