# E09: Needs, Physiology & Metabolism

**Status**: ⏸️ DEFERRED

## Epic Summary
Implement agent physiology as concrete body-state carriers with tick-based progression, explicit action body costs, consumable effects, deprivation consequences, and personal-care actions. In Phase 2, **danger/fear is not a stored need score**. Danger pressure is derived by E13 from believed threats. **Pain is derived from wounds, not stored as an authoritative component.**

## Phase
Phase 2: Survival & Logistics

## Crate
`worldwake-systems`

## Dependencies
- E08 (scheduler drives tick-based progression and action progression)
- Shared body-harm schema in `worldwake-core` (`Wound`, `WoundCause`, `BodyPart`) so deprivation harm and combat harm use the same carrier of consequence

## Foundations Alignment Changes
This revision makes five non-negotiable corrections:

1. **No stored fear score.** A stored `fear: Permille` hides causality and duplicates the future belief/perception system. Phase 2 AI must derive danger from believed hostile presence, recent violence, and current wounds rather than reading a free-floating scalar.
2. **Needs remain embodied state, not designer-authored mood bars.** `HomeostaticNeeds` is interpreted as concrete body state on the agent entity: caloric depletion, hydration depletion, sleep debt, bladder fullness, and grime load.
3. **Critical unmet needs have bodily consequences.** Starvation and dehydration must eventually produce concrete harm through `WoundList`; exhaustion must eventually force collapse/sleep. Without this, the survival loop has no teeth.
4. **Every active action has body cost.** Travel, combat, harvesting, crafting, negotiation, carrying, washing, and similar actions must expose deterministic physiology deltas per tick, not a hand-wavy “activity modifier.”
5. **Action preconditions stay physical.** “Hungry enough” is a planning concern, not a physics gate. If an agent has food, they are physically able to eat even if the AI would not choose to.

## Deliverables

### HomeostaticNeeds Component
Per-agent embodied body state that progresses each simulation tick.

- `hunger: Permille` — short-horizon caloric depletion (`0 = fully sated`, `1000 = starving`)
- `thirst: Permille` — hydration depletion (`0 = hydrated`, `1000 = dehydrated`)
- `fatigue: Permille` — accumulated sleep debt and exertion (`0 = rested`, `1000 = exhausted`)
- `bladder: Permille` — bladder fullness (`0 = empty`, `1000 = desperate`)
- `dirtiness: Permille` — accumulated grime on body/clothing (`0 = clean`, `1000 = filthy`)

These values are authoritative body state on the agent entity. They are not “mood” scores and they are not designer-authored urgency flags.

### DeprivationExposure Component
Per-agent counters for sustained time spent at critical physiological pressure.

- `hunger_critical_ticks: u32`
- `thirst_critical_ticks: u32`
- `fatigue_critical_ticks: u32`
- `bladder_critical_ticks: u32`

These counters increment only while the corresponding drive is at or above that drive’s `critical` threshold and reset when the drive falls back below critical.

This component is required because `Permille` saturates at `1000`; prolonged exposure beyond the cap must still have consequences.

### MetabolismProfile Component
Per-agent physiological parameters enabling Principle 11 (agent diversity).

Per-tick basal progression:
- `hunger_rate: Permille`
- `thirst_rate: Permille`
- `fatigue_rate: Permille`
- `bladder_rate: Permille`
- `dirtiness_rate: Permille`

Recovery / tolerance:
- `rest_efficiency: Permille` — fatigue reduction per sleep tick before sleep-site modifiers
- `starvation_tolerance_ticks: NonZeroU32` — sustained critical-hunger time before deprivation harm is added
- `dehydration_tolerance_ticks: NonZeroU32` — sustained critical-thirst time before deprivation harm is added
- `exhaustion_collapse_ticks: NonZeroU32` — sustained critical-fatigue time before forced collapse / sleep
- `bladder_accident_tolerance_ticks: NonZeroU32` — sustained critical-bladder time before involuntary relief
- `toilet_ticks: NonZeroU32`
- `wash_ticks: NonZeroU32`

