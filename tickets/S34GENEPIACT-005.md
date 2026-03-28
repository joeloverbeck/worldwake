# S34GENEPIACT-005: Planner ops — PlannerOpKind::VerifyBelief, PlannerOpKind::AskWitness, goal model integration

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — worldwake-ai: new planner op kinds, goal model extensions, planner transition logic
**Deps**: S34GENEPIACT-001 (GoalKind::VerifyBelief), S34GENEPIACT-003 (verify_belief action def exists), S34GENEPIACT-004 (ask_witness action def exists)

## Problem

The GOAP planner cannot construct plans involving epistemic actions. Without `PlannerOpKind::VerifyBelief` and `PlannerOpKind::AskWitness`, the planner cannot find paths from a `VerifyBelief` goal to terminal actions. Without `GoalKindTag::VerifyBelief` and the `GoalKindPlannerExt` implementation, the planner has no dispatch logic for the new goal kind.

## Assumption Reassessment (2026-03-28)

1. `PlannerOpKind` is defined in `crates/worldwake-ai/src/planner_ops.rs:12-39` with ~26 variants. The `Investigate` variant (from S27) is the closest precedent — it is terminal for `InvestigateViolation` goals and is a progress barrier.
2. `PlannerOpSemantics` at `planner_ops.rs:42-48` defines per-op properties: `may_appear_mid_plan`, `is_materialization_barrier`, `transition_kind`, `relevant_goal_kinds`. Both new ops should have `may_appear_mid_plan = false` and `transition_kind = GoalModelFallback` per the spec.
3. `GoalKindTag` is defined in `crates/worldwake-ai/src/goal_model.rs:25-47` with 20 variants matching `GoalKind`. A `VerifyBelief` tag must be added.
4. `GoalKindPlannerExt` trait at `goal_model.rs:49-96` with implementation at `goal_model.rs:387-676` provides `goal_kind_tag()`, `relevant_op_kinds()`, `build_payload_override()`, `apply_planner_step()`, `is_satisfied()`, `matches_binding()`. All must be extended for `VerifyBelief`.
5. `relevant_op_kinds` constants are defined at `goal_model.rs:98-172`. The spec defines `VERIFY_BELIEF_OPS: &[PlannerOpKind] = &[Travel, VerifyBelief, AskWitness]`.
6. `is_satisfied()` for `VerifyBelief` checks `believed_entity_state(subject_entity).observed_tick >= generation_tick`. This is a belief-view read.
7. `is_progress_barrier` for both ops should return true for `VerifyBelief` goals — observation/witness results are unknown to the planner, so it cannot plan past verification.
8. The `matches_binding()` implementation needs to handle `VerifyBelief` goals. Since verification targets are embedded in the `VerificationSubject`, binding should check that the affordance payload matches the goal's subject.

## Architecture Check

1. Two new `PlannerOpKind` variants follow the exact pattern established by `Investigate`. Both are terminal barriers for `VerifyBelief` goals with `GoalModelFallback` transition. This is the minimal, clean addition.
2. `AskWitness` as a terminal op for `VerifyBelief` goals means the planner can choose between traveling to the target place (Travel + VerifyBelief) or asking a co-located witness (AskWitness). This is correct — both paths satisfy the goal.
3. No backward-compatibility shims.

## Verification Layers

1. Planner constructs Travel -> VerifyBelief plan for remote VerifyBelief goal -> focused planner test with decision trace
2. Planner constructs AskWitness plan for VerifyBelief goal with co-located witness -> focused planner test with decision trace
3. VerifyBelief satisfaction: goal satisfied when `observed_tick >= generation_tick` -> focused unit test on `is_satisfied()`
4. VerifyBelief satisfaction: goal NOT satisfied when belief is stale -> focused unit test on `is_satisfied()`
5. Both ops are progress barriers for VerifyBelief goals -> focused unit test on `is_progress_barrier()`
6. `relevant_op_kinds` returns correct set for VerifyBelief -> focused unit test

## What to Change

### 1. Add PlannerOpKind variants

In `crates/worldwake-ai/src/planner_ops.rs`, add:
```rust
VerifyBelief,
AskWitness,
```

Add `PlannerOpSemantics` entries for both:
- `VerifyBelief`: `may_appear_mid_plan = false`, `is_materialization_barrier = false`, `transition_kind = GoalModelFallback`, `relevant_goal_kinds = &[GoalKindTag::VerifyBelief]`. `is_progress_barrier` returns true for `VerifyBelief` goals.
- `AskWitness`: Same properties. Terminal for `VerifyBelief` goals when a co-located witness is available.

