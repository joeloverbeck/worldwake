# S33OPPSCOGOAIDE-016: Same-goal sibling planning and selection traceability

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes — `worldwake-ai` decision-trace and planner-contract docs for same-goal sibling admission/selection provenance; no planner-behavior change
**Deps**: [archive/specs/S33-opportunity-scoped-goal-identity.md](/home/joeloverbeck/projects/worldwake/archive/specs/S33-opportunity-scoped-goal-identity.md), [archive/tickets/completed/S35OBSACTSIG-007.md](/home/joeloverbeck/projects/worldwake/archive/tickets/completed/S35OBSACTSIG-007.md), [tickets/S33OPPSCOGOAIDE-015-first-class-opportunity-planning-carrier.md](/home/joeloverbeck/projects/worldwake/tickets/S33OPPSCOGOAIDE-015-first-class-opportunity-planning-carrier.md), [docs/planner-contracts.md](/home/joeloverbeck/projects/worldwake/docs/planner-contracts.md)

## Problem

The recent S35 observable-competition fix exposed a remaining debugging weakness. Decision traces were strong enough to show that a local sibling opportunity had been discounted, but weaker than they should be at explaining:

- why same-goal sibling search stopped where it did
- why the planning loop stopped after a found plan
- whether selection chose a different sibling opportunity for the same `GoalKey`
- whether the selected branch came from search or from retaining the current same-goal plan

The live planner now behaves correctly, and current traces already expose ranked same-goal attempt order, but the traceability contract still under-specifies why the sibling scan stopped and whether a same-goal search result refreshed the current branch versus replacing it with a different sibling opportunity. That still forces code inspection where structured trace data should be sufficient. The clean fix is to extend traceability at the exact same-goal sibling admission/selection boundary, not to broaden golden assertions or rely on ad hoc logging.

## Assumption Reassessment (2026-03-29)

1. The exact shared abstraction boundary under audit is the post-ranking planner pipeline in [crates/worldwake-ai/src/agent_tick/planning.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/agent_tick/planning.rs), plus the public decision-trace model in [crates/worldwake-ai/src/decision_trace.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/decision_trace.rs). The live runtime symbols directly involved are `build_candidate_plans()`, `summarize_search_provenance()`, `determine_selected_plan_source()`, `PlanSearchTrace`, and `SelectionTrace`.
2. S33 already made stage-1 candidate/ranking identity opportunity-scoped, and S33/S35 already made runtime selection behavior opportunity-correct. That work lives in [archive/tickets/S33OPPSCOGOAIDE-011-opportunity-scoped-stage1-trace.md](/home/joeloverbeck/projects/worldwake/archive/tickets/S33OPPSCOGOAIDE-011-opportunity-scoped-stage1-trace.md), [archive/tickets/completed/S33OPPSCOGOAIDE-014-migrate-selection-trace-consumers-to-derived-helpers.md](/home/joeloverbeck/projects/worldwake/archive/tickets/completed/S33OPPSCOGOAIDE-014-migrate-selection-trace-consumers-to-derived-helpers.md), and [archive/tickets/completed/S35OBSACTSIG-007.md](/home/joeloverbeck/projects/worldwake/archive/tickets/completed/S35OBSACTSIG-007.md).
3. The remaining gap is narrower and later in the pipeline. The current trace model already exposes same-goal attempt order through `PlanSearchTrace.attempts`, proved by `agent_tick::planning::tests::traced_planning_records_same_goal_opportunity_attempt_order`. The missing trace surface is not "were siblings admitted at all?" but "why did the sibling scan stop here?" and "did search refresh the current branch or replace it with a different sibling opportunity?"
4. `determine_selected_plan_source()` still answers only a coarse question: `SearchSelection` vs `RetainedCurrentPlan` at the `GoalKey` level, while `SelectedPlanReplacementTrace.kind` distinguishes only `SameGoalBranchReplanned` vs `GoalChanged`. Reassessment shows that combination is still too coarse for the architectural question exposed by S35: "did search select a different sibling opportunity for the same goal, or merely refresh the same branch?" The current helpers cannot answer that directly.
5. The planning loop in `build_candidate_plans()` continues through contiguous same-goal siblings after a found plan and stops when the ranked stream moves to a different `GoalKey`; it also stops naturally when the admitted list ends. That rule is architecturally intentional, but the trace contract in [docs/planner-contracts.md](/home/joeloverbeck/projects/worldwake/docs/planner-contracts.md) does not yet document it, and `PlanSearchTrace` does not expose a structured same-goal stop reason or the attempt that triggered same-goal continuation.
6. The live `GoalKind` that exposed the weakness was `GoalKind::RestockCommodity { commodity: Apple }`, but the missing trace surface is not restock-specific. It is a same-goal sibling planning/selection provenance gap that applies to any opportunity-scoped goal family.
7. This is a single-layer `worldwake-ai` traceability ticket. No authoritative world behavior changes, no action-trace ordering changes, and no candidate-generation legality changes are required. The intended proof surfaces are focused `agent_tick::planning`, `agent_tick`, and `decision_trace` tests, plus the existing S35 golden as a consumer of the stronger selection contract.
8. Adjacent contradiction classification:
   - required consequence in scope: expose same-goal sibling admission/stop/selection provenance in trace data
   - separate structural cleanup already owned by [tickets/S33OPPSCOGOAIDE-015-first-class-opportunity-planning-carrier.md](/home/joeloverbeck/projects/worldwake/tickets/S33OPPSCOGOAIDE-015-first-class-opportunity-planning-carrier.md): replace tuple plumbing with a first-class carrier
   - separate docs contradiction in scope: [docs/planner-contracts.md](/home/joeloverbeck/projects/worldwake/docs/planner-contracts.md) should explicitly document same-goal sibling planning admission/stop semantics once the trace contract exists
