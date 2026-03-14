# E17: Crime, Theft & Justice

## Epic Summary
Implement theft actions, crime evidence, accusation system, guard response, and punishment actions.

## Phase
Phase 3: Information & Politics (final epic before Phase 3 gate)

## Crate
`worldwake-systems`

## Dependencies
- E15 (social information transmission — provides Discovery events for crime awareness, Tell action for witness testimony sharing)
- S01 (production output ownership claims — ownership-aware audit accuracy)
- S03 (planner target identity & affordance binding)

## Dependency Note
E15 delivers belief-mismatch Discovery events (InventoryDiscrepancy, EntityMissing, AliveStatusChanged) that serve as the crime awareness trigger in this epic. E17 owns the inventory audit action (active, deliberate audit by a merchant or guard) and the crime-specific interpretation of Discovery events — E15 provides only the passive mismatch detection foundation.

This epic assumes ownership-aware produced goods already exist (S01). Unauthorized taking of owned-but-unpossessed goods must not remain a lawful `pick_up` path, or theft will be bypassed. See [S01-production-output-ownership-claims.md](/home/joeloverbeck/projects/worldwake/specs/S01-production-output-ownership-claims.md).

## Deliverables

### Theft Action
- **Steal**: agent takes item without ownership
  - Precondition: target item accessible, agent at same place, item not reserved
  - Duration: 3-10 ticks (depending on stealth)
  - Effect: transfer possession to thief, ownership NOT transferred
  - Visibility: Hidden or Private (not automatically seen)
  - Event emitted with Hidden visibility
  - Crime tag on event

### Crime Evidence
- Physical evidence from theft:
  - Missing inventory (discovered via audit)
  - Witnesses if anyone was present (but event was Hidden → only if perception check passes)
  - Circumstantial: agent was at location during crime window
- Evidence weight scoring:
  - Witness testimony: high weight
  - Missing inventory: medium weight (proves crime, not who)
  - Circumstantial: low weight

### Accusation System
- Accusation requires evidence threshold:
  - Sum of evidence weights must exceed accusation threshold
  - Different thresholds for different severities
- `Accusation` event:
  - accuser, accused, crime_type, evidence_list
  - Triggers guard response

### Guard Response
Based on evidence and public order:
- **Investigate**: guard goes to crime location, searches for evidence
  - Duration: 20-60 ticks
  - May find additional evidence
- **Pursue**: guard tracks accused to current location
  - Requires: accusation with sufficient evidence
  - Duration: travel time to accused's location
- **Arrest**: guard confronts accused
  - May lead to surrender or combat
- Response intensity scales with:
  - Evidence strength
  - Crime severity
  - Public order level
  - Guard availability

### Punishment Actions
- **Fine**: take coin from convicted agent
  - Precondition: accused has coin, evidence threshold met
  - Effect: transfer coin to faction/office treasury
- **Imprison**: confine agent to designated location
  - Effect: agent restricted to jail/holding area for duration
  - Duration: proportional to crime severity
- **Exile**: remove agent from faction, hostile marking
  - Effect: HostileTo relation, loss of faction membership

## Invariants Enforced
- 9.17: Traceable discovery - no immediate global accusation
- 9.11: Crime awareness through information channels only

## Tests
- [ ] Theft transfers possession but not ownership
- [ ] No immediate global accusation after theft
- [ ] Crime discovered only through: witness, inventory audit, or rumor
- [ ] Guard response proportional to evidence strength
- [ ] Accusation requires evidence above threshold
- [ ] Punishment follows investigation → pursuit → arrest flow
- [ ] Hidden theft at empty location remains unknown until discovery
- [ ] Fine transfers coin (conservation maintained)
- [ ] Response intensity scales with public order

## Phase 3 Gate
After E17, verify:
- [ ] Information propagates through explicit channels
- [ ] Offices transfer through succession
- [ ] Crimes discovered through defined pathways
- [ ] No omniscient NPCs
- [ ] Causal chains from crime → discovery → response traceable

## Acceptance Criteria
- Theft as a real action with evidence generation
- Multi-step justice: evidence → accusation → investigation → punishment
- No instant crime detection
- Guard behavior driven by evidence and world state

## Spec References
- Section 4.5 (crime and theft)
- Section 7.3 (informational propagation: suspicion, discovery delays)
- Section 8 (no global omniscience for NPCs)
- Section 9.17 (traceable discovery)
