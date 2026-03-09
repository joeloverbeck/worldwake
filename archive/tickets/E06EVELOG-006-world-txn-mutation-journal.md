# E06EVELOG-006: WorldTxn Mutation Journal

## Archive Amendment (2026-03-09)

The final authoritative `WorldTxn` implementation lives in `worldwake-core`, not `worldwake-sim`. This archived ticket keeps the intermediate plan, but the final journal boundary was moved beside `World`.

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: Yes — new `WorldTxn` journal in `worldwake-sim`, plus minimal read-only core accessors needed for canonical journaling
**Deps**: E06EVELOG-003 (StateDelta, delta types exist), E06EVELOG-004 (EventLog exists)

## Problem

The spec requires that "all persistent world writes in normal simulation code go through `WorldTxn`" and that "no public API may mutate authoritative state without either emitting an event or being explicitly marked as transient/cache-only." `WorldTxn` is the mutation journal that captures canonical deltas as mutations occur. In this ticket it does not yet append to the log; it prepares the mutation history that E06EVELOG-007 will later emit as a single `EventRecord`.

## Assumption Reassessment (2026-03-09)

1. `World` struct exists in `worldwake-core::world` with mutation methods for components, relations, placement, ownership, reservations — confirmed
2. `StateDelta`, `EntityDelta`, `ComponentDelta`, `RelationDelta`, `QuantityDelta`, `ReservationDelta` exist from E06EVELOG-002/003 — prerequisite
3. `CauseRef`, `EventTag`, `VisibilitySpec`, `WitnessData` exist from E06EVELOG-001/002 — prerequisite
4. `EventLog` exists from E06EVELOG-004 — prerequisite
5. `ComponentKind`, `ComponentValue`, `RelationKind`, and `RelationValue` exist from E06EVELOG-002 — prerequisite
6. Current `World` mutation APIs are explicit and tick-bearing, not generic:
   - lifecycle: `create_entity`, `create_agent`, `create_office`, `create_faction`, `create_item_lot`, `create_unique_item`, `create_container`, `archive_entity`
   - placement: `set_ground_location`, `put_into_container`, `remove_from_container`, `move_container_subtree`
   - ownership/social: `set_owner`, `clear_owner`, `set_possessor`, `clear_possessor`, `add_member`, `remove_member`, `set_loyalty`, `clear_loyalty`, `assign_office`, `vacate_office`, `add_hostility`, `remove_hostility`, `add_known_fact`, `remove_known_fact`, `add_believed_fact`, `remove_believed_fact`
   - reservations: `try_reserve`, `release_reservation`
7. `World` create APIs currently take a `Tick`; `WorldTxn` should use the transaction tick for these calls instead of reintroducing per-call tick parameters
8. Physically placeable entities (`Agent`, `ItemLot`, `UniqueItem`, `Container`) are born `InTransit`; create journaling must record that canonical relation, not just the entity/component writes
9. `World::release_reservation` only returns `Result<(), WorldError>` and there is currently no direct public lookup by reservation id, so this ticket must add a minimal read-only reservation accessor in `worldwake-core`
10. There is no single `move_entity` world API; placement journaling must wrap the actual placement methods and emit canonical semantic relation deltas (`LocatedIn`, `ContainedBy`, `InTransit`) as needed
11. `archive_entity` is a composite teardown that removes many relation families and reservations in one call. The current ticket text understates that blast radius. Shipping a partial archive journal here would make the mutation record less trustworthy, not more useful.

## Architecture Check

1. `WorldTxn` borrows `&mut World`, owns event metadata plus `Vec<StateDelta>`, and stays decoupled from `EventLog` until E06EVELOG-007. This keeps mutation capture separate from append-only storage.
2. `WorldTxn` should wrap explicit world APIs, not introduce a fake generic mutation layer that the core crate does not have. The journal surface needs to reflect real semantic operations.
3. Journaled deltas must use the canonical payloads from E06EVELOG-002 directly:
   - component writes emit `ComponentDelta` with typed `ComponentValue` before/after snapshots
   - relation writes emit `RelationDelta` with typed `RelationValue` payloads
   - reservation writes emit full `ReservationRecord` snapshots
