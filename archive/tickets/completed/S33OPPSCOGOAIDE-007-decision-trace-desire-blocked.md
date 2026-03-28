# S33OPPSCOGOAIDE-007: Add DesireFullyBlocked diagnostic to decision trace

**Status**: ✅ COMPLETED
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: Yes — decision trace diagnostics only
**Deps**: S33OPPSCOGOAIDE-002

## Problem

When all opportunities for a `GoalKey` are blocked by the live post-emission blocker filter, the decision trace does not currently expose that desire-level outcome clearly. The trace can show that no candidate survived, but it does not summarize that every concrete opportunity for one desire was suppressed. That weakens the debugging contract for opportunity-scoped planning.

## Assumption Reassessment (2026-03-28)

1. The shared abstraction boundary under audit is the handoff from candidate generation into planning trace construction: `generate_candidates_with_travel_horizon()` in `crates/worldwake-ai/src/candidate_generation.rs`, `ReadPhaseResult` in `crates/worldwake-ai/src/agent_tick/observation.rs`, and `CandidateTrace` in `crates/worldwake-ai/src/decision_trace.rs`.
2. The live blocker path is already post-emission and opportunity-aware. `generate_candidates_with_travel_horizon()` emits all `GroundedGoal`s, then `filter_blocked_candidates()` removes blocked siblings via `candidate_matches_blocker()` in `crates/worldwake-ai/src/candidate_generation.rs`. The older narrative that candidate generation still relies on the global `BlockedIntentMemory::is_blocked(..., None, None, None, ...)` shortcut is stale.
3. Opportunity-scoped planning already shipped beyond the original spec baseline. Live code already has `OpportunityAnchor`/`OpportunityKey` in `crates/worldwake-core/src/goal.rs`, opportunity-scoped exhaustion in `crates/worldwake-ai/src/decision_runtime.rs`, and per-attempt `opportunity_anchor` trace provenance in `PlanAttemptTrace` inside `crates/worldwake-ai/src/decision_trace.rs`.
4. The remaining trace gap is narrower: stage-1 candidate trace data still collapses most surfaces to `GoalKey`. `CandidateGenerationDiagnostics.evidence` is keyed by `GoalKey`, `ReadPhaseResult.generated_keys` stores `Vec<GoalKey>`, and `CandidateTrace.generated` is also `Vec<GoalKey>`. That means the trace can show that a desire was generated or omitted, but not explicitly summarize that every emitted sibling opportunity for that desire was removed by the blocker filter before ranking.
5. Existing focused coverage already proves the live blocker/filter architecture and must be treated as baseline, not planned work: `candidate_generation::tests::acquire_multi_source_emits_distinct_place_anchors_and_isolated_evidence` and `candidate_generation::tests::blocked_acquire_place_only_suppresses_matching_opportunity` in `crates/worldwake-ai/src/candidate_generation.rs`, plus `decision_trace::tests::summary_planning_includes_attempt_anchor` in `crates/worldwake-ai/src/decision_trace.rs`.
6. This ticket remains diagnostic-only. It must not change candidate emission, blocker memory semantics, ranking, search admission, or plan selection. If reassessment finds that a richer per-opportunity stage-1 trace model is needed later, that is follow-up architecture work, not hidden scope for this ticket.
7. The canonical fact to surface here is: "this `GoalKey` emitted one or more concrete opportunities this tick, and the live blocker filter removed all of them before ranking." That fact should be recorded once in trace data and reused by tests/debug output instead of being reconstructed ad hoc from raw candidates plus blocker state.

## Architecture Check

1. A desire-level diagnostic attached to stage-1 candidate trace data is cleaner than a new top-level `DecisionOutcome` variant because the phenomenon happens inside candidate filtering, before ranking/search/selection decide anything.
2. The cleanest minimal implementation is to derive the diagnostic directly at the live filter boundary, carry it through `ReadPhaseResult`, and expose it on `CandidateTrace`. Reconstructing it later from ranked output or plan attempts would be lossy and would couple tests to incidental pipeline details.
3. This ticket should not broaden into a full per-opportunity stage-1 trace refactor. That could become a good follow-up if the team wants complete candidate-stage opportunity provenance, but it is larger than the current debugging gap.
4. No backward-compatibility shims or alias fields.

## Verification Layers

1. Live blocker filter emits desire-level full-blocking diagnostics only when every emitted sibling opportunity for one `GoalKey` is removed -> focused candidate-generation test over `CandidateGenerationDiagnostics`.
2. Planning trace carries the same diagnostic from read phase into `DecisionOutcome::Planning` -> focused `agent_tick`/decision-trace construction test.
3. Human-readable trace output includes the diagnostic -> focused formatting assertion on `format_outcome()` or `dump_agent()` helper-facing output.
4. Additional mixed-layer verification is not required because the ticket is trace-only and does not alter authoritative behavior, ranking arithmetic, or planner search semantics.

