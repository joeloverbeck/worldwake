# HARPREE14-010: Extract goal planning semantics from search.rs

**Status**: ✅ COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes -- internal planner refactor and test strengthening
**Deps**: None (Wave 3, same area as HARPREE14-006 but independent)
**Spec Reference**: HARDENING-PRE-E14.md, HARDEN-A01

## Problem

`search.rs` still contains goal-specific planning behavior that belongs with the goal model rather than the generic search loop. The current search core decides:

1. which payload override a goal needs for trade / combat / loot,
2. whether a hypothetical post-step state satisfies the goal,
3. whether a materialization barrier should terminate the current plan,
4. how some planner ops mutate hypothetical state for that goal.

That is the wrong long-term boundary. The search loop should orchestrate expansion, ordering, and budget enforcement; goal-defined planning semantics should live behind the existing goal-model seam.

## Assumption Reassessment (2026-03-12)

1. `goal_is_satisfied()` still exists in `crates/worldwake-ai/src/search.rs` and is still goal-specific -- confirmed.
2. `build_payload_override()` still exists in `crates/worldwake-ai/src/search.rs` and is still goal-specific -- confirmed.
3. `progress_barrier()` still exists in `crates/worldwake-ai/src/search.rs` and is still goal-specific -- confirmed.
4. `apply_step()` also contains goal-specific planner semantics and must be part of the extraction -- corrected.
5. `search_plan()` has only one production call site today: `process_agent()` in `crates/worldwake-ai/src/agent_tick.rs`; the rest are local unit tests -- corrected.
6. `worldwake-ai` already has an existing goal/planner boundary in `goal_model.rs` through `GoalKindPlannerExt::relevant_op_kinds()` -- corrected.
7. HARDEN-C01 and HARDEN-D02 have already landed: `search.rs` uses a `BinaryHeap`, and the beam/budget edge-case tests already exist -- corrected.

## Architecture Check

1. A runtime-injected `&dyn GoalSemantics` parameter on `search_plan()` is not the strongest architecture here. Goal planning semantics are intrinsic to `GoalKind`, not caller-selected policy.
2. The cleaner boundary is to extend the existing goal-model planner seam so `GoalKind` owns:
   - payload override semantics,
   - goal satisfaction semantics,
   - progress-barrier semantics,
   - goal-specific hypothetical state application.
3. This is more robust than the current architecture because it removes domain matches from the search core without introducing needless runtime indirection or widening the public planning API.
4. This is more extensible than the original ticket proposal because adding a new `GoalKind` stays a localized change to one goal-semantics surface instead of splitting responsibility between `goal_model.rs` and a separate injected implementation.
5. Ideal longer-term architecture would likely make unsupported-goal handling explicit on the same goal-planning surface as well. This ticket may leave that detail in `search.rs` if moving it would add churn without a clear payoff.

## What to Change

### 1. Extract goal planning semantics out of `search.rs`

Move the goal-specific logic now in:

- `build_payload_override()`
- `goal_is_satisfied()`
- `progress_barrier()`
- the goal-sensitive parts of `apply_step()`

into the goal-model layer.

### 2. Extend the existing goal planner abstraction

Prefer extending `GoalKindPlannerExt` in `crates/worldwake-ai/src/goal_model.rs` or introducing a closely related goal-semantics helper in the same area.

Do not add a caller-injected `&dyn GoalSemantics` parameter to `search_plan()` unless the code proves that intrinsic goal semantics are insufficient.

### 3. Keep `search_plan()` behavior-preserving

`search_plan()` should still:

- reject unsupported goals,
- filter affordances by relevant op kinds,
- expand nodes deterministically,
- respect beam/budget limits,
- return the same terminal kinds for the same situations.

The change is about ownership of goal semantics, not planner behavior.

### 4. Strengthen direct tests around the extracted seam

The old ticket assumption that no new tests are needed is too weak. Add focused unit coverage for the extracted goal-semantics behavior so future `GoalKind` changes fail close to the source rather than only through broad search integration tests.

## Files to Touch

- `crates/worldwake-ai/src/goal_model.rs` (modify; preferred extraction target)
- `crates/worldwake-ai/src/search.rs` (modify; remove inlined goal-specific logic)
- `crates/worldwake-ai/src/lib.rs` (modify only if re-exports change)
- `crates/worldwake-ai/src/agent_tick.rs` (only if a signature change becomes unavoidable)
- `tickets/HARPREE14-010-extract-goal-semantics-trait.md` (this ticket; update before implementation)

## Out of Scope

- Adding new `GoalKind` variants
- Changing planner behavior or decision policy
- Modifying ranking, candidate generation, or observation filtering
- Introducing backward-compatibility shims, alias APIs, or duplicate semantics layers
- Large file rewrites or goal-type decomposition beyond what is needed to centralize the semantics cleanly

## Acceptance Criteria

### Tests That Must Pass

1. Existing `search.rs` unit tests pass unchanged in behavior
2. New focused goal-semantics unit tests pass
3. Golden e2e hashes remain identical
4. `cargo test --workspace` passes
5. `cargo clippy --workspace --all-targets -- -D warnings` passes

### Invariants

1. Search behavior is unchanged for all existing `GoalKind`s
2. Goal-specific planner semantics no longer live inline in `search.rs`
3. The extraction keeps semantics attached to the goal model rather than adding a parallel compatibility layer
4. Golden e2e state hashes remain identical

## Test Plan

### New/Modified Tests

1. Add focused unit tests for the extracted goal-semantics methods in `goal_model.rs` (or the extracted semantics module).
2. Keep the existing `search.rs` tests as integration coverage for behavior preservation.
3. Keep golden e2e as the determinism and cross-system regression gate.

### Commands

1. `cargo test -p worldwake-ai goal_model`
2. `cargo test -p worldwake-ai search`
3. `cargo test -p worldwake-ai --test golden_e2e`
4. `cargo test --workspace`
5. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

- Completed: 2026-03-12
- What actually changed:
  - Reassessed the ticket and corrected its architectural assumption before implementation.
  - Extended the existing `GoalKindPlannerExt` seam in `goal_model.rs` to own payload override, hypothetical state application, goal satisfaction, and progress-barrier semantics.
  - Removed the inlined goal-specific planner logic from `search.rs` without changing `search_plan()`'s public signature.
  - Replaced the old `Result<_, ()>` extraction seam with an explicit internal `GoalPayloadOverrideError`.
  - Added focused unit coverage around the extracted goal-semantics behavior.
- Deviations from original plan:
  - Did not add a caller-injected `&dyn GoalSemantics` parameter or a new `goal_semantics.rs` file.
  - Kept the semantics attached to `GoalKind` because that is the cleaner and more extensible boundary in the current architecture.
  - Corrected the ticket scope to include the goal-sensitive logic inside `apply_step()`, which the original ticket missed.
- Verification:
  - `cargo test -p worldwake-ai goal_model`
  - `cargo test -p worldwake-ai search`
  - `cargo test -p worldwake-ai --test golden_e2e`
  - `cargo test --workspace`
  - `cargo clippy --workspace --all-targets -- -D warnings`
