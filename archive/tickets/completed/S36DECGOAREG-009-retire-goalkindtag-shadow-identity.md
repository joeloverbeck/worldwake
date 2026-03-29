# S36DECGOAREG-009: Retire unused `GoalKindTag` shadow identity from AI dispatch

**Status**: ✅ COMPLETED
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes — `worldwake-ai` planner semantics metadata, goal-model trait surface, test scaffolding
**Deps**: S36DECGOAREG-006, S36DECGOAREG-007

## Problem

`GoalKindTag` is now a coarse shadow identity in `worldwake-ai`, not the real AI dispatch contract. Live runtime dispatch already routes through `GoalDispatchKey` declarations for relevant ops, and S36 is migrating invalidation/feasibility to declarations as well. The remaining `GoalKindTag` footprint is concentrated in `PlannerOpSemantics.relevant_goal_kinds` plus `GoalKindPlannerExt::goal_kind_tag()`, but those surfaces are metadata/test scaffolding rather than active runtime decision boundaries. Keeping them around as production API leaves a stale, coarser second identity in the AI crate after S36 has already established `GoalDispatchKey` as the real declaration substrate.

## Assumption Reassessment (2026-03-29)

1. `specs/S36-declarative-goal-registration.md` explicitly says `GoalKindTag` may survive only where a deliberately coarse family contract is still the real contract, and must not remain as a competing declaration identity. That is the governing standard for this follow-up.
2. The exact AI-internal abstraction boundary under audit is `GoalDispatchKey` / `GoalDispatchDeclaration` versus the residual coarse surfaces `GoalKindTag`, `GoalKindPlannerExt::goal_kind_tag()`, and `PlannerOpSemantics.relevant_goal_kinds` in [`crates/worldwake-ai/src/goal_model.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/goal_model.rs) and [`crates/worldwake-ai/src/planner_ops.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/planner_ops.rs).
3. The live planner/search contract already routes through declarations. `GoalKindPlannerExt::relevant_op_kinds()` in [`crates/worldwake-ai/src/goal_model.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/goal_model.rs) delegates to `GoalDispatchKey::from_goal_kind(self).declaration().relevant_ops`, and `feasibility.rs`, `exhaustion.rs`, and `decision_trace.rs` also dispatch through `GoalDispatchKey` declarations rather than `GoalKindTag`.
4. `PlannerOpSemantics.relevant_goal_kinds` is not part of live planner execution. Repository-wide search shows reads only inside [`crates/worldwake-ai/src/planner_ops.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/planner_ops.rs) tests plus test scaffolds in [`crates/worldwake-ai/src/goal_model.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/goal_model.rs) and [`crates/worldwake-ai/src/search/tests.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/search/tests.rs). No production caller branches on that field.
5. `GoalKindTag` and `GoalKindPlannerExt::goal_kind_tag()` are likewise dead outside local tests. Repository-wide search shows `.goal_kind_tag()` only in [`crates/worldwake-ai/src/goal_model.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/goal_model.rs) test coverage.
6. The remaining declaration-key contract is already exercised by focused tests in [`crates/worldwake-ai/src/goal_dispatch_decl.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/goal_dispatch_decl.rs), especially `test_declaration_relevant_ops_match_live_goal_model`, `test_trace_labels_nonempty_and_distinct_for_payload_splits`, `test_invalidation_strategies_match_payload_sensitive_and_shared_families`, and `test_feasibility_strategies_match_payload_sensitive_and_shared_families`. The ticket’s original test emphasis on coarse-tag reverse membership is therefore stale.
7. The declaration-derived reverse-membership helper in `planner_ops.rs` is not a production dependency either; it exists only to populate `PlannerOpSemantics.relevant_goal_kinds` and to support tests that assert the removed coarse view. Because no runtime consumer needs reverse membership today, the clean architecture is to delete the reverse-membership builder instead of migrating it to `GoalDispatchKey`.
8. If reassessment during implementation discovered a new real reverse-membership consumer, the correct identity would be `GoalDispatchKey`, not `GoalKindTag`, because `AcquireSelfConsume` / `AcquireRecipeInput` / `AcquireRestock` and `PunishFine` / `PunishExile` are intentionally dispatch-distinct under S36. Current code search found no such consumer.
9. `cargo test -p worldwake-ai -- --list` confirms the currently live focused tests affected by this cleanup are real: `goal_model::tests::goal_kind_tag_tracks_goal_families_without_payload_identity`, `planner_ops::tests::derived_reverse_membership_matches_expected_goal_tags`, `planner_ops::tests::derived_reverse_membership_covers_declared_ops_and_intentional_empties`, plus planner/search scaffolds constructing `PlannerOpSemantics` with the stale `relevant_goal_kinds` field.
10. Adjacent contradiction classification: this is a required architectural consequence of S36’s declaration-key migration, not a separate unrelated bug. Leaving the coarse identity in place would violate the spec’s stated `GoalKindTag` survival rule and preserve a second dead contract surface.

