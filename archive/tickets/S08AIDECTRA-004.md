# S08AIDECTRA-004: GoldenHarness Trace Integration and dump_agent

**Status**: ✅ COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: None — test infrastructure only
**Deps**: S08AIDECTRA-001, S08AIDECTRA-002

## Problem

The trace data model exists and the pipeline populates it, but golden tests have no way to enable tracing or query traces. This ticket exposes trace capabilities through the golden test harness and adds a `dump_agent` diagnostic method for interactive debugging.

## Assumption Reassessment (2026-03-16)

1. There is no `GoldenHarness` struct. The golden harness is a module of helper functions in `crates/worldwake-ai/tests/golden_harness/mod.rs`. Confirmed.
2. Golden tests create an `AgentTickDriver` directly, then use it as an `AutonomousController` via `tick_input_producer` / `TickInputContext`. The driver is owned by the test. Confirmed.
3. After S08AIDECTRA-002, `AgentTickDriver` will have `enable_tracing()` and `trace_sink()` methods. The harness just needs to expose these to test code.
4. `ActionDefRegistry` is available in golden tests (built via `build_full_registries`). It can be passed to `dump_agent` for action name resolution. Confirmed.

## Architecture Check

1. Since there's no `GoldenHarness` struct, the integration is simpler: golden tests call `driver.enable_tracing()` directly on their `AgentTickDriver` before stepping, and query via `driver.trace_sink()` after stepping.
2. The main deliverable is: (a) a convenience helper function in `golden_harness/mod.rs` if useful, (b) the `dump_agent` method on `DecisionTraceSink`, and (c) a documented pattern for trace-enabled golden tests.
3. `dump_agent` is a display-only method — it writes to stderr and has no return value. It resolves action def IDs to names via the `ActionDefRegistry`.

## What to Change

### 1. Add `dump_agent` to `DecisionTraceSink`

In `crates/worldwake-ai/src/decision_trace.rs`, add:

```rust
impl DecisionTraceSink {
    pub fn dump_agent(&self, agent: EntityId, action_defs: &worldwake_sim::ActionDefRegistry) {
        for trace in self.traces_for(agent) {
            eprintln!("[tick {}] {}", trace.tick.0, format_outcome(&trace.outcome, action_defs));
        }
    }
}
```

Add a private `format_outcome` helper that produces a human-readable one-line summary per trace:
- `Dead` → `"DEAD — no decision"`
- `ActiveAction { .. }` → `"ACTIVE: {action_name} — interrupt: {decision:?}"`
- `Planning { .. }` → `"PLAN: selected={goal:?}, candidates={n}, plans_found={m}"`

### 2. Add `summary()` method on `DecisionOutcome`

A short `fn summary(&self) -> String` that returns a one-liner (no action name resolution needed — just uses stored `action_name` strings).

### 3. Add golden harness convenience (optional)

If a wrapper simplifies repeated patterns, add a helper to `golden_harness/mod.rs`:

```rust
pub fn assert_candidate_generated(
    sink: &DecisionTraceSink,
    agent: EntityId,
    tick: Tick,
    goal_kind: GoalKind,
) -> bool { ... }
```

This is optional — if the raw `trace_at` API is sufficient, skip it.

### 4. Add a trace-enabled example golden test

Add a small test in `crates/worldwake-ai/tests/golden_ai_decisions.rs` (or a new file) that:
1. Creates a driver with `enable_tracing()`
2. Runs a simple 5-tick scenario
3. Queries traces and asserts basic properties
4. Calls `dump_agent` (verifies it doesn't panic)

This test validates the end-to-end flow: enable → step → query.

## Files to Touch

- `crates/worldwake-ai/src/decision_trace.rs` (modify — add `dump_agent`, `summary`, `format_outcome`)
- `crates/worldwake-ai/tests/golden_harness/mod.rs` (modify — optional convenience helpers)
- `crates/worldwake-ai/tests/golden_ai_decisions.rs` (modify — add trace-enabled example test)

## Out of Scope

- Changes to `worldwake-core` or `worldwake-sim`
- S02c golden e2e test (S08AIDECTRA-005)
- CLI/UI integration of traces
- Serialization of traces
- Any changes to the decision pipeline itself (S08AIDECTRA-002)
- Trace filtering or aggregation APIs beyond what the spec defines

## Acceptance Criteria

### Tests That Must Pass

1. New golden test: trace-enabled 5-tick scenario asserts `trace_sink().traces().len() > 0` and at least one `DecisionOutcome::Planning` with non-empty candidates.
2. `dump_agent` test: call `dump_agent` on a populated sink — must not panic (output goes to stderr, not asserted).
3. `summary()` test: verify `DecisionOutcome::Dead.summary()` returns a non-empty string.
4. Existing suite: `cargo test -p worldwake-ai` — all existing golden tests pass unchanged.
5. `cargo clippy --workspace` — no new warnings.

### Invariants

1. Tracing is opt-in: existing golden tests that do NOT call `enable_tracing()` are completely unaffected.
2. `dump_agent` is a diagnostic tool — it must never panic regardless of trace contents.
3. No new dependencies added (the `ActionDefRegistry` import already exists in test scope).

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_ai_decisions.rs` — trace-enabled golden test
2. `crates/worldwake-ai/src/decision_trace.rs` — `summary()` unit test

### Commands

1. `cargo test -p worldwake-ai golden_ai_decisions`
2. `cargo test -p worldwake-ai decision_trace`
3. `cargo test --workspace`
4. `cargo clippy --workspace`

## Outcome

- **Completion date**: 2026-03-16
- **What changed**:
  - Added `DecisionOutcome::summary()` for one-line human-readable outcome strings.
  - Added `DecisionTraceSink::dump_agent()` with `format_outcome()` helper for registry-resolved stderr output.
  - Added 3 unit tests for `summary()` variants (Dead, ActiveAction, Planning).
  - Added `golden_trace_enabled_scenario` golden test: 5-tick trace-enabled end-to-end validation.
  - Added "Debugging AI Decisions with Decision Traces" sections to both `CLAUDE.md` and `AGENTS.md`.
- **Deviations from ticket**: Ticket assumption #1 incorrectly stated "no GoldenHarness struct" — there is one, but since `driver` is a public field, the direct-call approach (`h.driver.enable_tracing()`) works as designed. The optional `assert_candidate_generated` helper was skipped since the raw `trace_at` API is sufficient. `Planning` variant uses `Box<PlanningPipelineTrace>` (actual code) not unboxed as shown in ticket examples.
- **Verification**: `cargo test --workspace` — 1762 tests pass, 0 failures. `cargo clippy --workspace` — clean.
