# E09: Needs & Metabolism

## Epic Summary
Implement the agent needs system with tick-based progression, consumption actions, and urgency thresholds. All need values use `Permille` (0=satisfied, 1000=desperate) with unified polarity.

## Phase
Phase 2: Survival & Logistics

## Crate
`worldwake-systems`

## Dependencies
- E08 (scheduler drives tick-based progression)

## Deliverables

### HomeostaticNeeds Component
Per-agent homeostatic drives that tick forward each simulation tick via the metabolism system:
- `hunger: Permille` (0=full, 1000=starving)
- `thirst: Permille` (0=hydrated, 1000=dehydrated)
- `fatigue: Permille` (0=rested, 1000=exhausted)
- `bladder: Permille` (0=empty, 1000=desperate)
- `dirtiness: Permille` (0=clean, 1000=filthy)

These are the only needs ticked by the metabolism system. All five share unified polarity: 0 = satisfied, 1000 = desperate.

### AgentCondition Component
Event-driven pressures that are NOT ticked by metabolism — they change only in response to specific world events:
- `pain: Permille` (0=none, 1000=agony) — derived from `WoundList` (E12); increases when wounds are inflicted, decreases when wounds heal
- `fear: Permille` (0=calm, 1000=terrified) — set by perception of threats; decays over time when no threat is present

### Removed from Needs
The following are NOT needs and are NOT tracked as need components:
- **loyalty** — already exists as a `Permille` relation in `crates/worldwake-core/src/relations.rs` (`loyalty_from` field in `RelationTables`)
- **social_standing** — derived from reputation events in a future system; no stored component
- **wealth_pressure** — derivable from coin holdings vs. obligations; a transient read-model, not stored state

### MetabolismProfile Component
Per-agent metabolism rates enabling Principle 11 (agent diversity). All rates are `Permille` values representing per-tick increase:
- `hunger_rate: Permille` — base hunger increase per tick
- `thirst_rate: Permille` — base thirst increase per tick
- `fatigue_rate: Permille` — base fatigue increase per tick (modified by activity level)
- `bladder_rate: Permille` — base bladder increase per tick (modified by recent consumption)
- `dirtiness_rate: Permille` — base dirtiness increase per tick
- `rest_efficiency: Permille` — how quickly fatigue decreases during sleep (higher = faster recovery)
- `fear_decay_rate: Permille` — how quickly fear decreases when no threat is present

Different agents have different `MetabolismProfile` values seeded at creation, so two agents of the same role may have different hunger rates, fatigue rates, etc.

### Metabolism System
- Registered as `SystemId::Needs` handler in `SystemDispatch`
- Per-tick processing for all agents with `HomeostaticNeeds` + `MetabolismProfile`:
  - `hunger += profile.hunger_rate` per tick
  - `thirst += profile.thirst_rate` per tick
  - `fatigue += profile.fatigue_rate` per tick (multiplied by activity modifier if active)
  - `bladder += profile.bladder_rate` per tick (multiplied by consumption modifier after eating/drinking)
  - `dirtiness += profile.dirtiness_rate` per tick
- All values clamped to `Permille` range (0–1000) by type safety
- Rates are deterministic (no RNG in base metabolism)
- AgentCondition fields (pain, fear) are NOT ticked by metabolism — they change only via events

### Consumption Actions (ActionDef instances)
All follow the E07 action framework:

- **Eat**: actor has food item → consume → reduce hunger, increase bladder slightly
  - Precondition: actor possesses food, hunger above agent's urgency threshold
  - Effect: remove/reduce food lot, decrease hunger
  - Duration: `CommodityPhysicalProfile.consumption_ticks_per_unit` for the food type (from `crates/worldwake-core/src/items.rs`)

- **Drink**: actor has water → consume → reduce thirst, increase bladder
  - Precondition: actor possesses water
  - Effect: remove/reduce water lot, decrease thirst
  - Duration: `CommodityPhysicalProfile.consumption_ticks_per_unit` for water

- **Rest/Sleep**: actor at bed/shelter → sleep → reduce fatigue
  - Precondition: actor at place with bed/shelter, fatigue above agent's urgency threshold
  - Reservation: bed/sleeping spot
  - Effect: decrease fatigue by `profile.rest_efficiency` per tick over duration
  - Duration: derived from current fatigue level divided by `profile.rest_efficiency` (higher fatigue = longer sleep, faster recovery rate = shorter sleep)
  - Interruptibility: ByDanger, ByUrgentNeed

- **Toilet**: actor at latrine/facility → relieve → reduce bladder, produce waste
  - Precondition: actor at place with toilet OR wilderness
  - Reservation: toilet stall (if facility)
  - Effect: decrease bladder, create waste entity at location
  - Duration: profile-driven (base from `MetabolismProfile`)

- **Wash**: actor at water source → wash → improve dirtiness
  - Precondition: actor at place with water source
  - Effect: decrease dirtiness, consume small amount of water
  - Duration: profile-driven (base from `MetabolismProfile`)

