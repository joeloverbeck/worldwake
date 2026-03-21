# S12PLAPREAWA-002: Add `prerequisite_places()` method to `GoalKindPlannerExt`

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — new trait method on `GoalKindPlannerExt`
**Deps**: S12PLAPREAWA-001 (needs `max_prerequisite_locations` on `PlanningBudget`)

## Problem

The planner knows WHERE a goal ends (via `goal_relevant_places()`) but not WHERE the prerequisites are. For `TreatWounds`, the planner needs to know that Medicine can be acquired at specific locations. For `ProduceCommodity`, the planner needs to know where recipe inputs are. Without this, the A* heuristic cannot guide toward intermediate resource-acquisition steps, preventing multi-step plans like Travel→PickUp→Travel→Heal.

## Assumption Reassessment (2026-03-21)

1. `GoalKindPlannerExt` trait is defined at `crates/worldwake-ai/src/goal_model.rs` with methods including `goal_relevant_places(&self, state: &PlanningState<'_>, recipes: &RecipeRegistry) -> Vec<EntityId>` — confirmed.
2. `GoalKind` enum has variants: `ConsumeOwnedCommodity`, `AcquireCommodity`, `Sleep`, `Wash`, `Relieve`, `ReduceDanger`, `EngageHostile`, `ProduceCommodity`, `RestockCommodity`, `SellCommodity`, `MoveCargo`, `LootCorpse`, `BuryCorpse`, `TreatWounds`, `ShareBelief`, `ClaimOffice`, `SupportCandidateForOffice` — confirmed via spec cross-reference.
3. `PlanningState` provides `commodity_quantity(actor, kind) -> Quantity` for checking whether the agent hypothetically possesses a commodity — confirmed (used by `is_satisfied()` and `apply_*_transition()` methods).
4. `PlanningSnapshot` provides `min_travel_ticks(from, to)` for distance computation — confirmed (used by `compute_heuristic()`).
5. Resource source places and seller places are already queried by `goal_relevant_places()` for `ConsumeOwnedCommodity` and `AcquireCommodity` — the same query patterns will be reused.
6. `RecipeRegistry` provides recipe lookup by `RecipeId` and input requirements — confirmed.
7. `max_prerequisite_locations` field exists on `PlanningBudget` after S12PLAPREAWA-001.
8. Exhaustive match test `all_goal_kind_variants_have_goal_relevant_places_impl` exists — a parallel test for `prerequisite_places` is required.
9. This ticket adds a new trait method with an exhaustive `GoalKind` match — no heuristic removal, no ordering change, no AI regression risk.

## Architecture Check

1. Adding a new method to the existing `GoalKindPlannerExt` trait follows the established pattern (same as `goal_relevant_places`, `matches_binding`, `is_satisfied`). The alternative — computing prerequisites inside `search.rs` — would leak goal-kind-specific logic out of the goal model.
2. No backwards-compatibility shims. All `GoalKind` variants get an implementation in the exhaustive match.

## Verification Layers

1. `TreatWounds` without Medicine returns medicine locations → focused unit test
2. `TreatWounds` with Medicine returns empty → focused unit test
3. `ProduceCommodity` with missing inputs returns input source locations → focused unit test
4. `ProduceCommodity` with all inputs returns empty → focused unit test
5. Budget cap limits returned locations → focused unit test
6. All other goal kinds return empty → exhaustive match coverage test
7. Single-layer ticket (trait method) — verification is unit-test-only.

## What to Change

### 1. Add `prerequisite_places()` to `GoalKindPlannerExt` trait

In `crates/worldwake-ai/src/goal_model.rs`, add to the trait definition:

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

### 2. Implement for `GoalKind`

In the `impl GoalKindPlannerExt for GoalKind` block, add the exhaustive match:

- **`TreatWounds { patient }`**: If `state.commodity_quantity(actor, CommodityKind::Medicine) == Quantity(0)`, return resource source places and seller places for Medicine, capped to `budget.max_prerequisite_locations` closest by travel distance. Otherwise empty.
- **`ProduceCommodity { recipe_id }`**: Look up recipe inputs from `RecipeRegistry`. For each `(commodity, quantity)` where `state.commodity_quantity(actor, commodity) < quantity`, collect resource sources and seller places. Return union capped to N closest. Otherwise empty.
- **`ConsumeOwnedCommodity`**, **`AcquireCommodity`**, **`RestockCommodity`**: Already handled by `goal_relevant_places()`. Return empty.
- **All others** (`Sleep`, `Wash`, `Relieve`, `ReduceDanger`, `EngageHostile`, `SellCommodity`, `MoveCargo`, `LootCorpse`, `BuryCorpse`, `ShareBelief`, `ClaimOffice`, `SupportCandidateForOffice`): Return empty.

The distance-cap logic uses `PlanningSnapshot::min_travel_ticks` to sort candidate places by distance and truncate to `budget.max_prerequisite_locations`. The snapshot is accessible via `state.snapshot()`.

## Files to Touch

- `crates/worldwake-ai/src/goal_model.rs` (modify — trait definition + impl)

## Out of Scope

- `combined_relevant_places()` function (S12PLAPREAWA-003)
- `search_plan()` signature changes (S12PLAPREAWA-003)
- `agent_tick.rs` call site changes (S12PLAPREAWA-004)
- Decision trace changes (S12PLAPREAWA-005)
- Golden tests (S12PLAPREAWA-007)
- Changes to `goal_relevant_places()` — existing method is unchanged
- Changes to any file outside `goal_model.rs`

## Acceptance Criteria

### Tests That Must Pass

1. `prerequisite_places_treat_wounds_without_medicine` — agent lacks Medicine, Medicine resource source at Place B → returns `[Place_B]`
2. `prerequisite_places_treat_wounds_with_medicine` — agent has Medicine → returns `[]`
3. `prerequisite_places_treat_wounds_seller` — agent lacks Medicine, merchant selling Medicine at Place C → returns `[Place_C]`
4. `prerequisite_places_produce_commodity_missing_input` — recipe requires Wheat, agent lacks Wheat, Wheat source at Place D → returns `[Place_D]`
5. `prerequisite_places_produce_commodity_has_all_inputs` — agent has all recipe inputs → returns `[]`
6. `prerequisite_places_capped_by_budget` — agent knows Medicine at 5 places, `max_prerequisite_locations: 3` → returns 3 closest
7. `all_goal_kind_variants_have_prerequisite_places_impl` — exhaustive match coverage
8. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. `prerequisite_places()` only queries agent beliefs via `PlanningState` — never reads world state directly (Principle 7)
2. All `GoalKind` variants are covered in the match (no wildcard/default arm)
3. Goals that already handle resource locations in `goal_relevant_places()` return empty from `prerequisite_places()` — no duplication

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/goal_model.rs` (test module) — 7 new unit tests as listed in acceptance criteria

### Commands

1. `cargo test -p worldwake-ai goal_model`
2. `cargo test -p worldwake-ai && cargo clippy --workspace`
