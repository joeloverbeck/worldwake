# S37COOBASEXH-006: Decision trace extension for cooldown state

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: Yes — decision_trace.rs new struct and field on PlanningPipelineTrace
**Deps**: S37COOBASEXH-002 (ExhaustionEntry has cooldown fields)

## Problem

Decision traces do not log exhaustion cooldown state. For debuggability (P27), `PlanningPipelineTrace` should include a snapshot of the exhaustion cache showing retry state, consecutive failures, next retry tick, and current eligibility per opportunity.

## Assumption Reassessment (2026-03-29)

1. `PlanningPipelineTrace` defined at `crates/worldwake-ai/src/decision_trace.rs:189-208`. Currently has 8 fields: `dirty`, `plan_continued`, `candidates`, `planning`, `selection`, `execution`, `action_start_failures`, `unknown_blockers`, `frame_transition`.
2. Spec S37 Section 8 adds `ExhaustionTraceEntry` struct and `exhaustion_snapshot: Vec<ExhaustionTraceEntry>` field.
3. `PlanningPipelineTrace` is constructed during the planning pipeline — need to find the construction site to populate the snapshot.
4. N/A — no golden scenario.
5. N/A — not planner-driven.
6. N/A — not an AI regression.
7. N/A — no ordering.
8. N/A — no heuristic removal.
9-12. N/A.
13. No adjacent contradictions.
14. No mismatch.
15. N/A.

## Architecture Check

1. Adding a diagnostic snapshot field follows the established pattern of `unknown_blockers` and `action_start_failures` on `PlanningPipelineTrace`. The `ExhaustionTraceEntry` struct is a derived read-model (P3 compliant).
2. No backward-compatibility shims.

## Verification Layers

1. `exhaustion_snapshot` populated with correct entries → focused unit test
2. `retry_eligible` field matches `is_retry_eligible(current_tick)` → focused unit test
3. Single-layer: diagnostic trace output. No authoritative state change.

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

Find where `PlanningPipelineTrace` is constructed in the planning pipeline. Add population from `runtime.exhaustion_cache`:

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

Ensure `ExhaustionTraceEntry` is exported from the crate if needed by test consumers.

## Files to Touch

- `crates/worldwake-ai/src/decision_trace.rs` (modify)
- `crates/worldwake-ai/src/agent_tick/planning.rs` (modify — trace construction site)

## Out of Scope

- `ExhaustionEntry` struct changes (S37COOBASEXH-002)
- `PlanningBudget` changes (S37COOBASEXH-001)
- Planning logic changes (S37COOBASEXH-003, -004, -005)
- Save/load changes (S37COOBASEXH-007)
- `dump_agent()` or `summary()` output format changes (nice-to-have, not in spec)
- Golden test changes

## Acceptance Criteria

### Tests That Must Pass

1. Decision trace `exhaustion_snapshot` populated with current cooldown state when tracing is enabled
2. `retry_eligible` field correctly reflects `is_retry_eligible(current_tick)` for each entry
3. Empty exhaustion cache → empty `exhaustion_snapshot`
4. Existing suite: `cargo test -p worldwake-ai -- decision_trace`

### Invariants

1. `ExhaustionTraceEntry` is a derived read-model — no authoritative state mutation
2. Tracing remains opt-in and zero-cost when disabled
3. Existing `PlanningPipelineTrace` fields unchanged

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/decision_trace.rs` or planning test module — new test: `exhaustion_snapshot_populated_in_trace`
2. Update any existing trace construction tests to include the new field

### Commands

1. `cargo test -p worldwake-ai -- decision_trace`
2. `cargo test -p worldwake-ai -- trace`
3. `cargo clippy --workspace && cargo test -p worldwake-ai`
