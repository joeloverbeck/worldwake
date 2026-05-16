# S145PLASUBHAR-002: Strategic budget exhaustion trace surface

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — new trace type in `decision_trace.rs`, new optional field on `PlanAttemptTrace`, strategic-search instrumentation, observer Section 9 rendering
**Deps**: archive/tickets/S145PLASUBHAR-001.md

## Problem

Before this ticket, when the strategic search at `crates/worldwake-ai/src/search/strategic.rs:124-131` exhausted its budget, it `break`ed and returned `None`. The caller in `crates/worldwake-ai/src/agent_tick/planning.rs:2551` recorded a `PlanAttemptTrace` with no provenance about how many stages were attempted or how much of the budget was consumed. An operator inspecting tactical thrash could only infer "the chain probably busted the strategic budget" from indirect signals. Per S145 D2 and Design Goal 2, the strategic-search outcome needed to be diagnosable per attempt with stage-count and used-vs-total budget so observer Section 9 (Budget Exhaustion Snapshots) and S144's aggregator can attribute exhaustion to chain depth rather than guessing.

## Assumption Reassessment (2026-05-16)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. `PlanAttemptTrace` exists at `crates/worldwake-ai/src/decision_trace.rs:1121` deriving `Clone, Debug` (no `Serialize` — runtime-only trace). Sibling type `StrategicStepTrace` at `:1142-1147` derives `Clone, Debug, Eq, PartialEq`. The new `StrategicBudgetTrace` follows the parent `PlanAttemptTrace` derive shape (`Clone, Debug`) per S145 reassessment finding M3.
2. Five construction sites of `PlanAttemptTrace` exist workspace-wide: `crates/worldwake-cli/src/bin/observer.rs:5497`, `crates/worldwake-ai/src/scenario_diagnostics/aggregator.rs:663`, `:695`, `:972`, and `crates/worldwake-ai/src/agent_tick/planning.rs:2551`. None use spread syntax; each enumerates fields explicitly. All five sites add `strategic_budget: None` as the trivial default for the new optional field.
3. Shared abstraction boundary: `PlanAttemptTrace` is the per-plan-search audit record consumed by both the observer binary (Section 9 rendering at `crates/worldwake-cli/src/bin/observer.rs:1076`) and `worldwake-ai`'s scenario diagnostics aggregator. Adding an `Option<StrategicBudgetTrace>` field is structurally additive and preserves the existing dual-consumer pattern (FND-26: read-only observability, no system-to-system mutation).
4. Observer Section 9 (Budget Exhaustion Snapshots) at `crates/worldwake-cli/src/bin/observer.rs:1076` already renders `max_prerequisite_locations` (line 1134) and per-snapshot search metrics (lines 1108-1118); the new `StrategicBudgetTrace` render lands inside this section's existing per-snapshot block. S145 D2 (reassessment finding I2) corrects the original "Section 7" mis-citation to Section 9.

## Architecture Check

1. The trace is `Option<_>` because not every plan attempt enters the two-phase strategic path (`strategic::plan` may early-return at `strategic.rs:80` or `:99-104`); `None` correctly signals "strategic phase did not run" while `Some { exhausted: false }` signals "strategic phase succeeded" and `Some { exhausted: true }` signals the actual failure case Design Goal 2 targets.
2. Populating the trace at strategic-search-internal hook points (`strategic.rs:124-131`) and plumbing it back through `plan()`'s return type, rather than mutating a sink trait, preserves `strategic::plan` as a pure function. The trace is constructed once at the caller (`agent_tick/planning.rs:2551`) using strategic-side outcome data — symmetric with how `expansion_summaries` already flow through `PlanAttemptTrace`.

## Verified Layers

1. Trace fields populated correctly on strategic budget exhaustion → focused unit test in `crates/worldwake-ai/src/search/strategic.rs` `#[cfg(test)]` module asserting `StrategicBudgetTrace { exhausted: true, ... }` when expansion count reaches the budget cap.
2. Observer Section 9 rendering of the strategic-budget trace → snapshot-style test in observer test scaffolding (or text comparison) that the rendered output contains the stage-count and used/total budget lines.
3. Single planner-instrumentation ticket; the trace is captured at the decision-trace layer (FND-29 debuggability). Action and event-log layers do not observe strategic-search outcome — this is the strongest available surface. Verification Layer 6 (single-layer rationale): observability extensions surface at the decision trace; downstream action/event observability is not relevant because no authoritative state mutates.

