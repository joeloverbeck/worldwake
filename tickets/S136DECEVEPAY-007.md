# S136DECEVEPAY-007: Thread introduced_at_step provenance into PlanAssumptionRef

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes - `worldwake-ai::agent_tick::frame::populate_assumptions` provenance tracking and S136 assumption payload conversion
**Deps**: archive/tickets/S136DECEVEPAY-002.md

## Problem

S136DECEVEPAY-002 can populate `PlanAssumptionRef.assumption` from the active `IntentionFrame.assumptions`, but `FrameAssumption` currently stores only the assumption value. It does not record which plan step introduced the assumption. The ticket 002 converter must therefore emit `introduced_at_step: 0` for every assumption, which is truthful but loses step-level provenance.

This ticket threads the real step provenance into the frame-assumption population path so S136 payloads can identify the plan step that introduced each assumption.

## Assumption Reassessment (2026-05-06)

1. `FrameAssumption` is defined in `crates/worldwake-core/src/intention_frame.rs` and is stored as `IntentionFrame.assumptions: Vec<FrameAssumption>`. The live type carries no step-index metadata.
2. `PlanAssumptionRef` in `crates/worldwake-core/src/decision_event_payload.rs` already has `introduced_at_step: u8`, so no core payload widening is required for this ticket.
3. `populate_assumptions` in `crates/worldwake-ai/src/agent_tick/frame.rs` receives an `IntentionFrame` and plan completion tick, but it does not receive a per-step provenance map. The missing contract is AI-side provenance capture, not event-log shape.
4. The data contract under audit is `FrameAssumption` value -> `PlanAssumptionRef { assumption, introduced_at_step }` in the always-on decision-event payload.

## Architecture Check

1. The clean fix is to make assumption provenance explicit at the frame-population boundary, then keep the event emitter a mechanical conversion from active-frame assumptions to payload refs.
2. No compatibility shim or duplicate event payload path is needed; S136 payload shape already contains the destination field.

## Verification Layers

1. Step provenance capture -> focused unit test around `populate_assumptions` or its replacement helper using a multi-step plan where assumptions originate from different steps.
2. Payload conversion -> focused decision-event test asserting `GoalCommittedPayload.assumptions[*].introduced_at_step` reflects the non-zero source step.
3. Integration sanity -> `cargo test -p worldwake-ai agent_tick::`.

## What to Change

### 1. Represent assumption provenance before conversion

Add the narrowest AI-side carrier needed to preserve `(FrameAssumption, introduced_at_step)` from frame population through decision-event emission. Prefer a local helper/type in `agent_tick::frame` or `agent_tick` unless reassessment proves the provenance must persist in core `IntentionFrame`.

### 2. Replace the ticket 002 fallback

Update the S136 assumption conversion so `introduced_at_step` is populated from real provenance instead of the ticket 002 fallback value `0`.

## Files to Touch

- `crates/worldwake-ai/src/agent_tick/frame.rs` (modify)
- `crates/worldwake-ai/src/agent_tick/mod.rs` (modify)
- `crates/worldwake-ai/src/agent_tick/planning.rs` (modify if conversion call shape changes)
- `crates/worldwake-ai/src/agent_tick/tests.rs` or nearby focused test module (modify)

## Out of Scope

- Adding new `FrameAssumption` variants.
- Widening S136 core payload structs.
- Populating `decisive_*` fields.
- Observer rendering changes.

## Acceptance Criteria

### Tests That Must Pass

1. Focused provenance test proves at least one emitted `PlanAssumptionRef` has a non-zero `introduced_at_step` when the originating assumption comes from a later plan step.
2. Existing S136 assumption-emission tests still pass.
3. Existing agent tick suite passes: `cargo test -p worldwake-ai agent_tick::`.

### Invariants

1. `assumption` remains the same value produced by the existing frame-assumption logic.
2. `introduced_at_step` is bounded to `u8` with explicit conversion failure if a plan step index exceeds the payload limit.
3. No new belief query or ranking pass is introduced solely to compute provenance.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/agent_tick/frame.rs::tests` or `crates/worldwake-ai/src/agent_tick/tests.rs` - focused provenance fixture for non-zero `introduced_at_step`.
2. Existing S136 assumption event tests - update expected refs from fallback `0` to real provenance where applicable.

### Commands

1. `cargo test -p worldwake-ai --lib <focused provenance test name>`
2. `cargo test -p worldwake-ai agent_tick::`
