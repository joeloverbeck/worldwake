# ACTEXETRA-001: Create `action_trace.rs` module with types and sink

**Status**: PENDING
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — new module in worldwake-sim
**Deps**: None

## Problem

There is no structured way to observe action lifecycle events (started, committed, aborted, start-failed) during `step_tick()`. This closes the Principle 27 debuggability gap for the causal path. This ticket creates the foundational types and the append-only sink, following the proven `DecisionTraceSink` pattern from `worldwake-ai`.

## Assumption Reassessment (2026-03-17)

1. `CommitOutcome` exists at `crates/worldwake-sim/src/action_handler.rs:10` with `#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]` — confirmed.
2. `CommitOutcome::empty()` exists as a constructor — confirmed at line 16.
3. `ActionInstanceId` is `pub use`d from `action_ids` — confirmed in `lib.rs:61`.
4. `ActionDefId`, `EntityId`, `Tick` come from `worldwake_core` — confirmed in `tick_step.rs:10-12`.

## Architecture Check

1. Follows the exact same pattern as `DecisionTraceSink` in `crates/worldwake-ai/src/decision_trace.rs` — append-only collector with query methods and dump output.
2. No backwards-compatibility shims. This is a new module with no existing callers.

## What to Change

### 1. Create `crates/worldwake-sim/src/action_trace.rs`

Define the following types per the spec:

- `ActionTraceEvent` struct — fields: `tick: Tick`, `actor: EntityId`, `def_id: ActionDefId`, `action_name: String`, `kind: ActionTraceKind`
- `ActionTraceKind` enum — variants: `Started { targets: Vec<EntityId> }`, `Committed { instance_id: ActionInstanceId, outcome: CommitOutcome }`, `Aborted { instance_id: ActionInstanceId, reason: String }`, `StartFailed { reason: String }`
- `ActionTraceEvent::summary()` method — one-line human-readable string per variant
- `ActionTraceSink` struct — `events: Vec<ActionTraceEvent>` with methods: `new()`, `record()`, `events()`, `events_for(actor)`, `events_at(tick)`, `events_for_at(actor, tick)`, `last_committed(actor)`, `clear()`, `dump_agent(actor)`
- `Default` impl for `ActionTraceSink`
- Unit tests: `sink_starts_empty`, `record_and_query_by_actor`, `query_by_tick`, `last_committed_returns_most_recent`, `summary_format_covers_all_variants`, `clear_removes_all_events`

All derives: `Clone, Debug, Eq, PartialEq` on `ActionTraceEvent` and `ActionTraceKind`.

### 2. Register module in `crates/worldwake-sim/src/lib.rs`

- Add `pub mod action_trace;` (alphabetical placement after `action_status`)
- Add re-exports: `pub use action_trace::{ActionTraceSink, ActionTraceEvent, ActionTraceKind};`

## Files to Touch

- `crates/worldwake-sim/src/action_trace.rs` (new)
- `crates/worldwake-sim/src/lib.rs` (modify — module declaration + re-exports)

## Out of Scope

- Threading the sink through `TickStepServices` / `TickStepRuntime` (ACTEXETRA-002)
- Recording trace events at lifecycle hook points (ACTEXETRA-003)
- `GoldenHarness` integration (ACTEXETRA-004)
- Documentation updates (ACTEXETRA-005)
- Any changes to `tick_step.rs`
- Any changes to test harnesses or golden tests

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-sim action_trace::tests::sink_starts_empty` — sink initializes with no events
2. `cargo test -p worldwake-sim action_trace::tests::record_and_query_by_actor` — events filtered by actor
3. `cargo test -p worldwake-sim action_trace::tests::query_by_tick` — events filtered by tick
4. `cargo test -p worldwake-sim action_trace::tests::last_committed_returns_most_recent` — reverse scan for committed
5. `cargo test -p worldwake-sim action_trace::tests::summary_format_covers_all_variants` — all 4 kind variants produce summaries
6. `cargo test -p worldwake-sim action_trace::tests::clear_removes_all_events` — clear empties the vec
7. Existing suite: `cargo test --workspace` (no regressions)

### Invariants

1. `ActionTraceSink` is append-only (no mutation of existing events, only `record()` and `clear()`)
2. Zero external dependencies added — only uses types from `worldwake-core` and `worldwake-sim`
3. No `Serialize`/`Deserialize` on trace types — traces are ephemeral debugging aids, not persisted state

## Test Plan

### New/Modified Tests

1. `crates/worldwake-sim/src/action_trace.rs` (inline `#[cfg(test)]` module) — 6 unit tests covering sink lifecycle, query methods, and summary formatting

### Commands

1. `cargo test -p worldwake-sim action_trace`
2. `cargo clippy -p worldwake-sim`
3. `cargo test --workspace`
