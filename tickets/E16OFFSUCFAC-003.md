# E16OFFSUCFAC-003: Add support_declarations Relation to RelationTables

**Status**: PENDING
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — new relation storage in worldwake-core relations
**Deps**: E16OFFSUCFAC-001

## Problem

E16 needs a `support_declarations` relation to track which agent publicly declares support for which candidate for a given office. This is stored as `(supporter, office) -> candidate` and is a public declaration separate from loyalty. An agent can be loyal to one candidate but publicly declare support for another (e.g., under coercion). This separation enables coerced support and secret loyalty — richer emergence.

The existing `office_holder`, `member_of`, `loyal_to`, `hostile_to` relations already exist and will be reused. Only `support_declarations` is new.

## Assumption Reassessment (2026-03-15)

1. `RelationTables` in `crates/worldwake-core/src/relations.rs` already has `office_holder`, `offices_held`, `member_of`, `members_of`, `loyal_to`, `loyalty_from`, `hostile_to`, `hostility_from` — confirmed, these will be reused not recreated.
2. `RelationTables` stores relations as `BTreeMap` fields — confirmed, deterministic.
3. `WorldTxn` has relation deltas (`RelationDelta`) for staged mutations — confirmed.
4. No `support_declarations` field exists yet — confirmed.

## Architecture Check

1. `support_declarations` is stored as `BTreeMap<(EntityId, EntityId), EntityId>` where key is `(supporter, office)` and value is `candidate`. This allows at most one declaration per agent per office (the latest overwrites).
2. A reverse index `declarations_for_office: BTreeMap<EntityId, BTreeSet<EntityId>>` tracks which agents have declared for each office, enabling efficient vote counting during succession resolution.
3. API methods follow the existing relation patterns: `declare_support()`, `get_support_declaration()`, `clear_declarations_for_office()`, `count_declarations_for_candidate()`.
4. `RelationDelta` needs a new variant for support declaration changes.

## What to Change

### 1. Add `support_declarations` storage to `RelationTables`

In `crates/worldwake-core/src/relations.rs`:

```rust
// (supporter, office) -> candidate
pub support_declarations: BTreeMap<(EntityId, EntityId), EntityId>,
// office -> set of supporters who have declared
pub declarers_for_office: BTreeMap<EntityId, BTreeSet<EntityId>>,
```

### 2. Add support declaration API methods

On `RelationTables` (or `World`/`WorldTxn` as appropriate per existing patterns):

- `declare_support(supporter, office, candidate)` — inserts/overwrites declaration, updates reverse index
- `get_support_declaration(supporter, office) -> Option<EntityId>` — returns candidate if declared
- `declarations_for_office(office) -> impl Iterator<(EntityId, EntityId)>` — returns (supporter, candidate) pairs for an office
- `clear_declarations_for_office(office)` — removes all declarations for an office (used after succession resolves)
- `count_declarations_for_candidate(office, candidate) -> usize` — counts how many supporters declared for a candidate

### 3. Add `RelationDelta` variant for support declarations

Add to the `RelationDelta` enum:

```rust
DeclareSupport { supporter: EntityId, office: EntityId, candidate: EntityId },
ClearDeclarationsForOffice { office: EntityId },
```

### 4. Wire into `WorldTxn` commit

Ensure `WorldTxn` can stage and commit support declaration deltas following the existing relation delta pattern.

### 5. Add `ArchiveDependency` handling

When an office or agent entity is archived, support declarations referencing it must be cleaned up.

## Files to Touch

- `crates/worldwake-core/src/relations.rs` (modify — add storage, API methods, archive cleanup)
- `crates/worldwake-core/src/delta.rs` (modify — add `RelationDelta` variants)
- `crates/worldwake-core/src/world_txn.rs` (modify — commit support declaration deltas)

## Out of Scope

- The `DeclareSupport` action handler that calls these APIs (E16OFFSUCFAC-006)
- The succession system that reads declarations (E16OFFSUCFAC-007)
- Modifying existing relation APIs (`office_holder`, `member_of`, `loyal_to`, `hostile_to`)
- AI planner ops or goal generation
- Event emission

## Acceptance Criteria

### Tests That Must Pass

1. `declare_support(a, office, candidate_x)` stores the declaration and is retrievable.
2. `declare_support(a, office, candidate_y)` overwrites previous declaration for same (supporter, office).
3. `declarations_for_office(office)` returns all (supporter, candidate) pairs.
4. `clear_declarations_for_office(office)` removes all declarations for that office.
5. `count_declarations_for_candidate(office, candidate)` returns correct count.
6. `WorldTxn` stages `DeclareSupport` delta and commits correctly.
7. `WorldTxn` stages `ClearDeclarationsForOffice` delta and commits correctly.
8. Archive dependency cleanup removes stale declarations.
9. `cargo clippy --workspace --all-targets -- -D warnings`
10. `cargo test --workspace`

### Invariants

1. All storage uses `BTreeMap`/`BTreeSet` for determinism.
2. At most one declaration per (supporter, office) pair.
3. Reverse index (`declarers_for_office`) stays consistent with forward map.
4. No existing relations are modified.
5. Save/load roundtrip preserves declarations.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/relations.rs` — unit tests for declare/overwrite/clear/count/archive operations.
2. `crates/worldwake-core/src/world_txn.rs` — test staged support declaration deltas commit correctly.

### Commands

1. `cargo test -p worldwake-core`
2. `cargo clippy --workspace --all-targets -- -D warnings`
3. `cargo test --workspace`
