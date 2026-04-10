# S85OBSBEHENR-002: Frontier-exhaustion rejection reasons in observer

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: None
**Deps**: S85 (Observer Behavioral Enrichment)

## Problem

When a plan frontier-exhausts at depth 0, the observer shows only `"frontier-exhausted (1 expansion, 0 depth)"` with no explanation of why operators did not match. This makes it impossible to diagnose whether the issue is zero candidates, all candidates pruned by beam, or some other cause.

## Assumption Reassessment (2026-04-10)

1. `PlanSearchOutcome::FrontierExhausted { expansions_used }` at `decision_trace.rs:881`. `PlanAttemptTrace` has `expansion_summaries: Vec<SearchExpansionSummary>` at `decision_trace.rs:865`. `SearchExpansionSummary` has `depth: u8`, `candidates_generated: u16`, `candidates_skipped: u16`, `terminal_successors: u16`, `non_terminal_after_beam: u16` (among other fields) at `decision_trace.rs:764-783`. Observer currently renders frontier-exhausted outcomes at `observer.rs:1174-1176` with only `expansions_used` and max depth.
2. S85 spec (Deliverable 2) describes this change. S78 (completed) added frontier-exhausted counting and failed-plan tables.
3. Single-layer ticket: observer-only formatting of existing trace data. No shared abstraction boundary.

## Architecture Check

1. Reads existing `expansion_summaries` data that is already collected during planning — no new trace infrastructure needed. The enhancement is purely formatting: when `expansions_used <= 1` and the outcome is `FrontierExhausted`, extract depth-0 summary fields to produce a human-readable reason string.
2. No backwards-compatibility aliasing or shims introduced.

## Verification Layers

1. Depth-0 frontier-exhaustion reason displayed → focused unit test with mock trace containing depth-0 expansion summary
2. Non-depth-0 frontier-exhaustion unchanged → focused unit test with multi-expansion trace
3. Single-layer observer-only ticket; no action/planning/event-log layer mapping applicable.

## What to Change

### 1. Enhance frontier-exhausted rendering

In the observer's failed-plan detail rendering (around `observer.rs:1174`), when the outcome is `FrontierExhausted` and `expansions_used <= 1`, check whether `expansion_summaries` has a depth-0 entry. If so, format a reason string:

- If `candidates_generated == 0`: `"frontier-exhausted at depth 0: 0 candidates generated"`
- If `candidates_generated > 0` and `non_terminal_after_beam == 0` and `terminal_successors == 0`: `"frontier-exhausted at depth 0: {n} candidates generated, all pruned by beam"`
- If `candidates_generated > 0` and `candidates_skipped == candidates_generated`: `"frontier-exhausted at depth 0: {n} candidates generated, all skipped (build_successor returned None)"`
- Otherwise: `"frontier-exhausted at depth 0: {gen} generated, {skipped} skipped, {term} terminal, {beam} after beam"`

### 2. Add unit test

Add a test that constructs a `PlanAttemptTrace` with `FrontierExhausted { expansions_used: 1 }` and a depth-0 `SearchExpansionSummary` with zero candidates, verifies the output contains the enriched reason. Add a second case with candidates generated but all pruned.

## Files to Touch

- `crates/worldwake-cli/src/bin/observer.rs` (modify)

## Out of Scope

- Modifying simulation behavior or AI decision-making
- Changing how `SearchExpansionSummary` is collected during planning
- Adding new trace fields to `PlanAttemptTrace`
- Enriching non-depth-0 frontier-exhaustion (future work)

## Acceptance Criteria

### Tests That Must Pass

1. New test: depth-0 frontier-exhaustion with 0 candidates shows `"0 candidates generated"`
2. New test: depth-0 frontier-exhaustion with candidates all pruned shows `"all pruned by beam"`
3. Existing suite: `cargo test -p worldwake-cli`

### Invariants

1. Observer remains read-only — no mutation of trace or world state
2. Non-depth-0 frontier-exhaustion output is unchanged

## Test Plan

### New/Modified Tests

1. `crates/worldwake-cli/src/bin/observer.rs` (inline test) — verifies enriched depth-0 frontier-exhaustion reason formatting

### Commands

1. `cargo test -p worldwake-cli`
2. `cargo clippy --workspace --all-targets -- -D warnings`
