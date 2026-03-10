# E12: Combat & Health

## Epic Summary
Implement combat actions, wound system, healing, death with finality, and corpse handling. There is no stored Health component — agent physical condition is derived entirely from the wound list (Principle 3).

## Phase
Phase 2: Survival & Logistics

## Crate
`worldwake-systems`

## Dependencies
- E08 (actions and scheduler)

## Deliverables

### CombatProfile Component
Per-agent combat parameters enabling Principle 11 (agent diversity):
- `wound_capacity: Permille` — total wound severity an agent can sustain before death (higher = more resilient)
- `incapacitation_threshold: Permille` — wound severity sum at which agent becomes incapacitated (lower than wound_capacity)
- `base_attack: Permille` — base offensive capability
- `base_defense: Permille` — base defensive capability

Different agents have different `CombatProfile` values seeded at creation.

### Combat Actions

- **Attack**: agent attacks target with weapon or fists
  - Precondition: attacker and target at same place, attacker alive and not incapacitated, has weapon (or unarmed)
  - Duration: derived from weapon's `CommodityPhysicalProfile` (attack speed) — no hardcoded tick counts
  - Effect: wound applied to target based on weapon + attacker's `base_attack` vs target's `base_defense`
  - Visibility: Public at the place
  - Witnesses: all agents at the place

- **Defend**: agent assumes defensive stance
  - Effect: increases effective defense for duration
  - Duration: matches attacker's action duration

### Hit Resolution
- Deterministic given RNG state:
  - Attack value (weapon profile + attacker's `base_attack`) vs defense value (armor + target's `base_defense`)
  - Hit/miss determination
  - Wound severity if hit (derived from weapon damage profile minus effective defense)

### WoundList Component
`WoundList` is a `Vec<Wound>` stored on the agent entity. Each `Wound` contains:
- `severity: Permille` — wound severity (higher = worse)
- `location: BodyPart` — simplified enum: Head, Torso, Arms, Legs
- `bleeding: bool` — whether the wound is actively bleeding
- `ticks_since_inflicted: u32` — age of the wound

**No stored Health component.** Agent physical condition is derived from the wound list:
- **Current wound load**: sum of all `wound.severity` values in the `WoundList`
- **Incapacitation**: when wound load ≥ agent's `CombatProfile.incapacitation_threshold`
- **Death**: when wound load ≥ agent's `CombatProfile.wound_capacity`

### Wound Effects (Cross-System via State)
Wound effects propagate through shared state (Principle 12), never through direct system calls:
- **Pain**: E09 needs system reads `WoundList`, derives `AgentCondition.pain` from wound severity sum
- **Bleeding**: bleeding wounds increase in `severity` by a per-wound tick amount each tick (concrete state change — NOT "health decreases")
- **Movement restriction**: severe wounds on Legs reduce movement capability (read by travel action preconditions)
- **Action restriction**: severe wounds on Arms restrict weapon use and crafting
- **Incapacitation**: wound load ≥ `incapacitation_threshold` prevents new actions
- **Fatigue interaction**: E09's fatigue level is independently readable by combat system to modify `base_defense` (fatigued agents defend worse)

### Healing Action
- **Heal**: agent applies medicine to wounded agent
  - Precondition: healer has medicine, target has wounds, both at same place
  - Effect: wound severity decreases over duration, consume medicine
  - Duration: derived from medicine's `CommodityPhysicalProfile` (healing_ticks_per_severity) scaled by wound severity — no hardcoded tick counts
  - Bleeding stopped immediately on treatment start

### Death
- Triggered when sum of wound severity values in `WoundList` ≥ agent's `CombatProfile.wound_capacity`
- When death triggers:
  - Agent state changes to Dead
  - Remove from scheduler (no new plans or actions)
  - Body persists as entity with:
    - All inventory intact
    - Location unchanged
    - `WoundList` recorded
  - Emit death event with cause chain (traces back to attack events)

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

## Component Registration
New components to register in `component_schema.rs`:
- `WoundList` — on `EntityKind::Agent`
- `CombatProfile` — on `EntityKind::Agent`

