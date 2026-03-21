# S18PREAWAEME-001: Extend `prerequisite_places()` for `RestockCommodity`

**Status**: ✅ COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — `goal_model.rs` prerequisite spatial guidance
**Deps**: S12 (planner prerequisite-aware search) — completed

## Problem

`RestockCommodity` includes `PlannerOpKind::Craft` in its allowed ops (`RESTOCK_OPS`), so the planner's search CAN chain restock → craft operations. However, `prerequisite_places()` returns `Vec::new()` for `RestockCommodity` (falls through to the `_ =>` arm at line ~760 in `goal_model.rs`). This means the A* heuristic receives no spatial guidance toward remote recipe inputs during restock — the planner cannot find craft-via-remote-inputs plans within budget.

## Assumption Reassessment (2026-03-21)

1. `GoalKind::RestockCommodity` already has goal-side spatial guidance in `crates/worldwake-ai/src/goal_model.rs::goal_relevant_places()` (the dedicated arm at lines 697-703), but `crates/worldwake-ai/src/goal_model.rs::prerequisite_places()` still only handles `TreatWounds` and `ProduceCommodity` and otherwise falls through to `_ => Vec::new()`. The real gap is specifically missing prerequisite-input guidance, not missing restock location guidance in general.
2. `RestockCommodity { commodity }` is defined in `worldwake-core/src/goal.rs:38–40` with a single `commodity: CommodityKind` field — no `recipe_id` field. The code must look up matching recipes from `RecipeRegistry` by output commodity.
3. `RESTOCK_OPS` (goal_model.rs lines 134–141) includes `PlannerOpKind::Craft`, confirming the planner already supports the craft chain — only spatial guidance is missing.
4. `ProduceCommodity` arm (lines 734–759) provides the pattern to follow: iterate recipe inputs, check actor's hypothetical quantity, call `acquisition_places_for_commodity()` for missing inputs, deduplicate via `append_unique_places()`, cap via `cap_places_by_travel_distance()`.
5. `RecipeRegistry` does not expose `recipes_for_output()`. The live API in `crates/worldwake-sim/src/recipe_registry.rs` exposes `get()` and `iter()`, so this ticket should iterate the registry and filter recipes whose outputs contain the requested commodity.
6. There is no existing focused unit test asserting the current empty behavior for `RestockCommodity::prerequisite_places()`. The nearby `RestockCommodity` tests around lines 2267, 2489, and 2515 in `crates/worldwake-ai/src/goal_model.rs` cover planner-step behavior and op families, not prerequisite-place resolution. This ticket needs new focused tests rather than updates to stale assertions.
7. `cargo test -p worldwake-ai -- --list` confirms the current focused prerequisite tests are only for `TreatWounds` and `ProduceCommodity`, plus one search-level summary test for remote treat-wounds. There is currently no restock-prerequisite coverage in either focused unit tests or goldens.
8. `crates/worldwake-ai/tests/golden_supply_chain.rs` still covers only the harvest-based restock segment for apples; the craft-based restock segment named in `specs/S18-prerequisite-aware-emergent-chain-goldens.md` remains unimplemented and is correctly left to later tickets.
9. No isolation issue exists at this ticket's layer. This is a `goal_model.rs` planning helper change and focused unit-test addition; no runtime or golden setup isolation is needed here.
10. Scope correction: this is not just "extend a match arm." The clean implementation should extract the shared "missing recipe input -> acquisition places" logic used by `ProduceCommodity` and `RestockCommodity` into a local helper inside `goal_model.rs`, rather than duplicating the loop in two match arms.

## Architecture Check

1. Reusing `acquisition_places_for_commodity()` and factoring the recipe-input scan into one local helper is cleaner than adding a second hand-written loop in `RestockCommodity`. The architecture already treats prerequisite-place computation as a goal-model concern; the robust extension is to share the recipe-input discovery logic, not fork it.
2. The alternative of expanding `RestockCommodity` to carry a `recipe_id` would overfit the goal model to a single recipe choice and would collapse a commodity-level enterprise goal into an early recipe commitment. That is worse for extensibility because multiple recipes may legitimately satisfy the same restock commodity.
3. No backward-compatibility shims or alias paths are needed. `RestockCommodity` currently under-expresses prerequisite places; fixing that directly is the clean authority path.

## Verification Layers

1. `RestockCommodity` for a craftable commodity exposes missing recipe-input locations -> focused unit test in `crates/worldwake-ai/src/goal_model.rs`
2. `RestockCommodity` with no matching recipe remains empty -> focused unit test in `crates/worldwake-ai/src/goal_model.rs`
3. `RestockCommodity` with all candidate recipe inputs already satisfied remains empty -> focused unit test in `crates/worldwake-ai/src/goal_model.rs`
4. Existing prerequisite-place behavior for `TreatWounds` and `ProduceCommodity` remains unchanged -> focused unit suite in `crates/worldwake-ai/src/goal_model.rs`
5. Search/planner integration remains intact after the helper change -> `cargo test -p worldwake-ai`
6. Single-layer ticket: no action-trace, decision-trace, or event-log proof surface is required because the contract under change is a pure planning helper.

