# S147HTNMETDEC-009: MethodPlanAttemptTrace, PlanAttemptTrace.method_trace, PlanningMetrics.method_usage

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — extends `PlanAttemptTrace` with `method_trace` field, defines `MethodPlanAttemptTrace`, extends `PlanningMetrics` with `method_usage` aggregation. ~10 construction sites updated workspace-wide.
**Deps**: 004 (MethodFailureMode), 008 (planner integration emits method_trace)

## Problem

S147 D5 surfaces method choice and decomposition through `PlanAttemptTrace.method_trace`. Per FND-29: debuggability is a product feature. Without this trace surface, the observer (ticket 010) and scenario diagnostics cannot answer "which method did the planner select for this attempt, and why did it fail?" The trace also drives the `PlanningMetrics.method_usage` aggregate used in `ScenarioDiagnosticsReport` to surface per-method attempts/selections/fallbacks/failures across an entire scenario run.

## Assumption Reassessment (2026-05-17)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. `PlanAttemptTrace` struct lives at `crates/worldwake-ai/src/decision_trace.rs:1185` with existing fields (goal, opportunity_anchor, outcome, goal_budget, strategic_budget, strategic_plan, tactical_goal, landmarks, target_belief_presence, binding_rejections, expansion_summaries — verified during S147 reassessment). 10 construction sites workspace-wide (verified):
   - Production (worldwake-ai/src/): `agent_tick/planning.rs:2728` (the runtime emit site), `decision_trace.rs:4605, 5046, 5241` (test), `survival_forensics.rs:788` (test), `scenario_diagnostics/aggregator.rs:777, 811, 1158` (test).
   - Tests outside src/: `crates/worldwake-ai/tests/golden_harness/survival_forensics_assertions.rs:236`.
   - Observer test surface: 5 sites in `crates/worldwake-cli/src/bin/observer.rs` (5689, 6717, 6721, 6751, 6794, 6798, 6813, 6826, 6839 — mostly use `..sample_attempt(…)` spread syntax; only `sample_attempt` itself enumerates fields).
2. Spread-syntax check (Step 2 sub-check (d)): observer.rs test sites use `..sample_attempt(…)` spread, so only the `sample_attempt` helper needs `method_trace: None`. The other observer sites inherit through spread. With `Option`'s default being `None`, the rollout is mostly mechanical at the production sites; the 1 production site in `agent_tick/planning.rs` needs an actual method-trace emission when `select_method` returned `Some(_)`.
3. `PlanningMetrics` lives at `crates/worldwake-ai/src/scenario_diagnostics/mod.rs:32` with existing fields (plan_attempts, plan_attempts_by_kind, budget_exhaustion_count, budget_exhaustion_rate, frontier_exhaustion_count, frontier_exhaustion_rate, beam_truncation_ratio, plan_depth, terminal_kind_distribution, heuristic_helpful_action_hit_rate). The `method_usage` field is net-new and aggregates per-method counts from `PlanAttemptTrace.method_trace` across a scenario run.
4. Shared boundary: `PlanAttemptTrace` is the contract between planner emission (the runtime `build_stages` caller at `agent_tick/planning.rs`) and downstream consumers (observer ticket 010, scenario diagnostics, goldens). The `method_trace` field is `Option<MethodPlanAttemptTrace>` so flat-GOAP fallback records `None` rather than synthesizing a trace.
5. `MethodFailureMode` and `From<&MethodFailureMode> for MethodFailureKind` exist after ticket 004 lands. `MethodPlanAttemptTrace.failure_mode: Option<MethodFailureMode>` carries the rich ai-side payload; the typed `Discrepancy::MethodFailure(MethodFailureContext)` (from ticket 002) is the parallel core-side surface.

## Architecture Check

