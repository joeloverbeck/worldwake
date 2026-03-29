# S37COOBASEXH-005: Cooldown-aware exhaustion recording

**Status**: ✅ COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — record_exhausted_goals function signature and body
**Deps**: S37COOBASEXH-001 (PlanningBudget cooldown fields), S37COOBASEXH-002 (ExhaustionEntry cooldown methods)

## Problem

This ticket was written against a pre-cooldown snapshot of the code. On reassessment, the live implementation already records budget exhaustion through cooldown-aware `ExhaustionEntry` helpers, passes `tick` and `budget` through `record_exhausted_goals()`, and has focused coverage for the intended invariants.

## Assumption Reassessment (2026-03-29)

1. `record_exhausted_goals()` in `crates/worldwake-ai/src/agent_tick/planning.rs` already has the corrected signature: `tick: Tick` and `budget: &PlanningBudget`. The `BudgetExhausted` branch already reuses `ExhaustionEntry::record_budget_exhaustion(tick, budget)` for repeated retries and `ExhaustionEntry::budget_retry_pending(..., tick, budget)` for fresh entries.
2. The planning pipeline call sites already pass `tick` and `budget` from both same-goal planning paths in `crates/worldwake-ai/src/agent_tick/planning.rs`.
3. The lower-level cooldown architecture this ticket depends on is also already live in `crates/worldwake-ai/src/decision_runtime.rs`: `ExhaustionEntry` stores `next_retry_tick` and `consecutive_failures`, and cooldown arithmetic is encapsulated in `record_budget_exhaustion`.
4. The focused tests this ticket asked for already exist in `crates/worldwake-ai/src/agent_tick/planning.rs`: `record_exhausted_goals_replaces_frontier_suppression_with_budget_retry_state`, `record_exhausted_goals_derives_goal_aware_conditions_and_baseline`, `record_exhausted_goals_doubles_cooldown_for_repeated_budget_retry_entries`, `record_exhausted_goals_removes_only_successful_opportunity_entry`, and `record_exhausted_goals_records_frontier_exhaustion_as_suppressing_retry_state`.
5. Additional neighboring invariants are also already covered: `has_pending_budget_retry_detects_retryable_budget_entries` and `build_candidate_plans_uses_full_budget_for_retry_eligible_exhaustion_entry`.
6. This is a stale-ticket discrepancy, not a code discrepancy. Scope correction is required; no engine edit is warranted.
7. No adjacent contradiction was exposed that requires a follow-up ticket. The live shape remains aligned with `specs/S37-cooldown-based-exhaustion.md`.

## Architecture Check

1. The spec’s proposed architecture is better than the old manual-counter shape because retry timing is owned by `ExhaustionEntry`, while planning code stays a thin recorder. That is the cleaner long-term boundary, and it is already the live boundary.
2. No backward-compatibility shims or alias paths are present. The old budget-halving recording path has already been replaced rather than preserved.
3. I do not see a cleaner competing architecture for this slice. The current split between `decision_runtime.rs` for retry-state mechanics and `planning.rs` for pipeline orchestration is the durable one.

## Verification Layers

1. `record_exhausted_goals` fresh budget exhaustion -> `next_retry_tick` and retry-state transition -> `agent_tick::planning` focused unit tests
2. Consecutive budget exhaustion on one opportunity -> doubled cooldown and incremented `consecutive_failures` -> `agent_tick::planning` focused unit tests
3. Retry gating -> cooldown eligibility checked before planning resumes -> `has_pending_budget_retry_detects_retryable_budget_entries`
4. Full-budget retry architecture -> admitted retryable entries keep full search budget -> `build_candidate_plans_uses_full_budget_for_retry_eligible_exhaustion_entry`
5. Crate-level regression safety for adjacent AI behavior -> `cargo test -p worldwake-ai`

## What Changed

1. No engine code change was needed during this pass because the implementation described by this ticket was already present in the repository.
2. The necessary correction was to the ticket itself: its assumptions, scope, and verification commands were stale.

## Files to Touch

- `tickets/S37COOBASEXH-005-cooldown-aware-exhaustion-recording.md` (scope/status correction, then archive)

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
5. Focused suite: `cargo test -p worldwake-ai record_exhausted_goals`

### Invariants

1. Cooldown computation delegated to `ExhaustionEntry::record_budget_exhaustion()` — no manual counter management in `record_exhausted_goals`
2. `FrontierExhausted` recording unchanged
3. Successful plan finding still clears exhaustion entry

## Tests

### New/Modified Tests

1. None in this pass. Rationale: the live code already had the focused tests this ticket called for, and reassessment did not expose an uncovered invariant that required additional test authoring.

### Existing Tests That Prove Scope

1. `agent_tick::planning::tests::record_exhausted_goals_replaces_frontier_suppression_with_budget_retry_state`
Rationale: proves `record_exhausted_goals()` records a fresh budget exhaustion as `BudgetRetryPending` with cooldown timing and goal-aware invalidation conditions.
2. `agent_tick::planning::tests::record_exhausted_goals_derives_goal_aware_conditions_and_baseline`
Rationale: proves cooldown-aware recording still preserves the correct invalidation baseline rather than regressing the goal-aware cache contract.
3. `agent_tick::planning::tests::record_exhausted_goals_doubles_cooldown_for_repeated_budget_retry_entries`
Rationale: proves repeated budget exhaustion reuses the existing entry and doubles cooldown through `record_budget_exhaustion()`.
4. `agent_tick::planning::tests::record_exhausted_goals_removes_only_successful_opportunity_entry`
Rationale: proves successful planning still clears only the solved opportunity instead of disturbing unrelated exhaustion entries.
5. `agent_tick::planning::tests::record_exhausted_goals_records_frontier_exhaustion_as_suppressing_retry_state`
Rationale: proves frontier exhaustion remains a suppressing state and is not accidentally folded into cooldown retry behavior.
6. `agent_tick::planning::tests::has_pending_budget_retry_detects_retryable_budget_entries`
Rationale: proves the adjacent planning trigger honors retry eligibility timing.
7. `agent_tick::planning::tests::build_candidate_plans_uses_full_budget_for_retry_eligible_exhaustion_entry`
Rationale: proves the broader S37 architecture still uses full-depth search once cooldown allows a retry.

### Commands

1. `cargo test -p worldwake-ai record_exhausted_goals`
2. `cargo test -p worldwake-ai has_pending_budget_retry`
3. `cargo test -p worldwake-ai build_candidate_plans_uses_full_budget_for_retry_eligible_exhaustion_entry`
4. `cargo test -p worldwake-ai`
5. `cargo clippy --workspace`

## Outcome

- Completed: 2026-03-29
- What actually changed: no engine change was required; this pass corrected the ticket to match the already-implemented cooldown-aware exhaustion recording architecture and verified the live behavior.
- Deviations from original plan: the original plan assumed `record_exhausted_goals()` still used the obsolete manual exhaustion-counter path. Reassessment showed that work had already landed, so the correct action was status/scope correction plus archival rather than new code edits.
- Verification results: `cargo test -p worldwake-ai record_exhausted_goals`, `cargo test -p worldwake-ai has_pending_budget_retry`, `cargo test -p worldwake-ai build_candidate_plans_uses_full_budget_for_retry_eligible_exhaustion_entry`, `cargo test -p worldwake-ai`, and `cargo clippy --workspace` all passed on 2026-03-29.
