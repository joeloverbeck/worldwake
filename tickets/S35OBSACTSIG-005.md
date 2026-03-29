# S35OBSACTSIG-005: Add `CompetitionDiscount` trace struct and extend `RankedGoal`

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: Yes — worldwake-ai decision trace and goal model
**Deps**: S35OBSACTSIG-001 (ActionDomain in core)

## Problem

Before implementing the ranking discount, the data structures to record the discount for debuggability must exist. This ticket adds the `CompetitionDiscount` trace struct and the `competition_discount` field on `RankedGoal`, without any behavioral change.

## Assumption Reassessment (2026-03-29)

1. `RankedGoal` at `crates/worldwake-ai/src/goal_model.rs:1844` has fields: `grounded`, `priority_class`, `motive_score`, `provenance`, `feasibility`. No `competition_discount` field.
2. `RankedGoal` derives `Clone, Debug, Eq, PartialEq, Serialize, Deserialize`.
3. `DecisionTraceSink` at `crates/worldwake-ai/src/decision_trace.rs:812` records `AgentDecisionTrace` entries. Traces reference `RankedGoal` through `PlanningPipelineTrace.candidates`.
4. `dump_agent()` on `DecisionTraceSink` renders trace data to stderr — must be updated to render `CompetitionDiscount` when present.
5. `ActionDomain` will be in `worldwake-core` after S35OBSACTSIG-001. The AI crate depends on core, so it can use `ActionDomain` directly.
6. `Permille` is in `worldwake-core::numerics`.

## Architecture Check

1. Separating the trace data structure from the behavioral logic (S35OBSACTSIG-006) keeps this ticket small and reviewable.
2. `CompetitionDiscount` is an `Option` on `RankedGoal` — zero-cost when absent, follows existing `provenance: Option<RankedGoalProvenance>` pattern.
3. No shims or compatibility layers.

## Verification Layers

1. `CompetitionDiscount` construction -> focused unit test
2. `RankedGoal` with/without competition_discount -> focused unit test
3. `dump_agent()` rendering -> manual inspection (too coupled to output format for assertion)
4. Single-layer ticket: data types only

## What to Change

### 1. Add `CompetitionDiscount` struct

In `crates/worldwake-ai/src/decision_trace.rs` (or a new `competition.rs` if preferred):

```rust
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct CompetitionDiscount {
    pub observed_competitors: Vec<EntityId>,
    pub domain: ActionDomain,
    pub effective_discount: Permille,
    pub pre_discount_motive: u32,
    pub post_discount_motive: u32,
}
```

### 2. Add field to `RankedGoal`

In `crates/worldwake-ai/src/goal_model.rs`, add:
```rust
#[serde(default)]
pub competition_discount: Option<CompetitionDiscount>,
```

Note: `CompetitionDiscount` does not need `Serialize/Deserialize` — it's trace-only ephemeral data. If `RankedGoal` requires it for derive consistency, add the derives or use `#[serde(skip)]`.

### 3. Update all `RankedGoal` construction sites

Every place that constructs `RankedGoal` must now include `competition_discount: None`.

### 4. Update `dump_agent()` rendering

In `DecisionTraceSink::dump_agent()`, add a rendering block for `CompetitionDiscount` when `ranked_goal.competition_discount.is_some()`.

### 5. Update `summary()` on `DecisionOutcome`

If `CompetitionDiscount` is present on the selected goal, append a note to the summary string.

## Files to Touch

- `crates/worldwake-ai/src/decision_trace.rs` (modify — add `CompetitionDiscount` struct, update `dump_agent()`)
- `crates/worldwake-ai/src/goal_model.rs` (modify — add field to `RankedGoal`)
- `crates/worldwake-ai/src/ranking.rs` (modify — update `RankedGoal` construction to include `competition_discount: None`)
- Any other files constructing `RankedGoal` (modify — add field)

## Out of Scope

- Populating `CompetitionDiscount` with actual data (S35OBSACTSIG-006)
- Perception system changes (S35OBSACTSIG-003)
- `GoalBeliefView` extensions (S35OBSACTSIG-004)
- Golden tests (S35OBSACTSIG-007)

## Acceptance Criteria

### Tests That Must Pass

1. `CompetitionDiscount` can be constructed with example data.
2. `RankedGoal` with `competition_discount: None` matches existing behavior.
3. `RankedGoal` with `competition_discount: Some(...)` compiles and compares correctly.
4. Existing suite: `cargo test --workspace`

### Invariants

1. All existing `RankedGoal` constructions initialize `competition_discount: None` — no behavioral change.
2. `CompetitionDiscount` is ephemeral trace data, never serialized to saves.
3. `dump_agent()` gracefully handles `None` (no output for that field).

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/decision_trace.rs` or `goal_model.rs` — unit test for `CompetitionDiscount` construction and `RankedGoal` field inclusion.

### Commands

1. `cargo test -p worldwake-ai`
2. `cargo test --workspace`
3. `cargo clippy --workspace`
