# S12PLAPREAWA-002: Align `prerequisite_places()` scope with current planner architecture

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: No new engine behavior; lock current behavior with focused coverage
**Deps**: None

## Problem

This ticket drifted behind the code. `GoalKindPlannerExt::prerequisite_places()` already exists and is already used by search, but the original ticket still describes an unimplemented trait method and a broader scope that no longer matches the planner architecture.

The main stale assumption is that `ProduceCommodity` should also use `prerequisite_places()`. Current candidate generation deliberately emits `AcquireCommodity` and suppresses `ProduceCommodity` while recipe inputs are missing. That decomposition is cleaner than teaching `ProduceCommodity` to perform input acquisition itself: acquisition remains a first-class goal, production remains the terminal conversion goal, and search avoids duplicating procurement logic across goal kinds.

The remaining work for this ticket is to correct the recorded scope, verify the live architecture, and add a regression test that makes the `AcquireCommodity`/`ProduceCommodity` split explicit before archival.

## Assumption Reassessment (2026-03-21)

1. `GoalKindPlannerExt` already defines `fn prerequisite_places(&self, state: &PlanningState<'_>, budget: &PlanningBudget) -> Vec<EntityId>` in `crates/worldwake-ai/src/goal_model.rs` and `GoalKind` already implements it.
2. `PlanningBudget::max_prerequisite_locations` already exists in `crates/worldwake-ai/src/budget.rs`. This ticket does not need to add it.
3. `search_plan()` in `crates/worldwake-ai/src/search.rs` still accepts a static `goal_relevant_places: &[EntityId]`, then dynamically unions that set with `goal.key.kind.prerequisite_places(...)` per node via `combined_relevant_places(...)`. This ticket does not own a search signature change.
4. The live `prerequisite_places()` implementation is intentionally narrow: it handles `TreatWounds` and returns `Vec::new()` for every other goal kind, including `ProduceCommodity`.
5. `crates/worldwake-ai/src/candidate_generation.rs` already encodes the production split that makes `ProduceCommodity` prerequisite search undesirable:
   - `missing_recipe_input_emits_acquire_goal_and_suppresses_produce_goal`
   - `satisfiable_recipe_with_current_need_emits_produce_goal`
6. Existing unit coverage already proves the `TreatWounds` branch works for remote loose medicine, for already-owned medicine, and for loose-lot priority over sellers/sources in `crates/worldwake-ai/src/goal_model.rs`.
7. The missing coverage gap is architectural, not functional: there is no focused `goal_model.rs` regression test asserting that `ProduceCommodity` continues to return no prerequisite places when inputs are missing because procurement belongs to `AcquireCommodity`.

## Architecture Check

1. Keeping `prerequisite_places()` on `GoalKindPlannerExt` is still the right abstraction. Search should ask the goal model for spatial guidance instead of hard-coding goal-specific prerequisite logic in `search.rs`.
2. Narrowing the method to `TreatWounds` is better than the original broader proposal. `TreatWounds` remains the selected goal even when medicine is missing, so prerequisite-aware search is the clean way to let that goal acquire medicine lawfully.
3. Extending `ProduceCommodity` the same way would now be a regression in architecture. Missing recipe inputs already resolve through `AcquireCommodity`, which keeps acquisition, production, and future procurement strategies decoupled. Duplicating those semantics inside `ProduceCommodity` would create overlapping goal ownership and make candidate generation and planning harder to reason about.
4. No backwards-compatibility shims or aliases are needed. The correct move is to archive this ticket against the architecture that actually stands up, not to force the code back toward the older plan.

## Verification Layers

1. `TreatWounds` prerequisite guidance remains present -> existing focused unit tests in `crates/worldwake-ai/src/goal_model.rs`
2. `ProduceCommodity` remains a terminal conversion goal rather than a procurement goal -> existing candidate-generation test plus one new focused `goal_model.rs` regression test
3. Planner/search integration for remote medicine procurement remains intact -> existing `search.rs` unit tests and `crates/worldwake-ai/tests/golden_care.rs`
4. Package and workspace regression safety -> `cargo test -p worldwake-ai`, `cargo test --workspace`, and `cargo clippy --workspace`

