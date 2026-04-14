# S102FROAWAEXP-006: Exploration-chain belief protection via synthetic presentation ticks

**Status**: COMPLETED
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
6. To apply the boost, need access to the agent's `AgentBeliefStore` and the destination place's `BelievedEntityState`. The travel commit handler has access to `WorldTxn`, which dereferences to the staged `World`, so it can read `ActiveGoal`, `ExplorationProfile`, and rewrite the belief store.
7. To determine if travel was motivated by ExploreLocation, the lawful signal is the authoritative `ActiveGoal` component on the actor. `GoalKind::ExploreLocation { target_place, motivating_need }` at `goal.rs:115-118`.
8. `BelievedEntityState::MAX_PRESENTATION_TICKS` is private to `belief.rs`, so the travel layer cannot name it directly. The commit path must derive the buffer capacity from the live belief state or another local constant instead of calling the private associated const cross-crate.
9. `compute_activation()` is already public and already has lower-layer focused coverage for multiple presentation ticks in `crates/worldwake-core/src/belief.rs`. This ticket does not need to duplicate that proof if the travel-layer tests already show the boost increases the stored presentation history lawfully.

## Architecture Check

1. Pushing synthetic presentation ticks reuses the existing S101 activation mechanism — no new fields on `BelievedEntityState`. The boost is just "this place was observed more recently/more times."
2. The boost is applied in the travel commit handler (worldwake-systems), which already writes to world state. This is consistent with FND-26 — the commit handler writes stored state that the belief pruning system later reads.
3. The lived edit surface remains local to the travel action implementation and its same-file tests. Existing core activation tests are sufficient lower-layer proof for the activation math itself.
4. No backward-compatibility shims. `exploration_arrival_boost: 0` means no ticks pushed (no-op).

## Verification Layers

1. Synthetic ticks pushed on ExploreLocation arrival → focused travel-action test confirming belief-state change
2. Destination belief is synthesized and then boosted when no prior place belief exists → focused travel-action test
3. No boost when travel is not ExploreLocation-motivated or when the profile boost is zero → focused travel-action tests
4. Cross-system: commit handler (systems) writes belief state → pruning (core) reads it. Mediated through stored state (FND-26), with existing lower-layer activation math coverage reused from `worldwake-core`

## What to Change

### 1. Add exploration boost to travel commit handler

In `commit_travel()` at `crates/worldwake-systems/src/travel_actions.rs`, after the existing logic (line ~295):

1. Check if the agent's active goal is `ExploreLocation` targeting the destination
2. Read `ExplorationProfile.exploration_arrival_boost` for the agent
3. Compute tick count from the configured `Permille` and the live presentation buffer capacity (8 today, derived lawfully at the call site rather than naming the private associated const)
4. Get the destination place's `BelievedEntityState` from the agent's `AgentBeliefStore`
5. Call `push_presentation_tick(current_tick, MAX_PRESENTATION_TICKS)` for each computed tick

Handle edge cases:
- If `exploration_arrival_boost` is 0, skip entirely
- If the place has no `BelievedEntityState` yet (first visit), synthesize the normal believed place snapshot from authoritative world state first, then apply the boost to that newly inserted belief

## Files to Touch

- `crates/worldwake-systems/src/travel_actions.rs` (modify — `commit_travel()` and same-file focused tests)

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
4. Boosted travel leaves the destination belief with more presentation history than the unboosted control, which composes with the existing lower-layer `compute_activation()` proof
5. Existing suite: `cargo test --workspace`

### Invariants

1. Only ExploreLocation-motivated travel triggers the boost — no other goal kinds
2. Synthetic ticks use `current_tick` — they represent the arrival moment, not fake historical observations
3. The boost is additive to normal perception ticks — it doesn't replace or override existing belief state

## Test Plan

### New/Modified Tests

1. `crates/worldwake-systems/src/travel_actions.rs` — focused test: ExploreLocation travel pushes the expected number of synthetic ticks
2. `crates/worldwake-systems/src/travel_actions.rs` — focused test: non-ExploreLocation travel does not push synthetic ticks
3. `crates/worldwake-systems/src/travel_actions.rs` — focused test: `exploration_arrival_boost: 0` is a no-op
4. Existing lower-layer proof in `crates/worldwake-core/src/belief.rs::test_activation_computation_multiple_observations` remains the activation-math check

### Commands

1. `cargo test -p worldwake-systems --lib -- --list`
2. `cargo test -p worldwake-systems --lib travel_actions::tests::explore_location_travel_pushes_synthetic_presentation_ticks -- --exact`
3. `cargo test -p worldwake-systems --lib travel_actions::tests::non_explore_travel_does_not_push_synthetic_presentation_ticks -- --exact`
4. `cargo test -p worldwake-systems --lib travel_actions::tests::zero_exploration_arrival_boost_is_no_op -- --exact`
5. `cargo test -p worldwake-systems --lib travel_actions::tests::explore_location_travel_seeds_destination_belief_before_applying_boost -- --exact`
6. `cargo test -p worldwake-core --lib belief::tests::test_activation_computation_multiple_observations -- --exact`
7. `cargo test -p worldwake-systems`
8. `cargo build --workspace`
9. `cargo test --workspace`
10. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

Completed on 2026-04-14.

- `commit_travel()` now checks the actor's authoritative `ActiveGoal` and only applies the arrival boost when the committed trip matches an `ExploreLocation` destination.
- The travel commit path now reads `ExplorationProfile.exploration_arrival_boost`, derives a local presentation-buffer capacity lawfully, and pushes the configured number of synthetic current-tick presentation entries into the destination place belief.
- If the actor does not already know the destination place, the travel commit path now synthesizes the normal believed place snapshot from authoritative world state before applying the boost.
- Same-file travel tests now cover explore-motivated reinforcement, non-explore no-op behavior, zero-boost no-op behavior, and first-visit belief synthesis. The existing `worldwake-core` activation test remains the lower-layer proof for the activation math itself.

## Deviations

- Reassessment corrected the motivation lookup from a vague "active goal / intention frame" check to the authoritative `ActiveGoal` component, which is directly readable from `WorldTxn` inside `commit_travel()`.
- Reassessment also corrected the buffer-capacity sketch: `BelievedEntityState::MAX_PRESENTATION_TICKS` is private to `belief.rs`, so the landed travel code derives the capacity from the live belief state instead of naming the private associated const cross-crate.
- The original ticket treated "place belief missing on first visit" as an edge the normal perception system would pick up later. The landed implementation absorbs that locally by seeding the destination place belief before applying the boost so the commit-time protection is real on first arrival too.

## Verification Result

- Passed `cargo test -p worldwake-systems --lib travel_actions::tests::explore_location_travel_pushes_synthetic_presentation_ticks -- --exact`
- Passed `cargo test -p worldwake-systems --lib travel_actions::tests::non_explore_travel_does_not_push_synthetic_presentation_ticks -- --exact`
- Passed `cargo test -p worldwake-systems --lib travel_actions::tests::zero_exploration_arrival_boost_is_no_op -- --exact`
- Passed `cargo test -p worldwake-systems --lib travel_actions::tests::explore_location_travel_seeds_destination_belief_before_applying_boost -- --exact`
- Passed `cargo test -p worldwake-core --lib belief::tests::test_activation_computation_multiple_observations -- --exact`
- Passed `cargo test -p worldwake-systems`
- Passed `cargo build --workspace`
- Passed `cargo test --workspace`
- Passed `cargo clippy --workspace --all-targets -- -D warnings`
