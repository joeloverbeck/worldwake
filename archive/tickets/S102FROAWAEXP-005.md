# S102FROAWAEXP-005: Multi-hop frontier BFS target selection

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: Yes — select_exploration_target in candidate_generation.rs
**Deps**: archive/tickets/S102FROAWAEXP-001.md

## Problem

`select_exploration_target()` builds candidates from known places plus ONE hop of topology adjacency. After exhausting all one-hop targets, exploration stops entirely. Places two or more hops from known locations are invisible, causing agents at barren locations to starve despite reachable resources beyond the one-hop horizon.

## Assumption Reassessment (2026-04-14)

1. `select_exploration_target()` at `crates/worldwake-ai/src/candidate_generation.rs:4264-4303`. BTreeMap seeded at line 4275. Single-hop adjacency loop at lines 4276-4281 using `ctx.view.adjacent_places_with_travel_ticks(*place)`.
2. `adjacent_places_with_travel_ticks()` is a `GoalBeliefView` method at `crates/worldwake-sim/src/belief_view.rs:132` returning `Vec<(EntityId, NonZeroU32)>`. In runtime, reads world topology.
3. `profile.frontier_depth` will be available from ticket 001 (ExplorationProfile new fields).
4. Ranking logic at lines 4283-4302 selects by `(observed_tick.is_some(), travel_ticks, ...)` — frontier places with `None` observed_tick naturally rank highest. No ranking changes needed.
5. FND-07 pragmatic approximation: depth > 1 queries topology for unvisited places. Spec explicitly acknowledges this as a planning heuristic. Agent does not gain knowledge of destination contents.

## Architecture Check

1. BFS replaces the single-hop loop in-place. The landed code keeps the change in `candidate_generation.rs` and adds only a private same-file helper so focused tests can assert the discovered frontier set directly. The algorithm is O(V+E) bounded by `frontier_depth` and the place graph size, which is small.
2. No backward-compatibility shims. With `frontier_depth: 1`, behavior is identical to current code. With `frontier_depth: 2` (new default), one additional hop is explored.
3. FND-07 tension acknowledged: depth > 1 is a pragmatic planning heuristic, not omniscient knowledge (see spec P7 alignment note).

## Verification Layers

1. BFS discovers places at depth 2+ → focused unit test with multi-hop topology
2. BFS terminates at `frontier_depth` cap → focused unit test with deeper topology
3. Ranking naturally favors frontier places → existing ranking logic (no change needed, verified by reading code)
4. Single function modification — focused unit test is sufficient proof surface

## What to Change

### 1. Replace single-hop adjacency with BFS

In `select_exploration_target()` at `crates/worldwake-ai/src/candidate_generation.rs:4275-4281`:

Replace the current loop with a BFS that:
- Seeds candidates with known places (depth 0)
- Iterates `0..profile.frontier_depth` hops
- At each hop, expands frontier places via `adjacent_places_with_travel_ticks()`
- Inserts new frontier places with `None` observed_tick
- Terminates early if no new frontier places are discovered

The landed implementation may use a private same-file helper to collect the candidate frontier before `select_exploration_target()` applies the existing ranking/filtering logic.

The BFS code is specified in S102 Deliverable 5.

## Files to Touch

- `crates/worldwake-ai/src/candidate_generation.rs` (modify — `select_exploration_target()`)

## Out of Scope

- Exploration gate modification (ticket 004)
- Belief protection for explored places (ticket 006)
- Ranking changes — existing ranking naturally handles frontier places
- Need-directed exploration targeting (biasing by facility type)

## Acceptance Criteria

### Tests That Must Pass

1. With `frontier_depth: 1`, BFS produces same candidates as current single-hop logic
2. With `frontier_depth: 2`, BFS discovers places 2 hops from known places
3. With `frontier_depth: 3`, BFS discovers places 3 hops from known places
4. BFS terminates when no new frontier exists (finite topology)
5. BFS does not include the agent's current place as an exploration target (existing filter)
6. Existing suite: `cargo test --workspace`

### Invariants

