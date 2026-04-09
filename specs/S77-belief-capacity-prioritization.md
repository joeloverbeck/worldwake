# S77: Belief Capacity Prioritization

**Status**: Draft

## Summary

The simulation observer report (2026-04-09) revealed agents with 91-204 perception events but belief stores containing only Waste items -- no resource sources, no places, no agents. Investigation confirmed the perception system DOES observe resource sources and places, but the belief store's capacity enforcement (`enforce_capacity()`) evicts infrastructure beliefs in favor of high-volume ground-item inventory claims. Additionally, `should_observe_current_place_entity()` gates place observation on `SceneEvidence`, silently skipping places without evidence markers. This spec fixes belief capacity prioritization so that actionable beliefs (resource sources, places, agents) survive eviction in the presence of low-value ground items.

## Phase

Phase 7: Consequence Carriers (adjunct)

## Crates

- `worldwake-core` (belief store capacity enforcement)
- `worldwake-systems` (perception place observation gate)

## Dependencies

- S70 (Belief Store Query Encapsulation) -- completed. The accessor API is available for any new methods.

## Design Goals

- Resource source and place beliefs survive capacity enforcement when ground items compete for capacity.
- Place observation is not gated on `SceneEvidence` -- agents always observe the place they occupy.
- No new abstract scores or priority systems. Use existing concrete state (EntityKind, component presence) to determine claim value.
- Minimal change to the existing capacity enforcement algorithm.

## Non-Goals

- Increasing belief capacity globally (this masks the prioritization problem).
- Changing the perception system's observation scope (it already observes all colocated entities).
- Changing how claims are formed (claim generation is correct; eviction is the problem).
- Adding new perception modulation mechanics.

## FOUNDATIONS Alignment

| Principle | Alignment |
|-----------|-----------|
| P3 (Concrete State Over Abstract Scores) | Eviction priority derived from concrete entity properties (EntityKind, component presence), not abstract priority scores |
| P7 (Locality) | Agents learn about resources through local perception; this fix ensures that learning persists |
| P14 (World State Is Not Belief State) | Beliefs formed through observation remain separate from world state |
| P15 (Knowledge Acquired Locally) | Resource knowledge enters through perception and must survive memory |
| P16 (Ignorance, Uncertainty, Contradiction) | Capacity enforcement is finite and lossy -- this fix changes what survives, not whether eviction happens |
| P20 (Resource-Bounded Practical Reasoning) | Planner needs resource and place beliefs to generate multi-step plans |
| P26 (Systems Interact Through State) | No cross-system calls; perception writes beliefs, planner reads them |
| P28 (No Backward Compatibility) | Old eviction behavior is replaced, not shimmed |

## Section H: Causal Hooks

### Information-Path Analysis

- **Perception -> Belief Store**: Colocated entities observed -> snapshots built -> claims generated -> stored in `entity_claims` / `known_entities`. Path is unchanged; this spec only changes which claims survive capacity enforcement.
- **Belief Store -> Planner**: `direct_acquisition_path_opportunities()` reads `known_entities` to find remote resource sources. If resource source beliefs are evicted, this function returns empty and the planner cannot generate remote acquisition goals. This is the causal path that breaks.

### Positive-Feedback Analysis

No positive-feedback loops introduced. Capacity enforcement is a dampening mechanism.

### Concrete Dampeners

N/A (no new feedback loops).

### Stored State vs. Derived

- **Stored state (authoritative)**: `AgentBeliefStore.entity_claims`, `AgentBeliefStore.known_entities` -- unchanged storage, changed eviction priority.
- **Derived**: None. Eviction ordering is computed at enforcement time, not stored.

---

## Deliverables

### D1: Claim Value Ordering in `enforce_entity_claim_capacity()`

**File**: `crates/worldwake-core/src/belief.rs`

**Current behavior** (lines 232-253): Claims are sorted by confidence (highest first), then timestamp, then claim ID. Claims are truncated to `entity_claim_capacity`. This means high-volume inventory claims from ground items crowd out resource-source claims.

