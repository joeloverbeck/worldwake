# S145PLASUBHAR-002: Strategic budget exhaustion trace surface

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — new trace type in `decision_trace.rs`, new optional field on `PlanAttemptTrace`, strategic-search instrumentation, observer Section 9 rendering
**Deps**: S145PLASUBHAR-001

## Problem

When the strategic search at `crates/worldwake-ai/src/search/strategic.rs:124-131` exhausts its budget, it `break`s and returns `None`. The caller in `crates/worldwake-ai/src/agent_tick/planning.rs:2551` records a `PlanAttemptTrace` with no provenance about how many stages were attempted or how much of the budget was consumed. An operator inspecting tactical thrash today can only infer "the chain probably busted the strategic budget" from indirect signals. Per S145 D2 and Design Goal 2, the strategic-search outcome must be diagnosable per attempt with stage-count and used-vs-total budget so observer Section 9 (Budget Exhaustion Snapshots) and S144's aggregator can attribute exhaustion to chain depth rather than guessing.

## Assumption Reassessment (2026-05-16)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. `PlanAttemptTrace` exists at `crates/worldwake-ai/src/decision_trace.rs:1121` deriving `Clone, Debug` (no `Serialize` — runtime-only trace). Sibling type `StrategicStepTrace` at `:1142-1147` derives `Clone, Debug, Eq, PartialEq`. The new `StrategicBudgetTrace` follows the parent `PlanAttemptTrace` derive shape (`Clone, Debug`) per S145 reassessment finding M3.
2. Five construction sites of `PlanAttemptTrace` exist workspace-wide: `crates/worldwake-cli/src/bin/observer.rs:5497`, `crates/worldwake-ai/src/scenario_diagnostics/aggregator.rs:663`, `:695`, `:972`, and `crates/worldwake-ai/src/agent_tick/planning.rs:2551`. None use spread syntax; each enumerates fields explicitly. All five sites add `strategic_budget: None` as the trivial default for the new optional field.
3. Shared abstraction boundary: `PlanAttemptTrace` is the per-plan-search audit record consumed by both the observer binary (Section 9 rendering at `crates/worldwake-cli/src/bin/observer.rs:1076`) and `worldwake-ai`'s scenario diagnostics aggregator. Adding an `Option<StrategicBudgetTrace>` field is structurally additive and preserves the existing dual-consumer pattern (FND-26: read-only observability, no system-to-system mutation).
4. Observer Section 9 (Budget Exhaustion Snapshots) at `crates/worldwake-cli/src/bin/observer.rs:1076` already renders `max_prerequisite_locations` (line 1134) and per-snapshot search metrics (lines 1108-1118); the new `StrategicBudgetTrace` render lands inside this section's existing per-snapshot block. S145 D2 (reassessment finding I2) corrects the original "Section 7" mis-citation to Section 9.

## Architecture Check

1. The trace is `Option<_>` because not every plan attempt enters the two-phase strategic path (`strategic::plan` may early-return at `strategic.rs:80` or `:99-104`); `None` correctly signals "strategic phase did not run" while `Some { exhausted: false }` signals "strategic phase succeeded" and `Some { exhausted: true }` signals the actual failure case Design Goal 2 targets.
2. Populating the trace at strategic-search-internal hook points (`strategic.rs:124-131`) and plumbing it back through `plan()`'s return type, rather than mutating a sink trait, preserves `strategic::plan` as a pure function. The trace is constructed once at the caller (`agent_tick/planning.rs:2551`) using strategic-side outcome data — symmetric with how `expansion_summaries` already flow through `PlanAttemptTrace`.

## Verification Layers

1. Trace fields populated correctly on strategic budget exhaustion → focused unit test in `crates/worldwake-ai/src/search/strategic.rs` `#[cfg(test)]` module asserting `StrategicBudgetTrace { exhausted: true, ... }` when expansion count reaches the budget cap.
2. Observer Section 9 rendering of the new trace → snapshot-style test in observer test scaffolding (or text comparison) that the rendered output contains the stage-count and used/total budget lines.
3. Single planner-instrumentation ticket; the trace is captured at the decision-trace layer (FND-29 debuggability). Action and event-log layers do not observe strategic-search outcome — this is the strongest available surface. Verification Layer 6 (single-layer rationale): observability extensions surface at the decision trace; downstream action/event observability is not relevant because no authoritative state mutates.

## What to Change

### 1. Define `StrategicBudgetTrace` in `decision_trace.rs`

In `crates/worldwake-ai/src/decision_trace.rs`, add a new type alongside `StrategicStepTrace`:

```rust
#[derive(Clone, Debug)]
pub struct StrategicBudgetTrace {
    pub stages_count: u16,
    pub budget_total: u32,
    pub budget_used: u32,
    pub exhausted: bool,
}
```

Re-export from `crates/worldwake-ai/src/lib.rs` alongside the existing `StrategicStepTrace` re-export.

### 2. Add optional field to `PlanAttemptTrace`

In the same file, extend the `PlanAttemptTrace` struct (at `:1121`) with:

