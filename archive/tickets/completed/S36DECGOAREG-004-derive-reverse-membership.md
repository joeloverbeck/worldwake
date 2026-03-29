# S36DECGOAREG-004: Derive planner-op reverse membership from declarations

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: `worldwake-ai` planner-op semantics derivation
**Deps**: S36DECGOAREG-002

## Problem

`crates/worldwake-ai/src/planner_ops.rs` still maintains a manually curated reverse matrix from `PlannerOpKind` to coarse `GoalKindTag` slices (`GOALS_*`). But the forward mapping already lives in S36 declarations: `GoalDispatchKey::declaration().relevant_ops` in `crates/worldwake-ai/src/goal_dispatch_decl.rs`, and `GoalKindPlannerExt::relevant_op_kinds()` already routes through that declaration path in `crates/worldwake-ai/src/goal_model.rs`. This leaves one fact with two production transport paths:

1. canonical forward declaration path: `GoalDispatchKey -> GoalDispatchDeclaration.relevant_ops`
2. duplicate reverse planner-op path: manual `GOALS_*` arrays in `planner_ops.rs`

That duplicate reverse path is now the architectural contradiction. It should be derived from the declaration table instead of maintained by hand.

## Assumption Reassessment (2026-03-29)

Shared abstraction boundary under audit: `GoalDispatchDeclaration.relevant_ops` in `crates/worldwake-ai/src/goal_dispatch_decl.rs` versus `PlannerOpSemantics.relevant_goal_kinds` in `crates/worldwake-ai/src/planner_ops.rs`.

1. The ticket’s original “add `GoalDispatchKey::goal_kind_tag()`” assumption is stale. `GoalDispatchKey` already exists in `crates/worldwake-ai/src/goal_dispatch_key.rs`, and S36 declaration lookups already exist in `crates/worldwake-ai/src/goal_dispatch_decl.rs`.
2. The ticket’s original “add `GoalDispatchKey::all_keys()`” assumption is also stale as written. There is already an exhaustive `ALL_KEYS` list in `goal_dispatch_decl.rs` tests, proving the key set is known today. The real gap is that production code does not yet expose a canonical exhaustive iterator/slice for reverse derivation.
3. `GoalKindPlannerExt::relevant_op_kinds()` already routes through `GoalDispatchKey::from_goal_kind(self).declaration().relevant_ops` in `crates/worldwake-ai/src/goal_model.rs`. So this ticket is no longer introducing declaration-backed relevant-op dispatch; it is removing the remaining duplicate reverse matrix in `planner_ops.rs`.
4. `PlannerOpSemantics.relevant_goal_kinds` is still `&'static [GoalKindTag]` in `crates/worldwake-ai/src/planner_ops.rs`. The live contract is therefore still coarse on the planner-op side. Reverse derivation must intentionally collapse payload-sensitive declaration keys back to coarse `GoalKindTag` values.
5. `planner_ops.rs` currently maintains 21 manual `GOALS_*` constants and wires them through `semantics_for()` / `social_or_combat_semantics()`. Those constants are production data, not test-only scaffolding, so the ticket cannot remain marked `Engine Changes: None`.
6. The forward declaration data is not one-to-one with coarse tags. `AcquireSelfConsume`, `AcquireRecipeInput`, and `AcquireRestock` all collapse to `GoalKindTag::AcquireCommodity`; `PunishFine` and `PunishExile` collapse to `GoalKindTag::PunishAccused`. The reverse derivation therefore needs deterministic deduplication by coarse tag.
7. `PlannerOpSemantics.relevant_goal_kinds` is not consumed by the live search pipeline today; runtime candidate filtering uses `GoalKindPlannerExt::relevant_op_kinds()` in `crates/worldwake-ai/src/goal_model.rs`, which already reads declaration `relevant_ops`. This makes the planner-op reverse table metadata-only at present, but still worth deriving because duplicate static facts are architectural debt and future consumers would otherwise inherit drift.
8. The manual reverse matrix is not behaviorally equivalent to the live declaration table. At least one old membership (`PlannerOpKind::Travel -> GoalKindTag::EngageHostile`) is stale relative to declarations, and the manual table also omits several declaration-backed travel memberships. Because the reverse field is metadata-only today, aligning it to declarations is the cleaner architecture and does not change search behavior.
9. `cargo test -p worldwake-ai -- --list` confirms the current focused planner-op tests live in `crates/worldwake-ai/src/planner_ops.rs`. A single substring filter such as `reverse_membership` is the simplest copy-paste runnable command for the two new focused tests.

## Architecture Check

1. Deriving reverse membership is cleaner than the current architecture because it restores one canonical static fact path: declarations own forward membership, planner semantics consume an inverted view of that same data. This removes the last duplicated matrix without introducing a compatibility layer.
2. A production `GoalDispatchKey::all()` or equivalent canonical exhaustive slice is more robust than leaving the only exhaustive key list inside tests. It gives declaration-derived features one stable enumeration surface and makes future S36 work easier to compose.
3. The coarse `GoalKindTag` contract in `PlannerOpSemantics` is still acceptable for this ticket because planner-op search filtering is intentionally tag-based today. Forcing `PlannerOpSemantics` to become dispatch-key-granular here would expand scope into a broader architectural migration better handled by a separate ticket.
4. The ideal long-term architecture is still to eliminate coarse shadow identities where they are no longer the real contract. This ticket should not silently widen into that migration, but the remaining coarse `PlannerOpSemantics.relevant_goal_kinds` contract is worth calling out as the next architectural pressure point once reverse derivation lands.

## Verification Layers

