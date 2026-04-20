# S110DECHISEVE-007: Candidate offer and suppression provenance for decision events

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes — candidate-generation/runtime plumbing to expose authoritative `GoalOffered` and `GoalSuppressed` payload causes
**Deps**: archive/tickets/S110DECHISEVE-004.md

## Problem

`S110DECHISEVE-004` intentionally defers `GoalOffered` and `GoalSuppressed` because the live runtime does not yet expose foundations-honest emitter identity and suppression reasons at an `EventLog` write seam. This ticket adds that missing provenance so those events can become authoritative history rather than inferred observer summaries.

## Assumption Reassessment (2026-04-20)

1. `crates/worldwake-ai/src/candidate_generation.rs::emit_candidate_with_trace` knows the concrete candidate-emission site, but today it does not surface an authoritative `EmitterTag` to any caller that owns `EventLog`.
2. Candidate blocker/discrepancy suppression happens in `candidate_generation.rs::{filter_suppressed_candidates,find_matching_suppression}` before ranking, while ranking-level suppression comes from `crates/worldwake-ai/src/goal_policy.rs::evaluate_suppression`.
3. The core enum `GoalRejectionReason` currently covers blocker/discrepancy/contention classes but not generic stress-policy suppression, and the core `EmitterTag` / `EvidenceKindTag` families are also narrower than the live candidate-generation emitter inventory. This ticket must widen the core schema honestly instead of inferring lossy fallback categories from `GoalKind`.
4. Shared abstraction boundary under audit: candidate-generation and ranking diagnostics carried through `refresh_runtime_for_read_phase_with_memories(...)` into the `agent_tick/mod.rs` orchestration layer that owns `ctx.event_log`.

## Architecture Check

1. Surfacing emitter and suppression provenance through candidate-generation diagnostics is cleaner than reconstructing those reasons later from goal kind or trace output.
2. No compatibility aliasing: one authoritative transport path for offer/suppression provenance should exist after this ticket.

## Verification Layers

1. Emitter provenance survives candidate generation -> focused candidate-generation/runtime test.
2. Suppression reason mapping remains authoritative -> focused candidate-generation test for blocker/discrepancy/contention and any schema-added policy case.
3. Event emission uses returned provenance without inference -> focused `agent_tick` runtime test.

## What to Change

### 1. Add authoritative offer provenance to candidate-generation diagnostics

Surface enough data from candidate-generation emit sites to construct `GoalOfferedPayload { emitter, source_evidence }` at the `agent_tick` write seam without guessing.

### 2. Add typed suppression provenance

Surface blocker/discrepancy suppression reasons directly from candidate generation, surface stress-policy suppression from ranking, and widen the core schema as needed so every emitted `GoalSuppressed` reason comes from a concrete live classifier.

### 3. Emit `GoalOffered` and `GoalSuppressed`

Once the provenance exists, emit those events from the `agent_tick/mod.rs` read-phase orchestration seam that owns both the returned diagnostics and `EventLog`.

## Files to Touch

- `crates/worldwake-ai/src/candidate_generation.rs` (modify)
- `crates/worldwake-ai/src/ranking.rs` (modify)
- `crates/worldwake-ai/src/agent_tick/{observation,mod}.rs` (modify)
- `crates/worldwake-core/src/decision_event_payload.rs` (modify — schema correction required)

## Out of Scope

- Plan invalidation or repair events
- Observer rendering

## Acceptance Criteria

### Tests That Must Pass

1. Focused test proves one emitted `GoalOffered` payload contains the correct authoritative emitter and evidence summary.
2. Focused test proves one emitted `GoalSuppressed` payload carries the exact suppression reason from the authoritative suppression seam.
3. `cargo test -p worldwake-ai`

### Invariants

1. `GoalOffered` and `GoalSuppressed` payloads are constructed from live runtime provenance, not inferred from `GoalKind`.
2. No suppression reason is emitted unless the live runtime actually classified that reason.

## Test Plan

### New/Modified Tests

1. Candidate-generation focused tests for surfaced provenance.
2. `agent_tick` runtime test for actual event emission.

### Commands

1. `cargo test -p worldwake-ai candidate_generation`
2. `cargo test -p worldwake-ai`
3. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

Completed on 2026-04-20.

1. Widened the core decision-history schema in `crates/worldwake-core/src/decision_event_payload.rs` so live candidate-generation emitters and evidence families have authoritative tags, and added `GoalRejectionReason::SuppressedByStressPolicy` for ranking-level suppression.
2. Extended candidate-generation diagnostics in `crates/worldwake-ai/src/candidate_generation.rs` to carry authoritative offer provenance (`EmitterTag`, `EvidenceSummary`) and typed suppression reasons from blocker/discrepancy filtering.
3. Extended ranking suppression output in `crates/worldwake-ai/src/ranking.rs` so stress-policy suppression is surfaced as explicit decision-event provenance instead of being reconstructed later from goal kind.
4. Carried offer/suppression diagnostics through `crates/worldwake-ai/src/agent_tick/observation.rs` and emitted `GoalOffered` / `GoalSuppressed` from the real `EventLog` seam in `crates/worldwake-ai/src/agent_tick/mod.rs`.
5. Added focused tests in `crates/worldwake-ai/src/candidate_generation.rs`, `crates/worldwake-ai/src/ranking.rs`, and `crates/worldwake-ai/src/agent_tick/tests.rs` proving offer provenance, stress-policy suppression, and runtime event emission for a blocked acquire candidate.

## Verification Result

Passed on 2026-04-20:

1. `cargo test -p worldwake-ai candidate_generation::tests::diagnostics_record_offer_emitter_and_blocker_suppression_reason -- --exact`
2. `cargo test -p worldwake-ai ranking::tests::suppressed_candidates_record_stress_policy_reason -- --exact`
3. `cargo test -p worldwake-ai agent_tick::tests::read_phase_emits_goal_offered_and_goal_suppressed_events_from_candidate_provenance -- --exact`
4. `cargo test -p worldwake-ai`
5. `cargo test --workspace`
6. `cargo clippy --workspace --all-targets -- -D warnings`
