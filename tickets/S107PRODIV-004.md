# S107PRODIV-004: PlaceVisitRecord update mechanism — arrival and presence tracking

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: Yes — new visit-tracking logic in perception/location system
**Deps**: S107PRODIV-003

## Problem

`PlaceVisitRecord` entries in `AgentBeliefStore.place_visits` need to be updated when agents arrive at or occupy places. No existing infrastructure tracks discrete visit counts or ticks-present — this is new behavior that hooks into the location-change detection system.

## Assumption Reassessment (2026-04-17)

1. Current perception infrastructure: `record_entity_snapshot_claims` in `belief.rs` records observation timestamps in `BelievedEntityState::presentation_ticks` but does not track visit counts or ticks-present per place.
2. Location-change detection: agents have `effective_place` tracked by the ECS. When an agent's place changes (via travel completion), perception updates fire. The arrival tracking hooks into this same detection point.
3. `DiversificationProfile` is role-specific — only agents with it need visit tracking. However, the spec stores `place_visits` on `AgentBeliefStore` (which all agents have), so tracking runs for all agents. This is acceptable: the data is small (one entry per visited place) and enables future use cases. If perf becomes a concern, gate tracking on `has_component_diversification_profile`.

## Architecture Check

1. Visit tracking is agent-local (FND-7): each agent writes to its own belief store based on its own location. No global queries, no cross-agent state.
2. Concrete state (FND-3): `PlaceVisitRecord` stores visit counts and timestamps, not derived scores. Familiarity/novelty are computed on query (ticket 006).
3. No backward-compatibility shims.

## Verification Layers

1. Arrival increments visit_count → focused unit test
2. Arrival sets last_arrival_tick to current tick → focused unit test
3. Each tick at place increments ticks_present → focused unit test
4. New place visit creates fresh PlaceVisitRecord → focused unit test
5. Visit records are never removed → invariant (no removal code path)

## What to Change

### 1. Arrival tracking

Identify the code path where an agent's `effective_place` change is detected. Add logic to update `AgentBeliefStore.place_visits`:
- If place has no entry: insert `PlaceVisitRecord { ticks_present: 0, last_arrival_tick: current_tick, visit_count: 1 }`
- If place has entry: increment `visit_count`, set `last_arrival_tick` to current tick

This may be in the perception tick, the location-update system, or the travel-completion handler. Grep for `effective_place` mutation sites to find the right hook point.

### 2. Presence tracking

Each tick, for each agent occupying a place, if `place_visits` contains an entry for the current `effective_place`, increment `ticks_present`. This can be:
- A lightweight pass in the existing perception tick, or
- A dedicated system function registered in the tick scheduler

The simplest approach is to combine both (arrival + presence) into a single function called during the perception/belief-update phase.

## Files to Touch

- Location to be determined during implementation — grep for `effective_place` mutation and perception tick ordering to find the correct insertion point. Likely candidates:
  - `crates/worldwake-sim/src/perception.rs` or equivalent perception module (modify)
  - `crates/worldwake-core/src/belief.rs` (modify) — add `update_place_visit` helper method on AgentBeliefStore

## Out of Scope

- Familiarity/novelty computation (ticket 006)
- Proactive exploration candidate emission (ticket 006)
- Gating visit tracking on DiversificationProfile presence (optimization — not needed now)

## Acceptance Criteria

### Tests That Must Pass

1. Agent arriving at a new place creates PlaceVisitRecord with visit_count=1, ticks_present=0, last_arrival_tick=current
2. Agent arriving at a previously visited place increments visit_count, updates last_arrival_tick
3. Agent staying at a place for N ticks has ticks_present=N
4. Agent visiting place A, traveling to B, returning to A: place A has visit_count=2
5. Existing suite: `cargo test -p worldwake-core`
6. Existing suite: `cargo clippy --workspace --all-targets -- -D warnings`

### Invariants

1. PlaceVisitRecord entries are never removed (FND-18 — permanent knowledge)
2. All updates are agent-local — no cross-agent state access (FND-7)
3. visit_count monotonically increases for each place
4. ticks_present monotonically increases while agent occupies the place

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/belief.rs` or equivalent — focused unit tests for arrival tracking and presence counting
2. Integration test verifying visit records accumulate correctly across multiple travel cycles

### Commands

1. `cargo test -p worldwake-core`
2. `cargo test -p worldwake-sim`
3. `cargo clippy --workspace --all-targets -- -D warnings`