1. `method_trace: Option<MethodPlanAttemptTrace>` makes the trace cleanly absent for flat-GOAP attempts (no method selected) rather than requiring a synthesized "no method" trace. This matches the spec's intent (Non-Goal: "No method-only goals") and avoids over-surfacing on goals without methods.
2. Splitting the failure payload — rich `MethodFailureMode` on the ai-side trace AND minimal `MethodFailureContext` on the core-side `Discrepancy` — preserves the FND-29A authoritative chain (typed channel carries enough provenance) without forcing every core-side blocker-memory consumer to handle the rich ai-side payload (which would create a crate-layer violation).
3. `PlanningMetrics.method_usage: BTreeMap<Option<MethodSchemaId>, MethodUsageCounts>` keys on `Option<MethodSchemaId>` so the `None` slot tracks flat-GOAP fallbacks alongside per-method counts. This is necessary because "method-disabled fallback" is the explicit golden case in ticket 011.
4. No backwards-compatibility shims. `Option<>` defaults to `None`; the new aggregation slot is additive.

## Verification Layers

1. `method_trace` populated when planner selected a method → focused unit test in `decision_trace.rs` constructs a `PlanAttemptTrace` with `method_trace: Some(…)` and asserts serde round-trip.
2. `method_trace` absent when flat-GOAP fallback → unit test asserts `method_trace: None` round-trips correctly.
3. `PlanningMetrics.method_usage` aggregates correctly → focused test in `scenario_diagnostics/aggregator.rs` feeds a stream of attempts with mixed method selection and asserts the per-method counts.
4. Production emission site → ticket 008's planner integration emits the trace correctly; this ticket adds the recording surface and ticket 008 owns the call path. Verify via integration test in ticket 011 (golden e2e) that emission lands.
5. Construction-site rollout complete → `cargo build --workspace --all-targets` succeeds after the field is added.

## What to Change

### 1. Extend `PlanAttemptTrace` with `method_trace` field

Modify `crates/worldwake-ai/src/decision_trace.rs:1185`:

```rust
pub struct PlanAttemptTrace {
    // ... existing fields unchanged ...
    pub method_trace: Option<MethodPlanAttemptTrace>,    // NEW
}
```

### 2. Define `MethodPlanAttemptTrace` and supporting types

In `crates/worldwake-ai/src/decision_trace.rs` (or a new sibling module if the file is already large):

```rust
#[derive(Clone, Debug, Eq, PartialEq)]
#[derive(serde::Serialize, serde::Deserialize)]
pub struct MethodPlanAttemptTrace {
    pub method_id: Option<MethodSchemaId>,    // None = flat GOAP fallback
    pub subgoals_attempted: Vec<SubgoalAttemptResult>,
    pub failure_mode: Option<MethodFailureMode>,
    pub motive_score: u32,                     // 0..=1_000_000 per D3
}

#[derive(Clone, Debug, Eq, PartialEq)]
#[derive(serde::Serialize, serde::Deserialize)]
pub struct SubgoalAttemptResult {
    pub template_index: usize,
    pub kind: SubgoalAttemptKind,
    pub outcome: SubgoalAttemptOutcome,
}

#[derive(Clone, Debug, Eq, PartialEq)]
#[derive(serde::Serialize, serde::Deserialize)]
pub enum SubgoalAttemptKind { /* one variant per SubgoalTemplate per spec D1 */ }

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
#[derive(serde::Serialize, serde::Deserialize)]
pub enum SubgoalAttemptOutcome { Pending, Succeeded, Failed }
```

### 3. Extend `PlanningMetrics` with `method_usage`

Modify `crates/worldwake-ai/src/scenario_diagnostics/mod.rs:32`:

```rust
pub struct PlanningMetrics {
    // ... existing 10 fields unchanged ...
    pub method_usage: BTreeMap<Option<MethodSchemaId>, MethodUsageCounts>,    // NEW
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct MethodUsageCounts {
    pub attempts: u64,
    pub selected_count: u64,    // attempts where method_trace.method_id == this key
    pub fallback_count: u64,    // attempts where None key, i.e., flat GOAP
    pub failure_count: u64,     // attempts where method_trace.failure_mode.is_some()
}
```

Update `aggregator.rs` to populate `method_usage` from `PlanAttemptTrace.method_trace` per attempt.

### 4. Update all PlanAttemptTrace construction sites

10 sites total — most use spread syntax, so only the spread-source needs the new field:

