# ACTEXETRA-003: Record trace events at lifecycle hook points in `tick_step.rs`

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — adds trace recording calls at 6 hook points
**Deps**: ACTEXETRA-001, ACTEXETRA-002

## Problem

The trace sink is threaded through `step_tick()` (ACTEXETRA-002) but nothing writes to it. This ticket adds `record_action_trace()` calls at the 6 lifecycle hook points identified in the spec: Started, StartFailed, Committed, Aborted (progress), Aborted (cancel), Aborted (dead actor).

## Assumption Reassessment (2026-03-17)

1. `apply_input()` handles `RequestAction` at line 202 — `start_affordance()` success at ~line 247, BestEffort failure at ~line 232-243. Confirmed.
2. `apply_input()` handles `CancelAction` at line 252 — abort at ~line 257-274. Confirmed.
3. `progress_active_actions()` at line 361 — `Committed` at line 401, `Aborted` at line 415. Confirmed.
4. `abort_actions_for_dead_actors()` at line 427 — iterates dead actors at line 446. Confirmed.
5. `services.action_defs.get(def_id)` returns `Option<&ActionDef>` where `ActionDef` has a `name: String` field. Must verify.
6. The `instance` variable is cloned before `tick_active_action()` at line 376-381, so it's available after the tick outcome.

## Architecture Check

1. All recording calls use the `runtime.record_action_trace()` helper which is a no-op when `None` — zero-cost.
2. Action name lookup uses `services.action_defs.get(def_id).map_or_else(|| "unknown".to_owned(), |d| d.name.clone())` — graceful fallback for unknown defs.
3. For `CancelAction`, instance info must be captured BEFORE the abort call removes it from the scheduler. The spec explicitly calls this out.
4. For `abort_actions_for_dead_actors`, instance must be looked up before each abort.

## What to Change

### 1. Record `Started` in `apply_input()` — after `start_affordance()` succeeds (~line 247)

After the affordance starts successfully, before returning `Ok(InputOutcome { actions_started: 1, ... })`, record a `Started` event with the actor, def_id, and targets from the input.

### 2. Record `StartFailed` in `apply_input()` — at BestEffort failure (~line 242)

After `record_action_start_failure()`, before `return Ok(InputOutcome::default())`, record a `StartFailed` event with the error formatted as the reason.

### 3. Record `Committed` in `progress_active_actions()` — at `TickOutcome::Committed` (~line 401)

After `retain_committed_action()`, record a `Committed` event using the cloned `instance` and the `outcome`.

### 4. Record `Aborted` in `progress_active_actions()` — at `TickOutcome::Aborted` (~line 415)

After `retain_replan()`, record an `Aborted` event using the cloned `instance` and the abort reason.

### 5. Record `Aborted` in `apply_input()` — at `CancelAction` (~line 252)

Before calling `abort_active_action()`, look up the instance's `def_id` from `runtime.scheduler.active_actions().get(&action_instance_id)`. After the abort, record an `Aborted` event with reason including the sequence_no.

### 6. Record `Aborted` in `abort_actions_for_dead_actors()` (~line 446)

Before each `abort_active_action()` call, clone the instance. After the abort, record an `Aborted` event with reason `"ActorMarkedDead"`.

## Files to Touch

- `crates/worldwake-sim/src/tick_step.rs` (modify — 6 recording sites in 3 functions)

## Out of Scope

- The `action_trace.rs` module types (ACTEXETRA-001)
- The `TickStepServices`/`TickStepRuntime` threading (ACTEXETRA-002)
- `GoldenHarness` integration and golden tests (ACTEXETRA-004)
- Documentation (ACTEXETRA-005)
- Any changes to action handlers, scheduler, or other modules
- Adding trace recording in any file other than `tick_step.rs`

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test --workspace` — all existing tests still pass (trace is `None` in all existing callers, so recording calls are no-ops)
2. `cargo clippy --workspace` — no warnings
3. `cargo clippy -p worldwake-sim` — no unused variable warnings from trace recording code

### Invariants

1. All 6 hook points from the spec table are instrumented — no lifecycle event goes unrecorded
2. Zero behavioral change when `action_trace` is `None` — recording is gated by `if let Some(sink)`
3. No `ActionTraceEvent` objects are constructed when tracing is disabled — the `record_action_trace()` helper short-circuits on `None`
4. Instance data for cancel/dead-actor aborts is captured BEFORE the abort removes it from the scheduler

## Test Plan

### New/Modified Tests

1. No new tests in this ticket — behavioral verification requires the `GoldenHarness` integration in ACTEXETRA-004. This ticket only adds no-op recording calls (all existing callers pass `action_trace: None`).

### Commands

1. `cargo build -p worldwake-sim`
2. `cargo test --workspace`
3. `cargo clippy --workspace`
