# S83: Belief-Informed Candidate Pruning

## Summary

Filter candidate generation for multi-location goals (primarily `AcquireCommodity`) so that only places the agent has beliefs about containing the target resource are considered. Currently `reachable_places_within_horizon()` returns all topologically reachable places (potentially 1000-7000+), each generating a candidate that the 224-300 expansion budget cannot process. This spec adds a belief-gating layer between place enumeration and candidate emission, reducing the candidate set to places where the agent believes the target commodity exists.

## Phase

Phase 7: Consequence Carriers (Adjunct — Simulation Remediation)

## Status

Draft

## Crates

- `worldwake-ai` (candidate generation, diagnostics)
- `worldwake-sim` (GoalBeliefView trait extension, RuntimeBeliefView impl)
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
| FND-26 (Systems Through State) | Belief store provides the filtering data; candidate generation reads it via GoalBeliefView. No direct system-to-system coupling |
| FND-29 (Debuggability) | Diagnostic counters (`places_reachable`, `places_after_belief_filter`) surface the filtering ratio in decision traces, making belief pruning observable and debuggable |

## Deliverables

### 1. Belief-Gated Place Filtering Function

In `crates/worldwake-ai/src/candidate_generation.rs`:

```rust
/// Filter reachable places to only those where the agent believes
/// the target commodity exists (via resource sources or the agent's
/// own controlled inventory at that place).
fn belief_gated_places(
    view: &dyn GoalBeliefView,
    agent: EntityId,
    reachable: &[EntityId],
    commodity: CommodityKind,
    include_speculative: bool,
) -> Vec<EntityId> {
    // For the speculative path, build the set of places the agent
    // has any beliefs about (visited or heard of). Reuses the existing
    // known_place_observations() helper (line ~4147) which derives this
    // from the agent's belief store.
    let known_places = if include_speculative {
        Some(known_place_observations(view, agent))
    } else {
        None
    };

    reachable
        .iter()
        .copied()
        .filter(|&place| {
            // Include current place always (local acquisition is always considered)
            if view.effective_place(agent) == Some(place) {
                return true;
            }
            // Check believed resource sources at this place (e.g., orchards,
            // mines, wells — entities the agent believes produce this commodity)
            if !view.resource_sources_at(place, commodity).is_empty() {
                return true;
            }
            // Check agent's own controlled commodity quantity at this place
            // (the agent's own remote stockpiles, not other entities' inventory)
            if view.controlled_commodity_quantity_at_place(agent, place, commodity).0 > 0 {
                return true;
            }
            // Speculative: include places the agent has any beliefs about
            // (visited or heard of) even if no current resource evidence exists.
            // The resource may have been depleted since last observation.
            if let Some(ref known) = known_places {
                if known.contains_key(&place) {
                    return true;
                }
            }
            false
        })
        .collect()
}
```

The function uses existing belief view methods:
- `resource_sources_at(place, commodity)` (defined in `per_agent_belief_view.rs`, trait at `belief_view.rs:184`) — checks believed resource sources. Note: no `agent` parameter; the belief view is already agent-scoped.
- `controlled_commodity_quantity_at_place(agent, place, commodity)` — checks the agent's own controlled commodity quantities at the place (returns `Quantity`, a newtype over `u32`)
- `known_place_observations(view, agent)` (existing helper at `candidate_generation.rs:4147`) — returns `BTreeMap<EntityId, Tick>` of places the agent has beliefs about, derived from the agent's belief store

### 2. GoalBeliefView Accessor for CognitiveProfile

In `crates/worldwake-sim/src/belief_view.rs`, add to the `GoalBeliefView` trait (following the pattern of `exploration_profile` at line 198 and `disposal_profile` at line 194):

```rust
fn cognitive_profile(&self, agent: EntityId) -> Option<CognitiveProfile> {
    let _ = agent;
    None
}
```

Add the corresponding `RuntimeBeliefView` impl that reads `CognitiveProfile` from the world's component table, and the `GoalBeliefView` blanket-impl forwarding in the `ProfileBeliefView` block. This follows the exact pattern used for `ExplorationProfile` and `DisposalProfile`.

Update `TestBeliefView` in `candidate_generation.rs` tests to support the new method (add a `cognitive_profiles: BTreeMap<EntityId, CognitiveProfile>` field).

### 3. Integration into AcquireCommodity Candidate Generation

In `crates/worldwake-ai/src/candidate_generation.rs`, modify `acquisition_path_opportunities_inner()` (line ~4058).

**Before** (current code at lines 4071-4085):
```rust
reachable_places_within_horizon(view, origin, travel_horizon)
    .into_iter()
    .filter_map(|candidate_place| {
        acquisition_path_evidence_at_place(
            view, agent, candidate_place, commodity,
            recipes, travel_horizon, options,
        )
        .map(|(evidence, trace)| (candidate_place, evidence, trace))
    })
    .collect()
```