## What to Change

### 1. Extend `prerequisite_places()` match in `goal_model.rs`

Add a `GoalKind::RestockCommodity { commodity }` arm before the `_ =>` fallback (approximately line 760). Logic:

1. Iterate `recipes.iter()` and keep recipes whose outputs contain `*commodity`.
2. For each matching recipe, iterate its inputs.
3. For each input where `state.commodity_quantity(actor, input_commodity) < required_quantity`, call `acquisition_places_for_commodity()`.
4. Deduplicate via `append_unique_places()`.
5. Cap via `cap_places_by_travel_distance()`.

If no recipes match the commodity, return `Vec::new()` (falls back to harvest-only restock, which works via `goal_relevant_places()` already).

**Implementation note**: avoid duplicating the existing `ProduceCommodity` recipe-input loop. Introduce a small local helper in `goal_model.rs` that collects prerequisite acquisition places for one recipe or for a filtered set of recipes, and have both `ProduceCommodity` and `RestockCommodity` use it.

### 2. Add/update unit test for `RestockCommodity` prerequisite places

Add a focused unit test `restock_commodity_prerequisite_places_returns_recipe_input_sources` that:
- Sets up a `PlanningState` with a `RecipeRegistry` containing `Bake Bread` (input: Firewood, output: Bread)
- Creates a `RestockCommodity { commodity: Bread }` goal
- Seeds a believed firewood source at a remote location
- Asserts `prerequisite_places()` returns that remote location
- Also asserts empty when actor already has firewood

Also add dedicated empty-path tests for:
- no matching recipe output
- all candidate recipe inputs already satisfied

## Files to Touch

- `crates/worldwake-ai/src/goal_model.rs` (modify — add `RestockCommodity` arm to `prerequisite_places()`)

## Out of Scope

- Golden E2E tests (covered by S18PREAWAEME-002 and S18PREAWAEME-003)
- Changes to `RecipeRegistry` API (use existing iteration methods)
- Changes to `goal_relevant_places()` (already handles `RestockCommodity`)
- Changes to `relevant_op_kinds()` (already includes `Craft` via `RESTOCK_OPS`)
- Any other `GoalKind` variants' `prerequisite_places()` behavior
- Changes to `worldwake-core`, `worldwake-sim`, or `worldwake-systems` crates

## Acceptance Criteria

### Tests That Must Pass

1. New unit test: `restock_commodity_prerequisite_places_returns_recipe_input_sources` — `RestockCommodity{Bread}` returns firewood source locations when actor lacks firewood
2. New unit test: `restock_commodity_prerequisite_places_empty_when_no_recipe` — `RestockCommodity` for a commodity with no matching recipe returns empty
3. New unit test: `restock_commodity_prerequisite_places_empty_when_inputs_satisfied` — returns empty when actor already has required recipe inputs
4. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. `ProduceCommodity` prerequisite_places behavior unchanged — no regression
2. `TreatWounds` prerequisite_places behavior unchanged — no regression
3. All other `GoalKind` variants still return empty from prerequisite_places
4. `prerequisite_places()` reads only from `PlanningState` (belief-derived), never from authoritative world state (Principle 12)

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/goal_model.rs` — `prerequisite_places_restock_commodity_include_missing_recipe_input_places`; proves commodity-level restock inherits missing-input place guidance from matching recipes
2. `crates/worldwake-ai/src/goal_model.rs` — `prerequisite_places_restock_commodity_empty_when_no_recipe_matches_output`; proves non-craftable restock stays empty
3. `crates/worldwake-ai/src/goal_model.rs` — `prerequisite_places_restock_commodity_empty_when_all_recipe_inputs_are_owned`; proves satisfied prerequisites do not emit redundant places
4. `crates/worldwake-ai/src/goal_model.rs` — existing `TreatWounds` and `ProduceCommodity` prerequisite tests remain unchanged; they guard against regressions in the shared helper

### Commands

1. `cargo test -p worldwake-ai prerequisite_places_restock_commodity`
2. `cargo test -p worldwake-ai`
3. `cargo clippy --workspace`

## Outcome

- Completion date: 2026-03-21
- What actually changed:
  - Corrected the ticket's stale assumptions before implementation.
  - Added three focused `RestockCommodity` prerequisite-place tests in `crates/worldwake-ai/src/goal_model.rs`.
  - Extended `GoalKind::prerequisite_places()` so `RestockCommodity` now unions prerequisite acquisition places from matching recipe inputs.
  - Extracted the shared recipe-input prerequisite scan into a local helper used by both `ProduceCommodity` and `RestockCommodity`.
- Deviations from original plan:
  - The implementation did not use a hypothetical `recipes_for_output()` API because the live `RecipeRegistry` exposes `iter()`.
  - The targeted verification command needed correction from an `--exact` form that matched zero tests to the real substring command.
  - The cleanest implementation was a small local helper extraction, not a second hand-written loop in the new match arm.
- Verification results:
  - `cargo test -p worldwake-ai prerequisite_places_restock_commodity` ✅
  - `cargo test -p worldwake-ai` ✅
  - `cargo clippy --workspace` ✅
