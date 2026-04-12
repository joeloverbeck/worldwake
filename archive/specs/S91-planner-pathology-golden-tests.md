# S91 — Planner Pathology Golden Tests

**Status**: ✅ COMPLETED

## Problem Statement

The simulation observer report (`archive/reports/simulation-observer-report.md`, seed 7777, scenario `cli-evaluation.ron`) identifies three root-cause planner pathologies that render the simulation non-functional:

1. **Budget exhaustion on multi-location acquisition (CRITICAL)**: AcquireCommodity{Water} generates 1400–2600+ candidates per expansion, exhausting the 224-node budget at depth 0–1. Agents at resource-poor locations (Dusty Trail) cannot plan travel→acquire→consume chains. Three agents slowly starve. S88/S89/S90 introduced strategic+tactical layering but the combinatorial explosion persists in the tactical phase.

2. **Degenerate 0-step plan loop (CRITICAL)**: Forager Lina's FreeCarryCapacity goal returns `GoalSatisfied[steps=0]` every tick (score 280000) when her inventory is full of Waste. The 0-step plan produces no executable action, yet its priority blocks all other goals (eat, drink, sleep). 708 consecutive idle ticks result.

3. **Missing survival goal generation for role agents (CRITICAL)**: Guard Theron's patrol/investigation role goals dominate goal selection. No hunger or thirst goal is ever generated despite needs reaching 900+. He dies at tick 422 from NeedDeprivation{Hunger} without a single eat/drink attempt.

These pathologies are not covered by existing golden tests. Without targeted reproduction tests, fixes cannot be verified and regressions cannot be prevented.

## Deliverables

**D1**: Add 3 golden E2E tests under `crates/worldwake-ai/tests/golden_planner_pathology.rs`, one per root-cause pathology.

Each test uses a minimal isolated scenario distilled from the motivating observer evidence and the existing `GoldenHarness` infrastructure. Tests follow a dual-assertion strategy:

- **Phase 1 (bug reproduction)**: Assertions prove the pathology exists. These tests pass today.
- **Phase 2 (fix verification)**: Assertions prove the pathology is resolved. These replace Phase 1 assertions when the corresponding fix lands, becoming permanent regression guards.

Phase 1 and Phase 2 assertions are both specified below. Implementation initially uses Phase 1. Each assertion set is wrapped in a clearly labeled block comment so the flip is a mechanical edit.

## Test Designs

### Test 1: `budget_exhaustion_blocks_cross_location_water_acquisition`

**Pathology reproduced**: Budget exhaustion on AcquireCommodity{Water} when water is at a remote location.

**Scenario setup**:
- use the `cli-evaluation.ron` place graph slice: `Thornwall Village`, `Dusty Trail`, `Eldergrove Forest`, `Hearthstone Inn`, `Golden Fields`
- keep the live `Thornwall Village <-> Dusty Trail` edge at `travel_ticks: 2`
- place the named village well at `Thornwall Village` with a water `ResourceSource`
- use a Dusty Trail guard-style agent boundary (patrol/perception/utility shape distilled from Guard Theron) with elevated thirst so remote water search is exercised immediately
- seed local Dusty Trail beliefs plus inferred beliefs about `Thornwall Village` and the village well

**Tick count**: 60 ticks (enough to observe the immediate Dusty Trail water-search failure pattern).

**Phase 1 assertions (bug reproduction — tests pass today)**:
1. At tick 0, the `PlanningPipelineTrace` contains at least one `PlanAttemptTrace` where `goal.kind` matches `AcquireCommodity { commodity: Water, .. }`.
2. That attempt's `outcome` is `PlanSearchOutcome::BudgetExhausted { .. }`.
3. No committed `drink` occurs during the run.
4. After 60 ticks, agent's `HomeostaticNeeds.thirst` is higher than the initial level.

**Phase 2 assertions (fix verification — replace Phase 1 when fix lands)**:
1. Within the first 10 ticks, a `PlanAttemptTrace` for `AcquireCommodity { commodity: Water, .. }` has outcome `PlanSearchOutcome::Found { steps, .. }` where `steps.len() >= 2` (at minimum: travel + drink).
2. A travel/drink chain commits successfully.
3. Within 60 ticks, agent's `HomeostaticNeeds.thirst` is lower than the initial level.

**Trace inspection pattern**:
```rust
fn planning_trace_at(h: &GoldenHarness, agent: EntityId, tick: Tick)
    -> Option<&PlanningPipelineTrace>
{
    let trace = h.driver.trace_sink()?.trace_at(agent, tick)?;
    match &trace.outcome {
        DecisionOutcome::Planning(planning) => Some(planning),
        _ => None,
    }
}
```
Access plan attempts via `planning.planning.attempts` and check each `PlanAttemptTrace.outcome`.

---

### Test 2: `degenerate_zero_step_loop_blocks_actionable_goals`

