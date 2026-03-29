# E18BANDYN-008: Route threat estimate derived query for AI heuristic

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: Yes — worldwake-ai (new module or extension to planning_snapshot.rs)
**Deps**: E14/E15 (belief system — completed)

## Problem

Agents need a way to assess route safety through their beliefs when making travel decisions. The spec requires `route_threat_estimate(agent, edge)` as a derived query that checks the agent's beliefs about hostile presence near edge endpoints. This is never stored as authoritative state (FND-3, FND-25) — it is computed on demand from the agent's current beliefs and discarded after use.

## Assumption Reassessment (2026-03-29)

1. `AgentBeliefStore` component exists on Agent entities (from E14/E15). It stores beliefs with provenance, acquisition time, and confidence.
2. `TravelEdge` in `crates/worldwake-core/src/topology.rs` connects two `Place` entities. Route planning uses `Topology::shortest_path()` (Dijkstra).
3. The AI planner's route selection happens in `crates/worldwake-ai/src/search.rs` when expanding Travel ops. The heuristic can be injected as a cost modifier during plan search.
4. `PlanningSnapshot` in `crates/worldwake-ai/src/planning_snapshot.rs` provides the immutable belief state snapshot for planning. The threat estimate should be a method on this or on the belief view.
5. Per FND-25: the threat estimate is a derived cache, never stored as authoritative state. It must be recomputable from beliefs at any time. It must be invalidated when beliefs change.
6. Per FND-12: the estimate uses the agent's beliefs, not world truth. An agent who has never heard of bandits on a route perceives it as safe.
7. Per FND-14: ignorance is first-class — agents without danger beliefs continue using routes normally.
8. Belief aging: beliefs about danger at a location age and lose confidence if no new evidence arrives. The threat estimate reflects current belief confidence.

## Architecture Check

1. A pure derived function `route_threat_estimate(beliefs, edge_endpoints) -> Permille` is the correct approach because: (a) it's computed from existing belief state with no additional storage (FND-25), (b) it's agent-specific (each agent has different beliefs) (FND-12), (c) it can be used as a cost modifier in the existing Dijkstra-based route planner without changing the planner architecture.
2. Alternative: storing danger scores on edges would violate FND-3 and FND-25. Alternative: a separate "danger map" system would violate FND-7 (locality — danger is not a property of edges but of agent beliefs about places).
3. No backwards-compatibility shims. New derived function only.

## Verification Layers

1. Threat estimate reflects agent beliefs → focused unit test: agent with "bandits at place X" belief produces nonzero estimate for edges touching X
2. Threat estimate is zero for unknown routes → focused unit test: agent without danger beliefs returns zero
3. Threat estimate decreases as beliefs age → focused unit test: old belief produces lower estimate than fresh belief
4. Estimate is derived (never stored) → structural: function signature returns value, no component mutation
5. Route planner uses estimate → decision trace: travel plan avoids high-threat routes when safe alternative exists

## What to Change

### 1. Implement route_threat_estimate function

In a new file `crates/worldwake-ai/src/route_threat.rs` or as a method on `PlanningSnapshot`:

```rust
/// Derived query: estimates threat level for a travel edge based on
/// the agent's current beliefs about hostile presence at the edge's
/// endpoints. Returns Permille (0 = no perceived threat, 1000 = maximum).
/// This is NEVER stored as authoritative state (FND-25).
pub fn route_threat_estimate(
    beliefs: &AgentBeliefStore,
    edge_from: EntityId,
    edge_to: EntityId,
    current_tick: Tick,
) -> Permille {
    // 1. Check beliefs about hostile agents at edge_from and edge_to
    // 2. Weight by belief confidence and freshness
    // 3. Aggregate into a single Permille threat score
    // 4. Return 0 if no relevant beliefs exist (ignorance = safe)
}
```

### 2. Integrate into planner route selection

In `crates/worldwake-ai/src/search.rs`, when expanding Travel ops and selecting routes, use `route_threat_estimate` as an additive cost modifier on edge weights. High-threat edges become more expensive in the planner's search, causing it to prefer safer alternatives when available.

### 3. Re-export from lib.rs

Add module and re-export from `crates/worldwake-ai/src/lib.rs`.

## Files to Touch

- `crates/worldwake-ai/src/route_threat.rs` (new — derived query function)
- `crates/worldwake-ai/src/search.rs` (modify — integrate threat estimate into Travel op route selection)
- `crates/worldwake-ai/src/lib.rs` (modify — add module + re-export)

## Out of Scope

- Belief formation from witnessing raids (perception system from E14/E15 — already completed)
- Belief propagation via Tell action (E14/E15 — already completed)
- Belief aging/decay mechanics (E14/E15 — already completed)
- Raid action that creates the events leading to danger beliefs (E18BANDYN-003)
- AI candidate generation (E18BANDYN-006)
- Golden test T22 (E18BANDYN-009)
- Guard response to bandit reports (E19 — future epic)
- Merchant route avoidance in trade system (trade actions already use the planner's route selection)

## Acceptance Criteria

### Tests That Must Pass

1. Agent with "bandits at place X" belief: `route_threat_estimate` returns nonzero for edges touching X
2. Agent with no danger beliefs: `route_threat_estimate` returns zero for all edges
3. Agent with old, low-confidence belief: estimate is lower than for fresh, high-confidence belief
4. Agent with beliefs about multiple dangerous places: estimates aggregate correctly
5. Planner selects longer safe route over shorter dangerous route when threat estimate is high enough
6. Planner selects shorter route when no danger beliefs exist (ignorance = safe)
7. Existing suite: `cargo test -p worldwake-ai`
8. Existing suite: `cargo clippy --workspace`

### Invariants

1. FND-3 (Concrete State): no stored danger scores on edges or places
2. FND-25 (Derived Summaries): function returns value, never writes to world state. Delete and recompute produces identical results.
3. FND-12 (Belief != State): uses agent's `AgentBeliefStore`, not authoritative world state
4. FND-14 (Ignorance First-Class): agents without danger beliefs perceive routes as safe
5. FND-7 (Locality): only the agent's own beliefs are consulted, no global queries
6. No `f32`/`f64` — threat estimate is `Permille`
7. Existing planner behavior unchanged for agents without danger beliefs

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/route_threat.rs` — focused unit tests for threat estimate computation
2. `crates/worldwake-ai/src/search.rs` — integration test: planner route selection with threat-modified costs

### Commands

1. `cargo test -p worldwake-ai`
2. `cargo clippy --workspace`
3. `cargo build --workspace`
