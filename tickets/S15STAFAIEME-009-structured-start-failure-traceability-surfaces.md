# S15STAFAIEME-009: Structured Start-Failure Traceability Surfaces

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — traceability surface improvements in `worldwake-sim` and `worldwake-ai`, no authority-path behavior changes
**Deps**: `docs/FOUNDATIONS.md`, `docs/golden-e2e-testing.md`, archived `S15STAFAIEME-001`

## Problem

S15 implementation exposed a traceability gap rather than a behavior bug. To fully explain a lawful start failure today, tests often need to combine three surfaces:

1. action trace for the lifecycle fact that `StartFailed` happened
2. scheduler start-failure records for the structured authoritative reason
3. decision trace on the next tick for AI reconciliation and plan replacement

That works, but it is more fragmented than it should be. Two concrete missing data points stood out:

- `ActionTraceKind::StartFailed` only stores a debug string, not structured reason data
- `StartFailed` does not carry attempted targets, so same-tick contested-entity debugging requires extra cross-referencing

This is a traceability gap. The behavior is already correct.

## Assumption Reassessment (2026-03-19)

1. Current action trace data in [crates/worldwake-sim/src/action_trace.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/action_trace.rs) stores `ActionTraceKind::StartFailed { reason: String }` only. There is no structured reason enum or attempted-target list on failed starts.
2. Current tick-step recording in [crates/worldwake-sim/src/tick_step.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/tick_step.rs) already has the source data needed at record time:
   - `affordance.bound_targets`
   - structured `ActionStartFailureReason`
3. Current decision trace data in [crates/worldwake-ai/src/decision_trace.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/decision_trace.rs) already exposes:
   - `planning.action_start_failures`
   - `planning.selection.selected_plan`
   - `planning.selection.selected_plan_source`
   - `selected_plan.next_step`
   This is useful for next-tick reconciliation, but it does not by itself make the failed same-tick attempted targets visible at the action-trace layer.
4. Existing action-trace tests in `worldwake-sim` cover ordering and summaries, but none prove structured failed-start target capture because that surface does not exist yet:
   - `action_trace::tests::summary_format_covers_all_variants`
   - `tick_step::tests::action_trace_assigns_explicit_order_across_started_failed_and_committed_events`
5. This ticket targets traceability surfaces, not simulation semantics. It should not change authoritative behavior, planner selection logic, or blocker handling.
6. This aligns with `docs/FOUNDATIONS.md`: making the world’s causal record more legible is an architectural improvement, not instrumentation garnish.
7. Scope correction: if a proposed traceability field would duplicate authoritative truth in a way that can drift, prefer storing canonical structured references already present at record time rather than derived summaries.

## Architecture Check

1. Extending trace events with structured failure data is cleaner than leaving tests to stitch together string summaries and scheduler internals. It creates a single first-class record of "what failed to start, against which targets, and why."
2. This should be additive to observability, not additive to behavior. The authoritative rule path remains unchanged; only its trace representation becomes more complete.
3. No backward-compatibility shim is needed. Update the trace structs and their tests directly, then update any downstream test helpers and docs that rely on the old shape.

## Verification Layers

1. Same-tick failed start records structured reason and attempted targets -> action trace tests in `worldwake-sim`.
2. Scheduler and action trace remain consistent for the same failure -> focused tick-step test in `worldwake-sim`.
3. AI next-tick reconciliation still reads scheduler failures as before -> existing `worldwake-ai` trace/runtime tests plus one guardrail if needed.
4. Golden guidance reflects the stronger trace surface -> docs update in `docs/golden-e2e-testing.md`.

## What to Change

### 1. Upgrade `ActionTraceKind::StartFailed`

Revise [crates/worldwake-sim/src/action_trace.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/action_trace.rs) so failed starts can carry:

- structured `ActionStartFailureReason`
- attempted target ids for the failed affordance

Prefer a shape that preserves canonical structured data rather than only a rendered string. Summary formatting can still render a readable string from that data.

### 2. Record the richer data at tick-step time

Update [crates/worldwake-sim/src/tick_step.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/tick_step.rs) to record the structured reason and attempted targets when `BestEffort` start failure is converted into a trace event.

### 3. Add focused traceability tests

Add or update tests proving:

- `StartFailed` events preserve structured reason and targets
- same-tick ordering still works with the richer event shape
- scheduler failure records and action traces agree on actor/def/reason

### 4. Update golden testing guidance

Revise [docs/golden-e2e-testing.md](/home/joeloverbeck/projects/worldwake/docs/golden-e2e-testing.md) so the recommended assertion surface for start-failure goldens reflects the richer trace model once it exists.

## Files to Touch

- `crates/worldwake-sim/src/action_trace.rs` (modify)
- `crates/worldwake-sim/src/tick_step.rs` (modify)
- `docs/golden-e2e-testing.md` (modify)
- `crates/worldwake-ai/tests/golden_production.rs` (modify only if the new trace fields enable cleaner assertions)
- `crates/worldwake-ai/tests/golden_care.rs` (modify only if the new trace fields enable cleaner assertions)

## Out of Scope

- changing reservation, validation, scheduler, or AI failure-handling behavior
- adding speculative or derived trace fields that can drift from authoritative truth
- redesigning decision traces beyond what is necessary to consume the richer action trace surface

## Acceptance Criteria

### Tests That Must Pass

1. New `worldwake-sim` tests proving `StartFailed` preserves structured reason and attempted targets
2. `cargo test -p worldwake-sim tick_step::tests::action_trace_assigns_explicit_order_across_started_failed_and_committed_events -- --exact`
3. `cargo test -p worldwake-sim tick_step::tests::best_effort_request_records_abort_requested_start_failure_without_failing_tick -- --exact`
4. `cargo test -p worldwake-ai golden_contested_harvest_start_failure_recovers_via_remote_fallback -- --exact`
5. `cargo clippy -p worldwake-sim --tests -- -D warnings`

### Invariants

1. Traceability becomes more structured without changing authoritative simulation behavior.
2. Failed-start traces expose enough canonical data that same-tick contested-action debugging does not require brittle string matching or scheduler-only inspection.
3. Trace fields remain derived from canonical runtime data available at record time, not from later recomputation.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-sim/src/action_trace.rs` — add unit coverage for richer `StartFailed` data shape and summary rendering.
2. `crates/worldwake-sim/src/tick_step.rs` — add focused runtime coverage that a real `BestEffort` failed start records structured reason plus attempted targets.
3. `crates/worldwake-ai/tests/golden_production.rs` or `crates/worldwake-ai/tests/golden_care.rs` — optional cleanup to use the richer action-trace surface directly where it materially improves assertions.

### Commands

1. `cargo test -p worldwake-sim -- --list`
2. `cargo test -p worldwake-sim tick_step::tests::action_trace_assigns_explicit_order_across_started_failed_and_committed_events -- --exact`
3. `cargo test -p worldwake-sim tick_step::tests::best_effort_request_records_abort_requested_start_failure_without_failing_tick -- --exact`
4. `cargo test -p worldwake-ai golden_contested_harvest_start_failure_recovers_via_remote_fallback -- --exact`
5. `cargo clippy -p worldwake-sim --tests -- -D warnings`
