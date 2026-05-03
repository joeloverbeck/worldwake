# S133SOUCOMTIE-005: Strip vestigial wait and capacity fields from SourceReliabilityDiscount

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: Yes — `SourceReliabilityDiscount` shrinks; `format_source_reliability_discount_summary` rewinds; `S132` stale comment in `ranking.rs` rewritten to reference S133
**Deps**: S133SOUCOMTIE-004

## Problem

After tickets 002–004 land, the wait/capacity surface lives entirely on `SourceCompositeRank`. The fields `average_wait_ticks`, `wait_penalty`, `last_observed_capacity`, `capacity_freshness_ticks`, `capacity_signal` on `SourceReliabilityDiscount` are zero-filled holdovers from S131SOURELWAI-004's rolled-back motive-additive composite. Per FND-28 (no backward-compat in live authority paths), they must be removed once their replacement is live; leaving them violates the principle and adds dead width to the trace format.

## Assumption Reassessment (2026-05-03)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. `SourceReliabilityDiscount` lives at `crates/worldwake-ai/src/decision_trace.rs:546` with five fields the rollback reduced to constant zero (`average_wait_ticks`, `wait_penalty`, `last_observed_capacity`, `capacity_freshness_ticks`, `capacity_signal`). Construction sites: `ranking.rs:595` (production `source_reliability_failure_discount`), `ranking.rs:5749`, `ranking.rs:6048`, `ranking.rs:6173` (test fixtures), `decision_trace.rs:2462` (sample fixture), `agent_tick/planning.rs:4136` (test fixture), `goal_model.rs:2836` (test fixture). The Display formatter at `decision_trace.rs:1957-1971` consumes the dropped fields — must be rewritten back to the pre-S131 shape `", source_reliability=entity={} commodity={:?} failure={} pre={} post={}"`.
2. Per spec D1: "The save format does not need a bump: `SourceReliabilityDiscount` is part of the per-tick decision trace, which is not persisted across saves." Verified — the struct is not in any `with_component_schema_entries!` macro invocation and is not serialized in save_load.rs.
3. Shared abstraction boundary under audit: the per-candidate decision-trace projection of `SourceReliabilityDiscount` and its Display formatter. The wait/cap surface migrates to `SourceCompositeRank` (delivered by tickets 002+004); this ticket is the strip step.
13. Adjacent contradiction surfaced: the comment block at `ranking.rs:564-572` references "S132 will reintegrate wait/capacity..." — this is stale and must be replaced with a brief comment pointing at S133 (and the live `source_composite_rank` consumer). Treat as required consequence of this ticket because the comment narrates the very shape this ticket finishes deleting.

## Architecture Check

1. Removing dead fields straightens the trace shape and matches FND-28's "no backward compat in live authority paths." Alternatives considered: (i) leaving the fields in place "for stability" — rejected because their values are always zero post-rollback and `SourceCompositeRank` already carries the live versions of the same data; (ii) gating behind a feature flag — rejected per FND-28 and the project's no-shim policy.
2. Existing `source_reliability_discount_*` focused tests at `ranking.rs:5592-5827, 5983, 6074` exercise the failure-ratio path only; they continue to pass after the field shrink because the failure-ratio fields are preserved.

## Verification Layers