9. Mismatch + correction: the correct fix is not to make goldens assert more execution timing details, and it is not to add a second helper path beside `SelectedPlanSource` / `SelectedPlanReplacementTrace`. The missing information belongs in focused planning/selection trace surfaces and planner-contract documentation, consistent with [tickets/README.md](/home/joeloverbeck/projects/worldwake/tickets/README.md).

## Architecture Check

1. The clean architecture is to extend traceability at the exact boundary where same-goal sibling opportunity reasoning becomes opaque: planning admission, search stop reason, and branch-source attribution. That aligns with [docs/FOUNDATIONS.md](/home/joeloverbeck/projects/worldwake/docs/FOUNDATIONS.md) P3 and P25 by exposing concrete planner facts instead of forcing source-diving.
2. This is cleaner than broadening goldens or adding ad hoc debug prints. Goldens should prove behavior; focused traces should explain planner-side provenance.
3. This is cleaner than folding the work into S33OPPSCOGOAIDE-015. The carrier ticket owns structural transport cleanup; this ticket owns the public debugging contract over that transport. The two should compose, not blur into one mixed-responsibility ticket.
4. No backwards-compatibility aliasing or parallel trace paths should be introduced. Existing trace structs should gain the missing bounded fields in place.

## Verification Layers

1. Same-goal sibling planning attempts remain distinguishable and the planning loop stop reason is explicit -> focused `agent_tick::planning` tests plus `decision_trace` summary coverage
2. Selection trace can distinguish retained-current, same-goal same-branch refresh, same-goal sibling replacement, and different-goal search replacement -> focused `agent_tick` tests
3. Planner-contract docs explicitly name the same-goal sibling admission/continuation/stop contract -> focused doc update verified against live symbols in `agent_tick/planning.rs`
4. Existing golden `golden_observed_harvest_competition_redirects_to_remote_sibling` remains the end-to-end consumer proving the stronger trace surface did not change behavior -> existing golden
5. Additional action-trace or authoritative-world assertions are not the contract here -> out of scope because the gap is planner-side provenance
6. If traces still cannot explain one same-goal sibling boundary after this ticket, the strongest lower-layer proof remains focused planning tests and any further missing provenance should become a new traceability follow-up rather than a broader golden rewrite

