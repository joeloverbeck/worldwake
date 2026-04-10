# S84SHBELOP-002: Reassess pre-search target validation for ShareBelief

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: No — reassessment-only closeout
**Deps**: S84SHBELOP-001

## Problem

The original proposal assumed ShareBelief still entered search through a generic target scan that could waste one expansion discovering zero Tell affordances, and that the planner lacked a lawful carrier for explaining the root rejection. Live reassessment disproved both assumptions.

## Assumption Reassessment (2026-04-10)

1. **Feasibility system remains reordering-only**: `feasibility.rs` still documents that it must not exclude goals from search, so the original ticket was right not to overload feasibility.
2. **ShareBelief already synthesizes exact Tell root targets**: `GroundedGoal::synthesized_root_candidate_targets` in `crates/worldwake-ai/src/goal_model.rs` returns the listener identity for `PlannerOpKind::Tell` on `GoalKind::ShareBelief`. Root admission is therefore not driven by a broad snapshot scan for arbitrary Tell candidates.
3. **Planner already records lawful root rejections**: `search_plan` and `get_affordances_for_defs` already populate `SearchExpansionSummary.root_omissions`, and `decision_trace.rs` already renders those omissions in `PlanAttemptTrace`.
4. **Tell is already treated as a progress barrier for ShareBelief**: `GoalKind::is_progress_barrier` covers the Tell step through the declaration table, so the planning/search contract this ticket wanted to protect is already explicit and tested.
5. **Lower-layer transition semantics do not expose the proposed gap**: Tell hypothetical transitions use `PlannerTransitionKind::GoalModelFallback`, so the original proposed snapshot pre-scan does not match the actual planner-root admission path under audit.

## Architecture Check

1. The proposed `snapshot_has_no_matching_targets` gate is not aligned with the live ShareBelief root contract. Adding it now would duplicate or partially bypass existing root synthesis and omission tracing rather than fixing a proven defect.
2. No backward-compatibility shims. Because reassessment found no remaining production gap in this area, the correct outcome is to close the ticket without code changes.

## Verification Layers

1. Root target synthesis for ShareBelief/Tell remains exact-bound to the listener identity -> existing lower-layer test in `goal_model.rs`
2. Tell remains a progress barrier for ShareBelief -> existing lower-layer test in `goal_model.rs`
3. Root omission evidence remains carried into `PlanAttemptTrace` -> existing `decision_trace.rs` rendering path plus search-layer omission capture
4. Single-layer ticket after reassessment: no new implementation work remained in the AI planning pipeline

## What to Change

### 1. Reassess the proposed planner gate against live ShareBelief root synthesis

Inspect the actual root-candidate path for ShareBelief and confirm whether it still relies on a broad snapshot scan. If the root is already synthesized from goal identity, do not add a duplicate pre-search gate.

### 2. Reassess whether `PlanAttemptTrace` already carries the claimed rejection signal

Confirm whether search-layer root omissions already flow into decision-trace output. If they do, close the ticket rather than introducing a parallel rejection reason path.

### 3. Close the ticket according to the live contract

If reassessment shows the planner already owns the intended invariant, update the ticket to document the confirmed symbols and verification instead of adding redundant code.

## Files to Touch

- `crates/worldwake-ai/src/goal_model.rs` (read-only reassessment reference)
- `crates/worldwake-ai/src/search/candidates.rs` (read-only reassessment reference)
- `crates/worldwake-ai/src/search/mod.rs` (read-only reassessment reference)
- `crates/worldwake-ai/src/decision_trace.rs` (read-only reassessment reference)

## Out of Scope

- Modifying the feasibility system's reordering-only contract
- Adding a duplicate pre-search gate that bypasses existing ShareBelief root synthesis
- Adding a second diagnostic carrier when `root_omissions` already exists
- Snapshot construction changes (handled by S84SHBELOP-001)

## Acceptance Criteria

### Tests That Must Pass

1. Existing focused test proving ShareBelief synthesizes Tell root targets from goal identity still passes
2. Existing focused test proving ShareBelief Tell is a progress barrier still passes
3. Existing focused suite: `cargo test -p worldwake-ai share_belief`

### Invariants

1. Feasibility remains reordering-only
2. ShareBelief Tell root admission remains owned by `synthesized_root_candidate_targets`, not by a parallel snapshot pre-scan
3. Root rejection evidence remains carried through `root_omissions` into decision traces

## Test Plan

### New/Modified Tests

1. Existing `crates/worldwake-ai/src/goal_model.rs` tests covering ShareBelief Tell root synthesis and progress-barrier behavior
2. Existing `crates/worldwake-ai/src/search/` plus `crates/worldwake-ai/src/decision_trace.rs` omission pipeline inspected during reassessment

### Commands

1. `cargo test -p worldwake-ai share_belief`

## Outcome

Completed on 2026-04-10.

- Reassessed the proposed pre-search targetless-goal gate against the live ShareBelief planning contract and found no remaining production delta to implement.
- Confirmed `GoalKind::ShareBelief` already synthesizes exact Tell root targets from the listener identity via `GroundedGoal::synthesized_root_candidate_targets` in `crates/worldwake-ai/src/goal_model.rs`.
- Confirmed Tell is already treated as a progress barrier for ShareBelief in `GoalKind::is_progress_barrier`.
- Confirmed the planner already carries root rejection evidence through `SearchExpansionSummary.root_omissions`, which `decision_trace.rs` renders in `PlanAttemptTrace`.
- Closed the ticket without code changes because the original proposed gate would duplicate or partially bypass the live root-admission path rather than fixing a proven defect.

## Verification Result

- Passed `cargo test -p worldwake-ai share_belief`
