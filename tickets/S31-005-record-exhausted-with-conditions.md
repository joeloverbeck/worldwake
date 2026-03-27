# S31-005: Update `record_exhausted_goals` to Capture Conditions and Baseline

**Status**: PENDING
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — AI planning pipeline
**Deps**: S31-001, S31-002

## Problem

When a goal exhausts the search budget, its `ExhaustionEntry` must now record the invalidation conditions and baseline snapshot so that future ticks can check whether the world changed in relevant ways. Currently `record_exhausted_goals` only sets `exhausted_at` and `count`.

## Assumption Reassessment (2026-03-27)

1. `record_exhausted_goals` is at `planning.rs:350-382`. It takes `&mut AgentDecisionRuntime`, `plans` slice, and `tick`.
2. The call site at `planning.rs:484` passes `runtime`, `&plans`, and `tick`.
3. The function needs additional parameters: `agent: EntityId`, `view: &dyn GoalBeliefView`, `recipe_registry: &RecipeRegistry` — all available at the call site.
4. `derive_invalidation_conditions` (S31-002) returns `(Vec<ExhaustionInvalidationCondition>, ExhaustionBaseline)`.
5. The `.and_modify()` path must update both `invalidation_conditions` and `baseline` on re-exhaustion (not just `exhausted_at`), because the world state may have changed since the last exhaustion.
6. The `.or_insert()` path creates a new entry with `count: 0` and the derived conditions/baseline.
7. Goals that did NOT exhaust still get `.remove()` — clearing both skip state and backoff count (existing behavior preserved).

## Architecture Check

1. Minimal signature extension — 3 additional parameters, all already available at the call site.
2. No backward-compatibility concerns — existing entry creation is extended, not replaced.

## Verification Layers

1. Exhausted goals get conditions + baseline stored -> unit test
2. Re-exhausted goals get updated conditions + baseline -> unit test
3. Non-exhausted goals are removed from cache (existing behavior) -> unit test
4. Call site compiles with new parameters -> compilation
5. Existing golden tests pass -> `cargo test -p worldwake-ai`

## What to Change

### 1. Update `record_exhausted_goals` signature in `planning.rs`

Add parameters:
```rust
fn record_exhausted_goals(
    runtime: &mut AgentDecisionRuntime,
    plans: &[...],
    tick: Tick,
    agent: EntityId,
    view: &dyn GoalBeliefView,
    recipe_registry: &RecipeRegistry,
)
```

### 2. Update function body

On exhaustion (BudgetExhausted | FrontierExhausted):
- Call `derive_invalidation_conditions(&key.kind, agent, view, recipe_registry)`
- Store `conditions` and `baseline` in the entry (both `.and_modify()` and `.or_insert()` paths)

### 3. Update call site at `planning.rs:484`

Pass `agent`, `&view`, and `recipe_registry` to the updated function.

### 4. Update tests that call `record_exhausted_goals`

Tests at `planning.rs:802+` call `record_exhausted_goals` directly. Update to pass the new parameters (using a mock view).

## Files to Touch

- `crates/worldwake-ai/src/agent_tick/planning.rs` (modify — function signature + body + call site)
- `crates/worldwake-ai/src/agent_tick/tests.rs` (modify — update test calls if tests are in separate file)

## Out of Scope

- `invalidate_exhausted_goals` (S31-004 — may be done in parallel)
- Removing TTL (S31-006)
- Golden tests (S31-007)
- Changes to `build_candidate_plans` skip predicate (S31-006)

## Acceptance Criteria

### Tests That Must Pass

1. Unit test: exhausted goal stores non-empty `invalidation_conditions` and populated `baseline`
2. Unit test: re-exhausted goal updates `invalidation_conditions` and `baseline` (not just `exhausted_at`)
3. Unit test: non-exhausted goal is removed from cache entirely
4. Unit test: `count` increments on re-exhaustion via `.and_modify()`
5. Existing suite: `cargo test --workspace`

### Invariants

1. Every exhausted entry has non-empty `invalidation_conditions` (guaranteed by `derive_invalidation_conditions` Invariant 3 from S31-002)
2. Baseline captures current agent state at exhaustion time
3. Non-exhausted goals are fully removed from cache (existing behavior preserved)

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/agent_tick/planning.rs` tests — update `record_exhausted_goals_refreshes_tick_without_resetting_count` and `record_exhausted_goals_removes_only_successful_goal_entry` to pass new parameters and assert conditions/baseline are stored

### Commands

1. `cargo test -p worldwake-ai planning`
2. `cargo clippy --workspace && cargo test --workspace`
