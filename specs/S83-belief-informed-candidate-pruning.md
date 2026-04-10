# S83: Belief-Informed Candidate Pruning

## Summary

Filter candidate generation for multi-location goals (primarily `AcquireCommodity`) so that only places the agent has beliefs about containing the target resource are considered. Currently `reachable_places_within_horizon()` returns all topologically reachable places (potentially 1000-7000+), each generating a candidate that the 224-300 expansion budget cannot process. This spec adds a belief-gating layer between place enumeration and candidate emission, reducing the candidate set to places where the agent believes the target commodity exists.

## Phase

Phase 7: Consequence Carriers (Adjunct — Simulation Remediation)

## Status

Draft

## Crates

- `worldwake-ai` (candidate generation, CognitiveProfile extension)
- `worldwake-core` (CognitiveProfile field addition)

## Dependencies

- E06 (GOAP planner) — completed
- S73 (Planning Snapshot Entity Relevance) — completed (parallel pattern for entity-level filtering)
- S80 (Exploration Drive) — completed (provides fallback when no places with believed resources are known)

## Design Goals

- Candidate generation for `AcquireCommodity` only emits candidates at places the agent believes contain the target commodity
- Agents with no beliefs about remote resources generate zero remote acquisition candidates (instead relying on S80's `ExploreLocation` goal to discover resources)
- Per-agent diversity: `CognitiveProfile` controls whether an agent also considers places where resources were last known but may be depleted (speculative candidates)
- The filtering is belief-informed, not world-informed — an agent with false beliefs about a place having food will still generate candidates there (FND-14)
- The change is localized to candidate generation — search, snapshot filtering, and goal dispatch are unaffected

## Non-Goals

- Hierarchical plan decomposition (TravelTo + AcquireLocal subgoals) — a valid future optimization but not required for the immediate budget exhaustion fix
- Dynamic expansion budget scaling per-agent — could enhance diversity but the core issue is candidate volume, not budget size
- Modifying `reachable_places_within_horizon()` itself — the BFS traversal remains useful for other purposes (travel candidates, exploration)

## FOUNDATIONS Alignment

| Principle | Alignment |
|-----------|-----------|
| FND-01 (Emergence) | Multi-location resource acquisition emerges from agent beliefs about the world, not from exhaustive search of all places |
| FND-05 (Carriers of Consequence) | Beliefs about place resources are carriers of consequence — agents who have explored or heard about resources act on that knowledge |
| FND-14 (World ≠ Belief) | Pruning uses beliefs, not world state. Agents with wrong beliefs generate wrong candidates. Agents with no beliefs generate no remote candidates |
| FND-15 (Knowledge Local) | Agents only consider places they have knowledge about — knowledge acquired through perception, testimony, or exploration |
| FND-20 (Resource-Bounded Reasoning) | Planner budget is spent on belief-informed candidates, not wasted on blind enumeration of all reachable places |
| FND-22 (Agent Diversity) | `speculative_acquisition` profile parameter creates behavioral diversity: cautious agents only go where they're sure, optimistic agents try places they've heard rumors about |
| FND-26 (Systems Through State) | Belief store provides the filtering data; candidate generation reads it. No direct system-to-system coupling |

## Deliverables

### 1. Belief-Gated Place Filtering Function

In `crates/worldwake-ai/src/candidate_generation.rs`:

```rust
/// Filter reachable places to only those where the agent believes
/// the target commodity exists (via resource sources or inventory).
fn belief_gated_places(
    view: &dyn GoalBeliefView,
    agent: EntityId,
    reachable: &[EntityId],
    commodity: CommodityKind,
    include_speculative: bool,
) -> Vec<EntityId> {
    reachable
        .iter()
        .copied()
        .filter(|&place| {
            // Include current place always (local acquisition is always considered)
            if view.effective_place(agent) == Some(place) {
                return true;
            }
            // Check believed resource sources at this place
            if !view.resource_sources_at(agent, place, commodity).is_empty() {
                return true;
            }
            // Check believed commodity inventory at this place (sellers, stockpiles)
            if view.controlled_commodity_quantity_at_place(agent, place, commodity).0 > 0 {
                return true;
            }
            // Speculative: include places the agent has visited but doesn't
            // currently believe have the resource (might have been depleted)
            if include_speculative {
                if view.agent_has_visited_place(agent, place) {
                    return true;
                }
            }
            false
        })
        .collect()
}
```

The function uses existing belief view methods:
- `resource_sources_at()` (defined in `per_agent_belief_view.rs`) — checks believed resource sources
- `controlled_commodity_quantity_at_place()` — checks believed commodity quantities
- `agent_has_visited_place()` — checks if agent has a belief record about the place (for speculative mode)

If `agent_has_visited_place()` does not exist on the belief view trait, it must be added. The information is derivable from the agent's belief store: a place the agent has claims about has been visited or heard about.

### 2. Integration into AcquireCommodity Candidate Generation

In `crates/worldwake-ai/src/candidate_generation.rs`, modify `acquisition_path_opportunities_inner()` (around line 4007):

**Before** (current):
```rust
let reachable = reachable_places_within_horizon(view, origin, travel_horizon);
for candidate_place in &reachable {
    // ... generate candidates at every place
}
```

**After**:
```rust
let reachable = reachable_places_within_horizon(view, origin, travel_horizon);
let cognitive = view.cognitive_profile(agent);
let include_speculative = cognitive
    .map(|p| p.speculative_acquisition)
    .unwrap_or(false);
let belief_filtered = belief_gated_places(
    view, agent, &reachable, commodity, include_speculative,
);
for candidate_place in &belief_filtered {
    // ... generate candidates only at belief-supported places
}
```

### 3. CognitiveProfile Extension

In `crates/worldwake-core/src/cognitive_profile.rs`:

```rust
/// Whether this agent generates acquisition candidates at places they've
/// visited but don't currently believe have the target resource.
/// Creates behavioral diversity: cautious agents (false) only go where
/// they're sure; optimistic agents (true) try previously-visited places.
pub speculative_acquisition: bool,
```

Default: `false`. Added to the `Default` impl. This means agents by default only consider places with positive belief evidence.

### 4. AgentDef Integration

In `crates/worldwake-cli/src/scenario/types.rs`, add `speculative_acquisition` to `CognitiveProfileDef` (if the field is scenario-definable) or ensure it propagates through the existing `CognitiveProfile` scenario path.

### 5. Diagnostic Tracing

In `crates/worldwake-ai/src/candidate_generation.rs`, add a diagnostic trace to `CandidateGenerationDiagnostics`:

```rust
/// Number of reachable places before belief filtering.
pub places_reachable: u32,
/// Number of places after belief filtering.
pub places_after_belief_filter: u32,
```

This allows the observer and decision traces to surface the filtering ratio (e.g., "1200 reachable → 3 belief-supported"), directly supporting FND-29 (Debuggability).

## Section H: Causal Hooks (FND-01)

### H1. Information-Path Analysis

- **Trigger**: Agent has unmet need → candidate generation fires → `AcquireCommodity` candidates generated.
- **Path**: Authoritative resource sources → perception → belief claims about places and their resources → `belief_gated_places()` filters reachable places → candidates emitted only for belief-supported places.
- Agents who have never perceived or been told about a remote resource generate zero remote candidates. They rely on S80 (`ExploreLocation`) to discover new places.

### H2. Positive-Feedback Analysis

- No new positive feedback loops introduced. This spec reduces candidate volume, which is a constraint, not an amplifier.

### H3. Concrete Dampeners

- N/A — no new loops to dampen.

### H4. Stored State vs. Derived

- **Stored**: `CognitiveProfile.speculative_acquisition` (per-agent parameter), belief claims (existing).
- **Derived**: `belief_gated_places()` output (computed from beliefs at generation time, never stored). Diagnostic counters (transient per-planning-cycle).

## SystemFn Integration

No new SystemFn required. The change is entirely within candidate generation, which runs as part of the existing planner pipeline.

## Component Registration

- `CognitiveProfile` already registered. The new `speculative_acquisition` field is added to the existing component.

## Cross-System Interactions

- **Perception → Candidate generation** (via belief state): Agents acquire beliefs about place resources through perception. Candidate generation reads these beliefs to filter places.
- **S80 Exploration → Candidate generation** (via belief state): When an agent has no belief-supported places, zero remote acquisition candidates are generated. The planner then falls through to `ExploreLocation` (from S80), which sends the agent to discover new places. After exploration, new beliefs are formed, enabling future acquisition candidates.
- **S73 Snapshot filtering → Candidate generation** (complementary): S73 filters entities within the planning snapshot. This spec filters places before candidate emission. The two are orthogonal and compose naturally.

## Profile-Driven Parameters

| Parameter | Type | Default | Purpose |
|-----------|------|---------|---------|
| `speculative_acquisition` | `bool` | `false` | Whether to include visited-but-no-current-evidence places in acquisition candidates |
