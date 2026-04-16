# S106GROITEDEC-001: GroundSince component registration and lifecycle hooks

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — new ECS component `GroundSince`, lifecycle hooks across tick-aware loose-ground transitions (`set_ground_location`, container transitions, possession transitions, transit) plus archive-preparation drop/spill paths
**Deps**: None

## Problem

Items placed on the ground have no timestamp tracking when they arrived. The upcoming `ItemDecay` system (S106) needs to know how long each item has been on the ground to determine if it should be archived. Without `GroundSince`, there is no authoritative state to base decay decisions on.

## Assumption Reassessment (2026-04-16)

1. `WorldTxn::set_ground_location` exists at `crates/worldwake-core/src/world_txn.rs:436` and remains the normal runtime/scenario entry point for explicit placement on the ground. Confirmed via grep.
2. `with_component_schema_entries!` macro expansion sites confirmed at 5 files: `component_schema.rs`, `component_tables.rs`, `delta.rs`, `world.rs`, `world_txn.rs` — matches `tickets/README.md` check #13.
3. `EntityKind::ItemLot` and `EntityKind::UniqueItem` exist in `crates/worldwake-core/src/entity.rs:10-11`. These are the entity kinds eligible for `GroundSince`.
4. `remove_from_container` is also a loose-ground creation path: `remove_from_container_clears_parent_but_keeps_effective_place` (world.rs:2980) proves removing an item from a grounded container clears `contained_by` while retaining `located_in`.
5. Possession is independent of placement: `WorldTxn::set_possessor` and `clear_possessor` only mutate the possession relation, while `World::clear_possessor` leaves `effective_place` unchanged when one already exists. Dropping possessions can therefore create loose ground items without passing through `set_ground_location`.
6. Archive preparation currently resolves `DetachContentsToGround`, `SpillContentsRecursively`, and `DropPossessions` inside `crates/worldwake-core/src/world/lifecycle.rs` without a tick parameter. To keep `GroundSince` authoritative on items dropped by archive preparation, the archive-preparation API must become tick-aware in this ticket.

## Architecture Check

1. `GroundSince` needs one shared “loose ground item” rule rather than a one-off `set_ground_location` hook. The live loose-ground transitions include `set_ground_location`, `remove_from_container`, `clear_possessor`, and archive-preparation drop/spill resolutions, while `put_into_container`, `set_possessor`, and `set_in_transit` clear the timestamp. A single helper keyed off the post-mutation state is cleaner than duplicating branch logic at each call site.
2. No backward-compatibility shims. `GroundSince` is a new component that simply appears when items are placed on ground.

## Verification Layers

1. GroundSince set on ground placement → focused unit test (create item, place on ground, verify component exists with correct tick)
2. GroundSince set when an item becomes loose on the ground via container/possession teardown → focused unit test
3. GroundSince cleared when an item leaves the loose-ground state → focused unit test
4. GroundSince reset on re-drop → focused unit test (place, pick up, re-place at later tick, verify new tick value)
5. Archive-preparation drop/spill paths stamp GroundSince with the archive-preparation tick → focused world test
6. Existing set_ground_location behavior preserved → existing tests pass unchanged

## What to Change

### 1. Define GroundSince type

In `crates/worldwake-core/src/items.rs` (or a new section in the items module):

```rust
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct GroundSince(pub Tick);
```

Implement `Component` for `GroundSince`.

### 2. Register GroundSince in component schema

Add entry to `with_component_schema_entries!` macro in `crates/worldwake-core/src/component_schema.rs`. Entity kinds: `EntityKind::ItemLot` and `EntityKind::UniqueItem`. This generates methods across all 5 macro expansion sites (`component_tables.rs`, `delta.rs`, `world.rs`, `world_txn.rs`, `component_schema.rs`).

Ensure `GroundSince` is imported at each macro expansion site (bare type name must be in scope).

### 3. Add a shared loose-ground helper

Add a helper that derives whether an entity is currently a loose ground item from authoritative post-mutation state:
- eligible kind: `EntityKind::ItemLot` or `EntityKind::UniqueItem`
- `effective_place` present
- no direct container
- no possessor
- not in transit

The helper returns `Some(GroundSince(tick))` when the entity is loose on the ground and `None` otherwise.

### 4. Sync GroundSince across tick-aware lifecycle paths

Use the helper after the live tick-aware mutations:
- `WorldTxn::set_ground_location`
- `WorldTxn::put_into_container`
- `WorldTxn::remove_from_container`
- `WorldTxn::set_possessor`
- `WorldTxn::clear_possessor`
- `WorldTxn::set_in_transit`

When the post-mutation state is loose on the ground, write `GroundSince(self.tick)`. Otherwise clear the component.

### 5. Make archive preparation tick-aware for dropped live items

Change `World::prepare_entity_for_archive` / `prepare_entity_for_archive_with_policy` and the internal archive-resolution helpers to accept `Tick`. When archive preparation detaches contents to ground, spills contents recursively, or drops possessions, sync `GroundSince` for the affected live items using that tick.

### 6. Scenario initialization: items spawned on ground get GroundSince(Tick(0))

