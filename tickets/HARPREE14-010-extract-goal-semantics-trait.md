# HARPREE14-010: Extract GoalSemantics trait from search.rs

**Status**: PENDING
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: Yes -- new trait, new file, search.rs API change
**Deps**: None (Wave 3, independent but same area as HARPREE14-006)
**Spec Reference**: HARDENING-PRE-E14.md, HARDEN-A01

## Problem

Adding a new `GoalKind` requires editing 3 match blocks deep in the search core: `goal_is_satisfied()` (lines 330-380), `build_payload_override()` (lines 169-234), and `progress_barrier()` (lines 311-328). This violates Principle 12 -- the search algorithm should not know domain-specific goal logic.

## Assumption Reassessment (2026-03-11)

1. `goal_is_satisfied()` exists at line 330 with GoalKind match arms -- confirmed
2. `build_payload_override()` exists at line 169 with GoalKind match arms -- confirmed
3. `progress_barrier()` exists at line 311 with GoalKind match arms -- confirmed
4. All three are called within `search_plan()` flow -- confirmed

## Architecture Check

1. A `GoalSemantics` trait separates the search algorithm (generic) from goal-specific logic (domain). This follows Principle 12 (System Decoupling).
2. The trait can have a single default implementation that uses the existing match-based logic, so the refactor is behavior-preserving.
3. `search_plan()` accepts `&dyn GoalSemantics` -- callers pass the implementation.

## What to Change

### 1. Create `crates/worldwake-ai/src/goal_semantics.rs`

Define the trait:
```rust
pub trait GoalSemantics {
    fn is_satisfied(&self, goal: &GroundedGoal, state: &PlanningState<'_>) -> bool;
    fn build_payload_override(
        &self,
        affordance_payload: Option<&ActionPayload>,
        state: &PlanningState<'_>,
        targets: &[EntityId],
        def: &ActionDef,
        semantics: &PlannerOpSemantics,
    ) -> Result<Option<ActionPayload>, ()>;
    fn is_progress_barrier(&self, goal: &GroundedGoal, step: &PlannedStep) -> bool;
}
```

### 2. Implement `DefaultGoalSemantics`

Move the existing match-based logic from `search.rs` into a struct implementing the trait. The match blocks become method bodies.

### 3. Update `search_plan()` signature

Accept `&dyn GoalSemantics` as a parameter. Replace inline calls to the old functions with trait method calls.

### 4. Update all callers of `search_plan()`

Pass `&DefaultGoalSemantics` (or equivalent) at all call sites.

### 5. Re-export from `lib.rs`

Add the new module and key types to the crate's public API.

## Files to Touch

- `crates/worldwake-ai/src/goal_semantics.rs` (new)
- `crates/worldwake-ai/src/search.rs` (modify -- remove inline logic, accept trait)
- `crates/worldwake-ai/src/lib.rs` (modify -- add module, re-export)
- Callers of `search_plan()` -- grep to find all call sites and update signatures

## Out of Scope

- Adding new `GoalKind` variants
- Changing goal satisfaction logic
- Modifying `PlanningState` or `GroundedGoal`
- Implementing per-goal-kind separate structs (the default implementation uses match, which is fine)

## Acceptance Criteria

### Tests That Must Pass

1. All existing search tests pass unchanged (behavior preserved)
2. Golden e2e hashes identical
3. `cargo test --workspace` passes
4. `cargo clippy --workspace` -- no new warnings

### Invariants

1. Search algorithm behavior unchanged for all existing GoalKinds
2. Golden e2e state hashes identical
3. `search_plan()` is now extensible via trait without editing search core

## Test Plan

### New/Modified Tests

1. No new behavior tests needed -- this is a pure refactor. Existing tests validate identical behavior.
2. Optionally add a doc-test or unit test showing a custom `GoalSemantics` impl can be passed.

### Commands

1. `cargo test -p worldwake-ai search` (targeted)
2. `cargo test -p worldwake-ai --test golden_e2e` (determinism check)
3. `cargo test --workspace`
4. `cargo clippy --workspace`
