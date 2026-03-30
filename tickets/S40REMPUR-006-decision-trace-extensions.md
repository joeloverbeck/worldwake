# S40REMPUR-006: Decision-trace extensions for remote pursuit

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Small
**Deps**: S40REMPUR-004 (remote candidates exist), S40REMPUR-005 (invalidation exists)

## Problem

Remote pursuit decisions are invisible in the current decision trace. Without explicit tracing, debugging "why did the bandit pursue / not pursue?" requires ad-hoc instrumentation. The spec requires traces that show: why a remote pursuit goal was emitted or omitted, what believed place anchored the chase, what derived confidence was computed and whether it met the threshold, whether omission was due to unknown place / low confidence / over-range route / blocked memory, and when a running pursuit was invalidated.

## Assumption Reassessment (2026-03-30)

1. `DecisionTraceSink` and `AgentDecisionTrace` are in `decision_trace.rs`.
2. `CandidateGenerationDiagnostics` already records per-candidate evidence including `CandidateEvidenceTrace` with `knowledge_path` fields.
3. `TravelPruningTrace` at `decision_trace.rs:~723` already traces Travel destination pruning decisions.
4. `GoalTraceStatus` enum provides disposition categories (Blocked, Satisfied, etc.).
5. The spec says to reuse existing decision-trace patterns — no ad hoc logging.
6. The key new information to trace is:
   - Remote target belief (place, source, observed_tick)
   - Derived confidence value and comparison to min_location_confidence
   - Route cost and comparison to max_pursuit_travel_ticks
   - Blocker check result
   - Invalidation reason during active pursuit
7. No adjacent contradictions exposed.

## Architecture Check

1. Extending existing `CandidateGenerationDiagnostics` and `CandidateEvidenceTrace` is cleaner than adding a parallel trace system. The information naturally fits the existing "why was this candidate emitted/omitted?" pattern.
2. Invalidation traces fit naturally into the existing replan-reason tracking in `agent_tick`.
3. No backwards-compatibility shims.

## Verification Layers

1. Trace emission → focused test: emit remote candidate, verify trace includes belief/confidence/route data
2. Trace omission → focused test: omit remote candidate, verify trace includes omission reason
3. Invalidation trace → focused test: invalidate pursuit, verify trace records reason
4. Single-layer ticket (trace extension); no authoritative state changes.

## What to Change

### 1. Extend candidate-generation diagnostics

In `candidate_generation.rs`, when evaluating a remote pursuit candidate:
- Record the `PursuitTargetBelief` (target, believed_place, source, observed_tick)
- Record the derived confidence value
- Record whether confidence met `min_location_confidence` threshold
- Record route cost and whether it met `max_pursuit_travel_ticks`
- Record blocker check result
- On omission, record which check failed (unknown place, low confidence, over-range, blocked)

Use existing `CandidateEvidenceTrace` or extend it with a new optional `PursuitDiagnostic` field.

### 2. Extend decision trace for pursuit invalidation

In the revalidation/dirty-flag path (S40REMPUR-005):
- When a pursuit is invalidated, record the reason (place changed, confidence decayed, target dead, target not hostile) in the existing trace sink.

### 3. Ensure `dump_agent()` renders pursuit trace data

Verify that `DecisionTraceSink::dump_agent()` renders the new pursuit fields in a human-readable format for debugging.

## Files to Touch

- `crates/worldwake-ai/src/decision_trace.rs` (modify) — add pursuit diagnostic fields
- `crates/worldwake-ai/src/candidate_generation.rs` (modify) — emit pursuit trace data during evaluation
- `crates/worldwake-ai/src/plan_revalidation.rs` (modify) — emit invalidation trace data

## Out of Scope

- Golden tests (S40REMPUR-007)
- Any changes to trace rendering format beyond adding pursuit fields
- Action-trace extensions (pursuit uses standard Travel + Attack actions; existing action traces suffice)
- New trace sink types or parallel trace infrastructure

## Acceptance Criteria

### Tests That Must Pass

1. Decision trace for emitted remote pursuit candidate includes: believed place, source, observed_tick, derived confidence, route cost.
2. Decision trace for omitted remote pursuit includes the specific omission reason.
3. Decision trace for invalidated pursuit includes the invalidation reason.
4. `dump_agent()` renders pursuit trace data without panic.
5. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. Trace data is opt-in and zero-cost when tracing is disabled.
2. No authoritative state is created or modified by tracing.
3. Existing trace patterns are reused, not duplicated.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/decision_trace.rs` (test module) — `test_pursuit_trace_emitted_candidate`, `test_pursuit_trace_omitted_candidate`, `test_pursuit_invalidation_trace`

### Commands

1. `cargo test -p worldwake-ai decision_trace`
2. `cargo test -p worldwake-ai pursuit`
3. `cargo clippy -p worldwake-ai && cargo test -p worldwake-ai`
