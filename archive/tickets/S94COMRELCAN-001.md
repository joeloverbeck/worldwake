# S94COMRELCAN-001: Add `target_commodity()` method and `CommodityIrrelevant` trace variant

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: None — planner-internal additions only
**Deps**: S90 (completed), S93 (completed)

## Problem

The GOAP planner generates 1400-5700 candidates for simple commodity goals because no commodity-relevance filtering exists. Before the filter can be built (ticket 002), the planner needs a way to extract the target commodity from a goal, and the decision trace system needs a variant to record why candidates were filtered.

## Assumption Reassessment (2026-04-11)

1. `GoalKindPlannerExt` trait exists at `crates/worldwake-ai/src/goal_model.rs:38` with 11 methods. No `target_commodity()` method exists yet. Existing `relevant_observed_commodities(&self, recipes: &RecipeRegistry)` at line 41 provides the parameter pattern to follow.
2. `RootCandidateFilterReason` enum exists at `crates/worldwake-ai/src/decision_trace.rs:672` with 4 variants: `BindingMismatch`, `GoalUnavailable`, `BlockedFacilityUse`, `PlaceBlocker`. No `CommodityIrrelevant` variant exists yet.
3. `social_query_commodity()` is a private function at `crates/worldwake-ai/src/search/strategic.rs:221-233` performing goal-to-commodity mapping only for the strategic social-query fallback surface: `AcquireCommodity`, `ConsumeOwnedCommodity`, `RestockCommodity`, `TreatWounds`, and `ProduceCommodity` via `missing_commodities.first()`. The new `target_commodity()` is a superset. Reusing it is still correct, but only for the overlapping subset; blindly delegating all goals would widen social-query fallback to goals like `SellCommodity`, `MoveCargo`, and `FreeCarryCapacity`, which is not the live strategic contract.
4. `GoalKind` variants confirmed at `crates/worldwake-core/src/goal.rs:18-133`: `AcquireCommodity { commodity, purpose }`, `ConsumeOwnedCommodity { commodity }`, `RestockCommodity { commodity }`, `SellCommodity { commodity }`, `MoveCargo { commodity, destination }`, `TreatWounds { patient }`, `ProduceCommodity { recipe_id }`, `FreeCarryCapacity` (unit variant).
5. `RecipeRegistry` is in `worldwake-sim` and already imported in `goal_model.rs` (used by `relevant_observed_commodities` and `goal_relevant_places`).
6. `CommodityKind` is in `worldwake-core::items` (line 10), already imported in `goal_model.rs`.

## Architecture Check

1. Adding `target_commodity()` to `GoalKindPlannerExt` follows the established pattern — the trait already has `relevant_observed_commodities()` with the same `recipes: &RecipeRegistry` parameter. The new method is a simpler single-commodity extraction vs. the existing set-returning method.
2. No backward-compatibility shims. The method is new and the `social_query_commodity()` refactoring can reuse it only for the subset of goals already supported by strategic social-query fallback, preserving existing planner behavior instead of widening it.

## Verification Layers

1. `target_commodity()` returns correct commodity for each GoalKind variant → focused unit tests matching each row of the spec's mapping table
2. `CommodityIrrelevant` variant compiles and serializes correctly → workspace build + clippy
3. `social_query_commodity()` refactoring preserves strategic planner behavior → existing strategic search tests in `search/strategic.rs`
4. Single-layer ticket (planner-internal additions) — no cross-system verification needed

## What to Change

### 1. Add `target_commodity()` to `GoalKindPlannerExt` trait

In `crates/worldwake-ai/src/goal_model.rs`:

- Add trait method declaration: `fn target_commodity(&self, recipes: &RecipeRegistry) -> Option<CommodityKind>;`
- Add implementation in the `impl GoalKindPlannerExt for GoalKind` block (starts at line 548):

```
AcquireCommodity { commodity, .. } => Some(commodity)
ConsumeOwnedCommodity { commodity } => Some(commodity)
RestockCommodity { commodity } => Some(commodity)
SellCommodity { commodity } => Some(commodity)
MoveCargo { commodity, .. } => Some(commodity)
TreatWounds { .. } => Some(CommodityKind::Medicine)
ProduceCommodity { recipe_id } => recipes.get(recipe_id).and_then(|r| r.outputs.first().map(|(c, _)| c)).copied()
FreeCarryCapacity => Some(CommodityKind::Waste)
_ => None
```

### 2. Add `CommodityIrrelevant` variant to `RootCandidateFilterReason`

In `crates/worldwake-ai/src/decision_trace.rs`:

