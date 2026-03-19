# S15STAFAIEME-009: Structured Start-Failure Traceability Surfaces

**Status**: REJECTED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: None — ticket reassessment only
**Deps**: `docs/FOUNDATIONS.md`, `docs/golden-e2e-testing.md`, `specs/S15-start-failure-emergence-golden-suites.md`, archived `S15STAFAIEME-001`

## Problem

This ticket proposed enriching `ActionTraceKind::StartFailed` with structured reason data and attempted targets so start-failure debugging would not require cross-referencing scheduler and decision-trace surfaces.

After reassessment, that is not the right architectural move for the current codebase. The authoritative start-failure record already lives in the scheduler, the AI reconciliation path already consumes that record directly, and the originally claimed S15 golden-coverage gap is no longer current.

## Assumption Reassessment (2026-03-19)

1. [action_trace.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/action_trace.rs) still stores `ActionTraceKind::StartFailed { reason: String }` only. That part of the ticket was factually correct.
2. [tick_step.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/tick_step.rs) already records the canonical structured failure into `scheduler::ActionStartFailure` before emitting the action-trace event. The action trace is therefore not the source of truth for start-failure semantics; the scheduler record is.
3. [decision_trace.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/decision_trace.rs) already exposes `planning.action_start_failures`, `planning.selection.selected_plan`, `planning.selection.selected_plan_source`, and `selected_plan.next_step`, which is the correct next-tick AI reconciliation surface.
4. The ticket's coverage assumptions are stale. The active golden suite already includes [golden_production.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_production.rs) tests `golden_contested_harvest_start_failure_recovers_via_remote_fallback` and `golden_contested_harvest_start_failure_recovers_via_remote_fallback_replays_deterministically`, so S15 Scenario 26 is no longer missing. [golden_care.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_care.rs) also already proves the care-domain `StartFailed` -> blocked-intent handoff.
5. Focused runtime coverage also already exists for the named lower-layer surfaces:
   - `action_trace::tests::summary_format_covers_all_variants`
   - `tick_step::tests::action_trace_assigns_explicit_order_across_started_failed_and_committed_events`
   - `tick_step::tests::best_effort_request_records_abort_requested_start_failure_without_failing_tick`
6. Mismatch + correction: the real remaining architectural question is not "should action traces duplicate scheduler failure data?" but "if we ever need richer canonical failed-start identity, which layer should own it?" The correct owner would be `scheduler::ActionStartFailure`, not `ActionTraceKind::StartFailed`.
7. Mismatch + correction: [specs/S15-start-failure-emergence-golden-suites.md](/home/joeloverbeck/projects/worldwake/specs/S15-start-failure-emergence-golden-suites.md) still describes Scenario 26 and related docs coverage as missing. That spec is stale relative to the shipped tests, but updating the spec is outside this ticket.

## Architecture Check

1. Rejecting the proposed `ActionTraceKind::StartFailed` expansion is cleaner than duplicating canonical start-failure data across scheduler records and action traces. Principle 25 applies here: the scheduler failure record is the authoritative structured surface, while the action trace is a lifecycle ledger optimized for execution visibility and ordering.
2. If future architecture needs structured attempted-target identity for failed starts, the robust design is to enrich `scheduler::ActionStartFailure` once and let decision traces and any derived debug surfaces read from that one record. Adding the same structured payload directly to `ActionTraceKind::StartFailed` first would create a second authority path and drift risk.
3. No backward-compatibility shims or aliases are warranted. The current architecture is already cleaner than the proposed duplication.

## Verification Layers

1. same-tick lifecycle fact that `StartFailed` occurred -> action trace via `tick_step` focused tests
2. canonical structured failure reason -> scheduler `ActionStartFailure` assertions in focused `tick_step` tests and current golden tests
3. next-tick AI reconciliation and stale-plan clearing -> decision trace assertions in `golden_care.rs` and `golden_production.rs`
4. cross-system downstream recovery after lawful start rejection -> authoritative world state and golden assertions in `golden_production.rs`

## What to Change

### 1. Correct the ticket scope

Record that the originally proposed trace-surface expansion is not the preferred architecture and that the claimed Scenario 26 gap has already been closed by current golden coverage.

### 2. Archive without engine changes

Do not modify `worldwake-sim`, `worldwake-ai`, or golden docs under this ticket. If later work still needs richer canonical failed-start identity, open a new ticket scoped to `scheduler::ActionStartFailure` rather than `ActionTraceKind::StartFailed`.

## Files to Touch

- `/home/joeloverbeck/projects/worldwake/tickets/S15STAFAIEME-009-structured-start-failure-traceability-surfaces.md` (modify, then archive)

## Out of Scope

- changing `ActionTraceKind::StartFailed`
- changing `tick_step` recording behavior
- changing scheduler, reservation, validation, or AI failure-handling semantics
- changing golden tests or docs under this ticket
- updating the stale S15 spec in `specs/`

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-sim action_trace::tests::summary_format_covers_all_variants -- --exact`
2. `cargo test -p worldwake-sim tick_step::tests::action_trace_assigns_explicit_order_across_started_failed_and_committed_events -- --exact`
3. `cargo test -p worldwake-sim tick_step::tests::best_effort_request_records_abort_requested_start_failure_without_failing_tick -- --exact`
4. `cargo test -p worldwake-ai golden_care_pre_start_wound_disappearance_records_blocker -- --exact`
5. `cargo test -p worldwake-ai golden_contested_harvest_start_failure_recovers_via_remote_fallback -- --exact`
6. `cargo clippy -p worldwake-sim --tests -- -D warnings`

### Invariants

1. Canonical structured start-failure data remains owned by the scheduler path, not duplicated into a second authoritative trace surface.
2. Action traces continue to serve lifecycle ordering and execution visibility, not become a second source of structured authority-state truth.

## Test Plan

### New/Modified Tests

1. None — this ticket is rejected after reassessment; verification relies on existing focused and golden coverage.

### Commands

1. `cargo test -p worldwake-sim action_trace::tests::summary_format_covers_all_variants -- --exact`
2. `cargo test -p worldwake-sim tick_step::tests::action_trace_assigns_explicit_order_across_started_failed_and_committed_events -- --exact`
3. `cargo test -p worldwake-sim tick_step::tests::best_effort_request_records_abort_requested_start_failure_without_failing_tick -- --exact`
4. `cargo test -p worldwake-ai golden_care_pre_start_wound_disappearance_records_blocker -- --exact`
5. `cargo test -p worldwake-ai golden_contested_harvest_start_failure_recovers_via_remote_fallback -- --exact`
6. `cargo clippy -p worldwake-sim --tests -- -D warnings`

## Outcome

- **Completion date**: 2026-03-19
- **What actually changed**: Reassessed the active ticket against current runtime code, focused tests, golden tests, and the S15 spec. Corrected the ticket to reflect that the proposed `ActionTraceKind::StartFailed` expansion is not the clean long-term architecture and that Scenario 26 golden coverage already exists in `golden_production.rs`.
- **Deviations from original plan**: No production, test, or docs changes were implemented. The ticket was archived as rejected because its proposal would duplicate canonical structured start-failure data into the wrong layer.
- **Verification results**: Focused `worldwake-sim` tests, the relevant `worldwake-ai` goldens, and `cargo clippy -p worldwake-sim --tests -- -D warnings` were run against the current codebase before archival.
