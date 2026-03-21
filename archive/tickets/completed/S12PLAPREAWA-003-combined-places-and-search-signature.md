# S12PLAPREAWA-003: Make search own dynamic relevant-place computation

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — `search_plan()` signature change, `GoalKindPlannerExt::prerequisite_places()` contract change, focused test updates
**Deps**: S12PLAPREAWA-001, specs/S12-planner-prerequisite-aware-search.md

## Problem

The current code partially implements prerequisite-aware planning, but the critical architectural boundary is still wrong:

1. `search.rs` already has a private `combined_relevant_places(...)`, but it still starts from a static `goal_relevant_places: &[EntityId]` slice that `agent_tick.rs` computes once at search start.
2. `GoalKindPlannerExt::prerequisite_places(...)` exists, but its current signature does not accept `RecipeRegistry`, so `ProduceCommodity` cannot derive missing-input places from recipe inputs.
3. A focused test in `goal_model.rs` currently codifies the old architecture by asserting that `ProduceCommodity` prerequisite lookup stays empty when inputs are missing.

That leaves the planner in a split-brain design: search owns per-node hypothetical state, but the caller still owns terminal-place computation, and production goals still cannot expose lawful procurement locations from the goal model itself.

## Assumption Reassessment (2026-03-21)

1. `PlanningBudget::max_prerequisite_locations` already exists in [crates/worldwake-ai/src/budget.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/budget.rs) and has a default test. The original ticket’s budget work is already delivered.
2. `GoalKindPlannerExt::prerequisite_places(...)` already exists in [crates/worldwake-ai/src/goal_model.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/goal_model.rs), but it currently takes only `(&PlanningState, &PlanningBudget)` and currently implements prerequisite lookup only for `TreatWounds`.
3. `search.rs` already contains `combined_relevant_places(...)`, `root_node(...)`, and `build_successor(...)`, and the main loop already uses combined places for pruning and heuristic recomputation. The original “add the helper” scope is stale; the remaining bug is that the helper still depends on caller-supplied static terminal places.
4. `search_plan(...)` still takes `goal_relevant_places: &[EntityId]` in [crates/worldwake-ai/src/search.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/search.rs), and `agent_tick.rs` still computes that slice up front in [crates/worldwake-ai/src/agent_tick.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/agent_tick.rs).
5. `search_plan(...)` has more than one current call site. Besides the runtime call in `agent_tick.rs`, it is called by unit tests in [crates/worldwake-ai/src/search.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/search.rs), [crates/worldwake-ai/src/goal_model.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/goal_model.rs), [crates/worldwake-ai/tests/golden_care.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_care.rs), and other focused tests. The original “single call site” assumption was false for current test code.
6. Existing focused coverage already proves part of the feature:
   - `search::tests::combined_places_include_remote_medicine_lot_for_treat_wounds`
   - `search::tests::prune_travel_retains_remote_medicine_branch_for_treat_wounds`
   - `goal_model::tests::prerequisite_places_treat_wounds_*`
7. Existing golden coverage already exercises the intended runtime branch:
   - `golden_healer_acquires_remote_ground_medicine_for_patient`
   - `remote_treat_wounds_search_needs_eight_step_depth_budget_in_prototype_topology`
   - `golden_multi_recipe_craft_path`
   - `golden_materialization_barrier_chain`
8. The current `ProduceCommodity` prerequisite behavior is intentionally incomplete, not a settled architecture. The test `goal_model::tests::prerequisite_places_produce_commodity_stays_empty_when_inputs_are_missing` conflicts with the S12 spec and with the cleaner architecture where goal semantics expose lawful prerequisite acquisition points from beliefs.

## Architecture Check

1. `search_plan(...)` should own all relevant-place computation. The caller does not have node-local hypothetical state, so passing a precomputed slice across the API is the wrong boundary.
2. `GoalKindPlannerExt::prerequisite_places(...)` should accept `RecipeRegistry`. Recipe-input procurement is part of goal semantics for `ProduceCommodity`; leaving it outside the goal model forces ad hoc exceptions and weakens extensibility.
3. The cleaner architecture is:
   - `search_plan(...)` takes `recipes: &RecipeRegistry`
   - `combined_relevant_places(...)` computes both terminal and prerequisite places from the current hypothetical node state
   - `goal_model.rs` owns which goals expose prerequisite places and how they are capped
4. No backward-compatibility alias path. Remove the old slice parameter entirely rather than supporting both forms.

## Verification Layers

1. `TreatWounds` combined places include terminal and prerequisite locations -> focused `search.rs` unit test
2. Combined places drop prerequisite-only locations after hypothetical pickup -> focused `search.rs` unit test using hypothetical transition state
3. `ProduceCommodity` exposes missing-input locations from recipe beliefs -> focused `goal_model.rs` unit tests
4. Search still retains lawful travel toward prerequisite places -> focused `search.rs` pruning/search tests
5. Runtime care scenario still forms the remote medicine route -> existing golden care coverage
6. Runtime production path still works with materialization barriers and recipe input procurement -> existing golden production coverage

## Corrected Scope

### 1. Change `GoalKindPlannerExt::prerequisite_places(...)`

In [crates/worldwake-ai/src/goal_model.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/goal_model.rs):

- Change the trait and impl signature to:

```rust
fn prerequisite_places(
    &self,
    state: &PlanningState<'_>,
    recipes: &RecipeRegistry,
    budget: &PlanningBudget,
) -> Vec<EntityId>;
```

- Keep current `TreatWounds` support.
- Add `ProduceCommodity { recipe_id }` support:
  - inspect recipe inputs via `RecipeRegistry`
  - for each missing input, collect believed loose lots first, then sellers and resource sources
  - union and cap by `budget.max_prerequisite_locations`

