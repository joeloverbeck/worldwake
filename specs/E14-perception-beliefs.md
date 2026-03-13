# E14: Perception & Belief System

## Epic Summary
Implement visibility rules, direct perception, memory, belief staleness, and world/belief separation enforcement.

## Phase
Phase 3: Information & Politics

## Crate
`worldwake-systems`

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
  - Modified by event's Visibility setting (Public, SemiPublic, Private, Hidden)

### Direct Perception
- When an event occurs at an agent's location:
  - Agent automatically becomes a witness
  - Facts from the event are added to agent's knowledge
  - `KnowsFact` relation created with observation tick
- Perception is passive (agents don't choose what to notice at their location)

### Memory System
- `Memory` component per agent:
  - `known_facts: BTreeMap<FactId, PerceivedFact>`
  - Each fact records: what, where, when observed, source (direct/rumor)
- `PerceivedFact` struct:
  - `fact_type: FactType` (location of entity, stock level, ownership, death, crime, etc.)
  - `observed_tick: Tick`
  - `source: PerceptionSource` (DirectObservation, Rumor, Report, Inference)
  - `confidence: Permille` (`Permille(1000)` for direct observation; lower values for indirect sources such as reports and rumors)

### Belief Staleness
- Beliefs can become outdated:
  - Agent observed apple stock at tick 100, it's now tick 500
  - Staleness = current_tick - observed_tick
  - Stale beliefs still used for planning (agents don't know they're outdated)
  - Beliefs updated only when agent re-observes or receives new information

### KnowsFact / BelievesFact Management
- `KnowsFact(agent, fact)`: agent has direct knowledge
- `BelievesFact(agent, fact)`: agent believes (may be from rumor, may be stale)
- All facts are BelievesFact; KnowsFact is a subset with direct evidence
- Belief conflict resolution: newer information supersedes older

### World/Belief Separation Enforcement
- Planner (E13) can only query: `get_beliefs(agent) -> &Beliefs`
- No function allows planner to access raw `World` state
- API enforcement: belief query functions separate from world query functions
- Compile-time or runtime guard: planner context has no reference to World

### Social Evidence Boundary
- E14 owns the belief-side capture of social evidence, not the loyalty model itself.
- The system must preserve belief-traceable records for later social reasoning, including:
  - witnessed cooperation or conflict
  - fulfilled or broken obligations
  - public records, reports, and testimony
  - co-presence or shared travel history when it matters to later social inference
- E14 must not introduce belief APIs that expose or depend on omniscient scalar loyalty truth.
- The concrete replacement of `LoyalTo.strength` belongs to the later social/institutional work, which will consume these concrete records instead of adding another abstract score.

## Invariants Enforced
- 9.11: World/belief separation - agents react only to perceived, inferred, remembered, or told facts
- 9.15: Off-camera continuity - perception works regardless of any camera/visibility concept

## Tests
- [ ] T10: Belief isolation - agent does not react to unseen theft, death, or camp migration
- [ ] Direct perception: agent at same place learns about events there
- [ ] Agent at different place does not learn about events
- [ ] Memory records observation tick correctly
- [ ] Stale beliefs not auto-updated (agent believes old info until re-observed)
- [ ] Planner cannot access World state directly (API enforcement)
- [ ] Confidence degrades for indirect sources
- [ ] Belief conflict resolution: newer supersedes older

## Acceptance Criteria
- Perception based on co-location, not omniscience
- Memory system with staleness tracking
- Hard separation between world state and agent beliefs
- Planner API enforces belief-only access
- `OmniscientBeliefView` is a temporary stand-in and must be fully removed by the E14 implementation rather than wrapped indefinitely

## FND-01 Section B — Deferred Information Pipeline Requirements

The following requirements were identified in `specs/FND-01-phase1-foundations-alignment.md` Section B and MUST be satisfied by this epic's implementation:

1. **Time-per-hop propagation**: Any information propagation beyond direct perception MUST consume time per hop through the place graph. Instant multi-hop information spread is forbidden (Principle 7).
2. **PublicRecord = consultable-at-location**: A `PublicRecord` means "a record exists at a place/entity and can be physically consulted." It does NOT mean "this becomes globally known." Agents must travel to the record's location or receive a report from someone who did.
3. **AdjacentPlaces = immediate spillover only**: Adjacent-place perception MUST be limited to immediate physical spillover (e.g., loud sounds, visible fires) and MUST NOT serve as free multi-hop information spread.
4. **Belief traceability**: Every agent belief MUST be traceable to one of: direct witness observation, report from a co-located agent, record consultation at a location, or prior belief state. Untraceable beliefs are forbidden.

These constraints derive from Principle 7 (Locality of Interaction and Information) in `docs/FOUNDATIONS.md`.

## FND-01 Section H — Foundations Analysis

### H1. Information-Path Analysis
- Direct perception path: event occurs at a place -> co-located witness records a `PerceivedFact` -> fact enters that agent's `Memory`.
- Report / rumor path: witness or record-holder shares information through co-location or consultation at a place -> listener stores a new `PerceivedFact` with source metadata and reduced `Permille` confidence.
- Record consultation path: agent physically reaches the record location -> consults the record -> adds or refreshes the related `PerceivedFact`.
- Re-observation path: later direct perception updates the same fact entry with fresher observation metadata.

### H2. Positive-Feedback Analysis
- Information -> action -> new information: more perceived facts can cause more travel, reporting, investigation, or conflict, which in turn creates more observable events.
- Public discovery cascades: a visible event or consulted record can cause several nearby agents to act, creating secondary witness chains.

### H3. Concrete Dampeners
- Locality and travel time limit how quickly information can propagate between places.
- Co-location, consultation, and action durations occupy time and prevent free global spread.
- Belief staleness and source confidence limit how strongly indirect information should drive later behavior.
- Records, witnesses, and aftermath must exist at concrete places; no hidden manager can inject beliefs globally.

### H4. Stored State vs. Derived Read-Models
- Stored authoritative state:
  - per-agent `Memory` / fact entries
  - witness records and consulted records
  - source metadata, observation tick, and `Permille` confidence stored with perceived facts
- Derived read-models:
  - staleness queries derived from `current_tick - observed_tick`
  - confidence interpretation and filtering views
  - planner-facing belief queries assembled from stored fact records
  - any later social summaries derived from concrete witness/report/record history rather than stored as separate truth

## Spec References
- Section 3.5 (separate world truth from agent belief)
- Section 5.4 (knows_fact, believes_fact relations)
- Section 6.3 (planner uses believed facts only)
- Section 7.3 (informational propagation channel)
- Section 9.11 (world/belief separation)
- `specs/FND-01-phase1-foundations-alignment.md` Section B (deferred requirements)
- `specs/E15-rumor-witness-discovery.md` (confidence propagation and discovery follow-on)
- `specs/E16-offices-succession-factions.md` (later consumer of belief-side social evidence)
