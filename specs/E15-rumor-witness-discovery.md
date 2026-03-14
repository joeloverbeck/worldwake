# E15: Social Information Transmission

## Epic Summary
Implement the Tell action for social belief transmission between co-located agents, rumor propagation with source degradation and chain tracking, per-agent information-sharing profiles, and belief-mismatch discovery events that downstream systems consume.

## Phase
Phase 3: Information & Politics

## Crate
`worldwake-systems` (action handler, perception extension), `worldwake-core` (types, components), `worldwake-sim` (action payload, domain)

## Dependencies
- E14 (perception and belief system — provides AgentBeliefStore, PerceptionSource, PerceptionProfile, perception system, WitnessData/VisibilitySpec infrastructure)

## Dependency Note
E14 already delivers the witness system, passive local observation, and the PerceptionSource::Report/Rumor type variants. E15 provides the mechanism that populates those variants.

Inventory-audit discovery and crime evidence gathering are E17 scope (depends on S01 ownership claims). E15 provides the mismatch-detection foundation that E17 consumes, not the crime-specific interpretation.

## What E14 Already Provides (Not Repeated Here)
- Witness resolution via VisibilitySpec (SamePlace, AdjacentPlaces, ParticipantsOnly)
- Passive local observation via observe_passive_local_entities()
- AgentBeliefStore with BelievedEntityState snapshots
- PerceptionSource enum with DirectObservation, Report { from, chain_len }, Rumor { chain_len }, Inference
- PerceptionProfile with memory_capacity, memory_retention_ticks, observation_fidelity
- Confidence derived at query time from PerceptionSource variant + staleness (not stored)
- Body/entity discovery through passive perception (arrive at location, see what is there)

## Deliverables

### 1. Tell Action

The Tell action is the explicit social transmission mechanism for beliefs. It follows the established ActionDef pattern.

**Action definition:**
- Name: `tell`
- Domain: `ActionDomain::Social` (new variant)
- Preconditions:
  - Actor alive
  - Listener (target 0) exists, is an Agent, is at actor's place, is alive
  - Actor has a belief about the subject entity in their AgentBeliefStore
  - Actor's TellProfile.max_relay_chain_len permits relaying this belief's chain depth
- Duration: 2 ticks (fixed)
- Body cost per tick: zero
- Interruptibility: FreelyInterruptible
- Commit conditions: Actor alive, listener still at actor's place
- Visibility: VisibilitySpec::SamePlace (bystanders see the social act)
- Event tags: Social, WorldMutation

**Payload:**
```rust
pub struct TellActionPayload {
    pub listener: EntityId,
    pub subject_entity: EntityId,
}
```

**Handler semantics (commit):**
1. Speaker retrieves their BelievedEntityState for subject_entity from own AgentBeliefStore.
2. If speaker has no belief about the subject, commit aborts cleanly.
3. Compute listener's source from speaker's source:
   - Speaker DirectObservation -> listener gets Report { from: speaker, chain_len: 1 }
   - Speaker Report { from, chain_len: n } -> listener gets Rumor { chain_len: n + 1 }
   - Speaker Rumor { chain_len: n } -> listener gets Rumor { chain_len: n + 1 }
   - Speaker Inference -> listener gets Rumor { chain_len: 1 }
4. Check listener's TellProfile.acceptance_fidelity via RNG — if check fails, belief is not written (listener is skeptical).
5. Build the transferred BelievedEntityState with:
   - Same fields as speaker's belief (last_known_place, last_known_inventory, etc.)
   - observed_tick = speaker's observed_tick (preserves staleness, NOT current tick)
   - source = computed source from step 3
6. Call listener's AgentBeliefStore.update_entity() — newer-wins rule applies (listener keeps their belief if it's more recent).
7. Call listener's enforce_capacity() with their PerceptionProfile.
8. Emit commit event with VisibilitySpec::SamePlace. Bystanders observe the social act but do NOT receive the belief content — only the listener does.

