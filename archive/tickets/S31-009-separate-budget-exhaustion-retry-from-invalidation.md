# S31-009: Separate Budget Exhaustion Retry From Invalidation

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — AI planner runtime, retry-state persistence, save/load
**Deps**: S31-004, S31-005, S31-008

## Problem

The current S31 implementation still conflates two different planner outcomes:

- `PlanSearchResult::FrontierExhausted`, which means the current search space was fully explored and should stay suppressed until relevant local facts change
- `PlanSearchResult::BudgetExhausted`, which means the planner ran out of search budget and may still need a same-world retry

`EXHAUSTION_SKIP_TTL` currently hides that contradiction by periodically reopening both kinds of failure. That workaround is no longer acceptable as the final architecture. It uses abstract time as retry authority, and it keeps `build_candidate_plans()` responsible for a second retry policy on top of the goal-aware invalidation substrate.

This ticket should replace the TTL workaround with a clean retry contract: condition-driven invalidation for genuinely exhausted search spaces, and a separate deterministic same-world retry path for budget-truncated searches.

## Assumption Reassessment (2026-03-27)

1. The shared abstraction boundary under audit is the handoff between `crates/worldwake-ai/src/search/mod.rs::search_plan()` and `crates/worldwake-ai/src/agent_tick/planning.rs::{build_candidate_plans, record_exhausted_goals}`. `search_plan()` already distinguishes `BudgetExhausted` from `FrontierExhausted`, but `record_exhausted_goals()` still collapses both outcomes into the same `AgentDecisionRuntime.exhaustion_cache` contract.
2. The live retry workaround is still `EXHAUSTION_SKIP_TTL = 20` plus `exhaustion_skip_active()` in [`planning.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/agent_tick/planning.rs). `build_candidate_plans()` suppresses any cached goal while the TTL window is active, regardless of whether the previous failure was budget truncation or a fully explored frontier.
3. The live goal-aware invalidation substrate in [`exhaustion.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/exhaustion.rs) already handles concrete world-change invalidation for cached goals. That substrate is architecturally correct for `FrontierExhausted`, but not for `BudgetExhausted`, because same-world retries remain lawful even when no invalidation condition fires.
4. The invalidation substrate is stronger than the original ticket narrative in two important ways:
   - needs-driven invalidation is already band-based (`NeedChangedBands`) rather than fixed-delta
   - per-goal baselines and invalidation conditions already serialize through `AgentDecisionRuntime`
   The missing substrate is no longer “goal-aware invalidation”; it is explicit retry meaning for budget truncation.
5. The live retry backoff narrative is partially stale. `ExhaustionEntry.count` exists and `build_candidate_plans()` reads it to shrink search budget, but `record_exhausted_goals()` does not currently increment it. Live code therefore has a TTL gate plus a dormant backoff field, not a functioning repeated-budget-exhaustion policy.
6. The motivating invariant was rechecked against live behavior with a temporary local TTL-removal experiment. Replacing the skip predicate with unconditional cached-goal suppression still breaks `golden_goal_invalidation_by_another_agent`, `golden_three_way_need_competition`, and `golden_utility_weight_diversity_in_need_selection`, while `golden_wash_action` passes. The remaining gap is therefore same-world retry semantics after `BudgetExhausted`, not missing needs invalidation.
7. The live `GoalKind` families exposed by those regressions are needs-driven self-care and local-survival branches, including `GoalKind::ConsumeOwnedCommodity`, `GoalKind::Wash`, and competition with remote acquisition branches such as `GoalKind::AcquireCommodity { purpose: SelfConsume }`. The live operator surfaces involved are the local self-care actions (`Eat`, `Wash`, `Sleep`, `Relieve`) plus prerequisite travel/production/trade branches that can consume search budget without proving impossibility.
8. The current S31 spec text in [`specs/S31-goal-aware-exhaustion-invalidation.md`](/home/joeloverbeck/projects/worldwake/specs/S31-goal-aware-exhaustion-invalidation.md) still assumes condition-driven invalidation alone is sufficient to remove TTL. Live code and tests now contradict that assumption. This ticket should follow the live architectural contract rather than forcing implementation to match stale spec text.
9. A fully resumable persisted search frontier is not the cleanest in-scope architecture for the current planner. `search_plan()` owns a borrowed `PlanningSnapshot`, a local `BinaryHeap<FrontierEntry>`, and `PlanningState<'snapshot>` values. Serializing resumable frontier state would require crossing that borrowed-snapshot boundary and broadening the save/load contract far beyond the bug being fixed. That belongs in a dedicated planner-architecture ticket if performance data later justifies it.
10. Save/load implications are still real. `AgentDecisionRuntime` is persisted, and `golden_save_load_round_trip_under_ai` is sensitive to retry-state changes. If the retry-state shape changes incompatibly, the save format version should be bumped rather than relying on accidental bincode failure.
11. The traced planning path in `plan_and_validate_next_step_traced()` currently bypasses exhausted-goal suppression entirely for observability. This ticket should keep the runtime semantics honest first; if trace provenance needs an explicit “skipped due to frontier exhaustion” surface after the split, that proof should be added directly rather than keeping trace-only behavior divergent by accident.
12. Adjacent contradictions are already split correctly:
   - `S31-007` should stay the validation/proof ticket.
   - This ticket should own the production retry redesign.
   - Any later spec cleanup can be a documentation follow-up once the live contract is stable.

