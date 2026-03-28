# S33OPPSCOGOAIDE-007: Add DesireFullyBlocked diagnostic to decision trace

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: Yes — decision trace diagnostics only
**Deps**: S33OPPSCOGOAIDE-002

## Problem

When all opportunities for a `GoalKey` are blocked by the live post-emission blocker filter, the decision trace does not currently expose that desire-level outcome clearly. The trace can show that no candidate survived, but it does not summarize that every concrete opportunity for one desire was suppressed. That weakens the debugging contract for opportunity-scoped planning.

## Assumption Reassessment (2026-03-28)

1. `DecisionOutcome::Planning` already carries candidate-generation diagnostics in `crates/worldwake-ai/src/decision_trace.rs`. This remains the correct layer for the new debug surface.
2. The stale assumption that a separate two-pass filter return payload exists is no longer accurate. The blocker filter behavior shipped inside archived `S33OPPSCOGOAIDE-002`; this ticket must derive diagnostics from the live filtering path instead of assuming a separate output channel.
3. Archived `S33OPPSCOGOAIDE-010` already strengthened `PlanAttemptTrace` with per-attempt `OpportunityAnchor` provenance in `crates/worldwake-ai/src/decision_trace.rs`. The remaining gap is not "which concrete opportunity was searched" but the aggregated diagnostic "all emitted opportunities for this desire were blocked before search."
4. This is diagnostic-only work. It must not change candidate selection, blocker memory, or ranking behavior.
5. The canonical fact being surfaced is: "all known opportunities for this `GoalKey` were blocked this tick." It should be carried once in tracing, not reconstructed ad hoc in every test.

## Architecture Check

1. A diagnostic field on candidate-generation/planning trace data is cleaner than a new top-level `DecisionOutcome` variant because the behavior occurs inside candidate filtering, not after the pipeline has chosen a different outcome class.
2. No backward-compatibility shims.

## Verification Layers

1. Trace records desire-level full blocking when all opportunities are blocked -> focused trace test.
2. Trace does not record it when at least one opportunity survives -> focused trace test.
3. Human-readable dump includes the same information -> focused formatting assertion if practical.

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

Ensure `dump_agent()` and any relevant trace summary/debug output include the fully blocked desire information.

## Files to Touch

- `crates/worldwake-ai/src/decision_trace.rs` (modify — add diagnostic type/field)
- `crates/worldwake-ai/src/candidate_generation.rs` and/or live candidate-filtering site (modify — populate the diagnostic from current blocker filtering)
- `crates/worldwake-ai/src/agent_tick/candidates.rs` (modify if diagnostics plumbing needs adjustment)

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
3. `dump_agent()` or equivalent trace debug output includes the diagnostic.
4. Existing suite: `cargo test -p worldwake-ai`
5. Existing suite: `cargo clippy --workspace`

### Invariants

1. `DesireFullyBlocked` is diagnostic only.
2. Trace data remains opt-in.
3. The diagnostic is emitted only when the full set of emitted opportunities for a desire was blocked this tick.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/decision_trace.rs` or candidate-filter tests — `trace_records_desire_fully_blocked`
2. `trace_omits_desire_fully_blocked_when_one_opportunity_survives`
3. Trace dump formatting test if a human-readable printer is updated

### Commands

1. `cargo test -p worldwake-ai -- desire_fully_blocked`
2. `cargo test -p worldwake-ai -- decision_trace`
3. `cargo clippy --workspace`
4. `cargo test --workspace`
