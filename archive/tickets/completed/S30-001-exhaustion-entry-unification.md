# S30-001: Unify exhaustion cache into ExhaustionEntry struct

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — AgentDecisionRuntime field layout, planning exhaustion skip/backoff logic
**Deps**: None (standalone refactoring)

## Problem

`AgentDecisionRuntime` tracks goal exhaustion state across two separate maps: `search_exhausted_goals: BTreeMap<GoalKey, Tick>` and `exhaustion_counts: BTreeMap<GoalKey, u8>`. This duplication complicates serialization (S30-002) and blocks S31's plan to add `invalidation: Option<ExhaustionInvalidationCondition>` to each entry. Unifying into a single `ExhaustionEntry` struct is a prerequisite for clean serde derives and future extensibility.

## Assumption Reassessment (2026-03-27)

1. `AgentDecisionRuntime` still stores the live exhaustion state as two separate maps in [`crates/worldwake-ai/src/decision_runtime.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/decision_runtime.rs): `search_exhausted_goals: BTreeMap<GoalKey, Tick>` and `exhaustion_counts: BTreeMap<GoalKey, u8>`. The shared abstraction boundary under audit is the AI-runtime exhaustion cache contract between that runtime struct and the planning helpers in [`crates/worldwake-ai/src/agent_tick/planning.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/agent_tick/planning.rs).
2. The two-map contract is consumed at multiple live call sites, not only one search path. `build_candidate_plans` reads both maps for TTL skip and exponential backoff in [`crates/worldwake-ai/src/agent_tick/planning.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/agent_tick/planning.rs), `plan_and_validate_next_step` passes the runtime maps into that helper, and [`crates/worldwake-ai/src/agent_tick/active_action.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/agent_tick/active_action.rs) also calls `build_candidate_plans` on the interrupt path with explicit empty maps. The original ticket’s files-to-touch list was missing `active_action.rs`.
3. The maps are not just passive storage. `record_exhausted_goals` mutates the TTL skip set, `reset_exhausted_goals_if_needed` promotes skipped entries into cumulative backoff counts on dirty invalidation, and `plan_and_validate_next_step` clears counts only for goals that actually found plans. This is still a local AI-layer refactor, but it is behavior-bearing maintenance logic, not a field-layout-only change.
4. `GoalKey` already derives `Serialize`/`Deserialize` in [`crates/worldwake-core/src/goal.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/goal.rs), and `Tick` is already serializable, so S30-001 does not need transitive type work. The no-serde scope for `ExhaustionEntry` remains correct for this ticket; serde derives belong to S30-002 when `AgentDecisionRuntime` itself becomes serializable.
5. Existing focused coverage does not currently pin the exhaustion-cache maintenance contract. `cargo test -p worldwake-ai -- --list` confirms no named exhaustion-focused runtime tests today. The original “no new tests expected” assumption is incorrect; this ticket should add focused unit coverage for TTL recording/reset promotion and plan-success count clearing before the data model is changed.
6. The original `ExhaustionEntry { exhausted_at: Tick, count: u8 }` proposal is architecturally incomplete for the current behavior. Today the runtime can lawfully clear TTL skip while retaining and incrementing cumulative backoff state; a non-optional `Tick` cannot represent “not currently skipped, but still backoff-penalized” without sentinel hacks or reintroducing parallel state. The unified entry should therefore store `exhausted_at: Option<Tick>` and keep `count` alongside it.
7. The save/load golden tests in [`crates/worldwake-ai/tests/golden_determinism.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_determinism.rs) and the reset in [`crates/worldwake-ai/tests/golden_harness/mod.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_harness/mod.rs) remain future S30-002/S30-003 scope. This ticket should not claim save/load parity by itself; it only prepares the runtime cache shape for that later work.

## Architecture Check

1. A single `BTreeMap<GoalKey, ExhaustionEntry>` is cleaner than parallel maps because each goal’s exhaustion history becomes one explicit value object instead of an implicit cross-map invariant. That is the more robust long-term shape for S30 save/load work and S31 invalidation conditions.
2. This keeps the canonical exhaustion fact local to the AI runtime layer. The planning helpers read one cache, mutate one cache, and do not need alias fields or translation glue.
3. `exhausted_at: Option<Tick>` is cleaner than a mandatory `Tick` because it models the real state machine directly: some goals have cumulative exhaustion history without being inside the active TTL skip window. That avoids fake timestamps and keeps later S31 invalidation rules composable.
4. No backward-compatibility shims or alias maps are introduced. The old fields are removed outright and every caller is updated to the unified entry type.

## Verification Layers

1. Exhausted-goal TTL recording and invalidation promotion -> focused unit test on the planning helper layer
2. Successful replan clears only the selected goal’s cumulative exhaustion state -> focused unit/runtime test on `plan_and_validate_next_step`
3. Unified cache integration does not regress planner behavior -> `cargo test -p worldwake-ai`
4. Single-layer ticket: no additional action-trace or event-log mapping is required because the invariant under change is entirely inside the AI runtime/planning cache contract.

## What to Change

### 1. Define `ExhaustionEntry` struct in `decision_runtime.rs`

```rust
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct ExhaustionEntry {
    pub exhausted_at: Option<Tick>,
    pub count: u8,
}
```

Note: Do NOT add `Serialize, Deserialize` here. S30-001 changes the in-memory runtime contract only; serde derives are deferred to S30-002 when the full `AgentDecisionRuntime` gains save/load support.

### 2. Replace two fields with one in `AgentDecisionRuntime`

Remove:
- `search_exhausted_goals: BTreeMap<GoalKey, Tick>`
- `exhaustion_counts: BTreeMap<GoalKey, u8>`

Add:
- `exhaustion_cache: BTreeMap<GoalKey, ExhaustionEntry>`

### 3. Update `planning.rs` call sites

- `build_candidate_plans()`: Change parameters from `skip_exhausted: &BTreeMap<GoalKey, Tick>, exhaustion_counts: &BTreeMap<GoalKey, u8>` to `exhaustion_cache: &BTreeMap<GoalKey, ExhaustionEntry>`. TTL skip reads `entry.exhausted_at`; backoff reads `entry.count` regardless of whether the TTL skip is currently active.
- `record_exhausted_goals`: Update or prune unified entries in place. A budget/frontier exhausted result refreshes `exhausted_at = Some(tick)` while preserving the prior count; a searched non-exhausted result removes the unified entry because both the active skip state and the historical backoff state should clear on success for that exact goal.
- `reset_exhausted_goals_if_needed`: For entries whose `exhausted_at` is currently `Some`, increment `count` and clear only the TTL marker by setting `exhausted_at = None`. Do not drop the entry, because the cumulative backoff history is the canonical surviving fact after invalidation.
- `plan_and_validate_next_step`: When a plan is found, clear only the selected goal’s unified entry so chronically unsolved peers keep their backoff state.

### 4. Update non-runtime callers of `build_candidate_plans`

- `handle_active_action_phase` in `agent_tick/active_action.rs` currently passes explicit empty skip/count maps for interrupt evaluation. Update that call site to pass a single empty exhaustion cache value instead.

### 5. Add focused coverage for the unified cache contract

- Add a planning-layer test that proves exhausted entries preserve their prior `count`, refresh `exhausted_at`, and increment `count` while clearing only the TTL marker on invalidation.
- Add a focused runtime/planning test that proves a found plan removes only the successful goal’s exhaustion entry, leaving unrelated exhausted goals intact.

## Files to Touch

- `crates/worldwake-ai/src/decision_runtime.rs` (modify — define `ExhaustionEntry`, replace two fields)
- `crates/worldwake-ai/src/agent_tick/planning.rs` (modify — update unified exhaustion cache reads/writes and add focused tests)
- `crates/worldwake-ai/src/agent_tick/active_action.rs` (modify — update interrupt-path caller to pass unified cache)

## Out of Scope

- Adding `Serialize, Deserialize` to `ExhaustionEntry` or `AgentDecisionRuntime` (S30-002)
- Adding `invalidation` field to `ExhaustionEntry` (S31)
- Changing `EXHAUSTION_SKIP_TTL` value (S30-007)
- Save/load format changes (S30-003)
- Removing the golden harness driver reset or changing save/load tests (S30-002/S30-003)
- Any intentional behavioral change to exhaustion skip or backoff policy

## Acceptance Criteria

### Tests That Must Pass

1. New focused planning/runtime exhaustion tests pass and prove the unified cache preserves the pre-existing skip/backoff semantics
2. `cargo test -p worldwake-ai`
3. `cargo test --workspace`
4. `cargo clippy --workspace`

### Invariants

1. Observable behavior is identical — same goals are skipped, same backoff budgets are applied
2. `ExhaustionEntry` is the single canonical exhaustion record per `GoalKey`; no parallel map invariant remains
3. No new ECS components introduced
4. Determinism preserved — `BTreeMap` ordering unchanged

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/agent_tick/planning.rs` — add focused unit tests for exhaustion-entry refresh, invalidation promotion, and selective clearing so the refactor cannot silently change cache semantics.

### Commands

1. `cargo test -p worldwake-ai agent_tick::planning`
2. `cargo test -p worldwake-ai`
3. `cargo test --workspace`
4. `cargo clippy --workspace`

## Outcome

- Completed: 2026-03-27
- What actually changed:
  - Replaced `search_exhausted_goals` and `exhaustion_counts` with a single `exhaustion_cache: BTreeMap<GoalKey, ExhaustionEntry>` in `AgentDecisionRuntime`.
  - Added `ExhaustionEntry` and updated planning + interrupt call sites to use the unified cache.
  - Added focused planning-layer tests covering timestamp refresh, invalidation promotion, and selective clearing semantics.
- Deviations from original plan:
  - `ExhaustionEntry.exhausted_at` was implemented as `Option<Tick>`, not `Tick`. This was required to represent the real runtime state cleanly: a goal can retain cumulative backoff history after invalidation while no longer being in the active TTL skip window.
  - `agent_tick/active_action.rs` also required an update because the interrupt path calls `build_candidate_plans` directly.
- Verification results:
  - `cargo test -p worldwake-ai agent_tick::planning` passed.
  - `cargo test -p worldwake-ai` passed.
  - `cargo test --workspace` passed.
  - `cargo clippy --workspace` passed.