### Need Thresholds & Urgency
- Per-agent `UrgencyThresholds` struct (defined and owned by E13, referenced here):
  - `low: Permille` — minor discomfort, may seek to address
  - `medium: Permille` — significant pressure, will prioritize
  - `high: Permille` — urgent, overrides most other goals
  - `critical: Permille` — emergency, overrides everything except immediate danger
- Different agents have different thresholds (Principle 11)
- Urgency states feed into utility scoring (E13)
- No hardcoded global threshold constants

## Component Registration
New components to register in `component_schema.rs`:
- `HomeostaticNeeds` — on `EntityKind::Agent`
- `AgentCondition` — on `EntityKind::Agent`
- `MetabolismProfile` — on `EntityKind::Agent`

## SystemFn Integration
- Implements the `SystemId::Needs` handler registered in `SystemDispatch`
- Runs once per tick for all living agents
- Reads: `HomeostaticNeeds`, `MetabolismProfile`, `WoundList` (E12, for pain derivation)
- Writes: `HomeostaticNeeds` (tick forward), `AgentCondition` (pain from wounds, fear decay)

## Cross-System Interactions (Principle 12)
All cross-system effects propagate through shared state, never through direct function calls:
- **E12 → E09**: Combat writes `WoundList` on agent; needs system reads `WoundList` to derive `AgentCondition.pain` (sum of wound severities mapped to Permille)
- **E09 → E13**: Decision architecture reads `HomeostaticNeeds` and `AgentCondition` via `BeliefView` to compute utility scores
- **E09 → E12**: High fatigue (read from `HomeostaticNeeds`) affects combat effectiveness (E12 reads it independently)

## FND-01 Section H

### Information-Path Analysis
- Metabolism is agent-local: each agent's `MetabolismProfile` drives their own `HomeostaticNeeds`. No external information required.
- Pain derivation: `WoundList` (written by combat at the agent's location) → needs system reads wounds on same agent → updates `AgentCondition.pain`. Information is local to the agent entity.
- Fear: perception events at agent's location → fear increases. Fear decays locally via `fear_decay_rate`. No global queries.

### Positive-Feedback Analysis
- **Fatigue → poor decisions → danger → fear → poor sleep → more fatigue**: fatigue impairs decision quality, leading to danger, which increases fear, which impairs sleep, which increases fatigue.
- **Pain → inability to heal → more pain**: pain from wounds may prevent agents from seeking healing, worsening their condition.

### Concrete Dampeners
- **Fatigue loop**: exhausted agents eventually collapse (forced sleep at critical threshold), which reduces fatigue regardless of fear. Physical collapse is the dampener — the agent cannot stay awake past biological limits.
- **Pain loop**: incapacitated agents (from wound severity, E12) can be healed by other co-located agents. The dampener is social: other agents observe and assist. Additionally, wounds that are not bleeding stabilize in severity over time.

### Stored State vs. Derived Read-Model
**Stored (authoritative)**:
- `HomeostaticNeeds` component (hunger, thirst, fatigue, bladder, dirtiness)
- `AgentCondition` component (pain, fear)
- `MetabolismProfile` component (per-agent rates)

**Derived (transient read-model)**:
- Urgency level (computed from need value vs. agent's `UrgencyThresholds`)
- Overall agent "wellness" (never stored; derived from needs + condition on demand)
- Whether an agent "needs to eat" (derived from hunger vs. threshold)

## Invariants Enforced
- 9.16: Need continuity — needs change only through time, consumption, rest, toileting, washing, injury, healing, or defined effects. No silent resets.
- 9.15: Off-camera continuity — needs progress regardless of visibility/camera

## Tests
- [ ] T15: Need progression — `Permille` values evolve by metabolism and time, not frame rate or camera
- [ ] T26: Camera independence — needs don't reset on visibility change
- [ ] Eating reduces hunger and consumes food (conservation maintained)
- [ ] Drinking reduces thirst and consumes water
- [ ] Sleeping reduces fatigue over duration (rate controlled by `rest_efficiency`)
- [ ] Toilet reduces bladder and produces waste entity
- [ ] Washing reduces dirtiness
- [ ] Need values stay within `Permille` range (0–1000) by type safety
- [ ] Different `MetabolismProfile` values produce different need progression rates
- [ ] Pain derived from `WoundList` severity sum, not independently ticked
- [ ] Fear decays at `fear_decay_rate` when no threat present
- [ ] No social_standing, loyalty, or wealth_pressure in needs
- [ ] `UrgencyThresholds` are per-agent, not global constants

## Acceptance Criteria
- `HomeostaticNeeds` (5 fields) and `AgentCondition` (2 fields) tracked per agent
- All consumption actions follow E07 action framework
- Conservation maintained through consumption
- All durations derived from profiles or `CommodityPhysicalProfile`, not hardcoded
- Needs drive urgency that feeds into decision making (E13)
- Different agents progress at different rates (Principle 11)

## Spec References
- Section 4.4 (needs list)
- Section 4.5 (core systems: consumption, rest, toilet, hygiene)
- Section 7.5 (physiological/social propagation channel)
- Section 9.15 (off-camera continuity)
- Section 9.16 (need continuity)
- `docs/FOUNDATIONS.md` Principles 3, 7, 8, 11
