# S12PLAPREAWA-006: Focused coverage for prerequisite-aware search

**Status**: ✅ COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Already landed in live code before this reassessment: `goal_model.rs`, `search.rs`, `budget.rs`, `decision_trace.rs`, and `agent_tick.rs`
**Deps**: S12PLAPREAWA-001, S12PLAPREAWA-002, S12PLAPREAWA-003, S12PLAPREAWA-004, S12PLAPREAWA-005

## Problem

The original ticket assumed prerequisite-aware search production work had been split cleanly into S12PLAPREAWA-001 through 005 and that this ticket still needed to add the focused tests. That assumption is stale. The current repository already contains both the planner architecture and the focused/golden coverage needed for prerequisite-aware search.

This ticket is therefore closed as a reassessment-and-archival ticket, not as a fresh implementation ticket.

## Assumption Reassessment (2026-03-21)

1. `GoalKindPlannerExt::prerequisite_places()` already exists in [goal_model.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/goal_model.rs#L714) and is wired for `GoalKind::TreatWounds` and `GoalKind::ProduceCommodity`.
2. `combined_relevant_places()` already exists in [search.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/search.rs#L312) and returns a `CombinedRelevantPlaces` struct, not just a `Vec<EntityId>`.
3. `PlanningBudget::max_prerequisite_locations` already exists with default `3` in [budget.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/budget.rs#L9).
4. `SearchExpansionSummary` already carries `combined_places_count` and `prerequisite_places_count` in [decision_trace.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/decision_trace.rs#L204).
5. The planner is already using dynamic per-node combined places. `root_node()`, the search expansion loop, and successor construction all recompute combined places from hypothetical `PlanningState` in [search.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/search.rs#L156), [search.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/search.rs#L335), and [search.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/search.rs#L495). The old static pre-search place computation has already been removed from the call path in [agent_tick.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/agent_tick.rs).
6. The original `Engine Changes: None — test-only` claim is incorrect. The live architecture already includes the production changes this ticket treated as prerequisites.
7. The original list of 11 missing unit tests is also stale. Equivalent or stronger focused coverage already exists in [goal_model.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/goal_model.rs#L2712), [search.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/search.rs#L4300), [golden_care.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_care.rs#L698), and [golden_production.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_production.rs#L2299).

## Architecture Check

1. The live architecture is better than the pre-S12 terminal-only heuristic. Dynamic per-node `goal_relevant_places ∪ prerequisite_places` preserves belief-only planning, keeps search generic, and avoids HTN-style authored decomposition.
2. The current `CombinedRelevantPlaces` wrapper is cleaner than a bare vector because it keeps search guidance and traceability aligned. Search can consume the place set while decision tracing records how much of that guidance came from prerequisites.
3. No additional architectural rewrite is warranted in this ticket. The current design is robust, extensible, and already matches the intended S12 planner architecture. The stale part was the ticket, not the code.

## Verification Layers

1. prerequisite place derivation for `TreatWounds` and `ProduceCommodity` -> focused unit tests in `goal_model.rs`
2. dynamic place recomputation after hypothetical pickup -> focused unit tests in `search.rs`
3. travel pruning retains prerequisite detours instead of collapsing back to terminal-only guidance -> focused unit tests in `search.rs`
4. trace visibility of prerequisite-aware guidance -> focused unit tests in `search.rs` against `SearchExpansionSummary`
5. multi-hop medicine procurement through the full AI pipeline -> golden care tests
6. remote input acquisition before crafting at a workstation -> golden production test
7. package-level regression safety -> `cargo test -p worldwake-ai`
8. workspace lint cleanliness -> `cargo clippy --workspace --all-targets -- -D warnings`

## What Changed

No production or test code changes were needed during this reassessment. The necessary implementation and coverage were already present in the repository. This ticket was updated to reflect the live architecture, the actual focused tests, and the real verification commands.

## Files Touched

- `tickets/S12PLAPREAWA-006-unit-tests.md` (reassessed, marked completed, prepared for archival)

## Out of Scope

- Further planner changes
- Additional backward-compatibility layers or aliases
- Renaming already-adequate tests solely to match this ticket's original wording

## Acceptance Criteria

1. Ticket assumptions match the live code and live test suite.
2. The ticket no longer claims this is a test-only pending task.
3. The relevant focused tests, package suite, and workspace clippy pass.
4. The ticket is archived with an accurate `Outcome` section.

## Tests

## New/Modified Tests

1. `goal_model::tests::prerequisite_places_treat_wounds_include_remote_controllable_medicine_lot`
Rationale: proves `TreatWounds` exposes a remote loose medicine lot as a prerequisite location.
2. `goal_model::tests::prerequisite_places_treat_wounds_empty_when_actor_already_has_medicine`
Rationale: proves prerequisite guidance disappears once the actor already satisfies the medicine requirement.
3. `goal_model::tests::prerequisite_places_treat_wounds_prefer_loose_medicine_over_sellers_and_sources`
Rationale: proves direct controllable acquisition remains preferred over more indirect seller/resource-source routes.
4. `goal_model::tests::prerequisite_places_produce_commodity_include_missing_input_places`
Rationale: proves missing recipe inputs contribute acquisition locations to production planning.
5. `goal_model::tests::prerequisite_places_produce_commodity_partial_inputs_still_expose_missing_input_places`
Rationale: proves partial ownership still surfaces the remaining missing-input locations.
6. `goal_model::tests::prerequisite_places_produce_commodity_empty_when_inputs_are_already_owned`
Rationale: proves production prerequisite guidance is removed when all recipe inputs are already available.
7. `goal_model::tests::all_goal_kind_variants_have_prerequisite_places_impl`
Rationale: compile-time exhaustiveness guard for every `GoalKind` variant.
8. `search::tests::combined_places_include_remote_medicine_lot_for_treat_wounds`
Rationale: proves terminal and prerequisite locations are merged in search guidance.
9. `search::tests::combined_places_drop_medicine_place_after_hypothetical_pick_up`
Rationale: proves the heuristic updates dynamically after hypothetical state changes.
10. `search::tests::prune_travel_retains_remote_medicine_branch_for_treat_wounds`
Rationale: proves travel pruning does not incorrectly remove prerequisite detours.
11. `search::tests::search_expansion_summary_counts_prerequisite_places_for_remote_treat_wounds`
Rationale: proves decision traces expose prerequisite-aware search guidance.
12. `golden_healer_acquires_remote_ground_medicine_for_patient`
Rationale: end-to-end proof that the AI can travel, acquire remote medicine, return, and heal.
13. `golden_healer_acquires_remote_ground_medicine_for_patient_replays_deterministically`
Rationale: proves the multi-hop care scenario remains deterministic under replay.
14. `golden_multi_recipe_craft_path`
Rationale: end-to-end proof that prerequisite-aware planning also supports remote recipe-input acquisition before crafting.

## Commands

1. `cargo test -p worldwake-ai prerequisite_places`
2. `cargo test -p worldwake-ai combined_places`
3. `cargo test -p worldwake-ai search_expansion_summary_counts_prerequisite_places_for_remote_treat_wounds`
4. `cargo test -p worldwake-ai golden_healer_acquires_remote_ground_medicine_for_patient`
5. `cargo test -p worldwake-ai golden_multi_recipe_craft_path`
6. `cargo test -p worldwake-ai`
7. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

- Completion date: 2026-03-21
- What actually changed: the ticket was corrected to match already-landed prerequisite-aware search architecture and already-present focused/golden coverage, then archived.
- Deviations from original plan: the original ticket expected pending test-only work and 11 specific additions. In reality, the repository already contained the production planner changes plus equivalent or stronger focused and golden tests under different exact names.
- Verification results:
  - `cargo test -p worldwake-ai prerequisite_places` passed
  - `cargo test -p worldwake-ai combined_places` passed
  - `cargo test -p worldwake-ai search_expansion_summary_counts_prerequisite_places_for_remote_treat_wounds` passed
  - `cargo test -p worldwake-ai golden_healer_acquires_remote_ground_medicine_for_patient` passed
  - `cargo test -p worldwake-ai golden_multi_recipe_craft_path` passed
  - `cargo test -p worldwake-ai` passed
  - `cargo clippy --workspace --all-targets -- -D warnings` passed
