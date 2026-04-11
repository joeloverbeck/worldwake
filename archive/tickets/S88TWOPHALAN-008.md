# S88TWOPHALAN-008: Decision trace enrichment for two-phase planning

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: Yes — extends planner trace structs and search-root trace carriers
**Deps**: S88TWOPHALAN-007

## Problem

The decision trace currently has no visibility into strategic planning or landmark guidance. Without trace enrichment (S88 D9), debuggers cannot answer "why did the agent go there?" (strategic plan) or "why did it try that action first?" (preferred operator from landmark). This violates FND-29 (Debuggability).

## Assumption Reassessment (2026-04-11)

1. `PlanAttemptTrace` at `crates/worldwake-ai/src/decision_trace.rs:856` has fields: `goal`, `opportunity_anchor`, `outcome`, `target_belief_presence`, `binding_rejections`, `expansion_summaries`. New fields `strategic_plan`, `landmarks_extracted`, `landmark_orderings` will be added.
2. `SearchExpansionSummary` at `decision_trace.rs:764` has fields: `depth`, `remaining_travel_ticks`, `combined_places_count`, `prerequisite_places_count`, `candidates_generated`, `candidates_skipped`, `terminal_successors`, `non_terminal_before_beam`, `non_terminal_after_beam`, `found_goal_satisfied`, `travel_pruning`, `prerequisite_guidance`, `root_candidates`, `root_omissions`. New fields `preferred_candidates` and `landmark_heuristic` will be added.
3. `PlanAttemptTrace` is populated in `agent_tick/planning.rs` via `plan_search_result_to_trace()`, but the live planner boundary does not currently carry strategic-plan or landmark-count metadata out of `search_plan()`. This ticket therefore needs one explicit planner-trace metadata carrier from `search/mod.rs` into `agent_tick/planning.rs`, not just field additions at the final trace struct.

## Architecture Check

1. Extending existing trace structs with additional diagnostic fields follows the established pattern, but the planner should expose that data through one canonical trace carrier rather than recomputing strategic-plan state in `agent_tick/planning.rs`.
2. No backwards-compatibility shims. New fields are added to existing structs.

## Verification Layers

1. Strategic plan trace populated → focused test: decision trace after multi-location planning contains strategic step data
2. Landmark counts populated → focused test: trace contains non-zero landmark counts when landmarks are active
3. Preferred candidate counts → focused test: expansion summaries report preferred candidate counts
4. Single-layer ticket (diagnostic trace extension) — no cross-layer mapping beyond existing decision trace contract.

## What to Change

### 1. Add `StrategicStepTrace` to `decision_trace.rs`

```rust
#[derive(Clone, Debug)]
pub struct StrategicStepTrace {
    pub destination: EntityId,
    pub sub_goal: String,
    pub estimated_travel_ticks: u32,
}
```

### 2. Extend `PlanAttemptTrace`

Add fields:
```rust
pub strategic_plan: Option<Vec<StrategicStepTrace>>,
pub landmarks_extracted: u16,
pub landmark_orderings: u16,
```

### 3. Extend `SearchExpansionSummary`

Add fields:
```rust
pub preferred_candidates: u16,
pub landmark_heuristic: u32,
```

### 4. Add one explicit planner-trace metadata carrier and update all construction sites

- `search/mod.rs` — populate strategic-plan and landmark-count metadata once during live search and return it through the planner-root search boundary
- `PlanAttemptTrace` construction in `agent_tick/planning.rs` — add strategic-plan data from that search metadata carrier
- `SearchExpansionSummary` construction in `search/mod.rs` (lines ~238, ~287) — add preferred candidate count and landmark heuristic value from the expansion loop

### 5. Update trace formatting/display

If `PlanAttemptTrace` or `SearchExpansionSummary` have `Display` or formatting impls, include the new fields in the output.

## Files to Touch

- `crates/worldwake-ai/src/decision_trace.rs` (modify — add types and fields)
- `crates/worldwake-ai/src/search/mod.rs` (modify — populate new SearchExpansionSummary fields)
- `crates/worldwake-ai/src/agent_tick/planning.rs` (modify — populate new PlanAttemptTrace fields)

## Out of Scope

- Golden E2E tests that assert on trace content (S88TWOPHALAN-009)
- CLI formatting of strategic plan traces
- Observer diagnostic improvements (deferred per spec non-goals)

## Acceptance Criteria

### Tests That Must Pass

1. Focused trace tests prove strategic-plan and landmark fields are populated lawfully
2. Existing decision trace tests pass with new fields
3. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. `strategic_plan` is `None` when no strategic planning was performed (local-only goals)
2. `landmarks_extracted` and `landmark_orderings` are 0 when `landmark_extraction_depth = 0`
3. `preferred_candidates` is 0 when no landmarks are active
4. All existing golden tests pass (trace extension is additive)

## Outcome

- **Completion date**: 2026-04-11
- **What changed**: Implemented the two-phase trace enrichment through one canonical planner-root metadata carrier. `search/mod.rs` now records strategic-plan metadata plus landmark extraction/order counts during live search and populates `preferred_candidates` and `landmark_heuristic` on each `SearchExpansionSummary`. `agent_tick/planning.rs` threads that metadata into `PlanAttemptTrace`, and `decision_trace.rs` now exposes `StrategicStepTrace`, the new strategic/landmark fields, and formatted output for them. Focused coverage was added for planner-root trace metadata, planning-trace conversion, and decision-trace formatting, and existing observer fixtures were updated to match the additive trace shape.
- **Deviations from original plan**: The trace metadata path was kept internal to the planner root. Instead of widening the public `search_plan(...)` API, the implementation preserved the existing public signature and added an internal traced variant for the new carrier. Observer fixture fallout was also updated so CI-matching all-target verification remained green.
- **Verification results**: Passed `cargo test -p worldwake-ai search_trace_metadata_records_two_phase_strategic_and_landmark_details`, `cargo test -p worldwake-ai search_trace_metadata_zero_landmarks_reports_zero_counts`, `cargo test -p worldwake-ai plan_search_trace_converts_two_phase_trace_metadata`, `cargo test -p worldwake-ai expansion_summary_default_and_debug_format`, `cargo test -p worldwake-ai`, `cargo clippy --workspace --all-targets -- -D warnings`, and `cargo test --workspace`.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/search/tests.rs` — focused planner-root trace coverage for two-phase strategic and landmark fields
2. `crates/worldwake-ai/src/decision_trace.rs` and `crates/worldwake-ai/src/agent_tick/planning.rs` — construction and formatting fallout for the new trace fields
3. `crates/worldwake-cli/src/bin/observer.rs` — fixture fallout for additive trace struct fields

### Commands

1. `cargo test -p worldwake-ai search_trace_metadata_records_two_phase_strategic_and_landmark_details`
2. `cargo test -p worldwake-ai search_trace_metadata_zero_landmarks_reports_zero_counts`
3. `cargo test -p worldwake-ai plan_search_trace_converts_two_phase_trace_metadata`
4. `cargo test -p worldwake-ai expansion_summary_default_and_debug_format`
5. `cargo test -p worldwake-ai`
6. `cargo clippy --workspace --all-targets -- -D warnings`
7. `cargo test --workspace`
