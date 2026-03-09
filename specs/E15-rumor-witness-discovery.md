# E15: Rumor, Witness & Discovery

## Epic Summary
Implement the witness system, rumor creation and propagation, information reliability, and discovery pathways for crimes and events.

## Phase
Phase 3: Information & Politics

## Crate
`worldwake-systems`

## Dependencies
- E14 (perception and belief system)

## Deliverables

### Witness System
- Events emit witness data (from E06 WitnessData)
- Agents present at event location automatically become witnesses
- Witness status recorded: who saw what, when, with what clarity
- Witness reliability: direct witnesses have confidence 1.0

### Rumor Creation
- Witness shares information via social interaction:
  - **Tell** action: agent at same place as another agent → share a known fact
  - Precondition: both agents at same place, speaker knows the fact
  - Effect: listener gains BelievesFact with source=Rumor, confidence < 1.0
  - Duration: 1-3 ticks

### Rumor Propagation
- Rumors spread through contact:
  - Agents at same place may exchange information
  - Passive spread: during social interactions, facts naturally shared
  - Active spread: agent chooses to tell specific fact
- Propagation chain tracked: original source → intermediaries
- Each retelling may degrade confidence

### Information Reliability
- Confidence scoring by source:
  - DirectObservation: 1.0
  - FirstHandRumor: 0.7-0.8
  - SecondHandRumor: 0.4-0.6
  - ThirdHand+: 0.2-0.3
- Conflicting rumors: agent may hold contradictory beliefs
  - Resolution: prefer higher confidence, more recent
- Rumor accuracy: rumors faithfully transmit the believed fact (no telephone game distortion in v0)

### Discovery Pathways
Per spec section 9.17, things become known through explicit channels:

- **Inventory Audit**: merchant checks stock → discovers shortage/theft
  - Triggered periodically or when trade fails
  - Produces discovery event with what's missing

- **Body Discovery**: agent arrives at location with corpse → discovers death
  - Produces discovery event
  - Triggers investigation behavior

- **Crime Evidence**: physical traces of crimes
  - Missing items (inventory audit reveals discrepancy)
  - Witness testimony (someone saw the crime)
  - Circumstantial evidence (agent was at location at crime time)

### Discovery Delay
- Crimes not known until discovered
- Time between crime and discovery depends on:
  - How often the affected area is visited
  - Whether witnesses exist
  - How quickly inventory is audited
- No instant global notification

## Invariants Enforced
- 9.17: Traceable discovery - crimes, deaths, shortages become known through explicit channels
- 9.11: World/belief separation maintained through rumor system

## Tests
- [ ] T25: Unseen crime - no immediate global accusation, suspicion only after discovery
- [ ] Witnesses automatically created for events at their location
- [ ] Rumor transmission: speaker knows fact → listener gains belief
- [ ] Confidence degrades with each retelling
- [ ] Inventory audit discovers theft after delay
- [ ] Body discovery triggers investigation events
- [ ] Agents without information channels remain ignorant
- [ ] Discovery delay: crime at tick 100, discovery at tick 200+ depending on visits
- [ ] Rumor propagation chain tracked

## Acceptance Criteria
- Information flows through explicit channels only
- Discovery requires physical presence or communication
- Rumor confidence degrades realistically
- No omniscient crime detection
- All information sources traceable

## Spec References
- Section 3.5 (beliefs from perception, memory, reports, rumors)
- Section 7.3 (informational propagation: witnessing, rumor, suspicion, discovery delays)
- Section 9.17 (traceable discovery)
- Section 8 (no global omniscience for NPCs)
