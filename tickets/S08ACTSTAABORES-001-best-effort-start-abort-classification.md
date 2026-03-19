# S08ACTSTAABORES-001: Classify Start AbortRequested As Recoverable BestEffort Failure

**Status**: PENDING
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — `worldwake-sim` tick-step start-failure classification and trace coverage
**Deps**: `specs/S08-action-start-abort-resilience.md`

## Problem

`step_tick()` currently treats `ActionError::AbortRequested(_)` from `start_affordance()` as fatal even in `ActionRequestMode::BestEffort`. That breaks the intended "lawful world drift causes replanning, not simulation failure" contract and prevents downstream consumers from seeing a normal `ActionStartFailure`.

## Assumption Reassessment (2026-03-19)

1. `crates/worldwake-sim/src/tick_step.rs` still classifies recoverable BestEffort start failures via `is_best_effort_start_failure()`, and that helper currently matches only `ReservationUnavailable`, `PreconditionFailed`, and `InvalidTarget`. `AbortRequested` is still excluded.
2. The BestEffort fallback path in `process_input()` already records `scheduler::ActionStartFailure` and `ActionTraceKind::StartFailed` once a start error is classified as recoverable, so the missing behavior is classification, not downstream plumbing.
3. Existing focused coverage already exercises the recoverable path in `tick_step::tests::best_effort_request_drops_recoverable_start_failure_without_failing_tick`, but that test does not currently pin `ActionError::AbortRequested(_)` specifically.
4. Existing trace coverage in `tick_step::tests::action_trace_assigns_explicit_order_across_started_failed_and_committed_events` proves same-tick action tracing exists; this ticket only needs to preserve that contract for the new recoverable branch.
5. This ticket is authoritative runtime behavior, not AI logic. Additional AI pipeline checks are intentionally deferred to `S08ACTSTAABORES-003`.
6. `specs/S08-action-start-abort-resilience.md` and `specs/IMPLEMENTATION-ORDER.md` both still describe S08 as a current Phase 3 bug fix in `worldwake-sim` with no unmet phase dependencies.
7. No mismatch found between the spec and the current `tick_step.rs` implementation.

## Architecture Check

1. Extending the existing recoverable-start classifier is cleaner than adding a special-case around individual action handlers because the drift semantics belong to the shared action framework, not to combat, trade, or office actions individually.
2. No backwards-compatibility aliasing or shim path is introduced; the existing BestEffort handling path remains the only recovery path.

## Verification Layers

1. BestEffort start abort does not fail the tick -> focused runtime test in `crates/worldwake-sim/src/tick_step.rs`
2. Recoverable start abort is recorded on the scheduler -> focused runtime test in `crates/worldwake-sim/src/tick_step.rs`
3. Recoverable start abort emits `ActionTraceKind::StartFailed` with same-tick ordering preserved -> focused runtime test in `crates/worldwake-sim/src/tick_step.rs`
4. Strict-mode start abort still propagates as an action error -> focused runtime test in `crates/worldwake-sim/src/tick_step.rs`

## What to Change

### 1. Extend recoverable start-failure classification

Update `is_best_effort_start_failure()` in `crates/worldwake-sim/src/tick_step.rs` so `ActionError::AbortRequested(_)` is treated as a recoverable authoritative start failure in `ActionRequestMode::BestEffort`.

### 2. Tighten focused regression coverage

Adjust or add `tick_step` tests so they explicitly prove:
- BestEffort start aborts are recorded and traced without failing the tick.
- Strict requests still return `TickStepError::Action(ActionError::AbortRequested(_))`.

## Files to Touch

- `crates/worldwake-sim/src/tick_step.rs` (modify)

## Out of Scope

- `worldwake-systems` heal lifecycle or Medicine conservation changes
- `worldwake-ai` blocker derivation or plan-failure logic changes
- Broadening recoverable classification to `InternalError` or other framework-corruption paths
- Any redesign of action start semantics beyond the recoverable/fatal classification boundary

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-sim tick_step::tests::best_effort_request_drops_recoverable_start_failure_without_failing_tick`
2. A focused `tick_step` test proving strict-mode `AbortRequested` still propagates under `ActionRequestMode::Strict`
3. Existing suite: `cargo test -p worldwake-sim -- --list`

### Invariants

1. BestEffort still swallows only lawful authoritative start drift, not internal framework corruption.
2. `ActionStartFailure` recording and `ActionTraceKind::StartFailed` remain same-tick authoritative runtime outputs for recoverable start failures.
3. Strict-mode action requests remain fail-fast on `AbortRequested`.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-sim/src/tick_step.rs` — pin `AbortRequested` as a recoverable BestEffort start failure and verify scheduler/trace recording.
2. `crates/worldwake-sim/src/tick_step.rs` — add a strict-mode regression proving the same start error still returns a hard failure.

### Commands

1. `cargo test -p worldwake-sim tick_step::tests::best_effort_request_drops_recoverable_start_failure_without_failing_tick`
2. `cargo test -p worldwake-sim tick_step::tests::action_trace_assigns_explicit_order_across_started_failed_and_committed_events`
3. `cargo test -p worldwake-sim`
