# S25-003: Add feasibility to decision traces

**Status**: ✅ COMPLETED
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: Yes — extends decision trace output
**Deps**: S25-001

## Problem

Decision traces are the primary diagnostic tool for AI behavior. After S25-001 adds `FeasibilityHint` to `RankedGoal`, the trace system must surface feasibility hints per candidate so developers can see why candidate ordering changed.

## Assumption Reassessment (2026-03-25)

1. `RankedGoalSummary` in `crates/worldwake-ai/src/decision_trace.rs:243-248` has fields: `goal`, `priority_class`, `motive_score`, `provenance`. No `feasibility` field exists yet.
2. `summarize_ranked_goal()` in `crates/worldwake-ai/src/agent_tick/planning.rs:116-123` copies fields from `RankedGoal` to `RankedGoalSummary`. After S25-001 adds `feasibility` to `RankedGoal`, this function must copy the new field.
3. `format_outcome()` in `crates/worldwake-ai/src/decision_trace.rs:810-914` handles `DecisionOutcome::Planning` by printing candidates count, selected goal, plan source, etc. The ranked candidates are stored as `Vec<RankedGoalSummary>` in `CandidateTrace.ranked`. The feasibility hint should appear in per-candidate output.
4. `dump_agent()` in `crates/worldwake-ai/src/decision_trace.rs:693-701` delegates to `format_outcome()` — no direct changes needed there.
5. Existing tests that construct `RankedGoalSummary` directly (at lines ~1487, ~1572, ~1628 in decision_trace.rs) must be updated to include the new `feasibility` field.

## Architecture Check

1. Adding `feasibility` to `RankedGoalSummary` mirrors the same field on `RankedGoal`, maintaining the established pattern where the summary is a lightweight trace snapshot of the ranked goal.
2. No backward-compatibility shims. Tests updated inline.

## Verification Layers

1. Feasibility appears in trace output: focused test enabling tracing and checking trace data contains non-Uncertain feasibility hints
2. Existing trace tests compile: `cargo test -p worldwake-ai decision_trace`
3. Single-layer ticket — trace output is the only verification surface

## What to Change

### 1. Add `feasibility` field to `RankedGoalSummary` in `decision_trace.rs`

```rust
pub struct RankedGoalSummary {
    pub goal: GoalKey,
    pub priority_class: GoalPriorityClass,
    pub motive_score: u32,
    pub provenance: Option<RankedGoalProvenance>,
    pub feasibility: FeasibilityHint,
}
```

### 2. Update `summarize_ranked_goal()` in `agent_tick/planning.rs`

Copy the `feasibility` field from `RankedGoal` to `RankedGoalSummary`.

### 3. Update `format_outcome()` in `decision_trace.rs`

In the Planning arm, when printing per-candidate information, include feasibility when it is not `Uncertain`:
- For `selected_ranked_goal_summary`, append feasibility to the selection line if not `Uncertain`
- In `summary()` on `DecisionOutcome`, mention feasibility only when non-`Uncertain`

### 4. Update existing test construction sites

All inline `RankedGoalSummary` constructions in `decision_trace.rs` tests must add `feasibility: FeasibilityHint::Uncertain`.

## Files to Touch

- `crates/worldwake-ai/src/decision_trace.rs` (modify — add field to `RankedGoalSummary`, update format, update test constructions)
- `crates/worldwake-ai/src/agent_tick/planning.rs` (modify — copy feasibility in `summarize_ranked_goal`)

## Out of Scope

- The `FeasibilityHint` enum itself (S25-001)
- Integration into `process_agent()` (S25-002)
- Golden test behavior verification (S25-004)
- Any changes to `ActionTraceSink` or action trace system

## Acceptance Criteria

### Tests That Must Pass

1. `test_trace_shows_feasibility_hint` — enable tracing on a harness, step ticks, verify `RankedGoalSummary` in trace contains non-default feasibility values after annotation
2. Existing decision trace tests compile and pass with the new field initialized to `Uncertain`
3. Existing suite: `cargo test -p worldwake-ai decision_trace` — all pass

### Invariants

1. `RankedGoalSummary.feasibility` is always populated (no `Option` — defaults to `Uncertain`)
2. Trace output only shows feasibility annotation when it is not `Uncertain` (to avoid noise in common case)

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/decision_trace.rs` (inline tests) — 1 test verifying feasibility appears in trace output
2. Existing `RankedGoalSummary` construction sites in tests — updated to include `feasibility: FeasibilityHint::Uncertain`

### Commands

1. `cargo test -p worldwake-ai decision_trace` — decision trace tests
2. `cargo test -p worldwake-ai` — full AI crate
3. `cargo clippy -p worldwake-ai` — no new warnings

## Outcome

- **Completion date**: 2026-03-25
- **What changed**:
  - Added `pub feasibility: FeasibilityHint` field to `RankedGoalSummary` in `decision_trace.rs`
  - Updated `summarize_ranked_goal()` in `agent_tick/planning.rs` to copy feasibility from `RankedGoal`
  - Updated `summary()` and `format_outcome()` to append feasibility hint when non-`Uncertain`
  - Updated 7 test construction sites with `feasibility: FeasibilityHint::Uncertain`
- **Deviations from original plan**: None
- **Verification results**: `cargo test -p worldwake-ai` 32/32 pass, 21/21 golden tests pass, `cargo clippy -p worldwake-ai` zero warnings
