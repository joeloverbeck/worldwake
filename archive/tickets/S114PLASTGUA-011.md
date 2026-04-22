# S114PLASTGUA-011: Guard-breach `ExpectationMismatch` emission on AI start failure

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — AI execution/start-failure path emits populated `DecisionEventPayload::ExpectationMismatch` on guard breach and preserves `GuardInvalidator` detail.
**Deps**: `archive/tickets/S114PLASTGUA-005.md`, `archive/tickets/S114PLASTGUA-007.md`

## Problem

`S114PLASTGUA-007` landed the guard-check revalidation pass and preserved `PlanInvalidationReason::ExpectationMismatch { step_index }` through the AI replan path, but it did not emit a corresponding `EventTag::ExpectationMismatch` event or carry the guard invalidator into the widened `ExpectationMismatchPayload`. That leaves S114 spec test 12 and ticket 010 without the event-log proof surface they explicitly require, and it weakens FND-17's surprise signal on the guard-breach start-failure path.

## Assumption Reassessment (2026-04-22)

1. `classify_revalidation` in [plan_revalidation.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/plan_revalidation.rs:36) now returns `RevalidationOutcome::Invalidated { reason: PlanInvalidationReason::ExpectationMismatch { step_index } }` on guard breach, but it discards the concrete `Invalidator` returned by `check_guard`. No public result surface currently preserves `InvalidatorTag`.
2. The live start-failure caller is [agent_tick/execution.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/agent_tick/execution.rs:22). On failed revalidation, it classifies the step, routes through `handle_current_step_failure`, and emits `EventTag::ReplanTriggered`; it does not emit `EventTag::ExpectationMismatch` on this branch.
3. The only current AI-side helper that emits `DecisionEventPayload::ExpectationMismatch` is `emit_expectation_mismatch` in [agent_tick/observation.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/agent_tick/observation.rs:96). That helper currently hardcodes `expectation_kind: None` and `mismatch_detail: None`, and its existing call site covers post-commit materialization-binding mismatch, not pre-enqueue guard breach.
4. Shared boundary under audit: the AI execution/start-failure seam from `plan_revalidation.rs` through `agent_tick/execution.rs`, plus the shared event-payload contract on `ExpectationMismatchPayload` in `worldwake-core`. This is an AI-local producer-path fix; no sim-side system change is involved.
5. `specs/S114-plan-step-guards.md:319-332` says the richer invalidator detail is carried in the widened `ExpectationMismatchPayload` when the event is emitted downstream. `tickets/S114PLASTGUA-010.md` already depends on that contract and expects `mismatch_detail: Some(GuardInvalidator(InvalidatorTag::TargetMoved))` in the golden event-log assertion.
6. Live authored `trade` steps already carry a guard via `guard_template` from ticket 006, but `trade_actions.rs` still leaves `expectation_template: vec![]`. If the guard-breach event needs a non-`None` `expectation_kind`, implementation must derive that kind truthfully from the guard/revalidation contract rather than pretending an authored expectation already exists on the live `trade` step.
7. Intended layer is runtime `agent_tick`, not golden-only. A local needs-only harness is insufficient because the truthful proof must traverse the same `BestEffort` action-start/revalidation path that ticket 007 wired; use full action registries in focused runtime coverage.
8. Ordering matters at three distinct layers: decision classification (`classify_revalidation`), pre-enqueue request rejection in `agent_tick`, and event-log emission. The event should be proven on the same failure tick as the guard breach, not via a later overdue or post-commit surrogate.
9. This is a stale-request/start-failure ticket. The first live failure boundary is AI-side pre-enqueue step validation in `enqueue_valid_step_or_handle_failure`, not `worldwake-sim/src/tick_step.rs`. Exact shared symbols checked during reassessment: `classify_revalidation`, `handle_current_step_failure`, `emit_decision_event`, and `ExpectationMismatchPayload`.
10. Mismatch + correction: sibling ticket `S114PLASTGUA-009` previously claimed ticket 007 owned the `GuardInvalidator` mismatch-detail case. That claim is false on the landed branch; this ticket now owns the remaining guard-breach event-emission path.

## Architecture Check

1. The clean fix is to preserve guard-breach detail at the revalidation/execution seam once and emit the payload from the real start-failure branch, rather than re-running guard evaluation ad hoc later or inventing a synthetic overdue path just to get an event. That keeps the event tied to the actual local surprise boundary.
2. No backwards-compatibility aliasing or shim path is needed. Extend the live AI revalidation/emission contract in place and keep `ExpectationMismatchPayload` as the single authoritative event payload.

## Verification Layers

1. Guard-breach diagnostic preservation (`TargetMoved`, `CommodityDepleted`, etc.) -> focused unit test at the `classify_revalidation` / execution handoff seam.
2. Pre-enqueue rejection still occurs on guard breach -> focused runtime `agent_tick` test.
3. `DecisionEventPayload::ExpectationMismatch` emits on the guard-breach tick with populated `expectation_kind` + `mismatch_detail` -> event-log delta assertion in focused runtime coverage.
4. `ReplanReason::PlanInvalidated { reason: ExpectationMismatch { step_index } }` remains preserved after adding event emission -> focused helper/runtime assertion, reusing or extending the existing 007 proof.
5. Same-tick proof surfaces stay distinct: request rejection / runtime state transition -> runtime `agent_tick`; event payload contents -> event log; classification detail preservation -> focused unit/helper test.
6. No delayed authoritative effect is part of this contract; using overdue expectations or post-commit mismatch as a proxy would be dishonest because the owned boundary is the immediate AI start-failure path.

