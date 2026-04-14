# S102FROAWAEXP-005: Multi-hop frontier BFS target selection

**Status**: PENDING
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

1. BFS replaces the single-hop loop in-place — no new functions, no new types. The algorithm is O(V+E) bounded by `frontier_depth` and the place graph size, which is small.
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

1. `crates/worldwake-ai/src/candidate_generation.rs` — focused test: 3-place chain, `frontier_depth: 2`, verifies 2-hop place appears in candidates
2. `crates/worldwake-ai/src/candidate_generation.rs` — focused test: `frontier_depth: 1` matches current behavior
3. `crates/worldwake-ai/src/candidate_generation.rs` — focused test: BFS terminates on fully-explored topology

### Commands

1. `cargo test -p worldwake-ai -- select_exploration_target`
2. `cargo build --workspace && cargo test --workspace`
3. `cargo clippy --workspace --all-targets -- -D warnings`
