# S12PLAPREAWA-004: Reassess and verify `agent_tick.rs` integration for prerequisite-aware `search_plan()`

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: No new engine changes in this ticket; verified previously delivered planner integration
**Deps**: S12PLAPREAWA-003 (new `search_plan()` signature)

## Problem

This ticket was written as a pending single-call-site follow-up, but the codebase no longer matches that premise. The prerequisite-aware planner integration has already landed across `search.rs`, `goal_model.rs`, `budget.rs`, `agent_tick.rs`, and focused/golden tests. The remaining work is to correct this ticket so it reflects the delivered architecture, verify the live integration still behaves as intended, and archive the ticket with accurate scope and outcomes.

## Assumption Reassessment (2026-03-21)

1. `search_plan()` no longer accepts `goal_relevant_places: &[EntityId]`; the live signature in [`crates/worldwake-ai/src/search.rs`](../crates/worldwake-ai/src/search.rs) takes `recipes: &RecipeRegistry` and computes dynamic spatial guidance internally via `combined_relevant_places()`.
2. The `agent_tick.rs` call site is already updated. [`crates/worldwake-ai/src/agent_tick.rs`](../crates/worldwake-ai/src/agent_tick.rs) passes `recipe_registry` directly to `search_plan()` and no longer precomputes a root-only `goal_relevant_places` slice.
3. The prerequisite-aware architecture from the parent spec is already present:
   - `GoalKindPlannerExt::prerequisite_places()` exists in [`crates/worldwake-ai/src/goal_model.rs`](../crates/worldwake-ai/src/goal_model.rs).
   - `PlanningBudget::max_prerequisite_locations` exists in [`crates/worldwake-ai/src/budget.rs`](../crates/worldwake-ai/src/budget.rs).
   - `combined_relevant_places()` exists in [`crates/worldwake-ai/src/search.rs`](../crates/worldwake-ai/src/search.rs).
4. The ticket's original "call-site only" scope is therefore stale. This ticket should not request fresh implementation of an already-delivered path.
5. The original "no AI regression or heuristic concerns" assumption was too narrow. Even for a call-site change, the real invariant is end-to-end planner behavior through `agent_tick`, because static root-only spatial guidance would silently regress remote prerequisite acquisition.
6. Existing focused and golden coverage already exercises the intended behavior:
   - `remote_treat_wounds_search_needs_eight_step_depth_budget_in_prototype_topology`
   - `golden_healer_acquires_remote_ground_medicine_for_patient`
   - `golden_materialization_barrier_chain`
   - `golden_multi_recipe_craft_path`

## Architecture Check

1. The delivered design is architecturally better than the original static call-site slice: spatial guidance now lives beside the search-state transitions in `search.rs`, so prerequisite locations can disappear after hypothetical pickup or other mid-plan state changes.
2. Keeping this logic out of `agent_tick.rs` is cleaner and more extensible. `agent_tick` should orchestrate planning, not precompute stale heuristic inputs that belong to the planner's own state model.
3. No backward-compatibility shim or alias path is needed or desirable. The old root-only `goal_relevant_places` handoff is gone rather than preserved.

## Verification Layers

1. `agent_tick` integration uses the dynamic planner entrypoint -> targeted source inspection of `plan_ranked_candidates()` in `crates/worldwake-ai/src/agent_tick.rs`
2. Search-layer prerequisite guidance remains active -> focused tests `remote_treat_wounds_search_needs_eight_step_depth_budget_in_prototype_topology` and `remote_treat_wounds_snapshot_supports_pick_up_transition_at_orchard`
3. Full AI pipeline still reaches remote prerequisite acquisition through decision selection, action execution, and authoritative world change -> golden care test `golden_healer_acquires_remote_ground_medicine_for_patient`
4. Materialization-barrier and production follow-on behavior still works under the same architecture -> golden production tests `golden_materialization_barrier_chain` and `golden_multi_recipe_craft_path`
5. No hidden compile-only dependency drift -> `cargo test -p worldwake-ai`, `cargo test --workspace`, and `cargo clippy --workspace`

