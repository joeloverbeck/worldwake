# E16: Offices, Succession & Factions

## Epic Summary
Implement offices with succession law, factions with loyalty, coercion/bribery, and public order derived from institutional state.

## Phase
Phase 3: Information & Politics

## Crate
`worldwake-systems`

## Dependencies
- E14 (beliefs needed for loyalty decisions and succession awareness)

## Deliverables

### Office Component
- `Office` entity with:
  - `title: String` (e.g., "Village Ruler", "Guard Captain")
  - `holder: Option<EntityId>` (current holder, None if vacant)
  - `eligibility: Vec<EligibilityRule>` (who can hold this office)
  - `succession_law: SuccessionLaw`

### Office Vacancy
- When holder dies → office becomes vacant
- Vacancy event emitted (public, all at place learn immediately, others through rumor)
- Vacancy triggers succession process

### Succession Law
Per spec section 3.9:
- `SuccessionLaw` enum:
  - `Hereditary`: blood relation to previous holder
  - `Support`: most supported candidate claims
  - `Force`: whoever can take and hold it
  - `Appointment`: designated by previous holder or council
- Eligibility rules: age, faction membership, blood relation, support threshold

### Faction Component
- `Faction` entity with:
  - `name: String`
  - `leader: Option<EntityId>`
  - `members: Vec<EntityId>` (via MemberOf relation)
- Loyalty scores: per-agent loyalty to faction and to specific individuals
- `LoyalTo` relation with strength value

### Support & Loyalty
- Agents choose whom to support for vacant offices
- Support based on:
  - Existing loyalty relations
  - Faction membership
  - Personal benefit (bribery)
  - Coercion (threats)
  - Beliefs about candidates

### Coercion & Bribery Actions
- **Bribe**: offer coin/goods to gain loyalty
  - Effect: increase target's loyalty toward actor
  - Requires: goods to offer, both at same place
- **Threaten**: use force/position to compel support
  - Effect: increase fear + compliance, may decrease loyalty
  - Risk: target may resist or retaliate

### Public Order Metric
- Derived from:
  - Office state: vacant = lower order, filled = stability
  - Guard presence: more guards at location = higher order
  - Crime rate: recent crimes at location decrease order
  - Faction conflict: competing factions decrease order
- `public_order(place) -> f32` (0.0 = anarchy, 1.0 = peaceful)
- Updates each tick based on current state

### Claimant Behavior (ActionDefs)
- On office vacancy:
  - Eligible agents generate ClaimOffice goal
  - Seek support from other agents
  - May use bribery or coercion
  - When sufficient support: make formal claim
  - Contested claims may lead to conflict

## Invariants Enforced
- 9.13: Office uniqueness - each office has at most one holder at a time

## Tests
- [ ] T11: Office uniqueness - succession cannot produce two simultaneous rulers
- [ ] T21: Ruler death → office vacant → claimants seek support → successor emerges
- [ ] Vacancy event emitted on holder death
- [ ] Eligibility rules filter candidates correctly
- [ ] Bribery increases loyalty (with cost)
- [ ] Public order decreases during vacancy
- [ ] Public order increases with guard presence
- [ ] Faction membership tracked via MemberOf relation
- [ ] No cutscene or scripted succession (spec section 8)

## Acceptance Criteria
- Offices with explicit succession laws
- Factions with tracked loyalty
- Coercion and bribery as real actions
- Public order derived from world state
- No scripted succession

## Spec References
- Section 3.9 (social institutions as first-class systems)
- Section 4.5 (office succession, faction loyalty/support)
- Section 7.4 (institutional propagation: vacancy, legitimacy, loyalty, enforcement)
- Section 9.13 (office uniqueness)
- Section 8 (no leader replacement cutscene)
