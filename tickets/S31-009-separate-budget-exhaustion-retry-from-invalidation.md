# S31-009: Separate Budget Exhaustion Retry From Invalidation

**Status**: PENDING
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: Yes — AI search, planner runtime, save/load, decision traces
**Deps**: S31-004, S31-005, S31-008

## Problem

The current S31 implementation still conflates two different planner outcomes:

- `PlanSearchResult::FrontierExhausted`, which means the current search space was fully explored and should stay suppressed until relevant local facts change
- `PlanSearchResult::BudgetExhausted`, which means the planner ran out of search budget and may still need a same-world retry

`EXHAUSTION_SKIP_TTL` currently hides that contradiction by periodically reopening both kinds of failure. That workaround is no longer acceptable as the final architecture. It uses abstract time as retry authority, and it keeps `build_candidate_plans()` responsible for a second retry policy on top of the goal-aware invalidation substrate.

This ticket should replace the TTL workaround with a clean retry contract: condition-driven invalidation for genuinely exhausted search spaces, and a separate deterministic same-world retry path for budget-truncated searches.

## Assumption Reassessment (2026-03-27)

1. The shared abstraction boundary under audit is the handoff between `crates/worldwake-ai/src/search/mod.rs::search_plan()` and `crates/worldwake-ai/src/agent_tick/planning.rs::{build_candidate_plans, record_exhausted_goals}`. `search_plan()` classifies each attempt as `Found`, `Unsupported`, `BudgetExhausted`, or `FrontierExhausted`, but `record_exhausted_goals()` currently stores both exhaustion variants in the same `AgentDecisionRuntime.exhaustion_cache` shape.
2. The live retry workaround is still `EXHAUSTION_SKIP_TTL = 20` plus `exhaustion_skip_active()` in [`planning.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/agent_tick/planning.rs). `build_candidate_plans()` skips any cached goal while the TTL window is active, regardless of whether the previous failure was budget truncation or a fully explored frontier.
3. The live goal-aware invalidation substrate in [`exhaustion.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/exhaustion.rs) already handles concrete world-change invalidation for cached goals. That substrate is architecturally appropriate for `FrontierExhausted`, but it is insufficient for `BudgetExhausted` because same-world retries may still be lawful and necessary when no invalidation condition fires.
4. The motivating invariant was rechecked against live behavior with a temporary local TTL-removal experiment. Replacing the skip predicate with `entry.exhausted_at.is_some()` still breaks `golden_goal_invalidation_by_another_agent`, `golden_three_way_need_competition`, and `golden_utility_weight_diversity_in_need_selection`, while `golden_wash_action` now passes. The remaining gap is therefore not needs-band invalidation; it is same-world retry semantics after budget truncation.
5. The live `GoalKind` families exposed by those failures are needs-driven self-care and local-survival branches, including `GoalKind::ConsumeOwnedCommodity`, `GoalKind::Wash`, and competition against remote acquisition/search alternatives such as `GoalKind::AcquireCommodity { purpose: SelfConsume }`. The live operator surfaces involved are the local self-care actions (`Eat`, `Wash`, `Sleep`, `Relieve`) plus prerequisite travel/production/trade branches that can consume budget without proving impossibility.
6. This is an `agent_tick` plus search ticket, not a candidate-generation-only ticket. Focused local harnesses are useful for search/session proof, but the final acceptance surface must include full action registries and the existing golden scenarios because the contradiction only appears in the integrated ranking -> search -> runtime retry chain.
7. The exact heuristic standing in for missing substrate today is the TTL gate. It currently substitutes for a missing distinction between “fully exhausted under current local facts” and “search was truncated before that conclusion.” This ticket must add that missing substrate rather than weakening the gate and reopening the same regressions.
8. The current S31 spec text in [`specs/S31-goal-aware-exhaustion-invalidation.md`](/home/joeloverbeck/projects/worldwake/specs/S31-goal-aware-exhaustion-invalidation.md) still assumes condition-driven invalidation alone is sufficient to remove TTL. Live code and tests now contradict that assumption. This ticket should follow the live architectural contract rather than forcing implementation to match stale spec text.
9. Save/load implications are real, not optional. `AgentDecisionRuntime` is persisted, and `golden_save_load_round_trip_under_ai` is already sensitive to retry-state changes. Any new retry contract that stores planner-progress state must either serialize that state canonically or prove that recomputation on load is behaviorally equivalent under the save/load contract.
10. Adjacent contradictions are already split correctly:
   - `S31-007` should stay the validation/proof ticket.
   - This ticket should own the production retry redesign.
   - Any later spec cleanup can be a documentation follow-up once the live contract is stable.

## Architecture Check

1. The clean architecture is to stop treating `BudgetExhausted` as a world-invalidation fact. It is a planner-resource fact, not proof that the local search space is unchanged-until-world-change.
2. The recommended end-state is a two-track retry contract:
   - `FrontierExhausted` remains cached behind goal-aware invalidation conditions because the current search space was fully explored.
   - `BudgetExhausted` becomes resumable planner work, with deterministic same-world continuation that does not depend on tick TTLs.
3. The most robust long-term shape is resumable search state for budget-truncated goals rather than another cooldown, retry window, or renamed TTL. A resumable frontier preserves prior planner work, avoids repeated root churn, and aligns with `docs/FOUNDATIONS.md` Principle 11 because it changes computation strategy without changing world meaning.
4. The retry contract should stay planner-local and belief-local. No omniscient world polling, hidden global shortcuts, or special-case goal aliases belong here. The same belief snapshot and invalidation conditions should continue to govern whether cached search work is still compatible with the current local decision surface.
5. No backward-compatibility aliasing belongs in the implementation. When the new retry contract lands, `EXHAUSTION_SKIP_TTL` and `exhaustion_skip_active()` should be removed outright rather than preserved beside the new path.

