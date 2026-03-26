# E19: Guard & Patrol Adaptation

## Epic Summary
Implement guard patrol routes, intensity scaling, threat-based route adaptation, and the public order feedback loop.

## Phase
Phase 4: Group Adaptation, CLI & Verification

## Crate
`worldwake-systems`

## Dependencies
- E16 (public order metric, offices, factions)
- E16b (contested-office control state for coup response and patrol escalation)
- E16c (institutional beliefs & record consultation for CrimeRegister access)
- E17 (crime mechanics -- Steal action, CrimeRegister accusations/verdicts, SuspectedTheft violations, JusticeDispositionProfile)

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

### Guard Crime Response (absorbed from E17)

Guards respond to crime through the standard AI pipeline, not through special-case crime-response code. Guards are agents with `JusticeDispositionProfile` (from E17) who receive crime information via `ShareBelief`/Tell and act on it through normal goal generation.

#### Guard Investigation
- Guards who learn of a crime (via Tell from witness, or by discovering a `SuspectedTheft` violation themselves) generate `InvestigateViolation` goals through the S27 pipeline.
- Guards travel to the crime location and investigate, producing `SuspectedTheft` observations if they own the jurisdiction's property, or `WitnessedAbsence` otherwise.
- Investigation duration is profile-driven via `ViolationDispositionProfile`.

#### Guard Pursuit
- When a guard has evidence identifying a suspect (via `SuspectedTheft` with `suspect: Some(entity)` or via witness testimony received through Tell), the guard's `emit_justice_candidates()` (E17) generates an `Accuse` goal.
- The planner produces a multi-step plan: travel to accused's believed location (or to the CrimeRegister) -> file accusation -> travel to accused -> punish.
- Pursuit is not a separate action -- it is normal travel planning toward the accused's believed location, driven by the planner's precondition chain.

#### Guard Arrest / Confrontation
- When a guard with institutional authority is co-located with an accused agent and an unresolved accusation exists, `emit_justice_candidates()` generates `PunishAccused` goals.
- If the accused resists (modeled via existing combat system -- accused may attack guard), combat escalation occurs through the standard `EngageHostile` pipeline.
- Punishment is `Fine` or `Exile` per the guard's `JusticeDispositionProfile.fine_severity` and the severity of the crime.

#### Response Scaling
- Guard response intensity is belief-driven: guards who receive more crime reports (via Tell) generate more justice candidates.
- `JusticeDispositionProfile.accusation_motive_weight` controls how strongly a guard prioritizes justice over other goals (patrol, survival, etc.).
- Public order (E16) indirectly affects response: low public order increases the urgency of patrol goals, which brings guards to crime-prone areas more frequently, increasing their chance of witnessing crimes or receiving reports.

### Guard Integration with Office System
- Guards loyal to office holder (via LoyalTo relation)
- When ruler changes: guards may change patrol priorities based on new orders
- Guard captain office: if vacant, patrols may become disorganized
- If an office is contested under E16b force-legitimacy rules, guards may escalate around the disputed jurisdiction before a new holder is formally installed

## Tests

### Patrol Tests
- [ ] Patrols change when ruler dies (intensity increases during vacancy)
- [ ] Patrols change when crime spikes at a location
- [ ] Guard route adaptation reflects threat intelligence from beliefs
- [ ] Public order feedback loop: more patrols → less crime → higher order
- [ ] Guards follow assigned routes (visit places in order)
- [ ] Patrol intensity scales with office vacancy and crime rate
- [ ] Route adaptation uses beliefs, not world state

### Guard Crime Response Tests (from E17 absorption)
- [ ] Guard who receives theft report via Tell generates Accuse goal for the suspect
- [ ] Guard with institutional authority and co-located accused generates PunishAccused goal
- [ ] Guard travels to CrimeRegister to file accusation (multi-step plan)
- [ ] Guard confrontation with resisting accused escalates to combat
- [ ] Guard without institutional authority does NOT generate PunishAccused goal
- [ ] Guard response proportional to crime reports received (more reports → higher motive)
- [ ] Guard at unrelated location does not respond to crime they have not heard about (P7 locality)

## Acceptance Criteria
- Guards patrol real routes through the world graph
- Patrol behavior adapts to threats and institutional state
- Public order feedback loop creates emergent stability/instability
- No scripted patrol changes
- Guards respond to crime through standard AI pipeline (no special crime-response system)
- Guard crime response driven by beliefs and Tell-received evidence, not world state
- Guard pursuit is normal travel planning toward accused's believed location

## Spec References
- Section 4.5 (core systems include guard behavior)
- Section 7.4 (institutional propagation: law enforcement, patrol intensity)
- Section 3.9 (public-order consequences)
