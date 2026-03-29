# S37COOBASEXH-004: Cooldown-aware planning trigger and candidate filtering

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: No new production-path changes; focused regression-proof coverage only
**Deps**: None (`S37COOBASEXH-002` and `S37COOBASEXH-003` already delivered the production architecture this ticket originally described)

## Problem

This ticket’s original implementation plan is stale. The live planner already gates retry-triggered replanning on cooldown eligibility and already filters cooldown-ineligible exhausted opportunities out of candidate admission. The remaining work is to correct the ticket, verify the live assumptions against the code and tests, and strengthen the proof surface with one direct candidate-filter regression test.

## Assumption Reassessment (2026-03-29)

1. The exact shared boundary under audit is the exhaustion-retry contract between [`ExhaustionEntry::is_retry_eligible()`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/decision_runtime.rs), [`has_pending_budget_retry()`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/agent_tick/planning.rs), and [`build_candidate_plans()`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/agent_tick/planning.rs). This is planner-path work without a golden scenario.
2. The production change this ticket proposed is already live. [`has_pending_budget_retry()`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/agent_tick/planning.rs) already accepts `current_tick` and returns true only when some exhaustion entry is retry-eligible at that tick.
3. Both active planning entry points already use the cooldown-aware trigger. [`plan_and_validate_next_step()`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/agent_tick/planning.rs) and [`plan_and_validate_next_step_traced()`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/agent_tick/planning.rs) compute `should_plan` with `has_pending_budget_retry(runtime, tick)`.
4. [`build_candidate_plans()`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/agent_tick/planning.rs) already rejects both `FrontierExhausted` entries and `BudgetRetryPending` entries whose cooldown has not elapsed. The same admission rule is mirrored in [`summarize_same_goal_planning_trace()`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/agent_tick/planning.rs), so planning and trace summarization are consistent.
5. [`ExhaustionEntry`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/decision_runtime.rs) already carries `next_retry_tick` and `consecutive_failures`, exposes `is_retry_eligible()` and `suppresses_planning()`, and no longer exposes the old unconditional retry shape this ticket described.
6. [`PlanningBudget`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/budget.rs) already carries `initial_cooldown_ticks` and `max_cooldown_ticks`, so this ticket no longer owns schema work.
7. `cargo test -p worldwake-ai -- --list` confirms focused coverage already exists for retry eligibility and cooldown behavior: `agent_tick::planning::tests::has_pending_budget_retry_detects_retryable_budget_entries`, `agent_tick::planning::tests::cooldown_ineligible_entry_does_not_block_later_same_goal_sibling`, `agent_tick::planning::tests::frontier_exhaustion_suppresses_planning_but_budget_retry_does_not`, `agent_tick::planning::tests::build_candidate_plans_uses_full_budget_for_retry_eligible_exhaustion_entry`, and `decision_runtime::tests::retry_eligibility_respects_retry_tick_and_frontier_suppression`.
8. The remaining mismatch is proof surface, not architecture. The existing sibling test proves a cooldown-ineligible opportunity does not suppress a later same-goal sibling, but there is not yet a direct focused assertion that a lone cooldown-ineligible retry entry is filtered out of candidate admission entirely.
9. No adjacent architectural contradiction remains in production code. Reopening the planner implementation here would be churn, not cleanup.

## Architecture Check

1. The current architecture is stronger than the original ticket narrative. Cooldown-gated retry keeps retry timing explicit in the exhaustion state instead of smearing retry behavior across unconditional trigger wakeups and admission-time surprises.
2. Leaving the production planner unchanged is the cleaner move. The architecture already satisfies the spec intent: frontier exhaustion remains a hard suppression path, budget exhaustion becomes a timed retry path, and both the trigger and candidate gate read the same retry contract.
3. The only architectural caveat worth noting is duplication of the admission predicate between [`build_candidate_plans()`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/agent_tick/planning.rs) and [`summarize_same_goal_planning_trace()`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/agent_tick/planning.rs). That is a small DRY follow-up candidate, not a blocker for this ticket, because the two sites are currently behaviorally aligned and covered.
4. No backward-compatibility shim is needed or desired. The old unconditional retry behavior is already gone.

