# S108PERACTBIN-004: Decision trace — `PlannedStepSummary.binding_strictness`

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: Yes — `PlannedStepSummary` gains an optional `binding_strictness: Option<BindingStrictness>` trace field, populated from `ActionDef::binding_strictness` at summary construction time.
**Deps**: archive/tickets/S108PERACTBIN-001.md (needs `BindingStrictness` type exposed from `worldwake-sim`).

## Problem

After T-002 and T-003 land, an `ExactIdentity` request may be refused at dispatch (T-002) or at revalidation (T-003). Operators debugging "why did this step not execute?" need a direct answer in the decision trace, not a reconstruction from the `ActionDef` registry. FND-29 (Debuggability is a Product Feature) requires the dispatch-time classifier be visible in the trace.

This ticket extends `PlannedStepSummary` (`crates/worldwake-ai/src/decision_trace.rs:971`) with an optional `binding_strictness: Option<BindingStrictness>` field, populated from `ActionDef::binding_strictness` at the moment the selected plan is summarized into the decision trace. `PlannedStep` itself (in `planner_ops.rs:814`) is intentionally NOT extended — the authoritative source is always `ActionDef::binding_strictness`; the trace carries a snapshot for inspection (FND-27: derived summaries are caches, never truth).

## Assumption Reassessment (2026-04-18)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. `PlannedStepSummary` is defined at `crates/worldwake-ai/src/decision_trace.rs:971` with fields `action_def_id`, `action_name`, `op_kind`, `targets`, `estimated_ticks`. It is constructed at the plan-summary site(s) in `decision_trace.rs` (grep for `PlannedStepSummary {` during implementation). `SelectedPlanTrace` at line 1022 embeds it via `steps: Vec<PlannedStepSummary>` and `next_step: Option<PlannedStepSummary>`. Existing trace-formatting tests in `decision_trace.rs` will need minor updates to account for the new field's default rendering.
2. The reassessed spec (`specs/S108-per-action-binding-strictness.md`, D6) explicitly targets `PlannedStepSummary` and not `PlannedStep`, resolved during Q3 of the reassess session.
3. Shared abstraction boundary: the decision-trace summary surface consumed by goldens, observer output, and debugging tooling. This ticket adds a derived read-model field; the source of truth remains `ActionDef::binding_strictness`.
4. Not applicable — no failing golden.
5. Not applicable — not a planner- or golden-driven change.
6. AI regression intended layer: decision trace. Focused unit coverage of `PlannedStepSummary` construction is sufficient; goldens in T-005 exercise end-to-end.
7. Not applicable — no ordering claim.
8. Not applicable — no heuristic removal.
9. Not applicable — this ticket is a trace extension, not a failure-surface change.
10. Not applicable.
11. Not applicable.
12. Not applicable.
13. No adjacent contradictions discovered.
14. No mismatch discovered during reassessment.
15. Not applicable.

## Architecture Check

1. Trace surface receives a snapshot of authoritative metadata at the moment of plan summarization. The snapshot is a view, not a stored source of truth (FND-27). The alternative — extending `PlannedStep` in `planner_ops.rs` — was rejected during reassessment Q3 because `PlannedStep` is the authoritative runtime plan structure; embedding a derived classifier there would duplicate `ActionDef::binding_strictness` and create a consistency problem if the classification is ever changed.
2. `Option<BindingStrictness>` — the `None` case is for traces where the `ActionDef` lookup fails (defensive; in practice the def always exists when a plan step is summarized). This is not a shim — it's a representational honesty signal for the trace consumer, consistent with other optional trace fields.

## Verification Layers

1. Summary construction attaches the correct class -> focused unit test on the construction site in `agent_tick/planning.rs`, where `summarize_step` reads the registry and builds `PlannedStepSummary`.
2. Class propagates from `ActionDef::binding_strictness` without re-derivation -> the same constructor test asserts the traced value equals the registered value.
3. This ticket is single-layer (decision trace). No authoritative mutation, no action-trace change, no event-log impact.

## What to Change

### 1. Extend `PlannedStepSummary`

`crates/worldwake-ai/src/decision_trace.rs` at line 971:

```rust
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlannedStepSummary {
    pub action_def_id: ActionDefId,
    pub action_name: String,
    pub op_kind: PlannerOpKind,
    pub targets: Vec<EntityId>,
    pub estimated_ticks: u32,
    pub binding_strictness: Option<BindingStrictness>,   // NEW
}
```

