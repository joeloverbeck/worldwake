**Status**: PENDING

# Planner Prerequisite-Aware Search

## Summary

The GOAP planner cannot compose multi-step plans that require visiting intermediate locations to acquire prerequisites before reaching the goal-terminal location. For example, a `TreatWounds { patient }` goal where the patient is local but medicine is at a remote location requires the plan Travel(remote)→PickUp(medicine)→Travel(patient)→Heal — but the planner never finds this plan because:

1. **Single-terminal heuristic**: `goal_relevant_places()` returns only the patient's location. The A* heuristic has no concept of intermediate resource locations.
2. **Spatial pruning eliminates prerequisite travel**: `prune_travel_away_from_goal()` removes travel candidates that move the actor away from goal-relevant places. Traveling to get medicine is pruned because it increases distance from the patient.
3. **Budget exhaustion**: Even if pruning didn't eliminate the path, the combinatorial search space means 4-step cross-domain plans exhaust `max_node_expansions: 512` before being found.

This limits Principle 1 (Maximal Emergence): complex causal chains that require resource procurement before goal achievement cannot emerge from the planner. Agents fall back to sequential single-goal chaining (plan one step, execute, replan), which is fragile and slow.

The same problem affects `ProduceCommodity` when recipe inputs are at remote locations — the planner guides toward the workstation but not toward the input sources.

## Discovered Via

Golden E2E emergent tests (S07 care interaction coverage). A healer at Village Square with a wounded patient also at Village Square could not plan Travel(Orchard Farm)→PickUp(medicine)→Travel(Village Square)→Heal(patient), despite `max_plan_depth: 6` and all relevant ops being correctly configured. The test was redesigned to give the healer medicine upfront, reducing it to a 2-step plan (Travel→Heal) which succeeded.

## Foundation Alignment

- **Principle 1** (Maximal Emergence Through Local Causality): The planner is the engine of multi-step emergent behavior. If it can only compose 2-step plans, emergence ceilings at chains that happen to align spatially. Real emergence requires agents to autonomously plan "go get X, bring it to Y, use it on Z."
- **Principle 18** (Resource-Bounded Practical Reasoning): The planner must be efficient, but not so bounded that it cannot find plans that a minimally competent human would consider obvious. "Go get medicine from where you know it is, come back, heal the patient" is a plan any agent should be able to form.
- **Principle 7** (Locality): The solution must respect belief-only planning. The planner should use the agent's *believed* resource locations, never query the world directly.
- **Principle 8** (Duration, Cost, Occupancy): Multi-leg plans naturally have higher cost. The solution should not bypass cost accounting — longer plans should be correctly more expensive than shorter ones.
- **Principle 27** (Debuggability): The decision trace must reveal whether spatial guidance came from terminal or prerequisite locations, enabling "why did the agent travel there?" diagnosis.

## Phase

Phase 3: Information & Politics (planner enhancement, no phase dependency beyond completed E13/E14)

## Crates

- `worldwake-ai` (search, goal_model, agent_tick, budget, decision_trace)

## Dependencies

- E13 (decision architecture) — completed
- E14 (perception & belief) — completed

## Root Cause Analysis

### The A* heuristic uses static goal-relevant places

`goal_relevant_places()` is computed **once** from a fresh root `PlanningState` in `agent_tick.rs:1034-1038` and passed as a static `&[EntityId]` slice to `search_plan()`. This slice is threaded unchanged through every `compute_heuristic()` and `prune_travel_away_from_goal()` call. Even though the heuristic IS recomputed per-node in `build_successor()` (using the node's hypothetical `PlanningState`), it always evaluates distance against the **same static set of places**.

For TreatWounds, `goal_relevant_places()` returns only the patient's location. When the agent is already there (h=0) but lacks medicine, every travel candidate gets h > 0, making them appear worse than staying put. The non-travel actions fail (no medicine → can't heal), and the search stalls or exhausts budget exploring dead ends.

### Spatial pruning is too aggressive