## What to Change

### 1. Expose structured same-goal sibling planning admission/stop provenance

Extend the planning trace model so same-goal sibling continuation and stop provenance are inspectable without source-diving. At minimum, traces should make clear:

- which opportunity attempts were admitted in ranked order
- whether planning stopped because the loop moved to a different `GoalKey`, hit the candidate cap, or ran out of admitted opportunities
- which found attempt, if any, triggered the same-goal continuation rule

Keep the payload concrete and bounded. Do not add stringly free-form debug text as the source of truth.

### 2. Refine selection-source provenance for same-goal sibling branches

Refactor `SelectionTrace` / helper attribution so traces can answer more than “search vs retained current plan.” Reassessment should determine the cleanest bounded surface, but the delivered contract must be able to distinguish:

- retained current branch
- search selected a different-goal branch
- search selected a different sibling opportunity for the same goal
- search refreshed the same sibling/current branch

If a new enum is needed, prefer a typed trace enum integrated with the existing selection trace surface over bool/flag combinations or parallel alias helpers.

### 3. Update planner-contract documentation

Extend [docs/planner-contracts.md](/home/joeloverbeck/projects/worldwake/docs/planner-contracts.md) so it documents:

- the live same-goal sibling planning admission rule in `build_candidate_plans()`
- the intended stop boundary after a found plan
- the traceability contract for same-goal sibling branch attribution in selection

Do not leave this as ticket lore once the code lands.

### 4. Add focused traceability regressions

Add focused tests proving:

- same-goal sibling attempt order and stop reason are exposed structurally
- same-goal sibling replacement is distinguishable from retained-current behavior
- the existing S35 redirection scenario can read the stronger selection provenance without relying on extra execution-timing assertions
- human-readable decision-trace summaries still surface the new bounded fields coherently

## Files to Touch

- `crates/worldwake-ai/src/decision_trace.rs` (modify — bounded same-goal planning/selection provenance fields)
- `crates/worldwake-ai/src/agent_tick/planning.rs` (modify — emit same-goal planning stop/admission provenance)
- `crates/worldwake-ai/src/agent_tick/tests.rs` (modify — focused source-attribution / planning-trace coverage)
- `crates/worldwake-ai/tests/golden_production.rs` (modify only if the stronger trace contract is asserted directly; behavior must remain unchanged)
- `docs/planner-contracts.md` (modify — document same-goal sibling planning/selection trace contract)

## Out of Scope

- Any planner behavior change
- Carrier/tuple transport cleanup beyond what is required for the trace payloads
- Ranking arithmetic changes
- New golden scenarios
- Authoritative action/runtime trace changes outside the planner decision trace

## Acceptance Criteria

### Tests That Must Pass

1. Focused planning trace tests can distinguish why same-goal sibling planning stopped where it did
2. Focused selection trace tests can distinguish same-goal sibling search replacement from retained-current behavior
3. Existing focused same-goal sibling selection regression still passes
4. Existing golden `golden_observed_harvest_competition_redirects_to_remote_sibling` still passes
5. Existing suite: `cargo test -p worldwake-ai`
6. Existing suite: `cargo clippy -p worldwake-ai --all-targets -- -D warnings`

### Invariants

1. Same-goal sibling branch provenance remains concrete and opportunity-scoped in traces
2. Desire-level helper views remain derived and do not become a second source of truth for branch provenance
3. This ticket changes only traceability/documentation, not planner legality or search behavior

## Tests

### New/Modified Tests

