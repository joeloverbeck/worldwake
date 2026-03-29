# E18BANDYN-008: Route threat estimate derived query for AI heuristic

**Status**: ✅ COMPLETED
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: Yes — worldwake-ai (new module or extension to planning_snapshot.rs)
**Deps**: E14/E15 (belief system — completed)

## Problem

Agents need a way to assess route safety through their beliefs when making travel decisions. The spec requires `route_threat_estimate(agent, edge)` as a derived query that checks the agent's beliefs about hostile presence near edge endpoints. This is never stored as authoritative state (FND-3, FND-25) — it is computed on demand from the agent's current beliefs and discarded after use.

## Assumption Reassessment (2026-03-29)

Shared abstraction boundary under audit: planner-local travel cost over the belief-backed travel graph exposed by [`PlanningSnapshot`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/planning_snapshot.rs), [`PlanningState`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/planning_state.rs), and the search ordering/pruning surface in [`crates/worldwake-ai/src/search/`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/search/).

1. `AgentBeliefStore` exists on agents, but the planner does not read it directly. The live planner consumes belief-backed data through `RuntimeBeliefView -> PlanningSnapshot -> PlanningState`. The ticket must target that surface rather than propose a raw planner-owned `AgentBeliefStore` API.
2. Travel planning does not call `Topology::shortest_path()` during search. The runtime travel action only exposes adjacent-place affordances, and the planner builds multi-hop routes by chaining adjacent `travel` steps. Spatial guidance currently lives in `PlanningSnapshot::min_travel_ticks*()`, `search::heuristic::compute_heuristic()`, `search::heuristic::prune_travel_away_from_goal()`, and `search::frontier::compare_search_nodes()`.
3. The original "inject an additive cost modifier in `search.rs`" narrative is stale. There is no single route-selection hook in `search.rs`; the clean insertion point is planner-local perceived travel cost used consistently by the search frontier and travel pruning.
4. `PlanningSnapshot` already stores the planner-visible belief inputs needed for a derived route-threat query:
   - `actor_known_entity_beliefs`
   - `actor_known_social_observations`
   - `actor_confidence_policy`
   - `current_tick`
   - `places` / adjacency
5. The same fact currently has multiple lawful transport paths into the actor's reasoning:
   - persistent beliefs about entities at places via `known_entity_beliefs`
   - persistent witnessed conflict at places via `known_social_observations`
   - immediate local combat pressure via `visible_hostiles_for` / `current_attackers_of`
   Canonical path for this ticket: route threat is derived from persistent belief memory in snapshot/state, not from authoritative world state and not from the immediate-combat interrupt path.
6. Belief aging is live through [`belief_confidence()`](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/belief.rs) plus `BeliefConfidencePolicy`; there is no separate stored "danger decay" value. The derived query should reuse that live confidence model instead of introducing a second aging system.
7. `GoalKind::RegroupWithFaction` and rally-point planning already exist (`candidate_generation.rs`, `goal_model.rs`, `ranking.rs`). This ticket is narrower: perceived route threat for travel preference, not bandit regrouping or candidate generation.
8. No existing test currently proves that stale conflict/entity beliefs can make the planner prefer a longer but safer route. That is the real coverage gap this ticket should close.

## Architecture Check

1. A planner-local derived query over snapshot/state belief data is still the correct core design because it preserves FND-3/FND-25: no stored danger component, no authoritative edge score, no separate danger-map subsystem.
2. The original proposed integration point is not clean enough for the live planner. Bolting a penalty onto one route-selection call would leave the frontier ordering and travel pruning logic operating on a different cost model. The durable architecture is one perceived-travel-cost surface that the planner uses consistently wherever it reasons about travel.
3. The implementation should therefore prefer:
   - a pure route-threat/perceived-travel-cost helper derived from snapshot belief data
   - frontier ordering updated to use perceived travel cost for travel successors
   - pruning updated so "toward goal" uses the same perceived cost model rather than raw physical ticks
4. Rejected alternatives:
   - stored edge/place danger scores: violates FND-3 and FND-25
   - authoritative world queries for nearby hostiles: violates FND-12 and FND-7
   - a duplicate planner-only belief cache or alias layer around `AgentBeliefStore`: unnecessary indirection and contrary to the current snapshot contract
5. No backwards-compatibility shims or alias paths.

## Verification Layers

1. Derived route threat uses persistent belief memory, not world truth -> focused unit coverage on snapshot-backed query from believed entities / witnessed conflict
2. Ignorance remains safe -> focused unit coverage that missing relevant beliefs yields zero extra route threat
3. Stale beliefs decay under the live confidence policy -> focused unit coverage comparing fresh vs stale beliefs
4. Planner-local travel preference uses the same perceived cost surface -> focused search coverage proving a longer low-threat path beats a shorter high-threat path
5. Existing no-danger behavior stays intact -> focused search coverage proving unchanged shortest-path choice when no relevant beliefs exist
6. Derived-only invariant holds -> structural proof from pure helper API plus absence of world/component mutation in touched code

## What to Change

### 1. Implement route_threat_estimate function

In a new internal module such as `crates/worldwake-ai/src/route_threat.rs`:

