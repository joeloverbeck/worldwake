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
8. Reassessment after implementation of S31-002: this ticket should not assume `count` increments during `.and_modify()` on a repeated exhaustion record. The live semantics treat `count` as backoff history across invalidation cycles, not as "number of times `record_exhausted_goals` refreshed the same active skip window". Refreshing `exhausted_at` while a goal is still exhausted should preserve `count` unless a different architecture is chosen explicitly.
9. This is the right remaining S31 ticket to absorb the stale persistence contract cleanup. `ExhaustionEntry` in [`crates/worldwake-ai/src/decision_runtime.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/decision_runtime.rs) still uses `#[serde(default)]` on `invalidation_conditions` and `baseline`, which is a backward-compatibility path inconsistent with repo policy and the live save format (`SAVE_FORMAT_VERSION = 7`).

## Architecture Check

1. Minimal signature extension — 3 additional parameters, all already available at the call site.
2. Existing entry creation is extended, not replaced.
3. This ticket should explicitly remove the stale backward-compatibility fallback from `ExhaustionEntry` while it is already tightening the "every exhausted entry records conditions and baseline" contract. That keeps the persisted runtime shape honest instead of perpetuating an empty-condition side path that later tickets then have to special-case.

## Verification Layers

1. Exhausted goals get conditions + baseline stored -> unit test
2. Re-exhausted goals get updated conditions + baseline -> unit test
3. Non-exhausted goals are removed from cache (existing behavior) -> unit test
4. Re-recorded active exhaustion preserves `count` while refreshing `exhausted_at` -> unit test
5. Call site compiles with new parameters -> compilation
6. Runtime serialization still round-trips fully populated exhaustion entries under the current save format -> focused runtime test
7. Existing golden tests pass -> `cargo test -p worldwake-ai`

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

### 4. Remove stale serde-default fallback from `ExhaustionEntry`

In [`crates/worldwake-ai/src/decision_runtime.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/decision_runtime.rs), remove `#[serde(default)]` from:
- `invalidation_conditions`
- `baseline`

S31 is redefining the persisted runtime contract, not preserving the pre-S31 shape. If that changes any round-trip fixtures or helper constructors, update them instead of retaining an empty-condition alias path.

### 5. Update tests that call `record_exhausted_goals`

Tests at `planning.rs:802+` call `record_exhausted_goals` directly. Update to pass the new parameters (using a mock view).

## Files to Touch

- `crates/worldwake-ai/src/agent_tick/planning.rs` (modify — function signature + body + call site)
- `crates/worldwake-ai/src/decision_runtime.rs` (modify — remove stale serde defaults on `ExhaustionEntry`)
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
4. Unit test: `count` is preserved on re-exhaustion via `.and_modify()`
5. Focused runtime serialization test still round-trips populated exhaustion entries after removing serde defaults
6. Existing suite: `cargo test --workspace`

### Invariants

1. Every exhausted entry has non-empty `invalidation_conditions` (guaranteed by `derive_invalidation_conditions` Invariant 3 from S31-002)
2. Baseline captures current agent state at exhaustion time
3. Non-exhausted goals are fully removed from cache (existing behavior preserved)
4. Refreshing an already-exhausted entry does not silently rewrite backoff semantics
5. Persisted S31 exhaustion entries always serialize and deserialize with explicit conditions and baseline data; no empty-condition compatibility path remains

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/agent_tick/planning.rs` tests — update `record_exhausted_goals_refreshes_tick_without_resetting_count` and `record_exhausted_goals_removes_only_successful_goal_entry` to pass new parameters and assert conditions/baseline are stored
2. `crates/worldwake-ai/src/agent_tick/tests.rs` or `decision_runtime.rs` tests — assert current-format runtime serialization round-trips populated `ExhaustionEntry` values after removing serde defaults

### Commands

1. `cargo test -p worldwake-ai planning`
2. `cargo clippy --workspace && cargo test --workspace`
