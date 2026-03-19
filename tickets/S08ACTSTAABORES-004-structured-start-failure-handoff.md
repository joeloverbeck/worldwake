# S08ACTSTAABORES-004: Replace Trace-Only Start Failures With Canonical AI Execution-Failure Handoff

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — `worldwake-sim` structured start-failure records plus `worldwake-ai` runtime failure handoff and trace plumbing
**Deps**: `S08ACTSTAABORES-001`, `specs/S08-action-start-abort-resilience.md`

## Problem

`worldwake-sim` now records recoverable BestEffort start failures, but `worldwake-ai` still treats them as a trace-only side channel. The AI runtime does not consume the start-failure record as the canonical reason its in-flight step disappeared; instead, `reconcile_in_flight_state()` falls back to generic "no active action and no commit" handling. That loses authoritative rejection semantics, weakens blocker derivation, and encourages string-based or care-specific patches instead of a clean shared execution-failure substrate.

## Assumption Reassessment (2026-03-19)

1. `crates/worldwake-sim/src/scheduler.rs::ActionStartFailure` currently stores `tick`, `actor`, `def_id`, and a `reason: String`. The record does not preserve structured authoritative failure semantics.
2. `crates/worldwake-sim/src/tick_step.rs` now records recoverable BestEffort start failures for `ReservationUnavailable`, `PreconditionFailed`, `InvalidTarget`, and `AbortRequested`, but only into the scheduler plus action trace. No canonical replan/failure signal is emitted from action start.
3. `crates/worldwake-ai/src/agent_tick.rs` currently reads `scheduler.action_start_failures()` only to populate `PlanningPipelineTrace.action_start_failures`. That read path does not drive runtime reconciliation or blocker recording.
4. `crates/worldwake-ai/src/agent_tick.rs::reconcile_in_flight_state()` only handles structured failure semantics through `ReplanNeeded` from the active-action/abort pipeline. When an in-flight step vanishes without an active action, committed action, or `ReplanNeeded`, it calls `handle_current_step_failure(..., None)` and therefore discards the authoritative start-failure reason.
5. `crates/worldwake-ai/src/failure_handling.rs::derive_blocking_fact()` can already derive better blockers when it receives structured abort information through `replan_signal`, but start failures currently reach it without that signal and can therefore collapse to view-based heuristics or `BlockingFact::Unknown`.
6. Existing coverage proves the gap precisely:
   - `crates/worldwake-ai/src/agent_tick.rs::tests::planning_trace_includes_scheduler_start_failures_for_wound_abort_reasons` proves observability in planning traces plus generic reconciliation.
   - `crates/worldwake-ai/src/failure_handling.rs::tests::handle_plan_failure_drops_plan_records_blocker_and_marks_runtime_dirty` proves blocker handling once a structured failure signal exists.
   - No active ticket currently owns replacing the trace-only start-failure side channel with a canonical runtime failure substrate.
7. Mismatch found: `tickets/S08ACTSTAABORES-003-care-start-abort-ai-regression-coverage.md` covers regression assertions over the current behavior, but it does not introduce the architectural substrate. This ticket is needed to prevent the current generic missing-in-flight recovery path from ossifying into the permanent design.

## Architecture Check

1. The clean solution is a single canonical execution-failure handoff that covers both active-step replans and start-time rejections. AI should reconcile failed execution through one structured substrate, not through `ReplanNeeded` in one branch and "missing action" inference in another.
2. The start-failure record should preserve structured authoritative semantics rather than stringified debug output. String reasons are acceptable for human-readable traces, but not as the source of truth for runtime failure handling.
3. No backwards-compatibility shims or dual paths should survive the change. Replace the stringly/trace-only path instead of layering a second interpretation path beside it.

## Verification Layers

1. Recoverable start rejection is stored with structured authoritative semantics -> focused `worldwake-sim` scheduler/tick-step coverage
2. `reconcile_in_flight_state()` consumes matching start-failure records before generic missing-action fallback -> focused `worldwake-ai` runtime test in `agent_tick.rs`
3. Blocker derivation receives the structured authoritative rejection reason for start failures -> focused `worldwake-ai` failure-handling/runtime test
4. Decision traces summarize the canonical failure substrate rather than a side-channel-only list -> focused `worldwake-ai` trace assertion
5. Later replanning or world evolution must not be the only proof here; the ticket must assert the structured failure handoff itself, because that handoff is the architectural contract.

## What to Change

### 1. Replace string-only start-failure records with structured authority data

Refactor `crates/worldwake-sim/src/scheduler.rs::ActionStartFailure` so it carries structured failure information suitable for AI/runtime consumption.

