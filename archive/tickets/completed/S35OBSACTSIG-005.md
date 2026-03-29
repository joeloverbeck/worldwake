# S35OBSACTSIG-005: Add `CompetitionDiscount` trace struct and extend `RankedGoal`

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: Yes — worldwake-ai decision trace and goal model
**Deps**: S35OBSACTSIG-001 (ActionDomain in core)

## Problem

The live S35 codebase already had observable-activity beliefs and the belief-view surface, but the ranking/trace contract still had nowhere to carry or render a competition discount. This ticket fills that missing data-contract boundary so S35OBSACTSIG-006 can populate it without introducing a parallel trace path.

## Assumption Reassessment (2026-03-29)

Shared abstraction boundary under audit: live ranking data in [`RankedGoal`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/goal_model.rs), summarized trace data in [`RankedGoalSummary`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/decision_trace.rs), and the projection path between them in [`summarize_ranked_goal()`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/agent_tick/planning.rs).

1. [`RankedGoal`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/goal_model.rs) currently has `grounded`, `priority_class`, `motive_score`, `provenance`, and `feasibility`. It does not yet carry competition-discount metadata.
2. [`RankedGoal`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/goal_model.rs) derives `Clone, Debug, Eq, PartialEq, Serialize, Deserialize`, so adding a field requires a type that also satisfies that derive surface. `#[serde(default)]` / `#[serde(skip)]` would be the wrong architecture here because this ticket is not a backward-compatibility exercise.
3. The live trace pipeline does not render `RankedGoal` directly. `PlanningPipelineTrace.candidates.ranked` stores [`RankedGoalSummary`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/decision_trace.rs), populated by [`summarize_ranked_goal()`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/agent_tick/planning.rs). If the discount should be visible in summaries or `dump_agent()`, the summary type must also carry it.
4. [`DecisionTraceSink::dump_agent()`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/decision_trace.rs) currently prints candidate evidence and knowledge paths only. It does not print ranked-goal details today, so this ticket should add a narrowly scoped ranked-summary rendering block instead of pretending the discount will appear automatically.
5. `ActionDomain` is already live in `worldwake-core`, and the broader S35 prerequisites that expose observable activity are already present: `BelievedActivity` in [`crates/worldwake-core/src/belief.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/belief.rs), `activity_awareness_weight` in [`crates/worldwake-core/src/utility_profile.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/utility_profile.rs), and `GoalBeliefView::{believed_activity_of, agents_active_at}` in [`crates/worldwake-sim/src/belief_view.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/belief_view.rs). This ticket is no longer “before the ranking discount”; it is now the missing trace/data-contract piece that unblocks the ranking work.
6. `Permille` is in `worldwake-core`, and using it for the stored discount matches the S35 spec. No alias layer is needed.

## Architecture Check

1. The clean architecture is one data contract carried from ranking through trace summarization: `RankedGoal.competition_discount` as the live ranking artifact, mirrored into `RankedGoalSummary.competition_discount` for trace output. That preserves one source of truth and avoids a parallel “trace-only” side channel.
2. Keeping this ticket focused on the data contract and rendering boundary, while leaving score mutation to S35OBSACTSIG-006, still keeps review scope tight without splitting one concept across redundant structures.
3. `CompetitionDiscount` should derive serde with the rest of the ranking artifact instead of using skip/default attributes. That keeps the type coherent and avoids forbidden compatibility shims.
4. The ideal architecture here is still ranking artifact -> summary projection -> trace formatting. Pushing competition details straight into `DecisionOutcome::summary()` without storing them on the ranked artifacts would be weaker and less extensible.

## Verification Layers

1. ranking artifact carries discount metadata -> focused unit test around `RankedGoal` / `CompetitionDiscount`
2. trace projection preserves discount metadata -> focused unit test around `summarize_ranked_goal()`
3. trace summary / human-readable output surfaces discount when present -> focused decision-trace assertion
4. no-discount path remains silent -> focused decision-trace assertion
5. Single-layer ticket: ranking-trace data contract only

## What to Change

### 1. Add `CompetitionDiscount` struct

In `crates/worldwake-ai/src/decision_trace.rs`:

```rust
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
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
pub competition_discount: Option<CompetitionDiscount>,
```

Also extend `RankedGoalSummary` in `crates/worldwake-ai/src/decision_trace.rs` with the same optional field, and copy it in `summarize_ranked_goal()`.

### 3. Update all `RankedGoal` construction sites

Every place that constructs `RankedGoal` must now include `competition_discount: None`.

### 4. Update trace rendering

Update `DecisionOutcome::summary()`, `format_outcome()`, and `DecisionTraceSink::dump_agent()` to surface the selected/ranked competition discount when present, through `RankedGoalSummary`.

Do not add a second ad hoc trace path that bypasses ranked-goal summaries.

## Files to Touch

- `crates/worldwake-ai/src/decision_trace.rs` (modify — add `CompetitionDiscount` struct, update `dump_agent()`)
- `crates/worldwake-ai/src/goal_model.rs` (modify — add field to `RankedGoal`)
- `crates/worldwake-ai/src/agent_tick/planning.rs` (modify — project `competition_discount` into `RankedGoalSummary`)
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
4. `RankedGoalSummary` preserves `competition_discount` from `RankedGoal`.
5. Decision-trace summaries/rendering include the discount when present and omit it when absent.
6. Existing suite: `cargo test --workspace`

### Invariants

1. All existing `RankedGoal` constructions initialize `competition_discount: None` — no behavioral change.
2. `CompetitionDiscount` lives on the ranking artifact and its summary projection; no parallel alias path.
3. Trace formatting gracefully handles `None` (no discount text when absent).

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/goal_model.rs` — unit test for `CompetitionDiscount` construction / `RankedGoal` field inclusion.
2. `crates/worldwake-ai/src/agent_tick/planning.rs` — unit test that `summarize_ranked_goal()` preserves `competition_discount`.
3. `crates/worldwake-ai/src/decision_trace.rs` — unit tests for selected-goal summary/output including and omitting competition discount.

### Commands

1. `cargo test -p worldwake-ai`
2. `cargo test --workspace`
3. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

- Completion date: 2026-03-29
- Actually changed:
  `CompetitionDiscount` was added in [`crates/worldwake-ai/src/decision_trace.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/decision_trace.rs), `RankedGoal` and `RankedGoalSummary` both gained `competition_discount`, `summarize_ranked_goal()` now preserves it, summary/debug output now surfaces it, and all `RankedGoal` construction sites were updated to initialize `None`.
- Deviations from original plan:
  The ticket was corrected before implementation because S35 prerequisites were already live, `RankedGoalSummary` also needed the field, and serde skip/default shims were rejected in favor of one coherent ranking artifact plus summary projection.
- Verification results:
  Added focused tests for artifact construction, summary projection, and summary rendering. `cargo test -p worldwake-ai`, `cargo test --workspace`, and `cargo clippy --workspace --all-targets -- -D warnings` all passed.
