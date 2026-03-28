# S33OPPSCOGOAIDE-007: Add DesireFullyBlocked diagnostic to decision trace

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: Yes — new variant in decision trace types
**Deps**: S33OPPSCOGOAIDE-003

## Problem

When all opportunities for a `GoalKey` are blocked in the two-pass filter (S33OPPSCOGOAIDE-003), this should be recorded in the decision trace for debugging (P27 — Debuggability Is a Product Feature). Currently no trace variant captures desire-level escalation from per-opportunity blocking.

## Assumption Reassessment (2026-03-28)

1. `DecisionOutcome` at `crates/worldwake-ai/src/decision_trace.rs:77-91` has variants: `Dead`, `ActiveAction { ... }`, `Planning(Box<PlanningPipelineTrace>)`. The `Planning` variant carries a `PlanningPipelineTrace` which includes candidate generation diagnostics.
2. `CandidateGenerationDiagnostics` exists in `decision_trace.rs` — this is where per-generation diagnostic data is collected. This is the natural place to record desire-level escalation.
3. The two-pass filter (S33OPPSCOGOAIDE-003) returns filtered-out candidates — this data can be aggregated into the diagnostic.
4. This is a diagnostic-only ticket — no behavioral change to the planning pipeline.
5. Single-layer ticket (decision trace types only).

## Architecture Check

1. Adding a diagnostic field to `CandidateGenerationDiagnostics` (or a new sub-struct) is cleaner than adding a new `DecisionOutcome` variant, because desire-level escalation is part of the candidate generation phase, not a top-level outcome.
2. No backward-compatibility shims.

## Verification Layers

1. Trace records `DesireFullyBlocked` when all opportunities blocked → focused unit test with decision tracing enabled.
2. Trace does NOT record escalation when at least one opportunity survives → focused unit test.
3. Single-layer ticket; additional layer mapping not applicable.

## What to Change

### 1. Add `DesireFullyBlocked` diagnostic struct

```rust
#[derive(Debug, Clone)]
pub struct DesireFullyBlocked {
    pub goal_key: GoalKey,
    pub blocked_opportunities: Vec<OpportunityKey>,
}
```

### 2. Add field to `CandidateGenerationDiagnostics`

Add `pub fully_blocked_desires: Vec<DesireFullyBlocked>` to the existing diagnostics struct.

### 3. Populate during two-pass filter

After the Pass 2 filter in S33OPPSCOGOAIDE-003, aggregate filtered-out candidates by `GoalKey`. For each `GoalKey` where ALL opportunities were blocked, create a `DesireFullyBlocked` entry.

### 4. Include in trace output

Ensure `dump_agent()` and `summary()` on the decision trace include fully-blocked desires for debugging.

## Files to Touch

- `crates/worldwake-ai/src/decision_trace.rs` (modify — add `DesireFullyBlocked` struct, add field to diagnostics)
- `crates/worldwake-ai/src/candidate_generation.rs` (modify — populate `fully_blocked_desires` during two-pass filter)
- `crates/worldwake-ai/src/agent_tick/candidates.rs` (modify — pass diagnostics through pipeline if needed)

## Out of Scope

- Behavioral changes to planning pipeline
- New `DecisionOutcome` variants
- Changes to `BlockedIntentMemory`
- Golden tests (S33OPPSCOGOAIDE-009)
- Save/load (diagnostics are runtime-only, not persisted)

## Acceptance Criteria

### Tests That Must Pass

1. Decision trace records `DesireFullyBlocked` when all opportunities for a `GoalKey` are blocked.
2. Decision trace does NOT record `DesireFullyBlocked` when at least one opportunity survives.
3. `dump_agent()` output includes fully-blocked desire information.
4. Existing suite: `cargo test -p worldwake-ai`
5. Existing suite: `cargo clippy --workspace`

### Invariants

1. `DesireFullyBlocked` is diagnostic only — no behavioral change to the planning pipeline.
2. Trace data is opt-in (zero-cost when tracing disabled).
3. `fully_blocked_desires` is empty when no desire-level escalation occurs.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/decision_trace.rs` or `candidate_generation.rs` — `test_trace_desire_fully_blocked` — all opportunities blocked, trace records it.
2. `test_trace_no_escalation_when_partial` — one opportunity survives, no escalation recorded.

### Commands

1. `cargo test -p worldwake-ai -- desire_fully_blocked`
2. `cargo test -p worldwake-ai -- decision_trace`
3. `cargo clippy --workspace && cargo test --workspace`