### 2. Remove caller-owned `goal_relevant_places` from `search_plan(...)`

In [crates/worldwake-ai/src/search.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/search.rs):

- Replace `goal_relevant_places: &[EntityId]` with `recipes: &RecipeRegistry`
- Make `combined_relevant_places(...)` compute:
  - `goal.key.kind.goal_relevant_places(state, recipes)`
  - `goal.key.kind.prerequisite_places(state, recipes, budget)`
- Update `root_node(...)`, the main-loop pruning path, and `build_successor(...)` to use the new helper contract
- Leave `compute_heuristic(...)` and `prune_travel_away_from_goal(...)` signatures unchanged

### 3. Update runtime and test call sites

- Remove the precomputed `goal_relevant_places` variable from [crates/worldwake-ai/src/agent_tick.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/agent_tick.rs)
- Update all unit and golden `search_plan(...)` call sites to pass `&RecipeRegistry`

### 4. Correct stale tests

- Replace the stale production test that asserts missing recipe inputs should never appear in `prerequisite_places()`
- Add/adjust focused tests so they prove the desired architecture instead of the old one

## Files to Touch

- [crates/worldwake-ai/src/goal_model.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/goal_model.rs)
- [crates/worldwake-ai/src/search.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/search.rs)
- [crates/worldwake-ai/src/agent_tick.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/agent_tick.rs)
- [crates/worldwake-ai/tests/golden_care.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_care.rs)

## Out of Scope

- `PlanningBudget` changes in [crates/worldwake-ai/src/budget.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/budget.rs) — already delivered
- Decision-trace struct enrichment from S12PLAPREAWA-005
- New golden scenarios beyond the existing care/production coverage already present
- Changes to `PlanningState`, `PlanningSnapshot`, or transport/materialization semantics

## Acceptance Criteria

### Tests That Must Pass

1. `goal_model::tests::prerequisite_places_treat_wounds_include_remote_controllable_medicine_lot`
2. New/updated `goal_model::tests` proving `ProduceCommodity` prerequisite places are derived from missing recipe inputs and disappear once inputs are hypothetically available
3. Existing and updated `search::tests` proving combined places, pruning retention, and post-pickup heuristic shift
4. `remote_treat_wounds_search_needs_eight_step_depth_budget_in_prototype_topology`
5. `golden_healer_acquires_remote_ground_medicine_for_patient`
6. `golden_multi_recipe_craft_path`
7. `cargo test -p worldwake-ai`
8. `cargo clippy --workspace`

### Invariants

1. `search_plan(...)` no longer accepts caller-owned `goal_relevant_places`
2. Search computes relevant places from the current hypothetical node state only
3. `ProduceCommodity` prerequisite lookup is belief-driven and recipe-driven; no world-state shortcutting
4. Goals with no prerequisites still degrade to existing terminal-place guidance

## Tests

### New/Modified Tests

1. `crates/worldwake-ai/src/goal_model.rs`
   Rationale: replace the stale production assertion with recipe-input prerequisite coverage, plus verify prerequisites disappear once the actor already holds required inputs.
2. `crates/worldwake-ai/src/search.rs`
   Rationale: verify combined places are recomputed from node-local hypothetical state, including the post-pickup shift away from prerequisite locations.
3. `crates/worldwake-ai/tests/golden_care.rs`
   Rationale: update `search_plan(...)` call sites and keep the remote-care regression proving the runtime branch still works end to end.

### Commands

1. `cargo test -p worldwake-ai goal_model::tests::prerequisite_places`
2. `cargo test -p worldwake-ai search::tests::combined_places`
3. `cargo test -p worldwake-ai remote_treat_wounds_search_needs_eight_step_depth_budget_in_prototype_topology`
4. `cargo test -p worldwake-ai golden_healer_acquires_remote_ground_medicine_for_patient`
5. `cargo test -p worldwake-ai golden_multi_recipe_craft_path`
6. `cargo test -p worldwake-ai`
7. `cargo clippy --workspace`

## Outcome

- Completion date: 2026-03-21
- What actually changed:
  - `search_plan(...)` now takes `&RecipeRegistry` and owns dynamic relevant-place computation from node-local hypothetical state.
  - `GoalKindPlannerExt::prerequisite_places(...)` now takes `&RecipeRegistry` and `ProduceCommodity` now exposes missing-input procurement places from beliefs.
  - `agent_tick.rs` and the focused/golden `search_plan(...)` call sites were updated to the new search boundary.
  - The stale production test that asserted missing recipe inputs should never appear in prerequisite lookup was replaced with production prerequisite coverage, including the partial-input edge case.
  - `search.rs` gained a focused regression proving combined places drop the prerequisite location after hypothetical pickup.
- Deviations from original plan:
  - The helper and budget pieces described by the original ticket were already present in live code, so this completion focused on the remaining architectural gap instead of re-adding them.
  - The work absorbed the runtime call-site update that had been split into S12PLAPREAWA-004, because the signature change is one atomic change in practice.
  - Existing golden care/production coverage was sufficient; no new golden scenarios were required for this ticket.
- Verification results:
  - `cargo test -p worldwake-ai prerequisite_places_produce_commodity`
  - `cargo test -p worldwake-ai combined_places`
  - `cargo test -p worldwake-ai remote_treat_wounds_search_needs_eight_step_depth_budget_in_prototype_topology`
  - `cargo test -p worldwake-ai golden_multi_recipe_craft_path`
  - `cargo test --workspace`
  - `cargo clippy --workspace --all-targets -- -D warnings`
