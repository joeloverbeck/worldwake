# E15BSOCAIGOA-002: Add PlannerOpKind::Tell with PlannerOpSemantics

**Status**: PENDING
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — planner operation types in ai crate
**Deps**: E15BSOCAIGOA-001

## Problem

The GOAP planner has no operation kind for Tell actions. Without `PlannerOpKind::Tell` and its semantics, the planner cannot generate plans that include Tell steps, even if ShareBelief goals exist.

## Assumption Reassessment (2026-03-15)

1. `PlannerOpKind` in `crates/worldwake-ai/src/planner_ops.rs` has 14 variants (Travel through Defend). Confirmed no Tell exists.
2. `PlannerOpSemantics` struct has fields: op_kind, may_appear_mid_plan, is_materialization_barrier, transition_kind, relevant_goal_kinds.
3. `PlannerTransitionKind` has 4 variants including `GoalModelFallback` which is used by standalone goal ops (Bury, Loot).
4. Semantics are registered via a static array or registry pattern — need to add Tell entry.

## Architecture Check

1. Follows existing PlannerOpKind pattern exactly (Bury/Loot precedent for standalone-goal ops).
2. Tell semantics: `may_appear_mid_plan: false` (standalone goal, not a mid-plan step), `is_materialization_barrier: false`, `transition_kind: GoalModelFallback`, `relevant_goal_kinds: &[GoalKindTag::ShareBelief]`.

## What to Change

### 1. Add PlannerOpKind::Tell variant

In `crates/worldwake-ai/src/planner_ops.rs`, add `Tell` to the enum.

### 2. Register PlannerOpSemantics for Tell

Add semantics entry:
```rust
PlannerOpSemantics {
    op_kind: PlannerOpKind::Tell,
    may_appear_mid_plan: false,
    is_materialization_barrier: false,
    transition_kind: PlannerTransitionKind::GoalModelFallback,
    relevant_goal_kinds: &[GoalKindTag::ShareBelief],
}
```

### 3. Wire into affordance → planner op mapping

Ensure that Tell action affordances map to `PlannerOpKind::Tell` in any affordance-to-op conversion logic (check `planner_ops.rs` and `search.rs` for mapping patterns).

## Files to Touch

- `crates/worldwake-ai/src/planner_ops.rs` (modify)

## Out of Scope

- GoalKind::ShareBelief (E15BSOCAIGOA-001)
- Candidate generation (E15BSOCAIGOA-004)
- Ranking logic (E15BSOCAIGOA-005)
- Tell action definition or handler (these exist from E15, this ticket only adds the planner op)
- Any changes to worldwake-systems Tell handler

## Acceptance Criteria

### Tests That Must Pass

1. `PlannerOpKind::Tell` semantics: `may_appear_mid_plan == false`
2. `PlannerOpKind::Tell` semantics: `relevant_goal_kinds` contains exactly `GoalKindTag::ShareBelief`
3. `PlannerOpKind::Tell` semantics: `transition_kind == GoalModelFallback`
4. Existing suite: `cargo test -p worldwake-ai` — no regressions (exhaustive match arms compile)

### Invariants

1. All PlannerOpKind match arms remain exhaustive
2. Every PlannerOpKind has exactly one PlannerOpSemantics entry
3. Tell is standalone-only (may_appear_mid_plan: false) — it must never appear as an intermediate step in multi-step plans

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/planner_ops.rs` (inline tests) — Tell semantics correctness

### Commands

1. `cargo test -p worldwake-ai`
2. `cargo clippy --workspace --all-targets -- -D warnings`