```rust
/// Planner-local derived threat for one edge of the snapshot travel graph.
/// Computed from the actor's current belief memory and discarded after use.
pub(crate) fn route_threat_estimate(
    snapshot: &PlanningSnapshot,
    edge_from: EntityId,
    edge_to: EntityId,
) -> Permille {
    // 1. Scan believed entities whose last_known_place is one endpoint and
    //    whose believed state indicates a live, wounded, courage-bearing agent.
    // 2. Scan witnessed conflict observations at the endpoints.
    // 3. Weight each contributor by belief_confidence(source, staleness, policy).
    // 4. Aggregate to one bounded Permille.
    // 5. Return 0 when the actor lacks relevant beliefs.
}
```

### 2. Integrate into planner route selection

Integrate the derived query into the live planner surface, not a fictional Dijkstra hook:

- `PlanningSnapshot` gets a perceived travel-cost query alongside the existing travel-distance helpers
- `search::frontier::compare_search_nodes()` (or equivalent successor-cost accounting) uses perceived travel cost for travel-heavy branch ordering
- `search::heuristic::compute_heuristic()` / `prune_travel_away_from_goal()` use the same perceived cost model so pruning and frontier ranking stay consistent

### 3. Re-export from lib.rs

Only if the helper must be externally testable. Prefer keeping it crate-internal and testing via module tests plus search tests. Do not add a public API unless another crate actually needs it.

## Files to Touch

- `crates/worldwake-ai/src/route_threat.rs` (new — derived route-threat / perceived-travel-cost helpers)
- `crates/worldwake-ai/src/planning_snapshot.rs` (modify — expose snapshot-backed perceived travel cost queries if needed)
- `crates/worldwake-ai/src/search/heuristic.rs` (modify — use perceived travel cost in heuristic/pruning)
- `crates/worldwake-ai/src/search/frontier.rs` and/or `crates/worldwake-ai/src/search/transition.rs` (modify — keep frontier ordering aligned with perceived travel cost)
- `crates/worldwake-ai/src/lib.rs` (only if a non-test re-export is actually necessary)
- `crates/worldwake-ai/src/search/tests.rs` (modify — focused planner tests)

## Out of Scope

- Belief formation from witnessing raids (perception system from E14/E15 — already completed)
- Belief propagation via Tell action (E14/E15 — already completed)
- Belief aging/decay mechanics themselves (reuse existing `belief_confidence` contract)
- Raid action that creates the events leading to danger beliefs (E18BANDYN-003)
- AI candidate generation (E18BANDYN-006)
- Golden test T22 (E18BANDYN-009)
- Guard response to bandit reports (E19 — future epic)
- Any new stored "danger memory" component or planner/world alias layer

## Acceptance Criteria

### Tests That Must Pass

1. Agent with relevant hostile/conflict beliefs at place X: `route_threat_estimate` returns nonzero for edges touching X
2. Agent with no danger beliefs: `route_threat_estimate` returns zero for all edges
3. Agent with old, low-confidence belief: estimate is lower than for fresh, high-confidence belief
4. Agent with beliefs about multiple dangerous places: estimates aggregate correctly
5. Planner selects longer safe route over shorter dangerous route when perceived threat is high enough
6. Planner selects shorter route when no relevant danger beliefs exist (ignorance = safe)
7. Existing suite: `cargo test -p worldwake-ai`
8. Existing suite: `cargo clippy --workspace`

### Invariants

1. FND-3 (Concrete State): no stored danger scores on edges or places
2. FND-25 (Derived Summaries): function returns value, never writes to world state. Delete and recompute produces identical results.
3. FND-12 (Belief != State): uses snapshot/state belief memory derived from the actor's belief store, not authoritative world state
4. FND-14 (Ignorance First-Class): agents without danger beliefs perceive routes as safe
5. FND-7 (Locality): only the agent's own beliefs are consulted, no global queries
6. No `f32`/`f64` — threat estimate is `Permille`
7. Existing planner behavior unchanged for agents without danger beliefs

## Tests

### New/Modified Tests

1. `crates/worldwake-ai/src/route_threat.rs`
Rationale: proves the derived threat helper stays belief-backed, decays with staleness, remains zero under ignorance, and aggregates endpoint danger without adding stored world state.
2. `crates/worldwake-ai/src/search/tests.rs`
Rationale: proves the live planner search surface actually changes route choice under danger beliefs and preserves the shorter-route default when those beliefs are absent.

### Commands

1. `cargo test -p worldwake-ai`
2. `cargo clippy --workspace --all-targets -- -D warnings`
3. `cargo build --workspace`

## Outcome

- Completion date: 2026-03-29
- What actually changed:
  - Added `crates/worldwake-ai/src/route_threat.rs` with derived route-threat and perceived direct travel-cost helpers based on snapshot belief memory, social conflict observations, and live `belief_confidence()` aging.
  - Extended `PlanningSnapshot` with perceived travel-cost matrix support so the planner can query belief-backed route costs without touching authoritative world state.
  - Updated planner search ordering and travel pruning to use perceived travel cost consistently through `search::frontier`, `search::heuristic`, and `search::transition`.
  - Added focused route-threat unit tests and focused search tests covering safe-detour preference and no-danger fallback behavior.
- Deviations from original plan:
  - Did not add a public re-export from `lib.rs`; the helper remains crate-internal because no external crate needs it.
  - Did not implement a one-off additive modifier inside `search.rs`; the final architecture uses a consistent planner-local cost surface instead.
  - Threat evidence is derived from existing conflict/activity/wound belief memory rather than a new specialized "bandits at place" belief type.
- Verification results:
  - `cargo test -p worldwake-ai` passed
  - `cargo clippy --workspace --all-targets -- -D warnings` passed
  - `cargo build --workspace` passed
