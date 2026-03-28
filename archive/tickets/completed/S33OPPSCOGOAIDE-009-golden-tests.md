# S33OPPSCOGOAIDE-009: Golden tests for opportunity-scoped source switching

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Usually none; test harness/support updates allowed if needed for observability
**Deps**: S33OPPSCOGOAIDE-004, S33OPPSCOGOAIDE-005, S33OPPSCOGOAIDE-006, S33OPPSCOGOAIDE-010

## Problem

The remaining S33 proof is not generic "opportunity behavior"; it is source switching after one concrete opportunity is no longer admissible while another remains valid. Reassessment shows the blocked-source branch is already mostly covered by an existing golden, but the explicit selected-opportunity proof is still weaker than the live architecture now allows, and the exhausted-opportunity branch is still missing as a golden at the strongest currently isolated boundary.

## Assumption Reassessment (2026-03-28)

1. Golden tests live in `crates/worldwake-ai/tests/` and use the deterministic `GoldenHarness`.
2. The exact live shared abstraction boundary under audit is `OpportunityKey` flowing from candidate generation and ranked planning into `SelectionTrace.selected_opportunity`, while blocker/exhaustion state remains scoped to the same concrete opportunity.
3. The blocked-source branch is not wholly missing today. `golden_contested_harvest_start_failure_recovers_via_remote_fallback` in `crates/worldwake-ai/tests/golden_production.rs` already proves the authoritative `StartFailed` -> blocker persistence -> remote fallback chain for same-desire sibling sources. Duplicating that scenario as a second near-clone would add noise, not architectural value.
4. The missing gap is twofold:
   - strengthen the existing blocked-source golden so it asserts the canonical selected-opportunity boundary directly, not only downstream hunger relief and source depletion
   - add the still-missing exhausted-opportunity golden for `AcquireCommodity(SelfConsume)` on the live loose-lot surface, where a concrete exhausted opportunity remains suppressed while a sibling opportunity is still generated and selected
5. `OpportunityAnchor`, `OpportunityKey`, candidate-local snapshot scope, ranked same-goal fallthrough, and `PlannedPlan.opportunity` are already live in:
   - `crates/worldwake-core/src/goal.rs`
   - `crates/worldwake-ai/src/candidate_generation.rs`
   - `crates/worldwake-ai/src/agent_tick/planning.rs`
   - `crates/worldwake-ai/src/search/mod.rs`
   - `crates/worldwake-ai/src/planner_ops.rs`
6. `S33OPPSCOGOAIDE-006` is no longer an open architectural seam. The ticket's old assumption that selected-opportunity identity was still missing on `PlannedPlan` is stale and corrected out of scope.
7. Focused coverage already exists for the lower layers this golden depends on:
   - `agent_tick::planning::tests::exhausted_same_goal_opportunity_does_not_block_later_sibling`
   - `agent_tick::planning::tests::same_goal_ranked_opportunities_are_attempted_in_order`
   - `candidate_generation::tests::acquire_multi_source_emits_distinct_place_anchors_and_isolated_evidence`
   - `golden_unrelated_commodity_change_preserves_frontier_exhaustion` in `crates/worldwake-ai/tests/golden_ai_decisions.rs`
   This ticket should not duplicate those focused contracts; it should prove the final golden behavior at the stronger mixed-layer boundary.
8. Decision traces and action traces remain the correct proof surfaces per `docs/golden-e2e-testing.md`: decision traces for candidate presence/suppression and chosen concrete opportunity, action traces for the local `StartFailed` / remote harvest execution facts in the blocked-source scenario, and replay for the new exhausted-opportunity scenario.
9. Mismatch + correction: the earlier draft overfit both scenarios to remote harvest execution. The live `AcquireCommodity` loose-lot exhaustion scenario cleanly proves opportunity isolation at the decision-trace selection boundary, but later execution would conflate a separate planner/execution contradiction. Per `docs/golden-e2e-testing.md`, this ticket should stop at the earliest causal boundary that proves the S33 invariant there.

## Architecture Check

1. Golden tests are still the correct verification surface because the remaining risk is mixed-layer: candidate generation, blocker/exhaustion scope, ranked admission, selection, and execution must all agree on the same concrete opportunity.
2. Strengthening the existing blocked-source golden is cleaner than adding a second nearly identical scenario. The current architecture already has the right branch; the missing value is a stronger assertion at the selected-opportunity boundary.
3. Adding a dedicated exhausted-opportunity golden is more beneficial than changing production architecture again. The live architecture is already the clean one: desire identity on `GoalKey`, tactic identity on `OpportunityKey`, no compatibility alias path. What is missing is proof, not another refactor.
4. This ticket must not widen into trace-schema work or alternate identity plumbing. If the tests expose a real bug, fix that bug directly with the same no-shim rule; otherwise keep the work golden-only.

## Verification Layers

1. Candidate presence for both sibling sources while one concrete opportunity is blocked or exhausted -> decision trace.
2. Winning concrete source after blocker/exhaustion handling -> `SelectionTrace.selected_opportunity`.
3. Authoritative local rejection and later remote execution in the blocked-source scenario -> action trace.
4. Durable source usage / depletion after the switch in the blocked-source scenario -> authoritative world state (`ResourceSource.available_quantity`) and hunger relief.
5. Replay determinism for the new exhausted-opportunity scenario -> replay companion golden.

## What to Change