## Landed Changes

### 1. Defined `StrategicBudgetTrace` in `decision_trace.rs`

In `crates/worldwake-ai/src/decision_trace.rs`, this ticket added a new type alongside `StrategicStepTrace`:

```rust
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StrategicBudgetTrace {
    pub stages_count: u16,
    pub budget_total: u32,
    pub budget_used: u32,
    pub exhausted: bool,
}
```

The type is re-exported from `crates/worldwake-ai/src/lib.rs` alongside the existing trace types.

### 2. Added optional field to `PlanAttemptTrace`

In the same file, `PlanAttemptTrace` now carries:

```rust
pub strategic_budget: Option<StrategicBudgetTrace>,
```

### 3. Plumbed the trace through strategic search

In `crates/worldwake-ai/src/search/strategic.rs`, the existing `plan(...) -> Option<StrategicPlan>` wrapper remains for current callers and tests. A traced sibling, `plan_with_budget_trace(...) -> StrategicSearchResult`, returns both the existing plan result and `Option<StrategicBudgetTrace>`. The trace populates `stages_count` from `stages.len()`, `budget_total` from `execution_budget.strategic_budget_for_stages(stages.len())`, `budget_used` from the local `expansions` counter, and `exhausted` from the strategic loop's budget-break condition.

Early-return paths that bypass the expansion loop keep `strategic_budget: None`, preserving the distinction between "strategic stage-budgeted search did not run" and "strategic stage-budgeted search ran and succeeded or exhausted."

### 4. Updated `PlanAttemptTrace` construction sites

The live constructor sweep found the five ticket-listed sites plus two same-shape test/forensics harness sites. All now set `strategic_budget: None` except the `agent_tick/planning.rs` conversion path, which preserves the value from `SearchTraceMetadata`.

- `crates/worldwake-cli/src/bin/observer.rs:5497`
- `crates/worldwake-ai/src/scenario_diagnostics/aggregator.rs:663`
- `crates/worldwake-ai/src/scenario_diagnostics/aggregator.rs:695`
- `crates/worldwake-ai/src/scenario_diagnostics/aggregator.rs:972`
- `crates/worldwake-ai/src/agent_tick/planning.rs:2551` (populated from strategic-search outcome)
- `crates/worldwake-ai/src/survival_forensics.rs`
- `crates/worldwake-ai/tests/golden_harness/survival_forensics_assertions.rs`

### 5. Rendered in observer Section 9

In `crates/worldwake-cli/src/bin/observer.rs` Section 9 (Budget Exhaustion Snapshots), each snapshot now renders `StrategicBudgetTrace` fields when present, after the existing Search metrics block and before Planner configuration:

Rendered format:

```
**Strategic budget**:
- Stages attempted: <stages_count>
- Budget used / total: <budget_used> / <budget_total>
- Exhausted: <true | false>
```

## Landed Files

- `crates/worldwake-ai/src/decision_trace.rs` (modify — new type, new field on `PlanAttemptTrace`)
- `crates/worldwake-ai/src/lib.rs` (modify — re-export `StrategicBudgetTrace`)
- `crates/worldwake-ai/src/search/strategic.rs` (modify — traced sibling return path, populate strategic budget trace)
- `crates/worldwake-ai/src/search/mod.rs` (modify — carry strategic budget trace through `SearchTraceMetadata`)
- `crates/worldwake-ai/src/agent_tick/planning.rs` (modify — construct trace from strategic outcome at PlanAttemptTrace site 2551)
- `crates/worldwake-ai/src/scenario_diagnostics/aggregator.rs` (modify — add `strategic_budget: None` at construction sites 663, 695, 972)
- `crates/worldwake-cli/src/bin/observer.rs` (modify — add `strategic_budget: None` at construction site 5497 and render in Section 9)
- `crates/worldwake-ai/src/survival_forensics.rs` (modify — constructor fallout)
- `crates/worldwake-ai/tests/golden_harness/survival_forensics_assertions.rs` (modify — constructor fallout)

## Out of Scope

