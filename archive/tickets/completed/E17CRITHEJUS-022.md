# E17CRITHEJUS-022: Add planner traceability for omitted root operators and missing prerequisites

**Status**: ✅ COMPLETED
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes — `worldwake-ai` decision-trace schema and planner trace emission
**Deps**: E17CRITHEJUS-020, E17CRITHEJUS-021

## Problem

Decision traces were useful during `E17CRITHEJUS-008`, but they stopped at local symptoms. They could show `selected=none`, a root candidate skip, or `DurationEstimateFailed`, yet they could not explain two architecturally important failure modes:

1. a relevant live operator never became a root candidate because no affordance surfaced it and no goal-derived synthesis path existed
2. duration estimation failed because a specific planner prerequisite was missing from snapshot-backed state

That forces deeper source inspection for common planner failures and weakens the project’s causal debugging contract. The right fix is to teach the planner to trace those failure categories explicitly, not to rely on ad hoc debug output or downstream golden symptoms.

## Assumption Reassessment (2026-03-26)

1. Current root-candidate tracing already exists in `crates/worldwake-ai/src/decision_trace.rs` and `crates/worldwake-ai/src/search/candidates.rs`. The live trace model includes `RootCandidatePayloadStatus`, `RootCandidateFilterReason`, `RootCandidateOutcome`, and `RootCandidateSkipReason`, including `DurationEstimateFailed`.
2. Current traces only exist for candidates that were actually surfaced. They do not record when a relevant operator in `goal.relevant_op_kinds()` never produced a root candidate at all, so “candidate omitted” and “candidate surfaced then skipped” are still conflated at the debugging level.
3. The shared abstraction boundary under audit is: `GoalKind::relevant_op_kinds()` / `GroundedGoal::synthesized_root_candidate_targets()` in `crates/worldwake-ai/src/goal_model.rs` -> root candidate surfacing in `crates/worldwake-ai/src/search/candidates.rs` / `crates/worldwake-ai/src/search/mod.rs` -> successor construction in `crates/worldwake-ai/src/search/transition.rs` -> rendering in `crates/worldwake-ai/src/decision_trace.rs`.
4. The intended invariant is: planner traces must distinguish absent relevant operators, filtered surfaced operators, skipped surfaced operators, and named missing prerequisites well enough to explain common planner failures without source-diving into unrelated modules.
5. The live operator families that motivated this gap include at least `PlannerOpKind::Tell`, `PlannerOpKind::Investigate`, `PlannerOpKind::PressForceClaim`, `PlannerOpKind::Trade`, and `PlannerOpKind::Accuse`, because those are the exact-goal surfaces where “missing root operator” was hardest to diagnose.
6. This is a traceability ticket, not a planner behavior ticket. It must not change planner legality or search semantics except where additional structured error information is needed to report the existing behavior accurately.
7. The current missing-prerequisite trace gap is concrete, not hypothetical. `RootCandidateSkipReason::DurationEstimateFailed` exists in `crates/worldwake-ai/src/decision_trace.rs`, but it collapses all duration-estimation failures into one bucket even though `crates/worldwake-ai/src/planner_duration_contract.rs` already enumerates the planner-visible dependency classes that the live planner surface depends on.
8. The current goal-synthesis API can only return `Option<Vec<EntityId>>` from `GroundedGoal::synthesized_root_candidate_targets()` in `crates/worldwake-ai/src/goal_model.rs`. That means the live code can distinguish “synthesized targets exist” from “no synthesized targets,” but it cannot yet distinguish “no synthesis path exists for this op” from “a synthesis path was considered but target derivation failed” without extending that API. The original ticket narrative treated that distinction as already observable; it is not.
9. Existing trace-focused coverage already proves nearby behavior. Verified examples from `cargo test -p worldwake-ai -- --list` include `agent_tick::tests::trace_social_resend_omission_reason`, `decision_trace::tests::goal_status_reports_social_omission_reason`, `search::tests::place_scoped_blocker_prunes_candidate_at_blocked_place`, and `planner_conformance::conformance_accuse`. There is still no focused coverage for “relevant operator omitted before root-candidate creation” or for named duration prerequisite diagnostics.
10. If `E17CRITHEJUS-020` changes the root-candidate contract and `E17CRITHEJUS-021` tightens snapshot completeness, this ticket must reflect those final contracts rather than documenting the old implicit behavior.
11. Adjacent contradictions are out of scope here:
    - changing which operators are legally surfaced belongs to `E17CRITHEJUS-020`
    - preserving planner-visible state belongs to `E17CRITHEJUS-021`