Recommended direction:
- store a narrow structured reason enum for recoverable start failures, or
- store an authoritative execution-failure type that can also cover existing `ReplanNeeded` semantics cleanly.

Do not keep `reason: String` as the authoritative field. A human-readable summary may remain as derived trace output.

### 2. Introduce one canonical execution-failure handoff into AI runtime

Refactor `crates/worldwake-ai/src/agent_tick.rs` so in-flight step reconciliation consumes a structured failure signal regardless of whether the step failed:
- before action start, or
- after action start via `ReplanNeeded`.

Recommended direction:
- generalize `handle_current_step_failure()` away from `Option<&ReplanNeeded>` into a canonical execution-failure input type, and
- teach `reconcile_in_flight_state()` to match a start failure for the current agent/step before falling back to generic missing-action handling.

### 3. Route blocker derivation through the canonical signal

Update `crates/worldwake-ai/src/failure_handling.rs` so start-time authoritative rejection reasons reach blocker derivation through the same structured path used by active-action replans. This should remove the need to infer too much from current-view state when the authority already produced a concrete reason.

### 4. Keep decision traces derived from runtime truth

Update `crates/worldwake-ai/src/decision_trace.rs` and any related summaries so trace output is derived from the same canonical failure signal. Traces should remain human-readable, but they must not be the only place where structured start-failure semantics exist.

## Files to Touch

- `crates/worldwake-sim/src/scheduler.rs` (modify)
- `crates/worldwake-sim/src/tick_step.rs` (modify if the recorder needs to emit the new structured failure type)
- `crates/worldwake-ai/src/agent_tick.rs` (modify)
- `crates/worldwake-ai/src/failure_handling.rs` (modify)
- `crates/worldwake-ai/src/decision_trace.rs` (modify)
- `crates/worldwake-ai/src/lib.rs` (modify only if public trace/runtime types move)

## Out of Scope

- `worldwake-systems` heal medicine-conservation work from `S08ACTSTAABORES-002`
- Broadening the set of recoverable start failures beyond the lawful-drift boundary already defined by S08
- Candidate-generation, ranking, or goal-policy redesign unrelated to execution-failure propagation
- Ad-hoc parsing of debug strings as a permanent solution

## Acceptance Criteria

### Tests That Must Pass

1. Existing focused start-failure behavior still passes: `cargo test -p worldwake-sim tick_step::tests::best_effort_request_records_abort_requested_start_failure_without_failing_tick`
2. Existing focused AI observability still passes under the new substrate: `cargo test -p worldwake-ai agent_tick::tests::planning_trace_includes_scheduler_start_failures_for_wound_abort_reasons`
3. Existing blocker pipeline baseline still passes: `cargo test -p worldwake-ai failure_handling::tests::handle_plan_failure_drops_plan_records_blocker_and_marks_runtime_dirty`
4. Crate suites: `cargo test -p worldwake-sim` and `cargo test -p worldwake-ai`
5. Workspace lint/test boundary: `cargo test --workspace` and `cargo clippy --workspace`

### Invariants

1. Start-time authoritative rejection semantics are preserved as structured runtime data, not reduced to trace-only strings.
2. AI execution-failure handling uses one canonical handoff for both start-time and active-step failures.
3. Blocker derivation prefers authoritative failure semantics when they exist instead of inferring solely from the current belief/world snapshot.
4. No care-specific or action-specific recovery special case is introduced to paper over the missing substrate.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-sim/src/scheduler.rs` and/or `crates/worldwake-sim/src/tick_step.rs` — assert structured start-failure recording rather than string-only storage.
2. `crates/worldwake-ai/src/agent_tick.rs` — assert that matching start failures are consumed by in-flight reconciliation before generic missing-action fallback.
3. `crates/worldwake-ai/src/failure_handling.rs` — assert blocker derivation receives structured start-failure reasons through the canonical execution-failure path.
4. `crates/worldwake-ai/src/decision_trace.rs` or `agent_tick.rs` — assert trace summaries are derived from the same canonical failure substrate.

### Commands

1. `cargo test -p worldwake-ai -- --list`
2. `cargo test -p worldwake-sim tick_step::tests::best_effort_request_records_abort_requested_start_failure_without_failing_tick`
3. `cargo test -p worldwake-ai agent_tick::tests::planning_trace_includes_scheduler_start_failures_for_wound_abort_reasons`
4. `cargo test -p worldwake-ai failure_handling::tests::handle_plan_failure_drops_plan_records_blocker_and_marks_runtime_dirty`
5. `cargo test -p worldwake-sim`
6. `cargo test -p worldwake-ai`
7. `cargo test --workspace`
8. `cargo clippy --workspace`
