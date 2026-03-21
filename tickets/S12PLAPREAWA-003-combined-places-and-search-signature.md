# S12PLAPREAWA-003: Add `combined_relevant_places()` and update `search_plan()` signature

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — `search_plan()` signature change, new internal function
**Deps**: S12PLAPREAWA-001, S12PLAPREAWA-002

## Problem

`search_plan()` takes a static `goal_relevant_places: &[EntityId]` slice computed once before search begins. This means the heuristic cannot adapt as the agent hypothetically acquires prerequisites during planning. After a hypothetical PickUp(medicine), the heuristic should stop guiding toward the medicine location and shift to the patient — but with static places, it keeps guiding toward already-acquired resources.

This ticket replaces the static slice with per-node dynamic computation via a new `combined_relevant_places()` function.

## Assumption Reassessment (2026-03-21)

1. `search_plan()` signature at `crates/worldwake-ai/src/search.rs` takes `goal_relevant_places: &[EntityId]` as its 8th parameter — confirmed.
2. `goal_relevant_places` is used in three internal call sites: `root_node()`, `prune_travel_away_from_goal()` (main loop), and `build_successor()` — confirmed.
3. `compute_heuristic()` and `prune_travel_away_from_goal()` both take `&[EntityId]` for places — their signatures remain UNCHANGED; only callers pass different data.
4. `root_node()` takes `goal_relevant_places: &[EntityId]` — confirmed.
5. `build_successor()` takes `goal_relevant_places: &[EntityId]` — confirmed.
6. `RecipeRegistry` is already available in `agent_tick.rs` scope and can be threaded to `search_plan()` — confirmed.
7. `PlanningBudget` is already a parameter of `search_plan()` — confirmed.
8. `GroundedGoal` contains `key.kind: GoalKind` which implements `GoalKindPlannerExt` — confirmed.
9. No other crate calls `search_plan()` besides `agent_tick.rs` — confirmed via the single call site.
10. This ticket changes a public function signature — all callers must be updated (only `agent_tick.rs`, handled by S12PLAPREAWA-004, but must compile together).

## Architecture Check

1. Per-node dynamic computation is the correct approach: static combined lists cannot adapt to hypothetical state changes. The function is private to `search.rs`, keeping the logic contained.
2. No backwards-compatibility shims. The old `goal_relevant_places` parameter is removed and replaced.

## Verification Layers

1. `combined_relevant_places` includes prerequisites when lacking → focused unit test
2. `combined_relevant_places` excludes prerequisites after hypothetical pickup → focused unit test
3. Pruning retains travel to prerequisite location → focused unit test
4. Heuristic guides toward prerequisite when lacking → focused unit test
5. All existing search behavior unchanged for goals with empty `prerequisite_places()` → existing golden test suite

## What to Change

### 1. Add `combined_relevant_places()` private function

In `crates/worldwake-ai/src/search.rs`, add:

```rust
/// Computes the union of goal-terminal places and prerequisite places
/// from the current hypothetical planning state. As the agent hypothetically
/// acquires prerequisites, `prerequisite_places()` returns fewer locations,
/// naturally shifting heuristic guidance from "go get resources" to
/// "go to goal-terminal location."
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

### 2. Update `search_plan()` signature

Replace parameter `goal_relevant_places: &[EntityId]` with `recipes: &RecipeRegistry`.

### 3. Update `root_node()` call

Compute `combined_relevant_places(goal, &root_state, recipes, budget)` and pass to `root_node()` (which still takes `&[EntityId]`).

### 4. Update main loop pruning call

Before `prune_travel_away_from_goal()`, compute `combined_relevant_places(goal, &node.state, recipes, budget)` and pass the result.

### 5. Update `build_successor()` signature and internals

Replace `goal_relevant_places: &[EntityId]` parameter with `(goal: &GroundedGoal, recipes: &RecipeRegistry, budget: &PlanningBudget)`. Inside, after `apply_hypothetical_transition()` produces `transition.state`, compute `combined_relevant_places(goal, &transition.state, recipes, budget)` and pass to `compute_heuristic()`.

### 6. Leave `compute_heuristic()` and `prune_travel_away_from_goal()` signatures UNCHANGED

They continue to take `&[EntityId]`. The callers pass dynamically computed slices.

## Files to Touch

- `crates/worldwake-ai/src/search.rs` (modify)

## Out of Scope

- `compute_heuristic()` signature — unchanged, receives dynamically computed slices
- `prune_travel_away_from_goal()` signature — unchanged
- `agent_tick.rs` call site update (S12PLAPREAWA-004) — but note this ticket and 004 must be applied together to compile
- `goal_model.rs` changes (S12PLAPREAWA-002)
- `budget.rs` changes (S12PLAPREAWA-001)
- Decision trace struct changes (S12PLAPREAWA-005)
- Golden tests (S12PLAPREAWA-007)
- Any changes to `PlanningState`, `PlanningSnapshot`, or action transition logic

## Acceptance Criteria

### Tests That Must Pass

1. `combined_places_includes_prerequisites_when_lacking` — combined set for TreatWounds without medicine includes both patient location and medicine source
2. `combined_places_excludes_prerequisites_after_hypothetical_pickup` — after `apply_pick_up_transition()`, combined set no longer includes medicine source
3. `pruning_retains_travel_to_prerequisite_location` — agent at Place A (= patient location), medicine at Place B; travel to Place B is NOT pruned
4. `heuristic_guides_toward_prerequisite_when_lacking` — agent at Place A, patient at Place A, medicine at Place B; heuristic for travel-to-B is lower than staying
5. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. Goals returning empty from `prerequisite_places()` produce identical combined place sets to pre-S12 behavior — strictly additive change
2. `compute_heuristic()` and `prune_travel_away_from_goal()` signatures unchanged — downstream consumers unaffected
3. Per-node computation cost is bounded by `max_node_expansions` (512) × lightweight belief scan — negligible vs. affordance generation

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/search.rs` (test module) — 4 new unit tests as listed in acceptance criteria

### Commands

1. `cargo test -p worldwake-ai search`
2. `cargo test -p worldwake-ai && cargo clippy --workspace`

## Implementation Note

This ticket and S12PLAPREAWA-004 form an atomic compilation unit — `search_plan()` signature change requires the call site in `agent_tick.rs` to update simultaneously. Implement them together or use a feature flag. The recommended approach is to implement 003 and 004 in the same commit.