1. Struct shrink compiles cleanly across all construction sites → workspace build.
2. Display format change → existing summary assertion test at `decision_trace.rs:3841` shrinks (the wait/cap substring assertions land in ticket 004's composite assertions; the failure-ratio substrings remain).
3. Failure-ratio motive discount path remains intact → existing focused tests `source_reliability_discount_applies_failure_ratio_proportionally:5700`, `source_reliability_discount_floors_positive_motive_at_one:5765`, `source_reliability_discount_returns_none_when_failure_ratio_is_zero:5827`, `source_reliability_discount_composes_with_competition_discount:5983`, `pending_source_reliability_failure_reorders_candidates_before_persistence:6074` all remain green.
6. Single-layer (trace-projection struct shrink) ticket; no authoritative-state mutation, no comparator semantic change.

## What to Change

### 1. Shrink `SourceReliabilityDiscount`

In `crates/worldwake-ai/src/decision_trace.rs:546`:

```rust
pub struct SourceReliabilityDiscount {
    pub source_entity: EntityId,
    pub commodity: CommodityKind,
    pub failure_ratio_permille: u32,
    pub pre_discount_motive: u32,
    pub post_discount_motive: u32,
}
```

Drop: `average_wait_ticks`, `wait_penalty`, `last_observed_capacity`, `capacity_freshness_ticks`, `capacity_signal`.

### 2. Rewind Display formatter

In `decision_trace.rs:1957-1971`, replace with:

```rust
fn format_source_reliability_discount_summary(discount: &SourceReliabilityDiscount) -> String {
    format!(
        ", source_reliability=entity={} commodity={:?} failure={} pre={} post={}",
        discount.source_entity,
        discount.commodity,
        discount.failure_ratio_permille,
        discount.pre_discount_motive,
        discount.post_discount_motive,
    )
}
```

### 3. Update production construction site

In `crates/worldwake-ai/src/ranking.rs:595` (`source_reliability_failure_discount`), drop the five vestigial fields. Also drop the now-unused `capacity_freshness_ticks` local computation at `ranking.rs:591-593` and the `observation_record` arg threading if it becomes unused after the strip — verify by compile.

### 4. Update test fixtures

- `crates/worldwake-ai/src/decision_trace.rs:2462` `sample_source_reliability_discount` — drop the five fields.
- `crates/worldwake-ai/src/ranking.rs:5749, 6048, 6173` — drop the five fields.
- `crates/worldwake-ai/src/agent_tick/planning.rs:4136` — drop the five fields.
- `crates/worldwake-ai/src/goal_model.rs:2836` — drop the five fields.

### 5. Shrink existing summary assertion

In `decision_trace.rs:3839-3850`, remove the assertions for `wait_avg=`, `wait_pen=`, `cap=`, `cap_age=`, `cap_sig=` (those landed in ticket 004's composite-line assertions). Keep `source_reliability=entity=`, `commodity=Bread`, `failure=500`, `pre=700`, `post=350` assertions.

### 6. Replace stale narrative comment

In `crates/worldwake-ai/src/ranking.rs:564-572`, replace the "S132 will reintegrate..." block with a one-line note pointing readers at `source_composite_rank` (now the live consumer of wait/capacity per `SourceCompositeRank`).

## Files to Touch

- `crates/worldwake-ai/src/decision_trace.rs` (modify — struct, Display, sample, summary assertion)
- `crates/worldwake-ai/src/ranking.rs` (modify — production construction at 595, 4 test fixtures, narrative comment)
- `crates/worldwake-ai/src/agent_tick/planning.rs` (modify — test fixture at 4136)
- `crates/worldwake-ai/src/goal_model.rs` (modify — test fixture at 2836)

## Out of Scope

- New composite trace formatting (ticket 004).
- Save format change — `SourceReliabilityDiscount` is per-tick decision trace, not persisted (verified in spec D1).
- Golden coverage (ticket 006).

## Acceptance Criteria

### Tests That Must Pass

1. Existing focused tests for the failure-ratio path: `source_reliability_discount_skips_non_commodity_goals` (`ranking.rs:5592`), `source_reliability_discount_returns_none_without_experience:5611`, `source_reliability_discount_returns_none_without_preference_profile:5655`, `source_reliability_discount_applies_failure_ratio_proportionally:5700`, `source_reliability_discount_floors_positive_motive_at_one:5765`, `source_reliability_discount_returns_none_when_failure_ratio_is_zero:5827`, `source_reliability_discount_composes_with_competition_discount:5983`, `pending_source_reliability_failure_reorders_candidates_before_persistence:6074`.
2. Existing summary assertion test at `decision_trace.rs:3841` (shrunk) — `source_reliability=entity=`, `failure=`, `pre=`, `post=` substrings still present; wait/cap substrings absent.
3. Existing `summarize_ranked_goal_preserves_source_reliability_discount` (`agent_tick/planning.rs:4128`) — still preserves the surviving fields after roundtrip.
4. Workspace builds cleanly: `cargo build --workspace`.
5. Existing suite: `cargo test --workspace`.

### Invariants

1. `SourceReliabilityDiscount` carries only the failure-ratio motive surface (Design Goal 7); wait/capacity surface lives on `SourceCompositeRank` only (FND-28: one canonical representation).
2. Save format unchanged — `SourceReliabilityDiscount` is decision-trace state, not persisted.
3. No backward-compat shim and no zero-field placeholder remains (FND-28).

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/decision_trace.rs:3841` — shrunk to drop wait/cap substring assertions.
2. No new tests; the focused failure-ratio coverage already exists.

### Commands

1. `cargo build --workspace` (catches every construction site).
2. `cargo test -p worldwake-ai decision_trace::tests` (focused).
3. `cargo test -p worldwake-ai ranking::tests` (focused).
4. `cargo test --workspace` (full).