**Affordance enumeration:**
Speaker's AgentBeliefStore.known_entities keys provide subjects they can tell about, filtered by TellProfile.max_relay_chain_len. For each co-located alive agent target and each eligible subject, a TellActionPayload is generated. Bounded by TellProfile.max_tell_candidates (default 3, selecting most recently observed beliefs) to avoid combinatorial explosion.

### 2. TellProfile Component

Per-agent profile controlling information sharing and acceptance behavior. Enables agent diversity per Principle 20.

```rust
pub struct TellProfile {
    /// Maximum number of belief subjects offered as Tell affordances per decision pass.
    pub max_tell_candidates: u8,
    /// Maximum chain_len this agent will relay. Beliefs with deeper chains are not shared.
    pub max_relay_chain_len: u8,
    /// Probability (Permille) that this agent accepts a told belief. Checked via RNG on receive.
    pub acceptance_fidelity: Permille,
}
impl Component for TellProfile {}
```

Default values:
- max_tell_candidates: 3
- max_relay_chain_len: 3
- acceptance_fidelity: Permille(800)

Register on EntityKind::Agent in component schema.

### 3. Belief Mismatch Discovery Events

When the perception system updates a belief and detects material mismatch between the prior belief and the new observation, it emits a Discovery event into the append-only event log. This is the Principle 15 foundation: surprise comes from violated expectation.

**New EventTag variant:** `Discovery`

**New event evidence type:**
```rust
pub enum MismatchKind {
    EntityMissing,
    AliveStatusChanged,
    InventoryDiscrepancy {
        commodity: CommodityKind,
        believed: Quantity,
        observed: Quantity,
    },
    PlaceChanged {
        believed_place: EntityId,
        observed_place: EntityId,
    },
}
```

**Integration in perception system:**
During `observe_passive_local_entities()`, before overwriting a belief via `update_entity()`, compare the prior BelievedEntityState with the new snapshot. If material differences exist, emit a Discovery event with:
- actor_id = the observing agent
- place_id = observation place
- tags = {Discovery, WorldMutation}
- visibility = VisibilitySpec::ParticipantsOnly (private to the observer)
- witness_data with observer as direct witness
- Mismatch details carried in event evidence or a new evidence variant

Also during event-based perception: if the perception system updates beliefs from an event and detects material changes vs prior beliefs, the same mismatch emission applies.

**EntityMissing detection:** After passive observation completes at a place, for each belief where the agent expected an entity at this place (prior belief `last_known_place == current_place`) but the entity was NOT observed, emit an EntityMissing discovery. This requires the observer to have been at the place long enough for passive observation to run (which it does every tick).

**What consumes Discovery events:**
- E17 (Crime, Theft & Justice): InventoryDiscrepancy + EntityMissing trigger crime suspicion and investigation goals
- E16 (Offices): AliveStatusChanged can trigger succession awareness
- AI candidate generation: Discovery events can generate investigation or reporting goals

### 4. Social EventTag and ActionDomain

**New EventTag variant:** `Social` — tags Tell action events and future social interaction events.

**New ActionDomain variant:** `Social` — categorizes Tell and future social actions (bribe, threaten from E16 will also use this domain).

**New SocialObservationKind variant:** `WitnessedTelling` — recorded by bystanders who observe a Tell action commit at the same place. Enables social modeling of "who talks to whom."

### 5. Confidence Derivation (Clarification, Not New Code)

Confidence is NOT stored as a field. It is derived at query time from:
- PerceptionSource variant: DirectObservation > Report (chain_len 1) > Rumor (chain_len 1) > deeper chains
- Staleness: current_tick - observed_tick

This is an existing E14 design decision (Principle 3 compliance). E15 does not change this. The Tell action's source degradation logic (DirectObservation -> Report -> Rumor -> deeper Rumor) naturally produces the right confidence ordering without storing abstract scores.

If the AI planner or downstream systems need a concrete confidence derivation function, it should be added as a pure function:
```rust
pub fn belief_confidence(source: &PerceptionSource, staleness_ticks: u64) -> Permille { ... }
```
This is a derived read-model, not authoritative state.

