# S23-005: Reform Unknown blocker TTL and diagnostics

**Status**: ✅ COMPLETED
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes — PlanningBudget field, TTL logic, trace types (worldwake-ai)
**Deps**: S23-001, S23-002

## Problem

`BlockingFact::Unknown` gets `transient_block_ticks` (20 ticks) with zero diagnostic information. This silently suppresses goals for 20 ticks when the real cause is simply a missing `BlockingFact` variant in `derive_blocking_fact()`. The TTL should be shortened to 5 ticks, Unknown blockers should carry diagnostic context (the failed `ActionDefId`), and when decision tracing is active, Unknown blockers should emit trace events that include the `PlannerOpKind` so developers can identify the root cause and add proper variants.

## Assumption Reassessment (2026-03-24)

1. `PlanningBudget` at `budget.rs:5-15` has `transient_block_ticks: u32` (default 20) and `structural_block_ticks: u32` (default 200) — confirmed. Unknown currently maps to `transient_block_ticks`.
2. `blocking_fact_ttl()` in `failure_handling.rs` maps `Unknown => budget.transient_block_ticks` — confirmed.
3. `PlanningPipelineTrace` at `decision_trace.rs:94-107` has fields: `dirty_reasons`, `plan_continued`, `candidates`, `planning`, `selection`, `execution`, `action_start_failures` — confirmed. Needs new `unknown_blockers` field.
4. `BlockerDiagnostic` struct added in S23-001 — `action_def: ActionDefId` only. S23-005 populates it.
5. `handle_plan_failure()` has access to `context.failed_step.def_id` — confirmed. This is where `diagnostic_context` is set when blocking fact is Unknown.
6. `agent_tick` modules have access to `DecisionTraceSink` — confirmed. This is the emission point for `UnknownBlockerTrace`.
7. `dump_agent()` in `decision_trace.rs` — confirmed exists. Needs extension for unknown blocker details.
8. `PlannerOpKind` is available in AI crate from `planner_ops.rs` — confirmed. Used in trace only, not stored on component.

## Architecture Check

1. Separate `unknown_block_ticks` from `transient_block_ticks` prevents accidentally changing TTL for other transient blockers.
2. `BlockerDiagnostic` on the stored component carries only `ActionDefId` (core-layer safe). `PlannerOpKind` lives only in the trace event (AI-layer only). This respects the crate dependency boundary.
3. No backward-compatibility shims.

## Verification Layers

1. Unknown TTL is 5, not 20 → focused unit test in `failure_handling::tests`
2. `diagnostic_context` populated for Unknown → focused unit test
3. `UnknownBlockerTrace` emitted when tracing active → runtime trace test or golden test
4. `dump_agent()` includes unknown blocker details → manual verification or snapshot test
5. `transient_block_ticks` unchanged for other variants → focused unit test

## What to Change

### 1. `budget.rs` — add `unknown_block_ticks` field

```rust
pub unknown_block_ticks: u32,  // default: 5
```

Update `Default` impl and any test that checks budget default values (e.g., `planning_budget_default_matches_ticket_values`).

### 2. `failure_handling.rs` — update `blocking_fact_ttl()` for Unknown

```rust
BlockingFact::Unknown => budget.unknown_block_ticks,
```

### 3. `failure_handling.rs` — populate `diagnostic_context` for Unknown

In `handle_plan_failure()`, after constructing the `BlockedIntent`:

```rust
let diagnostic = if matches!(blocking_fact, BlockingFact::Unknown) {
    Some(BlockerDiagnostic {
        action_def: context.failed_step.def_id,
    })
} else {
    None
};
```

Set `diagnostic_context: diagnostic` on the `BlockedIntent`.

### 4. `decision_trace.rs` — add `UnknownBlockerTrace`

```rust
#[derive(Clone, Debug)]
pub struct UnknownBlockerTrace {
    pub goal_key: GoalKey,
    pub failed_action_def: ActionDefId,
    pub op_kind: PlannerOpKind,
    pub target: Option<EntityId>,
    pub place: Option<EntityId>,
}
```

Uses concrete `Option<EntityId>` from `BlockerKey.target` (not ephemeral `PlanningEntityRef`).
`op_kind` is derived from the semantics table lookup on `BlockerDiagnostic.action_def`.

Add to `PlanningPipelineTrace`:
```rust
pub unknown_blockers: Vec<UnknownBlockerTrace>,
```

### 5. `agent_tick/mod.rs` — populate `unknown_blockers` at trace construction

