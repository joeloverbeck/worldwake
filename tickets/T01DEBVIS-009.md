# T01DEBVIS-009: Per-agent trace ring buffers + Traces tab

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: None
**Deps**: [T01DEBVIS-004](../archive/tickets/T01DEBVIS-004.md), [T01DEBVIS-007](../archive/tickets/T01DEBVIS-007.md), [T01DEBVIS-008](../archive/tickets/T01DEBVIS-008.md)

## Problem

Spec T01 §D8 mandates per-agent ring buffers for `AgentDecisionTrace` and `ActionTraceEvent`, capped at 50 entries each. Sinks owned by `VisualizerApp` are borrowed into `TickStepServices` per tick (T01DEBVIS-004 already holds the sinks); after each `step_tick`, drained sink contents must route into the per-agent buffers. The Traces tab (§D7.6) renders the last 50 entries for the selected agent, two columns (Decision | Action), newest first. This ticket also completes archived T01DEBVIS-008's deferred Plan-tab "last replan reason" query that reads the same decision buffer.

## Assumption Reassessment (2026-04-25)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. `AgentDecisionTrace` at `crates/worldwake-ai/src/decision_trace.rs:89` exposes `agent: EntityId`, `tick: Tick`, `outcome: DecisionOutcome`. `DecisionTraceSink` at `crates/worldwake-ai/src/decision_trace.rs:1240` collects traces; `DecisionTraceSink::new()` and `Default::default()` are public constructors.
2. `ActionTraceEvent` at `crates/worldwake-sim/src/action_trace.rs:20` carries the per-action lifecycle event surface; `ActionTraceSink` at `action_trace.rs:468` collects events. Both sinks are passed by `&mut` reference into `TickStepServices` per tick (already wired in T01DEBVIS-004).
3. The drain pattern: after `step_tick` returns, iterate the sink's collected entries, partition by `agent`/`actor`, and push into per-agent `VecDeque`s with capacity `50`. Sinks expose internal `Vec<...>` accessors per their existing test patterns; confirm the exact accessor names during implementation (drain pattern may vary between `take_traces()`-style and `iter()`-style depending on each sink's API).
4. Tooling-only ticket — buffers are `VisualizerApp`-local; the simulation's authoritative event log is unaffected.

## Architecture Check

1. Per-agent ring buffers live on `VisualizerApp`, never on the engine — they are caches per FND-27. Capacity 50 is a UI tuning constant per spec, not a load-bearing simulation parameter.
2. Traces tab queries the same buffers it populates — single source of truth for the visualizer's debug history.
3. Plan tab's "last replan reason" query (archived T01DEBVIS-008) reuses the decision buffer rather than carrying a parallel store. This keeps the trace history mechanism single-implementation per FND-29 (debuggability).

## Verification Layers

1. Buffer cap correctness → focused unit test (`trace_buffers_capped_at_50`) pushing 100 entries for one agent, asserting `len == 50` and oldest entries dropped.
2. Sink-to-buffer routing correctness → focused integration test loading `survival-baseline.ron`, advancing N ticks, asserting that each agent's decision and action buffers contain entries whose `agent`/`actor` matches and whose `tick` is within the advanced range.
3. Per template item 6: this ticket spans (a) decision-trace surface (`AgentDecisionTrace` / `DecisionTraceSink`) and (b) action-trace surface (`ActionTraceEvent` / `ActionTraceSink`) — both layers are exercised by the integration test, with separate assertions per layer (no surface collapsing).

## What to Change

### 1. Implement `trace_buffers.rs`

Create `crates/worldwake-visualizer/src/trace_buffers.rs`:

```rust
pub struct AgentTraceBuffers {
    decisions: BTreeMap<EntityId, VecDeque<AgentDecisionTrace>>,
    actions: BTreeMap<EntityId, VecDeque<ActionTraceEvent>>,
    capacity: usize, // 50
}

impl AgentTraceBuffers {
    pub fn new(capacity: usize) -> Self { … }
    pub fn record_decision(&mut self, trace: AgentDecisionTrace) { … }
    pub fn record_action(&mut self, event: ActionTraceEvent) { … }
    pub fn decisions_for(&self, agent: EntityId) -> impl Iterator<Item = &AgentDecisionTrace> { … }
    pub fn actions_for(&self, agent: EntityId) -> impl Iterator<Item = &ActionTraceEvent> { … }
    pub fn last_replan_reason(&self, agent: EntityId) -> Option<&AgentDecisionTrace> { … }
}
```

