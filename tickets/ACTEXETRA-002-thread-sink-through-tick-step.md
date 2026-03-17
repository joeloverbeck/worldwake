# ACTEXETRA-002: Thread trace sink through `TickStepServices` and `TickStepRuntime`

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — API change to `TickStepServices`
**Deps**: ACTEXETRA-001

## Problem

The `ActionTraceSink` created in ACTEXETRA-001 has no way to reach the lifecycle hook points inside `step_tick()`. The sink must be threaded through the public `TickStepServices` struct (caller-facing) and the internal `TickStepRuntime` struct (used by all internal functions), plus a helper method for zero-cost recording.

## Assumption Reassessment (2026-03-17)

1. `TickStepServices` is at `crates/worldwake-sim/src/tick_step.rs:15` with 5 fields — confirmed.
2. `TickStepRuntime` is at `crates/worldwake-sim/src/tick_step.rs:23` with 4 fields — confirmed.
3. All callers constructing `TickStepServices` (found via grep for `TickStepServices {`):
   - `crates/worldwake-ai/tests/golden_harness/mod.rs` — GoldenHarness::step_once
   - `crates/worldwake-ai/src/agent_tick.rs` — AgentTickDriver
   - `crates/worldwake-sim/src/save_load.rs`
   - `crates/worldwake-sim/src/replay_execution.rs`
   - `crates/worldwake-systems/tests/e09_needs_integration.rs`
   - `crates/worldwake-systems/tests/e10_production_transport_integration.rs`
   - `crates/worldwake-systems/tests/e12_combat_integration.rs`
   - `crates/worldwake-systems/tests/e15_information_integration.rs`
   - `crates/worldwake-cli/src/handlers/tick.rs`
4. All callers must add `action_trace: None` to compile.

## Architecture Check

1. Uses `Option<&'a mut ActionTraceSink>` — zero-cost when `None`. The `take()` pattern mirrors `input_producer`.
2. No backwards-compatibility shim needed — this is a struct field addition. All callers must be updated in this ticket to compile.

## What to Change

### 1. Add `action_trace` field to `TickStepServices` (`tick_step.rs:15`)

```rust
pub action_trace: Option<&'a mut ActionTraceSink>,
```

### 2. Add `action_trace` field to `TickStepRuntime` (`tick_step.rs:23`)

```rust
action_trace: Option<&'a mut ActionTraceSink>,
```

### 3. Thread in `step_tick()` function

In the `step_tick()` function body, `take()` the sink from services and pass it into the runtime struct, exactly as `input_producer` is handled.

### 4. Add helper method on `TickStepRuntime`

```rust
fn record_action_trace(&mut self, event: ActionTraceEvent) {
    if let Some(sink) = self.action_trace.as_mut() {
        sink.record(event);
    }
}
```

### 5. Fix all callers — add `action_trace: None`

Every site that constructs `TickStepServices { ... }` must add `action_trace: None`. This is a mechanical change with no behavioral effect.

## Files to Touch

- `crates/worldwake-sim/src/tick_step.rs` (modify — struct fields, threading, helper method)
- `crates/worldwake-ai/tests/golden_harness/mod.rs` (modify — add `action_trace: None`)
- `crates/worldwake-ai/src/agent_tick.rs` (modify — add `action_trace: None`)
- `crates/worldwake-sim/src/save_load.rs` (modify — add `action_trace: None`)
- `crates/worldwake-sim/src/replay_execution.rs` (modify — add `action_trace: None`)
- `crates/worldwake-systems/tests/e09_needs_integration.rs` (modify — add `action_trace: None`)
- `crates/worldwake-systems/tests/e10_production_transport_integration.rs` (modify — add `action_trace: None`)
- `crates/worldwake-systems/tests/e12_combat_integration.rs` (modify — add `action_trace: None`)
- `crates/worldwake-systems/tests/e15_information_integration.rs` (modify — add `action_trace: None`)
- `crates/worldwake-cli/src/handlers/tick.rs` (modify — add `action_trace: None`)

## Out of Scope

- Actually recording trace events at hook points (ACTEXETRA-003)
- `GoldenHarness` enable/query methods (ACTEXETRA-004)
- The `action_trace.rs` module itself (ACTEXETRA-001)
- Documentation changes (ACTEXETRA-005)
- Adding `action_trace: Some(...)` to any caller — all callers pass `None` in this ticket

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test --workspace` — all existing tests pass with zero behavioral change (trace is `None` everywhere)
2. `cargo clippy --workspace` — no warnings

### Invariants

1. `TickStepServices` API is extended, not broken — all callers compile with `action_trace: None`
2. `record_action_trace()` is a no-op when `action_trace` is `None` — zero-cost when disabled
3. No new test files created — this is purely structural threading

## Test Plan

### New/Modified Tests

1. No new tests — this ticket is structural. Behavioral verification comes in ACTEXETRA-003 and ACTEXETRA-004.

### Commands

1. `cargo build --workspace`
2. `cargo test --workspace`
3. `cargo clippy --workspace`
