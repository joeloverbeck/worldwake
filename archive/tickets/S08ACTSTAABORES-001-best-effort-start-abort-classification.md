# S08ACTSTAABORES-001: Classify Start AbortRequested As Recoverable BestEffort Failure

**Status**: ✅ COMPLETED
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
4. Existing trace coverage in `tick_step::tests::action_trace_assigns_explicit_order_across_started_failed_and_committed_events` proves same-tick action tracing exists; this ticket needs to preserve that contract for the new recoverable branch rather than introducing a separate trace path.
5. `worldwake-ai` already consumes scheduler start failures in `crates/worldwake-ai/src/agent_tick.rs` by draining `scheduler.action_start_failures()` into `PlanningPipelineTrace.action_start_failures`, and `crates/worldwake-ai/src/failure_handling.rs` already derives blocker facts for `TargetLacksWounds` and `TargetHasNoWounds`. The missing piece is targeted runtime verification that this authoritative start-abort classification actually reaches that path.
6. Existing AI coverage is adjacent but not sufficient: `crates/worldwake-ai/src/failure_handling.rs::handle_plan_failure_drops_plan_records_blocker_and_marks_runtime_dirty` proves blocker handling in isolation, and `crates/worldwake-ai/tests/golden_emergent.rs` contains care goldens that explicitly avoid the abort race via `no_recovery_combat_profile()`. No current test pins the `AbortRequested` BestEffort start-failure handoff from `worldwake-sim` into `agent_tick`.
7. Mismatch found: the ticket's original scope said AI verification was deferred to `S08ACTSTAABORES-003`, but that conflicts with `AGENTS.md`'s Authoritative-To-AI Impact Rule and with Deliverable 5 in `specs/S08-action-start-abort-resilience.md`. Scope is corrected to include narrow downstream AI verification without expanding this ticket into the separate heal-resource-consumption defect.

## Architecture Check

1. Extending the existing recoverable-start classifier is cleaner than adding a special-case around individual action handlers because the drift semantics belong to the shared action framework, not to combat, trade, or office actions individually.
2. No backwards-compatibility aliasing or shim path is introduced; the existing BestEffort handling path remains the only recovery path.

## Verification Layers

1. BestEffort start abort does not fail the tick -> focused runtime test in `crates/worldwake-sim/src/tick_step.rs`
2. Recoverable start abort is recorded on the scheduler -> focused runtime test in `crates/worldwake-sim/src/tick_step.rs`
3. Recoverable start abort emits `ActionTraceKind::StartFailed` with same-tick ordering preserved -> focused runtime test in `crates/worldwake-sim/src/tick_step.rs`
4. Strict-mode start abort still propagates as an action error -> focused runtime test in `crates/worldwake-sim/src/tick_step.rs`
5. Scheduler-recorded start abort still reaches AI planning diagnostics and blocker handling inputs -> decision-trace-backed `agent_tick` runtime test in `crates/worldwake-ai/src/agent_tick.rs`
6. Later care goldens are not used as a proxy for this invariant because the contract under test is authoritative start-failure classification plus AI runtime handoff, not eventual healing outcome.

## What to Change

### 1. Extend recoverable start-failure classification

Update `is_best_effort_start_failure()` in `crates/worldwake-sim/src/tick_step.rs` so `ActionError::AbortRequested(_)` is treated as a recoverable authoritative start failure in `ActionRequestMode::BestEffort`.

### 2. Tighten focused regression coverage

Adjust or add `tick_step` tests so they explicitly prove:
- BestEffort start aborts are recorded and traced without failing the tick.
- Strict requests still return `TickStepError::Action(ActionError::AbortRequested(_))`.

### 3. Add narrow AI handoff coverage

Add or update a focused `agent_tick` runtime test so a scheduler-recorded heal-style start failure with `TargetHasNoWounds` or `TargetLacksWounds` appears in the planning trace and still feeds normal plan-failure handling expectations for replanning/blocker recording inputs.