`record_*` enforces the 50-entry cap by popping the front when the deque is full.

### 2. Wire sink drain into `app.rs::step_one_tick`

Modify `crates/worldwake-visualizer/src/app.rs` from T01DEBVIS-004 — after `step_tick` returns, drain `self.action_trace` and `self.decision_trace` into `self.trace_buffers` (a new field on `VisualizerApp`). Use the sinks' existing iteration/take APIs (confirm exact surface during implementation).

Add `trace_buffers: AgentTraceBuffers` field to `VisualizerApp`; default-construct with capacity 50.

### 3. Implement `tabs/traces.rs`

Create `crates/worldwake-visualizer/src/tabs/traces.rs`:

- Two-column layout (Decision | Action).
- Each column iterates `trace_buffers.decisions_for(agent)` / `actions_for(agent)` newest-first.
- Decision rows show `tick`, summarized `outcome` (e.g., `"Plan: walk to market"` / `"Replan: BeliefUpdate"`); details collapsible.
- Action rows show `tick`, summarized event kind, target.

### 4. Wire Plan tab's "last replan reason" hook (archived T01DEBVIS-008 dependency)

If archived T01DEBVIS-008's Plan tab placeholder still says `"no replan recorded"`, replace it now with a call to `trace_buffers.last_replan_reason(agent)` — this completes the `ReplanReason`/`PlanInvalidationReason` surface promised by spec §D7.5.

### 5. Wire modules into lib.rs and tabs router

Add `pub mod trace_buffers;` to `crates/worldwake-visualizer/src/lib.rs`. Replace the `DetailTab::Traces` placeholder dispatch in `tabs/mod.rs` with the actual call.

## Files to Touch

- `crates/worldwake-visualizer/src/trace_buffers.rs` (new)
- `crates/worldwake-visualizer/src/tabs/traces.rs` (new)
- `crates/worldwake-visualizer/src/app.rs` (modify — add `trace_buffers` field; drain sinks after `step_tick`)
- `crates/worldwake-visualizer/src/tabs/mod.rs` (modify — register Traces tab)
- `crates/worldwake-visualizer/src/tabs/plan.rs` (modify — replace `"no replan recorded"` placeholder with `last_replan_reason` query)
- `crates/worldwake-visualizer/src/lib.rs` (modify — add `trace_buffers` module declaration)

## Out of Scope

- Cross-agent trace correlation.
- Search/filter UI within the Traces tab.
- Persistence of trace buffers across scenario reloads (reset clears buffers per spec — implicit in `VisualizerApp::reset` from T01DEBVIS-004).
- Manual QA documentation (T01DEBVIS-010).

## Acceptance Criteria

### Tests That Must Pass

1. `trace_buffers_capped_at_50` — push 100 `AgentDecisionTrace` entries for one agent; assert `decisions_for(agent).count() == 50` and the oldest entries are dropped (newest tick is preserved).
2. `traces_populated_after_steps` — load `survival-baseline.ron`, advance N ticks, assert at least one agent's `decisions_for` and `actions_for` iterators are non-empty after the run.
3. `last_replan_reason_returns_most_recent` — push three decision traces with `outcome` encoding replan reasons across different ticks; assert `last_replan_reason(agent)` returns the entry with the highest tick.
4. Existing suite: `cargo test -p worldwake-visualizer` passes.

### Invariants

1. Per-agent buffers are capped at 50 entries; the cap is enforced on every `record_*` call.
2. Buffers are visualizer-local; the simulation's append-only event log is never modified by the trace-routing pipeline.
3. The Plan tab's "last replan reason" surface and the Traces tab read from the same `AgentTraceBuffers` instance — no parallel buffer.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-visualizer/src/trace_buffers.rs` (`#[cfg(test)] mod tests`) — `trace_buffers_capped_at_50`, `last_replan_reason_returns_most_recent`.
2. `crates/worldwake-visualizer/src/app.rs` (`#[cfg(test)] mod tests`) — `traces_populated_after_steps` (integration with scenario load + step).

### Commands

1. `cargo test -p worldwake-visualizer trace_buffers::`
2. `cargo test -p worldwake-visualizer tabs::traces::`
3. `cargo test -p worldwake-visualizer`
4. `cargo run -p worldwake-visualizer -- scenarios/survival-baseline.ron` (manual click + Traces tab smoke)
5. `./scripts/verify.sh`
