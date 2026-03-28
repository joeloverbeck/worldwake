# S33OPPSCOGOAIDE-004: Re-key exhaustion cache from GoalKey to OpportunityKey

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — exhaustion_cache key type change in AgentDecisionRuntime
**Deps**: S33OPPSCOGOAIDE-001

## Problem

`AgentDecisionRuntime.exhaustion_cache` is currently `BTreeMap<GoalKey, ExhaustionEntry>`. Exhausting search for one source (e.g., orchard) suppresses planning for ALL sources of the same commodity. The cache must be re-keyed to `BTreeMap<OpportunityKey, ExhaustionEntry>` so that exhaustion scopes to individual opportunities.

## Assumption Reassessment (2026-03-28)

1. `exhaustion_cache` is declared at `crates/worldwake-ai/src/decision_runtime.rs` as `BTreeMap<GoalKey, ExhaustionEntry>` on `AgentDecisionRuntime`. Confirmed.
2. `ExhaustionEntry` struct at `decision_runtime.rs:64-74` with fields `{ retry_state, invalidation_conditions, baseline, consecutive_budget_exhaustions }`. Confirmed — struct itself does not change.
3. `record_exhausted_goals()` at `crates/worldwake-ai/src/agent_tick/planning.rs:306-350` iterates `plans: &[(GoalKey, PlanSearchResult, ...)]` and calls `runtime.exhaustion_cache.insert(*key, entry)`. The tuple key type must change to `OpportunityKey`.
4. `invalidate_exhausted_goals()` — need to locate. It operates per-GoalKey and checks invalidation conditions. Must change to per-OpportunityKey iteration.
5. `build_candidate_plans()` at `planning.rs:146-175` checks `exhaustion_cache.get(&c.grounded.key)` — must change to check by `OpportunityKey`.
6. `has_pending_budget_retry()` at `planning.rs:352-357` iterates values only — no key type dependency.
7. `suppresses_planning()` at `decision_runtime.rs:112` — method on `ExhaustionEntry`, no change needed.
8. The S31 invalidation condition semantics (PositionChanged, CommodityChanged, etc.) remain unchanged — only the key granularity changes.

## Architecture Check

1. Re-keying the cache is the minimal change. The alternative — adding a secondary index by OpportunityKey alongside the GoalKey index — violates P26 (no backward compatibility layers) and doubles storage.
2. No backward-compatibility shims — all `GoalKey`-based exhaustion lookups are replaced with `OpportunityKey`-based lookups.

## Verification Layers

1. Exhaustion scoped per-opportunity → focused unit test: exhaust orchard, market remains plannable.
2. Invalidation fires per-opportunity → focused unit test: PositionChanged invalidates the specific OpportunityKey entry.
3. Budget backoff tracks per-opportunity → focused unit test: consecutive_budget_exhaustions increments per-OpportunityKey.

## What to Change

### 1. Change `exhaustion_cache` type in `AgentDecisionRuntime`

From `BTreeMap<GoalKey, ExhaustionEntry>` to `BTreeMap<OpportunityKey, ExhaustionEntry>`.

### 2. Update `record_exhausted_goals()`

Change the plans tuple from `(GoalKey, PlanSearchResult, ...)` to `(OpportunityKey, PlanSearchResult, ...)`. Insert entries under the `OpportunityKey`.

### 3. Update `build_candidate_plans()` exhaustion filter

Change `exhaustion_cache.get(&c.grounded.key)` to construct an `OpportunityKey` from `c.grounded.key` + `c.grounded.anchor` and look up by that.

### 4. Update `invalidate_exhausted_goals()`

Iterate over `BTreeMap<OpportunityKey, ExhaustionEntry>`. Invalidation condition checks may need to consider the anchor's entity for dead-entity checks. The condition semantics (PositionChanged, CommodityChanged, etc.) remain the same.

### 5. Update return type of `build_candidate_plans()`

The return type `Vec<(GoalKey, PlanSearchResult, ...)>` changes to `Vec<(OpportunityKey, PlanSearchResult, ...)>` so that `record_exhausted_goals` receives the opportunity key.

## Files to Touch

- `crates/worldwake-ai/src/decision_runtime.rs` (modify — change `exhaustion_cache` type)
- `crates/worldwake-ai/src/agent_tick/planning.rs` (modify — `record_exhausted_goals`, `build_candidate_plans` return type and exhaustion filter, `invalidate_exhausted_goals`)
- Any other files that read `exhaustion_cache` (search for `exhaustion_cache` usage across worldwake-ai)

## Out of Scope

- `GroundedGoal` struct changes (S33OPPSCOGOAIDE-002)
- Two-pass candidate filtering (S33OPPSCOGOAIDE-003)
- Post-rank dedup (S33OPPSCOGOAIDE-005)
- `PlannedPlan.opportunity` field (S33OPPSCOGOAIDE-006)
- Save/load version bump (S33OPPSCOGOAIDE-008)
- Changes to `ExhaustionEntry` struct fields
- Changes to invalidation condition semantics from S31

## Acceptance Criteria

### Tests That Must Pass

1. Exhausting `OpportunityKey { AcquireCommodity(Apple), Place(orchard) }` leaves `OpportunityKey { AcquireCommodity(Apple), Place(market) }` plannable.
2. `is_exhausted` / `suppresses_planning` checks the specific `OpportunityKey`, not the bare `GoalKey`.
3. Invalidation conditions fire per-opportunity and clear the specific `OpportunityKey` entry.
4. Consecutive budget exhaustion counter tracks per-`OpportunityKey`.
5. `has_pending_budget_retry()` still correctly detects pending retries.
6. Existing suite: `cargo test -p worldwake-ai`
7. Existing suite: `cargo clippy --workspace`

### Invariants

1. No `BTreeMap<GoalKey, ExhaustionEntry>` remains in authoritative runtime state.
2. `ExhaustionEntry` struct is unchanged (only the key changes).
3. S31 invalidation condition semantics are preserved — same conditions, scoped to opportunity.
4. Determinism: `BTreeMap<OpportunityKey, ExhaustionEntry>` iterates deterministically (Ord on OpportunityKey).

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/agent_tick/planning.rs` — `test_exhaustion_per_opportunity` — exhaust one anchor, other remains plannable.
2. `crates/worldwake-ai/src/agent_tick/planning.rs` — `test_invalidation_per_opportunity` — condition fires for specific OpportunityKey only.
3. `crates/worldwake-ai/src/agent_tick/planning.rs` — `test_budget_backoff_per_opportunity` — consecutive counter scopes to OpportunityKey.
4. Existing exhaustion tests updated to use `OpportunityKey`.

### Commands

1. `cargo test -p worldwake-ai -- exhaustion`
2. `cargo test -p worldwake-ai -- planning`
3. `cargo clippy --workspace && cargo test --workspace`