12. Mismatch + correction: the current traces are not “missing,” but they are missing categories. The corrected scope is to add first-class provenance for omitted relevant operators and named missing prerequisites, not to replace the entire trace system.
13. Mismatch + correction: the original “at minimum” omission taxonomy was too ambitious for the current API boundary. A clean implementation can and should distinguish:
    - relevant op has no registered action definition
    - relevant op has registered defs but no affordance and no synthesis result
    - relevant op has registered defs, no affordance, and synthesis is unsupported for that goal/op pair
    - relevant op has registered defs, no affordance, and synthesis was eligible but target derivation failed
    Achieving the last two distinctions requires extending `GroundedGoal::synthesized_root_candidate_targets()` to return structured diagnostics rather than `Option<Vec<EntityId>>`.

## Architecture Check

1. The clean fix is to extend planner traceability at the exact failure boundaries where the architecture currently becomes opaque: operator surfacing and prerequisite resolution.
2. That is cleaner than adding more debug dumps or asserting on downstream golden absence, because the trace then explains the planner’s own failure modes directly.
3. This aligns with `docs/FOUNDATIONS.md`: explainable emergence, local causality, and explicit world/planner state over hidden reasoning shortcuts.
4. The clean implementation path should reuse the existing planner-duration dependency contract in `crates/worldwake-ai/src/planner_duration_contract.rs` rather than introduce ad hoc strings or duplicate dependency enums inside `decision_trace.rs`.
5. No compatibility aliases or fallback behavior should be introduced. This ticket improves explanation, not semantics.

## Verification Layers

1. Relevant operators omitted before root-candidate creation are recorded with explicit omission reasons -> focused search/decision-trace tests
2. Surfaced root candidates still record filtered/skipped outcomes distinctly from omission outcomes -> focused decision-trace tests
3. Missing duration/prerequisite failures name the concrete missing dependency class when knowable -> focused search transition tests and decision-trace rendering tests
4. Existing planner behavior remains unchanged while traces become more informative -> `cargo test -p worldwake-ai`
5. Golden scenarios are not the primary proof surface; use them only as regression safety, not as the architectural explanation layer

## What to Change

### 1. Add omitted-operator trace categories

Extend decision trace data so the planner can report when a relevant operator never produced a root candidate and why. At minimum, distinguish:
- no matching action definition
- no affordance surfaced and no goal-side synthesis applies
- goal-side synthesis unsupported for that goal/op pair
- goal-side synthesis attempted but target derivation failed

### 2. Add named missing-prerequisite diagnostics

Refine root-candidate skip diagnostics so `DurationEstimateFailed` and similar failures can identify the planner-visible duration dependency class when the planner knows it. Reuse the dependency taxonomy already declared in `crates/worldwake-ai/src/planner_duration_contract.rs`.

### 3. Update rendering and focused tests

Update trace formatting and focused tests so the new categories are visible in `dump_agent()` output and asserted in planner/search unit coverage.

## Files to Touch

- `crates/worldwake-ai/src/decision_trace.rs` (modify)
- `crates/worldwake-ai/src/search/candidates.rs` (modify)
- `crates/worldwake-ai/src/search/mod.rs` (modify)
- `crates/worldwake-ai/src/search/transition.rs` (modify)
- `crates/worldwake-ai/src/goal_model.rs` (modify)
- `crates/worldwake-ai/src/planner_duration_contract.rs` (modify or reuse without duplication)
- `crates/worldwake-ai/src/search/tests.rs` (modify)
- `crates/worldwake-ai/src/decision_trace.rs` (tests)
- `crates/worldwake-ai/src/agent_tick/tests.rs` (modify, if end-to-end trace assertions are needed)