## Invariants Enforced
- Principle 7 (Locality): Tell requires co-location. Information travels only through physical carriers (agents moving between places).
- Principle 8 (Action semantics): Tell has preconditions, duration, cost, occupancy, and interruptibility.
- Principle 10 (Physical dampeners): Rumor spread dampened by travel time, action duration, memory capacity, memory retention, chain length filtering, acceptance fidelity, and action slot occupancy. No numerical clamps.
- Principle 12 (World != Belief): PerceptionSource tracking maintained through Tell chains.
- Principle 13 (Knowledge travels physically): Tell is the explicit carrier mechanism.
- Principle 14 (Ignorance/contradiction first-class): Agents can hold conflicting beliefs from different Tell chains. Newer-wins prevents same-source conflicts but different-source conflicts remain.
- Principle 15 (Violated expectation): Belief mismatch detection provides the foundation.
- Principle 24 (Systems through state): Tell commits write to AgentBeliefStore, discovery emits events. No cross-system calls.

## FND-01 Section H

### Information-Path Analysis
- **Tell path**: Speaker has belief (from DirectObservation, prior Report, or Rumor) -> speaker and listener are co-located -> Tell action commits -> listener's AgentBeliefStore updated with degraded PerceptionSource. Every belief has a `source` field tracing its provenance.
- **Mismatch path**: Agent has prior belief (any source) -> agent passively observes local entities via perception system -> belief differs from observation -> Discovery event emitted into event log. The event is local to the observer (ParticipantsOnly visibility).
- **No information teleportation**: An agent at Place A cannot learn about events at Place B unless another agent physically travels from B to A and executes a Tell action. Travel time IS the information propagation delay.
- **PublicRecord events**: Not affected. Tell is a private social interaction with SamePlace visibility for bystanders.

### Positive-Feedback Analysis
- **Information amplification**: Agent A tells B, B tells C, C tells D. In dense populations, information can spread rapidly through Tell chains.
- **Mismatch cascade**: Agent discovers mismatch -> tells others -> others investigate -> more discoveries -> more telling. This is the "news travels fast" amplification loop.

### Concrete Dampeners
1. **Travel time**: Information can only spread as fast as agents physically move between places via the topology graph.
2. **Tell action duration**: 2 ticks per Tell, occupying both speaker and listener. Cannot Tell everyone simultaneously.
3. **Memory capacity**: PerceptionProfile.memory_capacity limits beliefs held. Old beliefs evicted.
4. **Memory retention**: PerceptionProfile.memory_retention_ticks causes beliefs to expire. Rumors cannot propagate indefinitely.
5. **Chain length filtering**: TellProfile.max_relay_chain_len prevents deep chains. An agent with max_relay_chain_len=2 refuses to relay third-hand rumors.
6. **Acceptance fidelity**: TellProfile.acceptance_fidelity acts as a probabilistic filter on incoming rumors.
7. **Action slot occupancy**: An agent telling rumors cannot simultaneously eat, trade, craft, or travel.
8. **Competing priorities**: AI goal ranking weighs Tell against survival needs. Starving agents do not gossip.

### Stored State vs Derived State
**Stored authoritative state:**
- AgentBeliefStore (per-agent, existing)
- PerceptionProfile (per-agent, existing)
- TellProfile (per-agent, new)
- BelievedEntityState.source: PerceptionSource (per-belief, existing)
- Event log records of Tell commits and Discovery events

**Derived transient state (never stored as authoritative):**
- Confidence/reliability of a belief — derived from PerceptionSource + staleness
- Whether an agent "should" tell something — derived from planner goal ranking
- Rumor "freshness" — derived from observed_tick vs current_tick
- Mismatch significance — derived by comparing prior belief to new observation

## Component Registration
- `TellProfile` on `EntityKind::Agent` in component_tables/component_schema

No new components for mismatch detection — mismatches flow through the event log.

## SystemFn Integration

### worldwake-core
- Add `TellProfile` component type with Default impl
- Add `MismatchKind` enum for Discovery event evidence
- Add `EventTag::Social` and `EventTag::Discovery` variants
- Add `SocialObservationKind::WitnessedTelling` variant
- Add `ActionDomain::Social` variant (if ActionDomain is in core; otherwise in sim)
- Add `belief_confidence()` derivation helper (pure function)
- Register `TellProfile` in component schema for EntityKind::Agent

