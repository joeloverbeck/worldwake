# S33OPPSCOGOAIDE-015: First-class opportunity-scoped planning candidate carrier

**Status**: ✅ COMPLETED
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes — `worldwake-ai`: replace tuple-based post-search planning/selection plumbing with a first-class opportunity-scoped carrier
**Deps**: [archive/specs/S33-opportunity-scoped-goal-identity.md](/home/joeloverbeck/projects/worldwake/archive/specs/S33-opportunity-scoped-goal-identity.md), [archive/tickets/completed/S33OPPSCOGOAIDE-014-migrate-selection-trace-consumers-to-derived-helpers.md](/home/joeloverbeck/projects/worldwake/archive/tickets/completed/S33OPPSCOGOAIDE-014-migrate-selection-trace-consumers-to-derived-helpers.md), [archive/tickets/completed/S35OBSACTSIG-007.md](/home/joeloverbeck/projects/worldwake/archive/tickets/completed/S35OBSACTSIG-007.md)

## Problem

S33 established `OpportunityKey` as the canonical opportunity identity and S35OBSACTSIG-007 fixed two remaining behavioral collapses in planning and selection. But the planner pipeline still passes post-search data around as ad hoc tuples, which keeps opportunity identity partially implicit and easy to lose again.

Today the live flow is split across multiple representations:

- `build_candidate_plans()` in [crates/worldwake-ai/src/agent_tick/planning.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/agent_tick/planning.rs) returns `(OpportunityKey, PlanSearchResult, Vec<BindingRejection>, Vec<SearchExpansionSummary>)`
- `plans_as_options()` immediately collapses that into `(GoalKey, Option<PlannedPlan>)`
- `select_best_plan()` in [crates/worldwake-ai/src/plan_selection.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/plan_selection.rs) has to reconstruct opportunity-scoped meaning indirectly from `PlannedPlan.opportunity`
- `determine_selected_plan_source()` still reasons over `(GoalKey, Option<PlannedPlan>)`, which is a desire-level view of an opportunity-level search result

That shape works after the S35 fix, but it is not the cleanest long-term architecture. The same post-search fact currently has multiple lawful transport paths: tuple element 0, `PlannedPlan.opportunity`, and sometimes bare `GoalKey`. The canonical end-state should be one first-class carrier that owns the searched `OpportunityKey`, the `PlanSearchResult`, and any associated trace data until the pipeline intentionally projects to a narrower view.

## Assumption Reassessment (2026-03-29)

1. Live code still uses tuple carriers rather than a named planning-result type. The exact boundary under audit is the post-search planner pipeline in [crates/worldwake-ai/src/agent_tick/planning.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/agent_tick/planning.rs): `build_candidate_plans()` -> `plans_as_options()` -> `select_best_plan()` / trace summarizers, with the same derived projection also consumed by interrupt evaluation from [crates/worldwake-ai/src/agent_tick/active_action.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/agent_tick/active_action.rs).
2. The canonical opportunity identity already exists and is authoritative for this layer: `PlannedPlan.opportunity: OpportunityKey` and per-opportunity search results from `build_candidate_plans()` both reflect the S33 contract from [archive/specs/S33-opportunity-scoped-goal-identity.md](/home/joeloverbeck/projects/worldwake/archive/specs/S33-opportunity-scoped-goal-identity.md). This ticket should not invent a second identity surface.
3. The recent completed fix in [archive/tickets/completed/S35OBSACTSIG-007.md](/home/joeloverbeck/projects/worldwake/archive/tickets/completed/S35OBSACTSIG-007.md) proved the remaining risk is real. `select_best_plan()` had to be corrected to key candidate scores by `OpportunityKey`, and `build_candidate_plans()` had to continue through later same-goal siblings. The architecture now behaves correctly, but the tuple plumbing still leaves the identity contract less explicit than it should be.
4. The live duplicate-path issue is structural, not a new behavior regression. `plans_as_options()` currently discards the searched `OpportunityKey` from the outer carrier and keeps only `GoalKey` plus `Option<PlannedPlan>`, relying on `PlannedPlan.opportunity` to recover the opportunity later. That is not an immediate bug, but it is a fragile derived-path handoff that violates the “one canonical path” requirement from [tickets/README.md](/home/joeloverbeck/projects/worldwake/tickets/README.md) for information-path refactors.
5. `determine_selected_plan_source()` in [crates/worldwake-ai/src/agent_tick/planning.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/agent_tick/planning.rs) still accepts `selected_goal: GoalKey` and `plans: &[(GoalKey, Option<PlannedPlan>)]`. Reassessment shows this helper is currently sufficient only because it answers the narrow question “did search yield some plan for this goal?” The live focused proof is `agent_tick::tests::determine_selected_plan_source_distinguishes_search_selection_from_retention`. If future trace contracts need to distinguish “retained current same-goal branch” from “search selected different same-goal sibling,” this signature is too coarse.
6. This is a single-layer `worldwake-ai` structural ticket. No authoritative world state, scheduler semantics, or information-locality path changes are involved. The contract under audit is the AI-internal post-search planning/selection carrier shape.
7. The live `GoalKind` surface most recently exposing the weakness was `GoalKind::RestockCommodity { commodity: Apple }`, but the cleanup is goal-family agnostic. The invariant is not “restock behaves differently”; it is “post-search opportunity identity remains explicit until the code intentionally narrows to desire-level reasoning.”
8. Adjacent-ticket check: active tickets [tickets/S36DECGOAL-002.md](/home/joeloverbeck/projects/worldwake/tickets/S36DECGOAL-002.md), [tickets/S36DECGOAL-003.md](/home/joeloverbeck/projects/worldwake/tickets/S36DECGOAL-003.md), and [tickets/S36DECGOAL-004.md](/home/joeloverbeck/projects/worldwake/tickets/S36DECGOAL-004.md) are about declaration-backed dispatch keyed from `GoalKind`-derived AI families. They do not own opportunity-scoped search-result carriers, and broadening them to do so would mix declaration dispatch with post-search planning-state transport. Current focused coverage already present for this ticket’s area is `agent_tick::planning::tests::same_goal_ranked_opportunities_are_attempted_in_order`, `agent_tick::planning::tests::exhausted_same_goal_opportunity_does_not_block_later_sibling`, `agent_tick::planning::tests::traced_planning_records_same_goal_opportunity_attempt_order`, `plan_selection::tests::same_goal_sibling_opportunity_selection_uses_opportunity_scoped_scores`, and the golden `golden_observed_harvest_competition_redirects_to_remote_sibling`; the missing gap is carrier-specific structural coverage, not absence of behavior coverage.
9. This cleanup should remove a duplicate path, not add another compatibility wrapper. The canonical post-search carrier should become the only internal transport object, and any selection/interrupt projection should either disappear or become a tightly local named derived view at the final call site. A free-floating `(GoalKey, Option<PlannedPlan>)` helper should not remain as a cross-module transport alias.
10. Mismatch + correction: S33’s completed rollout correctly put `OpportunityKey` on `PlannedPlan`, but that alone did not finish the pipeline cleanup. The remaining tuple-based transport is a separate structural follow-up, not proof that S33’s behavioral work was incomplete.

