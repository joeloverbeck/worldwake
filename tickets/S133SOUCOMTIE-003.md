# S133SOUCOMTIE-003: Ranking comparator integrates SourceComposite tiebreaker

**Status**: PENDING
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: Yes — `AgendaEntry` schema extension, `RankedGoalComparisonDimension` enum extension, `ranked_goal_ordering` semantic change, `RankedGoalSummary` plumbing
**Deps**: S133SOUCOMTIE-002

## Problem

D4 is the load-bearing semantic change. Without comparator integration, motive-tied same-commodity siblings still resolve via late tiebreakers (place key, entity key) that don't reflect agent learning — wait/capacity history has no effect on source choice. Per the spec's Design Goal 2, the composite must govern ordering only between siblings sharing `(commodity, purpose)` keys; cross-category compares must continue to fall through `MotiveScore`.

## Assumption Reassessment (2026-05-03)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. The current `ranked_goal_ordering` (`crates/worldwake-ai/src/ranking.rs:2385`) compares in this order: `PriorityClass` → `SubstitutePreferenceOrder` → `MotiveScore` → `Feasibility` → `GoalSpecificity` → `OpportunityStrength` → `ShareBeliefTopicOrder` → `GoalKindOrder` → `CommodityKey` → `EntityKey` → `PlaceKey`. The spec's instruction "immediately after `MotiveScore` and before `Feasibility`" maps to inserting between lines 2402-2404 (MotiveScore) and 2407-2409 (Feasibility). Existing focused coverage: `same_priority_candidates_sort_by_motive_then_kind_then_ids` (`ranking.rs:7021`), `substitute_preference_order_outranks_same_category_self_consume_rival` (`ranking.rs:7085`), `simultaneous_critical_self_care_needs_rank_by_weighted_order` (`ranking.rs:7430`), `explain_ranked_goal_order_reports_decisive_dimension` (`ranking.rs:9303`).
2. `AgendaEntry` lives at `crates/worldwake-ai/src/agenda_types.rs:20`. Existing fields: `key`, `offer`, `phase`, `origin`, `introduced_tick`, `last_reconsidered_tick`, `revival_trigger`, `kill_condition`, `priority_class`, `motive_score`, `provenance`, `source_reliability_discount`, `competition_discount`, `feasibility`. The constructor `AgendaEntry::pending` (lines 40-69) takes positional args and enumerates every field — adding a new positional argument is the established pattern.
3. Shared abstraction boundary under audit: the comparator's same-commodity peer-key relation. The spec specifies `(GoalKind::AcquireCommodity { commodity, purpose, quantity_target }, _)` matched on `quantity.desired_target` (NOT the full `Quantity` struct) so that `acquire_commodity_quantity_bonus` differences don't gate the tiebreaker. `AcquisitionQuantity` exposes `.desired_target` per spec D4.
4. Ranking-sensitive precision check: when two `AcquireCommodity` siblings have *different* `motive_score` (because the failure-ratio discount applies to one but not the other), the existing `MotiveScore` dimension already separates them and `SourceComposite` does not fire — verified by the spec Design Goal 7 ("S133 layers the composite *on top of* that pre-existing motive discount"). When motive_score is tied, `SourceComposite` fires.
5. AgendaEntry literal construction sites count workspace-wide ≈60 across `interrupts.rs`, `side_benefit.rs`, `agent_tick/portfolio.rs`, `feasibility.rs`, `feasibility_probe.rs`, `ranking.rs`, `agenda_manager.rs`, `agent_tick/planning.rs`, `decision_runtime.rs`, `tests/golden_portfolio_planning.rs`, `crates/worldwake-cli/src/bin/observer.rs`. Most are test fixtures using literal struct construction (no `..Default::default()` spread; `AgendaEntry` has no `Default` impl). Each must add `source_composite: None` mechanically.
6. AI regression layer: this is a runtime ranking-pipeline change exercised by the existing `compare_ranked_goals` and downstream `agent_tick` flow. Local needs-only harness is sufficient for the new comparator unit tests; full action registries are not required.
13. Adjacent contradiction classification: the `S132` reference in the comment block at `ranking.rs:564-572` is a stale internal comment from a prior draft naming. Treat as future cleanup; not in this ticket's scope (the comment will be deleted alongside ticket 005's vestigial-field removal where the broader rollback narrative lives).

## Architecture Check

1. Inserting the composite check between MotiveScore and Feasibility preserves both Design Goals 1 (no motive_score mutation) and 2 (intra-commodity tiebreaker). Alternatives considered: (i) injecting at the candidate-generation phase — rejected because Spec Non-Goal "Pre-rank candidate deduplication" prohibits dropping siblings; (ii) replacing `EntityKey` as the deepest tiebreaker — rejected because that would regress determinism on cross-commodity ties.
2. The peer-key helper (`source_composite_peer_keys`) is a pure function over `GoalKind`/`OpportunityKey`, scoped to the comparator's call site — no new abstraction layer needed. Per FND-26, this remains state-mediated: the comparator reads `AgendaEntry.source_composite` (already populated upstream), it does not call into another system.
3. No backward-compat shim. AgendaEntry's structural change is propagated to every literal site (FND-28).

