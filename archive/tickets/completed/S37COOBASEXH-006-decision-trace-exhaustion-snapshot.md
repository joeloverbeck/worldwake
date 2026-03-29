# S37COOBASEXH-006: Decision trace extension for cooldown state

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: Yes — decision_trace.rs new struct and field on PlanningPipelineTrace
**Deps**: S37COOBASEXH-002 (ExhaustionEntry has cooldown fields)

## Problem

Decision traces do not log exhaustion cooldown state. For debuggability (P27), `PlanningPipelineTrace` should include a snapshot of the exhaustion cache showing retry state, consecutive failures, next retry tick, and current eligibility per opportunity.

## Assumption Reassessment (2026-03-29)

1. `PlanningPipelineTrace` is defined in `crates/worldwake-ai/src/decision_trace.rs` and currently lacks any exhaustion snapshot field. Live fields are `dirty`, `plan_continued`, `candidates`, `planning`, `selection`, `execution`, `action_start_failures`, `unknown_blockers`, and `frame_transition`.
2. `specs/S37-cooldown-based-exhaustion.md` Section 8 still requires `ExhaustionTraceEntry` plus `PlanningPipelineTrace.exhaustion_snapshot: Vec<ExhaustionTraceEntry>`. That requirement is not yet implemented.
3. The cooldown architecture this ticket depends on is already live in `crates/worldwake-ai/src/decision_runtime.rs`: `ExhaustionEntry` already has `next_retry_tick`, `consecutive_failures`, `is_retry_eligible(current_tick)`, and `record_budget_exhaustion(...)`.
4. Cooldown-aware admission and retry triggering are already live in `crates/worldwake-ai/src/agent_tick/planning.rs`, including `has_pending_budget_retry(runtime, current_tick)` and candidate filtering that checks both `suppresses_planning()` and `is_retry_eligible(current_tick)`.
5. The `PlanningPipelineTrace` construction site is `crates/worldwake-ai/src/agent_tick/mod.rs`, not `crates/worldwake-ai/src/agent_tick/planning.rs`. Scope corrected accordingly.
6. The shared abstraction boundary under audit is `AgentDecisionRuntime.exhaustion_cache -> PlanningPipelineTrace` as a derived debug surface. This is planner-adjacent, but not a planner-behavior change.
7. No golden scenario is required. The strongest proof surface is focused runtime and trace coverage.
8. Real current test targets exist and should replace the prior approximate filters: `cargo test -p worldwake-ai decision_runtime::tests::retry_eligibility_respects_retry_tick_and_frontier_suppression`, `cargo test -p worldwake-ai agent_tick::tests::trace_planning_outcome_for_hungry_agent`, and `cargo test -p worldwake-ai decision_trace::tests`.
9. No adjacent architectural contradiction was found in the live cooldown model. The only missing deliverable is debug trace visibility promised by `S37`.
10. Scope correction: no save/load, planning-behavior, or heuristic work belongs in this ticket.

## Architecture Check

1. Adding a diagnostic snapshot field on `PlanningPipelineTrace` is cleaner than expecting trace consumers to inspect `AgentDecisionRuntime` out of band. The planning trace already carries other derived debug views such as `unknown_blockers` and `action_start_failures`; exhaustion state belongs in the same per-tick debug record.
2. `ExhaustionTraceEntry` remains a derived read-model over `AgentDecisionRuntime.exhaustion_cache`, so authoritative meaning stays in `ExhaustionEntry` and no second source of truth is introduced.
3. No backward-compatibility shims or alternate trace paths should be added.

## Verification Layers

1. exhaustion snapshot population and field fidelity -> focused planning trace test using `AgentTickDriver` with tracing enabled
2. retry eligibility arithmetic itself -> existing focused `decision_runtime` tests
3. empty-cache behavior -> focused planning trace test
4. manual trace helper fixtures remain valid -> focused `decision_trace` tests

## What to Change

### 1. Add `ExhaustionTraceEntry` struct

In `crates/worldwake-ai/src/decision_trace.rs`:

```rust
/// Snapshot of one opportunity's exhaustion state at trace time.
#[derive(Clone, Debug)]
pub struct ExhaustionTraceEntry {
    pub opportunity: OpportunityKey,
    pub retry_state: ExhaustionRetryState,
    pub consecutive_failures: u8,
    pub next_retry_tick: Option<Tick>,
    pub retry_eligible: bool,
}
```

### 2. Add field to `PlanningPipelineTrace`

```rust
/// Exhaustion cache state at trace construction time (P27).
pub exhaustion_snapshot: Vec<ExhaustionTraceEntry>,
```

### 3. Populate snapshot at construction site

Populate from `runtime.exhaustion_cache` where `PlanningPipelineTrace` is constructed in `crates/worldwake-ai/src/agent_tick/mod.rs`:

