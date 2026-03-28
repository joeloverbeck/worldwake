# S33OPPSCOGOAIDE-011: Make stage-1 decision trace identity opportunity-scoped

**Status**: ✅ COMPLETED
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes — decision-trace identity/plumbing only
**Deps**: S33OPPSCOGOAIDE-006, S33OPPSCOGOAIDE-007, S33OPPSCOGOAIDE-010

## Problem

Opportunity-scoped planning is now live in production/runtime state, but the stage-1 decision-trace surfaces that explain candidate generation and ranking still collapse most concrete opportunities back to `GoalKey`. That means sibling opportunities for the same desire can remain indistinguishable in trace output even though the runtime now treats them as distinct identities.

This weakens the repository's debugging contract for explainable emergence: the planner can distinguish orchard-apples from market-apples, but the stage-1 trace still often cannot.

## Assumption Reassessment (2026-03-28)

1. The shared abstraction boundary under audit is the stage-1 planning trace handoff: `CandidateGenerationDiagnostics` in `crates/worldwake-ai/src/candidate_generation.rs`, `ReadPhaseResult` in `crates/worldwake-ai/src/agent_tick/observation.rs`, `CandidateTrace` / `CandidateEvidenceTrace` / `RankedGoalSummary` / `SelectionTrace` in `crates/worldwake-ai/src/decision_trace.rs`, `summarize_ranked_goal()` plus `plans_as_options()` / `determine_selected_plan_source()` in `crates/worldwake-ai/src/agent_tick/planning.rs`, and ranking-comparison provenance in `crates/worldwake-ai/src/ranking.rs`.
2. Live runtime identity is already opportunity-scoped where it matters behaviorally. `GroundedGoal` carries `anchor` in `crates/worldwake-ai/src/goal_model.rs`; plan attempts already record `opportunity_anchor` in `PlanAttemptTrace` inside `crates/worldwake-ai/src/decision_trace.rs`; and chosen runtime plans carry `PlannedPlan.opportunity` from the shipped S33 architecture.
3. The remaining trace collapse is concrete and live, not hypothetical:
   - `CandidateGenerationDiagnostics.evidence` is keyed by `GoalKey`
   - `ReadPhaseResult.generated_keys` is `Vec<GoalKey>`
   - `CandidateTrace.generated` is `Vec<GoalKey>`
   - `CandidateEvidenceTrace` stores `goal: GoalKey`
   - `RankedGoalSummary` stores `goal: GoalKey`
   - `RankedGoalComparison` stores `winner: GoalKey` / `loser: GoalKey`
   - `SelectionTrace.selected` stores only `Option<GoalKey>`
   - `selected_ranked_goal_summary()` in `crates/worldwake-ai/src/decision_trace.rs` recovers ranked provenance by matching that `GoalKey` against `CandidateTrace.ranked`
4. Because of that collapse, two sibling opportunities with the same desire can still be ambiguous in generated/evidence/ranked/comparison/selected-ranked trace surfaces even when focused runtime/search surfaces distinguish them. This is a missing focused runtime trace/integration proof surface, not a missing golden/E2E surface.
5. Existing focused coverage already proves the architectural context and should be treated as baseline:
   - `candidate_generation::tests::acquire_multi_source_emits_distinct_place_anchors_and_isolated_evidence`
   - `candidate_generation::tests::blocked_acquire_place_only_suppresses_matching_opportunity`
   - `candidate_generation::tests::diagnostics_record_desire_fully_blocked_when_all_opportunities_are_filtered`
   - `agent_tick::planning::tests::same_goal_ranked_opportunities_are_attempted_in_order`
   - `agent_tick::planning::tests::traced_planning_records_same_goal_opportunity_attempt_order`
   - `decision_trace::tests::summary_planning_includes_desire_fully_blocked`
6. No active remaining ticket owns this work:
   - `tickets/S33OPPSCOGOAIDE-008-save-load.md` is persistence-only.
   - `tickets/S33OPPSCOGOAIDE-009-golden-tests.md` is golden E2E proof and can rely on current lower-layer runtime/search surfaces without taking on trace-architecture refactoring.
7. The intended verification layer is focused runtime / decision-trace coverage inside `worldwake-ai`. Existing candidate-generation focused tests already prove opportunity-scoped emission and blocker filtering; this ticket adds or adjusts only the trace-facing assertions that still collapse identity.
8. This ticket must remain trace-only. It must not change candidate emission, blocker filtering, ranking arithmetic, planning admission order, or authoritative behavior.
9. Adjacent contradiction exposed by reassessment: once ranked candidate summaries become opportunity-scoped, `SelectionTrace` can no longer safely recover selected ranked provenance by matching only `GoalKey`. Carrying canonical selected opportunity identity in the trace is a required consequence of this ticket, not a separate optional cleanup.
10. Mismatch + correction: the original ticket's proposed tests under new names do not match the live test inventory. The implementation should extend the existing focused tests named above and add narrowly targeted trace tests, rather than introducing redundant duplicate baselines.

