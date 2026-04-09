# S77: Belief Capacity Prioritization

**Status**: COMPLETED

## Summary

The simulation observer report (2026-04-09) revealed agents with 91-204 perception events but belief stores containing only Waste items -- no resource sources, no places, no agents. Investigation confirmed the perception system DOES observe resource sources and places, but the belief store's capacity enforcement (`enforce_capacity()`) evicts infrastructure beliefs in favor of high-volume ground-item inventory claims. Additionally, `should_observe_current_place_entity()` gates place observation on `SceneEvidence`, silently skipping places without evidence markers. This spec fixes belief capacity prioritization so that actionable beliefs (resource sources, places, agents) survive eviction in the presence of low-value ground items.

## Phase

Phase 7: Consequence Carriers (adjunct)

## Crates

- `worldwake-core` (belief store capacity enforcement, believed entity state)
- `worldwake-systems` (perception place observation gate)

## Dependencies

- S70 (Belief Store Query Encapsulation) -- completed (archived). The accessor API is available for any new methods.

## Design Goals

- Resource source and place beliefs survive capacity enforcement when ground items compete for capacity.
- Place observation is not gated on `SceneEvidence` -- agents always observe the place they occupy.
- No new abstract scores or priority systems. Use existing concrete state (EntityKind, EntityBeliefAspect, component presence) to determine claim value.
- Minimal change to the existing capacity enforcement algorithm.

## Non-Goals

- Increasing belief capacity globally (this masks the prioritization problem).
- Changing the perception system's observation scope (it already observes all colocated entities).
- Changing how claims are formed (claim generation is correct; eviction is the problem).
- Adding new perception modulation mechanics.

## FOUNDATIONS Alignment

| Principle | Alignment |
|-----------|-----------|
| P3 (Concrete State Over Abstract Scores) | Eviction priority derived from concrete entity properties (EntityKind via `believed_kind`, EntityBeliefAspect, component presence), not abstract priority scores |
| P7 (Locality) | Agents learn about resources through local perception; this fix ensures that learning persists |
| P14 (World State Is Not Belief State) | Beliefs formed through observation remain separate from world state; `believed_kind` is the agent's observed classification, not a live world-state reference |
| P15 (Knowledge Acquired Locally) | Resource knowledge enters through perception and must survive memory |
| P16 (Ignorance, Uncertainty, Contradiction) | Capacity enforcement is finite and lossy -- this fix changes what survives, not whether eviction happens |
| P20 (Resource-Bounded Practical Reasoning) | Planner needs resource and place beliefs to generate multi-step plans |
| P26 (Systems Interact Through State) | No cross-system calls; perception writes beliefs, planner reads them |
| P28 (No Backward Compatibility) | Old eviction behavior is replaced, not shimmed |

## Section H: Causal Hooks

### Information-Path Analysis

- **Perception -> Belief Store**: Colocated entities observed -> snapshots built -> claims generated -> stored in `entity_claims` / `known_entities`. Path is unchanged; this spec only changes which claims survive capacity enforcement.
- **Belief Store -> Planner**: `direct_acquisition_path_opportunities()` reads `known_entities` to find remote resource sources. If resource source beliefs are evicted, this function returns empty and the planner cannot generate remote acquisition goals. This is the causal path that breaks.
- **Entity Kind -> Belief Store**: `build_observed_entity_snapshot()` captures `EntityKind` from authoritative world state into `believed_kind` on the snapshot. This is a one-way information transfer at observation time; the belief does not maintain a live reference to world state.

### Positive-Feedback Analysis

No positive-feedback loops introduced. Capacity enforcement is a dampening mechanism.

### Concrete Dampeners

N/A (no new feedback loops).

### Stored State vs. Derived

- **Stored state (authoritative)**: `AgentBeliefStore.entity_claims`, `AgentBeliefStore.known_entities` -- unchanged storage, changed eviction priority. `BelievedEntityState.believed_kind` -- new stored field, populated at observation time.
- **Derived**: None. Eviction ordering is computed at enforcement time, not stored.

---

## Deliverables

### D1: Believed Entity Kind Field

**Files**: `crates/worldwake-core/src/belief.rs`

`BelievedEntityState` (line 1297) and `ObservedEntitySnapshot` (line 1210) currently have no field recording the observed entity's `EntityKind`. The function `build_observed_entity_snapshot()` (line 1708) already calls `world.entity_kind(entity)` but discards the return value (uses it only as an existence check).

