# S88TWOPHALAN-008: Decision trace enrichment for two-phase planning

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: None — extends existing trace structs
**Deps**: S88TWOPHALAN-007

## Problem

The decision trace currently has no visibility into strategic planning or landmark guidance. Without trace enrichment (S88 D9), debuggers cannot answer "why did the agent go there?" (strategic plan) or "why did it try that action first?" (preferred operator from landmark). This violates FND-29 (Debuggability).

## Assumption Reassessment (2026-04-11)

1. `PlanAttemptTrace` at `crates/worldwake-ai/src/decision_trace.rs:856` has fields: `goal`, `opportunity_anchor`, `outcome`, `target_belief_presence`, `binding_rejections`, `expansion_summaries`. New fields `strategic_plan`, `landmarks_extracted`, `landmark_orderings` will be added.
2. `SearchExpansionSummary` at `decision_trace.rs:764` has fields: `depth`, `remaining_travel_ticks`, `combined_places_count`, `prerequisite_places_count`, `candidates_generated`, `candidates_skipped`, `terminal_successors`, `non_terminal_before_beam`, `non_terminal_after_beam`, `found_goal_satisfied`, `travel_pruning`, `prerequisite_guidance`, `root_candidates`, `root_omissions`. New fields `preferred_candidates` and `landmark_heuristic` will be added.
3. Construction sites for `PlanAttemptTrace` — populated in `agent_tick/planning.rs` via `plan_search_result_to_trace()` or similar. Construction sites for `SearchExpansionSummary` — populated in `search/mod.rs` at lines 238 and 287. Both need the new fields added at all construction sites.

## Architecture Check

1. Extending existing trace structs with additional diagnostic fields follows the established pattern. The new fields are populated during the search loop (S88TWOPHALAN-007 already has access to strategic plan and landmark data).
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

### 4. Update all construction sites

- `PlanAttemptTrace` construction in `agent_tick/planning.rs` — add strategic plan data from the search result
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

1. Existing decision trace tests pass with new fields
2. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. `strategic_plan` is `None` when no strategic planning was performed (local-only goals)
2. `landmarks_extracted` and `landmark_orderings` are 0 when `landmark_extraction_depth = 0`
3. `preferred_candidates` is 0 when no landmarks are active
4. All existing golden tests pass (trace extension is additive)

## Test Plan

### New/Modified Tests

1. None — documentation-only ticket; verification is command-based and existing runtime coverage is named in Assumption Reassessment. The trace fields are exercised by S88TWOPHALAN-009 golden tests.

### Commands

1. `cargo test -p worldwake-ai`
2. `cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`
