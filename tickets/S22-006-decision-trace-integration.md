# S22-006: Add decision trace integration for IntentionFrame lifecycle

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: Yes — new trace events in DecisionTraceSink
**Deps**: S22-002 (frame types active), S22-003 (assumption evaluation), S22-004 (progress detection), S22-005 (exhaustion/blocked intent)

## Problem

Without decision trace integration, frame lifecycle events (creation, progress, suspension, resume, exhaustion, clearing) are invisible in debug output. This violates P27 (debuggability is a product feature). Developers cannot answer "why did this agent abandon its journey?" or "why did this agent resume its care commitment?" without trace data.

## Assumption Reassessment (2026-03-24)

1. `DecisionTraceSink` currently records `AgentDecisionTrace` per agent per tick, with `DecisionOutcome` variants including `ActiveAction` and `Planning`. Confirmed in `decision_trace.rs`.
2. `dump_agent()` already produces human-readable debug output. Frame events must be included.
3. `summary()` on `DecisionOutcome` produces one-line strings. Frame events should appear in summaries.
4. The existing journey system had `JourneyDebugSnapshot` and `JourneyRuntimeSnapshot` for trace data. These are replaced by `FrameDebugSnapshot` and `FrameRuntimeSnapshot` in S22-002. This ticket adds lifecycle event recording on top of those snapshots.
5. This is a debuggability/observability ticket. Single-layer verification: trace output correctness.

## Architecture Check

1. Adding `FrameTransitionTrace` as an optional field on existing `DecisionOutcome` variants follows the existing pattern of optional trace extensions (e.g., `InterruptTrace`).
2. No backward-compatibility concerns — purely additive trace data.

## Verification Layers

1. Frame creation recorded in trace → focused test with `enable_tracing()`
2. Frame progress recorded → focused test
3. Frame suspension recorded with reason → focused test
4. Frame exhaustion recorded with stalled_ticks/patience_limit → focused test
5. Frame clearing recorded with reason → focused test
6. `dump_agent()` includes frame events → manual inspection or snapshot test
7. Single-layer ticket: trace data is the proof surface.

## What to Change

### 1. New trace types in `decision_trace.rs`

```rust
#[derive(Clone, Debug)]
pub enum FrameTransitionKind {
    Created { goal: GoalKey, domain_tag: IntentionDomainTag, patience_limit: u32, assumptions_count: usize },
    Progressed { tick: Tick },
    Suspended { reason: SuspensionReason, tick: Tick },
    Resumed { tick: Tick },
    Exhausted { stalled_ticks: u32, patience_limit: u32, blocked_intent_recorded: bool },
    Cleared { reason: FrameClearReason },
}

#[derive(Clone, Debug)]
pub struct FrameTransitionTrace {
    pub transitions: Vec<FrameTransitionKind>,
}
```

### 2. Extend DecisionOutcome

Add `frame_transition: Option<FrameTransitionTrace>` to `DecisionOutcome::ActiveAction` and `DecisionOutcome::Planning`.

### 3. Record events at lifecycle points

At each frame lifecycle point (in `agent_tick/frame.rs` and `agent_tick/mod.rs`), if tracing is enabled, push the corresponding `FrameTransitionKind` to the trace.

### 4. Update `dump_agent()` and `summary()`

Include frame lifecycle events in the human-readable trace output.

## Files to Touch

- `crates/worldwake-ai/src/decision_trace.rs` (modify — add `FrameTransitionKind`, `FrameTransitionTrace`, extend `DecisionOutcome`)
- `crates/worldwake-ai/src/agent_tick/frame.rs` (modify — emit trace events at lifecycle points)
- `crates/worldwake-ai/src/agent_tick/mod.rs` (modify — pass trace sink to frame lifecycle functions)
- `crates/worldwake-ai/src/agent_tick/execution.rs` (modify — emit trace on frame persist)
- `crates/worldwake-ai/src/agent_tick/planning.rs` (modify — emit trace on frame creation during plan adoption)

## Out of Scope

- Action trace integration (already exists separately in `ActionTraceSink`)
- Changes to `ActionTraceEvent` or `ActionTraceKind`
- Golden test assertions on trace data (unless a golden test already uses `enable_tracing()`)
- Trace data serialization/persistence — traces are ephemeral in-memory data

## Acceptance Criteria

### Tests That Must Pass

1. Focused test: frame creation with tracing enabled → `FrameTransitionKind::Created` recorded
2. Focused test: progress event → `FrameTransitionKind::Progressed` recorded
3. Focused test: suspension → `FrameTransitionKind::Suspended` with correct reason
4. Focused test: exhaustion → `FrameTransitionKind::Exhausted` with correct counters
5. Focused test: clearing → `FrameTransitionKind::Cleared` with correct reason
6. `cargo test -p worldwake-ai` — all golden tests pass
7. `cargo clippy --workspace` — no new warnings

### Invariants

1. Tracing is opt-in and zero-cost when disabled (no allocation, no computation)
2. Frame transition events are recorded in chronological order within a tick
3. `dump_agent()` output includes frame lifecycle events for any agent with an IntentionFrame
4. Trace data does not affect agent behavior (pure observation, no side effects)

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/decision_trace.rs` (test module) — trace event recording for each `FrameTransitionKind`
2. `crates/worldwake-ai/src/agent_tick/tests.rs` — integration: enable tracing, run travel scenario, verify frame events in trace sink

### Commands

1. `cargo test -p worldwake-ai`
2. `cargo clippy --workspace`
3. `cargo test --workspace`