1. `agent_tick::planning::tests::traced_planning_records_same_goal_opportunity_attempt_order` in `crates/worldwake-ai/src/agent_tick/planning.rs` — now asserts the bounded same-goal stop surface for an admitted sibling scan that exhausts its admitted opportunities without a found continuation trigger
2. `agent_tick::planning::tests::same_goal_planning_trace_records_different_goal_stop_after_found_sibling` in `crates/worldwake-ai/src/agent_tick/planning.rs` — proves a found same-goal sibling records the continuation trigger and the later different-goal stop reason without depending on a full golden
3. `agent_tick::planning::tests::same_goal_planning_trace_records_candidate_cap_stop_reason` in `crates/worldwake-ai/src/agent_tick/planning.rs` — proves the trace distinguishes candidate-cap truncation from same-goal exhaustion
4. `agent_tick::tests::summarize_plan_replacement_records_same_goal_sibling_replacement` in `crates/worldwake-ai/src/agent_tick/tests.rs` — proves same-goal search can now report sibling replacement rather than collapsing everything into a generic same-goal replan bucket
5. `agent_tick::tests::summarize_plan_replacement_records_same_goal_branch_refresh` in `crates/worldwake-ai/src/agent_tick/tests.rs` — proves same-opportunity refresh is distinct from sibling replacement
6. `decision_trace::tests::summary_planning_includes_same_goal_stop_and_replacement_kind` in `crates/worldwake-ai/src/decision_trace.rs` — keeps the human-readable summary aligned with the structured trace contract
7. Existing `plan_selection::tests::same_goal_sibling_opportunity_selection_uses_opportunity_scoped_scores` in `crates/worldwake-ai/src/plan_selection.rs` — remains green to prove selection behavior stayed opportunity-scoped
8. Existing `golden_observed_harvest_competition_redirects_to_remote_sibling` in `crates/worldwake-ai/tests/golden_production.rs` — remains the observable-competition consumer showing no planner behavior regression
9. Existing `golden_stale_prerequisite_belief_discovery_replan` in `crates/worldwake-ai/tests/golden_supply_chain.rs` — updated to assert the refined replacement-kind contract (`SameGoalBranchRefreshed` for that scenario's live behavior) instead of inferring a sibling swap

### Commands

1. `cargo test -p worldwake-ai agent_tick::planning::tests::traced_planning_records_same_goal_opportunity_attempt_order -- --exact`
2. `cargo test -p worldwake-ai agent_tick::planning::tests::same_goal_planning_trace_records_different_goal_stop_after_found_sibling -- --exact`
3. `cargo test -p worldwake-ai agent_tick::planning::tests::same_goal_planning_trace_records_candidate_cap_stop_reason -- --exact`
4. `cargo test -p worldwake-ai agent_tick::tests::summarize_plan_replacement_records_same_goal_sibling_replacement -- --exact`
5. `cargo test -p worldwake-ai agent_tick::tests::summarize_plan_replacement_records_same_goal_branch_refresh -- --exact`
6. `cargo test -p worldwake-ai decision_trace::tests::summary_planning_includes_same_goal_stop_and_replacement_kind -- --exact`
7. `cargo test -p worldwake-ai plan_selection::tests::same_goal_sibling_opportunity_selection_uses_opportunity_scoped_scores -- --exact`
8. `cargo test -p worldwake-ai golden_observed_harvest_competition_redirects_to_remote_sibling -- --exact`
9. `cargo test -p worldwake-ai golden_stale_prerequisite_belief_discovery_replan -- --exact`
10. `cargo test -p worldwake-ai`
11. `cargo clippy -p worldwake-ai --all-targets -- -D warnings`

## Outcome

- Completed: 2026-03-29
- What actually changed:
  - added explicit same-goal sibling stop provenance to `PlanSearchTrace`
  - refined same-goal search replacement provenance into `SameGoalBranchRefreshed` vs `SameGoalSiblingReplaced`
  - updated decision-trace summary rendering and planner-contract docs to describe the live contract
  - strengthened focused planner/selection tests and corrected one stale golden expectation to the live branch-refresh behavior
- Deviations from original plan:
  - no golden changes were needed for the observable-harvest competition scenario itself
  - one adjacent golden consumer in supply-chain coverage was updated because reassessment showed that scenario refreshes the same opportunity branch rather than replacing it with a sibling
- Verification results:
  - focused tests for same-goal stop reasons, branch attribution, summary rendering, selection behavior, and golden consumers passed
  - `cargo test -p worldwake-ai` passed
  - `cargo clippy -p worldwake-ai --all-targets -- -D warnings` passed
