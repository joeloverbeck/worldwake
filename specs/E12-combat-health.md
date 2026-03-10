# E12: Combat, Wounds & Healing

## Epic Summary
Implement combat actions, a unified wound system, natural stabilization / healing, death with finality, and corpse handling. There is **no stored Health component**. Agent bodily condition is derived from `WoundList` plus current physiological state.

## Phase
Phase 2: Survival & Logistics

## Crate
`worldwake-systems`

## Dependencies
- E08 (actions and scheduler)
- Shared body-harm schema in `worldwake-core` so combat wounds and deprivation wounds use the same data model

## Foundations Alignment Changes
This revision fixes three important gaps:

1. **Wounds are generalized bodily harm, not combat-only damage.** The same carrier must support deprivation harm from E09 and future non-combat injury.
2. **Wounds now stabilize and heal naturally.** Without this, the system lacks a physical dampener and fills the world with permanent one-way damage unless a medic intervenes.
3. **Bleeding progression must be per-wound state, not a hand-wavy global decrement.**

## Deliverables

### CombatProfile Component
Per-agent combat / bodily resilience parameters enabling Principle 11.

- `wound_capacity: Permille` — total wound load before death
- `incapacitation_threshold: Permille` — wound load before new actions are blocked
- `attack_skill: Permille`
- `guard_skill: Permille`

`attack_skill` / `guard_skill` replace the earlier `base_attack` / `base_defense` wording. They are still numeric, but they are explicitly traits of the body / training rather than floating anonymous formula knobs.

### WoundList Component
`WoundList` is a `Vec<Wound>` stored on the agent entity.

```rust
struct Wound {
    severity: Permille,
    location: BodyPart,          // Head, Torso, Arms, Legs
    bleed_rate_per_tick: Permille,
    ticks_since_inflicted: u32,
    cause: WoundCause,
}
```

`WoundCause` must include at least:
- `Combat { attacker: Option<EntityId>, weapon: Option<CommodityKind> }`
- `Deprivation(Starvation | Dehydration)`

Future-compatible causes may include accident, exposure, illness, etc.

### No Stored Health Component
Authoritative checks are always derived:

- **Wound load** = sum of `wound.severity`
- **Incapacitated** = `wound_load >= incapacitation_threshold`
- **Dead** = `wound_load >= wound_capacity`

No `health`, `hit_points`, or duplicate aggregate score may be stored.

### Combat Actions

#### Attack
- Precondition:
  - attacker and target co-located at the same place **or** the same route-occupancy entity
  - attacker alive
  - attacker not incapacitated
  - attacker has weapon or can fight unarmed
- Duration: derived from weapon profile or unarmed profile
- Effect:
  - resolve hit / guard outcome
  - append wound(s) to target
  - emit public combat event
- Visibility: public at place / route occupancy
- Witnesses: all co-located agents there

#### Defend
- Precondition: actor alive and capable
- Effect: raises effective guard during the stance
- Duration: action-defined and matched against incoming attack resolution window

### Hit Resolution
Deterministic given RNG state.

Inputs may include:
- weapon wound profile
- `attack_skill`
- `guard_skill`
- armor coverage / mitigation
- fatigue from E09
- existing wound penalties from `WoundList`

The output is not “health damage.” It is one or more new `Wound` values appended to `WoundList`.

### Per-Wound Progression
Each wound progresses independently each tick.

- `ticks_since_inflicted += 1`
- if `bleed_rate_per_tick > 0`, increase `severity` by that rate
- once clotting / treatment occurs, `bleed_rate_per_tick` becomes `0`
- non-bleeding wounds can slowly recover if recovery conditions are met

This must be implemented per wound, not as a global health drain.

### Natural Stabilization & Recovery
Phase 2 must include low-fidelity natural recovery.

A wound may naturally improve when:
- it is not actively bleeding
- the agent is alive
- the agent is not in immediate combat
- the agent has at least minimally tolerable hunger / thirst / fatigue

Medicine accelerates and improves this process, but medicine is not the sole path to recovery.

This is the physical dampener for the wound-spiral loop.

### Wound Consequences (Cross-System via State)
Wound effects propagate through shared state.

- **Pain**: derived by E13 from `WoundList`; not stored as a separate component
- **Bleeding**: wound severity increases per wound
- **Movement restriction**: severe leg wounds restrict travel and pursuit
- **Action restriction**: severe arm wounds hinder weapon use / crafting
- **Incapacitation**: wound load beyond threshold blocks new actions
- **Fatigue interaction**: combat reads `HomeostaticNeeds.fatigue` directly

### Healing Action
- **Heal**: agent treats wounded target
- Precondition:
  - healer and target co-located
  - target has wound(s)
  - healer has medicine / bandage
