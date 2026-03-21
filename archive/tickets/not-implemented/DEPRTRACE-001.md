# DEPRTRACE-001: Reassess Proposed First-Class Authoritative Deprivation Traceability

**Status**: NOT IMPLEMENTED
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: None
**Deps**: `docs/FOUNDATIONS.md`, `crates/worldwake-systems/src/needs.rs`, `crates/worldwake-systems/tests/e09_needs_integration.rs`, `crates/worldwake-ai/tests/golden_emergent.rs`, `docs/golden-e2e-testing.md`, `archive/tickets/completed/S17WOULIFGOLSUI-001.md`, `archive/tickets/completed/GOLAUTHVAL-001.md`

## Problem

This ticket originally proposed adding a dedicated authoritative deprivation/needs trace sink in `worldwake-sim` so physiology failures could be inspected without source-level debugging.

That proposal must be reassessed against the current code and tests before any implementation work, because the repo has changed materially since the ticket was drafted.

## Assumption Reassessment (2026-03-21)

1. The ticket's motivating golden gap is no longer real. `crates/worldwake-ai/tests/golden_emergent.rs` already contains `golden_deprivation_wound_worsening_consolidates_not_duplicates` and `golden_deprivation_wound_worsening_consolidates_not_duplicates_replays_deterministically`, and `docs/generated/golden-e2e-inventory.md` lists both. The archived outcome in [S17WOULIFGOLSUI-001.md](/home/joeloverbeck/projects/worldwake/archive/tickets/completed/S17WOULIFGOLSUI-001.md) records that delivery explicitly.
2. The authoritative deprivation behavior is already covered at multiple layers with exact current tests:
   - focused/unit in `crates/worldwake-systems/src/needs.rs`: `needs_system_adds_starvation_wound_and_resets_hunger_exposure`, `needs_system_requires_another_full_tolerance_period_before_second_wound`, `needs_system_second_starvation_threshold_worsens_existing_wound`, `needs_system_resets_deprivation_exposure_when_pressure_drops_below_critical`, and the helper-level `worsen_*` tests
   - runtime/integration in `crates/worldwake-systems/tests/e09_needs_integration.rs`: `scheduler_applies_starvation_and_dehydration_consequences_after_tolerance_windows`
   - golden/E2E in `crates/worldwake-ai/tests/golden_emergent.rs`: `golden_deprivation_wound_worsening_consolidates_not_duplicates`
3. The ticket's claim that deprivation debugging "relies on source inspection and indirect state assertions" is now overstated. The current architecture already exposes the earliest authoritative mutation boundary through focused needs-system tests and the durable outcome boundary through integration/golden coverage. What is still absent is a dedicated per-tick derived trace sink, not causal observability as such.
4. The proposed implementation would widen `worldwake-sim::SystemExecutionContext` and `TickStepServices` with a needs-specific optional sink purely for one system. That is a real architectural cost across the shared runtime path. Unlike request resolution, action lifecycle, or politics succession, deprivation firing currently has no cross-cutting multi-system ordering contract that requires a new first-class runtime trace surface.
5. `docs/golden-e2e-testing.md` already directs golden work to prefer the strongest semantic boundary. For this behavior, authoritative world state is the primary contract, and the repo already has the exact S17 golden proving it. Adding another trace layer would duplicate proof surfaces rather than closing a missing one.
6. The ticket's `Architecture Check` cites Principles 24, 25, and 27, but the current `docs/FOUNDATIONS.md` in this repository does not use that numbering. Any architectural argument here must be grounded in the actual current foundations document, especially concrete state, persistent identity, explicit transfer, locality, duration/cost, and revisable intent.
7. No AI reasoning gap, stale-request boundary, or same-tick action-ordering issue is involved. The proposed sink would instrument authoritative need progression only; it would not fix a planner blind spot or a shared runtime failure mode.
8. Existing named trace surfaces are real and current: `DecisionTraceSink` in `crates/worldwake-ai/src/decision_trace.rs`, `ActionTraceSink` in `crates/worldwake-sim/src/action_trace.rs`, `RequestResolutionTraceSink` in `crates/worldwake-sim/src/request_resolution_trace.rs`, and `PoliticalTraceSink` in `crates/worldwake-sim/src/politics_trace.rs`. The golden harness exposes opt-in helpers for action/request/politics tracing in `crates/worldwake-ai/tests/golden_harness/mod.rs`. There is no existing needs trace sink.
9. The follow-up process gap that the original S17 mismatch exposed was already handled by [GOLAUTHVAL-001.md](/home/joeloverbeck/projects/worldwake/archive/tickets/completed/GOLAUTHVAL-001.md), which strengthened ticket/spec authoring rules around concrete numeric validation and survivability. That reduced the strongest remaining justification for adding a new debug substrate here.
10. Corrected scope: this is not a production implementation ticket anymore. The right action is to retire the proposed needs trace sink unless a future bug demonstrates a concrete causal boundary that existing state, event, focused-runtime, action-trace, request-trace, or golden surfaces cannot express.

## Architecture Check