```rust
exhaustion_snapshot: runtime.exhaustion_cache.iter().map(|(key, entry)| {
    ExhaustionTraceEntry {
        opportunity: *key,
        retry_state: entry.retry_state.clone(),
        consecutive_failures: entry.consecutive_failures,
        next_retry_tick: entry.next_retry_tick,
        retry_eligible: entry.is_retry_eligible(current_tick),
    }
}).collect(),
```

### 4. Export new type

Export `ExhaustionTraceEntry` from the crate because trace-focused tests and public debug consumers use the public `worldwake-ai` trace types.

## Files to Touch

- `crates/worldwake-ai/src/decision_trace.rs` (modify)
- `crates/worldwake-ai/src/agent_tick/mod.rs` (modify — trace construction site)
- `crates/worldwake-ai/src/agent_tick/tests.rs` (modify — focused trace coverage)

## Out of Scope

- `ExhaustionEntry` struct changes already delivered by earlier `S37` work
- `PlanningBudget` changes
- Planning logic changes already delivered by prior cooldown tickets
- Save/load changes (`S37COOBASEXH-007`)
- `dump_agent()` or `summary()` output format changes unless needed for compilation
- Golden test changes

## Acceptance Criteria

### Tests That Must Pass

1. Decision trace `exhaustion_snapshot` populated with current cooldown state when tracing is enabled
2. `retry_eligible` field correctly reflects `is_retry_eligible(current_tick)` for each entry
3. Empty exhaustion cache → empty `exhaustion_snapshot`
4. Existing trace-focused coverage still passes after the new field is added

### Invariants

1. `ExhaustionTraceEntry` is a derived read-model — no authoritative state mutation
2. Tracing remains opt-in and zero-cost when disabled
3. Existing `PlanningPipelineTrace` fields remain semantically unchanged; only the new derived snapshot is added

## Tests

### New/Modified Tests

1. `agent_tick::tests::trace_planning_outcome_includes_exhaustion_snapshot`
Rationale: proves the real per-tick planning trace includes derived cooldown state from `AgentDecisionRuntime.exhaustion_cache`, including `retry_eligible` at the current tick.
2. `agent_tick::tests::trace_planning_outcome_uses_empty_exhaustion_snapshot_without_cache`
Rationale: proves the trace stays empty when no exhaustion entries exist, preventing stale or synthesized debug state.
3. `decision_trace::tests` updated manual `PlanningPipelineTrace` fixtures
Rationale: keeps direct trace helper coverage aligned with the public trace struct shape.

### Commands

1. `cargo test -p worldwake-ai decision_runtime::tests::retry_eligibility_respects_retry_tick_and_frontier_suppression`
2. `cargo test -p worldwake-ai agent_tick::tests::trace_planning_outcome_for_hungry_agent`
3. `cargo test -p worldwake-ai agent_tick::tests::trace_planning_outcome_includes_exhaustion_snapshot`
4. `cargo test -p worldwake-ai decision_trace::tests`
5. `cargo test -p worldwake-ai`
6. `cargo clippy --workspace`

## Outcome

- Completed: 2026-03-29
- Actual changes:
  - Added `ExhaustionTraceEntry` to `crates/worldwake-ai/src/decision_trace.rs`.
  - Added `PlanningPipelineTrace.exhaustion_snapshot`.
  - Populated the snapshot from `AgentDecisionRuntime.exhaustion_cache` in `crates/worldwake-ai/src/agent_tick/mod.rs`.
  - Exported `ExhaustionTraceEntry` from `worldwake-ai`.
  - Added focused trace coverage in `crates/worldwake-ai/src/agent_tick/tests.rs` and updated direct `PlanningPipelineTrace` fixtures in `crates/worldwake-ai/src/decision_trace.rs`.
- Deviations from original plan:
  - The trace construction site was corrected from `crates/worldwake-ai/src/agent_tick/planning.rs` to `crates/worldwake-ai/src/agent_tick/mod.rs`.
  - The ticket scope was narrowed to the missing trace snapshot only because the cooldown runtime and planning behavior were already implemented.
  - The focused runtime trace test uses a stable non-firing invalidation condition because empty invalidation-condition entries are intentionally dropped by `invalidate_exhausted_goals()`.
- Verification results:
  - `cargo test -p worldwake-ai decision_runtime::tests::retry_eligibility_respects_retry_tick_and_frontier_suppression` ✅
  - `cargo test -p worldwake-ai agent_tick::tests::trace_planning_outcome_for_hungry_agent` ✅
  - `cargo test -p worldwake-ai agent_tick::tests::trace_planning_outcome_includes_exhaustion_snapshot` ✅
  - `cargo test -p worldwake-ai decision_trace::tests` ✅
  - `cargo test -p worldwake-ai` ✅
  - `cargo clippy --workspace` ✅