- Effect:
  - reduce `bleed_rate_per_tick` immediately or over early treatment ticks
  - reduce wound severity over treatment duration
  - consume medicine
- Duration: derived from medicine profile and wound severity
- Result: treated wound becomes more stable and heals faster

### Death
Triggered when wound load reaches `wound_capacity`.

When death triggers:
- agent state changes to dead
- remove from planning / acting schedule
- body persists with:
  - inventory intact
  - `WoundList` intact
  - location or route occupancy preserved until later handling
- emit death event with cause chain

### Corpse Handling
Bodies remain in the world and are interactive.

Possible actions:
- **Loot**
- **Bury**
- **Discover / inspect**

No corpse auto-cleanup is allowed in Phase 2.

## Component Registration
New components to register in `component_schema.rs`:

- `WoundList` — on `EntityKind::Agent`
- `CombatProfile` — on `EntityKind::Agent`

## SystemFn Integration
- Implements the combat / wound handler in `SystemDispatch`
- Runs once per tick for active combat actions and wound progression
- Reads:
  - `WoundList`
  - `CombatProfile`
  - weapon / armor profiles
  - `HomeostaticNeeds`
- Writes:
  - `WoundList`
  - dead / alive state
  - combat events
  - death events

E12 does **not** call E09 or E13 directly.

## Cross-System Interactions (Principle 12)
- **E09 → E12**: physiology affects combat effectiveness and natural healing quality
- **E09 → E12**: deprivation can append `WoundCause::Deprivation(...)` entries to `WoundList`
- **E12 → E13**: AI derives pain / danger / treatment priorities from wounds and co-located threats
- **E12 → future systems**: corpses and wounds become discoverable material evidence

## FND-01 Section H

### Information-Path Analysis
- Combat is local to the shared place / route occupancy
- Witnesses are all co-located agents
- Wounds live on the wounded body
- Death events propagate through ordinary event visibility and later witness systems

### Positive-Feedback Analysis
- **Wounds → weakness → more wounds**
- **Bleeding → more wound load → less capacity to flee / heal**

### Concrete Dampeners
- **Wounds → weakness loop dampeners**:
  - physical separation if one side flees
  - incapacitation removing the target from active exchange
  - natural stabilization / healing when combat stops
- **Bleeding loop dampeners**:
  - clotting over time for survivable wounds
  - bandaging / healing by co-located agents

### Stored State vs. Derived Read-Model
**Stored (authoritative)**:
- `WoundList`
- `CombatProfile`
- dead / alive state

**Derived (transient read-model)**:
- wound load
- pain pressure
- incapacitation
- death imminence
- current combat disadvantage from wounds

## Invariants Enforced
- 9.14: Death finality — dead agents do not plan, act, trade, vote, or consume
- 9.5: Conservation — corpse inventory persists; treatment consumes medicine
- Principle 3: no stored health component
- Principle 8: wound loops have physical dampeners through clotting / recovery / separation

## Tests
- [ ] T14: Dead agents generate no new plans or actions
- [ ] Combat resolves deterministically with same RNG state
- [ ] New wounds append to `WoundList` with correct cause and location
- [ ] Per-wound bleeding increases severity over time
- [ ] Treatment reduces bleeding and consumes medicine
- [ ] Non-bleeding wounds can naturally stabilize / recover under acceptable physiological conditions
- [ ] Deprivation wounds coexist with combat wounds in the same `WoundList`
- [ ] Death triggers when wound load reaches `wound_capacity`
- [ ] Incapacitation triggers when wound load reaches `incapacitation_threshold`
- [ ] Corpse retains inventory and location context
- [ ] Death event traces back to cause chain
- [ ] Cannot attack dead agents
- [ ] No stored `Health` component exists
- [ ] Different `CombatProfile` values produce different outcomes
- [ ] Durations derive from weapon / medicine profiles, not hardcoded constants

## Acceptance Criteria
- Combat produces wounds, not hit-point subtraction
- Wounds are the unified bodily-harm carrier for combat and deprivation
- No stored health component exists
- Death is final
- Corpses persist as interactive entities
- Wounds can stabilize and heal naturally; medicine accelerates recovery
- All combat outcomes remain traceable in the event log

## FND-01 Route Presence Note
E10’s explicit `InTransitOnEdge` now provides a lawful route-presence carrier.  
This epic still does **not** implement large-scale patrol / ambush behavior; it only ensures combat can occur where co-presence actually exists.

## Spec References
- Section 3.4 (wounds as carriers of consequence)
- Section 4.5 (combat / injury / death, healing)
- Section 7.1 (material propagation: wounds, corpses)
- Section 9.14 (death finality)
- `docs/FOUNDATIONS.md` Principles 3, 6, 8, 11, 12