`prune_travel_away_from_goal()` removes travel candidates where the destination is farther from all goal-relevant places than the current location. When goal-relevant places = {patient's location} and the agent is already there, pruning is skipped (current_min == 0 bypass). But travel candidates still get poor heuristic scores.

More critically, if the agent is NOT at the patient's location but the medicine is in a third direction, travel toward the medicine IS pruned because it moves away from the patient.

### The fundamental gap: no prerequisite spatial awareness

The planner knows WHERE the goal ends but not WHERE the prerequisites are. It needs to understand that achieving TreatWounds requires medicine, and that medicine is believed to exist at specific locations. This spatial-prerequisite awareness is what enables multi-step resource-procurement plans.

## Design

### Approach: Dynamic Per-Node Combined Places

Replace the static `goal_relevant_places: &[EntityId]` parameter in `search_plan()` with per-node computation. Each node computes combined places = `goal_relevant_places(state) ∪ prerequisite_places(state)` from the node's `PlanningState`. This feeds naturally into the existing A* heuristic and spatial pruning without requiring new search infrastructure.

**Why dynamic per-node, not static**: A static combined list (computed once at search start) does not adapt as hypothetical state changes. After the planner hypothetically picks up medicine (via `apply_pick_up_transition()` updating `commodity_quantity_overrides` in `PlanningState`), a static list still guides toward the now-unnecessary medicine location. Dynamic per-node computation means `prerequisite_places()` returns empty once the prerequisite is hypothetically satisfied — the heuristic naturally shifts from "go get medicine" to "go to patient."

**Why not HTN**: Decomposition templates are pre-authored plans, not emergent behavior (violates Principle 1). Adding new acquisition methods requires template updates. The prerequisite-aware heuristic achieves the same result through better spatial guidance within existing GOAP search.

**Why not just increase budget**: Doesn't solve heuristic misdirection. Higher budget means more dead-end exploration. Better heuristics scale; bigger budgets don't.

### Deliverable 1: `prerequisite_places()` method on `GoalKindPlannerExt`

**File**: `crates/worldwake-ai/src/goal_model.rs`

Add a new method to `GoalKindPlannerExt`:

```rust
/// Places where prerequisites for this goal can be acquired,
/// given the agent's current hypothetical state.
///
/// Returns empty when the agent already possesses all prerequisites
/// in the given `PlanningState`, or when prerequisite locations are
/// unknown to the agent.
///
/// Combined with `goal_relevant_places()` to form the full set of
/// spatially relevant locations for A* guidance. The set is capped
/// to the N closest locations (by travel distance) via
/// `PlanningBudget::max_prerequisite_locations`.
fn prerequisite_places(
    &self,
    state: &PlanningState<'_>,
    recipes: &RecipeRegistry,
    budget: &PlanningBudget,
) -> Vec<EntityId>;
```

**Implementation per goal kind:**

- **`TreatWounds { patient }`**: The Heal action requires Medicine (enforced by `start_heal()` validation). If `state.commodity_quantity(actor, CommodityKind::Medicine) == Quantity(0)`, return resource sources and seller places for Medicine (reusing `places_with_resource_source()` and `places_with_sellers()`), capped to `budget.max_prerequisite_locations` closest by travel distance. Otherwise empty.
- **`ProduceCommodity { recipe_id }`**: Look up the recipe's inputs from `RecipeRegistry`. For each `(commodity, quantity)` where `state.commodity_quantity(actor, commodity) < quantity`, collect resource sources and seller places for that commodity. Return the union of all such places, capped to N closest. Otherwise empty.
- **`ConsumeOwnedCommodity`**: Already has prerequisite awareness built into its `goal_relevant_places()` — when the agent lacks the commodity, it returns resource source places. No additional prerequisite places needed. Return empty.
- **`AcquireCommodity`**: Same reasoning — `goal_relevant_places()` already covers resource sources and sellers. Return empty.
- **`RestockCommodity`**: Same — already handled by `goal_relevant_places()`. Return empty.
- **All other goals** (`Sleep`, `Wash`, `Relieve`, `ReduceDanger`, `EngageHostile`, `SellCommodity`, `MoveCargo`, `LootCorpse`, `BuryCorpse`, `ShareBelief`, `ClaimOffice`, `SupportCandidateForOffice`): Return empty. No spatial prerequisites.

This method only queries the agent's beliefs via `PlanningState` — it never reads world state directly (Principle 7, 12).

### Deliverable 2: `combined_relevant_places()` function

**File**: `crates/worldwake-ai/src/search.rs` (private)

```rust
fn combined_relevant_places(
    goal: &GroundedGoal,
    state: &PlanningState<'_>,
    recipes: &RecipeRegistry,
    budget: &PlanningBudget,
) -> Vec<EntityId> {
    let mut places = goal.key.kind.goal_relevant_places(state, recipes);
    let prereqs = goal.key.kind.prerequisite_places(state, recipes, budget);
    for p in prereqs {
        if !places.contains(&p) {
            places.push(p);
        }
    }
    places
}
```

This replaces the static `goal_relevant_places` parameter throughout `search_plan()`.

**Key property**: As the agent moves through the plan and hypothetically acquires prerequisites (via `PlanningState` overrides), `prerequisite_places()` returns fewer locations. When all prerequisites are satisfied, the combined set degrades to `goal_relevant_places()` alone — identical to pre-S12 behavior. The guidance naturally shifts from "go get resources" to "go to goal-terminal location."

### Deliverable 3: `search_plan()` signature and internal changes

**File**: `crates/worldwake-ai/src/search.rs`

**Signature change**: Replace `goal_relevant_places: &[EntityId]` with `recipes: &RecipeRegistry`. The `budget` parameter is already present.

**Internal changes**:

- **`root_node()`**: Compute `combined_relevant_places(goal, &root_state, recipes, budget)` for the initial heuristic.
- **Main loop, before pruning**: Compute `combined_relevant_places(goal, &node.state, recipes, budget)` and pass to `prune_travel_away_from_goal()`. This replaces the static `goal_relevant_places` reference.
- **`build_successor()`**: Replace `goal_relevant_places: &[EntityId]` parameter with `(goal, recipes, budget)`. After `apply_hypothetical_transition()` produces `transition.state`, compute `combined_relevant_places(goal, &transition.state, recipes, budget)` and pass to `compute_heuristic()`.
- **`compute_heuristic()` and `prune_travel_away_from_goal()`**: Signatures UNCHANGED — they still take `(snapshot, state, &[EntityId])`. The callers pass the dynamically computed slices.

**Cost**: One extra `combined_relevant_places()` call per node expansion. This involves `goal_relevant_places()` + `prerequisite_places()`, both iterating snapshot entities (typically 10-50) and checking commodity kinds. With `max_node_expansions: 512`, the overhead is negligible compared to affordance generation which already runs per-node.

### Deliverable 4: Call site update

**File**: `crates/worldwake-ai/src/agent_tick.rs` (lines 1034-1048)

Remove the pre-search `goal_relevant_places` computation. Pass `recipe_registry` (already in scope) to `search_plan()` instead.

### Deliverable 5: `PlanningBudget` extension

**File**: `crates/worldwake-ai/src/budget.rs`

New field:

```rust
pub max_prerequisite_locations: u8,  // default: 3
```

Used by `prerequisite_places()` to cap the returned set to the N closest prerequisite locations by travel distance (via `PlanningSnapshot::min_travel_ticks`). This prevents heuristic dilution when the agent believes a commodity exists at many distant locations.

### Deliverable 6: Decision trace enrichment

**File**: `crates/worldwake-ai/src/decision_trace.rs`

Extend `SearchExpansionSummary` with:

```rust
pub combined_places_count: u16,
pub prerequisite_places_count: u16,
```

When debugging "why did the agent travel toward X?", the trace reveals whether X was a prerequisite location or a goal-terminal location. The existing `TravelPruningTrace` (retained/pruned destinations) implicitly reflects prerequisite-aware pruning without structural change.

### Deliverable 7: Tests

**Unit tests** (goal_model.rs):

1. `prerequisite_places_treat_wounds_without_medicine`: Agent lacks Medicine, Medicine resource source at Place B. Returns `[Place_B]`.
2. `prerequisite_places_treat_wounds_with_medicine`: Agent has Medicine. Returns `[]`.
3. `prerequisite_places_treat_wounds_seller`: Agent lacks Medicine, merchant selling Medicine at Place C. Returns `[Place_C]`.
4. `prerequisite_places_produce_commodity_missing_input`: Recipe requires Wheat, agent lacks Wheat, Wheat source at Place D. Returns `[Place_D]`.
5. `prerequisite_places_produce_commodity_has_all_inputs`: Agent has all recipe inputs. Returns `[]`.
6. `prerequisite_places_capped_by_budget`: Agent knows Medicine at 5 places. With `max_prerequisite_locations: 3`, returns 3 closest.
7. `all_goal_kind_variants_have_prerequisite_places_impl`: Exhaustive match coverage (mirrors existing `all_goal_kind_variants_have_goal_relevant_places_impl`).

**Unit tests** (search.rs):

8. `combined_places_includes_prerequisites_when_lacking`: Combined set for TreatWounds without medicine includes both patient location and medicine source.
9. `combined_places_excludes_prerequisites_after_hypothetical_pickup`: After `apply_pick_up_transition()`, combined set no longer includes medicine source.
10. `pruning_retains_travel_to_prerequisite_location`: Agent at Place A (= patient location), medicine at Place B. Travel to Place B is NOT pruned.
11. `heuristic_guides_toward_prerequisite_when_lacking`: Agent at Place A, patient at Place A, medicine at Place B. Heuristic for travel-to-B node is lower than staying.

**Golden tests** (worldwake-ai):

12. `golden_multi_hop_medicine_procurement`: Healer at VS, patient at VS (wounded), Medicine on ground at OF. Healer autonomously plans Travel(OF)→PickUp(medicine)→Travel(VS)→Heal(patient). This is the test that originally failed during S07 and was redesigned — it should now pass with the planner enhancement. Include deterministic replay companion.
13. `golden_prerequisite_aware_craft`: Agent needs to ProduceCommodity (recipe requiring Wheat), Wheat on ground at Farm, workstation at Workshop. Agent plans Travel(Farm)→PickUp(Wheat)→Travel(Workshop)→Craft. Include deterministic replay companion.

**Regression**: All existing 133+ golden tests must pass. The combined heuristic is a superset of the previous heuristic — it adds information, never removes it. Goals that return empty from `prerequisite_places()` produce identical combined place sets.

## Cross-System Interaction Analysis

### S03: Goal Binding (matches_binding)

Auxiliary ops (Travel, MoveCargo, Trade, Consume, Harvest, Craft, QueueForFacilityUse) **always pass binding** at `goal_model.rs:716-728`. Only terminal ops enforce exact-bound checks. For TreatWounds, intermediate PickUp steps target a medicine lot (not the patient), but since `MoveCargo` is classified as an auxiliary op, binding is not checked for it. The terminal Heal step must target the exact patient, which it does since the goal binds `patient`.

**No S03 change needed.**

### S08: Failure Handling (handle_plan_failure)

`handle_plan_failure()` drops the entire plan, records a `BlockedIntent` with a derived `blocking_fact`, and sets `runtime.dirty = true` to force replanning. There is no distinction between failed prerequisite steps and failed terminal steps — the handling is uniform. If an intermediate PickUp fails (e.g., medicine was already taken by another agent), the plan is discarded and the agent replans from current world state. This is correct: the agent may find medicine elsewhere or adopt a different goal.

**No S08 change needed.**

### Transport: PickUp as Intermediate Step

`TREAT_WOUNDS_OPS` already includes `PlannerOpKind::MoveCargo` (`goal_model.rs:114`). The `pick_up` action maps to `PlannerTransitionKind::PickUpGroundLot` with:
- `may_appear_mid_plan: true` — CAN appear as an intermediate step in a multi-step plan
- `is_materialization_barrier: false` — NOT a barrier, the planner searches beyond it

`apply_pick_up_transition()` correctly updates `PlanningState`:
- Validates lot is `EntityKind::ItemLot`, unowned, uncontained, at same place as actor
- Moves lot from ground to actor possession via `move_lot_ref_to_holder()`
- Updates `direct_possessor_overrides` and `commodity_quantity_overrides`

This state change is what makes the dynamic heuristic shift work: after hypothetical pickup, `prerequisite_places()` returns empty because `commodity_quantity(actor, Medicine) > 0` in the node's `PlanningState`.

**No transport change needed.**

### Materialization Barriers

Trade, Harvest, and Craft have `is_materialization_barrier: true`. The planner terminates at these steps as `ProgressBarrier`. For a plan like Travel→Trade(buy medicine)→Travel→Heal, the planner finds Travel→Trade and terminates. The agent executes Travel→Trade, then replans with medicine in hand and finds Travel→Heal (or PickUp→Travel→Heal if the trade materializes ground lots).

The prerequisite-aware heuristic helps the agent find the Travel→Trade plan (medicine seller location is now in the combined places). Sequential replanning handles the rest. Full plan-through-barriers would require hypothetical materialization modeling, which is a separate, larger enhancement (not proposed here).

### Interaction with `snapshot_travel_horizon`

`PlanningBudget::snapshot_travel_horizon` (default 6) limits how many hops the planner considers for travel. Prerequisite locations beyond this horizon won't be reachable. This is acceptable — agents should plan within their travel horizon and replan as they move.

## Information-Path Analysis (FND-01 Section H)

- **Information path**: Agent beliefs about entity locations → `PlanningSnapshot` → `PlanningState` → `prerequisite_places()` → combined places → A* heuristic and spatial pruning. All information originates from the agent's existing belief store via `PlanningState`. No new information channel is introduced.
- **Positive feedback**: None. Better plans do not cause the world to produce more information or resources — they just make the agent more effective at exploiting existing beliefs.
- **Concrete dampeners**: N/A (no positive feedback loops).
- **Stored vs derived**: `prerequisite_places()` is fully derived — computed per-node from `PlanningState` (itself cloned per search node). `max_prerequisite_locations` in `PlanningBudget` is stored configuration. No new authoritative stored state.

## Risks

- **Prerequisite place explosion**: If the agent believes Medicine exists at many locations, `prerequisite_places()` could return a large set, diluting the heuristic. Mitigation: `max_prerequisite_locations` budget cap (default 3) limits to the N closest by travel distance.
- **False prerequisites**: Agent may believe medicine exists somewhere it doesn't (stale belief). The plan will fail at execution, and the agent will replan. This is correct behavior under Principle 14 (ignorance and uncertainty are first-class).
- **Increased planning time**: Per-node `combined_relevant_places()` adds a belief-state scan per expansion. With max 512 expansions and a lightweight query (~10-50 entity iterations + distance matrix lookups), the overhead is negligible compared to affordance generation which already runs per-node.
- **Regression risk**: Goals that return empty from `prerequisite_places()` produce identical combined place sets to pre-S12 behavior. Only TreatWounds and ProduceCommodity gain new places, strictly additive.