## Verification Layers

1. Same-commodity siblings with tied motive but divergent composite resolve to the higher composite → focused unit test on `ranked_goal_ordering`.
2. Cross-category compare (Wash vs AcquireCommodity) returns `MotiveScore` decisive dimension when motives differ; never reaches `SourceComposite` → focused unit test (regression contract for the four pre-existing failing goldens).
3. Different-commodity AcquireCommodity compare returns through subsequent dimensions, never `SourceComposite` → focused unit test.
4. Acquisition-quantity bonus difference does NOT gate the tiebreaker (peer keys ignore the bonus, compare on `desired_target` only) → focused unit test.
5. AgendaEntry plumbing — `summarize_ranked_goal` propagates `source_composite` into `RankedGoalSummary` → focused unit test (sibling of the existing `summarize_ranked_goal_preserves_source_reliability_discount` at `agent_tick/planning.rs:4128`).

## What to Change

### 1. Extend `AgendaEntry`

In `crates/worldwake-ai/src/agenda_types.rs:20`:

```rust
pub struct AgendaEntry {
    // existing fields unchanged ...
    pub competition_discount: Option<CompetitionDiscount>,
    pub source_composite: Option<SourceCompositeRank>,
    pub feasibility: FeasibilityHint,
}
```

Add the import for `SourceCompositeRank` to the file's `use` block.

Extend `AgendaEntry::pending` (`agenda_types.rs:40`) signature with a new positional `source_composite: Option<SourceCompositeRank>` parameter (placed between `competition_discount` and `feasibility` to mirror struct field ordering). All callers must pass it.

### 2. Extend `RankedGoalComparisonDimension`

In `crates/worldwake-ai/src/ranking.rs:2364`, add `SourceComposite` after `MotiveScore`:

```rust
pub enum RankedGoalComparisonDimension {
    PriorityClass,
    SubstitutePreferenceOrder,
    MotiveScore,
    SourceComposite,
    Feasibility,
    // existing variants unchanged
}
```

### 3. Insert the comparator dimension

In `ranked_goal_ordering` between lines 2404 (after MotiveScore return) and 2407 (before Feasibility check):

```rust
if let Some((left_key, right_key)) = source_composite_peer_keys(&left.offer, &right.offer)
    && left_key == right_key
{
    let ordering = right
        .source_composite
        .as_ref()
        .map_or(0, |c| c.composite_permille)
        .cmp(
            &left
                .source_composite
                .as_ref()
                .map_or(0, |c| c.composite_permille),
        );
    if ordering != Ordering::Equal {
        return (ordering, Some(RankedGoalComparisonDimension::SourceComposite));
    }
}
```

### 4. Add `source_composite_peer_keys` helper

In `ranking.rs`, near the comparator helpers, add:

```rust
fn source_composite_peer_keys(
    left: &GoalOffer,
    right: &GoalOffer,
) -> Option<((CommodityKind, CommodityPurpose, Quantity), (CommodityKind, CommodityPurpose, Quantity))> {
    // returns Some(((commodity, purpose, desired_target), same)) only when
    // both offers share the AcquireCommodity{commodity, purpose} key (compared
    // on quantity.desired_target ignoring the acquire_commodity_quantity_bonus
    // axis), or both offers share RestockCommodity{commodity}; None otherwise.
}
```

The exact key tuple shape is internal — match the ergonomic shape that lets callers compare with `==`. For `RestockCommodity`, fold a sentinel `CommodityPurpose::SelfConsume` and unit `Quantity` so a single tuple shape suffices, OR use a small `enum SourceCompositePeerKey { Acquire(...), Restock(...) }` — implementer's choice based on what reads cleanly at the call site.

### 5. Populate `source_composite` in the per-candidate ranking pass

In the per-candidate evaluation in `ranking.rs` where `apply_source_reliability_discount` and competition-discount are computed (around line 241), call `source_composite::source_composite_rank(candidate, &context)` and pass the result through `AgendaEntry::pending`.

### 6. Extend `RankedGoalSummary`

In `crates/worldwake-ai/src/decision_trace.rs:517`, add `pub source_composite: Option<SourceCompositeRank>` after `competition_discount`. Update `summarize_ranked_goal` (`crates/worldwake-ai/src/agent_tick/planning.rs:312`) to populate it from `ranked.source_composite`.

### 7. Update all AgendaEntry and RankedGoalSummary literal construction sites

The ~60 `AgendaEntry { ... }` literal sites and ~10 `RankedGoalSummary { ... }` sites across `worldwake-ai/src/`, `worldwake-ai/tests/`, and `worldwake-cli/src/bin/observer.rs` each need `source_composite: None` added. Target files include: `interrupts.rs`, `side_benefit.rs`, `agent_tick/portfolio.rs`, `feasibility.rs`, `feasibility_probe.rs`, `ranking.rs` (test fixtures), `agenda_manager.rs`, `agent_tick/planning.rs`, `decision_runtime.rs`, `tests/golden_portfolio_planning.rs`, `crates/worldwake-cli/src/bin/observer.rs`. Use `cargo build` iteratively to surface remaining sites by compile error.

