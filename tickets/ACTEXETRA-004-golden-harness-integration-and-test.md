# ACTEXETRA-004: Integrate into `GoldenHarness` and write golden test

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: None — test infrastructure only
**Deps**: ACTEXETRA-001, ACTEXETRA-002, ACTEXETRA-003

## Problem

The trace system is complete but has no end-to-end proof. This ticket wires `ActionTraceSink` into `GoldenHarness` with enable/query methods, then writes a golden test that exercises the full trace pipeline by verifying loot action lifecycle events.

## Assumption Reassessment (2026-03-17)

1. `GoldenHarness` struct is at `crates/worldwake-ai/tests/golden_harness/mod.rs`. It already has fields for `world`, `event_log`, `scheduler`, `controller`, `rng`, `defs`, `handlers`, `recipes`, `driver`. Confirmed.
2. `GoldenHarness::step_once()` constructs `TickStepServices` — after ACTEXETRA-002, it passes `action_trace: None`. This ticket changes it to `self.action_trace.as_mut()`.
3. `build_multi_corpse_loot_binding_scenario()` exists in `golden_combat.rs` and returns `(GoldenHarness, corpse_a, corpse_b, looter, ...)`. Must verify exact return signature.
4. `GoldenHarness` has constructors `new()`, `with_recipes()`, `from_simulation_state()` — all must initialize `action_trace: None`.

## Architecture Check

1. Follows `DecisionTraceSink` integration pattern — `enable_*()` method, `*_sink()` query method, `Option` field on harness.
2. No backwards-compatibility issues — existing tests don't call `enable_action_tracing()`, so they get `None` (zero-cost).

## What to Change

### 1. Add `action_trace` field to `GoldenHarness` struct

```rust
pub action_trace: Option<ActionTraceSink>,
```

Initialize as `None` in all constructors (`new()`, `with_recipes()`, `from_simulation_state()`).

### 2. Add enable/query methods

```rust
pub fn enable_action_tracing(&mut self) {
    self.action_trace = Some(ActionTraceSink::new());
}

pub fn action_trace_sink(&self) -> Option<&ActionTraceSink> {
    self.action_trace.as_ref()
}
```

### 3. Thread through `step_once()`

Change `action_trace: None` (from ACTEXETRA-002) to `action_trace: self.action_trace.as_mut()` in the `TickStepServices` construction.

### 4. Write golden test `golden_action_trace_records_loot_lifecycle`

In `crates/worldwake-ai/tests/golden_combat.rs`, add a test that:
- Uses `build_multi_corpse_loot_binding_scenario()` with a fixed seed
- Enables action tracing via `h.enable_action_tracing()`
- Runs 10 ticks
- Asserts the looter has >= 2 `Started` events
- Asserts the looter has >= 2 `Committed` events
- Asserts every `Started` has a matching `Committed` at same or later tick
- Asserts loot actions commit in the same tick they start (1-tick action validation)

## Files to Touch

- `crates/worldwake-ai/tests/golden_harness/mod.rs` (modify — field, constructors, enable/query methods, `step_once` threading)
- `crates/worldwake-ai/tests/golden_combat.rs` (modify — add `golden_action_trace_records_loot_lifecycle` test)

## Out of Scope

- The `action_trace.rs` module (ACTEXETRA-001)
- `TickStepServices`/`TickStepRuntime` changes (ACTEXETRA-002)
- Hook point recording in `tick_step.rs` (ACTEXETRA-003)
- Documentation (ACTEXETRA-005)
- Adding action tracing to any other test file — only `golden_combat.rs`
- Modifying any non-test code

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-ai --test golden_combat golden_action_trace_records_loot_lifecycle` — new test passes
2. `cargo test -p worldwake-ai --test golden_combat` — all existing golden combat tests still pass (no regressions)
3. `cargo test --workspace` — full workspace passes

### Invariants

1. Existing golden tests are unaffected — they don't call `enable_action_tracing()`, so trace is `None` (zero-cost)
2. The new test proves the full pipeline: sink creation → threading → recording → querying
3. Loot actions complete in the same tick they start (1-tick action invariant)
4. Every `Started` event has a corresponding `Committed` event (no orphaned starts)

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_combat.rs::golden_action_trace_records_loot_lifecycle` — end-to-end trace verification using multi-corpse loot scenario

### Commands

1. `cargo test -p worldwake-ai --test golden_combat golden_action_trace_records_loot_lifecycle`
2. `cargo test -p worldwake-ai --test golden_combat`
3. `cargo test --workspace`
4. `cargo clippy --workspace`
