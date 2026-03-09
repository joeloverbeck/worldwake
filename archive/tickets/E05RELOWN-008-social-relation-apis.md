# E05RELOWN-008: Social relation mutation and query APIs

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes — new methods on `World`
**Deps**: E05RELOWN-003 (social relation storage)

## Problem

Social relations (faction membership, loyalty, hostility, office holding, knowledge, belief) need controlled mutation and query APIs. `HoldsOffice` requires uniqueness enforcement (spec 9.13). All social relations need forward+reverse index consistency.

## Assumption Reassessment (2026-03-09)

1. `RelationTables` already has all required social relation storage after completed ticket `E05RELOWN-003` — confirmed in `crates/worldwake-core/src/relations.rs`
2. `FactId` already exists and is already used by relation storage — confirmed
3. `EntityKind::Faction` and `EntityKind::Office` already exist, and world helpers already validate live entities through `ensure_alive` — confirmed
4. `HoldsOffice` storage uniqueness is already represented correctly as `office -> holder` plus `holder -> offices`; what is missing is the public `World` mutation/query layer — confirmed
5. Archival already depends on these relations via `archive_dependencies` and `prepare_entity_for_archive`, so the new API must preserve those invariants rather than introducing a parallel path — confirmed
6. The E05 spec file is `specs/E05-relations-ownership.corrected.md`, not `specs/E05-identity-social-structure.md`

## Architecture Check

1. Mutation APIs should follow the existing `placement`, `ownership`, and `reservations` module split under `crates/worldwake-core/src/world/`, not expand `world.rs` directly
2. Many-to-many social relations should reuse the same forward/reverse-index mutation discipline already used elsewhere: validate entities, mutate both directions together, and keep removal idempotent
3. `assign_office(office, holder)` should replace the current holder atomically through the existing one-to-many relation helper instead of manual vacate-then-insert logic; this is cleaner and preserves both indices in one path
4. Public query helpers should hide archived entities just like existing placement/ownership queries do
5. Knowledge/belief relations remain agent-scoped fact sets in Phase 1; this ticket should not invent reverse fact indices or E14 memory semantics

## What to Change

### 1. Faction membership APIs

```rust
pub fn add_member(&mut self, member: EntityId, faction: EntityId) -> Result<(), WorldError>
pub fn remove_member(&mut self, member: EntityId, faction: EntityId) -> Result<(), WorldError>
pub fn members_of(&self, faction: EntityId) -> Vec<EntityId>
pub fn factions_of(&self, member: EntityId) -> Vec<EntityId>
```

### 2. Loyalty APIs

```rust
pub fn set_loyalty(&mut self, subject: EntityId, target: EntityId, strength: Permille) -> Result<(), WorldError>
pub fn clear_loyalty(&mut self, subject: EntityId, target: EntityId) -> Result<(), WorldError>
pub fn loyalty_to(&self, subject: EntityId, target: EntityId) -> Option<Permille>
pub fn loyal_targets_of(&self, subject: EntityId) -> Vec<(EntityId, Permille)>
pub fn loyal_subjects_of(&self, target: EntityId) -> Vec<(EntityId, Permille)>
```

### 3. Office APIs (with uniqueness enforcement)

```rust
pub fn assign_office(&mut self, office: EntityId, holder: EntityId) -> Result<(), WorldError>
pub fn vacate_office(&mut self, office: EntityId) -> Result<(), WorldError>
pub fn office_holder(&self, office: EntityId) -> Option<EntityId>
pub fn offices_held_by(&self, holder: EntityId) -> Vec<EntityId>
```

### 4. Hostility APIs

```rust
pub fn add_hostility(&mut self, subject: EntityId, target: EntityId) -> Result<(), WorldError>
pub fn remove_hostility(&mut self, subject: EntityId, target: EntityId) -> Result<(), WorldError>
pub fn hostile_targets_of(&self, subject: EntityId) -> Vec<EntityId>
pub fn hostile_towards(&self, target: EntityId) -> Vec<EntityId>
```

### 5. Knowledge/Belief APIs

```rust
pub fn add_known_fact(&mut self, agent: EntityId, fact: FactId) -> Result<(), WorldError>
pub fn remove_known_fact(&mut self, agent: EntityId, fact: FactId) -> Result<(), WorldError>
pub fn known_facts(&self, agent: EntityId) -> Vec<FactId>
pub fn add_believed_fact(&mut self, agent: EntityId, fact: FactId) -> Result<(), WorldError>
pub fn remove_believed_fact(&mut self, agent: EntityId, fact: FactId) -> Result<(), WorldError>
pub fn believed_facts(&self, agent: EntityId) -> Vec<FactId>
```