**Pathology reproduced**: FreeCarryCapacity selected every tick with 0-step GoalSatisfied, blocking urgent eat goal.

**Scenario setup**:
- use the live `cli-evaluation.ron` Eldergrove Forest slice rather than a synthetic one-place proxy
- keep Forager Lina at `Eldergrove Forest` with the scenario's exact startup values relevant to the pathology:
  - `HomeostaticNeeds { hunger: 200, thirst: 600, fatigue: 100, bladder: 100, dirtiness: 100 }`
  - scenario `UtilityProfile`, `MetabolismProfile`, `DriveThresholds`, `ExplorationProfile`, `DisposalProfile`, `PreferenceProfile`, `carry_capacity: 20`, and `KnownRecipes: ["Harvest Apples"]`
- keep the scenario-local Eldergrove substrate:
  - 8 ground Apples
  - 5 ground Water
  - `ChoppingBlock`
  - named orchard `Eldergrove Orchard`
  - Apple `ResourceSource` at that orchard (`regeneration_ticks_per_unit: 2`, `capacity: 20`)
- seed only Lina's local Eldergrove beliefs at tick 0, matching the observer report's “knows Eldergrove Forest contents” boundary

**Tick count**: long-horizon run using the scenario seed `7777`, with assertions evaluated over a late-run observation window after the report's waste-accumulation phase (loop onset observed around tick ~730).

**Phase 1 assertions (bug reproduction — tests pass today)**:
1. In the late-run observation window after accumulation, `GoalKind::FreeCarryCapacity` dominates selected-goal frequency.
2. In that same late-run window, repeated `FreeCarryCapacity` attempts have outcome `PlanSearchOutcome::Found { steps, .. }` where `steps.is_empty()` (0-step satisfied plan).
3. No `eat` action commits once the loop window begins, and `HomeostaticNeeds.hunger` is higher at the end of that window than at its start.

**Phase 2 assertions (fix verification — replace Phase 1 when fix lands)**:
1. FreeCarryCapacity either produces an executable plan with `steps.len() >= 1` (drop waste action), OR other goals (ConsumeOwnedCommodity/AcquireCommodity for food) are selected when FreeCarryCapacity yields 0-step plans.
2. Within 30 ticks, agent's `HomeostaticNeeds.hunger` decreases (ate something).
3. Agent is not idle for more than 10 consecutive ticks.

---

### Test 3: `role_agent_generates_survival_goals_under_critical_needs`

**Pathology reproduced**: Guard agent with patrol profile never generates hunger/thirst goals, dying from NeedDeprivation.

**Scenario setup**:
- 2 places: `Trail` (tags: `[Trail, Road]`), `Village` (tags: `[Village]`)
- 1 bidirectional edge: Trail ↔ Village, `travel_ticks: 2`
- 5 Bread items at Village (food available)
- 5 Water items at Village
- 1 AI agent ("Guard") at Trail:
  - `HomeostaticNeeds { hunger: 600, thirst: 600, fatigue: 100, bladder: 100, dirtiness: 100 }`
  - `CombatProfile` (standard guard: `attack_skill: 600`, `guard_skill: 550`, etc.)
  - `PatrolProfile { base_dwell_ticks: 5, dwell_vigilance_scale_ticks: 3, vigilance: 700, route_adaptation_sensitivity: 400, patrol_motive_weight: 600 }`
  - `PatrolRoute { assigned_places: [Trail, Village] }`
  - `UtilityProfile` with `hunger_weight: 400, thirst_weight: 400, danger_weight: 800`
  - `MetabolismProfile` with `hunger_rate: 3, thirst_rate: 3, starvation_tolerance_ticks: 200, dehydration_tolerance_ticks: 150`
  - `DriveThresholds` with hunger/thirst thresholds: `low: 250, medium: 500, high: 750, critical: 900`
  - `PerceptionProfile` (standard)
  - `CognitiveProfile` with defaults
  - Beliefs seeded: agent knows Village exists, knows Bread and Water at Village

**Tick count**: 250 ticks (enough to reach starvation_tolerance if no eating occurs, but well before death if eating works).

**Phase 1 assertions (bug reproduction — tests pass today)**:
1. Collect all selected goals across 250 ticks. Assert that no tick selects a goal with `GoalKind::AcquireCommodity { commodity: CommodityKind::Bread | CommodityKind::Water, .. }` or `GoalKind::ConsumeOwnedCommodity { commodity: CommodityKind::Bread | CommodityKind::Water, .. }`.
2. The selected goals are dominated by `Patrol`, `Sleep`, and `Relieve` (these three account for >90% of selected goals).
3. Agent either dies from `NeedDeprivation` or has hunger > 900 by tick 250.

