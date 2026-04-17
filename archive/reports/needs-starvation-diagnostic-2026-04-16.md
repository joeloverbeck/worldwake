**Status**: COMPLETED

# Needs Starvation Diagnostic

## Outcome
- **Completion date**: 2026-04-16
- **What changed**: Report superseded by unified `/scenario-analysis` skill which replaces both `simulation-observer` and `needs-starvation-diagnostic` skills.
- **Deviations**: None — report served its purpose during survival-baseline validation.
- **Verification**: Scenario analysis skill created; old skills deleted; cross-references updated.

## Run Summary
- **Scenario**: `scenarios/survival-baseline.ron`
- **Seed**: 104004
- **Ticks simulated**: 1440
- **Agents**: Agent A (Riverside Camp), Agent B (Riverside Camp), Agent C (Forest Clearing)
- **Places**: Riverside Camp, Fertile Fields, Forest Clearing, Hillside Shelter
- **Deaths**: None

### Pre-flight Warnings
1. **No WashBasin anywhere** — dirtiness predicted structurally unsatisfiable. Actual result: agents wash 2-3 times each using Wells, keeping dirtiness below 635 permille. The Well workstation apparently supports the Wash action.
2. **No food at starting locations** — Apples only at Fertile Fields. Agents successfully travel there (6-7 travel actions each) and spend the majority of their time at Fertile Fields (1102-1283 ticks).
3. **Hillside Shelter is barren** — no agents visited it during the run.

## Agent Needs Overview

| Agent | Need | Max Value | Ticks >750 | Death? | Root Cause Category |
|-------|------|-----------|------------|--------|---------------------|
| Agent A | all needs managed | max 490 hunger, 627 dirtiness | 0 | No | N/A (healthy) |
| Agent B | all needs managed | max 506 hunger, 635 dirtiness | 0 | No | N/A (healthy) |
| Agent C | all needs managed | max 434 hunger, 588 dirtiness | 0 | No | N/A (healthy) |

## Failure Classifications

**No needs failures detected.** All three agents successfully managed all five needs throughout the 1440-tick simulation.

### Agent A (healthy)
- **Location strategy**: Started at Riverside Camp, migrated to Fertile Fields by tick ~15, spent 1173/1440 ticks there
- **Food**: 30 eat actions, 15 Harvest Apples, 18 pick_up — self-sufficient at Fertile Fields
- **Water**: 4 drink actions, 3 Harvest Water — adequate hydration
- **Hygiene**: 2 wash actions — kept dirtiness below 627 permille
- **Planner**: 249 plans found, 0 budget-exhausted, 0 frontier-exhausted

### Agent B (healthy)
- **Location strategy**: Started at Riverside Camp, migrated to Fertile Fields, spent 1283/1440 ticks there
- **Food**: 32 eat actions, 16 Harvest Apples, 21 pick_up — self-sufficient
- **Water**: 3 drink actions, 3 Harvest Water — adequate
- **Hygiene**: 3 wash actions — kept dirtiness below 635 permille
- **Planner**: 253 plans found, 0 budget-exhausted, 0 frontier-exhausted

### Agent C (healthy)
- **Location strategy**: Started at Forest Clearing, migrated to Fertile Fields, spent 1102/1440 ticks there
- **Food**: 31 eat actions, 16 Harvest Apples, 23 pick_up — self-sufficient
- **Water**: 11 drink actions, 7 Harvest Water — highest water consumption (thirst_rate: 4)
- **Hygiene**: 3 wash actions — kept dirtiness below 588 permille
- **Planner**: 272 plans found, 0 budget-exhausted, 0 frontier-exhausted

## Damning Moments

**None.** No agent experienced needs above 750 permille for 100+ consecutive ticks.

## Proposed Solutions

**No solutions needed** — the scenario achieves its stated purpose of proving agents can bootstrap food/water access through normal perception and exploration.

### Observations for Future Scenarios

1. **Dirtiness approaching threshold**: All agents reached 588-635 permille dirtiness (max). With only 2-3 wash actions over 1440 ticks, a scenario with higher `dirtiness_rate` or without Wells (which appear to support washing) could trigger dirtiness starvation.

2. **Fertile Fields gravity well**: All agents converge on Fertile Fields (the only food source) and spend 76-89% of their time there. This is rational but means the other locations serve only as transit points. A scenario testing distributed survival would need food at multiple locations.

3. **Low water consumption**: Agents drink 3-11 times over 1440 ticks despite thirst_rate of 3-4. This suggests the Harvest Water action or drink action satisfies large amounts of thirst per action.

4. **No exploration of Hillside Shelter**: The barren location was never visited, which is correct behavior (no resources to attract agents).

## Non-Needs Anomalies (out of scope)

For completeness, 19 anomalies were detected — all REDUNDANT_PERCEPTION (16) or STUCK_AGENT (3, max 41 idle ticks). These are not needs-related and would be analyzed by the `simulation-observer` skill.

## Golden Test Recommendations

No golden tests needed for needs starvation — all agents are healthy. The scenario validates the survival baseline successfully.

| Priority | Observation | Potential Future Test | What It Would Guard Against |
|----------|------------|----------------------|----------------------------|
| Low | Dirtiness reaches 627-635 permille | golden_wash_frequency_under_pressure | Regression where wash frequency drops and dirtiness crosses 750 |
| Low | All agents converge on Fertile Fields | golden_multi_location_survival | Regression where agents fail to travel to food sources |
