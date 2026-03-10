# E12: Combat & Health

## Epic Summary
Implement combat actions, wound system, healing, death with finality, and corpse handling.

## Phase
Phase 2: Survival & Logistics

## Crate
`worldwake-systems`

## Dependencies
- E08 (actions and scheduler)

## Deliverables

### Combat Actions

- **Attack**: agent attacks target with weapon or fists
  - Precondition: attacker and target at same place, attacker alive, has weapon (or unarmed)
  - Duration: 2-5 ticks per exchange
  - Effect: wound applied to target based on weapon + defense
  - Visibility: Public at the place
  - Witnesses: all agents at the place

- **Defend**: agent assumes defensive stance
  - Effect: reduces incoming damage for duration
  - Duration: matches attacker's action

### Hit Resolution
- Deterministic given RNG state:
  - Attack value (weapon + attacker stats) vs defense value
  - Hit/miss determination
  - Wound severity if hit

### Wound System
- `Wound` component:
  - `severity: WoundSeverity` (Minor, Moderate, Severe, Critical, Fatal)
  - `location: BodyPart` (simplified: Head, Torso, Arms, Legs)
  - `bleeding: bool`
  - `ticks_since_inflicted: u32`
- Wound effects:
  - Pain need increases proportional to severity
  - Bleeding: health deteriorates over time without treatment
  - Severe wounds: movement speed reduction, action restrictions
  - Critical wounds: incapacitation
  - Fatal wounds: death

### Healing Action
- **Heal**: agent applies medicine to wounded agent
  - Precondition: healer has medicine, target has wounds, both at same place
  - Effect: wound severity decreases over time, consume medicine
  - Duration: 20-60 ticks depending on severity
  - Bleeding stopped immediately on treatment start

### Death
- When wounds become fatal (cumulative damage threshold):
  - Agent state changes to Dead
  - Remove from scheduler (no new plans or actions)
  - Body persists as entity with:
    - All inventory intact
    - Location unchanged
    - Wounds recorded
  - Emit death event with cause chain (traces back to attack)

### Death Finality
- Dead agents:
  - Generate no new plans or actions
  - Cannot trade, vote, consume, or move
  - Are not removed from the world (body persists)
  - Can be acted upon (looted, buried, discovered)

### Corpse Handling
- Body remains at location with all possessions
- Other agents can:
  - Loot: take items from corpse (transfer possession)
  - Bury: move corpse to burial site (action with duration)
  - Discover: finding a body triggers investigation events

## Invariants Enforced
- 9.14: Death finality - dead agents do not plan, act, trade, vote, or consume
- 9.5: Conservation - items on corpse persist, medicine consumed in healing

## Tests
- [ ] T14: Dead agents generate no new plans or actions
- [ ] Combat resolves deterministically with same RNG state
- [ ] Wounds increase pain need
- [ ] Bleeding causes health deterioration over time
- [ ] Healing consumes medicine and reduces wound severity
- [ ] Death triggers: removed from scheduler, body persists
- [ ] Corpse retains inventory (can be looted)
- [ ] Death event traces back to attack via cause chain
- [ ] Cannot attack dead agents (precondition: target alive)
- [ ] Fatal wounds from cumulative damage

## Acceptance Criteria
- Combat follows action framework with full event emission
- Wounds have material consequences (pain, bleeding, incapacitation)
- Death is final and irreversible
- Corpses persist as interactive world entities
- All combat outcomes traceable through event log

## FND-01 Section D — Route Presence Gate

**GATE**: Any route-based combat encounter, ambush, patrol, or interception logic in this epic MUST NOT proceed until a concrete route presence model exists in the codebase. This model must support:

- Determining which entities are physically on a route or route segment
- Determining which travelers can physically encounter each other
- Determining which agents can witness route events locally

It is **forbidden** to introduce stored route danger or visibility scores to compensate for missing route presence. All route risk/danger must be derived from concrete entity presence, never from stored abstract scores (Principle 3, `docs/FOUNDATIONS.md`).

See `specs/FND-01-phase1-foundations-alignment.md` Section D for full context.

## Spec References
- Section 3.4 (wounds as carriers of consequence)
- Section 4.5 (combat/injury/death, healing)
- Section 7.1 (material propagation: wounds, corpses)
- Section 9.14 (death finality)
- `specs/FND-01-phase1-foundations-alignment.md` Section D (route presence gate)