**New behavior**: Before truncation, partition claims into two tiers:

1. **Infrastructure tier**: Claims whose `EntityBeliefAspect` is `ResourceAvailable(*)`, `Location` on Place entities, or `Alive` on Agent entities. These represent actionable knowledge the planner needs.
2. **Item tier**: All other claims (inventory of ground items, waste observations, etc.).

Within each tier, sort by confidence -> timestamp -> claim_id as before. Truncation removes item-tier claims first, then infrastructure-tier claims only if capacity is still exceeded.

**Implementation**: Add a `fn claim_eviction_tier(aspect: &EntityBeliefAspect) -> u8` helper that returns 0 for infrastructure claims and 1 for item claims. Modify the sort key in `enforce_entity_claim_capacity()` to include the tier as the primary sort dimension (lower tier = higher priority = sorted first = survives truncation).

### D2: Entity-Level Eviction Priority in `enforce_capacity()`

**File**: `crates/worldwake-core/src/belief.rs`

**Current behavior** (lines 197-214): When `known_entities` exceeds `entity_memory_capacity`, entities are evicted by last-observation tick (oldest first).

**New behavior**: Before eviction, partition entities into two tiers based on `EntityKind`:

1. **Infrastructure tier**: Entities whose `BelievedEntityState.believed_kind` is `Place`, `Workstation`, or any kind with a `resource_source` field present.
2. **Transient tier**: All other entities (ground items, waste, etc.).

Within each tier, sort by last-observation tick as before. Eviction removes transient-tier entities first.

**Implementation**: Add a `fn entity_eviction_tier(state: &BelievedEntityState) -> u8` helper. Modify the eviction sort key to include tier as primary dimension.

### D3: Remove SceneEvidence Gate on Place Observation

**File**: `crates/worldwake-systems/src/perception.rs`

**Current behavior** (lines 484-494): `should_observe_current_place_entity()` returns `true` only if the place has `SceneEvidence` or the agent already has an evidence belief about it.

**New behavior**: An agent always observes the place entity they currently occupy. Remove the `SceneEvidence` condition. The function can be simplified to always return `true` for the agent's current place (or removed entirely if it serves no other purpose).

**Rationale**: An agent standing at a location should always form beliefs about that location's existence and properties. Gating on `SceneEvidence` means agents at evidence-free places never learn about the place itself, which breaks the planner's ability to reason about the place graph.

**Risk assessment**: This may increase the number of beliefs per agent slightly (one additional place entity per tick). The per-entity claim capacity and entity memory capacity limits still apply, so this is bounded.

### D4: Profile-Driven Parameters

No new per-agent profile. The eviction tier logic uses concrete entity properties, not configurable weights. The existing `BeliefCapacityProfile` fields (`entity_claim_capacity`, `entity_memory_capacity`, `memory_retention_ticks`) remain unchanged.

## SystemFn Integration

No new SystemFn. Changes are to the belief store's capacity enforcement logic, which is called from within the existing perception system's `apply_direct_local_observation_batch()`.

## Component Registration

No new components.

## Cross-System Interactions

- **Perception -> Belief Store**: Unchanged interaction path. Perception calls `enforce_capacity()` after recording observations. The change is internal to `enforce_capacity()`.
- **Belief Store -> Planner**: `direct_acquisition_path_opportunities()` reads `known_entities` for resource sources. After this fix, resource source beliefs survive capacity enforcement, enabling the planner to generate remote acquisition goals.

## Verification

1. S76-C (perception belief coverage) golden test should pass after this fix -- resource source beliefs survive capacity enforcement in the presence of ground items.
2. Re-run the simulation observer scenario (`scenarios/cli-evaluation.ron`). Agent belief summaries should contain resource sources and places, not just Waste items.
3. `cargo test -p worldwake-core` -- existing belief store tests pass.
4. `cargo clippy --workspace --all-targets -- -D warnings` clean.