## Architecture Check

1. A named opportunity-scoped carrier is cleaner than tuple plumbing because it makes identity, search result, and trace attachments explicit and type-checked. That better satisfies [docs/FOUNDATIONS.md](/home/joeloverbeck/projects/worldwake/docs/FOUNDATIONS.md) P3 and P25: the concrete opportunity identity remains explicit, and any narrower derived summary becomes obviously derived.
2. This is cleaner than leaving multiple lawful transport paths for the same fact. The planner should not need to remember “tuple slot 0 is the opportunity, unless you only have the plan, unless you only have the goal-level projection.” One carrier avoids future regressions like the S35 selection collapse.
3. No backwards-compatibility aliasing should be introduced. The new carrier should replace the tuple pipeline directly; do not keep parallel tuple-return and struct-return APIs alive once the migration lands.

## Verification Layers

1. Post-search searched opportunity remains explicit through planning and selection handoff -> focused `agent_tick::planning` unit tests
2. Selection still compares opportunity-scoped ranked scores against searched plans correctly -> focused `plan_selection` unit tests
3. Decision-trace selected-plan source remains correct after carrier migration -> focused `agent_tick` source-attribution tests and traced planning tests
4. The S35 observable-competition end-to-end proof still selects the remote sibling opportunity -> existing golden `golden_observed_harvest_competition_redirects_to_remote_sibling`
5. If trace helpers still only prove outcome but not enough provenance for same-goal sibling branch retention, strengthen the lowest-layer focused planning/selection tests here rather than weakening the golden
6. Single-layer `worldwake-ai` structural ticket; no action trace, event-log, or authoritative world-state mapping applies

## What to Change

### 1. Introduce a first-class post-search planning carrier

Add a named `worldwake-ai` type for one searched ranked opportunity, for example:

- searched `OpportunityKey`
- `PlanSearchResult`
- binding rejections
- search expansion summaries

If the planner needs both “full search record” and “selection-ready found-plan projection,” express that as named methods on the carrier rather than a second unrelated tuple shape.

### 2. Migrate planning helpers to the carrier

Refactor [crates/worldwake-ai/src/agent_tick/planning.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/agent_tick/planning.rs) so:

- `build_candidate_plans()` returns the named carrier list
- `summarize_search_provenance()` consumes the carrier list
- `record_exhausted_goals()` consumes the carrier list
- `plans_as_options()` is removed or reduced to a tightly local derived projection if one truly remains necessary

The canonical path after the change should be carrier -> explicit methods / views, not carrier -> anonymous tuple -> reconstructed opportunity identity.

### 3. Migrate plan selection and source attribution off the coarse tuple

Refactor [crates/worldwake-ai/src/plan_selection.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/plan_selection.rs), [crates/worldwake-ai/src/agent_tick/planning.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/agent_tick/planning.rs), and, if still needed after refactor, [crates/worldwake-ai/src/agent_tick/active_action.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/agent_tick/active_action.rs) so selection and interrupt consumers read either the first-class carrier or a named derived view that still preserves searched opportunity identity.

