**Status**: PENDING

# Planner Prerequisite-Aware Search

## Summary

The GOAP planner cannot compose multi-step plans that require visiting intermediate locations to acquire prerequisites before reaching the goal-terminal location. For example, a `TreatWounds { patient }` goal where the patient is local but medicine is at a remote location requires the plan Travel(remote)→PickUp(medicine)→Travel(patient)→Heal — but the planner never finds this plan because:

1. **Single-terminal heuristic**: `goal_relevant_places()` returns only the patient's location. The A* heuristic has no concept of intermediate resource locations.
2. **Spatial pruning eliminates prerequisite travel**: `prune_travel_away_from_goal()` removes travel candidates that move the actor away from goal-relevant places. Traveling to get medicine is pruned because it increases distance from the patient.
3. **Budget exhaustion**: Even if pruning didn't eliminate the path, the combinatorial search space means 4-step cross-domain plans exhaust `max_node_expansions: 512` before being found.

This limits Principle 1 (Maximal Emergence): complex causal chains that require resource procurement before goal achievement cannot emerge from the planner. Agents fall back to sequential single-goal chaining (plan one step, execute, replan), which is fragile and slow.

## Discovered Via

Golden E2E emergent tests (S07 care interaction coverage). A healer at Village Square with a wounded patient also at Village Square could not plan Travel(Orchard Farm)→PickUp(medicine)→Travel(Village Square)→Heal(patient), despite `max_plan_depth: 6` and all relevant ops being correctly configured. The test was redesigned to give the healer medicine upfront, reducing it to a 2-step plan (Travel→Heal) which succeeded.

## Foundation Alignment

- **Principle 1** (Maximal Emergence Through Local Causality): The planner is the engine of multi-step emergent behavior. If it can only compose 2-step plans, emergence ceilings at chains that happen to align spatially. Real emergence requires agents to autonomously plan "go get X, bring it to Y, use it on Z."
- **Principle 18** (Resource-Bounded Practical Reasoning): The planner must be efficient, but not so bounded that it cannot find plans that a minimally competent human would consider obvious. "Go get medicine from where you know it is, come back, heal the patient" is a plan any agent should be able to form.
- **Principle 7** (Locality): The solution must respect belief-only planning. The planner should use the agent's *believed* resource locations, never query the world directly.
- **Principle 8** (Duration, Cost, Occupancy): Multi-leg plans naturally have higher cost. The solution should not bypass cost accounting — longer plans should be correctly more expensive than shorter ones.

## Phase

Phase 3: Information & Politics (planner enhancement, no phase dependency beyond completed E13/E14)

## Crates

- `worldwake-ai` (search, goal_model, planner_ops, planning_snapshot)

## Dependencies

- E13 (decision architecture) — completed
- E14 (perception & belief) — completed

## Root Cause Analysis

### The A* heuristic is goal-terminal-only

`goal_relevant_places()` returns the places where the goal can be **completed** — the patient's location for TreatWounds, the corpse location for LootCorpse, the destination for MoveCargo. This is correct for the terminal step but provides no guidance for prerequisite steps.

