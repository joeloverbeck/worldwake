# S15STAFAIEME-008: Add Start-Failure Provenance Across Request Resolution, Action Trace, And Decision Trace

**Status**: ✅ COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — `worldwake-sim` request/action start-failure trace surfaces, `worldwake-ai` decision-trace summaries
**Deps**: S15STAFAIEME-003

## Problem

The current trace surfaces make recoverable start failures observable, but they do not carry enough shared provenance to explain the whole causal chain without manual cross-inference.

In particular, a `StartFailed` at authoritative action start can currently be seen in:

- request-resolution trace as a `Bound { ... start_attempted: true }` request
- action trace as `ActionTraceKind::StartFailed`
- AI decision trace as `planning.action_start_failures`

But the start-failure summaries do not preserve the request provenance/binding path that led into the failed start, and they do not give a direct cross-sink handle for "this failure came from a stale externally injected request" versus "this came from the AI's retained current plan." That gap makes mixed-layer debugging and golden authoring harder than it should be.

## Assumption Reassessment (2026-03-20)

1. The request-resolution layer already records provenance and binding in [request_resolution_trace.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/request_resolution_trace.rs): `RequestResolutionTraceEvent` stores `provenance`, and `RequestResolutionOutcome::Bound` stores `RequestBindingKind` plus `start_attempted`.
2. The authoritative start-failure layer currently drops that provenance. [scheduler.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/scheduler.rs) stores `ActionStartFailure { tick, actor, def_id, reason }` only, and [action_trace.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/action_trace.rs) stores `ActionTraceKind::StartFailed { reason }` without request provenance or binding details.
3. The AI layer inherits the same loss of context. [decision_trace.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/decision_trace.rs) stores `ActionStartFailureSummary { tick, def_id, reason }` only, so `planning.action_start_failures` cannot directly tell whether the failure came from an `AiPlan` or `External` request path.
4. Existing focused coverage already proves the request-resolution and start-failure boundaries separately: `tick_step::tests::strict_request_records_resolution_rejection_without_start_attempt`, `tick_step::tests::best_effort_stale_request_records_start_failure_when_affordance_no_longer_matches`, and `tick_step::tests::reproduced_request_records_request_resolution_trace_before_start` in [tick_step.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/tick_step.rs). I confirmed the exact sim test names with `cargo test -p worldwake-sim --lib -- --list`.
5. Existing focused AI/runtime coverage already proves next-tick consumption of structured start failures in [agent_tick.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/agent_tick.rs), but that summary currently includes only `tick`, `def_id`, and `reason`.
6. Existing golden coverage already proves the AI recovery side of the chain in care, production, trade, and politics, including `golden_contested_harvest_start_failure_recovers_via_remote_fallback` in [golden_production.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_production.rs), `golden_local_trade_start_failure_recovers_via_production_fallback` in [golden_trade.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_trade.rs), and `golden_remote_office_claim_start_failure_loses_gracefully` in [golden_emergent.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_emergent.rs). I confirmed the office golden test names with `cargo test -p worldwake-ai --test golden_emergent -- --list`. The gap is not missing recovery behavior; it is missing traceability continuity across layers.
7. The first failure boundary for the target scenarios remains authoritative start, not request-resolution rejection. This ticket must preserve that distinction rather than flattening all stale-request failures into one generic path.
8. This should be implemented as shared runtime trace enrichment, not as political-specific or golden-specific metadata. The gap exists equally for care, production, trade, and politics.
9. Scope correction: do not solve this by adding ad hoc assertions to one golden. The architecture needs a durable shared provenance model linking request resolution, authoritative `StartFailed`, and decision-trace blocker consumption.
10. Scope correction: do not add new start-failure goldens for domains already covered. Reuse and strengthen the existing focused and golden coverage.

## Architecture Check

1. A shared request-attempt context carried by all start-failure sinks is cleaner than teaching each golden to correlate three sinks by hand using tick-local timing and actor/action guesses.
2. The stable correlation key should come from the real runtime request path, not a golden-only synthetic identifier. The current input `sequence_no` already exists at the boundary that binds the request and initiates authoritative start, so extending the trace model from that substrate is cleaner than inventing a second ID space.
3. No backwards-compatibility aliasing or duplicate trace pathways should be introduced. Extend the existing request-resolution/action-trace/decision-trace surfaces directly.