## Files to Touch

- `crates/worldwake-ai/src/agenda_types.rs` (modify — struct + constructor)
- `crates/worldwake-ai/src/ranking.rs` (modify — comparator extension, peer-key helper, per-candidate population, ~10 inline test fixtures, new focused tests)
- `crates/worldwake-ai/src/decision_trace.rs` (modify — `RankedGoalSummary` field)
- `crates/worldwake-ai/src/agent_tick/planning.rs` (modify — `summarize_ranked_goal`, ~10 test fixtures, extend the `summarize_ranked_goal_preserves_source_reliability_discount` test or add a sibling for the composite)
- `crates/worldwake-ai/src/interrupts.rs` (modify — fixtures)
- `crates/worldwake-ai/src/feasibility.rs` (modify — fixtures)
- `crates/worldwake-ai/src/feasibility_probe.rs` (modify — fixtures)
- `crates/worldwake-ai/src/side_benefit.rs` (modify — fixtures)
- `crates/worldwake-ai/src/agent_tick/portfolio.rs` (modify — fixtures)
- `crates/worldwake-ai/src/agenda_manager.rs` (modify — fixtures)
- `crates/worldwake-ai/src/decision_runtime.rs` (modify — fixtures)
- `crates/worldwake-ai/tests/golden_portfolio_planning.rs` (modify — fixtures)
- `crates/worldwake-cli/src/bin/observer.rs` (modify — fixtures)
- `crates/worldwake-ai/src/lib.rs` (modify — re-export if needed)

## Out of Scope

- Decision-trace text formatting (ticket 004).
- Vestigial `SourceReliabilityDiscount` field removal (ticket 005).
- Golden coverage (ticket 006).

## Acceptance Criteria

### Tests That Must Pass

1. `source_composite_tiebreaker_fires_when_motive_score_tied_and_peer_keys_match` — two `AcquireCommodity{Apple, SelfConsume, q1}` siblings with tied `motive_score` and divergent `composite_permille` resolve to the higher composite; decisive dimension `SourceComposite`.
2. `source_composite_tiebreaker_does_not_fire_for_cross_category_compare` — `Wash` (motive 600) vs `AcquireCommodity{Apple,..}` (motive 500) returns `MotiveScore` decisive dimension; the comparator never reads either entry's `source_composite`.
3. `source_composite_tiebreaker_does_not_fire_for_different_commodity_acquire` — `AcquireCommodity{Apple,..}` vs `AcquireCommodity{Bread,..}` falls through to later dimensions; `SourceComposite` never fires.
4. `source_composite_peer_keys_compare_acquisition_quantity_by_desired_target` — peer-key match between two offers with the same `desired_target` but different `desired_min` or `acquire_commodity_quantity_bonus`.
5. `summarize_ranked_goal_preserves_source_composite` — sibling of the existing source_reliability test at `agent_tick/planning.rs:4128`.
6. All four pre-existing failing survival goldens remain green: `survival_drive_escalation_lands_row_four`, `survival_offices_proves_force_law_uptake`, `survival_preferences_keeps_proactive_diversification_alive_under_survival`, `survival_tell_lands_row_five` (regression contract for cross-category neutrality).
7. Existing comparator tests remain green: `same_priority_candidates_sort_by_motive_then_kind_then_ids` (`ranking.rs:7021`), `simultaneous_critical_self_care_needs_rank_by_weighted_order` (`ranking.rs:7430`), `explain_ranked_goal_order_reports_decisive_dimension` (`ranking.rs:9303`).
8. Existing source-reliability tests remain green: `source_reliability_discount_*` (lines 5592–5827, 5983, 6074).
9. Existing suite: `cargo test --workspace`.

### Invariants

1. `motive_score` is never mutated by the composite (Design Goal 1).
2. `SourceComposite` is reached only when both compared entries share `(commodity, purpose)` peer keys (Design Goal 2).
3. `AgendaEntry::pending` populates `source_composite` exactly once per candidate; consumers downstream of `rank_goals` see a fully constructed entry (FND-27 caches).
4. No backward-compat shim: every `AgendaEntry { ... }` literal in-tree includes the new field (FND-28).

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/ranking.rs` — new focused tests for the comparator dimension (item 1-4 above).
2. `crates/worldwake-ai/src/agent_tick/planning.rs` — new sibling of `summarize_ranked_goal_preserves_source_reliability_discount` covering composite preservation.

### Commands

1. `cargo test -p worldwake-ai ranking::tests` (focused).
2. `cargo test -p worldwake-ai agent_tick::planning` (snapshot plumbing).
3. `cargo test -p worldwake-ai --test golden_survival_drive_escalation` (cross-category regression).
4. `cargo test -p worldwake-ai --test golden_survival_offices`.
5. `cargo test -p worldwake-ai --test golden_survival_preferences`.
6. `cargo test -p worldwake-ai --test golden_survival_tell`.
7. `cargo test --workspace` (full).