1. declaration completeness / reverse-membership correctness -> focused planner-op unit tests over `PlannerOpKind`, `GoalDispatchKey`, and `GoalKindTag`
2. runtime search behavior stability despite metadata refactor -> `cargo test -p worldwake-ai`
3. broader workspace stability -> `cargo test --workspace`
4. static data hygiene / lint regressions -> `cargo clippy --workspace`

## What to Change

### 1. Expose canonical exhaustive dispatch-key enumeration in production code

Add a production `GoalDispatchKey::all()` or equivalent canonical exhaustive slice/iterator in `crates/worldwake-ai/src/goal_dispatch_key.rs` or `goal_dispatch_decl.rs`. The reverse derivation must not depend on a test-only `ALL_KEYS` list.

### 2. Add a coarse-tag collapse helper at the correct boundary

Add the smallest production helper needed to map `GoalDispatchKey` back to coarse `GoalKindTag` for planner semantics. This can be `GoalDispatchKey::goal_kind_tag()` or an equivalent function colocated with the derivation logic, but it must be canonical rather than reimplemented ad hoc inside tests.

### 3. Derive reverse membership from declarations

Create a production reverse-membership builder that iterates the canonical dispatch-key list, reads `declaration().relevant_ops`, and inverts the mapping into `PlannerOpKind -> ordered unique GoalKindTag` values. Use deterministic ordering and deduplication.

### 4. Replace manual planner-op goal arrays

Replace the 21 manual `GOALS_*` constants in `crates/worldwake-ai/src/planner_ops.rs` with declaration-derived reverse membership. `semantics_for()` and `social_or_combat_semantics()` should consume the derived slices while keeping the `PlannerOpSemantics.relevant_goal_kinds` field type unchanged.

### 5. Remove the duplicate production matrix

After migration, no manually curated reverse membership table should remain in `planner_ops.rs`.

## Files to Touch

- `crates/worldwake-ai/src/goal_dispatch_key.rs` (modify — add canonical exhaustive enumeration and/or coarse-tag helper)
- `crates/worldwake-ai/src/goal_dispatch_decl.rs` (modify if the canonical enumeration/helper belongs nearer the declaration table)
- `crates/worldwake-ai/src/planner_ops.rs` (modify — replace manual reverse matrix with derived data)

## Out of Scope

- Changing the `PlannerOpSemantics.relevant_goal_kinds` field type (remains `&'static [GoalKindTag]`)
- Migrating `GoalKindTag` consumers to use `GoalDispatchKey` instead
- Trace label migration (ticket 005)
- Invalidation/feasibility strategy migration (tickets 006–007)
- Removing `*_OPS` const arrays from `goal_model.rs` (may still be used by declarations)

## Acceptance Criteria

### Tests That Must Pass

1. Focused planner-op test proves the derived reverse membership matches the declaration-backed expected coarse tag sets for every currently declared `PlannerOpKind`.
2. Focused planner-op test proves every op referenced by any declaration key appears in the derived reverse map, with `AskWitness` and `YieldForceClaim` remaining intentionally empty because no declaration currently routes to them.
3. Existing suite: `cargo test -p worldwake-ai`
4. Full workspace: `cargo test --workspace`
5. Lint: `cargo clippy --workspace`

### Invariants

1. No search-behavior regression: live candidate/op filtering still routes through declaration `relevant_ops` exactly as before.
2. `PlannerOpSemantics.relevant_goal_kinds` field type and coarse-tag contract remain unchanged.
3. No manual `GOALS_*` reverse-membership matrix remains in `planner_ops.rs` after migration.
4. The planner-op metadata now aligns with declarations, even where the old manual table had stale entries.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/planner_ops.rs` — reverse-membership expected-set test.
Rationale: locks the declaration-aligned coarse tag sets per planner op, including the stale manual memberships intentionally removed by this refactor.
2. `crates/worldwake-ai/src/planner_ops.rs` — reverse-membership completeness / intentional empties test.
Rationale: proves every declaration-routed planner op is represented in the derived map and documents the intentionally empty ops.

### Commands

1. `cargo test -p worldwake-ai reverse_membership`
2. `cargo test -p worldwake-ai`
3. `cargo test --workspace`
4. `cargo clippy --workspace`

## Outcome

- Completion date: 2026-03-29
- What actually changed:
  - Added a production `GoalDispatchKey::all()` exhaustive enumeration so declaration-driven features do not depend on test-only key lists.
  - Replaced the manual `GOALS_*` reverse-membership matrix in `crates/worldwake-ai/src/planner_ops.rs` with a declaration-derived reverse map built from `GoalDispatchKey::declaration().relevant_ops`.
  - Added focused planner-op tests that lock the declaration-aligned coarse `GoalKindTag` sets per `PlannerOpKind` and document the intentionally empty `YieldForceClaim` / `AskWitness` reverse memberships.
- Deviations from original plan:
  - The ticket originally assumed `GoalDispatchKey`, declarations, and goal-side declaration dispatch still needed to be introduced. Reassessment showed that work was already live, so the implemented scope narrowed to removing the remaining duplicate reverse matrix.
  - The old manual planner-op metadata was not fully equivalent to declarations. The completed change intentionally aligns planner-op metadata to declarations, including removing stale memberships such as `Travel -> EngageHostile`, rather than preserving the older duplicate table.
  - Reassessment also found that `PlannerOpSemantics.relevant_goal_kinds` is metadata-only in the current runtime; live search behavior already routes through declaration `relevant_ops`. The completed work therefore fixes architectural drift without changing the active search path.
- Verification results:
  - `cargo test -p worldwake-ai reverse_membership`
  - `cargo test -p worldwake-ai`
  - `cargo test --workspace`
  - `cargo clippy --workspace`
