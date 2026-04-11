# S89UNITWOPHA-002: Decision trace tactical goal recording

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: Yes — diagnostic metadata (SearchTraceMetadata)
**Deps**: S89UNITWOPHA-001

## Problem

After S89UNITWOPHA-001 and S89UNITWOPHA-004, agents receive `TravelToGoal` tactical scoping for strategic `SatisfyGoal` stages across non-whitelisted remote-goal families, and supported no-evidence exploration fallback now uses a dedicated `Explore` progress-barrier contract. But the decision trace does not record which tactical goal was active during search. When debugging why an agent scoped its search to a particular location, the trace shows the strategic plan but not the tactical goal derived from it. This violates FND-29 (Debuggability).

## Assumption Reassessment (2026-04-11)

1. `SearchTraceMetadata` at `crates/worldwake-ai/src/search/mod.rs:47` currently has 3 fields: `strategic_plan`, `landmarks_extracted`, `landmark_orderings`. No `tactical_goal` field exists. Derives `Clone, Debug, Default`.
2. `trace_state` is constructed in `crates/worldwake-ai/src/search/mod.rs::search_plan_with_trace_metadata()`, and the `tactical_goal` local is already constructed before the search loop. Recording it into `trace_state` after construction is still a single assignment.
3. The real carriage boundary is wider than the original ticket claimed. `SearchTraceMetadata` is converted into `decision_trace::PlanAttemptTrace` in `crates/worldwake-ai/src/agent_tick/planning.rs`, and `PlanAttemptTrace` is rendered in `crates/worldwake-ai/src/decision_trace.rs`.
4. The live focused proof surface already exists in `crates/worldwake-ai/src/search/tests.rs::search_trace_metadata_records_two_phase_strategic_and_landmark_details`, and `crates/worldwake-ai/src/agent_tick/planning.rs` already contains a manual `SearchTraceMetadata` literal in `plan_search_trace_converts_two_phase_trace_metadata()`. This ticket should extend those tests directly instead of deferring all focused validation to `S89UNITWOPHA-003`.

## Architecture Check

1. Adding a `tactical_goal: Option<String>` field to `SearchTraceMetadata` is still the minimal diagnostic extension. Using `String` (Debug-formatted) rather than storing the `TacticalGoal` value avoids exposing the `pub(super)` type outside the search module.
2. No backwards-compatibility aliasing/shims introduced.

## Verification Layers

1. Tactical goal recorded in trace → focused test: construct a search that produces a `TravelToGoal` tactical goal, verify `SearchTraceMetadata.tactical_goal` is `Some(...)` containing the variant name
2. Local goals produce `None` tactical goal in trace → focused test: Sleep goal produces `tactical_goal: None` in metadata
3. Single-layer ticket (diagnostic metadata only) — no cross-system mapping applicable

## What to Change

### 1. Add `tactical_goal` field to `SearchTraceMetadata`

In `crates/worldwake-ai/src/search/mod.rs`, add to the struct at line 47:

```rust
pub(crate) tactical_goal: Option<String>,
```

The `Default` derive will initialize this to `None`.

### 2. Record tactical goal after construction

After the tactical goal construction (post-001 change), add:

```rust
trace_state.tactical_goal = tactical_goal.as_ref().map(|tg| format!("{tg:?}"));
```

This records the Debug representation of the active tactical goal variant.

### 3. Preserve the trace-carriage and render path

- Extend `plan_search_result_to_trace()` in `crates/worldwake-ai/src/agent_tick/planning.rs` so `PlanAttemptTrace` receives the tactical goal string.
- Add `tactical_goal: Option<String>` to `decision_trace::PlanAttemptTrace` in `crates/worldwake-ai/src/decision_trace.rs`.
- Render the tactical goal in the textual decision trace when present so the new metadata is visible to the existing debug surface.

## Files to Touch

- `crates/worldwake-ai/src/search/mod.rs` (modify)
- `crates/worldwake-ai/src/agent_tick/planning.rs` (modify)
- `crates/worldwake-ai/src/decision_trace.rs` (modify)
- `crates/worldwake-ai/src/search/tests.rs` (modify)
- `crates/worldwake-cli/src/bin/observer.rs` (modify for workspace clippy fallout: manual `PlanAttemptTrace` literal)

## Out of Scope

- Changing decision trace formatting or output format
- Adding tactical goal information to event log or world state
- Modifying the `TacticalGoal` enum's Debug derive

## Acceptance Criteria

### Tests That Must Pass

1. Existing suite: `cargo test -p worldwake-ai`
2. `search_trace_metadata_records_two_phase_strategic_and_landmark_details` records `Some("TravelToGoal { ... }")` for a remote two-phase goal
3. A local-goal trace proof records `tactical_goal: None`
4. `plan_search_trace_converts_two_phase_trace_metadata` preserves the tactical-goal string through the `PlanAttemptTrace` conversion layer

### Invariants

1. `SearchTraceMetadata::tactical_goal` is diagnostic only — never used for search decisions, only for trace output
2. `Default` impl continues to work (all fields have defaults)

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/search/tests.rs::search_trace_metadata_records_two_phase_strategic_and_landmark_details` — extend to assert the tactical goal string
2. `crates/worldwake-ai/src/search/tests.rs` — add a local-goal metadata assertion proving `tactical_goal` stays `None`
3. `crates/worldwake-ai/src/agent_tick/planning.rs::plan_search_trace_converts_two_phase_trace_metadata` — extend to assert the tactical-goal field is preserved

### Commands

1. `cargo test -p worldwake-ai search_trace_metadata_records_two_phase_strategic_and_landmark_details`
2. `cargo test -p worldwake-ai search_trace_metadata_zero_landmarks_reports_zero_counts`
3. `cargo test -p worldwake-ai search_trace_metadata_records_no_tactical_goal_for_local_sleep`
4. `cargo test -p worldwake-ai plan_search_trace_converts_two_phase_trace_metadata`
5. `cargo test -p worldwake-ai` — all existing tests pass
6. `cargo clippy --workspace --all-targets -- -D warnings` — no new warnings

## Outcome

Completed on 2026-04-11.

- Added `tactical_goal: Option<String>` to `search::SearchTraceMetadata` and recorded the active tactical goal in `search_plan_with_trace_metadata()`.
- Carried that diagnostic field through `agent_tick::planning::plan_search_result_to_trace()` into `decision_trace::PlanAttemptTrace`, and rendered it in the textual decision trace when present.
- Extended focused trace tests to prove a remote two-phase plan records its tactical goal, a local `Sleep` plan keeps `tactical_goal = None`, and the conversion layer preserves the field.
- Updated one manual `PlanAttemptTrace` literal in `worldwake-cli/src/bin/observer.rs` as all-targets clippy fallout from the shared trace-shape change.

## Verification Result

- Passed `cargo test -p worldwake-ai search_trace_metadata_records_two_phase_strategic_and_landmark_details`
- Passed `cargo test -p worldwake-ai search_trace_metadata_zero_landmarks_reports_zero_counts`
- Passed `cargo test -p worldwake-ai search_trace_metadata_records_no_tactical_goal_for_local_sleep`
- Passed `cargo test -p worldwake-ai plan_search_trace_converts_two_phase_trace_metadata`
- Passed `cargo test -p worldwake-ai`
- Passed `cargo clippy --workspace --all-targets -- -D warnings`