## What to Change

### 1. Correct the ticket scope

Keep this ticket focused on the already-landed `prerequisite_places()` hook and the architecture it now serves:

- `TreatWounds` uses prerequisite-aware spatial guidance
- `ProduceCommodity` does not; missing recipe inputs are handled by `AcquireCommodity`

### 2. Add one focused regression test

In `crates/worldwake-ai/src/goal_model.rs`, add a unit test asserting that `GoalKind::ProduceCommodity { .. }.prerequisite_places(...)` returns `[]` even when recipe inputs are missing. This locks in the deliberate split between procurement and production.

## Files to Touch

- `crates/worldwake-ai/src/goal_model.rs` (modify test module only)
- `tickets/S12PLAPREAWA-002-prerequisite-places-method.md` (this ticket)

## Out of Scope

- Expanding `prerequisite_places()` to `ProduceCommodity`
- Changing `search_plan()` signature
- Removing the static `goal_relevant_places` parameter from `agent_tick.rs` or `search.rs`
- Decision-trace enrichment
- New golden scenarios

## Acceptance Criteria

### Tests That Must Pass

1. `goal_model::tests::prerequisite_places_treat_wounds_include_remote_controllable_medicine_lot`
2. `goal_model::tests::prerequisite_places_treat_wounds_empty_when_actor_already_has_medicine`
3. `goal_model::tests::prerequisite_places_treat_wounds_prefer_loose_medicine_over_sellers_and_sources`
4. `goal_model::tests::prerequisite_places_produce_commodity_stays_empty_when_inputs_are_missing`
5. `goal_model::tests::all_goal_kind_variants_have_prerequisite_places_impl`
6. `cargo test -p worldwake-ai`
7. `cargo test --workspace`
8. `cargo clippy --workspace`

### Invariants

1. `prerequisite_places()` remains goal-model-owned logic rather than moving into search.
2. `TreatWounds` keeps prerequisite-aware guidance because the goal remains active while medicine is missing.
3. `ProduceCommodity` keeps returning no prerequisite places because missing recipe inputs are modeled as `AcquireCommodity`, not hidden substructure inside production.

## Tests

### New/Modified Tests

1. `crates/worldwake-ai/src/goal_model.rs` -> `prerequisite_places_produce_commodity_stays_empty_when_inputs_are_missing`

### Rationale

1. Guards the current architecture against future drift by proving production does not silently absorb procurement logic that already belongs to candidate generation and `AcquireCommodity`.

### Commands

1. `cargo test -p worldwake-ai prerequisite_places_`
2. `cargo test -p worldwake-ai goal_model::tests::prerequisite_places_produce_commodity_stays_empty_when_inputs_are_missing`
3. `cargo test -p worldwake-ai`
4. `cargo test --workspace`
5. `cargo clippy --workspace`

## Outcome

- Completion date: 2026-03-21
- What actually changed:
  - Reassessed the ticket against the live `worldwake-ai` implementation and corrected the scope to match the architecture that is already in production.
  - Added `goal_model::tests::prerequisite_places_produce_commodity_stays_empty_when_inputs_are_missing` in `crates/worldwake-ai/src/goal_model.rs`.
- Deviations from the original plan:
  - Did not expand `prerequisite_places()` to `ProduceCommodity`.
  - Kept the current architecture where missing recipe inputs emit `AcquireCommodity` and suppress `ProduceCommodity`, which is cleaner and more extensible than duplicating procurement semantics inside production goals.
  - Did not change `search_plan()` or `agent_tick.rs`; those remain separate concerns and already reflect a later implementation path than the original ticket text described.
- Verification results:
  - `cargo test -p worldwake-ai prerequisite_places_`
  - `cargo test -p worldwake-ai goal_model::tests::all_goal_kind_variants_have_prerequisite_places_impl`
  - `cargo test -p worldwake-ai candidate_generation::tests::missing_recipe_input_emits_acquire_goal_and_suppresses_produce_goal`
  - `cargo test -p worldwake-ai`
  - `cargo test --workspace`
  - `cargo clippy --workspace`
