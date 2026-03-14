# E14: Perception & Belief System

## Epic Summary
Implement visibility-based perception, per-agent belief stores with state-snapshot model, perception profiles for agent diversity, and world/belief separation enforcement via `PerAgentBeliefView`.

## Phase
Phase 3: Information & Politics

## Crates
- `worldwake-core`: `AgentBeliefStore`, `BelievedEntityState`, `PerceptionSource`, `PerceptionProfile`, `SocialObservation`, `SocialObservationKind` (all authoritative types)
- `worldwake-sim`: `PerAgentBeliefView` struct (implements `BeliefView` trait defined in sim)
- `worldwake-systems`: `perception_system()` function (the system that runs each tick)

## Dependencies
- E13 (decision architecture queries beliefs for planning)

## Deliverables

### Visibility Rules
- Agents perceive events based on location:
  - Same place = visible (direct perception)
  - Adjacent place = partially visible (depending on edge visibility)
  - Distant = not visible (requires information channels)
- Perception check: `can_perceive(agent, event) -> bool`
  - Based on agent location vs event location
  - Modified by event's `VisibilitySpec` setting (actual enum variants from `crates/worldwake-core/src/visibility.rs`):
    - `ParticipantsOnly` — only direct participants perceive
    - `SamePlace` — co-located agents perceive
    - `AdjacentPlaces { max_hops }` — agents within hop count perceive (immediate spillover only per FND-01 Section B)
    - `PublicRecord` — consultable at a location, not globally known
    - `Hidden` — no passive perception

### Direct Perception
- When an event occurs at an agent's location:
  - Agent automatically becomes a witness
  - Agent's `AgentBeliefStore` is updated with observed entity state
  - Observation is subject to `PerceptionProfile.observation_fidelity` roll
