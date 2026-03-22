# S12PLAPREAWA-001: Add prerequisite-aware location budgeting to planner search

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — `PlanningBudget`, `GoalKindPlannerExt`, planner semantics/state, search guidance, care golden coverage
**Deps**: `archive/specs/S12-planner-prerequisite-aware-search.md`

## Problem

`TreatWounds` candidates are generated even when the healer lacks Medicine, but planner search only guides toward the patient's location. When the patient is local and Medicine is remote, the planner has no spatial signal for the prerequisite acquisition leg, so lawful plans like Travel(remote medicine) -> PickUp -> Travel(patient) -> Heal are not found reliably. The current ticket split is also stale: adding an unused budget field alone would create dead configuration without fixing the planner.

## Assumption Reassessment (2026-03-21)

1. `PlanningBudget` lives in `crates/worldwake-ai/src/budget.rs` and already has focused default + serde coverage: `budget::tests::planning_budget_default_matches_ticket_values` and `budget::tests::planning_budget_roundtrips_through_bincode`.
2. `search_plan()` in `crates/worldwake-ai/src/search.rs` still takes a static `goal_relevant_places: &[EntityId]`, and `agent_tick.rs` computes that slice once before search. The heuristic and travel pruning therefore use one fixed place set for the entire search.
3. `GoalKindPlannerExt::goal_relevant_places()` currently returns only the patient location for `GoalKind::TreatWounds { patient }` in `crates/worldwake-ai/src/goal_model.rs`; it does not incorporate Medicine acquisition locations.
4. `GoalKind::TreatWounds` already permits acquisition-like intermediate ops via `TREAT_WOUNDS_OPS` in `crates/worldwake-ai/src/goal_model.rs`, including `Travel`, `Trade`, `QueueForFacilityUse`, `Craft`, `MoveCargo`, and `Harvest`.
5. `MoveCargo` hypothetical pickup is modeled in `apply_pick_up_transition()` in `crates/worldwake-ai/src/planner_ops.rs`, but the live remote-care path still depended on adjacent planner surfaces lining up: `MoveCargo` had to be marked relevant for `TreatWounds`, authoritative item-lot load had to remain derivable from belief state, and the default depth budget had to admit the actual prototype route length.
6. Existing golden care coverage is broader than this ticket originally claimed. `crates/worldwake-ai/tests/golden_care.rs` already proves local medicine acquisition and healing via `golden_healer_acquires_ground_medicine_for_patient`; this ticket therefore touches AI integration and needs new non-local coverage, not just a struct-field unit test.
7. The original spec and sibling tickets overstate `ProduceCommodity` as part of the same planner gap. In the current architecture, `emit_produce_goals()` in `crates/worldwake-ai/src/candidate_generation.rs` suppresses `ProduceCommodity` when recipe inputs are missing and emits `AcquireCommodity { purpose: RecipeInput(..) }` instead, as covered by `candidate_generation::tests::missing_recipe_input_emits_acquire_goal_and_suppresses_produce_goal`. Remote recipe-input procurement is therefore already handled through a different goal family and is out of scope for this ticket.
8. Because the live gap is cross-layer (`goal_model` spatial metadata -> `search` heuristic/pruning -> `agent_tick` call site -> golden care behavior), the previous “single-layer ticket” framing was incorrect.
9. Mismatch + correction: this ticket is no longer “add one unused field.” It is the smallest coherent vertical slice for remote-Medicine `TreatWounds` planning: add the budget knob, teach the goal model to expose prerequisite locations for `TreatWounds`, tighten travel pruning at alternative relevant places, align `MoveCargo` semantics with the goal model, keep item-lot load derivable in planning state, raise the default depth budget to fit the lawful prototype route, and cover the behavior with focused and golden tests.

## Architecture Check

