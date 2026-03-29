# S37COOBASEXH-004: Cooldown-aware planning trigger and candidate filtering

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — has_pending_budget_retry signature, candidate admission filter
**Deps**: S37COOBASEXH-002 (ExhaustionEntry has is_retry_eligible)

## Problem

`has_pending_budget_retry()` unconditionally returns true for any `BudgetRetryPending` entry regardless of tick timing, causing agents to enter the planning loop every tick even when all exhausted goals are in cooldown. The candidate filter in `build_candidate_plans()` only checks `suppresses_planning()` (frontier-exhausted), not cooldown eligibility. Both must become cooldown-aware.

## Assumption Reassessment (2026-03-29)

1. `has_pending_budget_retry()` defined at `crates/worldwake-ai/src/agent_tick/planning.rs:435-440`. Called at lines 509 and 664 (both `should_plan` computations). Neither call site currently passes `current_tick`. The candidate filter at lines 195-206 only checks `ExhaustionEntry::suppresses_planning`.
2. Spec S37 Section 4 adds `current_tick: Tick` param to `has_pending_budget_retry`. Section 5 adds a second filter arm for `!entry.is_retry_eligible(current_tick)`.
3. Both call sites at 509 and 664 have `current_tick` in scope (it's a parameter of their enclosing functions). `build_candidate_plans` also has `current_tick: Tick` in its signature (line 184).
4. N/A — no golden scenario.
5. N/A — not directly planner-driven.
6. N/A — not an AI regression.
7. N/A — no ordering dependency.
8. N/A — no heuristic removal (adding a filter, not removing one).
9. N/A — not a stale-request ticket.
10-12. N/A.
13. No adjacent contradictions.
14. No mismatch with spec.
15. N/A.

## Architecture Check

1. Adding `current_tick` to `has_pending_budget_retry` is a minimal signature change. The candidate filter extension follows the existing `suppresses_planning` pattern with an additional arm. Both changes are local.
2. No backward-compatibility shims.

## Verification Layers

1. `has_pending_budget_retry` returns false when all entries in cooldown → focused unit test
2. `has_pending_budget_retry` returns true when at least one entry eligible → focused unit test
3. Candidate filter skips non-eligible entries → focused unit test via `build_candidate_plans`
4. `FrontierExhausted` still filtered by `suppresses_planning` (unchanged) → existing test coverage
5. Single-layer ticket: planning trigger + candidate filtering. Integration via golden tests after full chain.

## What to Change

### 1. Update `has_pending_budget_retry` signature

In `crates/worldwake-ai/src/agent_tick/planning.rs`, change:

```rust
fn has_pending_budget_retry(runtime: &AgentDecisionRuntime, current_tick: Tick) -> bool {
    runtime
        .exhaustion_cache
        .values()
        .any(|entry| entry.is_retry_eligible(current_tick))
}
```

### 2. Update call sites

At line 509:
```rust
let should_plan = !runtime.dirty.is_empty() || has_pending_budget_retry(runtime, current_tick);
```

At line 664:
```rust
let should_plan = !runtime.dirty.is_empty() || has_pending_budget_retry(runtime, current_tick);
```

Both enclosing functions already have `current_tick` in scope.

### 3. Update candidate filter in `build_candidate_plans`

At lines 195-206, extend the filter:

```rust
let admitted_candidates: Vec<_> = ranked_candidates
    .iter()
    .filter(|c| {
        let key = OpportunityKey {
            goal_key: c.grounded.key,
            anchor: c.grounded.anchor,
        };
        match exhaustion_cache.get(&key) {
            Some(entry) if entry.suppresses_planning() => false,
            Some(entry) if !entry.is_retry_eligible(current_tick) => false,
            _ => true,
        }
    })
    .collect();
```

### 4. Update tests

Update `has_pending_budget_retry_detects_retryable_budget_entries` (line 1948) and related tests to pass `current_tick`. Add tests for cooldown-aware behavior.

## Files to Touch

- `crates/worldwake-ai/src/agent_tick/planning.rs` (modify)

## Out of Scope

- `ExhaustionEntry` struct or method changes (S37COOBASEXH-002)
- Budget reduction removal (S37COOBASEXH-003)
- Exhaustion recording changes (S37COOBASEXH-005)
- Decision trace changes (S37COOBASEXH-006)
- Save/load (S37COOBASEXH-007)
- `PlanningBudget` struct (S37COOBASEXH-001)
- Golden test adjustments
- `invalidate_exhausted_goals` (unchanged per spec Section 7)

## Acceptance Criteria

### Tests That Must Pass

1. `has_pending_budget_retry` returns false when all exhaustion entries have `next_retry_tick` in the future
2. `has_pending_budget_retry` returns true when at least one entry is retry-eligible at `current_tick`
3. Candidate filter skips `BudgetRetryPending` entries where `!is_retry_eligible(current_tick)`
4. Candidate filter still admits entries with no exhaustion cache entry (fresh goals)
5. Candidate filter still blocks `FrontierExhausted` entries
6. Existing suite: `cargo test -p worldwake-ai -- planning`

### Invariants

1. Agents do not enter the planning loop when all exhausted goals are in cooldown
2. `FrontierExhausted` entries continue to suppress planning entirely (unchanged)
3. Fresh goals (no exhaustion entry) always admitted

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/agent_tick/planning.rs::has_pending_budget_retry_detects_retryable_budget_entries` — updated with `current_tick` param
2. New test: `has_pending_budget_retry_returns_false_when_all_in_cooldown`
3. New test: `candidate_filter_skips_non_eligible_cooldown_entries`

### Commands

1. `cargo test -p worldwake-ai -- has_pending_budget_retry`
2. `cargo test -p worldwake-ai -- build_candidate`
3. `cargo clippy --workspace && cargo test -p worldwake-ai`