## Verification Layers

1. Request provenance and binding survive the request-resolution boundary -> focused runtime test in [tick_step.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/tick_step.rs).
2. The same provenance/binding is visible on authoritative `StartFailed` records -> focused runtime/action-trace coverage in [tick_step.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/tick_step.rs).
3. The AI decision trace receives the same enriched start-failure summary on the next tick -> focused AI runtime test in [agent_tick.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/agent_tick.rs).
4. An existing golden mixed-layer scenario can assert the full provenance chain without inference -> targeted assertion update in an already active S15 golden such as [golden_emergent.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_emergent.rs), [golden_production.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_production.rs), or [golden_trade.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_trade.rs).
5. Later authoritative office installation or other downstream world state is not a proxy for this ticket's contract. The contract is provenance continuity across request resolution, authoritative start failure, action trace, and AI recovery surfaces.

## What to Change

### 1. Extend shared start-failure records with request-attempt context

Add shared request-attempt data to the start-failure path so the same failed start can be inspected consistently across:

- request-resolution trace
- scheduler `ActionStartFailure`
- action trace `StartFailed`
- AI `ActionStartFailureSummary`

At minimum, preserve:

- input `sequence_no`
- `RequestProvenance`
- request binding path (`ReproducedAffordance` vs `BestEffortFallback`)
- enough stable identity to correlate the failed start across the sinks without relying on incidental timing alone

### 2. Add focused cross-layer tests

Strengthen focused runtime and AI tests so they prove the provenance survives the full chain from `RequestAction` resolution into `StartFailed` and then into next-tick AI reconciliation.

### 3. Strengthen one existing golden assertion on the shared provenance path

Update one existing S15/S08-style golden to assert the new shared provenance fields in a real mixed-layer scenario, so the contract stays usable at the E2E level without adding duplicate scenario coverage.

## Files to Touch

- `crates/worldwake-sim/src/request_resolution_trace.rs` (modify)
- `crates/worldwake-sim/src/action_trace.rs` (modify)
- `crates/worldwake-sim/src/scheduler.rs` (modify)
- `crates/worldwake-sim/src/tick_step.rs` (modify)
- `crates/worldwake-ai/src/decision_trace.rs` (modify)
- `crates/worldwake-ai/src/agent_tick.rs` (modify)
- `crates/worldwake-ai/tests/golden_emergent.rs` (modify) or another existing golden that already proves start-failure reconciliation

## Out of Scope