- Add to the `RootCandidateFilterReason` enum (at line 672):

```rust
CommodityIrrelevant {
    candidate_commodity: Option<CommodityKind>,
    goal_commodity: CommodityKind,
}
```

- Ensure `CommodityKind` is imported (from `worldwake_core::items`).

### 3. Refactor `social_query_commodity()` to reuse `target_commodity()` without widening fallback behavior

In `crates/worldwake-ai/src/search/strategic.rs`:

- Replace the overlapping branches of `social_query_commodity()` (lines 221-233) to call `goal.key.kind.target_commodity(recipes)` for the goals it already socially queries: `AcquireCommodity`, `ConsumeOwnedCommodity`, `RestockCommodity`, and `TreatWounds`.
- Preserve the existing `ProduceCommodity => missing_commodities.first().copied()` behavior so strategic fallback keeps querying for the missing recipe input commodity rather than the goal output commodity.
- Keep all other goals returning `None`, even if `target_commodity()` returns `Some(_)`, so strategic fallback does not expand to new goal families in this ticket.
- Verify all call sites of `social_query_commodity()` still compile.

## Files to Touch

- `crates/worldwake-ai/src/goal_model.rs` (modify) — trait declaration + impl
- `crates/worldwake-ai/src/decision_trace.rs` (modify) — new enum variant
- `crates/worldwake-ai/src/search/strategic.rs` (modify) — refactor `social_query_commodity()`

## Out of Scope

- The commodity-relevance filter function itself (ticket 002)
- Integration of the filter into the search pipeline (ticket 002)
- Golden test rewrites (ticket 003)
- Modifying `matches_binding()` — operates at binding level, not commodity relevance
- Changing `CognitiveProfile` or `ExecutionBudget` parameters
- Per-agent filter tuning

## Acceptance Criteria

### Tests That Must Pass

1. New focused unit tests for `target_commodity()` covering every GoalKind mapping row
2. Existing strategic search tests: `cargo test -p worldwake-ai strategic`
3. Existing suite: `cargo test --workspace`

### Invariants

1. `target_commodity()` returns `Some(commodity)` for every commodity-focused goal variant and `None` for non-commodity goals
2. `ProduceCommodity` returns the recipe's primary output commodity via `RecipeRegistry` lookup
3. `social_query_commodity()` refactoring produces identical results to the original for all currently supported social-query goal families and does not make new goal families socially queryable
4. `CommodityIrrelevant` variant is exhaustively handled in all existing match sites on `RootCandidateFilterReason`

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/goal_model.rs` (new `#[cfg(test)]` tests) — unit tests for `target_commodity()` covering each GoalKind mapping row, including `ProduceCommodity` with a populated `RecipeRegistry`
2. `crates/worldwake-ai/src/search/strategic.rs` (new `#[cfg(test)]` test) — prove `social_query_commodity()` still does not widen strategic social-query fallback to goals outside the pre-existing subset

### Commands

1. `cargo test -p worldwake-ai target_commodity` — new focused goal-model tests
2. `cargo test -p worldwake-ai strategic` — verify refactored strategic planner and preserved social-query scope
3. `cargo clippy --workspace --all-targets -- -D warnings` — ensure new variant is handled everywhere
4. `cargo test --workspace` — full regression

## Outcome

Completed on 2026-04-11.

- Added `GoalKindPlannerExt::target_commodity(&RecipeRegistry)` in `crates/worldwake-ai/src/goal_model.rs` with focused coverage for the mapped goal rows and missing-recipe fallback.
- Added `RootCandidateFilterReason::CommodityIrrelevant { candidate_commodity, goal_commodity }` in `crates/worldwake-ai/src/decision_trace.rs` to land the trace surface needed by ticket 002.
- Refactored `crates/worldwake-ai/src/search/strategic.rs` so `social_query_commodity()` reuses `target_commodity()` only for the pre-existing social-query goal subset and keeps `ProduceCommodity` on the missing-input path.
- Added a strategic regression test proving `SellCommodity` does not gain new social-query fallback behavior from the broader `target_commodity()` mapping.

## Deviations

- Reassessment corrected the strategic refactor scope: `social_query_commodity()` could not blindly delegate to `target_commodity()` for every goal that now maps to a commodity, because that would widen fallback behavior beyond the current strategic contract.

## Verification Result

- Passed `cargo test -p worldwake-ai target_commodity`
- Passed `cargo test -p worldwake-ai strategic`
- Passed `cargo clippy --workspace --all-targets -- -D warnings`
- Passed `cargo test --workspace`