Different agents have different `MetabolismProfile` values seeded at creation.

### Shared DriveThresholds Component
`DriveThresholds` is **shared Phase 2 schema**, stored on the agent and consumed by both E09 and E13. It must **not** be owned solely by E13 because E09 also needs it for deprivation tracking and collapse behavior.

```rust
struct ThresholdBand {
    low: Permille,
    medium: Permille,
    high: Permille,
    critical: Permille,
}

struct DriveThresholds {
    hunger: ThresholdBand,
    thirst: ThresholdBand,
    fatigue: ThresholdBand,
    bladder: ThresholdBand,
    dirtiness: ThresholdBand,
    pain: ThresholdBand,
    danger: ThresholdBand,
}
```

The earlier “one global threshold set for everything” design is rejected. Thirst, fatigue, dirtiness, pain, and danger must be able to trigger at different levels.

### Commodity Consumable Profile Extensions
Consumable effects must come from data on the commodity, not from hardcoded action logic.

Extend commodity data with a consumable profile (either by extending `CommodityPhysicalProfile` or by adding a dedicated `CommodityConsumableProfile`):

- `consumption_ticks_per_unit: NonZeroU32`
- `hunger_relief_per_unit: Permille`
- `thirst_relief_per_unit: Permille`
- `bladder_fill_per_unit: Permille`

Examples:
- Bread relieves hunger strongly, thirst weakly, and barely fills bladder.
- Water relieves thirst strongly and fills bladder significantly.
- Fruit can relieve both hunger and thirst.

### Action Body Cost Metadata
Every long-running action that can strain the body must expose deterministic per-tick body costs through action metadata or active action state.

```rust
struct BodyCostPerTick {
    hunger_delta: Permille,
    thirst_delta: Permille,
    fatigue_delta: Permille,
    dirtiness_delta: Permille,
}
```

Examples:
- Travel adds fatigue and thirst.
- Combat adds significant fatigue.
- Harvesting / crafting add fatigue and some dirtiness.
- Washing has near-zero fatigue and reduces dirtiness via its explicit effect rather than a negative body cost.
- Sleep has no positive exertion cost and instead applies recovery.

This replaces the earlier vague “activity modifier if active” language.

### Metabolism System
Registered as the physiology / needs handler in `SystemDispatch`.

Per tick, for each living agent with `HomeostaticNeeds + MetabolismProfile + DriveThresholds`:

1. Apply basal progression from `MetabolismProfile`
2. Read active action state and apply any `BodyCostPerTick`
3. Clamp all `Permille` values to the valid range
4. Update `DeprivationExposure`
5. Apply sustained-deprivation consequences

### Deprivation Consequences
Critical unmet needs must create concrete downstream effects.

- **Critical hunger held too long**  
  Add a wound with `WoundCause::Deprivation(Starvation)` to `WoundList`
- **Critical thirst held too long**  
  Add a wound with `WoundCause::Deprivation(Dehydration)` to `WoundList`
- **Critical fatigue held too long**  
  Force the agent into collapse / sleep if they are not already sleeping
- **Critical bladder held too long**  
  Trigger involuntary relief, create waste, and increase `dirtiness`

Phase 2 does **not** model disease or infection yet, so dirtiness does not directly add wounds on its own. It does, however, feed into later healing quality and future social systems.

### Consumption & Care Actions (`ActionDef` instances)

All actions follow the E07 action framework.  
All action preconditions remain **physical**, not motivational.

#### Eat
- Precondition: actor can access an edible lot they are allowed to consume
- Effect:
  - reduce item quantity
  - decrease `hunger` by commodity profile
  - optionally decrease `thirst` if the food is hydrating
  - increase `bladder` by commodity profile
- Duration: `consumption_ticks_per_unit` from the consumable profile
- Interruptibility: `ByDanger`, `ByMajorPain`
- Notes: there is **no** “must already be hungry enough” precondition

