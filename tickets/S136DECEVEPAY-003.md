# S136DECEVEPAY-003: Populate rejection_dimension from RankedGoalComparisonOutcome

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: Yes — `worldwake-ai::agent_tick::planning::build_rejected_alternatives`
**Deps**: archive/tickets/S136DECEVEPAY-001.md

## Problem

Spec D6 (rejection_dimension slice): `RejectedAlternativeSummary.rejection_dimension` (added by ticket 001) is currently populated as `None` at every construction site. The decisive ranking dimension is already computed by the planner — `RankedGoalComparisonOutcome::decisive_dimension` exists at `crates/worldwake-ai/src/ranking.rs:2367` and is produced by `ranked_goal_ordering` at `ranking.rs:2559`. Wiring it through into the payload makes the dimension that ordered each rejected goal against the chosen plan part of always-on history, satisfying the success-path side of the FND-29A reconstruction goal without new computation.

## Assumption Reassessment (2026-05-06)

1. `build_rejected_alternatives` lives at `crates/worldwake-ai/src/agent_tick/planning.rs:931-1002`. It receives `ranked_candidates: &OrderedRanked<'_>`, `portfolio`, `chosen_goal_key`, `committed_motive`, and `max_alternatives: u8`, and produces `Vec<worldwake_core::RejectedAlternativeSummary>`. The current implementation at `planning.rs:994` constructs each summary with `goal_key`, `rejection_reason`, `score_gap` only. After ticket 001, this site explicitly initializes `rejection_dimension: None`.
2. `RankedGoalComparisonOutcome` at `ranking.rs:2367` carries `decisive_dimension: RankedGoalComparisonDimension`. The pairwise comparison helper `ranked_goal_ordering(left, right)` at `ranking.rs:2559` returns `(Ordering, Option<RankedGoalComparisonDimension>)` — the optional dimension is populated when ordering is unequal. Verify at implementation time whether `OrderedRanked` already exposes the per-rejection comparison outcome against the chosen plan, or whether ticket 003 must call `ranked_goal_ordering(rejected_entry, committed_entry)` per rejected candidate.
3. Ticket 001 added the core-side `RankedGoalComparisonDimensionTag` mirror but did not add a `worldwake-sim` conversion; `worldwake-sim` cannot depend on `worldwake-ai`. This ticket owns the AI emission-site conversion via an inline match on the source enum, matching the existing precedent at `crates/worldwake-ai/src/agent_tick/execution.rs:548` where `belief_status_tag(BeliefStatus) -> BeliefStatusTag` is implemented inline at the AI emission site.
4. Existing test `emit_plan_selection_events_records_commit_then_adoption_with_truncation:3464` exercises `build_rejected_alternatives` indirectly. Existing test `build_rejected_alternatives` callers at `planning.rs:3924` (in the test module) construct synthetic rejected alternatives — these test fixtures may keep `rejection_dimension: None` since they don't have a real `RankedGoalComparisonOutcome` available.
5. Boundary under audit: the rejected-alternative summary's dimension surface. Compared branches: pre-ticket (None for all entries) vs. post-ticket (Some(dimension) for entries produced by `build_rejected_alternatives`; None preserved for synthetic test fixtures).

## Architecture Check

1. `rejection_dimension` is a derived projection of state already computed during ranking — no new ranking pass, no new authoritative state (FND-3, FND-27).
2. The `Tag` mirror at the AI emission site avoids inverting the `core ← ai` dependency and matches the existing `BeliefStatusTag` precedent (Core-Side Mirror Enum pattern).
3. `Option<RankedGoalComparisonDimensionTag>` (rather than required field) preserves graceful behavior when the comparison cannot be reconstructed (e.g., synthetic test fixtures, edge cases where ordering is undefined).

## Verification Layers

1. `rejection_dimension` correctness → focused unit on `build_rejected_alternatives` asserting the expected dimension for a contested-commit fixture.
2. Round-trip preservation → existing serde test in core (extended by ticket 001) exercises the new field's serialization.
3. No behavioral regression → existing planning suite passes (`cargo test -p worldwake-ai planning::`).

## What to Change

### 1. Pass comparison outcome into `build_rejected_alternatives`