## Architecture Check

1. The canonical trace identity for a concrete opportunity should be `OpportunityKey`, because that is now the canonical runtime identity for concrete opportunity-scoped planning. Keeping goal-only trace identity in stage-1 would preserve a weaker alias path after the architecture already moved on.
2. The clean design is to make generated/evidence/ranked/comparison surfaces opportunity-scoped, then derive desire-level helper answers such as `GoalTraceStatus` from those opportunity-scoped records. This preserves the useful desire-level API without duplicating canonical identity.
3. Carrying selected opportunity identity in `SelectionTrace` is cleaner than letting summary helpers guess by scanning the first ranked entry with a matching `GoalKey`.
4. No backward-compatibility shims or duplicate goal-only trace mirrors.

## Verification Layers

1. Candidate-generation trace records one generated/evidence identity per concrete opportunity -> focused candidate-generation diagnostics test.
2. Ranked stage-1 trace preserves sibling opportunity identity and ranking comparison provenance -> focused decision-trace test.
3. Selected ranked provenance resolves through canonical selected opportunity identity, not `GoalKey` guessing -> focused decision-trace / planning-trace test.
4. Desire-level helper APIs such as `goal_status_at()` and `goal_history_for()` still return the same desire-level answers by derivation over opportunity-scoped trace state -> focused decision-trace helper test.
5. Human-readable summary/debug output names the concrete selected/ranked opportunity when sibling desires share the same `GoalKey` -> focused formatting test.
6. Additional golden coverage is not required here because `tickets/S33OPPSCOGOAIDE-009-golden-tests.md` owns the end-to-end switching proof.

## What to Change

### 1. Make generated/evidence candidate identity opportunity-scoped

- Replace `ReadPhaseResult.generated_keys` / `CandidateTrace.generated` with opportunity-scoped generated identities.
- Replace `CandidateGenerationDiagnostics.evidence: BTreeMap<GoalKey, CandidateEvidenceTrace>` with an opportunity-scoped key.
- Replace `CandidateEvidenceTrace.goal` with canonical opportunity identity rather than desire-only identity.

### 2. Make ranked/comparison trace identity opportunity-scoped

- Replace `RankedGoalSummary.goal` with canonical opportunity identity.
- Replace goal-only ranking comparison provenance with opportunity-scoped winner/loser identity so same-desire siblings remain distinguishable.
- Update stage-1 ranking summary/debug output to use the new canonical identity.

### 3. Carry canonical selected opportunity identity in selection trace

- Add selected-opportunity identity to `SelectionTrace` (or an equivalent canonical trace field) so summary helpers do not guess the winning ranked sibling from `GoalKey` alone.
- Update selected-ranked-goal provenance lookup to use that canonical selected opportunity.
- Keep desire-level `selected` / `previous_goal` semantics if they are still useful for trace queries; do not introduce a parallel goal-only source of truth for ranked provenance.

### 4. Preserve desire-level query ergonomics by derivation

- Keep user-facing/helper queries like `goal_status_at()` and `goal_history_for()` desire-level.
- Recompute those answers from opportunity-scoped trace records instead of maintaining duplicate goal-only stage-1 trace state.

## Files to Touch

- `crates/worldwake-ai/src/candidate_generation.rs` (modify — opportunity-scoped candidate diagnostics identity)
- `crates/worldwake-ai/src/agent_tick/observation.rs` (modify — read-phase plumbing)
- `crates/worldwake-ai/src/agent_tick/mod.rs` (modify — trace construction)
- `crates/worldwake-ai/src/agent_tick/planning.rs` (modify — ranked summary construction if needed)
- `crates/worldwake-ai/src/decision_trace.rs` (modify — stage-1/selection trace structs, helper derivation, formatting)
- `crates/worldwake-ai/src/ranking.rs` (modify — opportunity-scoped ranking comparison provenance)
- `crates/worldwake-ai/src/lib.rs` (modify — public re-exports if trace types change)

## Out of Scope

- Candidate-generation behavior changes
- Ranking arithmetic changes
- Search/admission behavior changes
- Save/load format changes
- Golden scenario additions or modifications
- Non-trace runtime state changes

## Acceptance Criteria

### Tests That Must Pass