- changing start-failure behavior itself
- changing office claim rules or support-law succession semantics
- adding domain-specific provenance only for political actions
- introducing a separate compatibility trace sink for the same data

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-sim tick_step::tests::strict_request_records_resolution_rejection_without_start_attempt -- --exact`
2. `cargo test -p worldwake-sim tick_step::tests::best_effort_stale_request_records_start_failure_when_affordance_no_longer_matches -- --exact`
3. `cargo test -p worldwake-ai --test golden_emergent golden_remote_office_claim_start_failure_loses_gracefully -- --exact`
4. `cargo test -p worldwake-ai --lib agent_tick::tests::planning_trace_includes_scheduler_start_failures_for_wound_abort_reasons -- --exact`
5. Existing suites: `cargo test -p worldwake-sim` and `cargo test -p worldwake-ai`

### Invariants

1. A lawful start failure must remain distinguishable from request-resolution rejection before start.
2. Shared provenance for a failed start must survive from request resolution through authoritative `StartFailed` and into next-tick AI reconciliation.
3. The new traceability must be shared infrastructure, not a political-only or golden-only hack.

## Test Plan

### New/Modified Tests

1. `tick_step::tests::best_effort_stale_request_records_start_failure_when_affordance_no_longer_matches` in [tick_step.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/tick_step.rs) — strengthened to assert that input `sequence_no`, `RequestProvenance`, and `RequestBindingKind` survive consistently across request resolution, scheduler `ActionStartFailure`, and action trace `StartFailed`.
2. `tick_step::tests::reproduced_request_records_request_resolution_trace_before_start` in [tick_step.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/tick_step.rs) — strengthened to assert request `sequence_no` and provenance on successful request binding as the shared correlation substrate.
3. `tick_step::tests::strict_request_records_resolution_rejection_without_start_attempt` in [tick_step.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/tick_step.rs) — strengthened to assert request `sequence_no` and provenance are still preserved on the pre-start rejection branch, keeping the first-failure boundary explicit.
4. `agent_tick::tests::planning_trace_includes_scheduler_start_failures_for_wound_abort_reasons` in [agent_tick.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/agent_tick.rs) — strengthened to assert next-tick AI reconciliation receives the same request-attempt context carried by the scheduler failure.
5. `golden_remote_office_claim_start_failure_loses_gracefully` in [golden_emergent.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_emergent.rs) — strengthened to assert one real mixed-layer political start failure carries identical request context across request-resolution trace, action trace, scheduler failure storage, and AI next-tick reconciliation.
6. `golden_contested_harvest_start_failure_recovers_via_remote_fallback` in [golden_production.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_production.rs) — strengthened to assert the losing AI-plan start failure keeps the same shared request context on the scheduler failure record.
7. `golden_local_trade_start_failure_recovers_via_production_fallback` in [golden_trade.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_trade.rs) — updated for the request-attempt trace shape and kept asserting the authoritative start-failure boundary in trade.

### Commands

1. `cargo test -p worldwake-sim tick_step::tests::best_effort_stale_request_records_start_failure_when_affordance_no_longer_matches -- --exact`
2. `cargo test -p worldwake-sim tick_step::tests::strict_request_records_resolution_rejection_without_start_attempt -- --exact`
3. `cargo test -p worldwake-ai --lib agent_tick::tests::planning_trace_includes_scheduler_start_failures_for_wound_abort_reasons -- --exact`
4. `cargo test -p worldwake-ai --test golden_production golden_contested_harvest_start_failure_recovers_via_remote_fallback -- --exact`
5. `cargo test -p worldwake-ai --test golden_trade golden_local_trade_start_failure_recovers_via_production_fallback -- --exact`
6. `cargo test -p worldwake-ai --test golden_emergent golden_remote_office_claim_start_failure_loses_gracefully -- --exact`
7. `cargo test -p worldwake-sim`
8. `cargo test -p worldwake-ai`
9. `cargo clippy --workspace`

## Outcome

- Completion date: 2026-03-20
- What changed:
  - Added a shared request-attempt trace model in `worldwake-sim` carrying input `sequence_no`, `RequestProvenance`, and `RequestBindingKind`.
  - Extended scheduler `ActionStartFailure`, action-trace `StartFailed`, and AI `ActionStartFailureSummary` to carry that shared context directly.
  - Strengthened existing focused and golden tests to assert provenance continuity across request resolution, authoritative start failure, action trace, and next-tick AI reconciliation.
- Deviations from original plan:
  - Did not add new start-failure goldens. Reused the existing production, trade, and political S15 goldens because those scenarios already existed and already proved the recovery behavior the ticket initially described as missing.
  - Kept the change on shared trace infrastructure only; no action semantics or political/production/trade rules changed.
- Verification results:
  - `cargo test -p worldwake-sim tick_step::tests::best_effort_stale_request_records_start_failure_when_affordance_no_longer_matches -- --exact`
  - `cargo test -p worldwake-sim tick_step::tests::strict_request_records_resolution_rejection_without_start_attempt -- --exact`
  - `cargo test -p worldwake-ai --lib agent_tick::tests::planning_trace_includes_scheduler_start_failures_for_wound_abort_reasons -- --exact`
  - `cargo test -p worldwake-ai --test golden_production golden_contested_harvest_start_failure_recovers_via_remote_fallback -- --exact`
  - `cargo test -p worldwake-ai --test golden_trade golden_local_trade_start_failure_recovers_via_production_fallback -- --exact`
  - `cargo test -p worldwake-ai --test golden_emergent golden_remote_office_claim_start_failure_loses_gracefully -- --exact`
  - `cargo test -p worldwake-sim`
  - `cargo test -p worldwake-ai`
  - `cargo clippy --workspace`
