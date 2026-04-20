# S110DECHISEVE-007: Candidate offer and suppression provenance for decision events

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes — candidate-generation/runtime plumbing to expose authoritative `GoalOffered` and `GoalSuppressed` payload causes
**Deps**: archive/tickets/S110DECHISEVE-004.md

## Problem

`S110DECHISEVE-004` intentionally defers `GoalOffered` and `GoalSuppressed` because the live runtime does not yet expose foundations-honest emitter identity and suppression reasons at an `EventLog` write seam. This ticket adds that missing provenance so those events can become authoritative history rather than inferred observer summaries.

## Assumption Reassessment (2026-04-20)

1. `crates/worldwake-ai/src/candidate_generation.rs::emit_candidate_with_trace` knows the concrete candidate-emission site, but today it does not surface an authoritative `EmitterTag` to any caller that owns `EventLog`.
2. Candidate blocker/discrepancy suppression happens in `candidate_generation.rs::{filter_suppressed_candidates,find_matching_suppression}` before ranking, while ranking-level suppression comes from `crates/worldwake-ai/src/goal_policy.rs::evaluate_suppression`.
3. The core enum `GoalRejectionReason` currently covers blocker/discrepancy/contention classes but not generic stress-policy suppression. Any implementation must either extend the schema honestly or narrow the emitted scope explicitly.
4. Shared abstraction boundary under audit: candidate-generation diagnostics returned to `agent_tick/observation.rs`.

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

Surface blocker/discrepancy/contention suppression reasons directly from candidate generation, and reassess whether stress-policy suppression needs a new core enum variant or a narrowed event scope.

### 3. Emit `GoalOffered` and `GoalSuppressed`

Once the provenance exists, emit those events from the first `agent_tick` layer that owns both the diagnostics and `EventLog`.

## Files to Touch

- `crates/worldwake-ai/src/candidate_generation.rs` (modify)
- `crates/worldwake-ai/src/agent_tick/observation.rs` (modify)
- `crates/worldwake-core/src/decision_event_payload.rs` (modify if schema correction is required)

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
