# S33OPPSCOGOAIDE-011: Make stage-1 decision trace identity opportunity-scoped

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes — decision-trace identity/plumbing only
**Deps**: S33OPPSCOGOAIDE-006, S33OPPSCOGOAIDE-007, S33OPPSCOGOAIDE-010

## Problem

Opportunity-scoped planning is now live in production/runtime state, but the stage-1 decision-trace surfaces that explain candidate generation and ranking still collapse most concrete opportunities back to `GoalKey`. That means sibling opportunities for the same desire can remain indistinguishable in trace output even though the runtime now treats them as distinct identities.

This weakens the repository's debugging contract for explainable emergence: the planner can distinguish orchard-apples from market-apples, but the stage-1 trace still often cannot.

## Assumption Reassessment (2026-03-28)

1. The shared abstraction boundary under audit is the stage-1 planning trace handoff: `CandidateGenerationDiagnostics` in `crates/worldwake-ai/src/candidate_generation.rs`, `ReadPhaseResult` in `crates/worldwake-ai/src/agent_tick/observation.rs`, `CandidateTrace` / `CandidateEvidenceTrace` / `RankedGoalSummary` in `crates/worldwake-ai/src/decision_trace.rs`, `summarize_ranked_goal()` in `crates/worldwake-ai/src/agent_tick/planning.rs`, and ranking-comparison provenance in `crates/worldwake-ai/src/ranking.rs`.
2. Live runtime identity is already opportunity-scoped where it matters behaviorally. `GroundedGoal` carries `anchor` in `crates/worldwake-ai/src/goal_model.rs`; plan attempts already record `opportunity_anchor` in `PlanAttemptTrace` inside `crates/worldwake-ai/src/decision_trace.rs`; and chosen runtime plans carry `PlannedPlan.opportunity` from the shipped S33 architecture.
3. The remaining trace collapse is concrete and live, not hypothetical:
   - `CandidateGenerationDiagnostics.evidence` is keyed by `GoalKey`
   - `ReadPhaseResult.generated_keys` is `Vec<GoalKey>`
   - `CandidateTrace.generated` is `Vec<GoalKey>`
   - `CandidateEvidenceTrace` stores `goal: GoalKey`
   - `RankedGoalSummary` stores `goal: GoalKey`
   - `RankedGoalComparison` stores `winner: GoalKey` / `loser: GoalKey`
4. Because of that collapse, two sibling opportunities with the same desire can still be ambiguous in generated/evidence/ranked/comparison trace surfaces even when focused runtime/search surfaces distinguish them. This is a missing focused runtime trace/integration proof surface, not a missing golden/E2E surface.
5. Existing focused coverage already proves the architectural context and should be treated as baseline:
   - `candidate_generation::tests::acquire_multi_source_emits_distinct_place_anchors_and_isolated_evidence`
   - `candidate_generation::tests::blocked_acquire_place_only_suppresses_matching_opportunity`
   - `decision_trace::tests::summary_planning_includes_desire_fully_blocked`
6. No active remaining ticket owns this work:
   - `tickets/S33OPPSCOGOAIDE-008-save-load.md` is persistence-only.
   - `tickets/S33OPPSCOGOAIDE-009-golden-tests.md` is golden E2E proof and can rely on current lower-layer runtime/search surfaces without taking on trace-architecture refactoring.
7. This ticket must remain trace-only. It must not change candidate emission, blocker filtering, ranking arithmetic, planning admission, or authoritative behavior.
8. Adjacent contradiction exposed by reassessment: once ranked candidate summaries become opportunity-scoped, `SelectionTrace` can no longer safely recover selected ranked provenance by matching only `GoalKey`. Carrying canonical selected opportunity identity in the trace is a required consequence of this ticket, not a separate optional cleanup.

## Architecture Check

1. The canonical trace identity for a concrete opportunity should be `OpportunityKey`, because that is now the canonical runtime identity for concrete opportunity-scoped planning. Keeping goal-only trace identity in stage-1 would preserve a weaker alias path after the architecture already moved on.
2. The clean design is to make generated/evidence/ranked/comparison surfaces opportunity-scoped, then derive desire-level helper answers such as `GoalTraceStatus` from those opportunity-scoped records. This preserves the useful desire-level API without duplicating canonical identity.
3. Carrying selected opportunity identity in `SelectionTrace` is cleaner than letting summary helpers guess by scanning the first ranked entry with a matching `GoalKey`.
4. No backward-compatibility shims or duplicate goal-only trace mirrors.

## Verification Layers

1. Candidate-generation trace records one generated/evidence identity per concrete opportunity -> focused candidate-generation diagnostics test.
2. Ranked stage-1 trace preserves sibling opportunity identity and ranking comparison provenance -> focused decision-trace / planning-trace test.
3. Desire-level helper APIs such as `goal_status_at()` still return the same desire-level answers by derivation over opportunity-scoped trace state -> focused decision-trace helper test.
4. Human-readable summary/debug output names the concrete selected/ranked opportunity when sibling desires share the same `GoalKey` -> focused formatting test.
5. Additional golden coverage is not required here because `tickets/S33OPPSCOGOAIDE-009-golden-tests.md` owns the end-to-end switching proof.

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
6. Existing suite: `cargo test -p worldwake-ai`
7. Existing suite: `cargo clippy --workspace`
8. Existing suite: `cargo test --workspace`

### Invariants

1. `OpportunityKey` is the canonical trace identity for concrete candidate-stage opportunities.
2. Desire-level helper/query APIs are derived views, not parallel sources of truth.
3. No behavioral planner/runtime semantics change under this ticket.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/candidate_generation.rs` — `candidate_generation_trace_identity_is_opportunity_scoped`
   Rationale: proves emitted candidate/evidence diagnostics no longer collapse sibling opportunities onto one `GoalKey`.
2. `crates/worldwake-ai/src/decision_trace.rs` — `ranked_candidate_trace_identity_is_opportunity_scoped`
   Rationale: proves ranked summaries and comparison provenance remain distinguishable for same-desire siblings.
3. `crates/worldwake-ai/src/decision_trace.rs` — `goal_status_helpers_derive_desire_level_answers_from_opportunity_scoped_trace`
   Rationale: proves desire-level helper ergonomics survive the identity refactor without duplicate truth paths.
4. `crates/worldwake-ai/src/decision_trace.rs` — `summary_planning_identifies_selected_opportunity_for_same_goal_siblings`
   Rationale: proves human-readable summary/debug output no longer guesses the winning sibling by `GoalKey`.

### Commands

1. `cargo test -p worldwake-ai candidate_generation::tests::acquire_multi_source_emits_distinct_place_anchors_and_isolated_evidence`
2. `cargo test -p worldwake-ai candidate_generation::tests::blocked_acquire_place_only_suppresses_matching_opportunity`
3. `cargo test -p worldwake-ai decision_trace::tests::summary_planning_includes_desire_fully_blocked`
4. `cargo test -p worldwake-ai`
5. `cargo clippy --workspace`
6. `cargo test --workspace`
