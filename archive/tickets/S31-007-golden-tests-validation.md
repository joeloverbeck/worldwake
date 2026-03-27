# S31-007: Golden Tests for Exhaustion Invalidation End-State

**Status**: ✅ COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Tests only
**Deps**: S31-008, S31-009

## Problem

S31 production work is already complete, but the final validation ticket still describes a pre-S31-009 world where TTL removal, retry-state meaning, and save/load shape were unsettled. This ticket must be corrected to validate the live architecture that actually shipped: goal-aware invalidation plus explicit retry-state separation between `FrontierExhausted` and `BudgetRetryPending`.

## Assumption Reassessment (2026-03-27)

1. The exact shared abstraction boundary under audit is the persisted planner retry contract between [`crates/worldwake-ai/src/decision_runtime.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/decision_runtime.rs), [`crates/worldwake-ai/src/exhaustion.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/exhaustion.rs), and [`crates/worldwake-ai/src/agent_tick/planning.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/agent_tick/planning.rs). `ExhaustionEntry` now stores `retry_state`, `invalidation_conditions`, and `baseline`; the old TTL/count narrative no longer matches live code.
2. Archived [`S31-008`](/home/joeloverbeck/projects/worldwake/archive/tickets/S31-008-complete-exhaustion-invalidation-for-needs-driven-goals.md) and [`S31-009`](/home/joeloverbeck/projects/worldwake/archive/tickets/S31-009-separate-budget-exhaustion-retry-from-invalidation.md) already landed the production architecture this ticket originally treated as unfinished:
   - needs-driven invalidation is threshold-band based via `NeedChangedBands`
   - `FrontierExhausted` suppresses planning until concrete invalidation fires
   - `BudgetRetryPending` remains retryable on the next compatible pass
   - `EXHAUSTION_SKIP_TTL` is gone
3. `SAVE_FORMAT_VERSION` is now `9` in [`crates/worldwake-sim/src/save_load.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/save_load.rs), not `7` or `8`.
4. The live S31 focused proof surface is already stronger than this ticket claimed when it was written:
   - `exhaustion::tests::invalidate_exhausted_goals_removes_only_entries_with_fired_conditions`
   - `agent_tick::planning::tests::frontier_exhaustion_suppresses_planning_but_budget_retry_does_not`
   - `agent_tick::tests::irrelevant_commodity_change_does_not_trigger_replan_for_sleep_goal`
   - `agent_tick::tests::relevant_commodity_change_triggers_replan_for_consume_goal`
5. The named golden regressions still exist exactly as claimed and currently pass:
   - `golden_goal_invalidation_by_another_agent`
   - `golden_wash_action`
   - `golden_three_way_need_competition`
   - `golden_utility_weight_diversity_in_need_selection`
   - `golden_save_load_round_trip_under_ai`
6. The main remaining discrepancy is proof strength, not missing production substrate. `golden_goal_invalidation_by_another_agent` currently proves a downstream outcome, but it does not prove the earlier retry/invalidation boundary as directly as `docs/golden-e2e-testing.md` recommends.
7. The intended invariant for this ticket is now precise: unrelated commodity changes must not clear unrelated frontier-exhausted goals, while the live needs-driven and budget-retry scenarios must continue to pass under the already-landed S31 architecture.
8. No adjacent production contradiction remains in scope. If validation passes after strengthening the weak golden surface, this ticket should close and be archived alongside the S31 goal spec.

## Architecture Check

1. The cleanest path is to validate the landed architecture rather than inventing another test-only abstraction or resurrecting the stale TTL narrative.
2. Strengthening the weak golden at the earliest causal boundary is better than adding broad duplicate scenarios. The repo already has focused unit/runtime proof for invalidation semantics; this ticket should add only the minimal integrated proof still missing.
3. No backward-compatibility aliasing or production refactor belongs here. If the architecture had still been wrong, that would belong in a new implementation ticket, not hidden inside a validation ticket.

## Verification Layers

1. unrelated commodity changes do not clear unrelated frontier-exhausted goals -> strengthened golden E2E assertion at the runtime retry-state boundary plus durable downstream no-action confirmation
2. needs-driven invalidation remains lawful under the landed band-based contract -> existing `golden_wash_action`
3. local self-care competition and ranking remain lawful after S31 retry redesign -> existing `golden_three_way_need_competition` and `golden_utility_weight_diversity_in_need_selection`
4. unrelated actor action does not spuriously reopen another agent's exhausted acquisition branch -> existing `golden_goal_invalidation_by_another_agent`, strengthened to assert the earlier boundary directly
5. persisted runtime contract remains honest after S31 -> existing `golden_save_load_round_trip_under_ai`

## What to Change

### 1. Correct this ticket to match the live S31 end-state

Remove the stale TTL-removal and save-format assumptions, and restate the live retry-state architecture explicitly.

### 2. Strengthen the weakest integrated proof surface

