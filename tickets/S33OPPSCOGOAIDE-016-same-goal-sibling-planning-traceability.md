# S33OPPSCOGOAIDE-016: Same-goal sibling planning and selection traceability

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes — `worldwake-ai` decision-trace and planner-contract docs for same-goal sibling admission/selection provenance; no planner-behavior change
**Deps**: [archive/specs/S33-opportunity-scoped-goal-identity.md](/home/joeloverbeck/projects/worldwake/archive/specs/S33-opportunity-scoped-goal-identity.md), [archive/tickets/completed/S35OBSACTSIG-007.md](/home/joeloverbeck/projects/worldwake/archive/tickets/completed/S35OBSACTSIG-007.md), [tickets/S33OPPSCOGOAIDE-015-first-class-opportunity-planning-carrier.md](/home/joeloverbeck/projects/worldwake/tickets/S33OPPSCOGOAIDE-015-first-class-opportunity-planning-carrier.md), [docs/planner-contracts.md](/home/joeloverbeck/projects/worldwake/docs/planner-contracts.md)

## Problem

The recent S35 observable-competition fix exposed a remaining debugging weakness. Decision traces were strong enough to show that a local sibling opportunity had been discounted, but weaker than they should be at explaining:

- whether later same-goal siblings were admitted to search at all
- why the planning loop stopped after a found plan
- whether selection chose a different sibling opportunity for the same `GoalKey`
- whether the selected branch came from search or from retaining the current same-goal plan

The live planner now behaves correctly, but the traceability contract still under-specifies this boundary. That forces code inspection where structured trace data should be sufficient. The clean fix is to extend traceability at the exact same-goal sibling admission/selection boundary, not to broaden golden assertions or rely on ad hoc logging.

## Assumption Reassessment (2026-03-29)

1. The exact shared abstraction boundary under audit is the post-ranking planner pipeline in [crates/worldwake-ai/src/agent_tick/planning.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/agent_tick/planning.rs), plus the public decision-trace model in [crates/worldwake-ai/src/decision_trace.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/decision_trace.rs). The live runtime symbols directly involved are `build_candidate_plans()`, `summarize_search_provenance()`, `determine_selected_plan_source()`, `PlanSearchTrace`, and `SelectionTrace`.
2. S33 already made stage-1 candidate/ranking identity opportunity-scoped, and S33/S35 already made runtime selection behavior opportunity-correct. That work lives in [archive/tickets/S33OPPSCOGOAIDE-011-opportunity-scoped-stage1-trace.md](/home/joeloverbeck/projects/worldwake/archive/tickets/S33OPPSCOGOAIDE-011-opportunity-scoped-stage1-trace.md), [archive/tickets/completed/S33OPPSCOGOAIDE-014-migrate-selection-trace-consumers-to-derived-helpers.md](/home/joeloverbeck/projects/worldwake/archive/tickets/completed/S33OPPSCOGOAIDE-014-migrate-selection-trace-consumers-to-derived-helpers.md), and [archive/tickets/completed/S35OBSACTSIG-007.md](/home/joeloverbeck/projects/worldwake/archive/tickets/completed/S35OBSACTSIG-007.md).
3. The remaining gap is narrower and later in the pipeline. The current trace model can show ranked siblings and selected opportunity, but it does not explicitly explain why planning terminated where it did for same-goal siblings or whether a same-goal sibling search result displaced the current branch versus merely refreshing it.
4. `determine_selected_plan_source()` still answers only a coarse question: search selection vs retained current plan at the `GoalKey` level. Reassessment shows this is too coarse for the architectural question exposed by S35: "did search select a different sibling opportunity for the same goal?" The current helper cannot answer that directly.
5. The planning loop in `build_candidate_plans()` now continues through contiguous same-goal siblings after a found plan and stops when the ranked stream moves to a different `GoalKey`. That rule is architecturally intentional, but the trace contract in [docs/planner-contracts.md](/home/joeloverbeck/projects/worldwake/docs/planner-contracts.md) does not yet document it, and the decision trace does not expose a structured stop reason for the loop.
6. The live `GoalKind` that exposed the weakness was `GoalKind::RestockCommodity { commodity: Apple }`, but the missing trace surface is not restock-specific. It is a same-goal sibling planning/selection provenance gap that applies to any opportunity-scoped goal family.
7. This is a single-layer `worldwake-ai` traceability ticket. No authoritative world behavior changes, no action-trace ordering changes, and no candidate-generation legality changes are required. The intended proof surfaces are focused planning/trace tests plus the existing S35 golden as a consumer of the stronger trace contract.
8. Adjacent contradiction classification:
   - required consequence in scope: expose same-goal sibling admission/stop/selection provenance in trace data
   - separate structural cleanup already owned by [tickets/S33OPPSCOGOAIDE-015-first-class-opportunity-planning-carrier.md](/home/joeloverbeck/projects/worldwake/tickets/S33OPPSCOGOAIDE-015-first-class-opportunity-planning-carrier.md): replace tuple plumbing with a first-class carrier
   - separate docs contradiction in scope: [docs/planner-contracts.md](/home/joeloverbeck/projects/worldwake/docs/planner-contracts.md) should explicitly document same-goal sibling planning admission/stop semantics once the trace contract exists
