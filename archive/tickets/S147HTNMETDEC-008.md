# S147HTNMETDEC-008: Planner integration in build_stages

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — modifies `build_stages` in `crates/worldwake-ai/src/search/strategic.rs` to consult the method selector before flat-GOAP fallback, and carries `AgentSchemaContextProfile` through the planning snapshot/state boundary.
**Deps**: `archive/tickets/S147HTNMETDEC-006.md` (MethodRegistry + explicit method binding templates), `archive/tickets/S147HTNMETDEC-007.md` (`select_method` with actor-relative belief evaluation)

## Problem

S147 D4 wires `select_method` into the strategic search so methods can substitute their subgoals into the planner's stage list. Without this integration, the method registry and selector are unreachable from the planner and have no behavioral effect. The integration must preserve the flat-GOAP fallback path when no method applies, modify the existing `build_stages` function rather than introducing a parallel path, and keep the existing strategic-search loop and tactical descent machinery intact.

## Assumption Reassessment (2026-05-17)

1. `build_stages` exists in `crates/worldwake-ai/src/search/strategic.rs` and is called from `plan_with_budget_trace`. This ticket owns that single planner stage-decomposition entry point; no second planner decomposition function is introduced.
2. The shared boundary is `crate::htn::select_method(actor, goal, registry, profile, belief_view, motives)`. This ticket uses the selector result to choose method subgoals, then maps those subgoals to existing `StrategicStage` values. Tactical search and action validation are unchanged.
3. The method selector requires `AgentSchemaContextProfile` through `ProfileBeliefView`. Before this ticket, `PlanningState` did not expose the actor's `AgentSchemaContextProfile`, so the planner-visible snapshot/state boundary had to be extended rather than reading authoritative world state during planning.
4. Existing flat-GOAP strategic tests remain the regression surface for goals without matching methods. The broad `worldwake-ai` suite also exposed a real restock fallback boundary: canonical restock methods must not convert an already-terminal same-commodity restock goal into an unrelated acquisition stage.
5. During implementation, a separate selector gap was exposed: canonical recipe-input method preconditions such as `CommodityTemplate::RecipeInput { recipe: GoalRecipe, ordinal: 0 }` are not currently resolved by `htn::selector`. That gap is separate from planner dispatch and is captured in `tickets/S147HTNMETDEC-012.md`.

## Architecture Check

1. The method branch is a prefix in the existing `build_stages` function. If `select_method` returns `None` or the selected method cannot resolve any stage, the existing flat-GOAP logic still runs.
2. `PlanningSnapshot` now carries the actor's `AgentSchemaContextProfile`, and `PlanningState` exposes it through `ProfileBeliefView`. That preserves belief-only planning and avoids an authoritative read from the planner integration path.
3. `template_to_stages` resolves HTN templates into the existing `StrategicStageKind::{Acquire, Goal}` vocabulary. This keeps method decomposition as reusable affordance composition rather than a parallel action executor.
4. No backwards-compatibility shims or alternate planner entry points were added.

## Verified Layers

1. Method selection branch fires and substitutes method subgoals -> focused `search::strategic` unit test using a custom method registry and planner-visible beliefs.
2. Flat-GOAP fallback remains valid -> existing `search::strategic` tests continue to pass.
3. Restock tactical contract remains unchanged under canonical methods -> focused restock route-preference regression test.
4. Planner snapshot/state carries method-disable profile through the belief boundary -> compile coverage plus `worldwake-ai` tests over `PlanningState`/strategic search consumers.
5. Broad AI regression surface -> `cargo test -p worldwake-ai`.

## Landed Changes

### 1. Method-first stage decomposition

`crates/worldwake-ai/src/search/strategic.rs` now builds the canonical method registry in `plan_with_budget_trace`, reads the actor's `AgentSchemaContextProfile` from `PlanningState`, and passes both into `build_stages`. `build_stages` calls `select_method` first, expands selected method subgoals through `template_to_stages`, collapses repeated stages, appends the existing terminal goal stage when needed, and otherwise falls through to the pre-existing flat-GOAP logic.