1. The cleaner architecture is to keep goal-specific prerequisite discovery in `GoalKindPlannerExt` and keep `search.rs` generic. Hardcoding Medicine logic into `search.rs` would couple planner infrastructure to one goal family.
2. The budget field only belongs if it is consumed immediately by search guidance. Leaving it unused would be dead authority-path configuration and a poor ticket boundary.
3. The current architecture already routes missing recipe inputs through `AcquireCommodity`, so expanding this ticket to `ProduceCommodity` would duplicate live behavior instead of improving it.
4. The cleaner long-lived architecture is to derive cargo load for item lots from commodity + quantity in planning state rather than trusting snapshot baggage. That keeps hypothetical cargo transitions correct for both authoritative and reported beliefs.
5. The prototype topology is part of the architecture contract. If a lawful baseline route requires eight planner steps, a default depth of six is underspecified rather than conservative.
6. No backwards-compatibility shims or alias paths: the old static-place search path is replaced directly.

## Verification Layers

1. `TreatWounds` can expose remote Medicine prerequisite locations when the healer lacks Medicine -> focused `goal_model.rs` unit tests.
2. Search heuristic/pruning uses dynamic combined places instead of a static patient-only slice -> focused `search.rs` unit tests.
3. `TreatWounds` search exposes and can hypothetically apply `pick_up` against the actual remote-care golden snapshot -> focused golden-side planner test.
4. The prototype remote-care route is impossible at depth `6` but valid at depth `8` -> focused search-budget golden-side test.
5. AI can actually execute the remote pickup-then-heal chain -> golden care scenario with authoritative world-state assertions and action-trace assertions.
6. Deterministic replay of the new care scenario remains stable -> replay companion golden test.

## What to Change

### 1. Extend `PlanningBudget`

In `crates/worldwake-ai/src/budget.rs`, add:

```rust
pub max_prerequisite_locations: u8,
```

Set the default to `3`, raise `max_plan_depth` from `6` to `8` so the lawful Orchard Farm round trip fits the prototype topology, and extend the existing budget tests to assert the new field and updated depth default.

### 2. Add prerequisite location support to `GoalKindPlannerExt`

In `crates/worldwake-ai/src/goal_model.rs`, add:

```rust
fn prerequisite_places(
    &self,
    state: &PlanningState<'_>,
    budget: &PlanningBudget,
) -> Vec<EntityId>;
```

Current live implementation scope:

- `GoalKind::TreatWounds { .. }`: if the actor has no Medicine in the hypothetical `PlanningState`, prefer direct loose Medicine lots; only fall back to sellers/resource sources when no loose lot is known; cap returned places to the N closest by travel distance.
- All other goal kinds: return `Vec::new()` for now.

This keeps the extension point generic without inventing stale behavior for goal families that do not currently need it.

### 3. Replace static search places with per-node combined places

In `crates/worldwake-ai/src/search.rs`:

- add a private `combined_relevant_places()` helper that unions `goal_relevant_places()` and `prerequisite_places()`,
- keep the `search_plan()` call shape intact,
- recompute combined relevant places at the root, before travel pruning, and after each hypothetical transition before heuristic evaluation,
- when the actor is already at one relevant place, prune travel against the remaining relevant places instead of disabling pruning completely.

`compute_heuristic()` and `prune_travel_away_from_goal()` stay generic over `&[EntityId]`; only their callers change.

### 4. Align planner semantics and planning-state cargo derivation

- In `crates/worldwake-ai/src/planner_ops.rs`, mark `MoveCargo` as relevant for `TreatWounds`.
- In `crates/worldwake-ai/src/planning_state.rs`, derive authoritative item-lot load from commodity quantity instead of trusting snapshot `intrinsic_load`.

### 5. Add focused and golden coverage for the real gap

- Focused unit coverage in `crates/worldwake-ai/src/budget.rs`, `crates/worldwake-ai/src/goal_model.rs`, `crates/worldwake-ai/src/planning_state.rs`, `crates/worldwake-ai/src/planner_ops.rs`, and `crates/worldwake-ai/src/search.rs`
- Golden care coverage for remote medicine pickup before healing in `crates/worldwake-ai/tests/golden_care.rs`

## Files to Touch

- `crates/worldwake-ai/src/budget.rs` (modify)
- `crates/worldwake-ai/src/goal_model.rs` (modify)
- `crates/worldwake-ai/src/planner_ops.rs` (modify)
- `crates/worldwake-ai/src/planning_state.rs` (modify)
- `crates/worldwake-ai/src/search.rs` (modify)
- `crates/worldwake-ai/tests/golden_care.rs` (modify)