**Phase 2 assertions (fix verification — replace Phase 1 when fix lands)**:
1. At least one tick within the first 150 ticks selects a hunger or thirst-related goal (`AcquireCommodity` or `ConsumeOwnedCommodity` for food/water).
2. Survival goals outrank Patrol when hunger or thirst exceeds the `high` threshold (750).
3. Agent is alive at tick 250 with hunger < 900.

## Harness Reuse

All tests reuse the existing `GoldenHarness` from `crates/worldwake-ai/tests/golden_harness/mod.rs`:

- **World construction**: custom topology overrides when the motivating observer condition depends on non-prototype places, `new_txn()` + `commit_txn()` for entity setup
- **Belief seeding**: `seed_actor_beliefs()` to give agents knowledge of remote locations/resources
- **Tick stepping**: `h.step_once()` in a loop
- **Trace inspection**: `h.driver.trace_sink()?.trace_at(agent, tick)` → `DecisionOutcome::Planning(planning)` → `planning.planning.attempts` for plan search outcomes, `planning.selection.selected_goal()` for goal selection
- **State inspection**: `h.world.get_component_homeostatic_needs(agent)` for need levels, `h.world.get_component_location(agent)` or location relation for position checks

Helper functions (`planning_trace_at`, `selected_goal_sequence`) follow patterns from `conformance_execution_budget.rs` and `golden_expectation.rs`.

## Ticket Decomposition

| Ticket | Test | Depends On |
|--------|------|-----------|
| S91-001 | `budget_exhaustion_blocks_cross_location_water_acquisition` | — |
| S91-002 | `degenerate_zero_step_loop_blocks_actionable_goals` | — |
| S91-003 | `role_agent_generates_survival_goals_under_critical_needs` | — |

All three tickets are independent and can be implemented in parallel. Each ticket adds its own test function to `golden_planner_pathology.rs`, implements the Phase 1 assertions, and verifies the focused test plus `cargo test -p worldwake-ai`.

## Section H: FND-01 Analysis

### Information-path analysis (Principle 7)

These are test-only constructs. Agent beliefs are seeded explicitly via `seed_actor_beliefs()`, which traces through the standard belief update path (builds `BelievedEntityState` from world state at a declared observation tick with a declared `PerceptionSource`). No omniscient knowledge injection — agents know only what is explicitly seeded. The tests do not add any new information propagation paths to the engine.

### Positive-feedback analysis (Principle 11)

No positive-feedback loops are introduced. These are read-only diagnostic tests that set up a scenario, run ticks, and inspect traces. No new amplifying dynamics are created.

### Concrete dampeners

Not applicable — no feedback loops introduced.

### Stored state vs. derived read-model list (Principle 3)

**Stored state (test setup only)**:
- `HomeostaticNeeds` on test agents (existing component)
- `CognitiveProfile`, `PatrolProfile`, `PatrolRoute`, `CombatProfile`, `DisposalProfile`, `CarryCapacity` on test agents (existing components)
- `AgentBeliefStore` on test agents (seeded via existing `seed_actor_beliefs`)
- Item entities (Waste, Apple, Bread, Water) at test locations (existing item creation)
- Facility entities (Well, OrchardRow) at test locations (existing workstation creation)
- `ResourceSource` at test locations (existing component)

**Derived (read-only, never stored)**:
- `PlanningPipelineTrace` / `PlanAttemptTrace` / `PlanSearchOutcome` — decision trace output inspected by assertions
- `SelectionTrace.selected_goal()` — derived from trace, not stored
- Goal selection frequency counts — computed by test assertions
- Location checks, need level reads — read from authoritative world state

No new components, relations, or stored state are introduced by this spec.

## Outcome

- **Completion date**: 2026-04-11
- **What actually changed**:
  - landed the planner-pathology golden proof surface in `crates/worldwake-ai/tests/golden_planner_pathology.rs`
  - Scenario 142 now proves the former Dusty Trail remote-water pathology is fixed
  - Scenario 143 now proves the former `FreeCarryCapacity` zero-step loop is fixed
  - reassessment of the third intended pathology showed the original Guard Theron "no survival goals generated" claim is not reproducible on the current branch in the scenario-adjacent slice, so that ticket was closed as stale instead of landing a false golden
- **Deviations from original plan**:
  - Deliverable D1 finished with two shipped golden scenarios and one stale-ticket closeout rather than three landed pathologies.
  - The original Test 3 design was too approximate to the live `cli-evaluation` conditions and, once corrected toward the real scenario boundary, no longer reproduced the reported failure.
- **Verification results**:
  - `cargo test -p worldwake-ai --test golden_planner_pathology cross_location_water_acquisition_succeeds_without_budget_exhaustion`
  - `cargo test -p worldwake-ai --test golden_planner_pathology degenerate_zero_step_loop_blocks_actionable_goals`
  - `cargo test -p worldwake-ai role_agent_generates_survival_goals_under_critical_needs` used as a reassessment probe and disproved the original pathology on the current branch
