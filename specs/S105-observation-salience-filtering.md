# S105: Observation Salience Filtering

**Status**: DRAFT

## Summary

Add entity-kind-based observation priority and a per-agent observation budget to the perception pipeline. Currently, `collect_direct_local_observation_batch` in `perception.rs` iterates ALL co-located entities with only a random fidelity check — no prioritization by entity kind or goal relevance. In the survival-baseline scenario (1440 ticks), 55 Waste items accumulated at Fertile Fields, causing agents to generate 10-13 Discovery events per tick just observing waste. This wastes perception bandwidth and continuously refreshes belief activation scores, preventing S101's activation-based decay from ever evicting waste beliefs.

The fix: entities are sorted by observation priority (derived from entity kind, commodity type, and agent need state) and an observation budget caps how many get observed per tick. High-priority entities (agents, facilities, resource sources) are always observed first; low-priority entities (waste on the ground) are observed only if budget remains.

## Phase

Core infrastructure (perception system enhancement)

## Crates

- `worldwake-core` (PerceptionProfile additions)
- `worldwake-systems` (perception.rs pipeline modification)
- `worldwake-cli` (scenario types and loading)
- `worldwake-ai` (golden tests)

## Dependencies

None — modifies existing perception infrastructure with no new system dependencies.

## Problem Statement

### Evidence

Observer run on `scenarios/survival-baseline.ron` (seed 104004, 1440 ticks):

| Agent | Known Items | Total Known Entities | Discovery Events (last 100) | Waste at Location |
|-------|-------------|---------------------|---------------------------|-------------------|
| Agent A | 79 | 86 | ~10 per tick | 55 (Fertile Fields) |
| Agent B | 84 | 91 | ~10 per tick | 14 (Forest Clearing) |
| Agent C | 85 | 92 | ~13 per tick | 55 (Fertile Fields) |

The majority of known items are Waste entities. Each tick at Fertile Fields, agents observe ~55 entities (the waste pile) with no filtering. The existing `need_salience_boost` only affects belief *retention* during `prune_decayed_beliefs`, not observation *collection*. This means:

1. Perception budget is wasted: agents process 55 observation checks per tick instead of the ~5-10 that matter
2. Belief activation refresh: every observation refreshes the entity's presentation tick buffer, defeating S101's power-law decay
3. Scaling failure: entity count at busy locations grows without bound (waste, dropped items, social artifacts), making perception cost O(agents * entities_at_place) per tick

### Architectural Violations

- **FND-11**: Perception cost is an unbounded positive feedback loop — more entities at a location → more observation processing → more belief refresh → less effective decay. No physical dampener exists on the observation side.
- **FND-12**: Performance cost grows linearly with entity count, with no budget or compression mechanism.
- **FND-22**: All agents observe the same way regardless of attentional capacity.

## Design

### Observation Priority Function

A deterministic priority score (u16) is computed for each co-located entity before observation:

**Base priority by entity kind:**

| EntityKind | Base Priority | Rationale |
|------------|--------------|-----------|
| Agent | 900 | Other agents are always high-priority (social, threat, cooperation) |
| Place | 800 | Place-level observations (rare, since the place itself is observed separately) |
| Facility | 700 | Workstations are survival infrastructure |
| UniqueItem | 600 | Named items (weapons, tools) are high-value |
| Office | 550 | Institutional positions — relevant for social and authority interactions |
| Container | 500 | Storage is moderately relevant |
| Faction | 450 | Organizational entities — relevant for social context |
| Record | 400 | Posted notices, ledger entries, and other social records |
| SocialArtifact | 400 | Contracts, bounties, accusations, and other social artifacts |
| ItemLot | 300 | Commodity lots — base priority, boosted by need relevance |
| ItemLot (Waste) | 100 | Waste-commodity ItemLots get the lowest base priority |

**Need-based boosting for ItemLots:**

When the agent's maximum homeostatic need pressure exceeds `need_salience_urgency_threshold`, ItemLot entities that are NOT Waste receive a boost:

```
boost = max_need * need_salience_boost / 1000
priority = base_priority + boost
```

This reuses the existing `need_salience_boost` and `need_salience_urgency_threshold` fields from PerceptionProfile, extending their purpose from retention-only to observation-and-retention. The boost applies only to non-Waste ItemLots — a hungry agent is more attentive to food items on the ground but not to waste.