### 1. Strengthen the existing blocked-source golden

Modify `golden_contested_harvest_start_failure_recovers_via_remote_fallback` in `crates/worldwake-ai/tests/golden_production.rs`.

Keep its existing authoritative/action assertions, but add direct decision-trace proof that:
- tick 0 generates and selects the local `AcquireCommodity(Apple)` opportunity
- the first fresh post-failure search selects the remote `OpportunityKey` rather than retaining or reselecting the blocked local branch
- the fresh fallback search actually includes a remote opportunity-scoped planning attempt

### 2. Add a new exhausted-opportunity golden

Add a golden under `crates/worldwake-ai/tests/golden_ai_decisions.rs` that seeds:
- one hungry agent with knowledge of two lawful loose bread lots at distinct places
- a runtime `exhaustion_cache` entry for the local `OpportunityKey`
- the sibling remote source still available and known

Then step the scenario and assert:
- both sibling opportunities were generated for the same `GoalKey`
- the exhausted local opportunity stayed suppressed for planning
- the remote sibling was selected at the canonical selection boundary

This should prove fallthrough from existing opportunity-scoped exhaustion runtime state without introducing a duplicate production-only scenario or widening into a separate remote-execution bug.

### 3. Replay companion

Add a deterministic replay companion for the new exhausted-opportunity golden. The existing blocked-source golden already has deterministic coverage through the owning production test binary and does not need a second replay clone here.

## Files to Touch

- `crates/worldwake-ai/tests/golden_production.rs` (modify existing blocked-source golden)
- `crates/worldwake-ai/tests/golden_ai_decisions.rs` (add exhausted-opportunity golden + replay companion)

## Out of Scope

- Focused unit tests for individual components
- Stage-1 candidate/ranking trace-identity refactors
- Replacing or duplicating the existing contested-harvest blocked-source scenario with a second near-identical golden
- Production-code behavior changes unless the strengthened/new goldens expose a genuine bug that must be fixed in the same implementation sequence
- New action types or new commodity types
- Performance optimization

## Acceptance Criteria

### Tests That Must Pass

1. The strengthened blocked-source golden directly proves `SelectionTrace.selected_opportunity` switches from the blocked local source to the live remote source after the authoritative `StartFailed`/blocker path.
2. The new exhausted-opportunity golden proves an exhausted concrete `OpportunityKey` does not suppress its sibling for the same `AcquireCommodity` desire at the selection boundary.
3. The new exhausted-opportunity replay companion passes.
4. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. Agents plan from beliefs only, never direct world-state inspection.
2. Blocking one source does not suppress planning for alternative sources.
3. Exhaustion is scoped per-opportunity, not per-desire.
4. The selected concrete source is observable at the canonical runtime/trace boundary rather than inferred only from downstream side effects.
5. Planning already uses candidate-local evidence scope.
6. Deterministic replay produces identical outcomes from the same seed.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_production.rs` — `golden_contested_harvest_start_failure_recovers_via_remote_fallback`
   Rationale: strengthen the already-correct blocked-source golden to assert the canonical selected-opportunity boundary directly instead of relying only on downstream hunger/source state.
2. `crates/worldwake-ai/tests/golden_ai_decisions.rs` — new exhausted-opportunity source-switching golden
   Rationale: add the missing mixed-layer proof that an exhausted concrete `OpportunityKey` remains isolated while its sibling source is still generated and selected at the strongest currently isolated causal boundary.
3. `crates/worldwake-ai/tests/golden_ai_decisions.rs` — replay companion for the new exhausted-opportunity golden
   Rationale: follow the repo's normal golden determinism contract for new scenarios.

### Commands

1. `cargo test -p worldwake-ai -- --list`
2. `cargo test -p worldwake-ai --test golden_production golden_contested_harvest_start_failure_recovers_via_remote_fallback`
3. `cargo test -p worldwake-ai --test golden_ai_decisions exhausted_opportunity`
4. `cargo test -p worldwake-ai`
5. `cargo clippy --workspace`
6. `cargo test --workspace`

## Outcome

- Completed: 2026-03-28
- Actual changes:
  - strengthened `golden_contested_harvest_start_failure_recovers_via_remote_fallback` so it now proves the concrete blocked-source switch at the selected-opportunity boundary using the first fresh post-failure replan, while keeping its existing authoritative/action assertions
  - added `golden_exhausted_opportunity_switches_to_sibling_source` plus deterministic replay coverage in `crates/worldwake-ai/tests/golden_ai_decisions.rs`
  - corrected the exhausted-opportunity scenario to the live `AcquireCommodity(SelfConsume)` loose-lot surface and proved the invariant at `SelectionTrace.selected_opportunity`, which is the strongest isolated causal boundary for this setup
- Deviations from original plan:
  - did not add a second blocked-source golden; strengthening the existing contested-harvest scenario was cleaner and avoided duplicate coverage
  - did not require later execution/hunger assertions for the new exhausted-opportunity scenario because that would conflate a separate remote-execution contradiction rather than the S33 opportunity-isolation contract
- Verification results:
  - `cargo test -p worldwake-ai --test golden_production golden_contested_harvest_start_failure_recovers_via_remote_fallback`
  - `cargo test -p worldwake-ai --test golden_ai_decisions exhausted_opportunity`
  - `cargo test -p worldwake-ai`
  - `cargo clippy --workspace`
  - `cargo test --workspace`
