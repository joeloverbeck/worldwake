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
  - `known_facts: HashMap<FactId, PerceivedFact>`
  - Each fact records: what, where, when observed, source (direct/rumor)
- `PerceivedFact` struct:
  - `fact_type: FactType` (location of entity, stock level, ownership, death, crime, etc.)
  - `observed_tick: Tick`
  - `source: PerceptionSource` (DirectObservation, Rumor, Report, Inference)
  - `confidence: f32` (1.0 for direct, degrades for rumors)

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

## Spec References
- Section 3.5 (separate world truth from agent belief)
- Section 5.4 (knows_fact, believes_fact relations)
- Section 6.3 (planner uses believed facts only)
- Section 7.3 (informational propagation channel)
- Section 9.11 (world/belief separation)
