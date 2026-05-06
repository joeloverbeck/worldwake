# S136DECEVEPAY-007: Thread introduced_at_step provenance into PlanAssumptionRef

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes - S136 assumption payload conversion now derives step provenance from the current `PlannedPlan`
**Deps**: archive/tickets/S136DECEVEPAY-002.md

## Problem

S136DECEVEPAY-002 populated `PlanAssumptionRef.assumption` from the active `IntentionFrame.assumptions`, but `FrameAssumption` stores only the assumption value. Before this ticket, the converter emitted `introduced_at_step: 0` for every assumption, which was truthful but lost step-level provenance.

This ticket threads real step provenance into the S136 assumption payload conversion path so S136 payloads can identify the plan step that introduced each assumption.

## Assumption Reassessment (2026-05-06)

1. `FrameAssumption` is defined in `crates/worldwake-core/src/intention_frame.rs` and is stored as `IntentionFrame.assumptions: Vec<FrameAssumption>`. The live type carries no step-index metadata.
2. `PlanAssumptionRef` in `crates/worldwake-core/src/decision_event_payload.rs` already has `introduced_at_step: u8`, so no core payload widening is required for this ticket.
3. `populate_assumptions` in `crates/worldwake-ai/src/agent_tick/frame.rs` receives an `IntentionFrame` and plan completion tick, but it does not receive a per-step provenance map. Live reassessment found the current `PlannedPlan` is still available at the success and failure emission seams, so provenance can be derived during payload conversion without widening `IntentionFrame`.
4. The data contract under audit is `FrameAssumption` value -> `PlanAssumptionRef { assumption, introduced_at_step }` in the always-on decision-event payload.

## Architecture Check

1. The clean fix is to make assumption provenance explicit at the event payload conversion boundary, where the active frame assumptions and current `PlannedPlan` are both in scope.
2. No compatibility shim, duplicate event payload path, or core frame widening is needed; S136 payload shape already contains the destination field.

## Verification Layers

1. Step provenance capture -> focused unit test around `populate_assumptions` or its replacement helper using a multi-step plan where assumptions originate from different steps.
2. Payload conversion -> focused decision-event test asserting `GoalCommittedPayload.assumptions[*].introduced_at_step` reflects the non-zero source step.
3. Integration sanity -> `cargo test -p worldwake-ai agent_tick::`.

## What to Change

### 1. Represent assumption provenance before conversion

Extend the existing AI-side `AssumptionRefContext` so it can carry the current `PlannedPlan` alongside active-frame assumptions into decision-event emission. Derive source steps during conversion:

- `RouteExists` -> first matching travel step to the assumed destination.
- `CommodityAvailableAt` -> first non-travel step at the assumed place.
- `TargetAlive` -> first step targeting the entity.
- whole-plan assumptions (`NoCriticalThreat`, `NeedSafeUntilTick`) keep `introduced_at_step: 0`.

### 2. Replace the ticket 002 fallback

Update the S136 assumption conversion so `introduced_at_step` is populated from real provenance instead of the ticket 002 fallback value `0`.

## Files to Touch

- `crates/worldwake-ai/src/agent_tick/active_action.rs` (modify)
- `crates/worldwake-ai/src/agent_tick/execution.rs` (modify)
- `crates/worldwake-ai/src/agent_tick/mod.rs` (modify)
- `crates/worldwake-ai/src/agent_tick/observation.rs` (modify)
- `crates/worldwake-ai/src/agent_tick/planning.rs` (modify)
- `crates/worldwake-ai/src/agent_tick/tests.rs` or nearby focused test module (modify)
- `crates/worldwake-ai/src/agent_tick/frame.rs` (checked; no change required because provenance derives from `PlannedPlan`)

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

1. `cargo test -p worldwake-ai --lib agent_tick::tests::assumption_refs_record_nonzero_source_step_from_plan -- --exact`
2. `cargo test -p worldwake-ai --lib agent_tick::planning::tests::emit_plan_selection_events_records_commit_then_adoption_with_truncation -- --exact`
3. `cargo test -p worldwake-ai agent_tick::`

## Outcome

Completed on 2026-05-06.

Landed plan-derived assumption provenance in `worldwake-ai::agent_tick`: `AssumptionRefContext` now carries an optional current `PlannedPlan`, and `assumptions_to_refs` derives `introduced_at_step` from the first matching source step while preserving the existing bounded conversion. Success-path `GoalCommitted` / `PlanAdopted`, failure-path `ReplanTriggered` / `BlockerRecorded`, and `ExpectationMismatch` now pass the current plan where the live emission seam has it.

No `IntentionFrame` or core payload shape widening was required. Whole-plan assumptions that do not belong to one concrete step keep `introduced_at_step: 0`.

## Verification Result

Passed on 2026-05-06:

1. `cargo test -p worldwake-ai --lib assumption_refs_record_nonzero_source_step_from_plan -- --list`
2. `cargo test -p worldwake-ai --lib agent_tick::tests::assumption_refs_record_nonzero_source_step_from_plan -- --exact`
3. `cargo test -p worldwake-ai --lib agent_tick::planning::tests::emit_plan_selection_events_records_commit_then_adoption_with_truncation -- --exact`
4. `cargo test -p worldwake-ai --lib agent_tick::observation::tests::emit_expectation_mismatch_records_expected_tags_and_step_index -- --exact`
5. `cargo fmt --all`
6. `cargo test -p worldwake-ai agent_tick::`
