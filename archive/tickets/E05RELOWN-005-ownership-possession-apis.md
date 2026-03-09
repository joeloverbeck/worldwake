# E05RELOWN-005: Ownership and possession mutation APIs

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — new ownership/possession methods on `World`
**Deps**: E05RELOWN-002 (RelationTables with physical storage)

## Problem

Ownership and possession are distinct relations (spec 5.5, 9.7). The world needs controlled mutation helpers that maintain forward+reverse index consistency for both. Additionally, `can_exercise_control` is needed to validate whether an actor has legal authority over an entity.

## Assumption Reassessment (2026-03-09)

1. `RelationTables` already has `owned_by`, `property_of`, `possessed_by`, and `possessions_of` maps after E05RELOWN-002 — confirmed
2. Owner/possessor is any `EntityId`, not restricted to agents (spec says factions/offices can own) — confirmed
3. Ownership and possession are both optional — ground items may have neither — confirmed
4. `World::ensure_alive()` validates entity liveness — confirmed
5. There are currently no public `World` ownership/possession mutation APIs; only raw relation storage plus purge coverage exists today — confirmed

## Architecture Check

1. Ownership/possession mutations should follow the existing `world` submodule pattern used by placement APIs instead of adding a second unrelated relation-mutation implementation inline in `world.rs`
2. Forward/reverse index maintenance should reuse a shared internal helper rather than duplicating relation update logic for ownership and possession
3. `clear_owner` and `clear_possessor` should be idempotent for live entities; the relation itself is optional, so clearing an absent relation should succeed
4. `can_exercise_control(actor, entity)` checks: actor possesses entity, OR actor owns entity and entity is currently unpossessed — this is the legality gate for later trade/theft/use actions
5. All targets use `EntityId` — no restriction to agents

## What to Change

### 1. Add ownership/possession methods to `World`

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

Implementation notes:
- Keep the public API on `World`
- Place the implementation in the existing `world` module structure (`world.rs` plus a focused submodule such as `world/ownership.rs` or equivalent)
- If needed, lift the generic forward/reverse relation helpers out of `placement.rs` so both placement and ownership/possession mutations share one internal implementation

## Files to Touch

- `crates/worldwake-core/src/world.rs` (modify — expose the new API and shared helpers if needed)
- `crates/worldwake-core/src/world/*.rs` (modify/add — implement ownership/possession mutation logic in the existing world submodule layout)

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
7. `clear_owner` is a no-op for a live entity with no owner
8. `clear_possessor` is a no-op for a live entity with no possessor
9. Ownership and possession remain independent — setting or clearing one does not mutate the other
10. `can_exercise_control` succeeds when actor possesses entity
11. `can_exercise_control` succeeds when actor owns entity and entity is unpossessed
12. `can_exercise_control` fails when entity is possessed by someone else (even if actor owns it)
13. `can_exercise_control` fails when actor has no relation to entity
14. All methods reject archived/nonexistent entities
15. Existing suite: `cargo test -p worldwake-core`

### Invariants

1. Ownership and possession remain independently queryable at all times (spec 9.7)
2. Forward and reverse indices stay consistent after every mutation
3. `can_exercise_control` never grants control without a valid possession or ownership chain

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/world.rs` (extend inline `#[cfg(test)]`) — ownership/possession mutation tests, idempotent clear coverage, and control validation

### Commands

1. `cargo test -p worldwake-core world`
2. `cargo clippy --workspace && cargo test --workspace`

## Outcome

- Outcome amended: 2026-03-09
- Completed: 2026-03-09
- What changed:
  - Added public `World` APIs for `set_owner`, `clear_owner`, `set_possessor`, `clear_possessor`, and `can_exercise_control`
  - Implemented ownership/possession logic in a dedicated `world` submodule instead of duplicating relation mutation logic inline
  - Extracted the generic forward/reverse relation maintenance helper into its own internal `world` module so placement and ownership/possession updates share one relation-mutation layer without bloating `world.rs`
  - Added coverage for reverse-index replacement, idempotent clears, independence between ownership and possession, control gating, and archived/missing entity failures
- Deviations from original plan:
  - The ticket was corrected before implementation to match the existing `world` submodule architecture rather than forcing all logic directly into `world.rs`
  - `clear_owner` and `clear_possessor` were made explicitly idempotent for live entities because the relations are optional and this yields a cleaner API contract
- Verification results:
  - `cargo fmt --all`
  - `cargo test -p worldwake-core`
  - `cargo clippy --workspace --all-targets --all-features -- -D warnings`
  - `cargo test --workspace`
