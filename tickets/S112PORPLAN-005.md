# S112PORPLAN-005: Portfolio-driven planning loop

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — `worldwake-ai::agent_tick::planning` candidate selection rewrite
**Deps**: archive/tickets/S112PORPLAN-001.md, archive/tickets/S112PORPLAN-002.md, archive/tickets/S112PORPLAN-003.md, S112PORPLAN-004

## Problem

With the substrate in place (slot weights from 001, portfolio types + categorization from 002, feasibility probe from 003, trace surface from 004), this ticket performs the integration: replace the flat top-N candidate loop (preceded by `prioritize_same_goal_replan_candidates`) with portfolio assembly + score-weighted slot iteration + probe-gated search.

Per Q1(a)/Q3(a) from reassessment: portfolio assembly always runs (no `max_candidates_to_plan = 1` bypass — FND-28), and `prioritize_same_goal_replan_candidates` is subsumed by the commitment slot (FND-21, FND-28). The function and its inline caller/test are removed.

## Assumption Reassessment (2026-04-20)

1. The current flat loop lives in `crates/worldwake-ai/src/agent_tick/planning.rs`. Key sites: `prioritize_same_goal_replan_candidates` called at line 303, defined at line 405, with inline test `committed_opportunity_clusters_same_goal_siblings_ahead_of_interleaved_goals` at line 2948 + direct call at line 2985. All three sites are removed in this ticket. Admitted-candidates `.take(max_candidates_to_plan)` at line 306 is replaced by `.take(max_candidates_to_plan)` over the portfolio's plausible slots.
2. `committed_opportunity: Option<OpportunityKey>` is tracked on `AgentDecisionRuntime` and passed into the candidate-selection path at line 275 and 993/1026. Portfolio's commitment slot reads this value directly — no new runtime state.
3. Live `GoalKind`s under test: survival (`ConsumeOwnedCommodity`, `AcquireCommodity { purpose: SelfConsume }`, `Sleep`, `Relieve`, `Wash`, `TreatWounds`, `ReduceDanger`, `FreeCarryCapacity`), obligations (`PostNotice`, `PostBounty`, `ReportMissing`, `ReportFound`), economic (`AcquireCommodity { purpose: Restock | RecipeInput(_) }` plus the remaining enterprise kinds). All verified against `crates/worldwake-core/src/goal.rs:24-121` during reassessment.
4. Ticket 002 landed `assemble_portfolio(&[RankedGoal], Option<OpportunityKey>, probe)` without actor identity, so the current substrate classifies all `TreatWounds` candidates as survival. Ticket 005 must not assume the stricter draft-spec `patient == self` split is already implemented when integrating the portfolio into the live planning loop.
5. Motivating invariant (restated before trusting the scenario narrative): agents should not waste a planning tick on two infeasible top-scored candidates when a cheap probe can reject them and let a feasible lower-scored candidate plan the same tick. The feasibility probe (ticket 003) is the mechanism that makes this reachable.
6. Intended layer: runtime `agent_tick` decision-trace/integration coverage — full action registries are required because the regression scenario (ticket 006's golden) depends on action validation to distinguish infeasible from feasible goals.
7. Ordering contract: the `plausible_slots_by_score` ordering drives search-attempt sequence. Divergence from today's top-N behavior depends on `motive_score × slot_weight`, not `motive_score` alone. Tie-breaking is by `SlotKind::Ord` (Survival > Commitment > Economic) — stable and deterministic.
8. Heuristic removal discipline: `prioritize_same_goal_replan_candidates` is not removed-and-replaced-by-nothing; it is *subsumed* by the commitment slot, which explicitly surfaces `committed_opportunity` as the slot winner when still ranked. Multi-anchor retry within a committed goal is deferred to Phase 9 (spec S112 Non-Goals). This does not reopen unrelated regressions because goldens `golden_survival_baseline.rs` and `golden_survival_contested.rs` (ticket acceptance criteria) must still pass.
9. Classification of adjacent contradictions: The removed helper function has one inline test (line 2948-2985). That test is deleted in-scope; its assertion (same-goal clustering ahead of interleaved goals) is superseded by the commitment-slot invariant proven in archived ticket 002's unit test `commitment_slot_picks_committed_opportunity_when_ranked`.

## Architecture Check

1. FND-28 cleanup: removes `prioritize_same_goal_replan_candidates` and eliminates the `max_candidates_to_plan = 1` bypass. After this ticket, there is exactly one live candidate-selection path — portfolio assembly followed by `plausible_slots_by_score.take(max_candidates_to_plan)`.
2. FND-21 alignment: commitment slot explicitly surfaces `committed_opportunity` regardless of raw score. Commitments persist across ticks unless the goal is no longer ranked or the slot's probe rejects it. Margin-based commitment (S74, already landed) still decides whether to keep the commitment once it wins the slot.
3. FND-20 bounded reasoning: portfolio assembly is O(ranked × slot-categories); probe is O(plausible × belief-lookup); tactical search runs at most `max_candidates_to_plan` times. Total work per tick is bounded by the cognitive profile, not by the number of ranked candidates.
4. No backward-compatibility shim: `prioritize_same_goal_replan_candidates` is deleted, not aliased.

## Verification Layers

1. Candidate-selection ordering driven by `plausible_slots_by_score` → decision-trace assertion (new `portfolio` field on `PlanningPipelineTrace` emitted from ticket 004, now populated; `slots_attempted` must match the number of portfolio slots that actually reached `search_plan`).
2. Probe rejection path: probe-rejected slot contributes `GoalRejectionReason::FeasibilityProbeFailed` to `GoalCommittedPayload::rejected_alternatives` → event-log delta assertion on the authoritative decision-history event emitted by existing S110 infrastructure.
3. Agent commits goal C (feasible low-motive) within 2 ticks when goals A and B are probe-rejected → integration-level decision-trace assertion in ticket 006's golden (full action registries).
4. Existing `golden_survival_baseline.rs` and `golden_survival_contested.rs` pass unchanged → golden E2E regression check on this ticket.
5. Commitment-slot subsumption of `prioritize_same_goal_replan_candidates` → the deleted inline test's invariant (same-goal sibling clustering) is now proven by ticket 002's `commitment_slot_picks_committed_opportunity_when_ranked`; no separate regression surface needed here.

## What to Change

### 1. Remove `prioritize_same_goal_replan_candidates`

Delete:
- Definition at `planning.rs:405-437` (approximate).
- Call site at `planning.rs:303`.
- Inline test `committed_opportunity_clusters_same_goal_siblings_ahead_of_interleaved_goals` at `planning.rs:2948-2985` (including its direct `super::prioritize_same_goal_replan_candidates(...)` invocation).

### 2. Rewrite the candidate-selection block

Replace the current `admitted_candidates → .take(max_candidates_to_plan)` flow with:

```rust
let probe_ctx = feasibility_probe::ProbeContext {
    belief_view: &view,
    discrepancy_memory: ...,
    blocker_memory: ...,
    current_tick,
    agent_place: ...,
};
let portfolio = assemble_portfolio(
    &admitted_candidates,
    committed_opportunity,
    |ranked| feasibility_probe::probe(ranked, &probe_ctx),
);
let plausible = portfolio.plausible_slots_by_score(&cognitive.slot_weights);
let candidates_to_plan: Vec<&RankedGoal> = plausible
    .iter()
    .take(usize::from(cognitive.max_candidates_to_plan))
    .map(|(_, slot)| &slot.ranked)
    .collect();
```

Remove the `admitted_candidates.len() > admitted_cap` branch used only for `SameGoalPlanningStopReason::ReachedCandidatePlanCap` computation — its logic is subsumed by portfolio-driven slot attempts. Stop-reason computation is rewritten to track how many plausible slots remained versus were attempted.

The portfolio itself always runs; when `max_candidates_to_plan == 1`, the top plausible slot is the only one searched. No branch on `max_candidates_to_plan`.

### 3. Populate `PlanningPipelineTrace::portfolio`

Build a `PortfolioTrace` from the assembled portfolio (copy each slot's `GoalKey`, `motive_score`, and `FeasibilityVerdict` into a `PortfolioSlotTrace`) and set `slots_attempted` to the count of plausible slots that reached `try_plan`. Assign to `planning_trace.portfolio = Some(...)`.

### 4. Emit `FeasibilityProbeFailed` into `rejected_alternatives`

For each `RejectedBeforeSearch` slot in the assembled portfolio, append a `RejectedAlternativeSummary { goal_key, rejection_reason: GoalRejectionReason::FeasibilityProbeFailed, score_gap }` to the `GoalCommittedPayload::rejected_alternatives` when a winning plan is selected. `score_gap` follows the existing computation pattern used for `LowerMotive` rejections. Respect `CognitiveProfile::decision_history_alternatives` as a cap on the total number of rejections recorded.

### 5. Add a `record_blocker_or_discrepancy` helper

Small helper forwarding to `DiscrepancyMemory::record` or `BlockerMemory::record_blocker` using the `Discrepancy` returned by search failure. The spec's D4 pseudocode references this name; place it near the rewritten candidate-selection block.

### 6. Unit + integration tests

Add to `planning.rs` `#[cfg(test)]`:

1. `portfolio_assembly_always_runs_with_max_candidates_to_plan_one` — a profile with `max_candidates_to_plan = 1` and three plausible slots still produces a fully-populated `portfolio` trace; only the top-ranked slot's plan is attempted.
2. `infeasible_top_two_rejected_feasible_third_commits_same_tick` — three candidates: two probe-rejected, one plausible lowest-motive. Within one planning tick, the plausible candidate is committed and the rejected pair appears in `rejected_alternatives` with reason `FeasibilityProbeFailed`.

## Files to Touch

- `crates/worldwake-ai/src/agent_tick/planning.rs` (modify — rewrite candidate selection, remove helper, add tests)
- `crates/worldwake-ai/src/agent_tick/portfolio.rs` (possibly modify — if import visibility needs adjustment for the integration call site)

## Out of Scope

- New golden E2E test — ticket 006 adds `golden_portfolio_planning.rs`.
- Observer binary rendering of `PortfolioTrace` — deferred to a separate follow-up observer ticket.
- Per-anchor retry of a committed goal (multi-anchor fallback) — deferred to Phase 9 per spec Non-Goals.
- Information slot integration — deferred to S113 follow-up per spec Non-Goals.
- Any change to `validate_*`, action preconditions, or `can_exercise_control` — this ticket only reorders already-validated candidates via the probe.

## Acceptance Criteria

### Tests That Must Pass

1. `portfolio_assembly_always_runs_with_max_candidates_to_plan_one` (new) passes.
2. `infeasible_top_two_rejected_feasible_third_commits_same_tick` (new) passes.
3. `cargo test -p worldwake-ai --test golden_survival_baseline` passes unchanged.
4. `cargo test -p worldwake-ai --test golden_survival_contested` passes unchanged.
5. The removed inline test `committed_opportunity_clusters_same_goal_siblings_ahead_of_interleaved_goals` is intentionally deleted — its invariant is now proven by ticket 002's `commitment_slot_picks_committed_opportunity_when_ranked`.
6. Existing suite: `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`.

### Invariants

1. `prioritize_same_goal_replan_candidates` no longer exists anywhere in the codebase (grep-checkable).
2. Candidate selection has exactly one live path: portfolio assembly → `plausible_slots_by_score` → `.take(max_candidates_to_plan)` (FND-28).
3. `max_candidates_to_plan` value `1` does not bypass portfolio assembly — the portfolio is always populated and traced.
4. `GoalCommittedPayload::rejected_alternatives` records at most `CognitiveProfile::decision_history_alternatives` entries per commit.
5. Portfolio assembly reads only from agent-scoped state (ranked candidates + committed_opportunity) and agent-scoped belief/memory (via the probe). No authoritative world-state reads from the loop (FND-14).

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/agent_tick/planning.rs` — delete one existing inline test; add two new unit/integration tests per the What to Change section.

### Commands

1. `cargo test -p worldwake-ai agent_tick::planning`
2. `cargo test -p worldwake-ai --test golden_survival_baseline`
3. `cargo test -p worldwake-ai --test golden_survival_contested`
4. `cargo test --workspace`
5. `cargo clippy --workspace --all-targets -- -D warnings`