#### Drink
- Precondition: actor can access drinkable water / beverage
- Effect:
  - reduce item quantity
  - decrease `thirst`
  - increase `bladder`
- Duration: `consumption_ticks_per_unit` from the consumable profile
- Interruptibility: `ByDanger`, `ByMajorPain`

#### Sleep
- Precondition: actor can occupy current location or a reservable sleep affordance there
- Reservation: optional bed / cot / sleep spot entity if present
- Effect: decrease `fatigue` each tick by `rest_efficiency` modified by sleep-site quality
- Duration: derived from current `fatigue`, `rest_efficiency`, and sleep-site quality
- Interruptibility: `ByDanger`, `ByAcutePain`
- Important correction: sleep is allowed on the ground / wilderness if the agent has no bed. Beds and shelter improve recovery; they are **not** a binary gate.

#### Toilet
- Precondition: actor is at a latrine / toilet affordance **or** in wilderness
- Reservation: toilet stall if a facility is used
- Effect:
  - decrease `bladder`
  - create waste entity at location
  - wilderness toileting may increase `dirtiness`
- Duration: `MetabolismProfile.toilet_ticks`

#### Wash
- Precondition: actor can access water source, wash facility, or carried wash water
- Effect:
  - decrease `dirtiness`
  - consume wash water if applicable
- Duration: `MetabolismProfile.wash_ticks`

### Sleep-Site Quality
Sleep quality is a derived read-model from concrete affordances at the location:

- bed / cot / bunk reserved by actor
- shelter presence
- bare ground
- current local danger (read by AI, not the sleep action itself)

Beds do not grant a magic “can sleep” permission. They provide better recovery and lower interruption risk.

## Component Registration
New components to register in `component_schema.rs`:

- `HomeostaticNeeds` — on `EntityKind::Agent`
- `DeprivationExposure` — on `EntityKind::Agent`
- `MetabolismProfile` — on `EntityKind::Agent`
- `DriveThresholds` — on `EntityKind::Agent` (shared Phase 2 schema)

## SystemFn Integration
- Runs once per tick for all living agents
- Reads:
  - `HomeostaticNeeds`
  - `DeprivationExposure`
  - `MetabolismProfile`
  - `DriveThresholds`
  - active action state / `BodyCostPerTick`
- Writes:
  - `HomeostaticNeeds`
  - `DeprivationExposure`
  - `WoundList` (deprivation wounds only)
  - waste entities / relief events
  - collapse / forced sleep requests

The needs system does **not** own danger, fear, or abstract “wellness” scores.

## Cross-System Interactions (Principle 12)
All cross-system effects propagate through shared state, never through direct system calls.

- **E09 → E12**: sustained starvation / dehydration add deprivation wounds to `WoundList`
- **E12 → E09**: natural healing and combat can read `fatigue` / `dirtiness` when determining recovery quality or combat effectiveness
- **E09 → E13**: decision architecture reads `HomeostaticNeeds` and `DriveThresholds`
- **E10 / E11 / E12 → E09**: long-running actions expose `BodyCostPerTick`; E09 reads the active action state and applies the cost
- **E09 → future systems**: waste entities become discoverable material traces

## FND-01 Section H

### Information-Path Analysis
- Basal metabolism is agent-local
- Consumption requires co-location with the consumable or explicit possession
- Toileting and washing require co-location with affordances or explicit carried water
- Deprivation harm is generated from the agent’s own stored body state plus elapsed time
- No agent receives remote physiology information through this system

### Positive-Feedback Analysis
- **Fatigue → poor decisions → danger → lost sleep → more fatigue**
- **Hunger / thirst → incapacity → failure to procure food/water → more hunger / thirst**

