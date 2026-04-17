# S107PRODIV-004: PlaceVisitRecord update mechanism — arrival and presence tracking

**Status**: ✅ COMPLETED
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: Yes — new visit-tracking logic in perception/location system
**Deps**: archive/tickets/S107PRODIV-003.md

## Problem

`PlaceVisitRecord` entries in `AgentBeliefStore.place_visits` need to be updated when agents arrive at or occupy places. No existing infrastructure tracks discrete visit counts or ticks-present — this is new behavior that hooks into the location-change detection system.

## Assumption Reassessment (2026-04-17)

1. Current belief-store substrate: `AgentBeliefStore.place_visits` and `PlaceVisitRecord` already exist in `crates/worldwake-core/src/belief.rs`, but no helper currently mutates them.
2. Current mutation boundary: the live per-tick belief update hook is `observe_passive_local_entities` in `crates/worldwake-systems/src/perception.rs`, where each agent's `AgentBeliefStore` is cloned, updated, and written back through `WorldTxn`. The older ticket guess about `crates/worldwake-sim/src/perception.rs` is stale.
3. Canonical path after reassessment: visit tracking should run from that passive perception loop using the agent's current `effective_place`, not by duplicating logic across every authoritative placement mutation path in `worldwake-core`.
4. `DiversificationProfile` is role-specific, but the spec stores `place_visits` on `AgentBeliefStore` for all agents. Tracking for all agents is acceptable for this slice; any future gating remains an optimization, not part of this ticket.

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

Add an `AgentBeliefStore` helper in `crates/worldwake-core/src/belief.rs` that updates `place_visits` from the current believed/occupied place and tick:
- If place has no entry: insert `PlaceVisitRecord { ticks_present: 0, last_arrival_tick: current_tick, visit_count: 1 }`
- If place has entry: increment `visit_count`, set `last_arrival_tick` to current tick
Use the existing record to distinguish a same-visit tick from a return visit so revisits reset `ticks_present` while preserving prior history.

### 2. Presence tracking

Call that helper from `observe_passive_local_entities` in `crates/worldwake-systems/src/perception.rs` before the updated belief store is compared/committed. This is the honest owned per-tick boundary for "agent is presently at place X" in the AI-facing belief substrate.

## Files to Touch

- `crates/worldwake-core/src/belief.rs` (modify) — add the `AgentBeliefStore` place-visit transition helper and focused unit coverage
- `crates/worldwake-systems/src/perception.rs` (modify) — invoke visit tracking from the passive local perception loop and add focused systems coverage

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
6. Existing suite: `cargo test -p worldwake-systems`
7. Existing suite: `cargo clippy --workspace --all-targets -- -D warnings`

### Invariants

1. PlaceVisitRecord entries are never removed (FND-18 — permanent knowledge)
2. All updates are agent-local — no cross-agent state access (FND-7)
3. visit_count monotonically increases for each place
4. ticks_present monotonically increases while agent occupies the place

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/belief.rs` — focused unit tests for arrival tracking, revisits, and presence counting
2. `crates/worldwake-systems/src/perception.rs` — focused test proving passive perception updates place visits across travel / return cycles

### Commands

1. `cargo test -p worldwake-core`
2. `cargo test -p worldwake-systems`
3. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

Completed on 2026-04-17.

- Added `AgentBeliefStore::record_place_visit` in [`crates/worldwake-core/src/belief.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/belief.rs) to distinguish first arrival, contiguous presence ticks, and later revisits using the existing `PlaceVisitRecord` state.
- Wired passive place-visit tracking into [`crates/worldwake-systems/src/perception.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/perception.rs) so each living agent updates its own belief-store visit record from the passive local perception loop even when no other observed-entity batch is produced.
- Added focused proof in `worldwake-core` for fresh visits, contiguous presence growth, and revisit reset behavior, plus a systems-level return-cycle test proving visit counts and `ticks_present` update correctly across movement between places.

## Verification Result

- Passed `cargo test -p worldwake-core record_place_visit_resets_presence_and_increments_visit_count_on_return`
- Passed `cargo test -p worldwake-systems passive_perception_updates_place_visits_across_return_cycle`
- Passed `cargo test -p worldwake-core`
- Passed `cargo test -p worldwake-systems`
- Passed `cargo clippy --workspace --all-targets -- -D warnings`
