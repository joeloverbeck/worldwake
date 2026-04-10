# S80EXPDRI-003: Goal dispatch and planner integration

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — new GoalDispatchKey variant, dispatch declaration, GoalKindPlannerExt implementation
**Deps**: S80EXPDRI-001

## Problem

`GoalKind::ExploreLocation` now has compile-safe inert dispatch scaffolding from `S80EXPDRI-001`, but the planner still does not handle it as a live travel goal. Until this ticket replaces the inert declaration/goal-model branches with real travel semantics, any ExploreLocation goal produced by candidate generation would remain non-functional in `search_plan()`.

## Assumption Reassessment (2026-04-10)

1. `GoalDispatchKey` at `crates/worldwake-ai/src/goal_dispatch_key.rs:6` has 38 variants. `ALL` constant is `[Self; 38]`. `from_goal_kind` is a const fn match. Adding a variant requires updating all three.
2. `GoalDispatchDeclaration` entries in `crates/worldwake-ai/src/goal_dispatch_decl.rs` follow the pattern: const op array, const barrier array, match arm returning declaration struct. Patrol pattern at line 164: `PATROL_OPS = &[PlannerOpKind::Travel, PlannerOpKind::Patrol]`.
3. `GoalKindPlannerExt` trait at `crates/worldwake-ai/src/goal_model.rs:38` has 11 methods. `impl GoalKindPlannerExt for GoalKind` at line 448 uses a match on self for each method. Adding ExploreLocation requires match arms in all 11 methods.
5. The spec says `EXPLORE_OPS = &[PlannerOpKind::Travel]` — exploration uses Travel only, unlike Patrol which has its own PlannerOpKind. No new PlannerOpKind variant needed. Confirmed: `PlannerOpKind::Travel` exists at `crates/worldwake-ai/src/planner_ops.rs`.
6. `GoalPriorityClass` at `crates/worldwake-ai/src/goal_model.rs` has variants: `Background`, `Low`, `Medium`, `High`, `Critical`. ExploreLocation uses `Low`.
7. `InvalidationStrategy` and `FeasibilityStrategy` enums exist in `crates/worldwake-ai/src/goal_dispatch_decl.rs`. Need to select appropriate variants for ExploreLocation.
8. `S80EXPDRI-001` already landed the additive substrate needed to keep the new goal kind compile-safe: `GoalDispatchKey::ExploreLocation`, a placeholder declaration using `InvalidationStrategy::NoOpinion` / `FeasibilityStrategy::NoOpinion`, and inert `GoalKindPlannerExt` / ranking branches. This ticket now owns replacing those inert branches with the live planner contract rather than re-adding the symbol.

## Architecture Check

1. Follows the established pattern for every GoalKind variant: dispatch key + declaration + planner ext. ExploreLocation is simpler than most goals (Travel-only ops, no payload override, no commodity tracking). The `is_satisfied` check is a straightforward place comparison, matching the pattern used by Patrol.
2. No backward-compatibility shims. New dispatch/planner paths only — existing goals unaffected.

## Verification Layers

1. GoalDispatchKey::from_goal_kind maps ExploreLocation correctly → focused unit test
2. GoalDispatchDeclaration returns correct ops/barrier/invalidation → focused unit test
3. GoalKindPlannerExt::is_satisfied returns true when at target_place → focused unit test
4. GoalKindPlannerExt::goal_relevant_places returns target_place → focused unit test
5. search_plan finds a travel plan for ExploreLocation → planner integration test (decision trace)
6. Single-crate ticket (worldwake-ai); cross-system mapping not applicable beyond planner integration.

## What to Change

### 1. Replace inert GoalDispatchDeclaration scaffolding with live travel semantics

In `crates/worldwake-ai/src/goal_dispatch_decl.rs`:
- Replace the placeholder ExploreLocation declaration with the live dispatch contract:
  - `relevant_ops: &[PlannerOpKind::Travel]`
  - progress barrier matching terminal travel completion to `target_place`
  - appropriate invalidation/feasibility behavior for a belief-gated travel goal
  - trace label and tests aligned with the final live contract
- Keep the existing `GoalDispatchKey::ExploreLocation` mapping added by `S80EXPDRI-001`.

### 2. Replace inert GoalKindPlannerExt handling for ExploreLocation

In `crates/worldwake-ai/src/goal_model.rs`, add match arms in the `impl GoalKindPlannerExt for GoalKind` block:

- `ranked_goal_provenance_family()` → `None` initially (add `Exploration` variant to `RankedGoalProvenanceFamily` only if provenance tracking is now needed)
- `relevant_op_kinds()` → `EXPLORE_OPS`
- `relevant_observed_commodities()` → `None`
- `build_payload_override()` → `Ok(None)`
- `apply_planner_step()` → simulate travel to `target_place` (move agent's effective place in planning state)
- `is_progress_barrier()` → true when step op is Travel and target contains `target_place`
- `is_satisfied()` → true when agent's effective place == `target_place`
- `goal_relevant_places()` → `vec![target_place]`
- `prerequisite_places()` → empty (target_place IS the destination, not a prerequisite)
- `matches_binding()` → true when targets contain `target_place` and op is Travel
- `candidate_is_available()` → true

## Files to Touch

- `crates/worldwake-ai/src/goal_dispatch_decl.rs` (modify — replace inert ExploreLocation declaration with live travel semantics)
- `crates/worldwake-ai/src/goal_model.rs` (modify — replace inert ExploreLocation planner arms with live search/satisfaction behavior)

## Out of Scope

- Candidate generation emitter (ticket 004)
- Ranking motive score computation beyond the planner-visible minimum contract (ticket 004)
- Counter management for consecutive explorations (ticket 004)
- Golden E2E tests (ticket 005)
- New PlannerOpKind variant (not needed — reuses Travel)

## Acceptance Criteria

### Tests That Must Pass

1. ExploreLocation declaration uses the intended travel-only planner contract rather than the inert placeholder shipped in `S80EXPDRI-001`
2. `GoalKindPlannerExt::is_satisfied` returns true when at target_place, false otherwise
3. `GoalKindPlannerExt::goal_relevant_places` returns `[target_place]`
4. `search_plan` returns `Found` for an ExploreLocation goal with a reachable target
5. Existing suite: `cargo test -p worldwake-ai`
6. Existing suite: `cargo clippy --workspace --all-targets -- -D warnings`

### Invariants

1. No new PlannerOpKind variant — ExploreLocation reuses Travel only
2. `GoalDispatchKey::ExploreLocation` remains the canonical dispatch identity introduced by `S80EXPDRI-001`
3. Every GoalKindPlannerExt method has an ExploreLocation arm with live semantics (no wildcard fallthrough, no inert placeholder left behind)

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/goal_dispatch_decl.rs` (test module) — ExploreLocation declaration is live and travel-only
2. `crates/worldwake-ai/src/goal_model.rs` (test module) — is_satisfied, goal_relevant_places, matches_binding for ExploreLocation
3. `crates/worldwake-ai/src/search/` (test module) — search_plan finds travel plan for ExploreLocation

### Commands

1. `cargo test -p worldwake-ai`
2. `cargo clippy --workspace --all-targets -- -D warnings`
3. `cargo build --workspace`