Add the smallest golden coverage needed to prove the over-invalidation invariant at the retry-state boundary. Prefer strengthening an existing scenario in [`golden_ai_decisions.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_ai_decisions.rs) unless a new named scenario is materially clearer.

### 3. Keep the live acceptance set explicit

The existing S31 regression goldens plus save/load parity remain required final verification.

## Files to Touch

- [`tickets/S31-007-golden-tests-validation.md`](/home/joeloverbeck/projects/worldwake/tickets/S31-007-golden-tests-validation.md)
- [`crates/worldwake-ai/tests/golden_ai_decisions.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_ai_decisions.rs)
- [`specs/S31-goal-aware-exhaustion-invalidation.md`](/home/joeloverbeck/projects/worldwake/specs/S31-goal-aware-exhaustion-invalidation.md)

## Out of Scope

- Production retry/invalidation changes already delivered by S31-008 and S31-009
- Save-format migration work beyond validating the live format-9 contract
- New planner abstractions, aliases, or compatibility shims

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-ai --test golden_ai_decisions golden_goal_invalidation_by_another_agent -- --exact`
2. `cargo test -p worldwake-ai --test golden_ai_decisions golden_wash_action -- --exact`
3. `cargo test -p worldwake-ai --test golden_ai_decisions golden_three_way_need_competition -- --exact`
4. `cargo test -p worldwake-ai --test golden_ai_decisions golden_utility_weight_diversity_in_need_selection -- --exact`
5. `cargo test -p worldwake-ai --test golden_determinism golden_save_load_round_trip_under_ai -- --exact`
6. `cargo test -p worldwake-ai`
7. `cargo clippy --workspace --all-targets --all-features -- -D warnings`
8. `cargo test --workspace`

## Outcome

- Completion date: 2026-03-27
- What actually changed:
  - Corrected the ticket to match the live S31 architecture already delivered by [`S31-008`](/home/joeloverbeck/projects/worldwake/archive/tickets/S31-008-complete-exhaustion-invalidation-for-needs-driven-goals.md) and [`S31-009`](/home/joeloverbeck/projects/worldwake/archive/tickets/S31-009-separate-budget-exhaustion-retry-from-invalidation.md).
  - Added `golden_unrelated_commodity_change_preserves_frontier_exhaustion` and `golden_unrelated_commodity_change_preserves_frontier_exhaustion_replays_deterministically` in [`golden_ai_decisions.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_ai_decisions.rs) to prove the over-invalidation invariant at the runtime retry-state boundary.
- Deviations from original plan:
  - No production code changes were needed. Reassessment showed the S31 runtime, retry split, and save/load contract were already complete; the remaining work was to strengthen the weakest golden proof surface and close the validation ticket honestly.
- Verification results:
  - `cargo test -p worldwake-ai --test golden_ai_decisions golden_unrelated_commodity_change_preserves_frontier_exhaustion -- --exact`
  - `cargo test -p worldwake-ai --test golden_ai_decisions golden_unrelated_commodity_change_preserves_frontier_exhaustion_replays_deterministically -- --exact`
  - `cargo test -p worldwake-ai --test golden_ai_decisions golden_goal_invalidation_by_another_agent -- --exact`
  - `cargo test -p worldwake-ai --test golden_ai_decisions golden_wash_action -- --exact`
  - `cargo test -p worldwake-ai --test golden_ai_decisions golden_three_way_need_competition -- --exact`
  - `cargo test -p worldwake-ai --test golden_ai_decisions golden_utility_weight_diversity_in_need_selection -- --exact`
  - `cargo test -p worldwake-ai --test golden_determinism golden_save_load_round_trip_under_ai -- --exact`
  - `cargo test -p worldwake-ai`
  - `cargo clippy --workspace --all-targets --all-features -- -D warnings`
  - `cargo test --workspace`

### Invariants

1. Unrelated commodity changes do not clear unrelated frontier-exhausted goals.
2. Needs-driven retries still reopen lawfully under threshold-band invalidation.
3. Budget-retry and frontier-exhaustion behaviors remain distinct and save/load-stable under the live format-9 runtime contract.

## Test Plan

### New/Modified Tests

1. [`crates/worldwake-ai/tests/golden_ai_decisions.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_ai_decisions.rs) — strengthen over-invalidation proof so it asserts the earlier retry-state boundary directly instead of only a downstream outcome.
2. No new focused unit coverage is expected unless the strengthened golden exposes a real missing edge case.

### Commands

1. `cargo test -p worldwake-ai --test golden_ai_decisions golden_goal_invalidation_by_another_agent -- --exact`
2. `cargo test -p worldwake-ai --test golden_ai_decisions golden_wash_action -- --exact`
3. `cargo test -p worldwake-ai --test golden_ai_decisions golden_three_way_need_competition -- --exact`
4. `cargo test -p worldwake-ai --test golden_ai_decisions golden_utility_weight_diversity_in_need_selection -- --exact`
5. `cargo test -p worldwake-ai --test golden_determinism golden_save_load_round_trip_under_ai -- --exact`
6. `cargo test -p worldwake-ai`
7. `cargo clippy --workspace --all-targets --all-features -- -D warnings`
8. `cargo test --workspace`
