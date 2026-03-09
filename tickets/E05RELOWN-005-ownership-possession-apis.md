# E05RELOWN-005: Ownership and possession mutation APIs

**Status**: PENDING
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — new methods on `World`
**Deps**: E05RELOWN-002 (RelationTables with physical storage)

## Problem

Ownership and possession are distinct relations (spec 5.5, 9.7). The world needs controlled mutation helpers that maintain forward+reverse index consistency for both. Additionally, `can_exercise_control` is needed to validate whether an actor has legal authority over an entity.

## Assumption Reassessment (2026-03-09)

1. `RelationTables` has `owned_by`, `property_of`, `possessed_by`, `possessions_of` maps after E05RELOWN-002 — assumed
2. Owner/possessor is any `EntityId`, not restricted to agents (spec says factions/offices can own) — confirmed
3. Ownership and possession are both optional — ground items may have neither — confirmed
4. `World::ensure_alive()` validates entity liveness — confirmed

## Architecture Check

1. `set_owner` and `set_possessor` are simple forward+reverse index updates with alive checks
2. `clear_owner` and `clear_possessor` remove the relation (for unclaimed/unattended items)
3. `can_exercise_control(actor, entity)` checks: actor possesses entity, OR actor owns entity and entity is unpossessed — this is the legality gate for trade/theft/use actions in later epics
4. All targets use `EntityId` — no restriction to agents

## What to Change

### 1. Add ownership/possession methods to `World` in `world.rs`

```rust
pub fn set_owner(&mut self, entity: EntityId, owner: EntityId) -> Result<(), WorldError>
pub fn clear_owner(&mut self, entity: EntityId) -> Result<(), WorldError>
pub fn set_possessor(&mut self, entity: EntityId, holder: EntityId) -> Result<(), WorldError>
pub fn clear_possessor(&mut self, entity: EntityId) -> Result<(), WorldError>
pub fn can_exercise_control(&self, actor: EntityId, entity: EntityId) -> Result<(), WorldError>
```

`can_exercise_control` returns `Ok(())` if the actor has control, or `Err(WorldError::PreconditionFailed(...))` with a descriptive message if not.

Control logic:
- If actor possesses entity → Ok
- If actor owns entity AND entity has no possessor → Ok
- Otherwise → Err

## Files to Touch

- `crates/worldwake-core/src/world.rs` (modify — add ownership/possession methods)

## Out of Scope

- Placement/movement APIs (E05RELOWN-004)
- Query helpers like `owner_of()`, `possessor_of()` (E05RELOWN-006)
- Trade legality or transfer logic (E11)
- Theft mechanics (E17)
- Event emission for ownership changes (E06)

## Acceptance Criteria

### Tests That Must Pass

1. `set_owner` sets ownership; `owner_of` via raw map returns correct value
2. `set_owner` replaces previous owner; old owner's reverse index is cleared
3. `clear_owner` removes ownership relation
4. `set_possessor` sets possession; reverse index updated
5. `set_possessor` replaces previous possessor
6. `clear_possessor` removes possession relation
7. Ownership and possession are independently queryable — setting one does not affect the other
8. `can_exercise_control` succeeds when actor possesses entity
9. `can_exercise_control` succeeds when actor owns entity and entity is unpossessed
10. `can_exercise_control` fails when entity is possessed by someone else (even if actor owns it)
11. `can_exercise_control` fails when actor has no relation to entity
12. All methods reject archived/nonexistent entities
13. Existing suite: `cargo test -p worldwake-core`

### Invariants

1. Ownership and possession remain independently queryable at all times (spec 9.7)
2. Forward and reverse indices stay consistent after every mutation
3. `can_exercise_control` never grants control without a valid possession or ownership chain

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/world.rs` (extend inline `#[cfg(test)]`) — ownership/possession mutation tests, control validation

### Commands

1. `cargo test -p worldwake-core world`
2. `cargo clippy --workspace && cargo test --workspace`
