# S88TWOPHALAN-006: Implement strategic planner

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: None — new module internal to worldwake-ai planner
**Deps**: None

## Problem

The current planner performs flat A* search over all actions at all locations, producing 1400–2600 candidates per expansion. Multi-location goals (travel + acquire + consume) are never found within the expansion budget. The strategic planner (S88 D3) decomposes these goals into an ordered location-visit itinerary based on agent beliefs, so each tactical sub-problem operates in a locality-scoped domain with far fewer candidates.

## Assumption Reassessment (2026-04-11)

1. `PlanningSnapshot` is the agent's belief surface, used by `search_plan()` at `search/mod.rs:80`. It provides `min_perceived_travel_cost_to_any()` for travel cost estimation. The strategic planner will use this same interface.
2. `goal_relevant_places()` and `prerequisite_places()` exist on `GoalKindPlannerExt` in `goal_model.rs`, used by `combined_relevant_places()` in `heuristic.rs:66,80`. Both helpers require `RecipeRegistry`, so the strategic planner should accept the live recipes dependency rather than silently degrading recipe-driven goals.
3. `GroundedGoal` is passed to `search_plan()` at `search/mod.rs:81`. The strategic planner receives the same goal to determine what sub-goals are needed at each location.
4. The live `search_plan()` boundary does not receive `ExplorationProfile`, and neither `PlanningSnapshot` nor `PlanningState` exposes it. For this ticket, exploration fallback must stay within the existing planner contract: it derives exploration destinations from the actor's believed current place and believed adjacent places already present in `PlanningSnapshot`.
5. `TacticalSubGoal::SocialQuery` references `CommodityKind` from `worldwake-core`. This is implementable for commodity-driven goals (`AcquireCommodity`, `ConsumeOwnedCommodity`, `RestockCommodity`, `TreatWounds`, and recipe-input acquisition for `ProduceCommodity`) by asking about the missing commodity; non-commodity prerequisite cases stay out of scope for this ticket.

## Architecture Check

1. A standalone `strategic.rs` module with a pure function `plan(snapshot, goal, execution_budget, recipes) -> Option<StrategicPlan>` is the cleanest design. The function operates entirely on belief data (FND-14), uses only Travel as an abstract operator over the believed place graph (FND-7), and returns a data structure with no side effects.
2. No backwards-compatibility shims. This is entirely new code. The module is imported by S88TWOPHALAN-007 for integration.

## Verification Layers

1. Strategic plan correctness → focused unit tests (produces correct destination sequence for multi-location goals)
2. Belief-only compliance (FND-14) → focused unit test (agent with no beliefs about location C does not include C in itinerary)
3. Exploration fallback → focused unit test (no known relevant location produces adjacent-place exploration itinerary from `PlanningSnapshot`)
4. Social-query fallback → focused unit test (commodity-driven goal with co-located agents but no known source produces `SocialQuery`)
5. Empty strategic plan → focused unit test (no known locations, no reachable exploration destination, no commodity query target → returns `None`)
6. Single-layer ticket (planner-internal module) — no cross-layer mapping needed.

## What to Change

### 1. Create `crates/worldwake-ai/src/search/strategic.rs`

**Types**:

```rust
pub(crate) struct StrategicPlan {
    pub steps: Vec<StrategicStep>,
}

pub(crate) struct StrategicStep {
    pub destination: EntityId,
    pub sub_goal: TacticalSubGoal,
    pub estimated_travel_ticks: u32,
}

pub(crate) enum TacticalSubGoal {
    SatisfyGoal,
    AcquirePrerequisite(CommodityKind),
    Explore,
    SocialQuery(CommodityKind),
}
```

**Main function**:

```rust
pub(crate) fn plan(
    snapshot: &PlanningSnapshot,
    goal: &GroundedGoal,
    execution_budget: &ExecutionBudget,
    recipes: &RecipeRegistry,
) -> Option<StrategicPlan>
```