**Note on divergence from retention-side boost:** The existing `salience_boost()` function in `crates/worldwake-core/src/belief.rs` (used during `prune_decayed_beliefs`) applies the need boost to ALL ItemLots regardless of commodity. The observation-side boost described here excludes Waste ItemLots. This means a new observation-specific priority function is needed — the retention-side `salience_boost()` cannot be reused directly. Both functions use the same profile fields (`need_salience_boost`, `need_salience_urgency_threshold`) but apply different Waste-exclusion logic. The boost computation should use u32 intermediate math to avoid u16 overflow, following the same pattern as the existing `salience_boost()`.

**Entity kind detection:** The system reads `world.entity_kind(entity)` and, for ItemLots, reads `world.get_component_item_lot(entity)` to check `commodity == CommodityKind::Waste`. Both are authoritative world state reads, which the perception pipeline already performs when building belief snapshots.

### Observation Budget

New field on `PerceptionProfile`:

```rust
pub observation_budget: u8,  // default: 24
```

After computing priorities for all co-located entities, the list is sorted by priority (descending, with deterministic tie-breaking by EntityId for reproducibility) and truncated to `observation_budget` entries. The existing fidelity check (`passes_observation_check`) then runs on the truncated list.

**Default value rationale:** 24 is high enough that no existing scenario (with at most ~20 entities per place) hits the budget. In waste-heavy locations (55+ entities), the budget ensures only the top 24 are observed.

**Distinction from `observation_buffer_capacity`:** The existing `observation_buffer_capacity` field controls how many pending observations are buffered before processing. The new `observation_budget` controls how many co-located entities pass the attention filter per tick. They are independent limits at different stages of the perception pipeline.

**Serde compatibility:** The new field must have a serde default (e.g., `#[serde(default = "default_observation_budget")]` returning 24) so that existing `.ron` scenarios that define `perception_profile` without this field continue to deserialize correctly.

### Modified Pipeline

```
collect_direct_local_observation_batch(world, observer, place, colocated_entities, tick, fidelity, rng, store, needs, profile)
  1. For each entity in colocated_entities (excluding observer):
     - Compute observation_priority(world, entity, needs, profile)
  2. Sort by priority descending, then by EntityId ascending (deterministic)
  3. Truncate to profile.observation_budget entries
  4. For each entry in truncated list:
     - if !passes_observation_check(fidelity, rng): continue
     - build_believed_entity_state(world, entity, tick, DirectObservation)
  5. Place observation (unchanged — always observed if fidelity passes)
  6. Missing-entity detection (unchanged — checks believed-here entities)
```

The only change is steps 1-3 inserted before the existing loop. Steps 4-6 are the existing logic unchanged.

**Caller plumbing note:** In `observe_passive_local_entities`, `HomeostaticNeeds` is currently retrieved *after* the `collect_direct_local_observation_batch` call (for use in `apply_direct_local_observation_batch`). The `get_component_homeostatic_needs` call must be moved earlier so that needs are available as a parameter to `collect_direct_local_observation_batch`. The `PerceptionProfile` is already retrieved before the call (used for `effective_observation_fidelity`), so no additional plumbing is needed for the profile parameter.

### Profile Fields

All new fields use `Permille` or primitive types per CLAUDE.md spec-drafting rules:

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `observation_budget` | `u8` | 24 | Maximum entities observed per tick (excluding the place itself) |

No new profile struct is needed. The field is added to the existing `PerceptionProfile`.

## Non-Goals

- Goal-specific observation filtering (filtering by active planner goals would couple perception to planning, violating FND-26)
- Dynamic budget adjustment based on location density (deferred — static budget is sufficient for now)
- Observation priority for evidence entries or scene elements (this spec covers entity observation only)
- Commodity-specific salience mapping beyond Waste detection (which commodities satisfy which needs would require cross-system knowledge)

## FOUNDATIONS Alignment

