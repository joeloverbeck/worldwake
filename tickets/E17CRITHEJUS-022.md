# E17CRITHEJUS-022: Add planner traceability for omitted root operators and missing prerequisites

**Status**: PENDING
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
3. The shared abstraction boundary under audit is: `GoalKind::relevant_op_kinds()` -> root candidate surfacing in `search/candidates.rs` / `search/mod.rs` -> successor construction in `search/transition.rs` -> rendering in `decision_trace.rs`.
4. The intended invariant is: planner traces must distinguish absent relevant operators, filtered surfaced operators, skipped surfaced operators, and named missing prerequisites well enough to explain common planner failures without source-diving into unrelated modules.
5. The live operator families that motivated this gap include at least `PlannerOpKind::Tell`, `PlannerOpKind::Investigate`, `PlannerOpKind::PressForceClaim`, `PlannerOpKind::Trade`, and `PlannerOpKind::Accuse`, because those are the exact-goal surfaces where “missing root operator” was hardest to diagnose.
6. This is a traceability ticket, not a planner behavior ticket. It must not change planner legality or search semantics except where additional structured error information is needed to report the existing behavior accurately.
7. The current missing-prerequisite trace gap is concrete, not hypothetical. `RootCandidateSkipReason::DurationEstimateFailed` exists, but it does not identify whether the failure came from missing investigation profile, missing trade profile, missing reachable target, or some other planner-visible dependency.
8. Existing trace-focused coverage already proves nearby behavior. Examples from `cargo test -p worldwake-ai -- --list` include `agent_tick::tests::trace_social_resend_omission_reason` and multiple `decision_trace::tests::*`, but there is no focused coverage for “relevant operator omitted before root-candidate creation” or for named duration prerequisite diagnostics.
9. If `E17CRITHEJUS-020` changes the root-candidate contract and `E17CRITHEJUS-021` tightens snapshot completeness, this ticket must reflect those final contracts rather than documenting the old implicit behavior.
10. Adjacent contradictions are out of scope here:
    - changing which operators are legally surfaced belongs to `E17CRITHEJUS-020`
    - preserving planner-visible state belongs to `E17CRITHEJUS-021`
11. Mismatch + correction: the current traces are not “missing,” but they are missing categories. The corrected scope is to add first-class provenance for omitted relevant operators and named missing prerequisites, not to replace the entire trace system.

## Architecture Check

1. The clean fix is to extend planner traceability at the exact failure boundaries where the architecture currently becomes opaque: operator surfacing and prerequisite resolution.
2. That is cleaner than adding more debug dumps or asserting on downstream golden absence, because the trace then explains the planner’s own failure modes directly.
3. This aligns with `docs/FOUNDATIONS.md`: explainable emergence, local causality, and explicit world/planner state over hidden reasoning shortcuts.
4. No compatibility aliases or fallback behavior should be introduced. This ticket improves explanation, not semantics.

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
- no affordance surfaced
- no goal-derived synthesis path
- goal-derived synthesis attempted but failed binding/target derivation

### 2. Add named missing-prerequisite diagnostics

Refine root-candidate skip diagnostics so `DurationEstimateFailed` and similar failures can identify the missing planner-visible prerequisite class when the planner knows it.

### 3. Update rendering and focused tests

Update trace formatting and focused tests so the new categories are visible in `dump_agent()` output and asserted in planner/search unit coverage.

## Files to Touch

- `crates/worldwake-ai/src/decision_trace.rs` (modify)
- `crates/worldwake-ai/src/search/candidates.rs` (modify)
- `crates/worldwake-ai/src/search/mod.rs` (modify)
- `crates/worldwake-ai/src/search/transition.rs` (modify)
- `crates/worldwake-ai/src/search/tests.rs` (modify)
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

1. `crates/worldwake-ai/src/search/tests.rs` — add focused coverage for omitted relevant operators and named prerequisite failures at the search boundary.
2. `crates/worldwake-ai/src/decision_trace.rs` — add rendering and data-shape tests for the new omission/diagnostic categories.
3. `crates/worldwake-ai/src/agent_tick/tests.rs` — strengthen end-to-end trace assertions only where lower-layer coverage needs one integration proof.

### Commands

1. `cargo test -p worldwake-ai decision_trace::tests`
2. `cargo test -p worldwake-ai search::tests`
3. `cargo test -p worldwake-ai`
