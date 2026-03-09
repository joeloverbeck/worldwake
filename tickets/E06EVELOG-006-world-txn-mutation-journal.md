# E06EVELOG-006: WorldTxn Mutation Journal

**Status**: PENDING
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: Yes — new `WorldTxn` struct in `worldwake-sim` that wraps `&mut World`
**Deps**: E06EVELOG-003 (StateDelta, delta types exist), E06EVELOG-004 (EventLog exists)

## Problem

The spec requires that "all persistent world writes in normal simulation code go through `WorldTxn`" and that "no public API may mutate authoritative state without either emitting an event or being explicitly marked as transient/cache-only." `WorldTxn` is the mutation journal that captures before/after deltas as mutations occur, then commits them as a single `EventRecord`.

## Assumption Reassessment (2026-03-09)

1. `World` struct exists in `worldwake-core::world` with mutation methods for components, relations, placement, ownership, reservations — confirmed
2. `StateDelta`, `EntityDelta`, `ComponentDelta`, `RelationDelta`, `QuantityDelta`, `ReservationDelta` exist from E06EVELOG-002/003 — prerequisite
3. `CauseRef`, `EventTag`, `VisibilitySpec`, `WitnessData` exist from E06EVELOG-001/002 — prerequisite
4. `EventLog` exists from E06EVELOG-004 — prerequisite
5. `ComponentKind` and `RelationKind` exist from E06EVELOG-002 — prerequisite
6. `World` mutation methods are `pub` and directly mutate component/relation tables — confirmed

## Architecture Check

1. `WorldTxn` borrows `&mut World` and `&mut EventLog`, accumulates `Vec<StateDelta>` as mutations occur, then commits by appending one `EventRecord`
2. `WorldTxn` exposes journaled wrappers for the subset of `World` mutations needed by simulation code — initially: entity creation/archival, component set/remove, placement, quantity changes, reservation create/release
3. The journal does NOT replace `World` mutation methods; it wraps them. `World` methods remain for bootstrap/test scenarios explicitly marked as non-journaled
4. This ticket implements the journal accumulation and the journaled mutation wrappers. The commit-to-EventRecord flow is E06EVELOG-007
5. Relation mutations (ownership, social) get journaled wrappers as needed — start with placement and quantities since those are most critical for conservation auditability

## What to Change

### 1. Create `crates/worldwake-sim/src/world_txn.rs`

Define `WorldTxn`:
```rust
pub struct WorldTxn<'w> {
    world: &'w mut World,
    event_log: &'w mut EventLog,
    tick: Tick,
    cause: CauseRef,
    actor_id: Option<EntityId>,
    place_id: Option<EntityId>,
    tags: BTreeSet<EventTag>,
    target_ids: Vec<EntityId>,
    visibility: VisibilitySpec,
    witness_data: WitnessData,
    deltas: Vec<StateDelta>,
    committed: bool,
}
```

### 2. Implement journaled mutation helpers

Each wrapper calls the underlying `World` method, then pushes a `StateDelta` to `self.deltas`:

- `create_entity(kind, place) -> Result<EntityId, WorldError>` — calls `World::create_*`, pushes `EntityDelta::Created`
- `archive_entity(entity) -> Result<(), WorldError>` — calls `World::archive`, pushes `EntityDelta::Archived`
- `set_component<T>(entity, component) -> Result<(), WorldError>` — calls `World::insert_*`, pushes `ComponentDelta::Set`
- `remove_component<T>(entity) -> Result<(), WorldError>` — calls `World::remove_*`, pushes `ComponentDelta::Removed`
- `move_entity(entity, destination) -> Result<(), WorldError>` — calls `World::move_entity`, pushes `RelationDelta` for location change
- `change_quantity(entity, commodity, new_qty) -> Result<(), WorldError>` — records `QuantityDelta::Changed` with before/after
- `try_reserve(entity, reserver, range) -> Result<ReservationId, WorldError>` — calls `World::try_reserve`, pushes `ReservationDelta::Created`
- `release_reservation(id) -> Result<(), WorldError>` — calls `World::release_reservation`, pushes `ReservationDelta::Released`

### 3. Implement read-through accessors

`WorldTxn` exposes read-only accessors that delegate to `&World`:
- `get_*` component accessors
- `location_of`, `contents_of`, etc.
- These do NOT record deltas

### 4. Add `add_target`, `add_tag` builder methods

Allow callers to accumulate targets and tags before commit.

### 5. Register module in `crates/worldwake-sim/src/lib.rs`

## Files to Touch

- `crates/worldwake-sim/src/world_txn.rs` (new)
- `crates/worldwake-sim/src/lib.rs` (modify — register module, re-export)

## Out of Scope

- `commit()` method that finalizes deltas into an EventRecord (E06EVELOG-007)
- Preventing direct `World` mutation in simulation code (enforcement is E06EVELOG-009)
- Full relation mutation coverage (ownership, social — add as needed in future tickets)
- Rollback/abort semantics (WorldTxn is always committed or dropped; drop without commit is a no-op for now)

## Acceptance Criteria

### Tests That Must Pass

1. `WorldTxn::new()` constructs with required metadata (tick, cause, visibility, witness_data)
2. `create_entity` through WorldTxn records `EntityDelta::Created` in deltas
3. `archive_entity` through WorldTxn records `EntityDelta::Archived` in deltas
4. `move_entity` through WorldTxn records `RelationDelta` for location change
5. `change_quantity` through WorldTxn records `QuantityDelta::Changed` with correct before/after
6. `try_reserve` through WorldTxn records `ReservationDelta::Created`
7. `release_reservation` through WorldTxn records `ReservationDelta::Released`
8. Delta order in `self.deltas` matches mutation call order
9. Read-through accessors return current world state (including uncommitted mutations)
10. `add_target` and `add_tag` accumulate correctly
11. WorldTxn mutation errors propagate without recording a delta
12. Existing suite: `cargo test --workspace`

### Invariants

1. Every mutation through WorldTxn produces exactly one delta entry (spec 9.3)
2. Delta order matches mutation order (spec: deltas preserve committed order)
3. Failed mutations do not leave partial deltas
4. `World` state is mutated immediately (not deferred) — the journal is observational, not transactional

## Test Plan

### New/Modified Tests

1. `crates/worldwake-sim/src/world_txn.rs` — construction, each mutation wrapper records correct delta type, delta ordering, error propagation, read-through correctness, builder methods

### Commands

1. `cargo test -p worldwake-sim`
2. `cargo clippy --workspace --all-targets -- -D warnings`
3. `cargo test --workspace`
4. `cargo fmt --check`