| Principle | Alignment |
|-----------|-----------|
| FND-1 (Emergence) | Observation patterns emerge from entity composition at each place combined with agent need state and profile parameters |
| FND-2 (No Ungrounded Triggers) | Priority is a deterministic function of entity kind, commodity type, and agent need pressure — no drama levers |
| FND-3 (Concrete State) | Priority is computed on-demand, never stored as authoritative state |
| FND-7 (Locality) | Filtering is entirely local — only co-located entities considered |
| FND-9 (Scheduling/Tie-Breaking) | Deterministic tie-breaking by EntityId ensures reproducibility across seeds |
| FND-11 (Physical Dampener) | Observation budget is the dampener for perception pollution: more entities means lower-priority ones fall below the budget cutoff |
| FND-12 (Performance Compression) | Budget bounds per-tick perception cost without changing what the agent can lawfully perceive — it limits attention, not information access |
| FND-14 (World vs Belief) | Priority reads world state (entity kind, commodity) to decide what enters belief state. This is appropriate: perception already reads world state to build belief snapshots |
| FND-20 (Resource-Bounded Reasoning) | Observation budget is a cognitive limitation — agents have finite attention per tick |
| FND-22 (Agent Diversity) | Per-agent `observation_budget` allows different attentional capacity |
| FND-26 (Systems Through State) | No cross-system calls. Priority reads entity kind and commodity from world components; need boost reads HomeostaticNeeds from agent components |

**Tension: FND-14 and filtering.** An agent co-located with waste "should" be able to observe it. The budget does not make waste invisible — it limits how many entities the agent studies carefully enough to form beliefs about in a single tick. Waste can still be observed if budget permits or if the location has fewer entities. This is a cognitive attention limit (FND-20), not an information hiding mechanism.

## FND-01 Section H Analyses

### 1. Information-Path Analysis

No new information paths are created. Existing perception paths are preserved — the change is quantitative (how many entities per tick), not qualitative (which types of information can reach agents). An agent at a waste-heavy location still perceives waste if it falls within the budget. Over multiple ticks at the same location, the agent will eventually observe most entities as the fidelity check randomizes which budget-slot entities pass.

### 2. Positive-Feedback Analysis

**Existing loop addressed:** More entities at a location → more perception processing → more belief refresh → less decay → larger belief store → more snapshot entities for planner. The observation budget breaks this loop at the perception stage.

**New loop introduced?** No. The budget is a static per-agent parameter. It does not increase with entity count or agent success.

### 3. Concrete Dampeners

The observation budget itself is the dampener: regardless of how many entities accumulate at a location, the agent observes at most `observation_budget` per tick. The dampener is physical in the FND-11 sense — it represents finite cognitive attention per unit time.

### 4. Stored State vs. Derived Read-Model

| Item | Classification | Location |
|------|---------------|----------|
| `observation_budget` | Stored state (profile parameter) | `PerceptionProfile` component |
| Observation priority score | Derived (computed per tick, never stored) | `collect_direct_local_observation_batch` |
| Sorted/truncated entity list | Transient (exists only during the function call) | Stack-local in `perception.rs` |

## SystemFn Integration

No new SystemFn. The change is internal to the existing `perception_system` SystemFn (SystemId::Perception). The sort-and-truncate happens inside `collect_direct_local_observation_batch`, which is called from `observe_passive_local_entities`, which is called from `perception_system`.

## Component Registration

New field on existing `PerceptionProfile` component:
- `observation_budget: u8` — added to the existing struct with `Default` providing 24
- Universal component (already registered on `EntityKind::Agent`)
- Scenario-definable via existing `perception_profile` in `AgentDef`

No new component registration needed.

## Cross-System Interactions

The perception system already reads `HomeostaticNeeds` from agent components for the existing `need_salience_boost` retention logic. This spec threads `HomeostaticNeeds` into the observation collection function as well. No new cross-system dependency is introduced — the same components are read in the same system.

## Testing Strategy

1. **Unit test in `perception.rs`**: Create a world with 1 agent, 30 Waste ItemLots, 2 Facilities, and 1 other Agent at the same location. Set `observation_budget = 10`. Verify the observation batch contains the agent and facilities (high priority) and at most 7 Waste items (budget remainder). Verify deterministic ordering by EntityId within same-priority tier.

2. **Golden E2E test**: Scenario with 2 agents, 1 place containing a Well, an OrchardRow, and 40 pre-placed Waste items. Run for 20 ticks with `observation_budget = 12`. Assert agents observe the Well and each other every tick. Assert total unique Waste entities in belief stores is bounded (not all 40).

3. **Regression**: Default `observation_budget = 24` must not trigger in any existing golden test scenario (verify entity counts per place in existing scenarios are below 24).

4. **Observer validation**: Re-run `survival-baseline.ron` with `observation_budget = 20` in perception profiles. Verify Discovery events per tick are bounded by the observation budget (reduced from the unbounded ~55 entity observation pool), and all agents still survive (needs managed, 0 deaths).
