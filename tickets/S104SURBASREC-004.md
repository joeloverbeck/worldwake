# S104SURBASREC-004: Create survival baseline scenario

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: None
**Deps**: S104SURBASREC-003

## Problem

No scenario proves agents can bootstrap survival from realistic starting conditions. Existing scenarios (`default.ron`, `cli-evaluation.ron`) give agents role-specific profiles and place them adjacent to resources. The survival baseline scenario strips agents to survival-only profiles and creates geographic tension requiring exploration, covering all 5 homeostatic needs (Hunger, Thirst, Fatigue, Bladder, Dirtiness). This is the foundation for FND-06 (World Runs Without Observers) validation.

## Assumption Reassessment (2026-04-15)

1. Existing scenarios in `scenarios/`: `default.ron` and `cli-evaluation.ron` — confirmed. No `survival-baseline.ron` exists yet.
2. Required facility types validated against codebase:
   - `WorkstationTag::Well` — exists in `crates/worldwake-core/src/production.rs`
   - `WorkstationTag::FieldPlot` — exists
   - `WorkstationTag::OrchardRow` — exists
   - `WorkstationTag::WashBasin` — exists (not required for Wash action but available)
   - `PlaceTag::Latrine` — exists in `crates/worldwake-core/src/topology.rs`
   - `PlaceTag::Forest`, `PlaceTag::Field`, `PlaceTag::Farm` — exist as OUTDOOR_RELIEF_TAGS
   - `PlaceTag::Camp` — exists
3. Recipe names validated: `"Harvest Grain"`, `"Harvest Apples"`, `"Harvest Water"` — confirmed in existing scenario files and action registry.
4. `HomeostaticNeedId` has 5 variants: Hunger, Thirst, Fatigue, Bladder, Dirtiness — confirmed in `crates/worldwake-core/src/needs.rs`.
5. Need satisfaction mechanisms validated:
   - Hunger: Eat action (consume Apple, Grain, Bread) — no facility required, needs food in inventory
   - Thirst: Drink action (consume Water) — no facility required, needs water in inventory
   - Fatigue: Sleep action — no facility required, agents sleep anywhere
   - Bladder: Toilet (PlaceTag::Latrine) or Relieve Wilderness (OUTDOOR_RELIEF_TAGS: Forest, Trail, Field, Farm, Road)
   - Dirtiness: Wash action — consumes 1 Water from inventory, no facility required
6. `AgentDef` has optional fields for all required profiles: `metabolism_profile`, `perception_profile`, `exploration_profile`, `cognitive_profile`, `drive_thresholds` — confirmed in `crates/worldwake-cli/src/scenario/types.rs`.

## Architecture Check

1. The scenario uses only existing facility types, place tags, and recipes — no new engine types needed. It follows the established RON scenario format from `default.ron` and `cli-evaluation.ron`.
2. No backwards-compatibility shims. Pure scenario authoring within the existing framework.

## Verification Layers

1. Scenario loads without error → `observer scenarios/survival-baseline.ron --ticks 10` completes
2. All 5 needs are satisfiable → observer run at 1440 ticks meets success criteria
3. Single-layer ticket — scenario authoring only, no engine changes.

## What to Change

### 1. Create `scenarios/survival-baseline.ron`

Write a RON scenario file with:

**Places (4):**
- **Riverside Camp** (tags: Camp, Latrine): Well + Water resource source. Starting location for Agents A, B.
- **Fertile Fields** (tags: Field, Farm): FieldPlot + OrchardRow. Grain + Apple resource sources. 3 ticks from Riverside.
- **Forest Clearing** (tags: Forest): Well + Water resource source. 4 ticks from Riverside, 5 from Fields.
- **Hillside Shelter** (tags: Camp, Latrine): No workstations. 3 ticks from Forest, 6 from Riverside.

**Agents (3):**
Each with ONLY survival profiles: `MetabolismProfile`, `PerceptionProfile`, `ExplorationProfile`, `CognitiveProfile`, `DriveThresholds`. Varied metabolism rates for agent diversity (FND-22). No role-specific profiles (no PatrolProfile, TellProfile, CombatProfile, etc.).

**Recipes:**
All agents know: `"Harvest Grain"`, `"Harvest Apples"`, `"Harvest Water"`.

**Starting knowledge:**
- Agent A: knows Riverside Camp and Fertile Fields
- Agent B: knows only Riverside Camp (must explore for food)
- Agent C: starts at Forest Clearing, knows Forest and Hillside Shelter (must explore for food)

**Resource sources:**
- Water at Riverside Camp (via Well) with regeneration
- Water at Forest Clearing (via Well) with regeneration
- Grain at Fertile Fields (via FieldPlot) with regeneration
- Apple at Fertile Fields (via OrchardRow) with regeneration

Follow the format conventions from `scenarios/cli-evaluation.ron` for all field structures.

## Files to Touch

- `scenarios/survival-baseline.ron` (new)

## Out of Scope

- Modifying existing scenarios
- Adding new facility types or place tags
- Changing the observer binary
- Creating golden tests from this scenario (handled by S104SURBASREC-005)
- Modifying agent metabolism or needs systems

## Acceptance Criteria

### Tests That Must Pass

1. Scenario loads: `observer scenarios/survival-baseline.ron --ticks 10` completes without error
2. Observer validation at full duration: `observer scenarios/survival-baseline.ron --ticks 1440` produces:
   - Zero deaths
   - No need sustained above Permille 750 for more than 100 consecutive ticks (all 5 needs)
   - All agents execute at least one eat, drink, wash, sleep, and relieve action
   - Agent B explores to discover a food source
   - No planner budget exhaustion on survival-related goals
   - No idle stretches > 50 ticks or action loops flagged as anomalies

### Invariants

1. All 5 homeostatic needs are satisfiable within the scenario's place graph
2. Every agent can reach at least one food source, water source, and bladder relief location from their starting position
3. No agent requires role-specific profiles to survive

## Test Plan

### New/Modified Tests

1. None — scenario file only; verification is via observer run and subsequent golden tests (S104SURBASREC-005).

### Commands

1. `observer scenarios/survival-baseline.ron --ticks 10` — smoke test: loads and runs
2. `observer scenarios/survival-baseline.ron --ticks 1440 --output reports/survival-baseline-validation.md` — full validation run
3. `cargo build --workspace` — workspace still compiles (sanity check)