- Perception is passive (agents don't choose what to notice at their location)

### State-Snapshot Belief Model
`AgentBeliefStore` component per agent:

```
AgentBeliefStore:
  known_entities: BTreeMap<EntityId, BelievedEntityState>
  social_observations: Vec<SocialObservation>
```

```
BelievedEntityState:
  last_known_place: Option<EntityId>
  last_known_inventory: BTreeMap<CommodityKind, Quantity>
  alive: bool
  wounds: Vec<Wound>
  observed_tick: Tick
  source: PerceptionSource
```

Mirrors queryable `World` state per entity. Fields extend as needed by future epics.

```
PerceptionSource:
  DirectObservation
  Report { from: EntityId, chain_len: u8 }
  Rumor { chain_len: u8 }
  Inference
```

Confidence is **not stored** — it is derived at query time from `PerceptionSource` variant + staleness (`current_tick - observed_tick`). This satisfies Principle 3 (Concrete State over Abstract Scores): no abstract `confidence: Permille` is stored as authoritative state.

### Belief Staleness
- Beliefs can become outdated:
  - Agent observed apple stock at tick 100, it's now tick 500
  - Staleness = current_tick - observed_tick
  - Stale beliefs still used for planning (agents don't know they're outdated)
  - Beliefs updated only when agent re-observes or receives new information

### PerAgentBeliefView (implements BeliefView)

`PerAgentBeliefView` is a struct in `worldwake-sim` that implements the existing `BeliefView` trait by reading from `AgentBeliefStore`. It answers the ~50 `BeliefView` methods as follows:

- **Self-queries** (`homeostatic_needs`, `wounds`, `combat_profile`, `trade_disposition_profile`, `metabolism_profile`, `drive_thresholds`, `known_recipes`, `merchandise_profile`, `carry_capacity`, `load_of_entity` for self, own inventory queries): **authoritative from World**. An agent always knows its own state.
- **Observed entity state** (`effective_place`, `is_alive`, `commodity_quantity`, `direct_possessions`, `wounds` for others, `agents_selling_at`, `corpse_entities_at`): from `BelievedEntityState` if agent has observed that entity. Returns `None`/empty/default if not observed.
- **Topology queries** (`adjacent_places`, `adjacent_places_with_travel_ticks`, `place_has_tag`, `workstation_tag`, `matching_workstations_at`, `resource_sources_at`): **authoritative**. The place graph is public infrastructure — agents know where places are and how to travel between them.
- **Profile queries for others** (`combat_profile`, `trade_disposition_profile` for non-self): require observation. Unknown unless the agent has perceived them.
- **Unknown entities**: do not appear in planning snapshots — agent doesn't know they exist.

### OmniscientBeliefView Migration Strategy

Two distinct use contexts exist:

1. **Agent AI decisions** (in `worldwake-ai`): Replace `OmniscientBeliefView::new(world)` calls with `PerAgentBeliefView::new(agent, &world, &belief_stores)`. Primary call sites: `AgentTickDriver::produce_agent_input` (~16 call sites in `agent_tick.rs`).
2. **Action validation / system execution** (in `worldwake-systems`): These run on behalf of the world simulation, not agent reasoning. They continue accessing `World` directly. This is correct — action handlers validate against authoritative state.
3. **Affordance queries**: Already use `&dyn BeliefView` — concrete type changes transparently.

After migration, `OmniscientBeliefView` struct and `OmniscientBeliefRuntime` are deleted entirely.

### PerceptionProfile Component

Per Principle 20 (Agent Diversity) and CLAUDE.md spec drafting rules (profile-driven parameters):

```
PerceptionProfile:
  memory_capacity: u32            // max entities retained in belief store
  memory_retention_ticks: u32     // ticks before old observations may be forgotten
  observation_fidelity: Permille  // probability of noticing a co-located event (1000 = always)
```

Registered for `EntityKind::Agent` in `ComponentTables`.

### Social Evidence Boundary

E14 owns the belief-side capture of social evidence, not the loyalty model itself.

Concrete `SocialObservation` type stored in `AgentBeliefStore.social_observations`:

```
SocialObservation:
  kind: SocialObservationKind
  subjects: (EntityId, EntityId)  // the agents involved
  place: EntityId
  observed_tick: Tick
  source: PerceptionSource

SocialObservationKind:
  WitnessedCooperation
  WitnessedConflict
  WitnessedObligation
  CoPresence
```

E14 captures these observations through the same perception system. E15/E16 consume them. No special "social evidence API."

E14 must not introduce belief APIs that expose or depend on omniscient scalar loyalty truth. The concrete replacement of `LoyalTo.strength` belongs to the later social/institutional work, which will consume these concrete records instead of adding another abstract score.

### Human Agent Belief Policy

All agents accumulate beliefs through the same perception system regardless of `ControlSource`. The `AgentBeliefStore` component is added to all agents. The CLI may present both believed state (what the agent thinks) and an omniscient debug view, clearly labeled.

### Default Ignorance Policy

When `PerAgentBeliefView` is queried about an entity the agent has never observed:
- **Unknown entities**: Do not appear in the planning snapshot. The agent doesn't know they exist.
- **Self-queries**: Always authoritative (agent knows its own hunger, wounds, inventory, profiles).
- **Topology**: Treat as known (place graph is public infrastructure — agents know where places are and how to travel between them).
- **Others' profiles**: Unknown unless observed. Candidate generation and plan search must be robust to sparse snapshots.

### DemandMemory Coexistence

`DemandMemory` (trade domain) remains a separate component. It stores structured trade-specific data (commodity, quantity, place, counterparty, reason) that does not fit the general `BelievedEntityState` schema. The `BeliefView::demand_memory()` method continues reading from this component. General perception handles entity state; domain-specific memories remain as separate components.

### Removal of FactId/KnowsFact/BelievesFact Scaffolding

The existing `FactId`/`KnowsFact`/`BelievesFact` infrastructure in `worldwake-core` is unused scaffolding (zero usage in AI golden tests, no production code path calls it). E14 removes it per Principle 26 (No Backward Compatibility). Files affected:
- `crates/worldwake-core/src/ids.rs` — remove `FactId`
- `crates/worldwake-core/src/delta.rs` — remove `RelationKind::KnowsFact`, `RelationKind::BelievesFact`, `RelationValue::KnowsFact`, `RelationValue::BelievesFact`
- `crates/worldwake-core/src/relations.rs` — remove `knows_fact`, `believes_fact` fields from `RelationTables`
- `crates/worldwake-core/src/world/social.rs` — remove `add_known_fact`, `remove_known_fact`, `known_facts`, `add_believed_fact`, `remove_believed_fact`, `believed_facts`
- `crates/worldwake-core/src/world_txn.rs` — remove corresponding txn methods
- `crates/worldwake-core/src/verification.rs` — remove fact relation verification
- `crates/worldwake-core/src/event_record.rs` — remove fact-related test fixtures
- `crates/worldwake-core/src/world.rs` — remove fact-related tests

Debuggability (Principle 27) is satisfied by: `BelievedEntityState.observed_tick` + `BelievedEntityState.source` + the append-only event log with `WitnessData`.

## World/Belief Separation Enforcement
- Planner (E13) can only query beliefs through `&dyn BeliefView`
- `PerAgentBeliefView` implements `BeliefView` without holding `&World` reference for observed entity queries (self-queries and topology may read World)
- Compile-time enforcement: AI crate depends on `worldwake-sim` (which defines `BeliefView` trait), not on `World` methods directly for agent reasoning
- After E14, no code path in `worldwake-ai` constructs or uses `OmniscientBeliefView`

## SystemFn Integration

`SystemId::Perception` already exists in the canonical manifest (`system_manifest.rs`), running after `FacilityQueue` and before `Politics`. E14 implements the system function:

```rust
fn perception_system(ctx: SystemExecutionContext<'_>) -> Result<(), SystemError>
```

**Execution order within the tick** (from `system_manifest.rs` comments):
1. Needs → Production → Trade → Combat → FacilityQueue
2. **Perception** — processes all events emitted this tick, resolves witnesses per `VisibilitySpec` and co-location, updates each witness agent's `AgentBeliefStore`
3. Politics (future)

This ensures agents perceive current-tick events before AI input production runs.

## Component Registration

New components to register in `ComponentTables` via the `component_schema` macro:
- `AgentBeliefStore` — registered for `EntityKind::Agent`
- `PerceptionProfile` — registered for `EntityKind::Agent`

Follow the existing macro pattern in `component_schema.rs`.

## Cross-System Interactions (Principle 24)

All interactions are state-mediated — no direct system calls:

- **E09 Needs**: Writes `HomeostaticNeeds` changes. Perception system allows agents to observe other agents' visible deprivation state. No direct interaction.
- **E10 Production**: Resource regeneration creates events. Perception makes co-located agents aware of resource changes.
- **E11 Trade**: Trade actions create events. `DemandMemory` remains a separate domain-specific component (different concern from general perception). Perception updates location/inventory beliefs used in trade planning.
- **E12 Combat**: Combat events are perceived by co-located agents. `PerAgentBeliefView.visible_hostiles_for()` returns only hostiles the agent has perceived.
- **E13 Decision Architecture**: AI crate consumes `&dyn BeliefView`. After E14, receives `PerAgentBeliefView` instead of `OmniscientBeliefView`. No `BeliefView` trait changes required.

## Invariants Enforced
- 9.11: World/belief separation — agents react only to perceived, inferred, remembered, or told facts
- 9.15: Off-camera continuity — perception works regardless of any camera/visibility concept

## Tests
- [ ] T10: Belief isolation — agent does not react to unseen theft, death, or camp migration
- [ ] Direct perception: agent at same place learns about events there
- [ ] Agent at different place does not learn about events
- [ ] Direct observation creates `BelievedEntityState` with `source: DirectObservation`
- [ ] `PerceptionProfile.observation_fidelity` affects whether agent notices co-located event
- [ ] `PerceptionProfile.memory_capacity` bounds belief store size
- [ ] Social observations recorded for co-located cooperation/conflict events
- [ ] Stale beliefs not auto-updated (agent believes old info until re-observed)
- [ ] `PerAgentBeliefView` implements `BeliefView` without holding `&World` reference for observed-entity queries
- [ ] Belief conflict resolution: newer supersedes older
- [ ] `FactId`/`KnowsFact`/`BelievesFact` scaffolding fully removed — no compilation references remain

## Acceptance Criteria
- Perception based on co-location, not omniscience
- State-snapshot belief model with `AgentBeliefStore` per agent
- `PerceptionProfile` provides per-agent diversity in perception parameters
- Hard separation between world state and agent beliefs
- Planner API enforces belief-only access via `PerAgentBeliefView`
- `OmniscientBeliefView` fully removed — not wrapped, not retained
- Social observations captured for E15/E16 consumption

## FND-01 Section B — Deferred Information Pipeline Requirements

The following requirements were identified in `specs/FND-01-phase1-foundations-alignment.md` Section B and MUST be satisfied by this epic's implementation:

1. **Time-per-hop propagation**: Any information propagation beyond direct perception MUST consume time per hop through the place graph. Instant multi-hop information spread is forbidden (Principle 7).
2. **PublicRecord = consultable-at-location**: A `PublicRecord` means "a record exists at a place/entity and can be physically consulted." It does NOT mean "this becomes globally known." Agents must travel to the record's location or receive a report from someone who did.
3. **AdjacentPlaces = immediate spillover only**: Adjacent-place perception MUST be limited to immediate physical spillover (e.g., loud sounds, visible fires) and MUST NOT serve as free multi-hop information spread.
4. **Belief traceability**: Every agent belief MUST be traceable to one of: direct witness observation, report from a co-located agent, record consultation at a location, or prior belief state. Untraceable beliefs are forbidden.

These constraints derive from Principle 7 (Locality of Interaction and Information) in `docs/FOUNDATIONS.md`.

## FND-01 Section H — Foundations Analysis

### H1. Information-Path Analysis
- Direct perception path: event occurs at a place → co-located witness has `PerceptionProfile.observation_fidelity` checked → on success, `BelievedEntityState` entries updated in that agent's `AgentBeliefStore`.
- Report / rumor path (E15 scope): witness or record-holder shares information through co-location → listener stores a new `BelievedEntityState` with `source: Report { from, chain_len }` and reduced derived confidence.
- Record consultation path: agent physically reaches the record location → consults the record → adds or refreshes the related `BelievedEntityState`.
- Re-observation path: later direct perception updates the same entity entry with fresher observation metadata.

### H2. Positive-Feedback Analysis
- Information → action → new information: more perceived facts can cause more travel, reporting, investigation, or conflict, which in turn creates more observable events.
- Public discovery cascades: a visible event or consulted record can cause several nearby agents to act, creating secondary witness chains.

### H3. Concrete Dampeners
- **Travel time**: To re-observe, the agent must physically travel to the location, consuming ticks and occupying the body (Principle 8 — concrete world constraints limit information acquisition rate).
- **Occupancy**: Acting on beliefs requires actions with duration and preconditions — the agent can only do one thing at a time, preventing simultaneous cascading reactions.
- **Memory capacity**: `PerceptionProfile.memory_capacity` limits entities retained in the belief store, forcing forgetting of old information and limiting cascading inference chains.
- **Report action cost**: Sharing information with another agent will require a Tell action (E15) that has duration and co-location preconditions, limiting information spread rate through physical world constraints.

### H4. Stored State vs. Derived Read-Models
- Stored authoritative state:
  - Per-agent `AgentBeliefStore` with `BelievedEntityState` entries per observed entity
  - `PerceptionSource` enum stored with each belief entry (DirectObservation, Report, Rumor, Inference)
  - `observed_tick: Tick` stored with each belief entry
  - `PerceptionProfile` component per agent
  - `SocialObservation` entries in `AgentBeliefStore.social_observations`
- Derived read-models:
  - Confidence: derived at query time from `PerceptionSource` variant + staleness
  - Staleness: derived from `current_tick - observed_tick`
  - `PerAgentBeliefView` methods: assembled from stored belief entries and authoritative self/topology data
  - Any later social summaries derived from concrete `SocialObservation` records rather than stored as separate truth

## Spec References
- Section 3.5 (separate world truth from agent belief)
- Section 6.3 (planner uses believed facts only)
- Section 7.3 (informational propagation channel)
- Section 9.11 (world/belief separation)
- `specs/FND-01-phase1-foundations-alignment.md` Section B (deferred requirements)
- `specs/E15-rumor-witness-discovery.md` (confidence propagation and discovery follow-on)
- `specs/E16-offices-succession-factions.md` (later consumer of belief-side social evidence)
- `docs/FOUNDATIONS.md` (13 foundational principles)