## SystemFn Integration
- Implements the `SystemId::Combat` handler registered in `SystemDispatch`
- Runs once per tick for all active combat actions
- Reads: `WoundList`, `CombatProfile`, weapon/armor `CommodityPhysicalProfile`
- Writes: `WoundList` (add wounds, progress bleeding), agent death state
- Does NOT read or call E09 needs system — fatigue's effect on combat is achieved by reading `HomeostaticNeeds.fatigue` directly from component storage

## Cross-System Interactions (Principle 12)
- **E12 → E09**: Combat writes `WoundList`; needs system reads it to derive pain
- **E09 → E12**: Needs system writes `HomeostaticNeeds.fatigue`; combat reads it to modify defense effectiveness
- **E12 → E13**: Decision architecture reads `WoundList` via `BeliefView` to assess danger and prioritize healing/fleeing

## FND-01 Section H

### Information-Path Analysis
- Combat is local: attacker and target must be co-located (same place). All combat information is generated at the place where it occurs.
- Witnesses: all agents at the combat location perceive the event (co-location perception, Principle 7).
- Wound information: stored on the wounded agent entity, readable by any system processing that agent.
- Death events: emitted to the event log with cause chain, propagate through normal event visibility rules.

### Positive-Feedback Analysis
- **Wounds → weakness → more wounds**: wounded agents defend worse (reduced effective defense from pain/fatigue), making them more likely to receive additional wounds.
- **Violence → fear → flight → vulnerability → more violence**: agents fleeing combat are vulnerable to pursuit attacks.

### Concrete Dampeners
- **Wounds → weakness loop**: incapacitation threshold stops the loop — once wound load hits `incapacitation_threshold`, the agent can no longer fight (and typically attackers stop attacking incapacitated targets unless specifically motivated). The dampener is physical incapacity.
- **Violence → fear loop**: fleeing agents leave the combat location, breaking co-location — the attacker must pursue along travel edges, costing time. Geographic separation is the physical dampener. Additionally, attackers have their own needs (fatigue, hunger) that compete with pursuit.

### Stored State vs. Derived Read-Model
**Stored (authoritative)**:
- `WoundList` component (Vec<Wound> with severity, location, bleeding, age)
- `CombatProfile` component (wound_capacity, incapacitation_threshold, base_attack, base_defense)
- Agent dead/alive state

**Derived (transient read-model)**:
- "Health" / wound load (sum of wound severities — never stored, computed on demand)
- Whether agent is incapacitated (wound load vs. threshold — computed, not stored)
- Whether agent is about to die (wound load approaching capacity — computed)
- Pain level (derived by E09 from wound list)

## Invariants Enforced
- 9.14: Death finality — dead agents do not plan, act, trade, vote, or consume
- 9.5: Conservation — items on corpse persist, medicine consumed in healing
- Principle 3: No stored Health component — condition derived from wound list

## Tests
- [ ] T14: Dead agents generate no new plans or actions
- [ ] Combat resolves deterministically with same RNG state
- [ ] Wounds added to `WoundList` with correct severity based on weapon vs defense
- [ ] Bleeding wounds increase in severity per tick (concrete state change)
- [ ] Healing consumes medicine and reduces wound severity
- [ ] Death triggers when wound severity sum ≥ `wound_capacity`
- [ ] Incapacitation triggers when wound severity sum ≥ `incapacitation_threshold`
- [ ] Corpse retains inventory (can be looted)
- [ ] Death event traces back to attack via cause chain
- [ ] Cannot attack dead agents (precondition: target alive)
- [ ] No stored Health component — all condition checks derived from `WoundList`
- [ ] Different `CombatProfile` values produce different combat outcomes (Principle 11)
- [ ] Durations derived from weapon/medicine profiles, not hardcoded

## Acceptance Criteria
- Combat follows action framework with full event emission
- Wounds have material consequences (pain via E09, bleeding, incapacitation)
- No stored Health component — condition derived from wound list (Principle 3)
- Death is final and irreversible
- Corpses persist as interactive world entities
- All combat outcomes traceable through event log
- All durations derived from profiles, not hardcoded constants

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
- `docs/FOUNDATIONS.md` Principles 3, 8, 11, 12
- `specs/FND-01-phase1-foundations-alignment.md` Section D (route presence gate)
