# E16DPOLPLAN-027: Search Expansion Trace for Plan Search Debuggability

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes — `decision_trace.rs`, `search.rs`
**Deps**: None (extends existing opt-in trace architecture from S08)

## Problem

The plan search (`search_plan` in `search.rs`) is a black box between input candidates and output plan. The existing `PlanSearchOutcome` records *what* plan was found but not *why* the search chose it over alternatives. When a multi-step plan (e.g., Bribe + DeclareSupport) is expected but the search returns a shallower fallback (e.g., DeclareSupport ProgressBarrier), there is no structured way to determine the cause without adding temporary `eprintln!` instrumentation inside the search loop.

This violates Principle 27 (Debuggability Is a Product Feature): "Why did the planner not find plan X?" must be answerable from structured trace data, not guessed by developers.

### Concrete failure case (E16DPOLPLAN-010)

The bribe golden test expected the search to find a 2-step Bribe(B) → DeclareSupport plan (GoalSatisfied) but got a 1-step DeclareSupport plan (ProgressBarrier). Root cause: beam_width=8 truncated the Bribe(B) successor node at depth 0 because the prototype world's adjacency graph created 11 equal-cost candidates. Diagnosing this required manually instrumenting the search loop with `eprintln!` — a multi-hour process that a per-expansion trace summary would have resolved in minutes.

## Assumption Reassessment (2026-03-18)

1. `PlanSearchOutcome` in `decision_trace.rs` records `Found { steps, terminal_kind }`, `BudgetExhausted`, `FrontierExhausted`, `Unsupported` — confirmed, no per-expansion data
2. `search_plan` in `search.rs` already accepts `Option<&mut Vec<BindingRejection>>` for trace collection — confirmed, the opt-in pattern exists
3. `PlanAttemptTrace` records `goal`, `outcome`, `binding_rejections` — confirmed, expansion-level data is absent
4. Existing traces are opt-in and zero-cost when disabled — confirmed, new trace must follow this pattern

## Architecture Check

1. **Extends existing trace architecture** — adds a new field to `PlanAttemptTrace` rather than introducing a parallel tracing mechanism. The opt-in pattern (trace data collected only when a mutable collector is passed) is already established by `binding_rejections`.
2. **No backwards-compatibility shims** — adds an optional field to an existing struct. Existing code that doesn't collect traces is unaffected.
3. **Minimal search hot-path impact** — per-expansion summary is 3-4 integer counters accumulated during the loop. When tracing is disabled (the common case), the cost is a single `Option::is_some()` check per expansion.
4. **Principle 27 alignment** — makes "Why didn't the search find plan X?" answerable from structured data: candidate count, beam survivors, terminal kinds per depth, and total expansions used.

## What to Change

### 1. Add `SearchExpansionSummary` to `decision_trace.rs`

```rust
/// Per-expansion summary recorded during plan search.
#[derive(Clone, Debug)]
pub struct SearchExpansionSummary {
    /// Depth (number of steps already in the node being expanded).
    pub depth: u8,
    /// Total search candidates generated at this expansion.
    pub candidates_generated: u16,
    /// Candidates for which `build_successor` returned `None`.
    pub candidates_skipped: u16,
    /// Terminal successors found (GoalSatisfied, ProgressBarrier, CombatCommitment).
    pub terminal_successors: u16,
    /// Non-terminal successors before beam truncation.
    pub non_terminal_before_beam: u16,
    /// Non-terminal successors after beam truncation (pushed to frontier).
    pub non_terminal_after_beam: u16,
    /// Whether a GoalSatisfied terminal was found at this expansion
    /// (search returns immediately in this case).
    pub found_goal_satisfied: bool,
}
```

### 2. Add `expansion_summaries` field to `PlanAttemptTrace`

```rust
pub struct PlanAttemptTrace {
    pub goal: GoalKey,
    pub outcome: PlanSearchOutcome,
    pub binding_rejections: Vec<BindingRejection>,
    /// Per-expansion summaries. Empty when tracing is disabled.
    pub expansion_summaries: Vec<SearchExpansionSummary>,
}
```

### 3. Extend `search_plan` to collect expansion summaries

Add an `Option<&mut Vec<SearchExpansionSummary>>` parameter (or reuse the existing trace parameter pattern). Accumulate one `SearchExpansionSummary` per expansion iteration when the collector is `Some`. The counters are derived from values already computed in the loop (candidate vec lengths, terminal_successors vec length, successors vec length before/after truncate).

### 4. Wire through `agent_tick.rs`

Pass the expansion collector from the `PlanAttemptTrace` builder into `search_plan` when decision tracing is enabled.

### 5. Extend `dump_agent` / `summary()` for human-readable output

Add a compact one-line-per-expansion format to the decision trace dump, e.g.:
```
  search expansion d=0: 12 candidates, 1 skipped, 1 terminal (0 satisfied), 11→8 beam
  search expansion d=1: 12 candidates, 0 skipped, 1 terminal (1 satisfied), 11→8 beam
```

## Files to Touch

- `crates/worldwake-ai/src/decision_trace.rs` (modify — add `SearchExpansionSummary`, extend `PlanAttemptTrace`)
- `crates/worldwake-ai/src/search.rs` (modify — collect expansion summaries in `search_plan`)
- `crates/worldwake-ai/src/agent_tick.rs` (modify — wire trace collector to search call)

## Out of Scope

- Per-candidate detailed state logging (entity positions, commodity quantities) — too expensive for structured traces; use `eprintln` for one-off deep dives
- Search heuristic tuning or beam_width changes — separate concern
- Modifying `ActionTraceSink` — action execution tracing is already adequate

## Acceptance Criteria

### Tests That Must Pass

1. `search_expansion_summaries_collected_when_tracing_enabled` — search with tracing produces non-empty expansion summaries with correct depth progression
2. `search_expansion_summaries_empty_when_tracing_disabled` — search without tracing produces empty expansion summaries (zero-cost path)
3. `beam_truncation_visible_in_expansion_summary` — expansion summary shows `non_terminal_before_beam > non_terminal_after_beam` when beam truncation occurs
4. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. Opt-in, zero-cost when disabled — no allocation or computation when trace collector is None
2. Existing `PlanAttemptTrace` consumers unaffected — new field is additive
3. Search behavior unchanged — trace collection must not alter candidate ordering, beam truncation, or terminal selection

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/search.rs` (mod tests) — `search_expansion_summaries_collected_when_tracing_enabled`
2. `crates/worldwake-ai/src/search.rs` (mod tests) — `search_expansion_summaries_empty_when_tracing_disabled`
3. `crates/worldwake-ai/src/search.rs` (mod tests) — `beam_truncation_visible_in_expansion_summary`
4. `crates/worldwake-ai/src/decision_trace.rs` (mod tests) — `expansion_summary_default_and_debug_format`

### Commands

1. `cargo test -p worldwake-ai search_expansion`
2. `cargo test -p worldwake-ai`
3. `cargo clippy --workspace`
