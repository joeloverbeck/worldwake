# S03PLATARIDE-003: Add `BindingRejection` trace struct and wire into decision traces

**Status**: ✅ COMPLETED
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes — new trace struct, `PlanAttemptTrace` extended, `search_plan` signature gains optional trace output, `dump_agent` updated
**Deps**: S03PLATARIDE-001, S03PLATARIDE-002 (needs `matches_binding()` wired into search to have rejections to record)

## Problem

When the binding filter rejects candidates, there is no diagnostic surface to understand what was rejected and why. Principle 27 (debuggability is a product feature) requires that the decision trace system answers: what target was required, which affordances were rejected for wrong binding, which matched, and whether failure came from target disappearance, wrong target, or resource absence.

## Assumption Reassessment (2026-03-17)

1. `PlanAttemptTrace` is defined in `crates/worldwake-ai/src/decision_trace.rs:134` with fields `goal: GoalKey` and `outcome: PlanSearchOutcome`.
2. `plan_search_result_to_trace()` in `agent_tick.rs:1267` converts `PlanSearchResult` into `PlanAttemptTrace` and currently constructs `PlanAttemptTrace { goal, outcome }`.
3. `DecisionTraceSink::dump_agent()` in `decision_trace.rs:274` iterates traces and calls `format_outcome()`.
4. `search_plan()` in `search.rs:102` returns `PlanSearchResult` and does not currently output any trace data about candidate filtering.
5. The binding `.retain()` filter (S03PLATARIDE-002) operates inside `search_candidates()` (search.rs:372) which is called within `search_plan()`'s expansion loop.

## Architecture Check

1. `BindingRejection` is a diagnostic-only struct in the AI crate — it must never become authoritative world state.
2. Collecting rejections requires threading a mutable `Vec<BindingRejection>` through `search_candidates()`. The simplest approach: `search_plan()` accepts an optional `&mut Vec<BindingRejection>` and passes it into `search_candidates()`, which populates it during the `.retain()` filter.
3. `PlanAttemptTrace` gains a `binding_rejections: Vec<BindingRejection>` field.
4. No backward-compatibility shims.

## What to Change

### 1. Define `BindingRejection` struct

In `crates/worldwake-ai/src/decision_trace.rs`:

```rust
/// Diagnostic record of a candidate rejected by goal target binding.
#[derive(Clone, Debug)]
pub struct BindingRejection {
    pub def_id: ActionDefId,
    pub rejected_targets: Vec<EntityId>,
    pub required_target: Option<EntityId>,
}
```

Note: `rejected_targets` is `Vec<EntityId>` (not single `EntityId`) because `authoritative_targets` may contain multiple entities. `required_target` is `Option<EntityId>` because the required target is extracted from `GoalKey.entity` or `GoalKey.place`.

### 2. Add `binding_rejections` to `PlanAttemptTrace`

```rust
pub struct PlanAttemptTrace {
    pub goal: GoalKey,
    pub outcome: PlanSearchOutcome,
    pub binding_rejections: Vec<BindingRejection>,
}
```

### 3. Collect rejections in `search_candidates()`

In `crates/worldwake-ai/src/search.rs`, modify `search_candidates()` to accept an optional `&mut Vec<BindingRejection>` parameter. In the binding `.retain()` filter, when `matches_binding()` returns `false`, push a `BindingRejection` with the candidate's `def_id`, `authoritative_targets`, and the goal's required target (from `goal.key.entity` or `goal.key.place`).

### 4. Thread rejections through `search_plan()`

Modify `search_plan()` to accept an optional `&mut Vec<BindingRejection>` and pass it into `search_candidates()`. Accumulate rejections across all expansion iterations.

### 5. Update `plan_search_result_to_trace()` in `agent_tick.rs`

Pass the collected `Vec<BindingRejection>` into `PlanAttemptTrace` construction:

```rust
PlanAttemptTrace {
    goal,
    outcome,
    binding_rejections: rejections,
}
```

### 6. Update `format_outcome()` / `dump_agent()` in `decision_trace.rs`

When `PlanAttemptTrace` has non-empty `binding_rejections`, include them in the dump output. Format: `"  binding rejected: {def_name} targets={rejected_targets:?} required={required_target:?}"`.

### 7. Add unit tests for trace recording

- Construct a `PlanAttemptTrace` with non-empty `binding_rejections` and verify fields are accessible.
- Verify `BindingRejection` struct holds expected data.

## Files to Touch

- `crates/worldwake-ai/src/decision_trace.rs` (modify — add `BindingRejection` struct, extend `PlanAttemptTrace`, update `format_outcome`)
- `crates/worldwake-ai/src/search.rs` (modify — `search_candidates()` and `search_plan()` gain optional rejection collector parameter)
- `crates/worldwake-ai/src/agent_tick.rs` (modify — `plan_search_result_to_trace()` passes rejections, call sites of `search_plan()` pass rejection collector)

## Out of Scope

- The `matches_binding()` implementation — that is S03PLATARIDE-001.
- The `.retain()` filter itself — that is S03PLATARIDE-002.
- Golden integration tests — that is S03PLATARIDE-004.
- Any changes to `worldwake-core` or `worldwake-sim`.
- Making `BindingRejection` part of authoritative world state (it is diagnostic-only).

## Acceptance Criteria

### Tests That Must Pass

1. `binding_rejection_struct_holds_data` — construct `BindingRejection`, verify fields.
2. `plan_attempt_trace_includes_binding_rejections` — construct `PlanAttemptTrace` with rejections, verify accessible.
3. All existing `cargo test -p worldwake-ai` tests pass (the rejection collector is additive; existing call sites pass empty or `None`).
4. `cargo clippy --workspace` clean.

### Invariants

1. `BindingRejection` is never stored as authoritative state — it exists only in `DecisionTraceSink`.
2. Tracing is opt-in and zero-cost when disabled — rejection collection only occurs when tracing is enabled.
3. `search_plan` signature change is backward-compatible (optional parameter or default empty vec).
4. `dump_agent()` output includes binding rejection information when present.
5. Planner determinism preserved — trace collection is a side-effect that does not influence search decisions.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/decision_trace.rs` — 2 new unit tests for `BindingRejection` and extended `PlanAttemptTrace`.
2. Existing tests updated to supply the new `binding_rejections: vec![]` field where `PlanAttemptTrace` is constructed (if any exist in test code).

### Commands

1. `cargo test -p worldwake-ai -- binding_rejection`
2. `cargo test --workspace && cargo clippy --workspace`

## Outcome

- **Completion date**: 2026-03-17
- **What changed**:
  - `decision_trace.rs`: Added `BindingRejection` struct, extended `PlanAttemptTrace` with `binding_rejections` field, updated `format_outcome()` to print rejections via `dump_agent()`.
  - `search.rs`: `search_plan()` and `search_candidates()` gained `Option<&mut Vec<BindingRejection>>` parameter. When `Some`, the `.retain()` binding filter records rejected candidates. When `None`, zero-cost.
  - `agent_tick.rs`: `build_candidate_plans()` gained `collect_rejections: bool`, returns per-candidate rejections. Tracing path passes `true` and feeds rejections into `plan_search_result_to_trace()`.
  - `lib.rs`: Exported `BindingRejection`.
- **Deviations from plan**: None. All deliverables implemented as specified.
- **Verification**: `cargo test --workspace` (1,862 passed, 0 failed), `cargo clippy --workspace` clean. 3 new unit tests pass.