1. Generated candidate trace identity distinguishes same-desire sibling opportunities.
2. Candidate evidence trace identity distinguishes same-desire sibling opportunities.
3. Ranked candidate summary and ranking-comparison trace identity distinguish same-desire sibling opportunities.
4. Selection trace can identify the canonical winning opportunity without guessing from `GoalKey`.
5. Existing desire-level helper APIs still return correct `GeneratedOnly` / `Suppressed` / `Ranked` semantics.
6. Existing focused same-goal opportunity planning traces still record the same attempt order and no planner/runtime behavior changes.
7. Existing suite: `cargo test -p worldwake-ai`
8. Existing suite: `cargo clippy --workspace`
9. Existing suite: `cargo test --workspace`

### Invariants

1. `OpportunityKey` is the canonical trace identity for concrete candidate-stage opportunities.
2. Desire-level helper/query APIs are derived views, not parallel sources of truth.
3. No behavioral planner/runtime semantics change under this ticket.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/candidate_generation.rs` — modify existing diagnostics assertions in `candidate_generation::tests::acquire_multi_source_emits_distinct_place_anchors_and_isolated_evidence`
   Rationale: the live candidate-generation baseline already proves per-opportunity emission; extending it is the narrowest way to prove the public evidence trace now preserves the same identity.
2. `crates/worldwake-ai/src/decision_trace.rs` — add `decision_trace::tests::selected_ranked_summary_uses_selected_opportunity_for_same_goal_siblings`
   Rationale: proves ranked summaries, comparison provenance, and selected-ranked provenance remain distinguishable for same-desire siblings.
3. `crates/worldwake-ai/src/decision_trace.rs` — keep `decision_trace::tests::goal_status_distinguishes_omitted_suppressed_zero_motive_ranked_and_selected` green after the identity refactor
   Rationale: proves desire-level helper ergonomics survive the identity refactor without duplicate truth paths.
4. `crates/worldwake-ai/src/agent_tick/planning.rs` — keep `agent_tick::planning::tests::traced_planning_records_same_goal_opportunity_attempt_order` green with opportunity-scoped ranked/comparison plumbing
   Rationale: proves traced plan selection carries canonical opportunity identity while preserving existing plan-attempt ordering behavior.

### Commands

1. `cargo test -p worldwake-ai candidate_generation::tests::acquire_multi_source_emits_distinct_place_anchors_and_isolated_evidence`
2. `cargo test -p worldwake-ai candidate_generation::tests::blocked_acquire_place_only_suppresses_matching_opportunity`
3. `cargo test -p worldwake-ai agent_tick::planning::tests::traced_planning_records_same_goal_opportunity_attempt_order`
4. `cargo test -p worldwake-ai decision_trace::tests::goal_status_distinguishes_omitted_suppressed_zero_motive_ranked_and_selected`
5. `cargo test -p worldwake-ai decision_trace::tests::summary_planning_includes_ranking_comparison`
6. `cargo test -p worldwake-ai decision_trace::tests::selected_ranked_summary_uses_selected_opportunity_for_same_goal_siblings`
7. `cargo test -p worldwake-ai`
8. `cargo clippy --workspace`
9. `cargo test --workspace`

## Outcome

- Completion date: 2026-03-28
- Actually changed:
  - Stage-1 trace identity is now opportunity-scoped across generated candidates, candidate evidence, ranked summaries, and ranking comparisons.
  - `SelectionTrace` now carries canonical `selected_opportunity`, and selected-ranked provenance resolution now uses that opportunity identity instead of guessing from `GoalKey`.
  - Desire-level helper APIs remain desire-level by deriving from opportunity-scoped trace records rather than keeping a parallel goal-only source of truth.
  - Focused and golden tests that inspected the old goal-only trace fields were updated to read the new canonical opportunity-scoped surfaces.
- Deviations from original plan:
  - The ticket stayed trace-only as intended, but the finished work also required updating existing focused/golden assertions that consumed the public trace structs directly.
  - Rather than introducing multiple brand-new baseline tests, the implementation strengthened existing focused coverage where that was the cleaner proof surface.
- Verification results:
  - `cargo test -p worldwake-ai candidate_generation::tests::acquire_multi_source_emits_distinct_place_anchors_and_isolated_evidence`
  - `cargo test -p worldwake-ai candidate_generation::tests::blocked_acquire_place_only_suppresses_matching_opportunity`
  - `cargo test -p worldwake-ai agent_tick::planning::tests::traced_planning_records_same_goal_opportunity_attempt_order`
  - `cargo test -p worldwake-ai decision_trace::tests::goal_status_distinguishes_omitted_suppressed_zero_motive_ranked_and_selected`
  - `cargo test -p worldwake-ai decision_trace::tests::summary_planning_includes_ranking_comparison`
  - `cargo test -p worldwake-ai decision_trace::tests::selected_ranked_summary_uses_selected_opportunity_for_same_goal_siblings`
  - `cargo test -p worldwake-ai`
  - `cargo clippy --workspace`
  - `cargo test --workspace`