Reassess whether `determine_selected_plan_source()` should stay `GoalKey`-scoped or become opportunity-aware. If the helper can remain goal-scoped without losing the debugging contract, document why. If not, migrate it in-scope rather than preserving a coarse helper beside the cleaner carrier.

### 4. Preserve the strongest existing behavior proofs

Keep the S35 same-goal sibling opportunity regression in [crates/worldwake-ai/src/plan_selection.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/plan_selection.rs) and the production goldens in [crates/worldwake-ai/tests/golden_production.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_production.rs). Add focused structural coverage for the new carrier boundary instead of relying only on the golden.

## Files to Touch

- `crates/worldwake-ai/src/agent_tick/planning.rs` (modify — introduce and use first-class post-search carrier)
- `crates/worldwake-ai/src/plan_selection.rs` (modify — selection consumes named carrier/view instead of anonymous tuple plumbing)
- `crates/worldwake-ai/src/agent_tick/active_action.rs` (modify only if the interrupt path needs a local derived view after `plans_as_options()` removal)
- `crates/worldwake-ai/src/agent_tick/tests.rs` (modify — strengthen source-attribution / trace helper coverage if signatures change)
- `crates/worldwake-ai/tests/golden_production.rs` (modify only if a structural migration requires trace-surface assertion updates; behavior should remain unchanged)

## Out of Scope

- Any new ranking arithmetic
- Workstation-scoped opportunity identity redesign
- S36 declaration-backed `GoalKind` dispatch work
- Authoritative action semantics, scheduler ordering, or world-state changes
- New heuristics for branch choice

## Acceptance Criteria

### Tests That Must Pass

1. Post-search planning/selection no longer depends on anonymous tuple carriers that drop searched `OpportunityKey` from the canonical transport path
2. The focused same-goal sibling opportunity selection regression still passes
3. Existing golden `golden_observed_harvest_competition_redirects_to_remote_sibling` still passes
4. Existing suite: `cargo test -p worldwake-ai`
5. Existing suite: `cargo clippy -p worldwake-ai --all-targets -- -D warnings`

### Invariants

1. `OpportunityKey` remains the canonical opportunity identity throughout the post-search planning pipeline until code intentionally narrows to a desire-level question
2. No parallel tuple-based compatibility path survives beside the new carrier
3. Carrier migration does not change selection behavior, only the internal transport contract

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/agent_tick/planning.rs` — add focused tests proving the new carrier preserves searched `OpportunityKey` and full search outcome through helper boundaries
2. `crates/worldwake-ai/src/plan_selection.rs` — keep and strengthen `same_goal_sibling_opportunity_selection_uses_opportunity_scoped_scores` against the new selection input shape
3. `crates/worldwake-ai/src/agent_tick/tests.rs` — keep and strengthen `determine_selected_plan_source_distinguishes_search_selection_from_retention` if the helper signature changes
4. `crates/worldwake-ai/tests/golden_production.rs` — keep `golden_observed_harvest_competition_redirects_to_remote_sibling` as the end-to-end regression proof that same-goal sibling opportunity redirection still works

### Commands

1. `cargo test -p worldwake-ai same_goal_sibling_opportunity_selection_uses_opportunity_scoped_scores -- --exact`
2. `cargo test -p worldwake-ai determine_selected_plan_source_distinguishes_search_selection_from_retention -- --exact`
3. `cargo test -p worldwake-ai golden_observed_harvest_competition_redirects_to_remote_sibling -- --exact`
4. `cargo test -p worldwake-ai`
5. `cargo clippy -p worldwake-ai --all-targets -- -D warnings`

## Outcome

- Completion date: 2026-03-29
- What actually changed:
  - Replaced the anonymous post-search tuple transport in `worldwake-ai` with a first-class `CandidatePlanSearch` carrier in `agent_tick/planning.rs`.
  - Replaced the shared `(GoalKey, Option<PlannedPlan>)` cross-module transport with a named `SelectionCandidatePlan` view in `plan_selection.rs`, and updated selection plus interrupt consumers to use it.
  - Made selected-plan source attribution opportunity-aware by keying `determine_selected_plan_source()` from the selected `OpportunityKey`.
  - Tightened `summarize_search_provenance()` so it uses the selected opportunity's search record rather than the first found result in the planning batch.
  - Added focused coverage for the new carrier/provenance boundary while keeping the existing same-goal sibling selection regression and golden proof green.
- Deviations from original plan:
  - `active_action.rs` remained in scope because the interrupt path consumed the old derived tuple view and needed the new named selection view.
  - No `golden_production.rs` assertions needed to change; the existing golden remained sufficient as the end-to-end behavioral proof.
  - The refactor also fixed a trace/provenance correctness gap at the same boundary by selecting provenance from the chosen opportunity instead of whichever found result appeared first.
- Verification results:
  - `cargo test -p worldwake-ai`
  - `cargo clippy -p worldwake-ai --all-targets -- -D warnings`
  - `cargo test -p worldwake-ai golden_observed_harvest_competition_redirects_to_remote_sibling -- --exact`
