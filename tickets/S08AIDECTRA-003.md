# S08AIDECTRA-003: BestEffort Action Start Failure Recording in worldwake-sim

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: Yes — adds field to Scheduler in worldwake-sim
**Deps**: None (independent of S08AIDECTRA-001/002, but logically part of S08)

## Problem

`tick_step.rs` silently skips `BestEffort` action starts that fail (reservation unavailable, precondition failed, invalid target). When an agent's planned action is rejected at execution time, no record exists of the attempt or the failure reason. This makes Failure 5 from the spec ("Why did the action not start?") undiagnosable.

This ticket adds a lightweight `ActionStartFailure` record to the `Scheduler` that is populated when a BestEffort start fails, and drained each tick. The AI layer (S08AIDECTRA-002) can then incorporate this into the next tick's trace.

## Assumption Reassessment (2026-03-16)

1. `Scheduler` is in `crates/worldwake-sim/src/scheduler.rs:28-37`. It has fields: `current_tick`, `active_actions`, `system_manifest`, `input_queue`, `pending_replans`, `committed_actions`, `next_instance_id`. No existing failure tracking. Confirmed.
2. The BestEffort silent skip is in `tick_step.rs:232-236`. It matches on `ActionError::ReservationUnavailable | PreconditionFailed | InvalidTarget` and returns `Ok(InputOutcome::default())`. Confirmed.
3. `ActionError` variants contain descriptive strings. The `reason: String` field on `ActionStartFailure` will capture the `Display` output of the error. Confirmed.
4. `ActionDefId` is available in the `tick_step.rs` context (it's on the `InputKind::RequestAction` payload). Confirmed.

## Architecture Check

1. This is a minimal, backward-compatible addition: one new struct, one new `Vec` field on `Scheduler`, one drain method. No existing behavior changes.
2. The failure vec is drained per tick (not accumulated indefinitely), keeping memory bounded.
3. Using `String` for the reason (rather than storing the `ActionError` enum) keeps the dependency boundary clean — `ActionStartFailure` doesn't need to re-export `ActionError` variants.

## What to Change

### 1. New struct: `ActionStartFailure`

Add to `crates/worldwake-sim/src/scheduler.rs` (or a small new file if preferred):

```rust
#[derive(Clone, Debug)]
pub struct ActionStartFailure {
    pub tick: Tick,
    pub actor: EntityId,
    pub def_id: ActionDefId,
    pub reason: String,
}
```

### 2. Add failure vec to `Scheduler`

Add `action_start_failures: Vec<ActionStartFailure>` field to `Scheduler`. Initialize as empty in constructor.

### 3. Record failure in `tick_step.rs`

In the BestEffort early-return path (line ~232-236), before returning `Ok(InputOutcome::default())`, push an `ActionStartFailure` to the scheduler's vec with the tick, actor, action def id, and `err.to_string()` as the reason.

### 4. Drain API on `Scheduler`

Add:
- `pub fn drain_action_start_failures(&mut self) -> Vec<ActionStartFailure>` — takes all failures, leaving vec empty
- `pub fn action_start_failures(&self) -> &[ActionStartFailure]` — read access

### 5. Export from `lib.rs`

Re-export `ActionStartFailure` from `worldwake-sim/src/lib.rs`.

## Files to Touch

- `crates/worldwake-sim/src/scheduler.rs` (modify — add struct, field, methods)
- `crates/worldwake-sim/src/tick_step.rs` (modify — record failure in BestEffort path)
- `crates/worldwake-sim/src/lib.rs` (modify — re-export)

## Out of Scope

- Changes to `worldwake-core` — this is a sim-layer concern
- Changes to `worldwake-ai` — consumption of failures is S08AIDECTRA-002's job
- Changing BestEffort behavior — failures are still silently skipped from the action framework's perspective; we only record that they happened
- Persisting failures across save/load — failures are ephemeral per-tick data
- Any changes to non-BestEffort error handling

## Acceptance Criteria

### Tests That Must Pass

1. Unit test `scheduler::tests::action_start_failure_drain` — push 2 failures, drain returns both, subsequent drain returns empty.
2. Unit test `scheduler::tests::action_start_failure_read` — push failures, read via `action_start_failures()`, verify contents.
3. Integration test: Set up a scenario where a BestEffort action start fails (e.g., precondition not met), step one tick, verify `scheduler.action_start_failures()` contains one entry with correct actor, def_id, and non-empty reason.
4. Existing suite: `cargo test -p worldwake-sim` — no regressions.
5. `cargo clippy --workspace` — no new warnings.

### Invariants

1. **No behavioral change**: BestEffort failures are still silently skipped. The only addition is recording the failure for later observation.
2. **Bounded memory**: Failures are drained each tick. The vec never grows unboundedly.
3. **Backward-compatible Scheduler construction**: Existing code that creates a `Scheduler` must still compile (the new field has a sensible default: empty vec).
4. **Determinism preserved**: `ActionStartFailure` is diagnostic data, not consumed by the simulation. It does not affect `canonical_bytes` or `hash_world`.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-sim/src/scheduler.rs` (inline tests) — drain/read operations
2. `crates/worldwake-sim/tests/` or inline — integration test for BestEffort failure recording

### Commands

1. `cargo test -p worldwake-sim`
2. `cargo test --workspace`
3. `cargo clippy --workspace`