When building `PlanningPipelineTrace` in `mod.rs`, scan `blocked_memory` for active Unknown blockers and populate the `unknown_blockers` field. This is a derived view of authoritative state (P25), answers "why isn't the agent doing X now?" (P27), and avoids cross-phase coupling (P24). All needed data is available: `goal_key`/`place`/`target` from `BlockerKey`, `action_def` from `BlockerDiagnostic`, `op_kind` from semantics table lookup.

### 6. `decision_trace.rs` — integrate into `dump_agent()` and `summary()`

When printing planning traces, also emit unknown blocker details:
```
  Unknown blockers recorded:
    goal=AcquireCommodity(Bread) action=adef42 op=Trade place=Some(eid5)
```

## Files to Touch

- `crates/worldwake-ai/src/budget.rs` (modify — new field)
- `crates/worldwake-ai/src/failure_handling.rs` (modify — TTL mapping, diagnostic population)
- `crates/worldwake-ai/src/decision_trace.rs` (modify — `UnknownBlockerTrace`, `PlanningPipelineTrace` field, `dump_agent()`)
- `crates/worldwake-ai/src/agent_tick/mod.rs` (modify — populate `unknown_blockers` at `PlanningPipelineTrace` construction)

## Out of Scope

- **No changes to `blocked_intent.rs`** — `BlockerDiagnostic` added in S23-001
- **No changes to `search/`** — that is S23-004
- **No changes to `candidate_generation.rs`** — that is S23-003
- **No new golden tests** — that is S23-006
- **Do not change `derive_blocking_fact()` logic** — variant classification is unchanged
- **Do not change TTL for any blocking fact other than Unknown**
- **Do not add new `BlockingFact` variants**

## Acceptance Criteria

### Tests That Must Pass

1. `planning_budget_default_matches_ticket_values` — updated with `unknown_block_ticks: 5`
2. NEW: `unknown_blocker_uses_dedicated_ttl` — Unknown gets 5 ticks, not `transient_block_ticks` (20)
3. NEW: `unknown_blocker_carries_diagnostic_context` — `diagnostic_context.action_def` matches failed step
4. NEW: `transient_blockers_unchanged_ttl` — SellerOutOfStock etc. still get 20 ticks
5. Existing `failure_handling::tests` — all pass
6. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. `transient_block_ticks` remains 20 for all non-Unknown transient blockers
2. `structural_block_ticks` remains 200 for structural blockers
3. `BlockerDiagnostic` stored on component is core-safe (no AI-layer types)
4. `PlannerOpKind` in `UnknownBlockerTrace` is trace-only (not persisted)
5. `PlanningPipelineTrace` default has empty `unknown_blockers` — no trace overhead when no Unknown blockers

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/failure_handling.rs::tests` — 3 new tests (Unknown TTL, diagnostic context, transient unchanged)
2. `crates/worldwake-ai/src/budget.rs` or inline — updated default check

### Commands

1. `cargo test -p worldwake-ai -- failure_handling`
2. `cargo test -p worldwake-ai -- budget`
3. `cargo clippy -p worldwake-ai`

## Outcome

- **Completion date**: 2026-03-24
- **What changed**:
  - `budget.rs`: Added `unknown_block_ticks: u32` (default 5) to `PlanningBudget`
  - `failure_handling.rs`: `blocking_fact_ttl()` maps `Unknown => budget.unknown_block_ticks` (was `transient_block_ticks`). `handle_plan_failure()` populates `diagnostic_context` with `BlockerDiagnostic { action_def }` for Unknown blockers.
  - `decision_trace.rs`: Added `UnknownBlockerTrace` struct and `unknown_blockers: Vec<UnknownBlockerTrace>` field on `PlanningPipelineTrace`. Integrated into `format_outcome()` (dump_agent) and `summary()`.
  - `agent_tick/mod.rs`: Populates `unknown_blockers` by scanning `blocked_memory` at `PlanningPipelineTrace` construction time (derived view of authoritative state, per P25/P27).
- **Deviations from original ticket**:
  - Ticket originally said to emit trace from `agent_tick/planning.rs` after `handle_plan_failure()` call. Corrected to derive from `blocked_memory` at trace construction in `agent_tick/mod.rs` — cleaner architecturally (P24: no cross-phase coupling, P25: derived view, P27: shows active blockers on every planning tick).
  - `UnknownBlockerTrace.targets` changed from `Vec<PlanningEntityRef>` to `target: Option<EntityId>` — uses concrete entity from `BlockerKey.target` rather than ephemeral hypothetical refs (P3).
- **Verification**: `cargo test --workspace` (0 failures), `cargo clippy --workspace` (0 warnings). 3 new tests + 1 updated test all pass.
