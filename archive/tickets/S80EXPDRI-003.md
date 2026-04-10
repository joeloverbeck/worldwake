# S80EXPDRI-003: Goal dispatch and planner integration

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — GoalKindPlannerExt completion and planner regression coverage
**Deps**: S80EXPDRI-001

## Problem

`GoalKind::ExploreLocation` now has compile-safe inert dispatch scaffolding from `S80EXPDRI-001`, but the planner still does not handle it as a live travel goal. Until this ticket replaces the inert declaration/goal-model branches with real travel semantics, any ExploreLocation goal produced by candidate generation would remain non-functional in `search_plan()`.

## Assumption Reassessment (2026-04-10)

1. `GoalDispatchKey::ExploreLocation` already exists in `crates/worldwake-ai/src/goal_dispatch_key.rs`, is present in `GoalDispatchKey::ALL`, and already maps from `GoalKind::ExploreLocation { .. }`.
2. `GoalDispatchDeclaration` already contains `DECL_EXPLORE_LOCATION` in `crates/worldwake-ai/src/goal_dispatch_decl.rs` with `relevant_ops: &[PlannerOpKind::Travel]`, `InvalidationStrategy::NoOpinion`, `FeasibilityStrategy::NoOpinion`, and declaration-table coverage tests. No new declaration substrate is required for this ticket.
3. `GoalKindPlannerExt` in `crates/worldwake-ai/src/goal_model.rs` already has live ExploreLocation arms for `relevant_op_kinds()`, `build_payload_override()`, `apply_planner_step()`, `goal_relevant_places()`, `prerequisite_places()`, `matches_binding()`, and `candidate_is_available()`. The remaining planner-root defect is `is_satisfied()`, which still returned `false` unconditionally for ExploreLocation.
5. The spec says `EXPLORE_OPS = &[PlannerOpKind::Travel]` — exploration uses Travel only, unlike Patrol which has its own PlannerOpKind. No new PlannerOpKind variant needed. Confirmed: `PlannerOpKind::Travel` exists at `crates/worldwake-ai/src/planner_ops.rs`.
6. `GoalPriorityClass` at `crates/worldwake-ai/src/goal_model.rs` has variants: `Background`, `Low`, `Medium`, `High`, `Critical`. ExploreLocation uses `Low`.
7. `S80EXPDRI-001` already landed more than compile-safe substrate: the declaration table and most planner hooks are live enough for travel-only planning. This ticket should not re-add or churn those pieces.
8. The practical failure mode is narrower than the original ticket text claimed: because `GoalKind::ExploreLocation` never became satisfied after hypothetical travel, `search_plan()` could not terminate with `GoalSatisfied` for the exploration destination.

## Architecture Check

1. Follows the established pattern for every GoalKind variant: dispatch key + declaration + planner ext. ExploreLocation is simpler than most goals (Travel-only ops, no payload override, no commodity tracking). The `is_satisfied` check is a straightforward place comparison, matching the pattern used by Patrol.
2. No backward-compatibility shims. New dispatch/planner paths only — existing goals unaffected.

## Verification Layers

1. Existing declaration-table tests continue to prove `GoalDispatchKey` / `GoalDispatchDeclaration` coverage for ExploreLocation without new substrate changes.
2. `GoalKindPlannerExt::is_satisfied` returns true when at `target_place` → focused unit test.
3. `GoalKindPlannerExt::goal_relevant_places` and `matches_binding` continue to point at `target_place` → focused unit test.
4. `search_plan` finds a travel plan for ExploreLocation and terminates after arrival → planner integration test.
6. Single-crate ticket (worldwake-ai); cross-system mapping not applicable beyond planner integration.

## What to Change

### 1. Complete the remaining GoalKindPlannerExt ExploreLocation behavior

In `crates/worldwake-ai/src/goal_model.rs`, complete the remaining live ExploreLocation behavior in the `impl GoalKindPlannerExt for GoalKind` block:

- `is_satisfied()` → true when agent's effective place == `target_place`
- Keep the already-landed travel-only hooks (`apply_planner_step()`, `goal_relevant_places()`, `matches_binding()`, etc.) intact.

### 2. Add regressions around planner termination

- Expand the exhaustive GoalKind coverage tests so ExploreLocation is included in the `goal_relevant_places()` / `prerequisite_places()` variant inventories.
- Add focused GoalModel tests proving ExploreLocation satisfaction and target binding behavior.
- Add a search integration test proving `search_plan()` returns a single-step Travel plan for ExploreLocation once arrival can satisfy the goal.

## Files to Touch

- `crates/worldwake-ai/src/goal_model.rs` (modify — make ExploreLocation satisfy on arrival and add focused GoalKind coverage)
- `crates/worldwake-ai/src/search/tests.rs` (modify — prove planner returns Travel plan for ExploreLocation)

## Out of Scope

- Candidate generation emitter (ticket 004)
- Ranking motive score computation beyond the planner-visible minimum contract (ticket 004)
- Counter management for consecutive explorations (ticket 004)
- Golden E2E tests (ticket 005)
- New PlannerOpKind variant (not needed — reuses Travel)

## Acceptance Criteria

### Tests That Must Pass

1. `GoalKindPlannerExt::is_satisfied` returns true when at `target_place`, false otherwise.
2. `GoalKindPlannerExt::goal_relevant_places` returns `[target_place]` and binding continues to match travel to that place.
3. `search_plan` returns `Found` for an ExploreLocation goal with a reachable target and terminates after the travel step.
5. Existing suite: `cargo test -p worldwake-ai`
6. Existing suite: `cargo clippy --workspace --all-targets -- -D warnings`

### Invariants

1. No new `PlannerOpKind` variant — ExploreLocation reuses Travel only.
2. `GoalDispatchKey::ExploreLocation` remains the canonical dispatch identity introduced by `S80EXPDRI-001`.
3. No duplicate declaration substrate is introduced; this ticket only finishes the remaining planner behavior gap and its regressions.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/goal_model.rs` (test module) — ExploreLocation satisfaction, goal-relevant places, and binding.
2. `crates/worldwake-ai/src/search/tests.rs` — `search_plan` finds the travel plan for ExploreLocation.

### Commands

1. `cargo test -p worldwake-ai`
2. `cargo clippy --workspace --all-targets -- -D warnings`
3. `cargo build --workspace`

## Outcome

- **Completion date**: 2026-04-10
- **What changed**:
  - Completed the remaining `GoalKind::ExploreLocation` planner behavior by making `is_satisfied()` return true when the planning-state actor reaches `target_place`.
  - Added focused `worldwake-ai` regressions covering ExploreLocation satisfaction, goal-relevant-place coverage, and `search_plan()` termination on the travel step.
  - Updated the stale `GoalDispatchKey::ALL` length assertion so full-crate verification matched the already-landed `ExploreLocation` dispatch key substrate.
- **Deviations from original plan**:
  - Reassessment showed that `GoalDispatchKey::ExploreLocation`, the dispatch declaration, and most `GoalKindPlannerExt` hooks were already live. This ticket did not re-add declaration substrate; it narrowed to the remaining planner-satisfaction gap and its regressions.
- **Verification results**:
  - `cargo test -p worldwake-ai explore_location`
  - `cargo test -p worldwake-ai`
  - `cargo build --workspace`
  - `cargo clippy --workspace --all-targets -- -D warnings`