### worldwake-sim
- Add `ActionDomain::Social` variant (if not in core)
- Add `Tell(TellActionPayload)` to `ActionPayload` enum
- Add `TellActionPayload` struct
- Register Tell ActionDef in action_def_registry

### worldwake-systems
- New module: `tell_actions.rs` — Tell ActionDef, handler (start/tick/commit/abort), affordance enumeration
- Extend `perception.rs` — Add mismatch detection in observe_passive_local_entities() and event-based perception; emit Discovery events
- Update `social_kind()` in perception.rs to recognize Social event tag for WitnessedTelling
- Register tell action handler in action_handler_registry
- Export tell_actions module in lib.rs

### worldwake-ai
- No immediate changes required. Future work: Tell goal generation (agent decides to share information) and mismatch-driven investigation goal generation. These can be deferred to E17 or a later ticket.

## Cross-System Interactions (Principle 24)
- Perception writes beliefs into AgentBeliefStore, emits Discovery events into event log
- Tell action reads speaker's beliefs, writes to listener's beliefs, emits social events
- AI reads beliefs and events to generate goals (future work)
- E17 crime system reads Discovery events to trigger investigation (future dependency)
- E16 offices reads AliveStatusChanged discoveries for succession triggers (future dependency)

No direct system-to-system calls. All influence through state (beliefs, events, components).

## Tests
- [ ] Tell action registered with Social domain and correct preconditions
- [ ] Tell transmits belief: speaker DirectObservation -> listener Report { from: speaker, chain_len: 1 }
- [ ] Tell chain degrades: speaker Report { chain_len: n } -> listener Rumor { chain_len: n+1 }
- [ ] Tell from Rumor degrades: speaker Rumor { chain_len: n } -> listener Rumor { chain_len: n+1 }
- [ ] Tell from Inference: listener gets Rumor { chain_len: 1 }
- [ ] Tell aborts if speaker has no belief about subject
- [ ] Tell requires co-location: fails if listener moves mid-action
- [ ] Tell preserves observed_tick (speaker's original, NOT current tick)
- [ ] Newer-wins: listener keeps more recent belief, Tell does not overwrite
- [ ] Memory capacity enforced after Tell
- [ ] TellProfile.max_relay_chain_len filters: speaker with max_relay_chain_len=1 does not relay Rumor beliefs
- [ ] TellProfile.acceptance_fidelity: listener with acceptance_fidelity=0 never accepts told beliefs
- [ ] Bystanders observe WitnessedTelling social observation
- [ ] Bystanders do NOT receive the belief content
- [ ] Discovery event emitted on AliveStatusChanged (believed alive, observed dead)
- [ ] Discovery event emitted on InventoryDiscrepancy (believed N, observed M)
- [ ] Discovery event emitted on EntityMissing (believed at place, not found)
- [ ] No Discovery event when agent has no prior belief (first observation)
- [ ] T25: Hidden event at empty location, no witnesses, remote agent remains ignorant
- [ ] Information propagation delay: crime at tick 100, witness travels, tells at tick 112+
- [ ] Deterministic replay: Tell actions and Discovery events reproduce identically

## Acceptance Criteria
- Information flows through explicit channels only (Tell action + perception)
- Tell requires physical co-location
- Source degrades through Report -> Rumor -> deeper Rumor chains
- No confidence score stored — derived from PerceptionSource + staleness
- Discovery events emitted on belief mismatch, consumed by downstream systems
- No omniscient information access
- All belief sources traceable through PerceptionSource
- Per-agent diversity in information sharing/acceptance via TellProfile
- Physical dampeners limit rumor spread (no numerical clamps)

## Spec References
- Section 3.5 (beliefs from perception, memory, reports, rumors)
- Section 7.3 (informational propagation: witnessing, rumor, suspicion, discovery delays)
- Section 9.17 (traceable discovery)
- Section 8 (no global omniscience for NPCs)
- docs/FOUNDATIONS.md Principles 3, 7, 8, 10, 12, 13, 14, 15, 24, 28