In `crates/worldwake-cli/src/scenario/mod.rs`, after `set_ground_location` is called for spawned items during scenario loading, ensure `GroundSince(Tick(0))` is set. Since the hook is in `WorldTxn::set_ground_location`, this should happen automatically if scenario initialization uses WorldTxn. If it uses `World::set_ground_location` directly (which is `pub(crate)` in worldwake-core), the hook must be added there too or the initialization must be routed through WorldTxn.

### 7. Unit tests for GroundSince lifecycle

Add tests in `crates/worldwake-core/src/world_txn.rs` (test module):
- `ground_since_set_on_ground_placement`: Create ItemLot, call `set_ground_location`, verify `get_component_ground_since` returns `GroundSince(tick)`.
- `ground_since_set_when_removed_from_container_to_ground`: Put item into grounded container, remove it, verify `get_component_ground_since` returns `GroundSince(tick)`.
- `ground_since_cleared_on_pickup`: Place on ground, then give possessor or put in container, verify `get_component_ground_since` returns `None`.
- `ground_since_resets_on_re_drop`: Place at tick 10, pick up, re-place at tick 50, verify `GroundSince(Tick(50))`.
- `ground_since_not_set_for_inventory_items`: Create item directly in inventory (via possessor), verify no `GroundSince`.

Add one focused archive-preparation test in `crates/worldwake-core/src/world.rs` proving dropped possessions or detached contents receive `GroundSince(tick)`.

## Files to Touch

- `crates/worldwake-core/src/items.rs` (modify — add `GroundSince` type)
- `crates/worldwake-core/src/component_schema.rs` (modify — register component)
- `crates/worldwake-core/src/component_tables.rs` (modify — macro expansion)
- `crates/worldwake-core/src/delta.rs` (modify — macro expansion, import)
- `crates/worldwake-core/src/world.rs` (modify — macro expansion, import)
- `crates/worldwake-core/src/world/placement.rs` (modify — loose-ground helper)
- `crates/worldwake-core/src/world/lifecycle.rs` (modify — tick-aware archive-preparation resolution sync)
- `crates/worldwake-core/src/world_txn.rs` (modify — macro expansion, import, set/clear hooks, tests)
- `crates/worldwake-core/src/lib.rs` (modify — re-export `GroundSince`)
- `specs/S106-ground-item-decay.md` (modify — correct the live GroundSince lifecycle description)

## Out of Scope

- The `item_decay_system` that reads `GroundSince` (ticket 003)
- `CommodityDecayMap` type and scenario configuration (ticket 002)
- Golden E2E tests (ticket 004)
- Decay for carried or stored items (spec non-goal)

## Acceptance Criteria

### Tests That Must Pass

1. `ground_since_set_on_ground_placement` — verifies GroundSince is written on explicit ground placement
2. `ground_since_set_when_removed_from_container_to_ground` — verifies container teardown stamps GroundSince when the item becomes loose on the ground
3. `ground_since_cleared_on_pickup` — verifies GroundSince is removed when item leaves loose-ground state
4. `ground_since_resets_on_re_drop` — verifies tick resets correctly on re-placement
5. `ground_since_not_set_for_inventory_items` — verifies no spurious GroundSince on non-ground items
6. Archive-preparation focused test — verifies dropped live items receive GroundSince at the archive-preparation tick
7. Existing suite: `cargo test -p worldwake-core` — all existing tests pass, especially placement and archive-preparation tests

### Invariants

1. Every loose ground item (`effective_place` set, no possessor, no direct container, not in transit) has a `GroundSince` component.
2. No item with a possessor, inside a container, or in transit has a `GroundSince` component.
3. `GroundSince` matches the tick when the item most recently entered the loose-ground state.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/world_txn.rs` (test module) — 5 focused unit tests for GroundSince lifecycle as described above
2. `crates/worldwake-core/src/world.rs` (test module) — archive-preparation focused test for GroundSince stamping on dropped live items

### Commands

1. `cargo test -p worldwake-core ground_since` — targeted tests
2. `cargo test -p worldwake-core` — full crate suite
3. `cargo clippy --workspace --all-targets -- -D warnings` — lint

## Outcome

Completed on 2026-04-16.

- Added the new `GroundSince(Tick)` component, registered it across the authoritative component schema, and re-exported it from `worldwake-core`.
- Replaced the drafted one-off `set_ground_location` hook with a shared loose-ground rule: tick-aware txn paths now set or clear `GroundSince` based on post-mutation state across explicit ground placement, container removal, possession changes, and transit.
- Made archive preparation tick-aware so `DetachContentsToGround`, `SpillContentsRecursively`, and `DropPossessions` stamp `GroundSince` for live items they leave loose on the ground.
- Added focused coverage for the txn lifecycle and archive-preparation drop path, and updated the active S106 spec text to match the live loose-ground contract.

## Deviations

- Reassessment showed `set_ground_location` is not the only loose-ground entry path. `remove_from_container`, `clear_possessor`, and archive-preparation drop/spill resolutions also produce loose ground items, so the ticket and spec were corrected before implementation.
- `prepare_entity_for_archive` and `prepare_entity_for_archive_with_policy` now require an explicit `Tick` so archive-preparation drops can lawfully record when the loose-ground state began.

## Verification Result

- Passed `cargo test --workspace --no-run`
- Passed `cargo test -p worldwake-core ground_since`
- Passed `cargo test -p worldwake-core`
- Passed `cargo clippy --workspace --all-targets -- -D warnings`