## Verification Layers

1. Retry trigger stays asleep while all budget-retry entries are still cooling down -> focused unit test of [`has_pending_budget_retry()`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/agent_tick/planning.rs)
2. Candidate admission skips cooldown-ineligible exhausted opportunities -> focused unit tests in [`crates/worldwake-ai/src/agent_tick/planning.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/agent_tick/planning.rs)
3. Frontier exhaustion still suppresses planning while budget retry remains cooldown-gated rather than permanently suppressed -> focused unit tests across [`crates/worldwake-ai/src/agent_tick/planning.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/agent_tick/planning.rs) and [`crates/worldwake-ai/src/decision_runtime.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/decision_runtime.rs)

## What to Change

### 1. Add one direct cooldown-filter regression test

In [`crates/worldwake-ai/src/agent_tick/planning.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/agent_tick/planning.rs), add a focused unit test that:

- seeds a single ranked candidate,
- attaches a `BudgetRetryPending` exhaustion entry whose `next_retry_tick` is still in the future,
- calls `build_candidate_plans()` at an earlier tick,
- and asserts that the result is empty because the candidate is filtered out before search.

### 2. Archive this ticket after verification

Once the focused regression test and verification commands pass, mark this ticket completed and archive it with the actual outcome instead of the stale implementation narrative.

## Files to Touch

- `crates/worldwake-ai/src/agent_tick/planning.rs`
- `tickets/S37COOBASEXH-004-cooldown-aware-planning-trigger-and-candidate-filter.md`

## Out of Scope

- Any further production changes to cooldown retry behavior
- `ExhaustionEntry` schema changes
- `PlanningBudget` schema changes
- Exhaustion recording changes beyond current live behavior
- Decision trace or save/load changes
- Golden test changes

## Acceptance Criteria

### Tests That Must Pass

1. A focused `build_candidate_plans()` regression test proves a cooldown-ineligible `BudgetRetryPending` opportunity is filtered out before search
2. Focused planning tests covering retry trigger and candidate filtering pass
3. `cargo test -p worldwake-ai` passes
4. `cargo clippy --workspace` passes

### Invariants

1. Agents do not enter retry-driven planning when every exhausted retry entry is still in cooldown
2. Cooldown-ineligible `BudgetRetryPending` opportunities are not admitted into candidate search
3. `FrontierExhausted` entries remain hard-suppressed until invalidation

## Test Plan

### New/Modified Tests

1. [`crates/worldwake-ai/src/agent_tick/planning.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/agent_tick/planning.rs) — add a direct regression test proving a lone cooldown-ineligible retry opportunity is filtered out of `build_candidate_plans()`

### Commands

1. `cargo test -p worldwake-ai -- cooldown_ineligible_entry_is_filtered_out_of_candidate_plans`
2. `cargo test -p worldwake-ai -- planning`
3. `cargo test -p worldwake-ai`
4. `cargo clippy --workspace`

## Outcome

- Completion date: 2026-03-29
- What actually changed: reassessed the ticket against live `worldwake-ai` code, corrected the ticket scope to reflect that cooldown-aware retry triggering and candidate filtering were already implemented, and added one focused regression test proving a lone cooldown-ineligible `BudgetRetryPending` opportunity is filtered out of `build_candidate_plans()` before search.
- Deviations from original plan: no production-path planner code changed because the ticket’s proposed implementation was already present in [`crates/worldwake-ai/src/agent_tick/planning.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/agent_tick/planning.rs), [`crates/worldwake-ai/src/decision_runtime.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/decision_runtime.rs), and [`crates/worldwake-ai/src/budget.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/budget.rs). The real remaining work was ticket correction plus one direct proof-surface hardening test.
- Verification results: `cargo test -p worldwake-ai -- cooldown_ineligible_entry_is_filtered_out_of_candidate_plans`, `cargo test -p worldwake-ai -- planning`, `cargo test -p worldwake-ai`, and `cargo clippy --workspace` all passed.