**Changes**:

1. Add `believed_kind: Option<EntityKind>` to `ObservedEntitySnapshot`. Populate it in `build_observed_entity_snapshot()` by capturing the `entity_kind()` return value.
2. Add `#[serde(default)] pub believed_kind: Option<EntityKind>` to `BelievedEntityState`. The `#[serde(default)]` annotation provides backward compatibility with existing serialized beliefs (consistent with `believed_artifact`, `believed_contention`, `believed_evidence`).
3. Thread `believed_kind` through `to_believed_entity_state()` (line 1228).
4. In `derive_entity_summary()` (line 1838), propagate `believed_kind` from the metadata claim winner's source entity. Since `derive_entity_summary` reconstructs `BelievedEntityState` from claims (which don't carry entity kind), look up the entity's `believed_kind` from the prior `known_entities` entry if it exists, or default to `None`.

**Rationale**: D2 and D1 both need entity kind context for tier classification. Without `believed_kind`, Place entities cannot be reliably detected from belief state alone. Adding this field aligns with P3 (Concrete State Over Abstract Scores) -- the entity's observed kind becomes explicit stored state on the belief.

### D2: Claim Value Ordering in `enforce_entity_claim_capacity()`

**File**: `crates/worldwake-core/src/belief.rs`

**Current behavior** (lines 232-253): Claims are sorted by confidence (highest first), then acquired tick, then claim ID. Claims are truncated to `entity_claim_capacity`. This means high-volume inventory claims from ground items crowd out resource-source claims.

**New behavior**: Before truncation, partition claims into two tiers:

1. **Infrastructure tier**: Claims whose `EntityBeliefAspect` is:
   - `ResourceAvailable(*)` -- always infrastructure
   - `WorkstationPresent` -- production-location knowledge the planner needs
   - `Location` -- infrastructure when the entity's `believed_kind` is `Place`
   - `Alive` -- infrastructure when the entity's `believed_kind` is `Agent`
2. **Item tier**: All other claims (inventory of ground items, waste observations, etc.).

Within each tier, sort by confidence -> acquired tick -> claim_id as before. Truncation removes item-tier claims first, then infrastructure-tier claims only if capacity is still exceeded.

**Implementation**: Add a `fn claim_eviction_tier(aspect: &EntityBeliefAspect, believed_kind: Option<EntityKind>) -> u8` helper that returns 0 for infrastructure claims and 1 for item claims. The entity's `believed_kind` is looked up from `self.known_entities` at the start of `enforce_entity_claim_capacity()`. Modify the sort key to include the tier as the primary sort dimension (lower tier = higher priority = sorted first = survives truncation).

### D3: Entity-Level Eviction Priority in `enforce_capacity()`

**File**: `crates/worldwake-core/src/belief.rs`

**Current behavior** (lines 197-214): When `known_entities` exceeds `entity_memory_capacity`, entities are evicted by last-observation tick (oldest first).

**New behavior**: Before eviction, partition entities into two tiers based on `believed_kind` and component presence:

**Infrastructure tier (0)**:
| EntityKind | Condition |
|------------|-----------|
| `Place` | Always |
| `Facility` | Always |
| `Agent` | When `alive == true` |
| Any kind | When `resource_source.is_some()` (override) |

**Transient tier (1)**:
| EntityKind | Notes |
|------------|-------|
| `ItemLot` | Ground items, waste |
| `UniqueItem` | Individual items |
| `Container` | Storage |
| `Faction` | Organizational |
| `Office` | Institutional |
| `Record` | Documents |
| `SocialArtifact` | Social objects |
| `Agent` | When `alive == false` (dead agents are transient) |

**Fallback**: If `believed_kind` is `None` (old beliefs from before this field was added), treat as transient.

Within each tier, sort by last-observation tick as before. Eviction removes transient-tier entities first. When an entity is evicted from `known_entities`, the corresponding `entity_claims` entry is also removed (preserving the existing paired-removal behavior).

**Implementation**: Add a `fn entity_eviction_tier(state: &BelievedEntityState) -> u8` helper. Modify the eviction sort key to include tier as primary dimension.

### D4: Remove SceneEvidence Gate on Place Observation

**File**: `crates/worldwake-systems/src/perception.rs`

