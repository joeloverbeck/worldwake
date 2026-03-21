# S12PLAPREAWA-005: Enrich `SearchExpansionSummary` with prerequisite place counts

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: Yes — new fields on `SearchExpansionSummary`
**Deps**: S12PLAPREAWA-003 (combined places computation exists in search loop)

## Problem

When debugging "why did the agent travel toward location X?", the decision trace cannot distinguish whether X was a goal-terminal location or a prerequisite location. Adding `combined_places_count` and `prerequisite_places_count` to `SearchExpansionSummary` enables this diagnosis (Principle 27: Debuggability).

## Assumption Reassessment (2026-03-21)

1. `SearchExpansionSummary` is defined at `crates/worldwake-ai/src/decision_trace.rs` with fields: `depth`, `remaining_travel_ticks`, `candidates_generated`, `candidates_skipped`, `terminal_successors`, `non_terminal_before_beam`, `non_terminal_after_beam`, `found_goal_satisfied`, `travel_pruning` — confirmed.
2. `SearchExpansionSummary` derives `Clone, Debug` — confirmed.
3. `expansion_summaries: Option<&mut Vec<SearchExpansionSummary>>` is passed through `search_plan()` and populated in the main search loop — confirmed.
4. `TravelPruningTrace` already captures retained/pruned destinations — the new fields complement this with counts showing how many places drove the pruning decision.
5. Single-layer struct field addition — no AI regression, ordering, or heuristic concerns.

## Architecture Check

1. Adding two `u16` fields to an existing trace struct is the minimal change. The alternative — adding a separate `PrerequisitePlacesTrace` struct — is over-engineering for two counters.
2. No backwards-compatibility shims. All construction sites for `SearchExpansionSummary` must add the new fields.

## Verification Layers

1. New fields are populated during search loop → verified by inspecting trace output in existing golden tests with tracing enabled
2. Fields correctly reflect combined vs prerequisite counts → focused unit test or assertion in golden test
3. Single-layer ticket — verification is compilation + trace inspection.

## What to Change

### 1. Add fields to `SearchExpansionSummary`

In `crates/worldwake-ai/src/decision_trace.rs`:

```rust
pub struct SearchExpansionSummary {
    // ... existing fields ...
    /// Total number of combined places (goal-terminal + prerequisite) guiding this expansion.
    pub combined_places_count: u16,
    /// Number of prerequisite-only places in the combined set (0 when all prerequisites are satisfied).
    pub prerequisite_places_count: u16,
}
```

### 2. Populate fields in `search_plan()` main loop

In `crates/worldwake-ai/src/search.rs`, where `SearchExpansionSummary` is constructed in the main loop, compute and record:

- `combined_places_count`: length of the `combined_relevant_places()` result for this node
- `prerequisite_places_count`: length of the `prerequisite_places()` result for this node (= `combined_places_count - goal_relevant_places().len()`, but compute directly to avoid ambiguity from deduplication)

### 3. Update all `SearchExpansionSummary` construction sites

Ensure any existing construction of `SearchExpansionSummary` (including in tests) includes the two new fields.

## Files to Touch

- `crates/worldwake-ai/src/decision_trace.rs` (modify — struct definition)
- `crates/worldwake-ai/src/search.rs` (modify — populate new fields in expansion summary construction)

## Out of Scope

- `TravelPruningTrace` changes — existing struct is unchanged
- `DecisionTraceSink` API changes — no new query methods
- `dump_agent()` formatting changes — the new fields will appear in `Debug` output automatically
- `goal_model.rs` changes (S12PLAPREAWA-002)
- `agent_tick.rs` changes (S12PLAPREAWA-004)
- Golden tests (S12PLAPREAWA-007)

## Acceptance Criteria

### Tests That Must Pass

1. `SearchExpansionSummary` compiles with new fields
2. When tracing is enabled, expansion summaries contain correct `combined_places_count` and `prerequisite_places_count` values
3. Existing suite: `cargo test -p worldwake-ai`
4. Existing suite: `cargo clippy --workspace`

### Invariants

1. For goals with empty `prerequisite_places()`, `prerequisite_places_count == 0` and `combined_places_count == goal_relevant_places.len()`
2. `SearchExpansionSummary` remains `Clone + Debug`
3. Tracing remains opt-in and zero-cost when disabled

## Test Plan

### New/Modified Tests

1. None required beyond compilation — the fields are diagnostic. Golden tests in S12PLAPREAWA-007 will exercise and verify trace content.

### Commands

1. `cargo test -p worldwake-ai`
2. `cargo clippy --workspace`