## Architecture Check

1. Removing the unused coarse shadow identity is cleaner than preserving it as metadata or translating it to a new alias. The authoritative AI dispatch contract is already `GoalDispatchKey` plus `GoalDispatchDeclaration`; deleting the leftover coarse layer restores one explicit identity path and matches P25/P26 in `docs/FOUNDATIONS.md`.
2. Keeping reverse membership purely for tests would fossilize a second architectural story after S36 has already centralized dispatch declarations. Existing `goal_dispatch_decl.rs` tests already prove the live declaration contract directly, so deleting the reverse-membership scaffolding is cleaner than re-keying it.
3. No backwards-compatibility shims or aliasing: remove `GoalKindTag`, `goal_kind_tag()`, `relevant_goal_kinds`, and the declaration-to-coarse reverse map outright, then update focused tests to assert the real declaration-key contract or the remaining planner semantics behavior.

## Verification Layers

1. Dispatch declaration remains the only live static routing contract -> focused `goal_dispatch_decl` and `goal_dispatch_key` unit coverage plus compile success after removing `GoalKindTag` surfaces
2. Planner semantics behavior preservation -> focused `planner_ops` unit tests over classification, barrier flags, leaf/mid-plan semantics, and transition kinds after field removal
3. Search/runtime behavior preservation -> `cargo test -p worldwake-ai`
4. Workspace-level regression guard -> `cargo test --workspace`
5. Dead API / lint cleanliness -> `cargo clippy --workspace`
6. Single-crate architectural cleanup ticket: no action-trace or event-log proof surface is needed because the intended change is removal of unused AI-internal metadata, not a runtime ordering or authoritative-state mutation change

## What to Change

### 1. Remove planner-op reverse membership and coarse planner metadata

Delete `PlannerOpSemantics.relevant_goal_kinds`, `goal_tag_for_dispatch_key`, `derived_relevant_goal_kinds_by_op`, and `derived_relevant_goal_kinds` from [`crates/worldwake-ai/src/planner_ops.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/planner_ops.rs), because reassessment found no production consumer for that reverse view.

### 2. Remove dead `GoalKindTag` identity from `goal_model.rs`

Delete from [`crates/worldwake-ai/src/goal_model.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/goal_model.rs):
- the `GoalKindTag` enum
- `GoalKindPlannerExt::goal_kind_tag()`
- tests that only prove the removed payload-collapsing identity

No new coarse alias should replace it.

### 3. Update planner and search test scaffolding