**Current behavior** (lines 484-494): `should_observe_current_place_entity()` returns `true` only if the place has `SceneEvidence` or the agent already has an evidence belief about it.

**New behavior**: An agent always observes the place entity they currently occupy. Remove the `SceneEvidence` condition. The function can be simplified to always return `true` for the agent's current place (or removed entirely if it serves no other purpose).

**Rationale**: An agent standing at a location should always form beliefs about that location's existence and properties. Gating on `SceneEvidence` means agents at evidence-free places never learn about the place itself, which breaks the planner's ability to reason about the place graph.

**Risk assessment**: This may increase the number of beliefs per agent slightly (one additional place entity per tick). The per-entity claim capacity and entity memory capacity limits still apply, so this is bounded.

### D5: Profile-Driven Parameters

No new per-agent profile. The eviction tier logic uses concrete entity properties, not configurable weights. The existing `PerceptionProfile` fields (`entity_claim_capacity`, `entity_memory_capacity`, `memory_retention_ticks`) remain unchanged.

## SystemFn Integration

No new SystemFn. Changes are to the belief store's capacity enforcement logic, which is called from within the existing perception system's `apply_direct_local_observation_batch()` and other enforcement call sites (`apply_observations_for_witness`, `process_ask_witness_action`, tell action handlers).

## Component Registration

No new components. `BelievedEntityState` and `ObservedEntitySnapshot` are not ECS components -- they are belief-layer data structures. The `believed_kind` field addition is internal to the belief store.

## Cross-System Interactions

- **Perception -> Belief Store**: Unchanged interaction path. Perception calls `enforce_capacity()` after recording observations. The change is internal to `enforce_capacity()`.
- **Belief Store -> Planner**: `direct_acquisition_path_opportunities()` reads `known_entities` for resource sources. After this fix, resource source beliefs survive capacity enforcement, enabling the planner to generate remote acquisition goals.

## Verification

1. A unit test in `worldwake-core` demonstrates that when `entity_memory_capacity` is N and there are more than N entities including Places, Facilities with resource_source, and ItemLots, the Places and resource-bearing Facilities survive eviction while ItemLots are evicted first.
2. A unit test in `worldwake-core` demonstrates that `enforce_entity_claim_capacity()` retains `ResourceAvailable` and `WorkstationPresent` claims when item-tier claims compete for capacity.
3. Re-run the simulation observer scenario (`scenarios/cli-evaluation.ron`). Agent belief summaries should contain resource sources and places, not just Waste items.
4. `cargo test -p worldwake-core` -- existing belief store tests pass.
5. `cargo clippy --workspace --all-targets -- -D warnings` clean.
6. Note: S76-C (perception belief coverage golden test) depends on this spec but is not yet implemented. When S76-C is implemented, it should pass with these changes in place.

## Outcome

Completed on 2026-04-09.

- Landed the spec as seven archived implementation tickets: [S77BELCAPPRI-001](/home/joeloverbeck/projects/worldwake/archive/tickets/S77BELCAPPRI-001.md) through [S77BELCAPPRI-007](/home/joeloverbeck/projects/worldwake/archive/tickets/S77BELCAPPRI-007.md).
- Added `believed_kind` capture/preservation in `worldwake-core`, tiered claim eviction, tiered entity eviction, and removed the `SceneEvidence` gate from current-place observation in `worldwake-systems`.
- Corrected the downstream `e15` isolation proof to the live contract after the perception change: remote observers may lawfully know their occupied place, but still do not learn hidden remote events.
- Deviation from original plan: the implementation decomposed into multiple small tickets, and the final slice closed as a proof correction rather than a production fix because reassessment showed no information leak remained.

## Verification Result

- Passed `cargo test -p worldwake-core -- believed_kind`
- Passed `cargo test -p worldwake-core -- enforce_entity_claim_capacity`
- Passed `cargo test -p worldwake-core -- enforce_capacity`
- Passed `cargo test -p worldwake-systems -- agent_observes_place_without_scene_evidence`
- Passed `cargo test -p worldwake-systems --test e15_information_integration hidden_event_at_empty_location_remains_isolated_from_remote_agents`
- Passed `cargo test -p worldwake-core`
- Passed `cargo test -p worldwake-systems`
- Passed `cargo clippy --workspace --all-targets -- -D warnings`
