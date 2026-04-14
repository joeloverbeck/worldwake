# S102FROAWAEXP-006: Exploration-chain belief protection via synthetic presentation ticks

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: Yes — travel commit handler in worldwake-systems
**Deps**: archive/tickets/S102FROAWAEXP-001.md

## Problem

S101 activation-based belief decay can evict place beliefs between consecutive exploration rounds. An agent that visits an intermediate location during multi-hop exploration may lose the belief about it before the next cycle can use it as a stepping-stone, breaking exploration chains.

## Assumption Reassessment (2026-04-14)

1. `commit_travel()` at `crates/worldwake-systems/src/travel_actions.rs:264-297`. After setting ground location (line 275) and emitting movement trace (lines 282-293), there is no exploration-specific logic. Line 295 records route experience. The boost inserts after line 295.
2. `BelievedEntityState` at `crates/worldwake-core/src/belief.rs:1261-1281`. Has `presentation_ticks: [Tick; 8]` and `presentation_tick_count: u8`. `MAX_PRESENTATION_TICKS = 8` (line 1284).
3. `push_presentation_tick()` at `belief.rs:1318`. Signature: `pub fn push_presentation_tick(&mut self, tick: Tick, buffer_capacity: u8)`. Adds a new tick to the ring buffer, evicting the oldest if full.
4. `compute_activation()` at `belief.rs:2245-2256`. Takes `presentation_ticks` and `count`, computes power-law decay score. More recent/more numerous ticks → higher activation → stronger resistance to pruning.
5. `ExplorationProfile.exploration_arrival_boost: Permille` will be available from ticket 001.
6. To apply the boost, need access to the agent's `AgentBeliefStore` and the destination place's `BelievedEntityState`. The travel commit handler has access to `WorldTxn` and can read/write beliefs.
7. To determine if travel was motivated by ExploreLocation, need to inspect the agent's active goal or intention frame. `GoalKind::ExploreLocation { target_place, motivating_need }` at `goal.rs:115-118`.

## Architecture Check

1. Pushing synthetic presentation ticks reuses the existing S101 activation mechanism — no new fields on `BelievedEntityState`, no new APIs. The boost is just "this place was observed more recently/more times."
2. The boost is applied in the travel commit handler (worldwake-systems), which already writes to world state. This is consistent with FND-26 — the commit handler writes stored state that the belief pruning system later reads.
3. No backward-compatibility shims. `exploration_arrival_boost: 0` means no ticks pushed (no-op).

## Verification Layers

1. Synthetic ticks pushed on ExploreLocation arrival → focused test or action trace confirming belief state change
2. Boosted place survives pruning longer than unboosted → focused test comparing activation scores
3. No boost when travel is not ExploreLocation-motivated → focused test confirming no change
4. Cross-system: commit handler (systems) writes belief state → pruning (core) reads it. Mediated through stored state (FND-26)

## What to Change

### 1. Add exploration boost to travel commit handler

In `commit_travel()` at `crates/worldwake-systems/src/travel_actions.rs`, after the existing logic (line ~295):

1. Check if the agent's active goal is `ExploreLocation` targeting the destination
2. Read `ExplorationProfile.exploration_arrival_boost` for the agent
3. Compute tick count: `boost.value() as u32 * u32::from(BelievedEntityState::MAX_PRESENTATION_TICKS) / 1000`
4. Get the destination place's `BelievedEntityState` from the agent's `AgentBeliefStore`
5. Call `push_presentation_tick(current_tick, MAX_PRESENTATION_TICKS)` for each computed tick

Handle edge cases:
- If `exploration_arrival_boost` is 0, skip entirely
- If the place has no `BelievedEntityState` yet (first visit), the normal perception system will create it — the boost applies to the existing belief state from prior perception or a newly created one

## Files to Touch

- `crates/worldwake-systems/src/travel_actions.rs` (modify — `commit_travel()`)

## Out of Scope

- Modifying S101 decay rates or S100 retention windows
- Modifying `compute_activation()` or `prune_decayed_beliefs()`
- Adding new fields to `BelievedEntityState`
- Exploration gate or target selection logic (tickets 004, 005)

## Acceptance Criteria

### Tests That Must Pass

1. ExploreLocation arrival pushes N synthetic presentation ticks (where N = boost * 8 / 1000)
2. Non-ExploreLocation travel does NOT push synthetic ticks
3. `exploration_arrival_boost: 0` results in zero ticks pushed
4. Boosted place has higher activation than unboosted place at same age
5. Existing suite: `cargo test --workspace`

### Invariants

1. Only ExploreLocation-motivated travel triggers the boost — no other goal kinds
2. Synthetic ticks use `current_tick` — they represent the arrival moment, not fake historical observations
3. The boost is additive to normal perception ticks — it doesn't replace or override existing belief state

## Test Plan

### New/Modified Tests

1. `crates/worldwake-systems/src/travel_actions.rs` or `crates/worldwake-systems/tests/` — focused test: ExploreLocation travel pushes synthetic ticks
2. `crates/worldwake-systems/` — focused test: non-ExploreLocation travel doesn't push ticks
3. `crates/worldwake-core/src/belief.rs` — focused test: multiple push_presentation_tick calls increase activation score

### Commands

1. `cargo test -p worldwake-systems -- travel`
2. `cargo test -p worldwake-core -- compute_activation`
3. `cargo build --workspace && cargo test --workspace`
4. `cargo clippy --workspace --all-targets -- -D warnings`