## Verification Layers

1. `BudgetExhausted` and `FrontierExhausted` are recorded into distinct runtime retry states -> focused `agent_tick::planning` unit coverage plus runtime-state assertions
2. resumed budget-truncated searches continue deterministically from prior planner work instead of restarting from root or waiting on TTL -> focused `search` unit/runtime coverage
3. invalidation still clears only when concrete local facts changed -> focused `exhaustion` unit coverage
4. integrated self-care/competition regressions recover without TTL -> existing golden E2E coverage in `golden_ai_decisions`
5. save/load preserves the canonical retry contract -> `golden_save_load_round_trip_under_ai`
6. planner provenance remains explainable after the redesign -> decision-trace assertions or focused trace coverage for resumed budget-retry state

## What to Change

### 1. Split planner retry state by failure meaning

Replace the current single exhaustion-cache meaning with an explicit distinction between:

- condition-invalidated exhaustion after `FrontierExhausted`
- resumable retry state after `BudgetExhausted`

The exact type shape can vary, but the runtime contract must make the distinction explicit rather than encoding it indirectly through `exhausted_at` plus TTL.

### 2. Make budget-truncated searches resumable

Refactor the planner/search boundary so a budget-truncated search can resume deterministically from prior planner work under the same compatible belief surface.

This should include:

- a resumable search session/checkpoint type owned by the AI runtime, not by ad-hoc globals
- compatibility checks that discard stale resumed work when the same invalidation conditions that would clear the old cache now make the checkpoint obsolete
- deterministic continuation semantics that preserve current causal meaning while reducing repeated root-only churn

### 3. Remove TTL as retry authority

Once resumable budget retry exists and the existing goldens pass without TTL:

- remove `EXHAUSTION_SKIP_TTL`
- remove `exhaustion_skip_active()`
- stop using `exhausted_at` as a time-window skip marker
- keep condition-driven invalidation only for genuinely exhausted search spaces

### 4. Preserve save/load and tracing clarity

Update runtime persistence and tracing so the new retry contract is inspectable and survives the existing save/load guarantees. If a resumed search session is persisted, serialize it canonically. If any portion is intentionally recomputed after load, the ticket must prove behavior remains identical under the save/load contract rather than merely “close enough.”

## Files to Touch

- `crates/worldwake-ai/src/search/mod.rs` (modify)
- `crates/worldwake-ai/src/agent_tick/planning.rs` (modify)
- `crates/worldwake-ai/src/decision_runtime.rs` (modify)
- `crates/worldwake-ai/src/exhaustion.rs` (modify)
- `crates/worldwake-ai/src/decision_trace.rs` (modify if resumed-retry provenance needs new trace data)
- `crates/worldwake-ai/tests/golden_ai_decisions.rs` (modify if stronger assertions are needed)
- `crates/worldwake-ai/src/search/tests.rs` (modify/add focused resume coverage)
- `crates/worldwake-ai/src/agent_tick/tests.rs` or `crates/worldwake-ai/src/agent_tick/planning.rs` test module (modify/add focused runtime coverage)
- `crates/worldwake-ai/tests/golden_determinism.rs` or existing save/load golden surface if retry-state persistence changes visible behavior (modify only if required)

## Out of Scope

- broad candidate-generation retuning unrelated to retry semantics
- adding another time-based retry window, renamed TTL, or parallel fallback shim beside the new contract
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
2. `BudgetExhausted` is no longer treated as equivalent to frontier exhaustion; same-world retry remains lawful and deterministic without TTL.
3. Removing TTL does not reintroduce indefinite suppression for local self-care and competition scenarios.
4. The retry redesign does not introduce omniscient world reads, alias paths, or save/load behavior drift.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/search/tests.rs` — prove that a budget-truncated search session can resume deterministically and eventually find/finish the same search without root restart semantics.
2. `crates/worldwake-ai/src/agent_tick/planning.rs` test module or `crates/worldwake-ai/src/agent_tick/tests.rs` — prove that `BudgetExhausted` and `FrontierExhausted` produce different runtime retry records.
3. `crates/worldwake-ai/tests/golden_ai_decisions.rs` — keep the three remaining TTL-removal regression goldens plus `golden_wash_action` in the required pass set, and strengthen trace/state assertions if needed so the new retry contract is explainable.
4. `crates/worldwake-ai/tests/golden_determinism.rs` or existing save/load golden surface — prove the new retry state survives save/load without behavioral drift if runtime persistence changes.

### Commands

1. `cargo test -p worldwake-ai --lib search::tests::search_returns_deferred_barrier_on_budget_exhaustion -- --exact`
2. `cargo test -p worldwake-ai --lib agent_tick::planning::tests::record_exhausted_goals_derives_goal_aware_conditions_and_baseline -- --exact`
3. `cargo test -p worldwake-ai --test golden_ai_decisions golden_goal_invalidation_by_another_agent -- --exact`
4. `cargo test -p worldwake-ai --test golden_ai_decisions golden_wash_action -- --exact`
5. `cargo test -p worldwake-ai --test golden_ai_decisions golden_three_way_need_competition -- --exact`
6. `cargo test -p worldwake-ai --test golden_ai_decisions golden_utility_weight_diversity_in_need_selection -- --exact`
7. `cargo test -p worldwake-ai --test golden_determinism golden_save_load_round_trip_under_ai -- --exact`
8. `cargo test -p worldwake-ai`
9. `cargo clippy --workspace --all-targets --all-features -- -D warnings`
10. `cargo test --workspace`