## What to Change

### 1. Preserve guard-breach detail through revalidation

Extend the AI-side revalidation result or a nearby helper so a guard breach preserves enough structured data for downstream emission:

- `PlanInvalidationReason::ExpectationMismatch { step_index }`
- the concrete `InvalidatorTag`
- the truthful `expectation_kind` for this breach path

The implementation should not recompute guard breach detail later from scratch if the classifier can preserve it once.

### 2. Emit `ExpectationMismatch` on the start-failure branch

In the guard-breach branch of `enqueue_valid_step_or_handle_failure` (or the narrowest shared helper it delegates to), emit `EventTag::ExpectationMismatch` on the same tick as the failed revalidation before the replan-triggered event is recorded.

Populate:

- `step_index`
- `expected_materializations`
- `expectation_kind: Some(...)`
- `mismatch_detail: Some(MismatchDetail::GuardInvalidator(...))`

### 3. Reuse or refine the shared event helper

If `emit_expectation_mismatch` in `agent_tick/observation.rs` remains the shared helper, widen it so callers can pass populated `expectation_kind` and `mismatch_detail` instead of hardcoded `None` values. Keep the existing post-commit materialization-mismatch path truthful if it still lacks richer detail.

## Files to Touch

- `crates/worldwake-ai/src/plan_revalidation.rs` (modify — preserve guard-breach detail at the classification seam)
- `crates/worldwake-ai/src/agent_tick/execution.rs` (modify — emit `ExpectationMismatch` on the guard-breach start-failure branch)
- `crates/worldwake-ai/src/agent_tick/observation.rs` (modify — widen or reuse the shared emission helper)
- `crates/worldwake-ai/src/agent_tick/tests.rs` (modify — runtime proof of same-tick emission on guard breach)

## Out of Scope

- The overdue-record AI tick step for `PlanStepCompletion` records — ticket 009 owns that consumer path.
- New save-format or core-payload widening work — ticket 005 already landed the payload surface.
- Golden-scenario authoring — ticket 010 owns the end-to-end scenario once this producer path exists.

## Acceptance Criteria

### Tests That Must Pass

1. `classify_revalidation_fires_target_moved_on_believed_location_divergence` — focused unit/helper test proves the revalidation result preserves `ExpectationKindTag::State` plus `MismatchDetail::GuardInvalidator(InvalidatorTag::TargetMoved)` for the execution handoff.
2. `revalidation_guard_breach_emits_expectation_mismatch_before_enqueue` — focused execution-seam test proves a breached guard rejects the step before enqueue and records exactly one `DecisionEventPayload::ExpectationMismatch` with populated `expectation_kind` and `mismatch_detail: Some(GuardInvalidator(...))`.
3. Existing 007 regression coverage stays green: the guard-breach branch still clears the plan/goal and preserves `ReplanReason::PlanInvalidated { reason: ExpectationMismatch { step_index } }`.
4. Existing suite: `cargo test -p worldwake-ai agent_tick`

### Invariants

1. Guard-breach `ExpectationMismatch` emission happens on the AI start-failure path, not by fabricating an overdue expectation or waiting for a later commit-time mismatch.
2. The emitted `mismatch_detail` is derived from the actual guard invalidator that fired; it is never guessed from a later discrepancy classification or hardcoded by action name.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/agent_tick/tests.rs` — prove pre-enqueue rejection plus exact `ExpectationMismatch` payload at the execution helper seam.
2. `crates/worldwake-ai/src/plan_revalidation.rs` — prove guard-breach detail is preserved through the revalidation handoff.

### Commands

1. `cargo test -p worldwake-ai --lib plan_revalidation::tests::classify_revalidation_fires_target_moved_on_believed_location_divergence -- --exact`
2. `cargo test -p worldwake-ai --lib agent_tick::tests::revalidation_guard_breach_emits_expectation_mismatch_before_enqueue -- --exact`
3. `cargo test -p worldwake-ai`
4. `./scripts/verify.sh`

## Outcome

Completed on 2026-04-22.

- Preserved guard-breach mismatch context directly on `RevalidationOutcome`, carrying `ExpectationKindTag` plus `MismatchDetail::GuardInvalidator(...)` out of `classify_revalidation`.
- Reused the existing `emit_expectation_mismatch` helper by widening it with an `ExpectationMismatchContext`, then emitted `EventTag::ExpectationMismatch` from the real AI start-failure branch in `enqueue_valid_step_or_handle_failure` before `ReplanTriggered`.
- Added focused coverage for both the classification seam and the execution seam, and updated the route-known guard expectation to reflect the now-populated state-kind payload.

## Deviations

- The focused runtime proof landed at the direct execution helper seam (`enqueue_valid_step_or_handle_failure`) instead of a full `step_once` harness tick. That is the narrowest honest boundary that proves the owned pre-enqueue emission contract without over-claiming later active-goal persistence cleanup.

## Verification Result

- Passed `cargo test -p worldwake-ai --lib plan_revalidation::tests::classify_revalidation_fires_target_moved_on_believed_location_divergence -- --exact`
- Passed `cargo test -p worldwake-ai --lib agent_tick::tests::revalidation_guard_breach_emits_expectation_mismatch_before_enqueue -- --exact`
- Passed `cargo test -p worldwake-ai`
- Passed `./scripts/verify.sh`