1. Not adding a new needs-specific trace sink is cleaner than threading one more optional debug collector through the shared sim runtime for a behavior already covered by focused, integration, and golden tests. It keeps `SystemExecutionContext` from accreting one-off subsystem instrumentation fields.
2. The current architecture is stronger when each first-class trace surface corresponds to a genuinely distinct shared boundary:
   - decision trace -> AI reasoning and selection
   - request-resolution trace -> pre-start authority binding/rejection
   - action trace -> authoritative lifecycle ordering
   - politics trace -> office-succession evaluation inside a system with branch-rich internal phases
   Deprivation progression does not currently justify that same boundary split.
3. If a future deprivation debugging problem appears, the ideal architecture is still not "add a sink because one system lacks a sink." The right threshold would be a demonstrated repeated need for structured per-tick internal phase inspection that cannot be expressed cleanly through authoritative state or existing runtime surfaces. That case has not been shown here.
4. No backwards-compatibility aliasing, shim, or mixed-purpose "misc trace" surface should be introduced. The cleanest long-term outcome for this ticket is non-implementation.

## Verification Layers

1. starvation/dehydration threshold firing and exposure reset semantics -> focused authoritative runtime tests in `crates/worldwake-systems/src/needs.rs`
2. scheduler-driven deprivation consequences through the live system path -> `crates/worldwake-systems/tests/e09_needs_integration.rs::scheduler_applies_starvation_and_dehydration_consequences_after_tolerance_windows`
3. deprivation wound consolidation through the full golden harness -> `crates/worldwake-ai/tests/golden_emergent.rs::golden_deprivation_wound_worsening_consolidates_not_duplicates`
4. replay stability for the deprivation golden -> `crates/worldwake-ai/tests/golden_emergent.rs::golden_deprivation_wound_worsening_consolidates_not_duplicates_replays_deterministically`
5. no new runtime trace boundary is required -> ticket reassessment against current code/tests plus full workspace verification

## What To Change

### 1. Do not implement a new needs trace sink

Reject the proposed `NeedsTraceSink` / deprivation trace surface for now.

### 2. Archive this ticket as not implemented

Record that the original assumptions were stale and that the proposed architecture is not currently justified by the live codebase or coverage gaps.

## Files To Touch

- `tickets/DEPRTRACE-001.md` (modify)
- `archive/tickets/not-implemented/DEPRTRACE-001.md` (move/archive destination)

## Out Of Scope

- adding any production code under `crates/worldwake-sim/` or `crates/worldwake-systems/`
- widening `SystemExecutionContext`, `TickStepServices`, or golden harness tracing helpers
- adding a generic "trace everything" substrate
- weakening existing authoritative state assertions in favor of a new trace layer

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-systems needs::tests::needs_system_second_starvation_threshold_worsens_existing_wound -- --exact`
2. `cargo test -p worldwake-systems --test e09_needs_integration scheduler_applies_starvation_and_dehydration_consequences_after_tolerance_windows -- --exact`
3. `cargo test -p worldwake-ai --test golden_emergent golden_deprivation_wound_worsening_consolidates_not_duplicates -- --exact`
4. `cargo test --workspace`
5. `cargo clippy --workspace --all-targets -- -D warnings`

### Invariants

1. The ticket must accurately reflect the current coverage and architecture.
2. No new needs-specific trace architecture is introduced without a demonstrated missing causal boundary.
3. The repo continues to rely on the strongest existing semantic proof surface for deprivation behavior: authoritative state plus focused/integration/golden tests.

## Test Plan

### New/Modified Tests

1. None. This is a ticket reassessment and archival decision; no runtime or test code changes are warranted.

### Commands

1. `cargo test -p worldwake-systems needs::tests::needs_system_second_starvation_threshold_worsens_existing_wound -- --exact`
2. `cargo test -p worldwake-systems --test e09_needs_integration scheduler_applies_starvation_and_dehydration_consequences_after_tolerance_windows -- --exact`
3. `cargo test -p worldwake-ai --test golden_emergent golden_deprivation_wound_worsening_consolidates_not_duplicates -- --exact`
4. `cargo test --workspace`
5. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

- Completion date: 2026-03-21
- What changed:
  - corrected the ticket assumptions to match the current code, current tests, and archived S17/GOLAUTHVAL follow-up work
  - removed the proposed implementation scope for a new needs/deprivation trace sink
  - archived the ticket as not implemented because the additional trace architecture is not currently justified
- Deviations from original plan:
  - the original plan proposed new runtime plumbing, harness helpers, tests, and trace guidance
  - after reassessment, the motivating coverage gap had already been closed elsewhere, so the correct result was to retire the implementation instead of adding redundant architecture
- Verification results:
  - `cargo test -p worldwake-systems needs::tests::needs_system_second_starvation_threshold_worsens_existing_wound -- --exact` passed
  - `cargo test -p worldwake-systems --test e09_needs_integration scheduler_applies_starvation_and_dehydration_consequences_after_tolerance_windows -- --exact` passed
  - `cargo test -p worldwake-ai --test golden_emergent golden_deprivation_wound_worsening_consolidates_not_duplicates -- --exact` passed
  - `cargo test --workspace` passed
  - `cargo clippy --workspace --all-targets -- -D warnings` passed
