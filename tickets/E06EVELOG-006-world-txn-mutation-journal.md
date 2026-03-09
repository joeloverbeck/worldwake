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
5. `ComponentKind`, `ComponentValue`, `RelationKind`, and `RelationValue` exist from E06EVELOG-002 — prerequisite
6. Current `World` mutation APIs are explicit, not generic:
   - lifecycle: `create_entity`, `create_agent`, `create_office`, `create_faction`, `create_item_lot`, `create_unique_item`, `create_container`, `archive_entity`
   - placement: `set_ground_location`, `put_into_container`, `remove_from_container`, `move_container_subtree`
   - ownership/social: `set_owner`, `clear_owner`, `set_possessor`, `clear_possessor`, `add_member`, `remove_member`, `set_loyalty`, `clear_loyalty`, `assign_office`, `vacate_office`, `add_hostility`, `remove_hostility`, `add_known_fact`, `remove_known_fact`, `add_believed_fact`, `remove_believed_fact`
   - reservations: `try_reserve`, `release_reservation`
7. Reservation release only returns `Result<(), WorldError>`, so `WorldTxn` must snapshot the `ReservationRecord` before removing it in order to emit `ReservationDelta::Released { reservation }`
8. There is no single `move_entity` world API; placement journaling must wrap the actual placement methods and emit canonical semantic relation deltas (`LocatedIn`, `ContainedBy`, `InTransit`) as needed

## Architecture Check

1. `WorldTxn` borrows `&mut World` and `&mut EventLog`, accumulates `Vec<StateDelta>` as mutations occur, then commits by appending one `EventRecord`
2. `WorldTxn` should wrap explicit world APIs, not introduce a fake generic mutation layer that the core crate does not have. The journal surface needs to reflect real semantic operations.
3. Journaled deltas must use the canonical payloads from E06EVELOG-002 directly:
   - component writes emit `ComponentDelta` with typed `ComponentValue` before/after snapshots
   - relation writes emit `RelationDelta` with typed `RelationValue` payloads
   - reservation writes emit full `ReservationRecord` snapshots
4. The journal does NOT replace `World` mutation methods; it wraps them. `World` methods remain for bootstrap/test scenarios explicitly marked as non-journaled
5. This ticket implements the journal accumulation and the first journaled mutation wrappers. The commit-to-EventRecord flow is E06EVELOG-007
6. Initial wrapper coverage should focus on operations already central to E06 invariants and currently exercised by the core model: entity lifecycle, placement, reservations, and the smallest necessary set of typed component writes needed by the chosen create helpers

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

- `create_entity(kind) -> Result<EntityId, WorldError>` — calls `World::create_entity`, pushes `EntityDelta::Created { entity, kind }`
- `archive_entity(entity) -> Result<(), WorldError>` — snapshots `EntityKind`, calls `World::archive_entity`, pushes `EntityDelta::Archived { entity, kind }`
- Explicit create helpers matching the real world API, for example:
  - `create_agent(name, control_source) -> Result<EntityId, WorldError>`
  - `create_office(name) -> Result<EntityId, WorldError>`
  - `create_faction(name) -> Result<EntityId, WorldError>`
  - `create_item_lot(commodity, quantity) -> Result<EntityId, WorldError>`
  - `create_unique_item(...) -> Result<EntityId, WorldError>`
  - `create_container(...) -> Result<EntityId, WorldError>`
  These wrappers should record the resulting `EntityDelta` plus any component deltas implied by the helper, using typed `ComponentValue` snapshots.
- Placement wrappers around real APIs:
  - `set_ground_location(entity, place) -> Result<(), WorldError>`
  - `put_into_container(entity, container) -> Result<(), WorldError>`
  - `remove_from_container(entity) -> Result<(), WorldError>`
  - `move_container_subtree(container, new_place) -> Result<(), WorldError>`
  These wrappers should emit the necessary `RelationDelta` entries for the semantic changes they cause (`LocatedIn`, `ContainedBy`, `InTransit`) rather than a fake single “move” delta.
- `try_reserve(entity, reserver, range) -> Result<ReservationId, WorldError>` — calls `World::try_reserve`, then snapshots the created `ReservationRecord` and pushes `ReservationDelta::Created { reservation }`
- `release_reservation(id) -> Result<(), WorldError>` — snapshots the existing `ReservationRecord` before calling `World::release_reservation`, then pushes `ReservationDelta::Released { reservation }`

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
- Full ownership/social mutation coverage across every public world API in the first pass
- Rollback/abort semantics (WorldTxn is always committed or dropped; drop without commit is a no-op for now)

## Acceptance Criteria

### Tests That Must Pass

1. `WorldTxn::new()` constructs with required metadata (tick, cause, visibility, witness_data)
2. `create_entity` through WorldTxn records `EntityDelta::Created` in deltas
3. `archive_entity` through WorldTxn records `EntityDelta::Archived` in deltas
4. Placement wrappers around the actual world APIs record canonical `RelationDelta` entries for the semantic changes they cause
5. Create helpers that imply component writes record typed `ComponentDelta` snapshots using `ComponentValue`
6. `try_reserve` through WorldTxn records `ReservationDelta::Created`
7. `release_reservation` through WorldTxn records `ReservationDelta::Released` with the pre-removal `ReservationRecord`
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

1. `crates/worldwake-sim/src/world_txn.rs` — construction, lifecycle wrappers, placement wrappers, reservation snapshot correctness, typed component snapshot correctness where applicable, delta ordering, error propagation, read-through correctness, builder methods

### Commands

1. `cargo test -p worldwake-sim`
2. `cargo clippy --workspace --all-targets -- -D warnings`
3. `cargo test --workspace`
4. `cargo fmt --check`