### 2. HTN template expansion

`template_to_stages` and its resolver helpers map method templates for commodities, locations, entities, claims, artifacts, and action payloads to deterministic `StrategicStage` sequences using planner-visible beliefs and the `RecipeRegistry`. Same-commodity `RestockCommodity` method acquire templates resolve to the existing terminal goal stage so restock route preference remains unchanged.

### 3. Planner-visible profile boundary

`crates/worldwake-ai/src/planning_snapshot.rs` stores `AgentSchemaContextProfile` in `SnapshotProfiles`, and `crates/worldwake-ai/src/planning_state.rs` exposes it through `ProfileBeliefView::agent_schema_context_profile`. This lets method disablement work in planning without violating the belief-only planning boundary.

### 4. Focused test coverage

`method_selection_substitutes_method_subgoals_into_stage_list` constructs a planner-visible grain source, mill workstation, recipe, and custom matching method schema. It asserts that `build_stages` returns the method's `Acquire(Grain)` stage followed by the workstation goal stage rather than the flat missing-commodity-only stage list.

## Landed Files

- `crates/worldwake-ai/src/search/strategic.rs` (method-first `build_stages`, template resolvers, focused method-selection test)
- `crates/worldwake-ai/src/planning_snapshot.rs` (snapshot profile carrier)
- `crates/worldwake-ai/src/planning_state.rs` (`ProfileBeliefView` profile accessor)

## Outcome

Completed: 2026-05-17.

Planner integration is landed. `build_stages` now considers the S147 method registry before flat-GOAP fallback, selected method subgoals are expanded into the existing strategic stage vocabulary, and actor method-disable profile data reaches the selector through the planner snapshot/state belief boundary.

## Deviations

- The positive method-branch proof uses a custom method registry with concrete commodity templates. Canonical first-ship `ProduceCommodity` methods are still blocked by the pre-existing selector resolver gap for `CommodityTemplate::RecipeInput`, captured in `tickets/S147HTNMETDEC-012.md`.
- Full `./scripts/verify.sh` remains the final pre-push gate for the S147 harness run. This ticket iteration used the narrower `worldwake-ai` proof surface plus all-target clippy for the crate because the touched runtime code is contained to `worldwake-ai`.

## Out of Scope

- `MethodPlanAttemptTrace` recording during plan attempts (ticket 009).
- Observer rendering of method choice (ticket 010).
- Golden end-to-end method scenarios (ticket 011).
- Selector resolution for recipe-input method preconditions (ticket 012).

## Acceptance Result

### Tests Passed

1. `cargo test -p worldwake-ai --lib search::strategic`
2. `cargo test -p worldwake-ai --lib search::tests::search_restock_route_preference_follows_believed_combat_threat -- --exact`
3. `cargo test -p worldwake-ai`
4. `cargo clippy -p worldwake-ai --all-targets -- -D warnings`

### Invariants

1. `build_stages` remains the single strategic stage-decomposition function in `strategic.rs`.
2. When no method is selected, `build_stages` falls through to the existing flat-GOAP logic.
3. Method template expansion is deterministic over the same goal, recipe registry, and planner-visible beliefs.
4. No floats were introduced into the planner integration path.

## Verification Result

Passed the focused strategic tests, the restock route-preference regression test, the full `worldwake-ai` crate test suite, and all-target clippy for `worldwake-ai`.

## Test Plan Result

### Added/Modified Tests

1. `crates/worldwake-ai/src/search/strategic.rs` — added `method_selection_substitutes_method_subgoals_into_stage_list`; existing inline strategic tests continue to exercise the fallback path.

### Verification Commands

1. `cargo test -p worldwake-ai --lib search::strategic`
2. `cargo test -p worldwake-ai --lib search::tests::search_restock_route_preference_follows_believed_combat_threat -- --exact`
3. `cargo test -p worldwake-ai`
4. `cargo clippy -p worldwake-ai --all-targets -- -D warnings`