### Concrete Dampeners
- **Fatigue loop dampener**: physical collapse / forced sleep after sustained critical exhaustion
- **Hunger / thirst loop dampener**: co-located aid from other agents, carried provisions, and the fact that deprivation becomes a concrete wound that other systems can detect and react to
- **Bladder loop dampener**: involuntary relief; the body eventually resolves the pressure at the cost of waste and dirtiness

### Stored State vs. Derived Read-Model
**Stored (authoritative)**:
- `HomeostaticNeeds`
- `DeprivationExposure`
- `MetabolismProfile`
- `DriveThresholds`

**Derived (transient read-model)**:
- pain (`WoundList` → pain pressure)
- danger (believed hostile presence / recent violence)
- urgency levels (drive value vs per-drive threshold band)
- sleep-site quality
- “does this agent need to eat now?” as a decision-layer conclusion

## Invariants Enforced
- 9.15: Off-camera continuity — physiology progresses regardless of camera / visibility
- 9.16: Need continuity — physiology changes only through time, action body cost, consumption, rest, toileting, washing, injury, healing, or other explicit effects
- Principle 6: every active action that strains the body must expose body cost
- Principle 3: no stored fear or wellness score

## Tests
- [ ] T15: Need progression — values evolve by simulation tick, not frame rate or camera
- [ ] T26: Camera independence — physiology does not reset on visibility change
- [ ] Eating consumes food and applies commodity-defined relief
- [ ] Drinking consumes water and applies commodity-defined relief
- [ ] Sleep reduces fatigue even without a bed; beds improve recovery rate
- [ ] Toilet reduces bladder and creates waste entity
- [ ] Wash reduces dirtiness and consumes water when applicable
- [ ] Active action body costs increase fatigue / thirst deterministically
- [ ] Sustained critical hunger adds deprivation wound(s)
- [ ] Sustained critical thirst adds deprivation wound(s)
- [ ] Sustained critical fatigue triggers forced collapse / sleep
- [ ] Sustained critical bladder triggers involuntary relief
- [ ] Need values stay within `Permille` range
- [ ] Different `MetabolismProfile` values produce different progression / tolerance behavior
- [ ] `DriveThresholds` are per-drive and per-agent, not global constants
- [ ] There is no stored `fear` component and no stored `AgentCondition` component

## Acceptance Criteria
- Physiology is tracked as concrete body state, not motivational fluff
- Consumable effects come from commodity data, not hardcoded action logic
- Critical unmet needs create concrete downstream effects
- Sleep is possible without a bed; beds improve quality instead of acting as binary gates
- Every strenuous action has explicit per-tick body cost
- No stored fear score exists in Phase 2
- Deprivation, combat, and AI all interact through shared state only

## Spec References
- Section 4.4 (needs list)
- Section 4.5 (consumption, rest, toilet, hygiene)
- Section 7.5 (physiological propagation)
- Section 9.15 (off-camera continuity)
- Section 9.16 (need continuity)
- `docs/FOUNDATIONS.md` Principles 3, 5, 6, 7, 8, 11, 12

## Outcome

- Completion date: 2026-03-10
- What actually changed:
  - implemented and verified the grounded E09 slice covering `HomeostaticNeeds`, `DeprivationExposure`, `MetabolismProfile`, `DriveThresholds`, basal metabolism, action body costs, starvation/dehydration wounds, involuntary bladder relief, and explicit `Eat` / `Drink` / `Sleep` / `Toilet` / `Wash` actions
  - added scheduler-level integration tests for the shipped physiology and care-action path
- Deviations from original plan:
  - `Sleep` ships today as a repeatable one-tick rest action without bed or shelter quality bonuses
  - `Toilet` and `Wash` do not yet use facility-aware affordances or reservations
  - forced fatigue collapse / forced sleep is still not implemented
  - the spec therefore remains partially deferred and should not be treated as fully implemented as written
- Verification results:
  - `cargo test -p worldwake-systems` passed on 2026-03-10
  - `cargo test --workspace` passed on 2026-03-10
  - `cargo clippy --workspace` passed on 2026-03-10
