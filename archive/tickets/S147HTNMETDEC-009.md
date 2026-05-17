# S147HTNMETDEC-009: MethodPlanAttemptTrace, PlanAttemptTrace.method_trace, PlanningMetrics.method_usage

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — extends `PlanAttemptTrace` with `method_trace`, carries selected-method metadata from strategic planning, and extends `PlanningMetrics` with `method_usage`.
**Deps**: `archive/tickets/S147HTNMETDEC-004.md` (MethodFailureMode), `archive/tickets/S147HTNMETDEC-008.md` (planner integration emits method_trace), `archive/tickets/S147HTNMETDEC-012.md` (canonical recipe-input method preconditions)

## Problem

S147 D5 surfaces method choice and decomposition through `PlanAttemptTrace.method_trace`. Per FND-29, debuggability is a product feature. Without this trace surface, the observer (ticket 010) and scenario diagnostics cannot answer which method the planner selected for an attempt, which subgoals were decomposed, or how often the planner used HTN methods versus flat fallback.

## Assumption Reassessment (2026-05-17)

1. `PlanAttemptTrace` in `crates/worldwake-ai/src/decision_trace.rs` is an in-memory trace model, not currently a serde-derived persistence surface. The ticket's originally drafted serde round-trip tests were corrected to focused trace-struct tests plus existing diagnostics serde coverage.
2. The runtime trace handoff boundary is `SearchTraceMetadata` from `crates/worldwake-ai/src/search/mod.rs` into `agent_tick/planning.rs::plan_search_result_to_trace`.
3. Ticket 008's selected method lives in `search/strategic.rs` during stage construction. This ticket carries the selected method's id, subgoal kinds, and motive score back through `StrategicSearchResult` and `SearchTraceMetadata`.
4. Subgoal execution outcomes are not observed at method-selection time. This ticket records selected method subgoals as `Pending`; later execution/golden work can add success/failure attribution only where the action lifecycle actually proves it.
5. `PlanningMetrics` is a serde surface through `ScenarioDiagnosticsReport`, so `MethodUsageCounts` is serde-derived and included in both the report and CLI diagnostics JSON conversion.

## Architecture Check

1. `method_trace: Option<MethodPlanAttemptTrace>` is absent for flat-GOAP fallback and present when strategic planning actually used selected method stages. No synthetic "no method" trace is emitted.
2. Method trace data flows through the existing search trace metadata path instead of adding a side channel.
3. `PlanningMetrics.method_usage` keys counts by `Option<MethodSchemaId>`, with `None` aggregating flat fallback attempts.
4. No backwards-compatibility shims or save-version changes were added.

## Verified Layers

1. `MethodPlanAttemptTrace` stores selected method id, pending subgoal summaries, and motive score -> focused decision-trace tests.
2. `PlanAttemptTrace.method_trace: None` represents flat fallback -> focused decision-trace test.
3. Strategic planner selected methods produce a method trace -> existing strategic method-selection test now asserts method trace id.
4. `PlanningMetrics.method_usage` aggregates selected, fallback, and failure counts -> focused scenario-diagnostics aggregator test.
5. Construction-site rollout is complete -> `cargo build --workspace --all-targets`.
6. Workspace lint gate is clean -> `cargo clippy --workspace --all-targets -- -D warnings`.

## Landed Changes

### 1. Decision trace method surface

`crates/worldwake-ai/src/decision_trace.rs` now defines `MethodPlanAttemptTrace`, `SubgoalAttemptResult`, `SubgoalAttemptKind`, and `SubgoalAttemptOutcome`, and `PlanAttemptTrace` carries `method_trace: Option<MethodPlanAttemptTrace>`.

### 2. Planner trace propagation

`crates/worldwake-ai/src/search/strategic.rs` records selected-method metadata when method stages are used. `crates/worldwake-ai/src/search/mod.rs` carries it in `SearchTraceMetadata`, and `crates/worldwake-ai/src/agent_tick/planning.rs` copies it into `PlanAttemptTrace`.