## Files to Touch

- `crates/worldwake-sim/src/tick_step.rs` (modify)
- `crates/worldwake-ai/src/agent_tick.rs` (modify)

## Out of Scope

- `worldwake-systems` heal lifecycle or Medicine conservation changes from `specs/S08-action-start-abort-resilience.md`
- `worldwake-ai` blocker derivation or plan-failure logic changes
- Broadening recoverable classification to `InternalError` or other framework-corruption paths
- Any redesign of action start semantics beyond the recoverable/fatal classification boundary

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-sim tick_step::tests::best_effort_request_drops_recoverable_start_failure_without_failing_tick`
2. A focused `tick_step` test proving strict-mode `AbortRequested` still propagates under `ActionRequestMode::Strict`
3. A focused `agent_tick` trace test proving scheduler-recorded start failures are visible to the AI planning path for this abort reason family
4. Existing suites: `cargo test -p worldwake-sim` and `cargo test -p worldwake-ai`

### Invariants

1. BestEffort still swallows only lawful authoritative start drift, not internal framework corruption.
2. `ActionStartFailure` recording and `ActionTraceKind::StartFailed` remain same-tick authoritative runtime outputs for recoverable start failures.
3. Strict-mode action requests remain fail-fast on `AbortRequested`.
4. Downstream AI observability of scheduler start failures remains intact for care-style wound-invalidated aborts.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-sim/src/tick_step.rs` — pin `AbortRequested` as a recoverable BestEffort start failure and verify scheduler/trace recording.
2. `crates/worldwake-sim/src/tick_step.rs` — add a strict-mode regression proving the same start error still returns a hard failure.
3. `crates/worldwake-ai/src/agent_tick.rs` — prove the AI planning trace still sees scheduler-recorded start failures for wound-invalidated care aborts.

### Commands

1. `cargo test -p worldwake-sim tick_step::tests::best_effort_request_drops_recoverable_start_failure_without_failing_tick`
2. `cargo test -p worldwake-sim tick_step::tests::action_trace_assigns_explicit_order_across_started_failed_and_committed_events`
3. `cargo test -p worldwake-ai agent_tick::tests::planning_trace_includes_scheduler_start_failures_for_wound_abort_reasons`
4. `cargo test -p worldwake-sim`
5. `cargo test -p worldwake-ai`

## Outcome

- Completion date: 2026-03-19
- Actual changes:
  - `crates/worldwake-sim/src/tick_step.rs` now classifies `ActionError::AbortRequested(_)` as a recoverable BestEffort start failure.
  - Added focused `worldwake-sim` regressions for BestEffort and Strict handling of start-time `AbortRequested(TargetHasNoWounds)`.
  - Added focused `worldwake-ai` runtime coverage proving scheduler-recorded wound-abort start failures appear in planning traces and that missing in-flight actions reconcile without leaving the runtime stuck.
- Deviations from original plan:
  - The implementation stayed inside `worldwake-sim` for authoritative classification, but verification scope expanded into `worldwake-ai` to satisfy the repo's Authoritative-To-AI Impact Rule.
  - The downstream AI test verifies the current architecture as it exists today: structured start-failure details are surfaced in decision traces, while runtime recovery still happens through generic in-flight-step reconciliation rather than a dedicated start-failure-specific replan signal.
- Verification results:
  - `cargo test -p worldwake-sim tick_step::tests::best_effort_request_records_abort_requested_start_failure_without_failing_tick`
  - `cargo test -p worldwake-sim tick_step::tests::strict_request_propagates_abort_requested_start_failure`
  - `cargo test -p worldwake-ai agent_tick::tests::planning_trace_includes_scheduler_start_failures_for_wound_abort_reasons`
  - `cargo test -p worldwake-sim`
  - `cargo test -p worldwake-ai`
  - `cargo test --workspace`
  - `cargo clippy --workspace`