4. Read-only access should not duplicate the entire `World` API. `WorldTxn` should expose immutable read-through via `Deref<Target = World>` (or an equivalent `world(&self) -> &World` surface) so callers can inspect current state without a mirrored accessor maintenance burden.
5. The journal does NOT replace `World` mutation methods; it wraps them. `World` methods remain for bootstrap/test scenarios explicitly marked as non-journaled.
6. This ticket implements journal accumulation and a complete first slice of journaled mutation wrappers. The commit-to-EventRecord flow is E06EVELOG-007.
7. Initial wrapper coverage should focus on operations already central to E06 invariants and currently exercised by the core model: entity creation helpers, placement, reservations, and the smallest necessary set of typed component writes needed by the chosen create helpers.
8. Composite archival journaling is explicitly out of scope for this ticket. It needs a separate, fully enumerated teardown journal rather than a misleading one-delta wrapper.

## What to Change

### 1. Create `crates/worldwake-sim/src/world_txn.rs`

Define `WorldTxn`:
```rust
pub struct WorldTxn<'w> {
    world: &'w mut World,
    tick: Tick,
    cause: CauseRef,
    actor_id: Option<EntityId>,
    place_id: Option<EntityId>,
    tags: BTreeSet<EventTag>,
    target_ids: Vec<EntityId>,
    visibility: VisibilitySpec,
    witness_data: WitnessData,
    deltas: Vec<StateDelta>,
}
```

### 2. Implement journaled mutation helpers

Each wrapper calls the underlying `World` method, then pushes one or more canonical `StateDelta` entries to `self.deltas` in deterministic semantic order:

- `create_entity(kind) -> Result<EntityId, WorldError>` — calls `World::create_entity(self.tick)`, pushes `EntityDelta::Created { entity, kind }`, and for physically placeable kinds also records `RelationDelta::Added(InTransit { entity })`
- Explicit create helpers matching the real world API, for example:
  - `create_agent(name, control_source) -> Result<EntityId, WorldError>`
  - `create_office(name) -> Result<EntityId, WorldError>`
  - `create_faction(name) -> Result<EntityId, WorldError>`
  - `create_item_lot(commodity, quantity) -> Result<EntityId, WorldError>`
  - `create_unique_item(...) -> Result<EntityId, WorldError>`
  - `create_container(...) -> Result<EntityId, WorldError>`
  These wrappers should record the resulting `EntityDelta` plus any component deltas implied by the helper, using typed `ComponentValue` snapshots, and the initial `InTransit` relation where applicable.
- Placement wrappers around real APIs:
  - `set_ground_location(entity, place) -> Result<(), WorldError>`
  - `put_into_container(entity, container) -> Result<(), WorldError>`
  - `remove_from_container(entity) -> Result<(), WorldError>`
  - `move_container_subtree(container, new_place) -> Result<(), WorldError>`
  These wrappers should emit the necessary `RelationDelta` entries for the semantic changes they cause (`LocatedIn`, `ContainedBy`, `InTransit`) rather than a fake single “move” delta. Container moves must account for descendant location updates when the underlying world API propagates effective place changes.
- `try_reserve(entity, reserver, range) -> Result<ReservationId, WorldError>` — calls `World::try_reserve`, then snapshots the created `ReservationRecord` and pushes `ReservationDelta::Created { reservation }`
- `release_reservation(id) -> Result<(), WorldError>` — snapshots the existing `ReservationRecord` before calling `World::release_reservation`, then pushes `ReservationDelta::Released { reservation }`

### 3. Add the minimal core read-only reservation accessor needed for journaling

- `World::reservation(id) -> Option<&ReservationRecord>` (or equivalent)

This accessor is required so `WorldTxn` can snapshot a reservation before releasing it without reaching into core internals.

### 4. Implement immutable read-through

`WorldTxn` exposes immutable read-through to `World` without mirroring every getter:
- `Deref<Target = World>` or an equivalent immutable accessor
- these reads do NOT record deltas

