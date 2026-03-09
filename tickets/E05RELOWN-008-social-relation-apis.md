# E05RELOWN-008: Social relation mutation and query APIs

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes — new methods on `World`
**Deps**: E05RELOWN-003 (social relation storage)

## Problem

Social relations (faction membership, loyalty, hostility, office holding, knowledge, belief) need controlled mutation and query APIs. `HoldsOffice` requires uniqueness enforcement (spec 9.13). All social relations need forward+reverse index consistency.

## Assumption Reassessment (2026-03-09)

1. `RelationTables` has all social relation maps after E05RELOWN-003 — assumed
2. `FactId` exists after E05RELOWN-001 — assumed
3. `EntityKind::Faction` and `EntityKind::Office` exist — confirmed
4. `HoldsOffice` is unique per office (spec 9.13) — one holder at a time

## Architecture Check

1. Mutation APIs follow the same pattern as ownership/possession: validate alive, update forward+reverse indices
2. `assign_office(office, holder)` must vacate current holder first (if any) before assigning new one
3. `vacate_office(office)` removes current holder
4. Many-to-many relations use `add_*` / `remove_*` naming (e.g., `add_member`, `remove_member`)
5. Knowledge/belief APIs use `add_known_fact` / `remove_known_fact` / `add_believed_fact` / `remove_believed_fact`

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
pub fn add_loyalty(&mut self, subject: EntityId, target: EntityId) -> Result<(), WorldError>
pub fn remove_loyalty(&mut self, subject: EntityId, target: EntityId) -> Result<(), WorldError>
pub fn loyal_targets_of(&self, subject: EntityId) -> Vec<EntityId>
pub fn loyal_subjects_of(&self, target: EntityId) -> Vec<EntityId>
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

- `crates/worldwake-core/src/world.rs` (modify — add social relation methods)

## Out of Scope

- Physical relation APIs (E05RELOWN-004, -005, -006)
- Reservation API (E05RELOWN-007)
- Belief propagation or rumor mechanics (E14, E15)
- Faction behavior or politics (E16)
- Event emission for social changes (E06)

## Acceptance Criteria

### Tests That Must Pass

1. `add_member` / `remove_member` maintains consistent forward+reverse indices
2. Many-to-many: entity can be member of multiple factions, faction can have multiple members
3. `assign_office` sets holder; second `assign_office` to same office vacates first holder
4. `vacate_office` clears holder; `office_holder` returns `None`
5. `HoldsOffice` enforces at most one holder per office at all times (spec 9.13)
6. An entity can hold multiple offices simultaneously
7. `add_loyalty` / `remove_loyalty` is many-to-many
8. `add_hostility` / `remove_hostility` is many-to-many
9. `add_known_fact` / `add_believed_fact` store `FactId` sets per agent
10. Duplicate `add_*` calls are idempotent (no error, no duplicate entries)
11. `remove_*` on nonexistent relation returns error or is idempotent (design decision: prefer idempotent)
12. All APIs reject archived/nonexistent entities
13. Existing suite: `cargo test -p worldwake-core`

### Invariants

1. One holder per office at all times (spec 9.13)
2. Forward and reverse indices stay consistent for all social relations
3. Social relations support intentional many-to-many except `HoldsOffice`

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/world.rs` (extend inline `#[cfg(test)]`) — social relation API tests covering membership, loyalty, hostility, office uniqueness, knowledge/belief

### Commands

1. `cargo test -p worldwake-core world`
2. `cargo clippy --workspace && cargo test --workspace`