## Architecture Check

1. The clean architecture is to stop treating `BudgetExhausted` as a world-invalidation fact. It is a planner-resource result, not proof that the local search space stays closed until a world change arrives.
2. The recommended end-state is a two-track retry contract in persisted runtime state:
   - `FrontierExhausted` remains cached behind goal-aware invalidation because the current compatible search space was fully explored.
   - `BudgetExhausted` remains retryable on the next compatible planning pass instead of being hidden behind TTL.
3. The cleaner in-scope design is an explicit retry-state split, not a persisted resumable heap. That keeps the architecture local to planner runtime semantics, avoids serializing borrowed search internals, and still removes abstract-time retry authority.
4. The retry contract should stay planner-local and belief-local. The same invalidation conditions and baselines should determine whether a stored retry record is still compatible with the current local decision surface. No omniscient world polling, hidden global shortcuts, or goal aliases belong here.
5. No backward-compatibility aliasing belongs in the implementation. When the new retry contract lands, `EXHAUSTION_SKIP_TTL` and `exhaustion_skip_active()` should be removed outright rather than preserved beside the new path.
6. If runtime persistence changes incompatibly, bump the save format version instead of keeping dead fields solely to deserialize older retry entries.

## Verification Layers

1. `BudgetExhausted` and `FrontierExhausted` are recorded into distinct runtime retry states -> focused `agent_tick::planning` unit coverage plus runtime-state assertions
2. budget-truncated goals remain retryable on the next compatible planning pass without TTL suppression -> focused `agent_tick::planning` coverage over candidate filtering / budget application
3. frontier-exhausted goals remain suppressed until concrete invalidation fires -> focused `exhaustion` unit coverage plus planning skip assertions
4. integrated self-care/competition regressions recover without TTL -> existing golden E2E coverage in `golden_ai_decisions`
5. save/load preserves the canonical retry contract and fails honestly across incompatible format changes -> `golden_save_load_round_trip_under_ai` plus runtime serialization coverage
6. planner provenance stays explainable after the split -> focused decision-trace coverage only where live trace semantics actually change

## What to Change

### 1. Split planner retry state by failure meaning

Replace the current single exhaustion-cache meaning with an explicit distinction between:

- condition-invalidated frontier exhaustion after `FrontierExhausted`
- same-world retryable budget truncation after `BudgetExhausted`

The runtime contract must make the distinction explicit rather than encoding it indirectly through `exhausted_at` plus TTL.

### 2. Retry budget-truncated goals on the next compatible planning pass

Refactor the planner/runtime boundary so `BudgetExhausted` does not enter the frontier-exhaustion suppression path.

This should include:

- retry-state compatibility checks that discard stale budget-retry records when the same invalidation conditions that would clear frontier exhaustion make the old retry context obsolete
- deterministic next-pass retry semantics under the same compatible belief surface
- removal or correction of any dormant retry fields whose current behavior no longer matches the runtime contract

### 3. Remove TTL as retry authority

Once the explicit retry-state split exists and the existing goldens pass without TTL:

- remove `EXHAUSTION_SKIP_TTL`
- remove `exhaustion_skip_active()`
- stop using `exhausted_at` as a time-window skip marker
- keep condition-driven invalidation only for genuinely exhausted search spaces

### 4. Preserve save/load honesty

Update runtime persistence so the new retry contract is the only persisted contract. If the serialized retry-state shape changes incompatibly, bump the save format version instead of keeping dead fields or compatibility aliases.

## Files to Touch

- `crates/worldwake-ai/src/agent_tick/planning.rs` (modify)
- `crates/worldwake-ai/src/decision_runtime.rs` (modify)
- `crates/worldwake-ai/src/exhaustion.rs` (modify)
- `crates/worldwake-ai/src/decision_trace.rs` (modify only if retry-state provenance needs an explicit new trace surface)
- `crates/worldwake-ai/src/agent_tick/tests.rs` or `crates/worldwake-ai/src/agent_tick/planning.rs` test module (modify/add focused runtime coverage)
- `crates/worldwake-ai/tests/golden_ai_decisions.rs` (modify only if stronger assertions are needed)
- `crates/worldwake-ai/tests/golden_determinism.rs` or `crates/worldwake-ai/src/agent_tick/tests.rs` (modify if save/load/runtime serialization expectations need stronger proof)
- `crates/worldwake-sim/src/save_load.rs` (modify only if the persisted retry-state shape changes incompatibly)

## Out of Scope

- broad candidate-generation retuning unrelated to retry semantics
- adding another time-based retry window, renamed TTL, or parallel fallback shim beside the new contract
- serializing a borrowed search frontier / checkpoint heap as part of this ticket
- changing authoritative world rules to hide planner retry weaknesses
- the final golden-certification ticket work owned by `S31-007`