```rust
pub strategic_budget: Option<StrategicBudgetTrace>,
```

### 3. Plumb the trace through strategic search

In `crates/worldwake-ai/src/search/strategic.rs`, change the public `plan()` return type (or add a sibling function) so the caller in `agent_tick/planning.rs` receives both the `Option<StrategicPlan>` and the populated `StrategicBudgetTrace`. Populate `stages_count` from `stages.len()`, `budget_total` from `execution_budget.strategic_budget_for_stages(stages.len())` (from ticket 001), `budget_used` from the local `expansions` counter, and `exhausted: true` when the `expansions >= search_budget` `break` at `:128-130` fires (otherwise `false`).

For early-return paths in `strategic::plan` that bypass the expansion loop (success at `:80`, `:100`, `:110`; exploration/social-query fallback at `:102-103`), the trace records `stages_count` only when stages were built (post-`:88`); when the function returns before stage construction, the caller records `strategic_budget: None`.

### 4. Update all five `PlanAttemptTrace` construction sites

Add `strategic_budget: None` (or populated `Some(...)` at `agent_tick/planning.rs:2551`) at each enumerated site:

- `crates/worldwake-cli/src/bin/observer.rs:5497`
- `crates/worldwake-ai/src/scenario_diagnostics/aggregator.rs:663`
- `crates/worldwake-ai/src/scenario_diagnostics/aggregator.rs:695`
- `crates/worldwake-ai/src/scenario_diagnostics/aggregator.rs:972`
- `crates/worldwake-ai/src/agent_tick/planning.rs:2551` (populated from strategic-search outcome)

### 5. Render in observer Section 9

In `crates/worldwake-cli/src/bin/observer.rs` Section 9 (Budget Exhaustion Snapshots, starting at line 1076), extend the per-snapshot render to include the `StrategicBudgetTrace` fields when `Some`. The natural placement is after the existing "Search metrics" block (`observer.rs:1108-1118`) and before the "Planner configuration" block (`:1120-1135`).

Render format (mirroring the existing block style):

```
**Strategic budget**:
- Stages attempted: <stages_count>
- Budget used / total: <budget_used> / <budget_total>
- Exhausted: <true | false>
```

## Files to Touch

- `crates/worldwake-ai/src/decision_trace.rs` (modify — new type, new field on `PlanAttemptTrace`)
- `crates/worldwake-ai/src/lib.rs` (modify — re-export `StrategicBudgetTrace`)
- `crates/worldwake-ai/src/search/strategic.rs` (modify — plumb trace through return path, populate at exhaustion hook)
- `crates/worldwake-ai/src/agent_tick/planning.rs` (modify — construct trace from strategic outcome at PlanAttemptTrace site 2551)
- `crates/worldwake-ai/src/scenario_diagnostics/aggregator.rs` (modify — add `strategic_budget: None` at construction sites 663, 695, 972)
- `crates/worldwake-cli/src/bin/observer.rs` (modify — add `strategic_budget: None` at construction site 5497 and render in Section 9)

## Out of Scope

- No change to the strategic-search return type's success path semantics — only adds trace data alongside.
- No typed `BudgetExhausted` strategic terminal — Design Goal 2 reassessed (M4) explicitly defers this to S149.
- No `PlanningMetrics` (S144) aggregator extension to roll up strategic-budget exhaustion across attempts — S145 only adds the per-attempt trace; future S144 extension is a separate spec.
- No `PlanningStateCacheCounters` work — that is S145PLASUBHAR-003.

## Acceptance Criteria

### Tests That Must Pass

1. New focused unit test in `crates/worldwake-ai/src/search/strategic.rs` `#[cfg(test)]` module: build a multi-stage `PlanningSnapshot` whose strategic search exhausts the budget, assert the populated `StrategicBudgetTrace` has `exhausted: true`, `stages_count > 1`, `budget_used == budget_total`.
2. New focused unit test: build a single-stage scenario where strategic search succeeds, assert `StrategicBudgetTrace { exhausted: false, stages_count: 1, .. }` on the returned trace.
3. Existing suite: `cargo test --workspace`.

### Invariants

1. `StrategicBudgetTrace.budget_total == ExecutionBudget::strategic_budget_for_stages(stages_count)` for every populated trace (must equal the formula from ticket 001).
2. `StrategicBudgetTrace.budget_used <= StrategicBudgetTrace.budget_total` always — the expansions counter is monotone and bounded by the budget cap.
3. `PlanAttemptTrace.strategic_budget.is_none()` when the strategic phase did not run (early-return paths before stage construction); `is_some()` whenever the expansion loop was entered.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/search/strategic.rs` (modify, `#[cfg(test)]` module) — two new tests covering the exhausted/success paths for the new trace field.
2. `crates/worldwake-ai/src/agent_tick/planning.rs` (no new tests needed; PlanAttemptTrace shape change is exercised by existing golden coverage that runs through `agent_tick`).

### Commands

1. `cargo test -p worldwake-ai search::strategic`
2. `cargo test -p worldwake-ai`
3. `scripts/verify.sh`
