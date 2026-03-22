# S12PLAPREAWA-005: Enrich `SearchExpansionSummary` with prerequisite place counts

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: Yes — new fields on `SearchExpansionSummary`
**Deps**: S12PLAPREAWA-003 (combined places computation exists in search loop)

## Problem

When debugging "why did the agent travel toward location X?", the decision trace cannot distinguish whether X was a goal-terminal location or a prerequisite location. Adding `combined_places_count` and `prerequisite_places_count` to `SearchExpansionSummary` enables this diagnosis (Principle 27: Debuggability).

## Assumption Reassessment (2026-03-21)

1. `SearchExpansionSummary` is still defined at `crates/worldwake-ai/src/decision_trace.rs` and currently exposes `depth`, `remaining_travel_ticks`, `candidates_generated`, `candidates_skipped`, `terminal_successors`, `non_terminal_before_beam`, `non_terminal_after_beam`, `found_goal_satisfied`, and `travel_pruning`. The two place-count fields described by this ticket are still missing.
2. The broader S12 prerequisite-aware planner architecture is already live, so this ticket’s original implied scope is stale. Verified in current code:
   - `GoalKindPlannerExt::prerequisite_places()` exists in [crates/worldwake-ai/src/goal_model.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/goal_model.rs)
   - `PlanningBudget::max_prerequisite_locations` exists in [crates/worldwake-ai/src/budget.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/budget.rs)
   - `combined_relevant_places()` exists and is used inside `search_plan()` / `build_successor()` in [crates/worldwake-ai/src/search.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/search.rs)
   - `agent_tick` already calls `search_plan()` with `RecipeRegistry` rather than a static place slice in [crates/worldwake-ai/src/agent_tick.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/agent_tick.rs)
3. Focused coverage for the prerequisite-aware architecture already exists, so “no tests required” is incorrect. Current focused tests include:
   - `goal_model::tests::prerequisite_places_*`
   - `search::tests::combined_places_*`
   - `search::tests::prune_travel_retains_remote_medicine_branch_for_treat_wounds`
   - `golden_healer_acquires_remote_ground_medicine_for_patient`
4. The remaining gap is diagnostic precision only: `TravelPruningTrace` shows which travel successors were retained/pruned, but it still cannot tell whether the guidance set came from goal-terminal places, prerequisite-only places, or both.
5. This is a single-layer AI traceability ticket inside the planner search trace model. It does not change candidate generation, runtime `agent_tick` failure handling, or authoritative action ordering, but it should add focused assertions because the missing counters encode planner semantics, not just cosmetic formatting.
6. Mismatch + correction: the ticket previously described implementing prerequisite-aware search itself. That work is already present. Scope is corrected to adding and verifying `combined_places_count` and `prerequisite_places_count` on `SearchExpansionSummary`, plus any minimal helper needed to compute those values without duplicating planner logic.

## Architecture Check

1. Adding the counters to the existing search-expansion trace is still the cleanest design. The planner already computes combined place guidance; exposing its cardinality on the same trace record keeps the diagnostic model aligned with the actual search boundary instead of introducing a parallel trace type.
2. The implementation should compute counts from the same helper that produces the combined place set, so the trace cannot drift from live planner behavior.
3. No backwards-compatibility shims or alias fields. All `SearchExpansionSummary` construction sites must populate the canonical fields directly.

## Verification Layers

1. `SearchExpansionSummary` exposes the new counts and remains `Clone + Debug` -> focused unit test in `decision_trace.rs`
2. Search expansion summaries record the combined/prerequisite counts from the live planner guidance set -> focused search test with tracing enabled
3. Counts distinguish pure terminal guidance from mixed terminal+prerequisite guidance -> focused search test on a remote medicine `TreatWounds` branch
4. Single-layer ticket: action trace, event-log ordering, and authoritative world-state mapping are not the contract here because no authoritative behavior changes.

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

- `combined_places_count`: length of the deduplicated combined guidance set used by the node
- `prerequisite_places_count`: number of prerequisite-only places that survive deduplication into that combined guidance set

Use the same helper that feeds pruning/heuristics so the diagnostic data stays coupled to the actual planner guidance logic.

### 3. Update all `SearchExpansionSummary` construction sites

Ensure any existing construction of `SearchExpansionSummary` (including value tests in `decision_trace.rs`) includes the two new fields.

## Files to Touch

- `crates/worldwake-ai/src/decision_trace.rs` (modify — struct definition)
- `crates/worldwake-ai/src/search.rs` (modify — populate new fields in expansion summary construction and/or add a tiny shared helper for count derivation)

## Out of Scope

- `TravelPruningTrace` changes — existing struct is unchanged
- `DecisionTraceSink` API changes — no new query methods
- `dump_agent()` formatting changes — the new fields will appear in `Debug` output automatically
- Any further prerequisite-aware heuristic redesign; the current combined-place architecture already matches the spec direction
- `goal_model.rs` prerequisite-place semantics
- `agent_tick.rs` search call flow
- Golden scenario expansion unless focused coverage proves insufficient

## Acceptance Criteria

### Tests That Must Pass

1. `SearchExpansionSummary` compiles with the new fields and existing derives still hold
2. When tracing is enabled, expansion summaries contain correct `combined_places_count` and `prerequisite_places_count` values for both mixed-guidance and terminal-only cases
3. Existing suite: `cargo test -p worldwake-ai`
4. Existing suite: `cargo clippy --workspace`

### Invariants

1. For goals with empty `prerequisite_places()`, `prerequisite_places_count == 0` and `combined_places_count` equals the deduplicated goal-relevant place set used by the heuristic
2. `SearchExpansionSummary` remains `Clone + Debug`
3. Tracing remains opt-in and zero-cost when disabled

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/decision_trace.rs` — extend the existing value/Debug test so the new fields are covered as part of the trace data contract
2. `crates/worldwake-ai/src/search.rs` — assert expansion summaries report mixed terminal+prerequisite counts for remote-medicine `TreatWounds`
3. `crates/worldwake-ai/src/search.rs` — assert expansion summaries report zero prerequisite places for a terminal-only consume case

### Commands

1. `cargo test -p worldwake-ai decision_trace::tests::expansion_summary_default_and_debug_format search::tests::search_expansion_summaries_collected_when_tracing_enabled search::tests::search_expansion_summary_counts_prerequisite_places_for_remote_treat_wounds`
2. `cargo test -p worldwake-ai`
3. `cargo clippy --workspace`

## Outcome

- Completed: 2026-03-21
- Actual changes:
  - Added `combined_places_count` and `prerequisite_places_count` to `SearchExpansionSummary`
  - Computed those counters from the same combined-place helper used by search pruning and heuristic evaluation, keeping trace data coupled to live planner behavior
  - Strengthened focused tests in `decision_trace.rs` and `search.rs` to cover both terminal-only and mixed terminal+prerequisite guidance
- Deviations from original plan:
  - No planner-architecture work was needed because prerequisite-aware search, budget capping, and combined-place guidance were already implemented before this ticket
  - Added a minimal wound-capable field to the `search.rs` test belief view so the remote-`TreatWounds` trace assertion exercises a real unsatisfied care goal
- Verification results:
  - `cargo test -p worldwake-ai decision_trace::tests::expansion_summary_default_and_debug_format`
  - `cargo test -p worldwake-ai combined_places`
  - `cargo test -p worldwake-ai expansion_summary`
  - `cargo test -p worldwake-ai`
  - `cargo clippy --workspace --all-targets -- -D warnings`