## Acceptance Criteria

### Tests That Must Pass

1. `golden_goal_invalidation_by_another_agent`
2. `golden_wash_action`
3. `golden_three_way_need_competition`
4. `golden_utility_weight_diversity_in_need_selection`
5. `golden_save_load_round_trip_under_ai`
6. Existing suite: `cargo test -p worldwake-ai`
7. Existing suite: `cargo test --workspace`

### Invariants

1. `FrontierExhausted` is no longer retried because abstract time passed; it only reopens when concrete local invalidation conditions fire.
2. `BudgetExhausted` is no longer treated as equivalent to frontier exhaustion; same-world retry remains lawful and deterministic without TTL suppression.
3. Removing TTL does not reintroduce indefinite suppression for local self-care and competition scenarios.
4. The retry redesign does not introduce omniscient world reads, alias paths, or dishonest save/load behavior.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/agent_tick/planning.rs` test module or `crates/worldwake-ai/src/agent_tick/tests.rs` — prove that `BudgetExhausted` and `FrontierExhausted` produce different runtime retry records and different next-pass planning behavior.
2. `crates/worldwake-ai/src/exhaustion.rs` — keep or strengthen focused invalidation coverage so frontier-exhausted entries still clear only on concrete local fact changes.
3. `crates/worldwake-ai/tests/golden_ai_decisions.rs` — keep the three remaining TTL-removal regression goldens plus `golden_wash_action` in the required pass set; strengthen assertions only if needed to make the new retry contract explainable.
4. `crates/worldwake-ai/tests/golden_determinism.rs` or `crates/worldwake-ai/src/agent_tick/tests.rs` — prove the persisted retry state survives save/load honestly if runtime persistence changes.

### Commands

1. `cargo test -p worldwake-ai --lib agent_tick::planning::tests::record_exhausted_goals_derives_goal_aware_conditions_and_baseline -- --exact`
2. `cargo test -p worldwake-ai --lib exhaustion::tests::invalidate_exhausted_goals_removes_only_entries_with_fired_conditions -- --exact`
3. `cargo test -p worldwake-ai --test golden_ai_decisions golden_goal_invalidation_by_another_agent -- --exact`
4. `cargo test -p worldwake-ai --test golden_ai_decisions golden_wash_action -- --exact`
5. `cargo test -p worldwake-ai --test golden_ai_decisions golden_three_way_need_competition -- --exact`
6. `cargo test -p worldwake-ai --test golden_ai_decisions golden_utility_weight_diversity_in_need_selection -- --exact`
7. `cargo test -p worldwake-ai --test golden_determinism golden_save_load_round_trip_under_ai -- --exact`
8. `cargo test -p worldwake-ai`
9. `cargo clippy --workspace --all-targets --all-features -- -D warnings`
10. `cargo test --workspace`

## Outcome

- Completion date: 2026-03-27
- What actually changed:
  - replaced TTL-driven exhaustion suppression with an explicit persisted retry-state split in `AgentDecisionRuntime`
  - `FrontierExhausted` now persists as suppression state behind goal-aware invalidation only
  - `BudgetExhausted` now persists as retry-pending state that reopens planning on the next compatible tick without waiting on time
  - removed `EXHAUSTION_SKIP_TTL`, `exhaustion_skip_active()`, and the dormant count/backoff contract
  - aligned traced planning with live retry semantics instead of bypassing exhausted-goal behavior
  - bumped `SAVE_FORMAT_VERSION` to 9 so the incompatible persisted retry-state change fails honestly instead of through accidental runtime deserialization
- Deviations from original plan:
  - did not implement a persisted resumable search frontier/session
  - reassessment showed that serializing borrowed `PlanningSnapshot` / `PlanningState<'snapshot>` search internals would widen scope well beyond the bug; the cleaner in-scope architecture was an explicit retry-state split at planner-runtime level
- Verification results:
  - focused checks passed:
    - `cargo test -p worldwake-ai --lib agent_tick::planning::tests::record_exhausted_goals_derives_goal_aware_conditions_and_baseline -- --exact`
    - `cargo test -p worldwake-ai --lib exhaustion::tests::invalidate_exhausted_goals_removes_only_entries_with_fired_conditions -- --exact`
    - `cargo test -p worldwake-ai --test golden_ai_decisions golden_goal_invalidation_by_another_agent -- --exact`
    - `cargo test -p worldwake-ai --test golden_ai_decisions golden_wash_action -- --exact`
    - `cargo test -p worldwake-ai --test golden_ai_decisions golden_three_way_need_competition -- --exact`
    - `cargo test -p worldwake-ai --test golden_ai_decisions golden_utility_weight_diversity_in_need_selection -- --exact`
    - `cargo test -p worldwake-ai --test golden_determinism golden_save_load_round_trip_under_ai -- --exact`
  - broad verification passed:
    - `cargo test -p worldwake-ai`
    - `cargo clippy --workspace --all-targets --all-features -- -D warnings`
    - `cargo test --workspace`
