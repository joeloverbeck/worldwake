# S110DECHISEVE-008: Invalidation and goal-transition reason transport for decision events

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes — surface authoritative invalidation and goal-transition reasons to the event-log seam
**Deps**: archive/tickets/S110DECHISEVE-004.md

## Problem

`S110DECHISEVE-004` defers `PlanInvalidated`, `GoalSuspended`, `GoalAbandoned`, and richer `ReplanTriggered` because the live runtime does not currently carry those reasons to one authoritative `EventLog` write seam in the shapes S110 expects. This ticket adds that reason transport.

## Assumption Reassessment (2026-04-20)

1. `crates/worldwake-ai/src/plan_revalidation.rs` is a pure validator and returns `bool` plus local `PursuitInvalidationReason`; it does not own `EventLog`.
2. Frame suspension/clearing happens across `crates/worldwake-ai/src/agent_tick/{frame,mod,planning}.rs`, with local reasons such as `SuspensionReason`, `FrameClearReason`, and `GoalSwitchKind`, but not yet a unified S110 payload mapping.
3. `ReplanTriggered` currently has only partial local causes (`DirtySet::REPLAN_SIGNAL`, failed-step handling, pursuit invalidation, frame loss). These need explicit authoritative mapping before event emission.
4. Shared abstraction boundary under audit: runtime invalidation and frame-transition results returned into `agent_tick/mod.rs`.

## Architecture Check

1. Explicit reason transport is cleaner than reverse-engineering invalidation from final runtime state after the fact.
2. One authoritative mapping layer should own the conversion from local invalidation/frame reasons to S110 payload enums.

## Verification Layers

1. Invalidation reason mapping -> focused runtime/unit tests at the transport layer.
2. Goal suspension/abandonment mapping -> focused frame/agent_tick tests.
3. Event-log emission order and payloads -> focused `agent_tick` runtime tests.

## What to Change

### 1. Introduce authoritative invalidation result transport

Carry invalidation results from validators and frame transitions into the orchestration layer that owns `EventLog`.

### 2. Map live runtime reasons to S110 payload enums

Define the canonical mapping for `PlanInvalidationReason`, `GoalSuspendedPayload`, `GoalAbandonedPayload`, and `ReplanReason`.

### 3. Emit deferred decision events

Emit `PlanInvalidated`, `GoalSuspended`, `GoalAbandoned`, and the now-honest `ReplanTriggered`.

## Files to Touch

- `crates/worldwake-ai/src/plan_revalidation.rs` (modify)
- `crates/worldwake-ai/src/agent_tick/frame.rs` (modify)
- `crates/worldwake-ai/src/agent_tick/mod.rs` (modify)
- `crates/worldwake-core/src/decision_event_payload.rs` (modify if payload correction is needed)

## Out of Scope

- Candidate offer/suppression provenance
- Repair events

## Acceptance Criteria

### Tests That Must Pass

1. Focused tests prove each emitted invalidation/goal-transition event carries the exact authoritative reason.
2. `cargo test -p worldwake-ai`

### Invariants

1. No invalidation or goal-transition event is emitted from inferred final state alone.
2. `ReplanTriggered` reason is sourced from the concrete trigger that caused replanning.

## Test Plan

### New/Modified Tests

1. `plan_revalidation` and `agent_tick` focused tests.

### Commands

1. `cargo test -p worldwake-ai agent_tick`
2. `cargo test -p worldwake-ai`
3. `cargo clippy --workspace --all-targets -- -D warnings`