Update the helper at `planning.rs:931-1002` so each rejected candidate produces its dimension. Verify at implementation time whether `OrderedRanked` already carries pairwise outcomes against the chosen plan. If it does, read `decisive_dimension` per rejected entry. If not, invoke `ranked_goal_ordering(&rejected_entry, &committed_entry)` per rejected candidate to compute it.

The cap remains `cognitive.decision_history_alternatives` via the existing `rejected.truncate(usize::from(max_alternatives))` call at `planning.rs:991`.

### 2. Inline dimension → tag converter

Add an inline match at the AI call site (precedent: `belief_status_tag` at `execution.rs:548`):

```rust
fn ranked_goal_comparison_dimension_tag(
    d: worldwake_ai::ranking::RankedGoalComparisonDimension,
) -> worldwake_core::RankedGoalComparisonDimensionTag {
    use worldwake_ai::ranking::RankedGoalComparisonDimension as Src;
    use worldwake_core::RankedGoalComparisonDimensionTag as Tag;
    match d {
        Src::PriorityClass => Tag::PriorityClass,
        // ... 1:1 for every variant
    }
}
```

Ticket 001's save/load coverage proves the core tag field roundtrips in current-format payloads; the inline AI-side function is the only runtime source-enum conversion.

### 3. Replace `None` placeholder at the runtime site

In `planning.rs:993-998`, change the construction from `rejection_dimension: None` (placeholder from ticket 001) to:

```rust
.map(|rejected| {
    let dim = ranked_goal_ordering(&rejected.entry, &committed_entry)
        .1
        .map(ranked_goal_comparison_dimension_tag);
    worldwake_core::RejectedAlternativeSummary {
        goal_key: rejected.goal_key,
        rejection_reason: rejected.rejection_reason,
        score_gap: score_gap(committed_motive, rejected.motive_score),
        rejection_dimension: dim,
    }
})
```

Other construction sites (test fixtures in `save_load.rs`, `observer.rs`, `planning.rs::tests`) retain `rejection_dimension: None` unless the test explicitly wants to assert dimension behavior.

## Files to Touch

- `crates/worldwake-ai/src/agent_tick/planning.rs` (modify — `build_rejected_alternatives` body, inline tag converter)
- `crates/worldwake-ai/src/ranking.rs` (verify only — no expected changes; confirm `ranked_goal_ordering` is callable from the planning module)

## Out of Scope

- Threading dimension through pre-rank-filtered goals (those use `GoalSuppressed`/`GoalOffered` events with `GoalRejectionReason`, not dimension — spec design goal #5).
- Adding `rejection_dimension` to other payloads (only `RejectedAlternativeSummary` carries it per spec).
- Modifying `worldwake-sim::save_load`; ticket 001 proved current-format roundtrip of the core tag field, and this ticket owns only the AI-side source-enum conversion.
- Populating `assumptions` (ticket 002).
- Populating `decisive_*` (ticket 004).

## Acceptance Criteria

### Tests That Must Pass

1. Extended test `emit_plan_selection_events_records_commit_then_adoption_with_truncation:3464` asserts `rejection_dimension == Some(RankedGoalComparisonDimensionTag::MotiveScore)` for the contested-commit fixture.
2. New focused unit test on `build_rejected_alternatives` covering at least 2 dimension cases (e.g., motive-score loss, source-composite loss) — names the live `GoalKind` under test.
3. Existing planning suite passes: `cargo test -p worldwake-ai planning::`.

### Invariants

1. `rejection_dimension` is `Some(...)` for every entry in `build_rejected_alternatives`'s output when the chosen plan is in scope; only test-synthesized `RejectedAlternativeSummary` instances may carry `None`.
2. The recorded dimension matches what `ranked_goal_ordering` would return today for the same pair.
3. No new authoritative state introduced; `Option<RankedGoalComparisonDimensionTag>` is a derived projection.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/agent_tick/planning.rs::tests::emit_plan_selection_events_records_commit_then_adoption_with_truncation` — extend with dimension assertion.
2. `crates/worldwake-ai/src/agent_tick/planning.rs::tests` — new focused unit covering at least 2 distinct decisive dimensions (motive-score, source-composite or feasibility).

### Commands

1. `cargo test -p worldwake-ai planning::tests::emit_plan_selection_events_records_commit_then_adoption_with_truncation`
2. `cargo test -p worldwake-ai planning::`
3. `cargo test -p worldwake-ai`
4. `./scripts/verify.sh`