**After**:
```rust
let cognitive = view.cognitive_profile(agent);
let include_speculative = cognitive
    .map(|p| p.speculative_acquisition)
    .unwrap_or(false);
let reachable = reachable_places_within_horizon(view, origin, travel_horizon);
let belief_filtered = belief_gated_places(
    view, agent, &reachable, commodity, include_speculative,
);
belief_filtered
    .into_iter()
    .filter_map(|candidate_place| {
        acquisition_path_evidence_at_place(
            view, agent, candidate_place, commodity,
            recipes, travel_horizon, options,
        )
        .map(|(evidence, trace)| (candidate_place, evidence, trace))
    })
    .collect()
```

### 4. CognitiveProfile Extension

In `crates/worldwake-core/src/cognitive_profile.rs`:

```rust
/// Whether this agent generates acquisition candidates at places they've
/// heard of but don't currently believe have the target resource.
/// Creates behavioral diversity: cautious agents (false) only go where
/// they're sure; optimistic agents (true) try previously-known places.
pub speculative_acquisition: bool,
```

Default: `false`. Added to the existing `Default` impl (lines 21-38). This means agents by default only consider places with positive belief evidence.

The `bool` type satisfies all existing derives on `CognitiveProfile` (`Clone`, `Copy`, `Debug`, `Eq`, `PartialEq`, `Ord`, `PartialOrd`, `Serialize`, `Deserialize`).

No separate `CognitiveProfileDef` type is needed — `AgentDef` in `crates/worldwake-cli/src/scenario/types.rs` uses `Option<CognitiveProfile>` directly (line 86), and `spawn_agent()` in `crates/worldwake-cli/src/scenario/mod.rs` maps it via `unwrap_or_default()` (line 368). Scenario `.ron` files can set `speculative_acquisition: true` directly in the `cognitive_profile` block. The existing `cognitive_profile_roundtrips_through_bincode` test (cognitive_profile.rs:81) should be verified to cover the new field.

### 5. Diagnostic Tracing

In `crates/worldwake-ai/src/candidate_generation.rs`, add fields to `CandidateGenerationDiagnostics` (line ~159, currently 6 fields: `omitted_political`, `omitted_bandit`, `omitted_social`, `omitted_violation_detection`, `evidence`, `fully_blocked_desires`):

```rust
/// Number of reachable places before belief filtering.
pub places_reachable: u32,
/// Number of places after belief filtering.
pub places_after_belief_filter: u32,
```

These `u32` fields default to `0` via the struct's `#[derive(Default)]`.

**Integration point**: The diagnostic counters should be recorded at the call sites in `emit_self_consume_candidates` (line ~2284), which already has `&mut diagnostics` access. The counters are set after `reachable_places_within_horizon` returns (for `places_reachable`) and after `belief_gated_places` returns (for `places_after_belief_filter`). This may require `acquisition_path_opportunities_inner` to return the counts alongside its current `Vec` result, or the counters can be recorded by a wrapper at the `acquisition_path_opportunities` / `direct_acquisition_path_opportunities` level.

This allows the observer and decision traces to surface the filtering ratio (e.g., "1200 reachable -> 3 belief-supported"), directly supporting FND-29 (Debuggability).

## Section H: Causal Hooks (FND-01)

### H1. Information-Path Analysis

- **Trigger**: Agent has unmet need -> candidate generation fires -> `AcquireCommodity` candidates generated.
- **Path**: Authoritative resource sources -> perception -> belief claims about places and their resources -> `belief_gated_places()` filters reachable places -> candidates emitted only for belief-supported places.
- Agents who have never perceived or been told about a remote resource generate zero remote candidates. They rely on S80 (`ExploreLocation`) to discover new places.

### H2. Positive-Feedback Analysis

- No new positive feedback loops introduced. This spec reduces candidate volume, which is a constraint, not an amplifier.

### H3. Concrete Dampeners

- N/A — no new loops to dampen.

### H4. Stored State vs. Derived

- **Stored**: `CognitiveProfile.speculative_acquisition` (per-agent parameter), belief claims (existing).
- **Derived**: `belief_gated_places()` output (computed from beliefs at generation time, never stored). `known_place_observations()` output (derived from belief store, transient). Diagnostic counters (transient per-planning-cycle).

## SystemFn Integration

No new SystemFn required. The change is entirely within candidate generation, which runs as part of the existing planner pipeline.

## Component Registration

- `CognitiveProfile` already registered. The new `speculative_acquisition` field is added to the existing component.
- GoalBeliefView gains a `cognitive_profile()` accessor (Deliverable 2), following the existing profile accessor pattern.

## Cross-System Interactions

- **Perception -> Candidate generation** (via belief state): Agents acquire beliefs about place resources through perception. Candidate generation reads these beliefs to filter places.
- **S80 Exploration -> Candidate generation** (via belief state): When an agent has no belief-supported places, zero remote acquisition candidates are generated. The planner then falls through to `ExploreLocation` (from S80), which sends the agent to discover new places. After exploration, new beliefs are formed, enabling future acquisition candidates.
- **S73 Snapshot filtering -> Candidate generation** (complementary): S73 filters entities within the planning snapshot. This spec filters places before candidate emission. The two are orthogonal and compose naturally.

## Profile-Driven Parameters

| Parameter | Type | Default | Purpose |
|-----------|------|---------|---------|
| `speculative_acquisition` | `bool` | `false` | Whether to include known-but-no-current-evidence places in acquisition candidates |
