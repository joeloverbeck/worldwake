# E09: Needs & Metabolism

## Epic Summary
Implement the agent needs system with tick-based progression, consumption actions, and urgency thresholds.

## Phase
Phase 2: Survival & Logistics

## Crate
`worldwake-systems`

## Dependencies
- E08 (scheduler drives tick-based progression)

## Deliverables

### Needs Component
Per spec section 4.4, each agent tracks:
- `hunger: f32` (0.0 = full, 1.0 = starving)
- `thirst: f32` (0.0 = hydrated, 1.0 = dehydrated)
- `fatigue: f32` (0.0 = rested, 1.0 = exhausted)
- `bladder: f32` (0.0 = empty, 1.0 = desperate)
- `hygiene: f32` (1.0 = clean, 0.0 = filthy)
- `pain: f32` (0.0 = none, 1.0 = agony)
- `fear: f32` (0.0 = calm, 1.0 = terrified)
- `social_standing: f32` (0.0 = outcast, 1.0 = respected)
- `loyalty: f32` (0.0 = disloyal, 1.0 = devoted)
- `wealth_pressure: f32` (0.0 = comfortable, 1.0 = desperate)

### Metabolism System
- Per-agent metabolism rates (configurable per agent type)
- Tick-based progression:
  - Hunger increases by `hunger_rate` per tick
  - Thirst increases by `thirst_rate` per tick
  - Fatigue increases by `fatigue_rate` per tick (faster when active)
  - Bladder increases by `bladder_rate` per tick (faster after eating/drinking)
  - Hygiene decreases by `hygiene_decay_rate` per tick
- Rates are deterministic (no RNG in base metabolism)

### Consumption Actions (ActionDef instances)
All follow the E07 action framework:

- **Eat**: actor has food item → consume → reduce hunger, increase bladder slightly
  - Precondition: actor possesses food, hunger > threshold
  - Effect: remove/reduce food lot, decrease hunger
  - Duration: 5-10 ticks

- **Drink**: actor has water → consume → reduce thirst, increase bladder
  - Precondition: actor possesses water
  - Effect: remove/reduce water lot, decrease thirst
  - Duration: 2-5 ticks

- **Rest/Sleep**: actor at bed/shelter → sleep → reduce fatigue
  - Precondition: actor at place with bed/shelter, fatigue > threshold
  - Reservation: bed/sleeping spot
  - Effect: decrease fatigue over duration
  - Duration: 60-480 ticks (1-8 hours)
  - Interruptibility: ByDanger, ByUrgentNeed

- **Toilet**: actor at latrine/facility → relieve → reduce bladder, produce waste
  - Precondition: actor at place with toilet OR wilderness
  - Reservation: toilet stall (if facility)
  - Effect: decrease bladder, create waste entity at location
  - Duration: 5-15 ticks

- **Wash**: actor at water source → wash → improve hygiene
  - Precondition: actor at place with water source
  - Effect: increase hygiene, consume small amount of water
  - Duration: 10-20 ticks

### Need Thresholds & Urgency
- Each need has urgency thresholds:
  - `low` (0.3): minor discomfort, may seek to address
  - `medium` (0.6): significant pressure, will prioritize
  - `high` (0.8): urgent, overrides most other goals
  - `critical` (0.95): emergency, overrides everything except immediate danger
- Urgency states feed into utility scoring (E13)

## Invariants Enforced
- 9.16: Need continuity - needs change only through time, consumption, rest, toileting, washing, injury, healing, or defined effects. No silent resets.
- 9.15: Off-camera continuity - needs progress regardless of visibility/camera

## Tests
- [ ] T15: Need progression - values evolve by metabolism and time, not frame rate or camera
- [ ] T26: Camera independence - needs don't reset on visibility change
- [ ] Eating reduces hunger and consumes food (conservation maintained)
- [ ] Drinking reduces thirst and consumes water
- [ ] Sleeping reduces fatigue over duration
- [ ] Toilet reduces bladder and produces waste entity
- [ ] Washing improves hygiene
- [ ] Need values clamped to [0.0, 1.0]
- [ ] Metabolism rates configurable per agent
- [ ] Urgency thresholds correctly computed

## Acceptance Criteria
- All 10 needs tracked and progress per tick
- All consumption actions follow E07 action framework
- Conservation maintained through consumption
- Needs drive urgency that feeds into decision making

## Spec References
- Section 4.4 (needs list)
- Section 4.5 (core systems: consumption, rest, toilet, hygiene)
- Section 7.5 (physiological/social propagation channel)
- Section 9.15 (off-camera continuity)
- Section 9.16 (need continuity)