**Algorithm** (best-first search over believed place graph):
1. Determine goal-relevant places from `goal.key.kind.goal_relevant_places()` and `prerequisite_places()` using the live `RecipeRegistry`.
2. If no relevant places are known: attempt exploration itinerary from believed adjacent places in `PlanningSnapshot`, or a commodity-backed social query when the goal exposes a missing commodity. If neither applies, return `None`.
3. Search over state `(believed_location, unvisited_relevant_places)` with Travel as the only operator. Cost = `min_perceived_travel_cost`. Budget = `max_prerequisite_locations * 2` expansions.
4. Build `StrategicPlan` from the search path, assigning `TacticalSubGoal` per destination based on what the goal needs there (prerequisite resource → `AcquirePrerequisite`, final goal location → `SatisfyGoal`).

### 2. Register module in `crates/worldwake-ai/src/search/mod.rs`

Add `pub(crate) mod strategic;` to the module declarations.

### 3. Write focused unit tests

Tests within `strategic.rs`:

- `test_single_location_goal_no_travel` — goal satisfiable at current location → returns empty strategic plan (no travel needed) or None
- `test_multi_location_prerequisite_then_goal` — resource at location B, goal at location C → strategic plan: B (AcquirePrerequisite), C (SatisfyGoal)
- `test_belief_only_excludes_unknown_locations` — location with resource exists in world but NOT in agent's beliefs → not included in plan (FND-14)
- `test_empty_beliefs_exploration_fallback` — no known locations with resource → produces Explore itinerary for believed adjacent places
- `test_social_query_when_colocated_agents` — commodity-driven goal with no known resource locations but co-located agents → includes SocialQuery step
- `test_no_fallback_returns_none` — no known locations, no exploration targets, and no commodity-backed social query target → returns None
- `test_estimated_travel_ticks_from_beliefs` — travel cost estimates match `min_perceived_travel_cost` from snapshot

## Files to Touch

- `crates/worldwake-ai/src/search/strategic.rs` (new)
- `crates/worldwake-ai/src/search/mod.rs` (modify — add module declaration)

## Out of Scope

- Wiring strategic plan into `search_plan()` loop (S88TWOPHALAN-007)
- Modifying candidate generation or the tactical search (S88TWOPHALAN-007)
- Landmark extraction (S88TWOPHALAN-003)
- Decision trace enrichment with strategic plan data (S88TWOPHALAN-008)

## Acceptance Criteria

### Tests That Must Pass

1. All 7+ focused unit tests for strategic planning
2. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. Strategic planner never accesses world truth — only `PlanningSnapshot` beliefs (FND-14)
2. Locations not in agent's belief store are never included in the itinerary (FND-7)
3. Budget is bounded by `max_prerequisite_locations * 2` expansions
4. Empty strategic plan means goal is satisfiable at current location or no belief-backed strategic step exists within the current planner surface

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/search/strategic.rs` (inline tests) — strategic planning correctness, belief-only compliance, fallback behavior

### Commands

1. `cargo test -p worldwake-ai -- strategic`
2. `cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`

## Outcome

Completed on 2026-04-11.

- Added planner-internal `crates/worldwake-ai/src/search/strategic.rs` with
  staged `StrategicPlan`, `StrategicStep`, and `TacticalSubGoal` types plus a
  pure belief-backed `plan(snapshot, goal, execution_budget, recipes)` entry
  point.
- Registered the staged module in `crates/worldwake-ai/src/search/mod.rs` and
  deliberately marked the new module `#![allow(dead_code)]` because live
  `search_plan()` integration remains owned by `S88TWOPHALAN-007`.
- Implemented strategic staging for missing prerequisite commodities and final
  goal locations, including multi-hop itinerary search, adjacent-place
  exploration fallback, and commodity-backed social-query fallback.
- Corrected two ticket assumptions during reassessment so the landed contract
  matches the live codebase: recipe-driven goal support requires `RecipeRegistry`,
  and exploration fallback cannot depend on `ExplorationProfile` because that
  profile is not present on the current planner boundary.
- Added eight focused inline tests in `strategic.rs` covering local no-travel
  goals, prerequisite-then-goal sequencing, belief-only exclusion, exploration
  fallback, social-query fallback, no-fallback `None`, travel-cost estimation,
  and recipe-input social-query selection.

## Verification Result

- Passed `cargo test -p worldwake-ai -- strategic`
- Passed `cargo clippy --workspace --all-targets -- -D warnings`
- Passed `cargo test --workspace`
