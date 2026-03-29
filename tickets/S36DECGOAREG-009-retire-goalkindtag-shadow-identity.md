# S36DECGOAREG-009: Retire unused `GoalKindTag` shadow identity from AI dispatch

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes — `worldwake-ai` planner semantics metadata, goal-model trait surface, test scaffolding
**Deps**: S36DECGOAREG-006, S36DECGOAREG-007

## Problem

`GoalKindTag` is now a coarse shadow identity in `worldwake-ai`, not the real AI dispatch contract. Live runtime dispatch already routes through `GoalDispatchKey` declarations for relevant ops, and S36 is migrating invalidation/feasibility to declarations as well. The remaining `GoalKindTag` footprint is concentrated in `PlannerOpSemantics.relevant_goal_kinds` plus `GoalKindPlannerExt::goal_kind_tag()`, but those surfaces are metadata/test scaffolding rather than active runtime decision boundaries. Keeping them around as production API leaves a stale, coarser second identity in the AI crate after S36 has already established `GoalDispatchKey` as the real declaration substrate.

## Assumption Reassessment (2026-03-29)

1. `specs/S36-declarative-goal-registration.md` explicitly says `GoalKindTag` may survive only where a deliberately coarse family contract is still the actual contract, and must not remain as a competing declaration identity. That is the governing architectural standard for this follow-up.
2. The live shared abstraction boundary under audit is `GoalDispatchKey` / `GoalDispatchDeclaration` versus the residual coarse `GoalKindTag` surfaces in `crates/worldwake-ai/src/goal_model.rs` and `crates/worldwake-ai/src/planner_ops.rs`.
3. `GoalKindPlannerExt::relevant_op_kinds()` in `crates/worldwake-ai/src/goal_model.rs` already routes through `GoalDispatchKey::from_goal_kind(self).declaration().relevant_ops`. The active search path therefore already uses declarations rather than `GoalKindTag`.
4. `PlannerOpSemantics.relevant_goal_kinds` in `crates/worldwake-ai/src/planner_ops.rs` is not consumed by the live runtime search path. Repository-wide search shows production reads only inside `planner_ops.rs`; external uses are test scaffolds in `goal_model.rs` and `search/tests.rs`.
5. `GoalKindPlannerExt::goal_kind_tag()` and the `GoalKindTag` enum in `crates/worldwake-ai/src/goal_model.rs` are not used by production code outside their own definition site. Repository-wide search shows only local tests call `.goal_kind_tag()`.
6. Ticket 004 already removed the manual planner-op reverse matrix and replaced it with a declaration-derived reverse map. That reverse map still collapses `GoalDispatchKey` back to `GoalKindTag`, which is now the last production place where planner metadata depends on the coarse shadow identity.
7. Because `PlannerOpSemantics.relevant_goal_kinds` is metadata-only today, the cleanest architecture is not to replace it with `&'static [GoalDispatchKey]`; it is to remove the field entirely unless reassessment during implementation finds a real production consumer that truly needs planner-op reverse membership.
8. If implementation discovers a real production consumer that needs planner-op reverse membership after all, the contract should be `GoalDispatchKey`, not `GoalKindTag`, because `AcquireSelfConsume` / `AcquireRecipeInput` / `AcquireRestock` and `PunishFine` / `PunishExile` are dispatch-distinct families under S36.
9. `cargo test -p worldwake-ai -- --list` confirms existing focused tests that will need updating are real and current: `goal_model::tests::goal_kind_tag_tracks_goal_families_without_payload_identity`, `planner_ops::tests::derived_reverse_membership_matches_expected_goal_tags`, `planner_ops::tests::derived_reverse_membership_covers_declared_ops_and_intentional_empties`, plus multiple `planner_ops` and `search/tests.rs` scaffolds that construct `PlannerOpSemantics`.
10. Adjacent contradiction classification: this is not a separate bug uncovered during reassessment; it is a required architectural consequence of S36’s declaration-key migration. Leaving the coarse shadow identity in place would directly conflict with the spec’s stated `GoalKindTag` survival rule.

## Architecture Check

1. Removing the unused coarse shadow identity is cleaner than migrating more code to it or preserving it as metadata. The authoritative AI dispatch contract is already `GoalDispatchKey` plus declaration metadata; deleting the leftover coarse layer restores a single explicit identity path and matches P25/P26 in `docs/FOUNDATIONS.md`.
2. If a planner-op reverse surface is still needed after reassessment, it should be rebuilt on `GoalDispatchKey` because that is the dispatch-distinguishing identity. Reintroducing or preserving `GoalKindTag` here would re-create the same shadow-contract problem under a new name.
3. No backwards-compatibility shims or aliasing: remove `GoalKindTag`/`goal_kind_tag()`/`relevant_goal_kinds` outright where they are no longer the real contract, and update all affected tests/scaffolds in the same change.