- No change to the strategic-search return type's success path semantics — only adds trace data alongside.
- No typed `BudgetExhausted` strategic terminal — Design Goal 2 reassessed (M4) explicitly defers this to S149.
- No `PlanningMetrics` (S144) aggregator extension to roll up strategic-budget exhaustion across attempts — S145 only adds the per-attempt trace; future S144 extension is a separate spec.
- No `PlanningStateCacheCounters` work — that is S145PLASUBHAR-003.

## Acceptance Result

### Passed Tests

1. Added focused unit coverage in `crates/worldwake-ai/src/search/strategic.rs` for the successful strategic stage-search path: a one-stage remote patrol search returns `StrategicBudgetTrace { exhausted: false, stages_count: 1, budget_total: 6, budget_used: 1 }`.
2. Added focused unit coverage in `crates/worldwake-ai/src/search/strategic.rs` for the exhausted trace construction helper: `StrategicBudgetTrace { exhausted: true, stages_count: 5, budget_total: 30, budget_used: 30 }`.
3. Added observer binary coverage proving Section 9 renders stage count, used/total budget, and exhausted state when a budget-exhaustion snapshot carries `StrategicBudgetTrace`.
4. Existing suite: `scripts/verify.sh` passed, including `cargo test --workspace`.

### Invariants

1. `StrategicBudgetTrace.budget_total == ExecutionBudget::strategic_budget_for_stages(stages_count)` for populated stage-search traces.
2. `StrategicBudgetTrace.budget_used <= StrategicBudgetTrace.budget_total` because the trace records the bounded strategic expansion counter.
3. `PlanAttemptTrace.strategic_budget.is_none()` when the strategic stage-budgeted expansion loop did not run; the search pipeline preserves `Some(...)` from `SearchTraceMetadata` when that loop did run.

## Test Plan Result

### Added/Modified Tests

1. `crates/worldwake-ai/src/search/strategic.rs` (modify, `#[cfg(test)]` module) — added tests for successful strategic budget trace preservation and exhausted trace construction.
2. `crates/worldwake-ai/src/agent_tick/planning.rs` — extended `plan_search_trace_converts_two_phase_trace_metadata` so `PlanAttemptTrace` preserves `strategic_budget` from `SearchTraceMetadata`.
3. `crates/worldwake-cli/src/bin/observer.rs` — added `budget_exhaustion_snapshot_renders_strategic_budget_trace`.

### Passed Commands

1. Passed `cargo test -p worldwake-ai --lib search::strategic::tests::strategic_budget_trace`.
2. Passed `cargo test -p worldwake-cli --bin observer tests::budget_exhaustion_snapshot_renders_strategic_budget_trace -- --exact`.
3. Passed `cargo test -p worldwake-ai`.
4. Passed `cargo test -p worldwake-cli --bin observer`.
5. Passed `scripts/verify.sh`.

## Outcome

Completed on 2026-05-16.

- Added `StrategicBudgetTrace` and `PlanAttemptTrace.strategic_budget`.
- Added `strategic::plan_with_budget_trace` while preserving the existing `strategic::plan` wrapper.
- Carried the strategic budget trace through `SearchTraceMetadata` into `PlanAttemptTrace`.
- Rendered strategic budget provenance in observer Section 9 budget-exhaustion snapshots.
- Updated all live `PlanAttemptTrace` construction sites, including same-domain forensics and golden-harness constructors discovered by the broad suite.

## Deviations

- The public-private strategic search API stayed backward-compatible inside the crate by adding a traced sibling function rather than changing the existing private `plan(...) -> Option<StrategicPlan>` wrapper. There is still no legacy formula or dual authoritative budget path.
- The exhausted-path focused proof uses the strategic trace-construction helper plus full pipeline/observer propagation tests instead of a brittle synthetic full strategic-search fixture. The successful stage-search path is covered through `plan_with_budget_trace`, and the full workspace wrapper proves the carrier compiles and propagates across all current consumers.
- A false-start exact selector, `cargo test -p worldwake-ai --lib search::strategic::tests::strategic_budget_trace -- --exact`, ran zero tests and is not counted as proof.

## Verification Result

- Passed `cargo test -p worldwake-ai --lib search::strategic::tests::strategic_budget_trace`.
- Passed `cargo test -p worldwake-cli --bin observer tests::budget_exhaustion_snapshot_renders_strategic_budget_trace -- --exact`.
- Passed `cargo test -p worldwake-ai`.
- Passed `cargo test -p worldwake-cli --bin observer`.
- Passed `scripts/verify.sh`.