- `crates/worldwake-ai/src/agent_tick/planning.rs:2728` (production site): set `method_trace: select_method_outcome` based on whether a method was selected during this attempt. Requires coordination with ticket 008's call path — the planner branch must thread the selected method's outcome back to the trace emit point.
- `crates/worldwake-ai/src/decision_trace.rs:4605, 5046, 5241` (test sites): add `method_trace: None`.
- `crates/worldwake-ai/src/survival_forensics.rs:788` (test): add `method_trace: None`.
- `crates/worldwake-ai/src/scenario_diagnostics/aggregator.rs:777, 811, 1158` (test sites): add `method_trace: None` (or a fixture-constructed `Some(...)` for the aggregator tests that exercise `method_usage`).
- `crates/worldwake-ai/tests/golden_harness/survival_forensics_assertions.rs:236`: add `method_trace: None`.
- `crates/worldwake-cli/src/bin/observer.rs`: update `sample_attempt(…)` at line 5689 to set `method_trace: None`; the 4+ sites that spread through `..sample_attempt(…)` inherit automatically.

### 5. Focused tests

- `decision_trace::tests::method_plan_attempt_trace_round_trips_through_serde`.
- `decision_trace::tests::plan_attempt_trace_with_method_trace_none_round_trips`.
- `scenario_diagnostics::aggregator::tests::method_usage_counts_selected_fallback_and_failure_correctly`.

## Files to Touch

- `crates/worldwake-ai/src/decision_trace.rs` (modify — add field, define MethodPlanAttemptTrace + SubgoalAttemptResult + SubgoalAttemptKind + SubgoalAttemptOutcome, update 3 test sites)
- `crates/worldwake-ai/src/scenario_diagnostics/mod.rs` (modify — add method_usage field + MethodUsageCounts type)
- `crates/worldwake-ai/src/scenario_diagnostics/aggregator.rs` (modify — populate method_usage; update 3 test sites)
- `crates/worldwake-ai/src/agent_tick/planning.rs` (modify — production emission site at line 2728 records method_trace)
- `crates/worldwake-ai/src/survival_forensics.rs` (modify — 1 test site)
- `crates/worldwake-ai/tests/golden_harness/survival_forensics_assertions.rs` (modify — 1 test site)
- `crates/worldwake-cli/src/bin/observer.rs` (modify — update `sample_attempt` at line 5689; other spread sites inherit)

## Out of Scope

- Observer Section rendering of `method_trace` (ticket 010 — verifies actual observer section number too).
- `Discrepancy::MethodFailure` emission from action handlers — ticket 002 owns the variant; emission from planner failure paths may need wiring in `failure_handling.rs`, but that's part of the planner integration (ticket 008) rather than the trace recording (this ticket).
- Golden coverage of trace correctness — ticket 011.

## Acceptance Criteria

### Tests That Must Pass

1. `decision_trace::tests::method_plan_attempt_trace_round_trips_through_serde`.
2. `decision_trace::tests::plan_attempt_trace_with_method_trace_none_round_trips`.
3. `scenario_diagnostics::aggregator::tests::method_usage_counts_selected_fallback_and_failure_correctly`.
4. All existing `decision_trace`, `scenario_diagnostics`, `survival_forensics`, and observer tests pass after construction-site updates.
5. `cargo build --workspace --all-targets` succeeds (proves field rollout is complete workspace-wide).
6. `cargo clippy --workspace --all-targets -- -D warnings` clean.

### Invariants

1. `method_trace: None` for every flat-GOAP attempt — no synthesized "no method" trace.
2. `MethodPlanAttemptTrace.method_id: None` exactly when the planner fell back to flat GOAP within an attempt that *could* have considered methods (distinguishes from "goal had no methods registered" — that path also records `None`).
3. `PlanningMetrics.method_usage` is keyed by `Option<MethodSchemaId>` so the `None` slot aggregates all flat-GOAP fallbacks.
4. No `SAVE_FORMAT_VERSION` bump — additive `Option<>` field uses serde's natural default.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/decision_trace.rs` inline — new round-trip tests for both Some/None method_trace cases.
2. `crates/worldwake-ai/src/scenario_diagnostics/aggregator.rs` inline — new method_usage aggregation test.

### Commands

1. `cargo test -p worldwake-ai --lib decision_trace`
2. `cargo test -p worldwake-ai --lib scenario_diagnostics`
3. `cargo build --workspace --all-targets`
4. `./scripts/verify.sh`