## Verification Layers

1. Declaration-key singularity in production code -> focused grep-backed unit coverage and compile success after removing `GoalKindTag`-based planner metadata/types
2. Planner semantics behavior preservation -> focused `planner_ops` unit tests over classification, barrier flags, transition kinds, and derived metadata that still matters after field removal
3. Search/runtime behavior preservation -> `cargo test -p worldwake-ai`
4. Workspace-level regression guard -> `cargo test --workspace`
5. Dead API / lint cleanliness -> `cargo clippy --workspace`
6. Single-crate architectural cleanup ticket: no additional action-trace or event-log layer mapping is needed because the intended change is removal of unused AI-internal metadata, not a behavioral runtime contract change

## What to Change

### 1. Reassess whether planner-op reverse membership still needs to exist at all

Audit `PlannerOpSemantics.relevant_goal_kinds` and its callers. If there is still no production consumer after 006/007 land, remove the field from `PlannerOpSemantics` and delete the reverse-membership builder entirely.

If reassessment finds a real production consumer that still needs reverse membership, keep the smallest possible surface and migrate it to `GoalDispatchKey`, not `GoalKindTag`.

### 2. Remove `GoalKindTag` from `goal_model.rs` if no real production contract remains

Delete:
- the `GoalKindTag` enum
- `GoalKindPlannerExt::goal_kind_tag()`
- tests that only prove the coarse payload-collapsing behavior of that removed identity

If any legitimate coarse-family contract remains after reassessment, move that contract to the narrowest appropriate rendering/debug surface rather than leaving it in the planning trait.

### 3. Update planner and search test scaffolding

Remove `relevant_goal_kinds` initialization from test-only `PlannerOpSemantics` builders in `planner_ops.rs`, `goal_model.rs`, and `search/tests.rs`, or replace it with the new narrower surface if reassessment proves one is needed.

### 4. Replace coarse-tag-focused tests with declaration-key-focused or behavior-focused tests

Where current tests exist only to prove `GoalKindTag` collapsing, replace them with tests that assert the real dispatch contract:
- declaration-key splits/collapses where payload matters
- planner semantics classification/transition behavior
- any remaining reverse-membership behavior only if a real production consumer still exists

## Files to Touch

- `crates/worldwake-ai/src/goal_model.rs` (modify — remove `GoalKindTag` / `goal_kind_tag()` if unused, update tests)
- `crates/worldwake-ai/src/planner_ops.rs` (modify — remove `relevant_goal_kinds` and reverse-membership scaffolding if unused, or migrate any necessary residual contract to `GoalDispatchKey`)
- `crates/worldwake-ai/src/search/tests.rs` (modify — update `PlannerOpSemantics` test scaffolds)
- `crates/worldwake-ai/src/lib.rs` (modify — stop re-exporting removed coarse identity/types if deleted)

## Out of Scope

- Changing `worldwake_core::GoalKind`
- Reintroducing a new coarse alias identity in place of `GoalKindTag`
- Candidate-generation refactors
- Trace label rendering changes already covered by ticket 005
- Invalidation and feasibility strategy migration already covered by tickets 006–007

## Acceptance Criteria

### Tests That Must Pass

1. Focused tests prove the live dispatch contract is `GoalDispatchKey`/declarations rather than `GoalKindTag`.
2. Focused tests covering planner semantics still pass with the coarse planner metadata removed or narrowed to the real consumer contract.
3. Existing suite: `cargo test -p worldwake-ai`
4. Full workspace: `cargo test --workspace`
5. Lint: `cargo clippy --workspace`

### Invariants

1. No production AI dispatch surface in `worldwake-ai` keeps `GoalKindTag` as a competing shadow identity once the ticket lands, unless reassessment proves one remaining coarse rendering/debug contract that is intentionally not a dispatch substrate.
2. No behavioral runtime regression: search, planning, and decision behavior remain driven by declarations and concrete goal state exactly as before.
3. No backwards-compatibility aliasing: removed coarse surfaces are deleted rather than wrapped.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/planner_ops.rs` — update planner-semantics tests to assert remaining real metadata/behavior after removing coarse goal-tag scaffolding.
2. `crates/worldwake-ai/src/goal_model.rs` — replace `goal_kind_tag_*` coverage with declaration-key-focused coverage where the real dispatch identity matters.
3. `crates/worldwake-ai/src/search/tests.rs` — update local `PlannerOpSemantics` builders after field removal/narrowing.

### Commands

1. `cargo test -p worldwake-ai goal_dispatch_key`
2. `cargo test -p worldwake-ai planner_ops`
3. `cargo test -p worldwake-ai`
4. `cargo test --workspace`
5. `cargo clippy --workspace`