### 3. Diagnostics aggregation

`PlanningMetrics` now includes `method_usage: BTreeMap<Option<MethodSchemaId>, MethodUsageCounts>`. `scenario_diagnostics/aggregator.rs` increments selected, fallback, and failure counts per plan trace. `crates/worldwake-cli/src/diagnostics_json.rs` preserves the new field through CLI diagnostics JSON conversion.

### 4. Construction-site rollout

Existing `PlanAttemptTrace` literals in AI tests, survival forensics, golden harness helpers, and observer tests now set `method_trace: None` where they intentionally represent flat fallback or method-irrelevant fixtures.

## Landed Files

- `crates/worldwake-ai/src/decision_trace.rs`
- `crates/worldwake-ai/src/search/strategic.rs`
- `crates/worldwake-ai/src/search/mod.rs`
- `crates/worldwake-ai/src/agent_tick/planning.rs`
- `crates/worldwake-ai/src/scenario_diagnostics/mod.rs`
- `crates/worldwake-ai/src/scenario_diagnostics/aggregator.rs`
- `crates/worldwake-ai/src/survival_forensics.rs`
- `crates/worldwake-ai/tests/golden_harness/survival_forensics_assertions.rs`
- `crates/worldwake-cli/src/bin/observer.rs`
- `crates/worldwake-cli/src/diagnostics_json.rs`

## Outcome

Completed: 2026-05-17.

Selected HTN methods now appear on plan-attempt traces, and scenario diagnostics aggregate method usage. Flat-GOAP attempts remain represented by `method_trace: None`.

## Deviations

- The drafted serde round-trip tests for `PlanAttemptTrace.method_trace` were not implemented because `PlanAttemptTrace` and its nested search-summary types are not serde-derived today. The serde proof remains on `ScenarioDiagnosticsReport`, which is the existing persisted/reporting diagnostics surface.
- Subgoal outcomes are recorded as `Pending` at selection time. This ticket does not infer success or failure from later action execution.

## Out of Scope

- Observer section rendering of `method_trace` (ticket 010).
- Golden end-to-end trace assertions (ticket 011).
- Emitting authoritative `Discrepancy::MethodFailure` from action handlers.

## Acceptance Result

### Tests Passed

1. `cargo test -p worldwake-ai --lib decision_trace`
2. `cargo test -p worldwake-ai --lib scenario_diagnostics`
3. `cargo test -p worldwake-ai --lib search::strategic`
4. `cargo build --workspace --all-targets`
5. `cargo clippy --workspace --all-targets -- -D warnings`

### Invariants

1. `method_trace: None` represents flat-GOAP fallback; no synthetic fallback method is created.
2. Selected method attempts carry `Some(MethodSchemaId)` and pending subgoal summaries.
3. `PlanningMetrics.method_usage` is keyed by `Option<MethodSchemaId>`, so the `None` slot aggregates fallback attempts.
4. No save-format version bump or compatibility shim was introduced.

## Verification Result

All focused tests, the workspace all-target build, and the workspace all-target clippy gate passed. The build and clippy commands reported only Cargo's upstream future-incompatibility warning for `ashpd v0.8.1`.

## Test Plan Result

### Added/Modified Tests

1. `crates/worldwake-ai/src/decision_trace.rs` — method trace selected-method and flat-fallback tests.
2. `crates/worldwake-ai/src/scenario_diagnostics/aggregator.rs` — method usage selected/fallback/failure aggregation test.
3. `crates/worldwake-ai/src/search/strategic.rs` — existing method-selection test now asserts the selected method trace id.

### Verification Commands

1. `cargo test -p worldwake-ai --lib decision_trace`
2. `cargo test -p worldwake-ai --lib scenario_diagnostics`
3. `cargo test -p worldwake-ai --lib search::strategic`
4. `cargo build --workspace --all-targets`
5. `cargo clippy --workspace --all-targets -- -D warnings`