## Out of Scope

- `ProduceCommodity` prerequisite guidance; current candidate generation already redirects missing-input cases to `AcquireCommodity`
- Decision-trace schema expansion
- New planner infrastructure beyond dynamic place recomputation
- Spec or sibling-ticket cleanup beyond this ticket unless requested separately

## Acceptance Criteria

### Tests That Must Pass

1. Focused `goal_model` tests prove `TreatWounds` prerequisite places are empty when Medicine is already held, include remote lots when needed, and prefer direct loose lots over more indirect acquisition routes.
2. Focused `search` tests prove combined relevant places include prerequisite locations when needed and that travel pruning still narrows the search while allowing departure from one relevant place toward another.
3. Focused planning-state / golden-side tests prove remote-care planning can expose and hypothetically apply `pick_up` for the reported remote Medicine lot.
4. Focused golden-side search-budget test proves the prototype remote-care route is impossible at depth `6` and valid at depth `8`.
5. New golden care scenario proves a healer can travel to remote Medicine, pick it up, return, and commit `heal`.
6. Existing suite: `cargo test -p worldwake-ai`
7. Existing suite: `cargo clippy --workspace --all-targets -- -D warnings`

### Invariants

1. The planner still reasons from belief-only `PlanningState`; no authoritative world reads are added to search guidance.
2. Goals other than `TreatWounds` keep pre-existing search guidance unless they explicitly opt into `prerequisite_places()`.
3. The new budget field is consumed immediately by planner behavior; no unused config is introduced.
4. The default planner depth budget must admit lawful baseline routes in the prototype topology.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/budget.rs` — extend default + serde tests to cover `max_prerequisite_locations` and the new default `max_plan_depth = 8`.
2. `crates/worldwake-ai/src/goal_model.rs` — add `TreatWounds` prerequisite-place tests and an exhaustive coverage test for the new trait method.
3. `crates/worldwake-ai/src/search.rs` — add combined-places/pruning coverage plus a focused `TreatWounds` candidate test that proves `pick_up` is considered at the medicine location.
4. `crates/worldwake-ai/src/planning_state.rs` — add authoritative item-lot load derivation coverage for sparse belief snapshots.
5. `crates/worldwake-ai/tests/golden_care.rs` — add remote-care planner-state/search-budget focused tests plus a remote-medicine procurement scenario and deterministic replay companion.

### Commands

1. `cargo test -p worldwake-ai budget::tests::planning_budget_default_matches_ticket_values`
2. `cargo test -p worldwake-ai goal_model::tests::prerequisite_places`
3. `cargo test -p worldwake-ai search::tests::combined_places`
4. `cargo test -p worldwake-ai remote_treat_wounds_search_needs_eight_step_depth_budget_in_prototype_topology -- --nocapture`
5. `cargo test -p worldwake-ai golden_healer_acquires_remote_ground_medicine_for_patient -- --nocapture`
6. `cargo test -p worldwake-ai`
7. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

Completed: 2026-03-21

What actually changed:

- Added `PlanningBudget.max_prerequisite_locations` with default coverage and raised the default `max_plan_depth` from `6` to `8`.
- Added `GoalKindPlannerExt::prerequisite_places()` and implemented a `TreatWounds` strategy that prefers direct loose Medicine lots before broader acquisition routes.
- Changed search to use dynamic combined relevant places and to prune travel toward alternative relevant places even when the actor starts on one relevant place.
- Marked `MoveCargo` as relevant for `TreatWounds` and kept authoritative item-lot load derivable from belief-state commodity quantities.
- Added focused and golden tests covering prerequisite-place selection, travel pruning, cargo relevance, sparse-load derivation, remote-care planning depth, remote pickup, and deterministic replay.

Deviations from original plan:

- `search_plan()` and `agent_tick.rs` call shapes were left intact; the dynamic place recomputation happens inside `search.rs`.
- The implemented fix needed planner-semantics/state alignment and a higher default depth budget, not just the prerequisite-location budget field originally proposed.
- The final golden scenario waits for a committed `heal` step rather than stopping at the first wound-state delta.

Verification results:

- `cargo test -p worldwake-ai`
- `cargo clippy --workspace --all-targets -- -D warnings`