Remove `relevant_goal_kinds` initialization from test-only `PlannerOpSemantics` builders in:
- [`crates/worldwake-ai/src/planner_ops.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/planner_ops.rs)
- [`crates/worldwake-ai/src/goal_model.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/goal_model.rs)
- [`crates/worldwake-ai/src/search/tests.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/search/tests.rs)

### 4. Recenter focused tests on the real contract

Remove planner-op tests that only assert the deleted coarse reverse-membership view. Keep or add focused tests that assert:
- declaration-key payload-sensitive splits remain covered in `goal_dispatch_key.rs`
- declaration metadata matches live goal-model `relevant_op_kinds()` in `goal_dispatch_decl.rs`
- planner semantics classification/transition behavior still holds after the metadata field disappears

## Files to Touch

- `crates/worldwake-ai/src/goal_model.rs` (modify — remove `GoalKindTag` / `goal_kind_tag()` if unused, update tests)
- `crates/worldwake-ai/src/planner_ops.rs` (modify — remove `relevant_goal_kinds` and reverse-membership scaffolding)
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
2. Focused tests covering planner semantics still pass with the coarse planner metadata removed entirely.
3. Existing suite: `cargo test -p worldwake-ai`
4. Full workspace: `cargo test --workspace`
5. Lint: `cargo clippy --workspace`

### Invariants

1. No production AI dispatch surface in `worldwake-ai` keeps `GoalKindTag` as a competing shadow identity once the ticket lands.
2. No behavioral runtime regression: search, planning, and decision behavior remain driven by declarations and concrete goal state exactly as before.
3. No backwards-compatibility aliasing: removed coarse surfaces are deleted rather than wrapped.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/planner_ops.rs` — update planner-semantics tests to assert remaining real metadata/behavior after removing coarse goal-tag scaffolding.
2. `crates/worldwake-ai/src/goal_model.rs` — delete `goal_kind_tag_*` coverage and update local `PlannerOpSemantics` builders.
3. `crates/worldwake-ai/src/goal_dispatch_decl.rs` — retain declaration-key-focused coverage as the focused proof surface for static dispatch contract parity.
4. `crates/worldwake-ai/src/search/tests.rs` — update local `PlannerOpSemantics` builders after field removal.

### Commands

1. `cargo test -p worldwake-ai goal_dispatch_key`
2. `cargo test -p worldwake-ai goal_dispatch_decl`
3. `cargo test -p worldwake-ai planner_ops`
4. `cargo test -p worldwake-ai`
5. `cargo test --workspace`
6. `cargo clippy --workspace`

## Outcome

Completed: 2026-03-29

What actually changed:
- Removed `GoalKindTag` and `GoalKindPlannerExt::goal_kind_tag()` from `worldwake-ai`; `GoalDispatchKey` / `GoalDispatchDeclaration` are now the only AI dispatch identity surfaces.
- Removed `PlannerOpSemantics.relevant_goal_kinds` plus the declaration-to-coarse reverse-membership builder in `planner_ops.rs`.
- Updated planner/search/golden/conformance scaffolding to stop constructing or reading the removed coarse metadata.
- Kept focused declaration-contract proof on existing `goal_dispatch_decl.rs` and `goal_dispatch_key.rs` coverage instead of replacing the deleted coarse-tag tests with another alias layer.
- Narrowly adjusted a few helper signatures to pass `PlannerOpSemantics` by value after the struct became trivially copyable enough for `clippy::trivially_copy_pass_by_ref`.

Deviations from original plan:
- The ticket originally left open the possibility of migrating reverse membership to `GoalDispatchKey`. Reassessment showed no production consumer exists, so the reverse-membership surface was deleted entirely rather than re-keyed.
- No new declaration-key test module was needed; the existing focused declaration tests already covered the real static dispatch contract.

Verification results:
- `cargo test -p worldwake-ai goal_dispatch_key`
- `cargo test -p worldwake-ai goal_dispatch_decl`
- `cargo test -p worldwake-ai planner_ops`
- `cargo test -p worldwake-ai goal_model`
- `cargo test -p worldwake-ai search::tests`
- `cargo test -p worldwake-ai`
- `cargo test --workspace`
- `cargo clippy --workspace`
