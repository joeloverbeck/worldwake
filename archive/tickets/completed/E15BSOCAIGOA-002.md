# E15BSOCAIGOA-002: Verify Existing `PlannerOpKind::Tell` Integration And Close Ticket

**Status**: ✅ COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: No new planner architecture required; verification and test strengthening only
**Deps**: `tickets/E15BSOCAIGOA-001.md`, `specs/E15b-social-ai-goals.md`, `specs/IMPLEMENTATION-ORDER.md`, `docs/FOUNDATIONS.md`

## Problem

This ticket originally assumed the planner still lacked a Tell operation kind and Tell semantics. That assumption is now false. The current code already includes:

1. `GoalKind::ShareBelief` in `crates/worldwake-core/src/goal.rs`
2. `GoalKindTag::ShareBelief` in `crates/worldwake-ai/src/goal_model.rs`
3. `PlannerOpKind::Tell` in `crates/worldwake-ai/src/planner_ops.rs`
4. Tell affordance classification from `ActionDomain::Social`
5. Existing unit coverage for Tell goal relevance and planner-op semantics

The real task is to correct the ticket so it reflects the current architecture, verify the existing implementation, and avoid redoing already-landed work.

## Assumption Reassessment (2026-03-15)

1. `PlannerOpKind::Tell` already exists in `crates/worldwake-ai/src/planner_ops.rs`; the enum currently has 16 variants, not 14.
2. Tell affordances already classify to `PlannerOpKind::Tell` through `classify_action_def()` for `(ActionDomain::Social, "tell", ActionPayload::None)`.
3. Tell semantics are already registered through `semantics_for()` with:
   - `may_appear_mid_plan: false`
   - `is_materialization_barrier: false`
   - `transition_kind: PlannerTransitionKind::GoalModelFallback`
   - `relevant_goal_kinds: &[GoalKindTag::ShareBelief]`
4. `GoalKind::ShareBelief` already resolves to `PlannerOpKind::Tell` in `crates/worldwake-ai/src/goal_model.rs`.
5. The missing piece in current AI behavior is not planner-op wiring; it is autonomous social candidate generation, which remains deferred in `crates/worldwake-ai/src/candidate_generation.rs` and belongs to later E15b tickets.
6. Existing tests already covered Tell indirectly, but the exact Tell semantics contract was spread across broader assertions rather than locked by one focused regression test.

## Architecture Check

1. Re-implementing `PlannerOpKind::Tell` now would be pure churn and would not improve robustness, extensibility, or architectural clarity.
2. The current architecture is the right boundary for this concern:
   - `planner_ops.rs` owns action-definition classification and operation semantics.
   - `goal_model.rs` owns goal-to-op relevance.
   - `candidate_generation.rs` decides whether a goal family is emitted at all.
3. That separation is cleaner than collapsing Tell planning into search-specific special cases or duplicating planner-op aliases.
4. The real future-facing architectural work for social AI is autonomous ShareBelief candidate generation and ranking, not another planner-op layer.
5. No backwards-compatibility aliases or duplicate planner-op paths should be added.

## What to Change

### 1. Correct the ticket scope

Update this ticket to reflect that `PlannerOpKind::Tell` and its semantics already exist and are not missing work.

### 2. Verify existing implementation

Re-run the relevant `worldwake-ai` tests that exercise:

1. Tell planner-op semantics
2. ShareBelief goal-to-op mapping
3. Workspace lint cleanliness for the touched code

### 3. Strengthen direct regression coverage

Add one focused test in `crates/worldwake-ai/src/planner_ops.rs` that asserts the full Tell semantics contract explicitly in one place, rather than depending only on broader table tests.

## Files to Touch

- `tickets/E15BSOCAIGOA-002.md` (modify, then archive)
- `crates/worldwake-ai/src/planner_ops.rs` (modify tests only)

## Out of Scope

- Re-adding or renaming `PlannerOpKind::Tell`
- Any planner architecture rewrite
- `GoalKind::ShareBelief` modeling
- Social candidate generation (`E15BSOCAIGOA-004`)
- Social ranking / motive logic (`E15BSOCAIGOA-005`)
- Tell action handler behavior in `worldwake-systems`

## Acceptance Criteria

### Tests That Must Pass

1. Tell remains classified from the Social `tell` action definition.
2. Tell remains standalone-only: `may_appear_mid_plan == false`.
3. Tell remains non-barrier: `is_materialization_barrier == false`.
4. Tell remains `PlannerTransitionKind::GoalModelFallback`.
5. Tell remains mapped only to `GoalKindTag::ShareBelief`.
6. Focused `worldwake-ai` tests pass.
7. `cargo clippy --workspace --all-targets -- -D warnings` passes.

### Invariants

1. Planner-op semantics stay centralized in `planner_ops.rs`.
2. Goal-to-op relevance stays centralized in `goal_model.rs`.
3. Tell remains a single planner-op family with no aliases or compatibility wrappers.
4. This ticket does not claim candidate generation work that belongs to later E15b tasks.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/planner_ops.rs` — add a focused regression test for the Tell semantics contract.

### Commands

1. `cargo test -p worldwake-ai planner_ops -- --nocapture`
2. `cargo test -p worldwake-ai share_belief -- --nocapture`
3. `cargo test -p worldwake-ai`
4. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

- Completion date: 2026-03-15
- What actually changed:
  - Reassessed the ticket against the real code and corrected it from "implement Tell planner op" to "verify existing Tell planner integration and close ticket".
  - Confirmed that `PlannerOpKind::Tell`, Tell classification, Tell semantics, and ShareBelief goal wiring were already implemented.
  - Added one focused Tell semantics regression test so the exact contract is explicitly locked.
- Deviations from original plan:
  - No production planner code changed because the original ticket assumption was obsolete.
  - The remaining missing social-AI work is autonomous ShareBelief candidate generation, which belongs to later E15b tickets rather than this one.
- Verification results:
  - Passed `cargo test -p worldwake-ai planner_ops -- --nocapture`
  - Passed `cargo test -p worldwake-ai share_belief -- --nocapture`
  - Passed `cargo test -p worldwake-ai`
  - Passed `cargo clippy --workspace --all-targets -- -D warnings`
