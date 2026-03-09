# E19: Guard & Patrol Adaptation

## Epic Summary
Implement guard patrol routes, intensity scaling, threat-based route adaptation, and the public order feedback loop.

## Phase
Phase 4: Group Adaptation, CLI & Verification

## Crate
`worldwake-systems`

## Dependencies
- E16 (public order metric, offices, factions)

## Deliverables

### Guard Patrol Routes
- `PatrolRoute` component:
  - `assigned_places: Vec<EntityId>` (places to visit in order)
  - `current_index: usize` (current position in route)
  - `patrol_interval: u32` (ticks between place visits)
- Guards follow assigned routes, spending time at each place
- **Patrol** action: travel to next place in route → observe → continue

### Patrol Intensity
- Intensity scaling based on world state:
  - During office vacancy: increase patrol frequency
  - During high crime: increase patrol frequency and duration at crime locations
  - Low threat: normal/reduced patrol frequency
- Intensity modifier: `patrol_interval = base_interval / intensity_factor`

### Route Adaptation
- Guards shift patrols based on threat intelligence:
  - Crime reported at location → add to patrol route
  - Bandit sightings on route → increase coverage
  - Area cleared of threats → reduce patrol frequency
- Adaptation uses agent's beliefs (not omniscient world state)
- Guard captain (if office exists) may issue patrol orders

### Public Order Feedback Loop
- Public order (from E16) feeds back into guard behavior:
  - Low public order → more patrols → reduces crime → order improves
  - High public order → fewer patrols → may allow crime to increase
  - Stabilizing negative feedback loop
- Loop operates through real agent decisions, not scripted

### Guard Integration with Office System
- Guards loyal to office holder (via LoyalTo relation)
- When ruler changes: guards may change patrol priorities based on new orders
- Guard captain office: if vacant, patrols may become disorganized

## Tests
- [ ] Patrols change when ruler dies (intensity increases during vacancy)
- [ ] Patrols change when crime spikes at a location
- [ ] Guard route adaptation reflects threat intelligence from beliefs
- [ ] Public order feedback loop: more patrols → less crime → higher order
- [ ] Guards follow assigned routes (visit places in order)
- [ ] Patrol intensity scales with office vacancy and crime rate
- [ ] Route adaptation uses beliefs, not world state

## Acceptance Criteria
- Guards patrol real routes through the world graph
- Patrol behavior adapts to threats and institutional state
- Public order feedback loop creates emergent stability/instability
- No scripted patrol changes

## Spec References
- Section 4.5 (core systems include guard behavior)
- Section 7.4 (institutional propagation: law enforcement, patrol intensity)
- Section 3.9 (public-order consequences)