## Files to Touch

- `crates/worldwake-core/src/world.rs` (modify module wiring and inline tests)
- `crates/worldwake-core/src/world/relation_mutation.rs` (modify — reusable many-to-many helper for social APIs)
- `crates/worldwake-core/src/world/social.rs` (new — social relation mutation/query APIs)

## Out of Scope

- Physical relation APIs (E05RELOWN-004, -005, -006)
- Reservation API (E05RELOWN-007)
- Belief propagation or rumor mechanics (E14, E15)
- Faction behavior or politics (E16)
- Event emission for social changes (E06)
- Office eligibility rules from E16

## Acceptance Criteria

### Tests That Must Pass

1. `add_member` / `remove_member` maintains consistent forward+reverse indices
2. Many-to-many: entity can be member of multiple factions, faction can have multiple members
3. `assign_office` sets holder; second `assign_office` to same office vacates first holder
4. `vacate_office` clears holder; `office_holder` returns `None`
5. `HoldsOffice` enforces at most one holder per office at all times (spec 9.13)
6. An entity can hold multiple offices simultaneously
7. `set_loyalty` / `clear_loyalty` stores deterministic strength values per `(subject, target)` pair and keeps both directions in sync
8. `add_hostility` / `remove_hostility` is many-to-many
9. `add_known_fact` / `add_believed_fact` store `FactId` sets per agent
10. Duplicate `add_*` calls are idempotent (no error, no duplicate entries)
11. `remove_*` on nonexistent relation is idempotent, matching `clear_owner` / `clear_possessor`
12. All APIs reject archived/nonexistent entities
13. Public query helpers do not surface archived entities even if stale rows are injected in tests
14. Existing suite: `cargo test -p worldwake-core`

### Invariants

1. One holder per office at all times (spec 9.13)
2. Forward and reverse indices stay consistent for all social relations
3. Social relations support intentional many-to-many except `HoldsOffice`, and loyalty is modeled as a weighted relation rather than a plain set

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/world.rs` (extend inline `#[cfg(test)]`) — social relation API tests covering membership, loyalty, hostility, office uniqueness, knowledge/belief, and archived-entity filtering

### Commands

1. `cargo test -p worldwake-core world`
2. `cargo clippy --workspace && cargo test --workspace`

## Outcome

- Completion date: 2026-03-09
- Actual changes:
  - Added a dedicated `crates/worldwake-core/src/world/social.rs` module with public social relation APIs for membership, loyalty, office holding, hostility, and known/believed facts
  - Extended relation mutation helpers with a shared many-to-many insertion path so social APIs update both indices through one deterministic mechanism
  - Refined loyalty from a plain set relation to a weighted `Permille` relation with `set_loyalty` / `clear_loyalty` / `loyalty_to` APIs so future E16 systems can consume loyalty as a first-class score without another storage migration
  - Kept office uniqueness on the existing `office -> holder` authoritative map and exposed it through `assign_office` / `vacate_office` / query helpers instead of adding any alias layer
  - Strengthened `world.rs` tests to cover bidirectional index consistency, office reassignment, archived-entity filtering, idempotent removal, and agent-only fact APIs
- Deviations from original plan:
  - The ticket was corrected first because the original assumptions understated what was already implemented in storage, purge/archive cleanup, and world module layout
  - The implementation touched `world/relation_mutation.rs`, `world/lifecycle.rs`, and a new `world/social.rs` module, not just `world.rs`
  - `assign_office` uses the existing one-to-many relation helper directly rather than a vacate-then-reinsert sequence; this is cleaner and preserves both indices atomically
  - Fact APIs were constrained to live `Agent` entities, which aligns better with the current spec wording than allowing arbitrary entities to accumulate beliefs in Phase 1
  - Loyalty was intentionally upgraded from set membership to weighted storage because the E16 spec already requires loyalty scores; changing the core representation now is cleaner than preserving a set-based API that would need a second migration later
- Verification results:
  - `cargo test -p worldwake-core world`
  - `cargo test -p worldwake-core`
  - `cargo clippy --workspace --all-targets -- -D warnings`
  - `cargo test --workspace`
  - `cargo fmt`
  - `cargo fmt --check`