Wire the new ops to their corresponding action defs in the action-def-to-op mapping.

### 2. Add GoalKindTag::VerifyBelief

In `crates/worldwake-ai/src/goal_model.rs`, add `VerifyBelief` to the `GoalKindTag` enum.

### 3. Implement GoalKindPlannerExt for VerifyBelief

In `crates/worldwake-ai/src/goal_model.rs`:

- `goal_kind_tag()`: Return `GoalKindTag::VerifyBelief`.
- `relevant_op_kinds()`: Return `VERIFY_BELIEF_OPS` (Travel, VerifyBelief, AskWitness).
- `build_payload_override()`: Construct `VerifyBeliefPayload { subject }` for VerifyBelief op, `AskWitnessPayload { target, topic_entity, topic_commodity }` for AskWitness op.
- `apply_planner_step()`: For VerifyBelief op, mark goal as potentially satisfied (barrier prevents further planning). For AskWitness, same.
- `is_satisfied()`: Check `believed_entity_state(subject_entity).observed_tick >= generation_tick`.
- `matches_binding()`: Match affordance payload's subject against the goal's subject.

### 4. Add VERIFY_BELIEF_OPS constant

```rust
const VERIFY_BELIEF_OPS: &[PlannerOpKind] = &[
    PlannerOpKind::Travel,
    PlannerOpKind::VerifyBelief,
    PlannerOpKind::AskWitness,
];
```

### 5. Update all match arms

All existing match arms on `PlannerOpKind` and `GoalKindTag` must be extended with the new variants. This includes `action_def_to_op_kind`, `op_kind_to_action_def`, and any display/debug helpers.

## Files to Touch

- `crates/worldwake-ai/src/planner_ops.rs` (modify — add 2 op kinds, semantics, action-def mapping)
- `crates/worldwake-ai/src/goal_model.rs` (modify — add GoalKindTag variant, GoalKindPlannerExt impl, VERIFY_BELIEF_OPS constant)
- `crates/worldwake-ai/src/search/` (modify if match arms need updating — transition.rs, candidates.rs)

## Out of Scope

- Candidate generation (`emit_verify_belief_goals`) — ticket 006
- Ranking and motive scoring — ticket 007
- Golden E2E tests — ticket 008
- Changes to `PlanningState` (no new hypothetical state transitions needed — verification is a barrier)
- Changes to `BlockedIntentMemory` or `IntentionFrame`
- `GoalFamilyPolicy` entry for VerifyBelief (this ticket adds planner dispatch; policy entry is part of ranking in ticket 007)

## Acceptance Criteria

### Tests That Must Pass

1. Planner constructs Travel -> VerifyBelief plan for remote `VerifyBelief` goal
2. Planner constructs AskWitness plan for `VerifyBelief` goal with co-located witness
3. `VerifyBelief` satisfaction: goal satisfied when `observed_tick >= generation_tick`
4. `VerifyBelief` satisfaction: goal NOT satisfied when belief is stale (`observed_tick < generation_tick`)
5. `GoalKey` uniqueness: two `EntityLocation` verifications for different entities at same place coexist in planner
6. `GoalKey` uniqueness: `EntityLocation` and `SupplyAvailability` at same place coexist in planner
7. Both ops are progress barriers for `VerifyBelief` goals
8. `relevant_op_kinds` for `VerifyBelief` returns `[Travel, VerifyBelief, AskWitness]`
9. `PlannerOpKind` match exhaustiveness compiles (compiler-enforced)
10. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. Planner only reads belief state, never authoritative world state (P12)
2. Both epistemic ops are barriers — planner does not plan past verification (observation results unknown)
3. `GoalKindTag` exhaustiveness — all match arms cover new variant (compiler-enforced)
4. Determinism — no `HashMap`, no floats, no wall-clock time in planner paths

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/planner_ops.rs` (in-module tests) — barrier and semantics tests for both new ops
2. `crates/worldwake-ai/src/goal_model.rs` (in-module tests) — satisfaction, relevant_op_kinds, GoalKey uniqueness
3. `crates/worldwake-ai/src/search/` (focused planner test) — Travel->VerifyBelief and AskWitness plan construction

### Commands

1. `cargo test -p worldwake-ai`
2. `cargo clippy -p worldwake-ai`
3. `cargo build --workspace`