9. Mismatch + correction: the correct fix is not to make goldens assert more execution timing details. The missing information belongs in focused planning/selection trace surfaces and planner-contract documentation, consistent with [tickets/README.md](/home/joeloverbeck/projects/worldwake/tickets/README.md).

## Architecture Check

1. The clean architecture is to extend traceability at the exact boundary where same-goal sibling opportunity reasoning becomes opaque: planning admission, search stop reason, and branch-source attribution. That aligns with [docs/FOUNDATIONS.md](/home/joeloverbeck/projects/worldwake/docs/FOUNDATIONS.md) P3 and P25 by exposing concrete planner facts instead of forcing source-diving.
2. This is cleaner than broadening goldens or adding ad hoc debug prints. Goldens should prove behavior; focused traces should explain planner-side provenance.
3. This is cleaner than folding the work into S33OPPSCOGOAIDE-015. The carrier ticket owns structural transport cleanup; this ticket owns the public debugging contract over that transport. The two should compose, not blur into one mixed-responsibility ticket.
4. No backwards-compatibility aliasing or parallel trace paths should be introduced. Existing trace structs should gain the missing bounded fields in place.

## Verification Layers

1. Same-goal sibling planning attempts remain distinguishable and the planning loop stop reason is explicit -> focused `agent_tick::planning` / `decision_trace` tests
2. Selection trace can distinguish search-selected same-goal sibling replacement from retained-current same-goal branch -> focused `agent_tick` / `decision_trace` tests
3. Planner-contract docs explicitly name the same-goal sibling admission/stop contract -> focused doc update verified against live symbols
4. Existing golden `golden_observed_harvest_competition_redirects_to_remote_sibling` remains the end-to-end consumer proving the stronger trace surface did not change behavior
5. Additional action-trace or authoritative-world assertions are not the contract here; those layers remain out of scope because the gap is planner-side provenance
6. If traces still cannot explain one same-goal sibling boundary after this ticket, the strongest lower-layer proof remains focused planning tests and any further missing provenance should become a new traceability follow-up rather than a broader golden rewrite

## What to Change

### 1. Expose structured same-goal sibling planning admission/stop provenance

Extend the planning trace model so same-goal sibling admission is inspectable without source-diving. At minimum, traces should make clear:

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

If a new enum is needed, prefer a typed trace enum over bool/flag combinations.

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

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/agent_tick/planning.rs` or `crates/worldwake-ai/src/decision_trace.rs` — add focused tests proving same-goal sibling planning stop reason and attempt provenance are exposed structurally
2. `crates/worldwake-ai/src/agent_tick/tests.rs` — add focused tests proving same-goal sibling search replacement is distinguishable from retained-current same-goal behavior
3. `crates/worldwake-ai/src/plan_selection.rs` — keep the existing sibling opportunity regression green under the richer trace/source contract
4. `crates/worldwake-ai/tests/golden_production.rs` — keep the observable-competition golden as the end-to-end consumer of the stronger traceability contract

### Commands

1. `cargo test -p worldwake-ai same_goal_sibling_opportunity_selection_uses_opportunity_scoped_scores -- --exact`
2. `cargo test -p worldwake-ai traced_planning_records_same_goal_opportunity_attempt_order -- --exact`
3. `cargo test -p worldwake-ai golden_observed_harvest_competition_redirects_to_remote_sibling -- --exact`
4. `cargo test -p worldwake-ai`
5. `cargo clippy -p worldwake-ai --all-targets -- -D warnings`
