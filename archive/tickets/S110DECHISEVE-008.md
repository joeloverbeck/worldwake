# S110DECHISEVE-008: Invalidation and goal-transition reason transport for decision events

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes — widen core decision-event payload contracts to match authoritative invalidation and goal-transition causes, then surface them to the event-log seam
**Deps**: archive/tickets/S110DECHISEVE-004.md

## Problem

`S110DECHISEVE-004` defers `PlanInvalidated`, `GoalSuspended`, `GoalAbandoned`, and richer `ReplanTriggered` because the live runtime does not currently carry those reasons to one authoritative `EventLog` write seam in the shapes S110 expects. This ticket adds that reason transport.

## Assumption Reassessment (2026-04-20)

1. `crates/worldwake-ai/src/plan_revalidation.rs` is a pure validator and returns local `PursuitInvalidationReason`; it does not own `EventLog`, and the current core `PlanInvalidationReason` does not yet represent those causes honestly.
2. Frame suspension/clearing happens across `crates/worldwake-ai/src/agent_tick/{frame,mod,planning}.rs`, with local reasons such as `SuspensionReason`, `FrameClearReason`, and `GoalSwitchKind`; the current `GoalAbandonedPayload` shape does not match that runtime authority surface.
3. `ReplanTriggered` currently has multiple concrete causes (`InterruptReason`, pursuit invalidation, expectation mismatch, action-start failure, blocker/discrepancy failure paths), but the current core `ReplanReason` enum is too coarse to carry them honestly.
4. Shared abstraction boundary under audit: runtime invalidation and frame-transition results returned into `agent_tick/mod.rs`, plus the core decision-event enums that must represent those results without lossy inference.

## Architecture Check

1. Explicit reason transport is cleaner than reverse-engineering invalidation from final runtime state after the fact.
2. One authoritative mapping layer should own the conversion from local invalidation/frame reasons to S110 payload enums.
3. Because the live authority surface is broader than the current core enums, this ticket must correct those core enums first rather than coercing live causes into approximate existing variants.

## Verification Layers

1. Core payload widening -> focused `worldwake-core` round-trip tests.
2. Invalidation reason mapping -> focused runtime/unit tests at the transport layer.
3. Goal suspension/abandonment mapping -> focused frame/agent_tick tests.
4. Event-log emission order and payloads -> focused `agent_tick` runtime tests.

## What to Change

### 1. Correct the core payload contracts

Widen the shared `DecisionEventPayload` support types in `worldwake-core` so they can carry the real live causes exposed by frame clearing, pursuit invalidation, and replan triggers. This includes replacing the current `GoalAbandonedPayload` reason type and widening `PlanInvalidationReason` / `ReplanReason` where needed.

### 2. Introduce authoritative invalidation result transport

Carry invalidation results from validators and frame transitions into the orchestration layer that owns `EventLog`.

### 3. Map live runtime reasons to S110 payload enums

Define the canonical mapping for `PlanInvalidationReason`, `GoalSuspendedPayload`, `GoalAbandonedPayload`, and `ReplanReason`.

### 4. Emit deferred decision events

Emit `PlanInvalidated`, `GoalSuspended`, `GoalAbandoned`, and the now-honest `ReplanTriggered`.

## Files to Touch

- `crates/worldwake-core/src/decision_event_payload.rs` (modify)
- `crates/worldwake-core/src/lib.rs` (modify)
- `crates/worldwake-ai/src/agent_tick/{frame,mod,observation,active_action}.rs` (modify as needed)
- `crates/worldwake-ai/src/failure_handling.rs` (modify as needed)
- `crates/worldwake-ai/src/plan_revalidation.rs` (modify only if transport/helper changes are required)

## Out of Scope

- Candidate offer/suppression provenance
- Repair events

## Acceptance Criteria

### Tests That Must Pass

1. Focused tests prove each emitted invalidation/goal-transition event carries the exact authoritative reason.
2. Core payload round-trip tests cover every newly added reason variant.
3. `cargo test -p worldwake-ai`

### Invariants

1. No invalidation or goal-transition event is emitted from inferred final state alone.
2. `ReplanTriggered` reason is sourced from the concrete trigger that caused replanning.
3. Core payload enums reflect the live authoritative cause classes instead of coercing them into lossy legacy variants.

## Test Plan

### New/Modified Tests

1. `worldwake-core` decision-payload tests plus focused `agent_tick` tests.

### Commands

1. `cargo test -p worldwake-core decision_event_payload`
2. `cargo test -p worldwake-ai agent_tick`
3. `cargo test -p worldwake-ai`
4. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

Completed on 2026-04-20.

- Widened the core decision-event contracts in `worldwake-core` so `GoalAbandoned`, `PlanInvalidated`, and `ReplanTriggered` can carry live frame-clear, pursuit invalidation, interrupt, and blocker/discrepancy causes without lossy coercion.
- Threaded reconciliation and failure results into the `agent_tick` event seam, and emitted authoritative `GoalSuspended`, `PlanInvalidated`, `GoalAbandoned`, and `ReplanTriggered` events from the real runtime causes now exposed there.
- Added focused proof coverage for the new reason mappings plus a death-cleanup integration proof for `GoalAbandoned`.

## Verification Result

Passed:

1. `cargo test -p worldwake-core decision_event_payload`
2. `cargo test -p worldwake-ai agent_tick`
3. `cargo test -p worldwake-ai`
4. `cargo test --workspace`
5. `cargo clippy --workspace --all-targets -- -D warnings`
