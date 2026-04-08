# S75BELVDECOM-003: Extract SpatialBeliefView + TemporalBeliefView sub-traits

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes — RuntimeBeliefView trait decomposition
**Deps**: archive/tickets/S75BELVDECOM-001-extract-control-belief-view.md, archive/tickets/S75BELVDECOM-002-extract-entity-profile-belief-views.md

## Problem

Extract SpatialBeliefView (12 methods: place, transit, topology, routes) and TemporalBeliefView (10 methods: tick, contention, reservations, duration) from RuntimeBeliefView.

## Assumption Reassessment (2026-04-08)

1. `RuntimeBeliefView` already composes `ControlBeliefView + EntityBeliefView + ProfileBeliefView`, so this ticket is the next live domain split rather than a fresh decomposition baseline.
2. SpatialBeliefView methods confirmed (12): `effective_place`, `is_in_transit`, `in_transit_state`, `entities_at`, `locally_observed_entities_at`, `adjacent_places`, `adjacent_places_with_travel_ticks`, `place_has_tag`, `place_has_any_tag_in`, `route_exists`, `patrol_route`, `route_experience`.
3. TemporalBeliefView methods confirmed (10): `current_tick`, `has_contention_policy`, `facility_queue_position`, `facility_grant`, `contention_queue_is_full`, `facility_queue_join_tick`, `facility_queue_patience_ticks`, `reservation_conflicts`, `reservation_ranges`, `estimate_duration`.
4. The live impl fallout is 17 `RuntimeBeliefView` blocks across production and mocks; the same split pattern used in 002 still applies.

## Architecture Check

1. Same supertrait pattern. Add `SpatialBeliefView + TemporalBeliefView` bounds to RuntimeBeliefView and remove the moved methods from the monolithic surface.
2. Because `GoalBeliefView` already delegates many of these methods through `RuntimeBeliefView`, the delegation macro must move to the new owner traits in-scope to keep the trait split honest.
3. No backward-compatibility shims.

## Verification Layers

1. Shared trait split -> compile-time proof through all touched impl blocks
2. Spatial queries -> workspace tests cover `effective_place`, `is_in_transit`, `adjacent_places`, `route_exists`, and `patrol_route` through planning and travel behavior
3. Temporal queries -> workspace tests cover `current_tick`, contention queue reads, reservation reads, and duration estimation through planning and scheduling behavior

## What to Change

### 1. Define SpatialBeliefView and TemporalBeliefView sub-traits

Move 12 spatial and 10 temporal method signatures from RuntimeBeliefView to the new sub-traits in `belief_view.rs`.

### 2. Add supertrait bounds

Add `SpatialBeliefView + TemporalBeliefView` to RuntimeBeliefView's supertrait list. Remove the 22 methods from RuntimeBeliefView's body.

### 3. Update all 18 impl blocks

Split the spatial and temporal methods into separate impl blocks.

### 4. Export new sub-traits

## Files to Touch

- `crates/worldwake-sim/src/belief_view.rs` (modify)
- `crates/worldwake-sim/src/lib.rs` (modify — exports)
- `crates/worldwake-sim/src/per_agent_belief_view.rs` (modify)
- `crates/worldwake-ai/src/planning_state.rs` (modify)
- `crates/worldwake-ai/src/planning_snapshot.rs` (modify)
- All 16 test mock files (modify)

## Out of Scope

- Other domain sub-trait extractions
- SnapshotEntity sub-struct decomposition (ticket 007)
- Additional GoalBeliefView cleanup beyond the delegation changes required to keep this trait split truthful (ticket 008)

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test --workspace`
2. `cargo clippy --workspace --all-targets -- -D warnings`

### Invariants

1. `&dyn RuntimeBeliefView` remains usable at all existing call sites.
2. No behavioral change.

## Outcome

Completed: 2026-04-08

Implemented the spatial/temporal split on the live belief surface.

- Added `SpatialBeliefView` and `TemporalBeliefView` in `crates/worldwake-sim/src/belief_view.rs` and promoted them into `RuntimeBeliefView` supertraits.
- Split the production impl ownership in `crates/worldwake-sim/src/per_agent_belief_view.rs` and `crates/worldwake-ai/src/planning_state.rs`.
- Updated `impl_goal_belief_view!` delegation so moved spatial and temporal reads resolve through the new owner traits rather than the old monolithic runtime surface.
- Migrated all affected test/mocks and remaining workspace fallout, including AI golden harness/test imports and stale UFCS calls.
- Preserved runtime behavior while flattening the trait boundary; no compatibility shim was added.

Deviations from original plan:

- None. The delivered work matched the ticket's live structural scope after reassessment.

Verification results:

- `cargo build --workspace` passed
- `cargo test --workspace` passed
- `cargo clippy --workspace --all-targets -- -D warnings` passed

## Test Plan

### New/Modified Tests

1. None — pure structural refactor.

### Commands

1. `cargo build --workspace`
2. `cargo test --workspace`
3. `cargo clippy --workspace --all-targets -- -D warnings`

### Results

1. `cargo build --workspace` passed
2. `cargo test --workspace` passed
3. `cargo clippy --workspace --all-targets -- -D warnings` passed