## What to Change

### 1. Correct the ticket scope

Rewrite this ticket to describe the live state of the codebase: the call-site update is already complete, and the remaining task is verification plus archival.

### 2. Verify the delivered integration instead of re-implementing it

Confirm the live `agent_tick` call site, focused search behavior, and end-to-end golden behavior still match the prerequisite-aware planner design from [`archive/specs/S12-planner-prerequisite-aware-search.md`](../../specs/S12-planner-prerequisite-aware-search.md).

## Files to Touch

- `tickets/S12PLAPREAWA-004-agent-tick-call-site-update.md` (update, then archive)
- Code changes only if live verification exposes a real gap

## Out of Scope

- Re-implementing the already-landed `search_plan()` signature change
- Re-introducing any compatibility wrapper around static `goal_relevant_places`
- Broad planner redesign beyond the delivered dynamic per-node place computation

## Acceptance Criteria

### Tests That Must Pass

1. `cargo build -p worldwake-ai` — compilation succeeds
2. Existing suite: `cargo test -p worldwake-ai`
3. Existing suite: `cargo test --workspace`

### Invariants

1. `search_plan()` is invoked from `agent_tick` with `recipe_registry`, not a precomputed static place slice
2. `agent_tick.rs` contains no stale root-state `goal_relevant_places` computation
3. Remote prerequisite acquisition still works through the real AI pipeline, not just isolated search helpers
4. Production/materialization-barrier planning still works under the same dynamic-place architecture

## Test Plan

### New/Modified Tests

1. None expected if live coverage already proves the invariant. If verification reveals a missing layer, add the narrowest focused or golden test that exercises the regression through `agent_tick`.

### Commands

1. `cargo test -p worldwake-ai remote_treat_wounds -- --nocapture`
2. `cargo test -p worldwake-ai golden_healer_acquires_remote_ground_medicine_for_patient -- --nocapture`
3. `cargo test -p worldwake-ai golden_materialization_barrier_chain -- --nocapture`
4. `cargo test -p worldwake-ai golden_multi_recipe_craft_path -- --nocapture`
5. `cargo test -p worldwake-ai`
6. `cargo test --workspace`
7. `cargo clippy --workspace --all-targets -- -D warnings`

## Implementation Note

This ticket is no longer an atomic compile-unit follow-up. The signature change and call-site update have already landed together. The correct next step is to verify the delivered architecture, record the actual outcome, and archive the ticket rather than pretending the work is still pending.

## Outcome

- Completion date: 2026-03-21
- What actually changed:
  - Reassessed the ticket against the live `worldwake-ai` code and corrected the scope from "pending call-site update" to "verification and archival of an already-delivered planner integration"
  - Verified that `agent_tick.rs` already passes `recipe_registry` into `search_plan()` and no stale root-only `goal_relevant_places` computation remains
  - Verified that the delivered dynamic prerequisite-aware search architecture is already present in `goal_model.rs`, `search.rs`, and `budget.rs`
- Deviations from original plan:
  - No source-code implementation was needed because the call-site update and its parent architecture had already landed
  - No new tests were added because existing focused and golden coverage already proved the invariant at the search layer and through the full AI pipeline
- Verification results:
  - `cargo test -p worldwake-ai remote_treat_wounds -- --nocapture`
  - `cargo test -p worldwake-ai golden_healer_acquires_remote_ground_medicine_for_patient -- --nocapture`
  - `cargo test -p worldwake-ai golden_materialization_barrier_chain -- --nocapture`
  - `cargo test -p worldwake-ai golden_multi_recipe_craft_path -- --nocapture`
  - `cargo test -p worldwake-ai`
  - `cargo test --workspace`
  - `cargo clippy --workspace --all-targets -- -D warnings`
