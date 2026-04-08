# S75BELVDECOM-003: Extract SpatialBeliefView + TemporalBeliefView sub-traits

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes — RuntimeBeliefView trait decomposition
**Deps**: S75BELVDECOM-001

## Problem

Extract SpatialBeliefView (12 methods: place, transit, topology, routes) and TemporalBeliefView (10 methods: tick, contention, reservations, duration) from RuntimeBeliefView.

## Assumption Reassessment (2026-04-08)

1. SpatialBeliefView methods confirmed (12): `effective_place`, `is_in_transit`, `in_transit_state`, `entities_at`, `locally_observed_entities_at`, `adjacent_places`, `adjacent_places_with_travel_ticks`, `place_has_tag`, `place_has_any_tag_in`, `route_exists`, `patrol_route`, `route_experience`.
2. TemporalBeliefView methods confirmed (10): `current_tick`, `has_contention_policy`, `facility_queue_position`, `facility_grant`, `contention_queue_is_full`, `facility_queue_join_tick`, `facility_queue_patience_ticks`, `reservation_conflicts`, `reservation_ranges`, `estimate_duration`.
3. Same 18 impl blocks as 001.

## Architecture Check

1. Same supertrait pattern. Add `SpatialBeliefView + TemporalBeliefView` bounds to RuntimeBeliefView.
2. No backward-compatibility shims.

## Verification Layers

1. Spatial queries -> golden tests exercise `effective_place`, `is_in_transit`, `adjacent_places` via travel planning every tick
2. Temporal queries -> golden tests exercise `current_tick`, `reservation_conflicts` via contention and scheduling
3. Compile-time proof -> `cargo build --workspace`

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
- GoalBeliefView changes (ticket 008)

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test --workspace`
2. `cargo clippy --workspace --all-targets -- -D warnings`

### Invariants

1. `&dyn RuntimeBeliefView` remains usable at all existing call sites.
2. No behavioral change.

## Test Plan

### New/Modified Tests

1. None — pure structural refactor.

### Commands

1. `cargo build --workspace`
2. `cargo test --workspace`
3. `cargo clippy --workspace --all-targets -- -D warnings`