Import `BindingStrictness` from `worldwake_sim`.

### 2. Populate `binding_strictness` at summary construction

Grep `PlannedStepSummary {` in `crates/worldwake-ai/src/decision_trace.rs` and any other file that constructs it. For each call site that has access to the `ActionDefRegistry`, populate:

```rust
binding_strictness: action_defs.get(step.def_id).map(|def| def.binding_strictness),
```

For test-fixture construction sites (from T-001), use a conservative default (`Some(BindingStrictness::ExactIdentity)` or `None`, depending on what the test asserts).

### 3. Verify observer / renderer paths

Grep `binding_strictness` after the edit to confirm no consumer expects the field to be non-optional. The observer binary and any golden-support renderer should treat the field as informational — missing rendering is not a functional failure.

### 4. Unit test

Add focused tests asserting:
- In `agent_tick/planning.rs`, a `PlannedStepSummary` built from an `ActionDef` classified `ExactIdentity` carries `binding_strictness: Some(ExactIdentity)`.
- In `agent_tick/planning.rs`, a `PlannedStepSummary` built from a `FungibleEquivalentCommodity` def carries `Some(FungibleEquivalentCommodity)`.
- In `decision_trace.rs`, a summary whose constructor passes `None` renders without panic.

## Files to Touch

- `crates/worldwake-ai/src/decision_trace.rs` (modify — field addition, population sites, unit test)
- `crates/worldwake-ai/src/lib.rs` (modify if needed — re-export if external consumers require it; T-001 already re-exports `BindingStrictness` from `worldwake-sim`, so `decision_trace` imports it directly)

## Out of Scope

- Extending `PlannedStep` in `planner_ops.rs` — intentionally excluded per spec D6.
- Observer binary formatting changes to surface the new field in operator output — separate tooling-only spec if warranted.
- Golden-test assertions exercising the field — T-005.

## Acceptance Criteria

### Tests That Must Pass

1. New focused unit coverage proving `binding_strictness` population from `ActionDef::binding_strictness` at the `summarize_step` construction site, plus a `decision_trace.rs` formatter-safety test for `None`.
2. Existing `decision_trace` tests continue to pass (including `StrategicStepTrace`-related tests).
3. Existing suite: `cargo build --workspace && cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings`.

### Invariants

1. `PlannedStepSummary.binding_strictness` is a snapshot of `ActionDef::binding_strictness` at summary construction time. It is never written at runtime from any other source (FND-27: derived summary, not truth).
2. `PlannedStep` (in `planner_ops.rs`) is unchanged — the authoritative plan structure does not duplicate the classifier.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/agent_tick/planning.rs` — new constructor test asserting `binding_strictness` population from the `ActionDef` at summary time.
2. `crates/worldwake-ai/src/decision_trace.rs` — new formatter-safety case asserting `SelectedPlanTrace` rendering tolerates `binding_strictness: None`.

### Commands

1. `cargo test -p worldwake-ai decision_trace`
2. `cargo test -p worldwake-ai`
3. `cargo build --workspace && cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

Implemented as a trace-only change. `PlannedStepSummary` now carries `binding_strictness: Option<BindingStrictness>` in `crates/worldwake-ai/src/decision_trace.rs`, and `agent_tick/planning.rs::summarize_step` snapshots the value directly from `ActionDef::binding_strictness` when selected plans, replacement traces, and plan-attempt traces are summarized. Manual `PlannedStepSummary` fixtures were updated to populate the new optional field, and no observer/golden consumer required a behavior change because the field remains informational.

## Deviations

1. The population proof landed in `crates/worldwake-ai/src/agent_tick/planning.rs` rather than `decision_trace.rs`. That is the actual construction boundary, so proving the registry-to-summary copy there is more precise and keeps the ticket aligned with the live symbol ownership.

## Verification Result

Passed:

1. `cargo test -p worldwake-ai summarize_step_carries_binding_strictness_snapshot_from_action_def`
2. `cargo test -p worldwake-ai selected_plan_format_tolerates_missing_binding_strictness_snapshot`
3. `cargo test -p worldwake-ai decision_trace`
4. `cargo test -p worldwake-ai`
5. `cargo build --workspace`
6. `cargo test --workspace`
7. `cargo clippy --workspace --all-targets -- -D warnings`