### 5. Add `add_target`, `add_tag` builder methods

Allow callers to accumulate targets and tags before commit.

### 6. Register module in `crates/worldwake-sim/src/lib.rs`

## Files to Touch

- `crates/worldwake-sim/src/world_txn.rs` (new)
- `crates/worldwake-sim/src/lib.rs` (modify — register module, re-export)
- `crates/worldwake-core/src/world/reservations.rs` (modify — add read-only reservation accessor)

## Out of Scope

- `commit()` method that finalizes deltas into an EventRecord (E06EVELOG-007)
- Preventing direct `World` mutation in simulation code (enforcement is E06EVELOG-009)
- Full ownership/social mutation coverage across every public world API in the first pass
- `archive_entity` journaling; archive is a composite teardown and needs its own fully enumerated delta coverage rather than a misleading partial wrapper
- Rollback/abort semantics (WorldTxn` mutates the world immediately; this ticket is journaling, not transactional rollback)

## Acceptance Criteria

### Tests That Must Pass

1. `WorldTxn::new()` constructs with required metadata (tick, cause, visibility, witness_data)
2. `create_entity` through WorldTxn records `EntityDelta::Created` in deltas
3. Physically placeable entity creation records initial `RelationDelta::Added(InTransit { entity })`
4. Placement wrappers around the actual world APIs record canonical `RelationDelta` entries for the semantic changes they cause, including descendant location propagation for moved containers
5. Create helpers that imply component writes record typed `ComponentDelta` snapshots using `ComponentValue`
6. `try_reserve` through WorldTxn records `ReservationDelta::Created`
7. `release_reservation` through WorldTxn records `ReservationDelta::Released` with the pre-removal `ReservationRecord`
8. Delta order in `self.deltas` matches wrapper call order and each wrapper's internal deterministic delta ordering
9. Immutable read-through returns current world state (including uncommitted mutations)
10. `add_target` and `add_tag` accumulate correctly
11. WorldTxn mutation errors propagate without recording a partial delta batch
12. Existing suite: `cargo test --workspace`

### Invariants

1. A WorldTxn wrapper may emit multiple deltas when the underlying world mutation changes multiple canonical facts; the journal must capture all of them
2. Delta order matches mutation order (spec: deltas preserve committed order)
3. Failed mutations do not leave partial deltas
4. `World` state is mutated immediately (not deferred) — the journal is observational, not transactional

## Test Plan

### New/Modified Tests

1. `crates/worldwake-sim/src/world_txn.rs` — construction, create wrappers, placement wrappers, reservation snapshot correctness, typed component snapshot correctness where applicable, delta ordering, error propagation, immutable read-through correctness, builder methods
2. `crates/worldwake-core/src/world/reservations.rs` — direct reservation accessor returns created records and `None` after release

### Commands

1. `cargo test -p worldwake-sim`
2. `cargo clippy --workspace --all-targets -- -D warnings`
3. `cargo test --workspace`
4. `cargo fmt --check`

## Outcome

Outcome amended: 2026-03-09

Implemented:
- `WorldTxn` in `crates/worldwake-core/src/world_txn.rs` as a mutation journal over `&mut World` with explicit create, placement, reservation, tag, and target helpers
- immutable read-through via `Deref<Target = World>` instead of a duplicated getter surface
- canonical multi-delta journaling for create helpers, placement changes, and reservation create/release
- minimal core accessor `World::reservation(...)` so reservation release can be journaled without breaking the core/sim boundary

Changed from the original plan:
- `WorldTxn` was kept decoupled from `EventLog`; commit remains in E06EVELOG-007
- wrapper calls are allowed to emit multiple canonical deltas when one world mutation changes multiple facts
- archive journaling was intentionally removed from this ticket's scope because `archive_entity` is a composite teardown; a one-delta wrapper would have produced an incomplete causal record
- the final architecture moved `WorldTxn` into `worldwake-core` so normal authoritative writes cannot bypass the journal by crossing a crate boundary