When the agent is already at the goal-terminal location (h=0) but lacks a prerequisite resource, every travel candidate gets `h > 0`, making them appear worse than staying put and trying non-travel actions. The non-travel actions fail (no medicine → can't heal), the agent exhausts local candidates, and the search stalls.

### Spatial pruning is too aggressive

`prune_travel_away_from_goal()` removes travel candidates where the destination is farther from all goal-relevant places than the current location. When goal-relevant places = {patient's location} and the agent is already there, pruning is skipped (current_min == 0 bypass). But this only means travel candidates aren't pruned — they still get poor heuristic scores.

More critically, if the agent is NOT at the patient's location but the medicine is in a third direction, travel toward the medicine IS pruned because it moves away from the patient.

### The fundamental gap: no prerequisite spatial awareness

The planner knows WHERE the goal ends but not WHERE the prerequisites are. It needs to understand that achieving TreatWounds requires medicine, and that medicine is believed to exist at specific locations. This spatial-prerequisite awareness is what enables multi-step resource-procurement plans.

## Design

### Approach: Prerequisite-Aware Goal Relevant Places

Extend `goal_relevant_places()` to return not just the goal-terminal locations but also the locations of prerequisite resources the agent lacks. This feeds naturally into the existing A* heuristic and spatial pruning without requiring new search infrastructure.

This is the minimal architectural change that unlocks multi-step prerequisite plans. It works within the existing search framework — no HTN, no hierarchical decomposition, no new plan representation.

### Deliverable 1: `GoalPrerequisiteLocations` trait extension

Add a new method to `GoalKindPlannerExt`:

```rust
/// Places where prerequisites for this goal can be acquired.
/// Used alongside `goal_relevant_places()` to guide the A* heuristic
/// toward intermediate resource locations when the agent lacks
/// prerequisites.
///
/// Returns empty if the agent already has all prerequisites or if
/// prerequisite locations are unknown.
fn prerequisite_places(
    &self,
    state: &PlanningState<'_>,
    recipes: &RecipeRegistry,
) -> Vec<EntityId>;
```

**Implementation per goal kind:**

- **`TreatWounds`**: If agent lacks Medicine, return places where agent believes Medicine exists (ground lots, seller locations). Otherwise empty.
- **`AcquireCommodity`**: Already has rich `goal_relevant_places` that includes resource sources and sellers. May benefit from including known ground-lot locations. Otherwise unchanged.
- **`ProduceCommodity`**: If agent lacks recipe inputs, return places where inputs are believed available (ground lots, resource sources). Otherwise return workstation locations (already covered by `goal_relevant_places`).
- **All other goals**: Return empty (no prerequisites, or prerequisites are non-spatial).

This method only queries the agent's beliefs via `PlanningState` — it never reads world state directly (Principle 7, 12).

### Deliverable 2: Unified spatial guidance in `search_plan()`

Merge `goal_relevant_places()` and `prerequisite_places()` into a single `combined_goal_places` set passed to `compute_heuristic()` and `prune_travel_away_from_goal()`:

```rust
let mut goal_places = goal.key.kind.goal_relevant_places(&root_state, recipes);
let prerequisite_places = goal.key.kind.prerequisite_places(&root_state, recipes);
goal_places.extend(prerequisite_places);
goal_places.sort_unstable();
goal_places.dedup();
```

**Effect**: The A* heuristic now considers both the goal-terminal location AND prerequisite resource locations when computing h-cost. Travel toward medicine (a prerequisite location) gets lower h than travel to a random unrelated place. Spatial pruning retains travel toward prerequisite locations instead of eliminating it.

**Key property**: As the agent moves through the plan and hypothetically picks up medicine (via `PlanningState` override), the prerequisite is satisfied. On subsequent expansions, `prerequisite_places()` returns empty (agent now has medicine), and the heuristic reverts to guiding toward the goal-terminal location (patient). The guidance naturally shifts from "go get medicine" to "go to patient" as the plan progresses.

### Deliverable 3: Per-node heuristic recomputation

Currently `compute_heuristic()` is called once per successor node using the initial `goal_relevant_places`. For prerequisite-aware search, the heuristic should be recomputed per node using the node's hypothetical state:

```rust
fn compute_node_heuristic(
    snapshot: &PlanningSnapshot,
    goal: &GroundedGoal,
    state: &PlanningState<'_>,
    recipes: &RecipeRegistry,
) -> u32 {
    let mut places = goal.key.kind.goal_relevant_places(state, recipes);
    places.extend(goal.key.kind.prerequisite_places(state, recipes));
    places.sort_unstable();
    places.dedup();
    compute_heuristic_for_places(snapshot, state, &places)
}
```

This ensures that once the agent hypothetically acquires the prerequisite, the heuristic stops guiding toward the prerequisite location and starts guiding toward the goal-terminal location.

**Cost**: One extra `prerequisite_places()` call per node expansion. This is a lightweight belief-state query (iterate known entities, check commodity kinds). Given `max_node_expansions: 512`, the overhead is negligible.

### Deliverable 4: Spatial pruning respects prerequisites

`prune_travel_away_from_goal()` currently uses the initial `goal_relevant_places` list. Update it to accept the combined (goal + prerequisite) places so that travel toward prerequisite locations is not pruned:

```rust
// In the search loop, before pruning:
let combined_places = compute_combined_goal_places(goal, &node.state, recipes);
prune_travel_away_from_goal(
    &mut candidates,
    current_place,
    &combined_places,  // was: goal_relevant_places
    snapshot,
    semantics_table,
);
```

### Deliverable 5: Tests

**Unit tests** (worldwake-ai):
- `prerequisite_places_returns_medicine_locations_for_treat_wounds`: Agent at VS, patient at VS, Medicine on ground at OF. `prerequisite_places()` returns `[OF]`.
- `prerequisite_places_empty_when_agent_has_prerequisite`: Agent at VS with Medicine. `prerequisite_places()` returns `[]`.
- `combined_heuristic_guides_toward_prerequisite`: Agent at VS, patient at VS, medicine at OF. Heuristic for travel-to-OF should be lower than h for staying (because OF is in combined places).
- `spatial_pruning_retains_travel_to_prerequisite`: Agent at Place A, patient at Place A, medicine at Place B. Travel to Place B is NOT pruned.

**Golden test** (worldwake-ai):
- **Multi-hop medicine procurement**: Healer at VS, patient at VS (wounded), Medicine on ground at OF. Healer autonomously plans Travel(OF)→PickUp→Travel(VS)→Heal. This is the test that originally failed and was redesigned — it should now pass with the planner enhancement.

**Regression**: All existing golden tests must pass. The combined heuristic is a superset of the previous heuristic — it adds information, never removes it.

## Architectural Considerations

### Why not HTN (Hierarchical Task Network)?

HTN decomposes goals into predefined sub-task templates (e.g., "TreatWounds decomposes into AcquireMedicine then ApplyTreatment"). This works but:

1. **Violates Principle 1**: Decomposition templates are pre-authored plans, not emergent behavior. The agent doesn't discover that it needs medicine — a human told the planner to look for it.
2. **Rigid**: Adding a new way to acquire medicine (trade, craft, steal) requires updating every HTN template that references medicine acquisition.
3. **Heavy**: Requires a new plan representation layer, sub-goal stack management, and template authoring for every goal kind.

The prerequisite-aware heuristic achieves the same practical result — the planner finds multi-step plans — without pre-authored decompositions. The plan structure emerges from the existing GOAP search guided by better spatial awareness.

### Why not just increase the budget?

Increasing `max_node_expansions` from 512 to, say, 4096 would help the planner find deeper plans by brute force. But:

1. **Doesn't solve the heuristic misdirection**: The A* heuristic still guides away from prerequisites. Higher budget means the planner explores more dead ends before stumbling onto the right path.
2. **Per-tick cost**: Planning runs every tick for every AI agent. 8x budget = 8x worst-case planning time per agent per tick. This violates Principle 11 (performance may compress computation, never causality — but we shouldn't waste computation either).
3. **Doesn't scale**: As the world grows (more places, more action types, more entities), the branching factor grows, and any fixed budget eventually becomes insufficient. Better heuristics scale; bigger budgets don't.

The prerequisite-aware heuristic makes the search more informed, not more expensive. It finds 4-step plans within the existing budget because it's looking in the right direction.

### Interaction with materialization barriers

Trade, Harvest, and Craft are materialization barriers — the planner terminates at these steps because it can't model their outputs hypothetically. This means "plan Travel→Trade(buy medicine)→Travel→Heal" still terminates at the Trade step (ProgressBarrier). The agent executes Travel→Trade, then replans with medicine in hand and finds Travel→Heal.

This is acceptable and consistent with the existing architecture. The prerequisite-aware heuristic helps the agent find the Travel→Trade plan (it can now reach a trade location), and sequential replanning handles the rest. Full plan-through-barriers would require hypothetical materialization modeling, which is a separate, larger enhancement (not proposed here).

### Interaction with `snapshot_travel_horizon`

`PlanningBudget::snapshot_travel_horizon` (default 6) limits how many hops the planner considers for travel. Prerequisite locations beyond this horizon won't be reachable in the plan. This is acceptable — agents should plan within their travel horizon and replan as they move.

## Information-Path Analysis (FND-01 Section H)

- **Information path**: Agent beliefs about entity locations → `PlanningSnapshot` → `prerequisite_places()` → A* heuristic. All information comes from the agent's existing belief store via `PlanningState`. No new information channel is introduced.
- **Positive feedback**: None. Better plans don't make the world state feed back into itself — they just make the agent more effective.
- **Stored vs derived**: `prerequisite_places()` is fully derived — computed from existing belief state each planning cycle. No new stored state.

## Risks

- **Prerequisite place explosion**: If the agent believes Medicine exists at many locations, `prerequisite_places()` could return a large set, diluting the heuristic. Mitigation: limit to the N closest prerequisite locations (e.g., 3), matching `snapshot_travel_horizon` spirit.
- **False prerequisites**: Agent may believe medicine exists somewhere it doesn't (stale belief). The plan will fail at execution, and the agent will replan. This is correct behavior under Principle 14 (ignorance and uncertainty are first-class).
- **Increased planning time**: Per-node `prerequisite_places()` adds a belief-state scan per expansion. With max 512 expansions and a lightweight query, this is negligible.