1. Frontier places have `observed_tick = None` — they are candidates, not known places
2. BFS follows topology edges, not beliefs about destination contents (FND-07 heuristic)
3. `frontier_depth: 1` preserves S80 behavior exactly

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/candidate_generation.rs` — focused test: `frontier_depth: 1` matches the old known-plus-adjacent frontier
2. `crates/worldwake-ai/src/candidate_generation.rs` — focused test: `frontier_depth: 2` discovers a 2-hop place
3. `crates/worldwake-ai/src/candidate_generation.rs` — focused test: `frontier_depth` cap controls 3-hop discovery
4. `crates/worldwake-ai/src/candidate_generation.rs` — focused test: cyclic topology terminates without duplicate frontier growth
5. `crates/worldwake-ai/src/candidate_generation.rs` — focused test: final target selection still skips the current place and recently visited places

### Commands

1. `cargo test -p worldwake-ai --lib candidate_generation::tests::exploration_candidate_places_frontier_depth_one_matches_single_hop_candidates -- --exact`
2. `cargo test -p worldwake-ai --lib candidate_generation::tests::exploration_candidate_places_frontier_depth_two_discovers_second_hop_places -- --exact`
3. `cargo test -p worldwake-ai --lib candidate_generation::tests::exploration_candidate_places_frontier_depth_cap_controls_third_hop_discovery -- --exact`
4. `cargo test -p worldwake-ai --lib candidate_generation::tests::exploration_candidate_places_terminates_on_cyclic_topology_without_duplicates -- --exact`
5. `cargo test -p worldwake-ai --lib candidate_generation::tests::select_exploration_target_skips_current_place_and_recently_visited_places -- --exact`
6. `cargo test -p worldwake-ai --lib candidate_generation::tests::generate_candidates_emits_exploration_when_food_path_is_known_but_exhausted -- --exact`
7. `cargo test -p worldwake-ai`
8. `cargo build --workspace`
9. `cargo test --workspace`
10. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

Completed on 2026-04-14.

- `select_exploration_target()` now explores a BFS frontier seeded from known places and expands through `frontier_depth` hops instead of stopping after one adjacency layer.
- The BFS records newly discovered frontier places once, preserves known places with their observed ticks, and terminates cleanly on cyclic or exhausted topologies.
- Ranking behavior stayed unchanged after candidate collection: current place is still excluded, recently visited known places are still filtered by `visit_lookback_ticks`, and the nearest novel frontier still wins.
- A private same-file helper now owns frontier collection so focused tests can assert the discovered candidate set directly without changing the external exploration behavior.

## Deviations

- Focused proof required a private same-file helper for frontier collection. `select_exploration_target()` only returns the top-ranked place, so asserting lawful 2-hop and 3-hop discovery directly from the selector would have hidden deeper candidates behind the existing proximity ranking.

## Verification Result

- Passed `cargo test -p worldwake-ai --lib candidate_generation::tests::exploration_candidate_places_frontier_depth_one_matches_single_hop_candidates -- --exact`
- Passed `cargo test -p worldwake-ai --lib candidate_generation::tests::exploration_candidate_places_frontier_depth_two_discovers_second_hop_places -- --exact`
- Passed `cargo test -p worldwake-ai --lib candidate_generation::tests::exploration_candidate_places_frontier_depth_cap_controls_third_hop_discovery -- --exact`
- Passed `cargo test -p worldwake-ai --lib candidate_generation::tests::exploration_candidate_places_terminates_on_cyclic_topology_without_duplicates -- --exact`
- Passed `cargo test -p worldwake-ai --lib candidate_generation::tests::select_exploration_target_skips_current_place_and_recently_visited_places -- --exact`
- Passed `cargo test -p worldwake-ai --lib candidate_generation::tests::generate_candidates_emits_exploration_when_food_path_is_known_but_exhausted -- --exact`
- Passed `cargo test -p worldwake-ai`
- Passed `cargo build --workspace`
- Passed `cargo test --workspace`
- Passed `cargo clippy --workspace --all-targets -- -D warnings`
