# S37COOBASEXH-005: Cooldown-aware exhaustion recording

**Status**: PENDING
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — record_exhausted_goals function signature and body
**Deps**: S37COOBASEXH-001 (PlanningBudget cooldown fields), S37COOBASEXH-002 (ExhaustionEntry cooldown methods)

## Problem

`record_exhausted_goals()` manually tracks `consecutive_budget_exhaustions` and creates `ExhaustionEntry` via the old factory without cooldown parameters. It needs to use the new `record_budget_exhaustion()` method and pass `tick` + `budget` to set cooldown timing.

## Assumption Reassessment (2026-03-29)

1. `record_exhausted_goals()` defined at `crates/worldwake-ai/src/agent_tick/planning.rs:388-433`. Currently takes `_tick: Tick` (unused — line 394). Budget reduction logic at lines 406-417: manually reads `consecutive_budget_exhaustions` from previous entry, creates new entry, sets counter.
2. Spec S37 Section 6 specifies: (a) rename `_tick` to `tick`, (b) add `budget: &PlanningBudget` parameter, (c) for `BudgetExhausted` results, clone existing entry and call `record_budget_exhaustion(tick, budget)` instead of manual counter management, (d) for new entries, use updated `budget_retry_pending(conditions, baseline, tick, budget)` factory.
3. This function is called from within the planning pipeline. Need to verify call sites pass `budget`.
4. N/A — no golden scenario.
5. N/A — not directly planner-driven.
6. N/A — not an AI regression.
7. N/A — no ordering dependency.
8. N/A — no heuristic removal.
9. N/A — not a stale-request ticket.
10-12. N/A.
13. No adjacent contradictions.
14. No mismatch with spec.
15. Cooldown arithmetic delegated to `ExhaustionEntry::record_budget_exhaustion()` (validated in S37COOBASEXH-002).

## Architecture Check

1. Encapsulating cooldown computation inside `ExhaustionEntry::record_budget_exhaustion()` instead of manual counter management in the caller is cleaner. The recording function becomes a simple delegator.
2. No backward-compatibility shims.

## Verification Layers

1. `record_exhausted_goals` with `BudgetExhausted` result → entry has correct `next_retry_tick` → focused unit test
2. Consecutive budget exhaustions on same opportunity → cooldown doubles → focused unit test
3. `FrontierExhausted` recording unchanged → existing tests
4. Found plan removes exhaustion entry → existing tests
5. Single-layer: exhaustion recording logic. Integration tested by golden tests.

## What to Change

### 1. Update `record_exhausted_goals` signature

In `crates/worldwake-ai/src/agent_tick/planning.rs`, line 388:

- Rename `_tick: Tick` to `tick: Tick`
- Add `budget: &PlanningBudget` parameter

### 2. Replace budget exhaustion recording logic

Replace lines 406-417 (the `prev_count` + manual counter block) with:

```rust
crate::PlanSearchResult::BudgetExhausted { .. } => {
    match runtime.exhaustion_cache.get(&plan.opportunity) {
        Some(existing) if existing.retry_state == ExhaustionRetryState::BudgetRetryPending => {
            let mut e = existing.clone();
            e.invalidation_conditions = invalidation_conditions;
            e.baseline = baseline;
            e.record_budget_exhaustion(tick, budget);
            e
        }
        _ => ExhaustionEntry::budget_retry_pending(
            invalidation_conditions, baseline, tick, budget,
        ),
    }
}
```

### 3. Update call site(s)

Find where `record_exhausted_goals` is called and pass `budget` and ensure `tick` (not `_tick`) is forwarded. The call site already has `budget` in scope from the planning pipeline context.

### 4. Update tests

Update tests for `record_exhausted_goals` to use the new signature and verify cooldown state.

## Files to Touch

- `crates/worldwake-ai/src/agent_tick/planning.rs` (modify)

## Out of Scope

- `ExhaustionEntry` struct/method changes (S37COOBASEXH-002)
- `PlanningBudget` struct changes (S37COOBASEXH-001)
- Budget reduction removal (S37COOBASEXH-003)
- Planning trigger / candidate filter (S37COOBASEXH-004)
- Decision trace (S37COOBASEXH-006)
- Save/load (S37COOBASEXH-007)
- `invalidate_exhausted_goals` (unchanged per spec)
- Golden test adjustments

## Acceptance Criteria

### Tests That Must Pass

1. Recording a `BudgetExhausted` result for a fresh opportunity creates entry with `next_retry_tick = current_tick + initial_cooldown_ticks`
2. Recording a second `BudgetExhausted` on the same opportunity preserves `consecutive_failures` count and doubles cooldown
3. Recording a `FrontierExhausted` result creates entry with `consecutive_failures: 0` (unchanged behavior)
4. Recording a `Found` result removes the exhaustion entry (unchanged behavior)
5. Existing suite: `cargo test -p worldwake-ai -- record_exhausted`

### Invariants

1. Cooldown computation delegated to `ExhaustionEntry::record_budget_exhaustion()` — no manual counter management in `record_exhausted_goals`
2. `FrontierExhausted` recording unchanged
3. Successful plan finding still clears exhaustion entry

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/agent_tick/planning.rs` — update existing `record_exhausted_goals` tests to use new signature
2. New test: `record_exhausted_goals_sets_cooldown_on_budget_exhaustion`
3. New test: `record_exhausted_goals_doubles_cooldown_on_consecutive_failure`

### Commands

1. `cargo test -p worldwake-ai -- record_exhausted`
2. `cargo clippy --workspace && cargo test -p worldwake-ai`