## What to Change

### 1. Add a desire-level blocked diagnostic

```rust
#[derive(Debug, Clone)]
pub struct DesireFullyBlocked {
    pub goal_key: GoalKey,
    pub blocked_opportunities: Vec<OpportunityKey>,
}
```

### 2. Store it in candidate-generation diagnostics

Add `pub fully_blocked_desires: Vec<DesireFullyBlocked>` to the existing diagnostics struct.

### 3. Populate it from the live blocker-filter implementation

Aggregate blocked opportunities by `GoalKey` after the live post-emission blocker filter runs. Emit a `DesireFullyBlocked` entry only when every emitted opportunity for that desire was filtered out as blocked.

### 4. Include in trace output

Ensure `CandidateTrace` carries the diagnostic and `format_outcome()` / `dump_agent()` expose it in human-readable trace output.

## Files to Touch

- `crates/worldwake-ai/src/decision_trace.rs` (modify — add diagnostic type/field)
- `crates/worldwake-ai/src/candidate_generation.rs` (modify — derive the diagnostic at the live blocker-filter boundary)
- `crates/worldwake-ai/src/agent_tick/observation.rs` (modify — carry diagnostics into read-phase trace data)
- `crates/worldwake-ai/src/agent_tick/mod.rs` (modify — wire the trace field into `CandidateTrace`)

## Out of Scope

- Behavioral changes to planning pipeline
- New `DecisionOutcome` variants
- Changes to `BlockedIntentMemory`
- Golden tests (S33OPPSCOGOAIDE-009)
- Save/load (diagnostics are runtime-only, not persisted)

## Acceptance Criteria

### Tests That Must Pass

1. Decision trace records `DesireFullyBlocked` when all opportunities for a `GoalKey` are blocked.
2. Decision trace does not record it when at least one opportunity survives.
3. `format_outcome()` / `dump_agent()` equivalent trace debug output includes the diagnostic.
4. Existing focused coverage for live opportunity filtering still passes.
5. Existing suite: `cargo test -p worldwake-ai`
6. Existing suite: `cargo clippy --workspace`

### Invariants

1. `DesireFullyBlocked` is diagnostic only.
2. Trace data remains opt-in.
3. The diagnostic is emitted only when the full set of emitted opportunities for a desire was blocked this tick.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/candidate_generation.rs` — `diagnostics_record_desire_fully_blocked_when_all_opportunities_are_filtered`
2. `crates/worldwake-ai/src/candidate_generation.rs` — `diagnostics_omit_desire_fully_blocked_when_one_opportunity_survives`
3. `crates/worldwake-ai/src/decision_trace.rs` — formatting test proving the planning trace summary/debug output includes the diagnostic

### Commands

1. `cargo test -p worldwake-ai diagnostics_record_desire_fully_blocked_when_all_opportunities_are_filtered`
2. `cargo test -p worldwake-ai diagnostics_omit_desire_fully_blocked_when_one_opportunity_survives`
3. `cargo test -p worldwake-ai summary_planning_includes_desire_fully_blocked`
4. `cargo test -p worldwake-ai`
5. `cargo clippy --workspace`
6. `cargo test --workspace`

## Outcome

- Completion date: 2026-03-28
- What actually changed:
  - Added `DesireFullyBlocked` to decision-trace candidate diagnostics and planning trace output.
  - Derived the diagnostic at the live post-emission blocker filter in `crates/worldwake-ai/src/candidate_generation.rs`.
  - Threaded the diagnostic through `ReadPhaseResult` into `CandidateTrace`, and rendered it in `format_outcome()` so `dump_agent()` exposes it.
  - Added focused tests for full-blocked and partial-survivor opportunity sets plus a planning-summary formatting test.
- Deviations from original plan:
  - Kept the change narrowly diagnostic-only; no ranking/search/admission behavior changed.
  - Did not broaden into a stage-1 per-opportunity trace refactor. Reassessment confirmed that would be follow-up architecture work, not required to close this ticket cleanly.
  - Fixed one unrelated test-only clippy violation in `crates/worldwake-ai/src/agent_tick/planning.rs` so workspace lint passes under the repo's strict settings.
- Verification results:
  - `cargo test -p worldwake-ai diagnostics_record_desire_fully_blocked_when_all_opportunities_are_filtered` ✅
  - `cargo test -p worldwake-ai diagnostics_omit_desire_fully_blocked_when_one_opportunity_survives` ✅
  - `cargo test -p worldwake-ai summary_planning_includes_desire_fully_blocked` ✅
  - `cargo test -p worldwake-ai` ✅
  - `cargo clippy --workspace --all-targets --all-features -- -D warnings` ✅
  - `cargo test --workspace` ✅
