# S08AIDECTRA-001: Trace Data Model and DecisionTraceSink

**Status**: ✅ COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — new module in worldwake-ai
**Deps**: None (foundational ticket for S08)

## Problem

The AI decision pipeline discards all reasoning context after each tick. Debugging emergent behavior requires hours of ad-hoc `eprintln` instrumentation. This ticket introduces the structured trace data model and the collection/query sink — the foundation all subsequent tickets build on.

## Assumption Reassessment (2026-03-16)

1. `GoalKey` is defined in `worldwake-core/src/goal.rs` and re-exported by worldwake-ai. Confirmed.
2. `GoalPriorityClass` is in `worldwake-ai/src/goal_model.rs` as a public enum. Confirmed.
3. `GoalSwitchKind` is `pub(crate)` in `goal_switching.rs` and NOT exported in `lib.rs`. The trace model references it in `GoalSwitchSummary` — it must be made `pub` and re-exported. This is a required visibility change.
4. `InterruptDecision` and `InterruptTrigger` are public in `interrupts.rs` and re-exported. Confirmed.
5. `PlannerOpKind` is public in `planner_ops.rs` and re-exported. Confirmed.
6. `ActionDefId` is in `worldwake-core/src/ids.rs`. Confirmed.
7. `EntityId` and `Tick` are in `worldwake-core/src/ids.rs`. Confirmed.
8. `PlanTerminalKind` is public in `planner_ops.rs`. Confirmed.

## Architecture Check

1. All trace types are defined in a single new module (`decision_trace.rs`) rather than scattered across pipeline modules. This keeps trace concerns isolated and makes the data model easy to review as a unit.
2. `DecisionTraceSink` is a simple `Vec`-backed append-only store with query helpers — no complex indexing, no derived state stored. Query methods (`traces_for`, `trace_at`) compute on the fly.
3. The only cross-module change is making `GoalSwitchKind` public — minimal blast radius.

## What to Change

### 1. New module: `crates/worldwake-ai/src/decision_trace.rs`

Define all trace structs and enums from the spec's "Trace Data Model" section:
- `AgentDecisionTrace` (top-level: agent, tick, outcome)
- `DecisionOutcome` enum (Dead, ActiveAction, Planning)
- `PlanningPipelineTrace` (dirty_reasons, candidates, planning, selection, execution)
- `CandidateTrace` (generated, ranked, suppressed, zero_motive)
- `RankedGoalSummary` (goal, priority_class, motive_score)
- `PlanSearchTrace` (attempts vec)
- `PlanAttemptTrace` (goal, outcome)
- `PlanSearchOutcome` enum (Found, BudgetExhausted, Unsupported, FrontierExhausted)
- `PlannedStepSummary` (action_def_id, action_name, op_kind, targets, estimated_ticks)
- `SelectionTrace` (selected, goal_switch, previous_goal)
- `GoalSwitchSummary` (from, to, kind)
- `ExecutionTrace` (enqueued_step, revalidation_passed, failure)
- `ExecutionFailureReason` enum
- `InterruptTrace` (decision, top_challenger)
- `DirtyReason` enum (NoPlan, PlanFinished, ReplanSignal, QueueTransition, BlockerCleanup, SnapshotChanged, QueuePatienceExhausted)

Define `DecisionTraceSink`:
- `new() -> Self`
- `record(&mut self, trace: AgentDecisionTrace)`
- `traces(&self) -> &[AgentDecisionTrace]`
- `traces_for(&self, agent: EntityId) -> Vec<&AgentDecisionTrace>`
- `trace_at(&self, agent: EntityId, tick: Tick) -> Option<&AgentDecisionTrace>`
- `clear(&mut self)`

All types derive `Clone, Debug`. No `Serialize`/`Deserialize` — traces are ephemeral.

### 2. Make `GoalSwitchKind` public

In `crates/worldwake-ai/src/goal_switching.rs`, change `pub(crate) enum GoalSwitchKind` to `pub enum GoalSwitchKind`.

### 3. Export from `lib.rs`

Add `pub mod decision_trace;` and re-export key types:
- `DecisionTraceSink`, `AgentDecisionTrace`, `DecisionOutcome`
- `PlanningPipelineTrace`, `CandidateTrace`, `RankedGoalSummary`
- `PlanSearchTrace`, `PlanAttemptTrace`, `PlanSearchOutcome`
- `PlannedStepSummary`, `SelectionTrace`, `GoalSwitchSummary`
- `ExecutionTrace`, `ExecutionFailureReason`
- `InterruptTrace`, `DirtyReason`

Also add `GoalSwitchKind` to the `goal_switching` re-exports.

## Files to Touch

- `crates/worldwake-ai/src/decision_trace.rs` (new)
- `crates/worldwake-ai/src/goal_switching.rs` (modify — visibility only)
- `crates/worldwake-ai/src/lib.rs` (modify — add module + re-exports)

## Out of Scope

- Threading traces through the pipeline (S08AIDECTRA-002)
- BestEffort failure recording in worldwake-sim (S08AIDECTRA-003)
- GoldenHarness integration (S08AIDECTRA-004)
- S02c golden e2e test (S08AIDECTRA-005)
- Any changes to `worldwake-core` or `worldwake-sim`
- Serialization/persistence of traces
- CLI/UI integration
- `dump_agent()` display method (S08AIDECTRA-004)

## Acceptance Criteria

### Tests That Must Pass

1. Unit test `decision_trace::tests::sink_record_and_query` — record 3 traces for 2 agents across 2 ticks, verify `traces()` returns all 3, `traces_for(agent_a)` returns 2, `trace_at(agent_a, tick_1)` returns the correct one.
2. Unit test `decision_trace::tests::sink_clear` — record traces, call `clear()`, verify `traces()` is empty.
3. Unit test `decision_trace::tests::trace_at_missing` — query for a non-existent agent/tick returns `None`.
4. Existing suite: `cargo test -p worldwake-ai` — no regressions.
5. `cargo clippy --workspace` — no new warnings.

### Invariants

1. All trace types are `Clone + Debug` — required for test assertions and diagnostic output.
2. `DecisionTraceSink` stores no derived state — all query methods compute from `Vec<AgentDecisionTrace>`.
3. No new dependencies added to worldwake-ai's `Cargo.toml`.
4. `GoalSwitchKind` visibility change does not break any existing code (it was `pub(crate)`, making it `pub` is strictly less restrictive).

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/decision_trace.rs` (inline `#[cfg(test)] mod tests`) — sink CRUD operations

### Commands

1. `cargo test -p worldwake-ai decision_trace`
2. `cargo test --workspace`
3. `cargo clippy --workspace`

## Outcome

- **Completion date**: 2026-03-16
- **What changed**:
  - Created `crates/worldwake-ai/src/decision_trace.rs` with all 16 trace types and `DecisionTraceSink`
  - Changed `GoalSwitchKind` from `pub(crate)` to `pub` in `goal_switching.rs`
  - Added `pub mod decision_trace`, re-exported all trace types and `GoalSwitchKind` in `lib.rs`
- **Deviations**: `DecisionOutcome::Planning` wraps `Box<PlanningPipelineTrace>` instead of bare `PlanningPipelineTrace`, per clippy `large_enum_variant` lint (464 bytes vs 88 bytes for the next largest variant)
- **Verification**: 3 new unit tests pass, full `cargo test -p worldwake-ai` (308 tests) pass, `cargo clippy --workspace` clean, no new dependencies
