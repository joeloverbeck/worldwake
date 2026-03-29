# S37COOBASEXH-003: Remove budget reduction in planning — use full budget on every retry

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: No new production-path changes; focused regression-proof coverage only
**Deps**: None (`S37COOBASEXH-002` already delivered the production architecture this ticket originally described)

## Problem

This ticket’s original implementation plan is stale. The live planner already uses cooldown-gated retry with the full configured `PlanningBudget::max_node_expansions` on retry attempts. The remaining gap is proof surface: there is not yet a focused `build_candidate_plans()` regression test that would fail if budget-halving were reintroduced for `BudgetRetryPending` entries.

## Assumption Reassessment (2026-03-29)

1. The exact boundary under audit is the retry contract between [`ExhaustionEntry`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/decision_runtime.rs) and [`build_candidate_plans()`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/agent_tick/planning.rs). This is planner-path work, even without a golden scenario.
2. The production change this ticket proposed is already present. [`build_candidate_plans()`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/agent_tick/planning.rs) now admits only frontier-unsuppressed and cooldown-eligible entries, then clones the full budget directly via `let effective_budget = budget.clone();`.
3. The old budget-halving helpers are already gone. [`ExhaustionEntry`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/decision_runtime.rs) has `next_retry_tick`, `consecutive_failures`, `is_retry_eligible()`, and `record_budget_exhaustion()`; it no longer exposes `effective_max_expansions()` or `is_budget_retry_pending()`.
4. [`PlanningBudget`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/budget.rs) already contains `initial_cooldown_ticks` and `max_cooldown_ticks`, so this ticket no longer has any configuration-schema work.
5. [`has_pending_budget_retry()`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/agent_tick/planning.rs) already accepts `current_tick` and only wakes planning for retry-eligible entries. The original split that deferred cooldown gating to later tickets is stale in the current codebase.
6. `cargo test -p worldwake-ai -- --list` confirms focused coverage already exists for cooldown eligibility, retry triggering, and sibling-admission behavior under `agent_tick::planning` and `decision_runtime`.
7. The remaining mismatch is test coverage, not production behavior. Existing focused tests prove cooldown gating and exhaustion recording, but none directly assert that a retry-eligible `BudgetRetryPending` entry still receives the caller’s full `max_node_expansions` budget inside `build_candidate_plans()`.
8. No adjacent architectural contradiction remains in production code. Reopening the implementation surface beyond a regression test would be churn, not cleanup.

## Architecture Check

1. The current architecture is better than the legacy design. Cooldown-based retry preserves planner competence and uses one explicit retry mechanism instead of encoding failure as hidden search degradation.
2. The clean move now is to keep production code unchanged and harden the proof surface. Reworking already-correct planner code would not improve extensibility or robustness.
3. No backward-compatibility shim is needed or wanted. The old budget-halving path is already removed.

## Verification Layers

1. Retry-eligible entries do not receive a reduced node-expansion budget -> focused unit test in [`crates/worldwake-ai/src/agent_tick/planning.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/agent_tick/planning.rs)
2. Cooldown gating still suppresses ineligible retries and admits eligible ones -> existing focused unit tests in [`crates/worldwake-ai/src/agent_tick/planning.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/agent_tick/planning.rs) and [`crates/worldwake-ai/src/decision_runtime.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/decision_runtime.rs)

## What to Change

### 1. Add a direct full-budget regression test

In [`crates/worldwake-ai/src/agent_tick/planning.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/agent_tick/planning.rs), add a focused unit test that:

- seeds a retry-eligible `BudgetRetryPending` exhaustion entry,
- calls `build_candidate_plans()` with a deliberately tiny custom `max_node_expansions`,
- and proves the search result consumed that exact configured budget rather than a reduced retry budget.

### 2. Archive this ticket once verification is complete

After the focused test and verification commands pass, mark this ticket completed and archive it with the actual outcome instead of the stale original implementation plan.

## Files to Touch

- `crates/worldwake-ai/src/agent_tick/planning.rs`
- `tickets/S37COOBASEXH-003-remove-budget-reduction-use-full-budget.md`

## Out of Scope

- Any further production changes to cooldown retry behavior
- `ExhaustionEntry` schema changes
- `PlanningBudget` schema changes
- Decision trace or save/load changes
- Golden test changes

## Acceptance Criteria

### Tests That Must Pass

1. A focused `build_candidate_plans()` test fails if retry entries receive less than the configured `max_node_expansions`
2. The current `worldwake-ai` focused planning tests pass
3. `cargo test -p worldwake-ai` passes

### Invariants

1. Retry-eligible planning attempts use the caller’s full `PlanningBudget::max_node_expansions`
2. Cooldown gating remains the only retry-throttling mechanism
3. No production-path API or behavior changes are introduced beyond regression-proof coverage

## Test Plan

### New/Modified Tests

1. [`crates/worldwake-ai/src/agent_tick/planning.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/agent_tick/planning.rs) — add a focused regression test proving full configured node-expansion budget is preserved for retry-eligible `BudgetRetryPending` entries

### Commands

1. `cargo test -p worldwake-ai -- build_candidate_plans_uses_full_budget_for_retry_eligible_exhaustion_entry`
2. `cargo test -p worldwake-ai -- planning`
3. `cargo test -p worldwake-ai`
4. `cargo clippy --workspace`

## Outcome

- Completion date: 2026-03-29
- What actually changed: reassessed the ticket against live `worldwake-ai` code, corrected the ticket scope to reflect that the cooldown-based full-budget architecture was already delivered, and added one focused regression test proving retry-eligible `BudgetRetryPending` entries still use the caller’s exact `max_node_expansions` budget inside `build_candidate_plans()`.
- Deviations from original plan: no production-path planner code changed because the originally proposed implementation was already present in [`crates/worldwake-ai/src/agent_tick/planning.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/agent_tick/planning.rs) and [`crates/worldwake-ai/src/decision_runtime.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/decision_runtime.rs). The real remaining work was proof-surface hardening plus ticket correction.
- Verification results: `cargo test -p worldwake-ai -- build_candidate_plans_uses_full_budget_for_retry_eligible_exhaustion_entry`, `cargo test -p worldwake-ai -- planning`, `cargo test -p worldwake-ai`, and `cargo clippy --workspace` all passed.
