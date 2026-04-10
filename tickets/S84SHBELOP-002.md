# S84SHBELOP-002: Add pre-search target validation for targetless goals

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: Yes — planning pipeline (`worldwake-ai`)
**Deps**: S84SHBELOP-001

## Problem

Even after S84SHBELOP-001 fixes the snapshot place indexing, there remain legitimate cases where a ShareBelief goal has no matching targets in the snapshot (e.g., the listener left the place between candidate generation and planning). Currently, the search wastes 1 expansion discovering zero affordances before returning `FrontierExhausted`. A pre-search check could skip the search entirely and record a clear rejection reason, saving budget (FND-20) and improving debuggability (FND-29).

## Assumption Reassessment (2026-04-10)

1. **Feasibility system is reordering-only**: `feasibility.rs:19-20` documents "Used to reorder candidates within the same `GoalPriorityClass` — never to exclude goals from search." The pre-search check must be a separate mechanism, not an extension of feasibility.
2. **Search pipeline location confirmed**: The planning pipeline in `agent_tick/mod.rs` calls `search_plan` after feasibility scoring. The pre-search check would be inserted between feasibility scoring and `search_plan`.
3. **Shared boundary**: The pre-search validation sits between the ranked candidate list and the search entry point. It reads the snapshot's place index and the goal's relevant ops to determine if any matching targets exist.

## Architecture Check

1. A separate pre-search validation is cleaner than overloading the feasibility system with gating behavior. The feasibility system's reordering-only contract is documented and relied upon by other goal kinds. A new `snapshot_has_no_matching_targets` function is explicit about its purpose and independently testable.
2. No backward-compatibility shims. The pre-search check is purely additive — goals that have matching targets proceed unchanged.

## Verification Layers

1. Pre-search check skips search when zero targets exist -> focused unit test returning `FrontierExhausted` with 0 expansions
2. Pre-search check passes when targets exist -> focused unit test proceeding to search
3. Rejection reason recorded in `PlanAttemptTrace` -> decision trace assertion
6. Single-layer ticket: pre-search validation is entirely within the AI planning pipeline.

## What to Change

### 1. Add `snapshot_has_no_matching_targets` function

In `crates/worldwake-ai/src/search/` or `crates/worldwake-ai/src/agent_tick/`, add a function that:
- Takes the goal's `relevant_ops`, the semantics table, and the snapshot
- For each relevant action def, checks if the snapshot contains any entities matching the def's `TargetSpec` at the actor's place
- Returns `true` if no matching targets exist for any relevant def

### 2. Insert pre-search check in planning pipeline

In the planning pipeline (after feasibility scoring, before `search_plan`), call the new function. If it returns `true`, skip the search and return `PlanSearchOutcome::FrontierExhausted { expansions_used: 0 }` with a recorded rejection reason.

### 3. Add focused tests

Test the pre-search check with:
- A snapshot containing matching targets (check passes, search proceeds)
- A snapshot with zero matching targets (check fails, search skipped)

## Files to Touch

- `crates/worldwake-ai/src/agent_tick/mod.rs` (modify — insert pre-search check)
- `crates/worldwake-ai/src/search/candidates.rs` or new file (modify/new — `snapshot_has_no_matching_targets` function)

## Out of Scope

- Modifying the feasibility system's reordering-only contract
- Adding travel planning for goals with remote targets
- Snapshot construction changes (handled by S84SHBELOP-001)

## Acceptance Criteria

### Tests That Must Pass

1. New focused test: pre-search check returns skip when snapshot has zero matching targets for Tell
2. New focused test: pre-search check returns proceed when snapshot has co-located agent for Tell
3. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. Feasibility system remains reordering-only — the pre-search check is a separate mechanism
2. Goals with matching targets proceed to search unchanged
3. Skipped goals record a clear rejection reason in `PlanAttemptTrace`

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/agent_tick/mod.rs` or `crates/worldwake-ai/src/search/candidates.rs` (test module) — pre-search target validation
2. `crates/worldwake-ai/src/agent_tick/mod.rs` (test module) — integration with planning pipeline

### Commands

1. `cargo test -p worldwake-ai snapshot_has_no_matching`
2. `cargo test -p worldwake-ai`
3. `cargo clippy --workspace --all-targets -- -D warnings`