## Out of Scope

- Changing planner legality or search ranking semantics
- Snapshot preservation changes beyond what is required to emit an accurate diagnostic
- New golden scenarios whose only purpose is to compensate for missing lower-layer trace assertions

## Acceptance Criteria

### Tests That Must Pass

1. Planner traces distinguish “relevant operator omitted” from “relevant operator surfaced and then filtered/skipped.”
2. Missing planner prerequisites reported during successor construction name the dependency class when knowable.
3. Existing trace dumps and focused search tests continue to pass with the richer trace model.
4. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. Decision traces remain the preferred first stop for planner-debugging questions because they expose the planner’s own failure boundary instead of only downstream symptoms.
2. Traceability changes do not alter planner behavior; they only improve provenance.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/search/tests.rs::search_trace_records_omitted_relevant_operator_when_no_matching_action_def_exists` — proves the root trace distinguishes “relevant op had no registered action def” from surfaced-and-filtered candidates.
2. `crates/worldwake-ai/src/search/tests.rs::search_trace_records_trade_omission_when_goal_side_target_derivation_fails` — proves a relevant op can be omitted with a synthesis target-derivation failure rather than disappearing into a generic “no candidate” bucket.
3. `crates/worldwake-ai/src/search/tests.rs::search_trace_records_duration_dependency_when_root_candidate_duration_estimate_fails` — proves duration failures name the planner-visible dependency class (`ActorTradeDisposition`) instead of collapsing to a flat `DurationEstimateFailed`.
4. `crates/worldwake-ai/src/goal_model.rs::grounded_goal_does_not_synthesize_trade_root_targets_from_ambiguous_evidence` — tightened to assert the new structured synthesis failure class.
5. `crates/worldwake-ai/src/goal_model.rs::grounded_goal_reports_unsupported_trade_synthesis_for_unrelated_goal` — proves the synthesis boundary distinguishes unsupported goal/op pairs from derivation failures.
6. `crates/worldwake-ai/src/decision_trace.rs::summary_planning_includes_root_candidate_omissions_and_dependency_diagnostics` — proves `dump_agent()`/summary rendering exposes both root omissions and dependency-tagged skip diagnostics.

### Commands

1. `cargo test -p worldwake-ai goal_model::tests::grounded_goal_reports_unsupported_trade_synthesis_for_unrelated_goal`
2. `cargo test -p worldwake-ai search::tests::search_trace_records_trade_omission_when_goal_side_target_derivation_fails`
3. `cargo test -p worldwake-ai decision_trace::tests::summary_planning_includes_root_candidate_omissions_and_dependency_diagnostics`
4. `cargo test -p worldwake-ai`
5. `cargo clippy --workspace`

## Outcome

Completed: 2026-03-26

What actually changed:
- Added first-class root operator omission traces at the root search boundary, separate from surfaced root-candidate outcomes.
- Extended `GroundedGoal::synthesized_root_candidate_targets()` from an `Option<Vec<EntityId>>` result to structured synthesis diagnostics so the planner can distinguish unsupported goal/op pairs from target-derivation failures.
- Replaced flat `DurationEstimateFailed` root skip diagnostics with dependency-tagged failures using the existing planner duration contract.
- Updated planning summary rendering so `dump_agent()` output exposes root omissions and dependency-tagged root candidate skips.
- Added focused search, goal-model, and decision-trace coverage for the new diagnostics.

Deviations from original plan:
- No `agent_tick` integration test was added. The lower-layer search and decision-trace proofs were already strong enough, so expanding the integration surface would have been redundant rather than architectural.
- The implementation reused and exposed `planner_duration_contract.rs` instead of introducing a new ad hoc dependency taxonomy inside the trace model.

Verification results:
- `cargo test -p worldwake-ai`
- `cargo clippy --workspace`
