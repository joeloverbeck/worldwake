# S15STAFAIEME-008: Add Start-Failure Provenance Across Request Resolution, Action Trace, And Decision Trace

**Status**: PENDING
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

## Assumption Reassessment (2026-03-19)

1. The request-resolution layer already records provenance and binding in [request_resolution_trace.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/request_resolution_trace.rs): `RequestResolutionTraceEvent` stores `provenance`, and `RequestResolutionOutcome::Bound` stores `RequestBindingKind` plus `start_attempted`.
2. The authoritative start-failure layer currently drops that provenance. [scheduler.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/scheduler.rs) stores `ActionStartFailure { tick, actor, def_id, reason }` only, and [action_trace.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/action_trace.rs) stores `ActionTraceKind::StartFailed { reason }` without request provenance or binding details.
3. The AI layer inherits the same loss of context. [decision_trace.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/decision_trace.rs) stores `ActionStartFailureSummary { tick, def_id, reason }` only, so `planning.action_start_failures` cannot directly tell whether the failure came from an `AiPlan` or `External` request path.
4. Existing focused coverage already proves the request-resolution and start-failure boundaries separately: `tick_step::tests::strict_request_records_resolution_rejection_without_start_attempt` and `tick_step::tests::best_effort_stale_request_records_start_failure_when_affordance_no_longer_matches` in [tick_step.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/tick_step.rs). I confirmed those exact test names today with `cargo test -p worldwake-sim --lib -- --list`.
5. Existing golden coverage already proves the AI recovery side of the chain in production, trade, care, and politics, including `golden_remote_office_claim_start_failure_loses_gracefully` in [golden_emergent.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_emergent.rs). The gap is not missing recovery behavior; it is missing traceability continuity across layers.
6. The first failure boundary for the target scenarios remains authoritative start, not request-resolution rejection. This ticket must preserve that distinction rather than flattening all stale-request failures into one generic path.
7. This should be implemented as shared runtime trace enrichment, not as political-specific or golden-specific metadata. The gap exists equally for care, production, trade, and politics.
8. Scope correction: do not solve this by adding ad hoc assertions to one golden. The architecture needs a durable shared provenance model linking request resolution, authoritative `StartFailed`, and decision-trace blocker consumption.

## Architecture Check

1. A shared provenance model for start-failure traces is cleaner than teaching each golden to correlate three sinks by hand.
2. No backwards-compatibility aliasing or duplicate trace pathways should be introduced. Extend the existing request-resolution/action-trace/decision-trace surfaces directly.

## Verification Layers

1. Request provenance and binding survive the request-resolution boundary -> focused runtime test in [tick_step.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/tick_step.rs).
2. The same provenance/binding is visible on authoritative `StartFailed` records -> focused runtime/action-trace coverage in [tick_step.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/tick_step.rs).
3. The AI decision trace receives the same enriched start-failure summary on the next tick -> focused AI runtime test in [agent_tick.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/agent_tick.rs).
4. A golden mixed-layer scenario can assert the full provenance chain without inference -> targeted golden update in [golden_emergent.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_emergent.rs) or another existing S08-style golden.
5. Later authoritative office installation or other downstream world state is not a proxy for this ticket's contract. The contract is provenance continuity across request resolution, authoritative start failure, and AI recovery surfaces.

## What to Change

### 1. Extend shared start-failure records with request provenance

Add shared provenance data to the start-failure path so the same failed start can be inspected consistently across:

- request-resolution trace
- scheduler `ActionStartFailure`
- action trace `StartFailed`
- AI `ActionStartFailureSummary`

At minimum, preserve:

- `RequestProvenance`
- request binding path (`ReproducedAffordance` vs `BestEffortFallback`)
- enough stable identity to correlate the failed start across the sinks without relying on incidental timing alone

### 2. Add focused cross-layer tests

Strengthen focused runtime and AI tests so they prove the provenance survives the full chain from `RequestAction` resolution into `StartFailed` and then into next-tick AI reconciliation.

### 3. Add one golden assertion on the shared provenance path

Update one existing S08-style golden to assert the new shared provenance fields in a real mixed-layer scenario, so the contract stays usable at the E2E level.

## Files to Touch

- `crates/worldwake-sim/src/request_resolution_trace.rs` (modify)
- `crates/worldwake-sim/src/action_trace.rs` (modify)
- `crates/worldwake-sim/src/scheduler.rs` (modify)
- `crates/worldwake-sim/src/tick_step.rs` (modify)
- `crates/worldwake-ai/src/decision_trace.rs` (modify)
- `crates/worldwake-ai/src/agent_tick.rs` (modify)
- `crates/worldwake-ai/tests/golden_emergent.rs` (modify) or another existing S08-style golden that already proves start-failure reconciliation

## Out of Scope

- changing start-failure behavior itself
- changing office claim rules or support-law succession semantics
- adding domain-specific provenance only for political actions
- introducing a separate compatibility trace sink for the same data

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-sim tick_step::tests::strict_request_records_resolution_rejection_without_start_attempt -- --exact`
2. `cargo test -p worldwake-sim tick_step::tests::best_effort_stale_request_records_start_failure_when_affordance_no_longer_matches -- --exact`
3. `cargo test -p worldwake-ai golden_remote_office_claim_start_failure_loses_gracefully -- --exact`
4. Existing suites: `cargo test -p worldwake-sim` and `cargo test -p worldwake-ai`

### Invariants

1. A lawful start failure must remain distinguishable from request-resolution rejection before start.
2. Shared provenance for a failed start must survive from request resolution through authoritative `StartFailed` and into next-tick AI reconciliation.
3. The new traceability must be shared infrastructure, not a political-only or golden-only hack.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-sim/src/tick_step.rs` — prove request provenance and binding survive from `RequestAction` resolution into authoritative `StartFailed`.
2. `crates/worldwake-ai/src/agent_tick.rs` — prove `planning.action_start_failures` exposes the enriched provenance on the next AI tick.
3. `crates/worldwake-ai/tests/golden_emergent.rs` — prove one real mixed-layer start-failure golden can assert the provenance chain without manual inference across unrelated sinks.

### Commands

1. `cargo test -p worldwake-sim tick_step::tests::best_effort_stale_request_records_start_failure_when_affordance_no_longer_matches -- --exact`
2. `cargo test -p worldwake-ai golden_remote_office_claim_start_failure_loses_gracefully -- --exact`
3. `cargo test -p worldwake-sim`
4. `cargo test -p worldwake-ai`
5. `cargo clippy --workspace`
