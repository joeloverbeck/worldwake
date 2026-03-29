# S35OBSACTSIG-006: Implement ranking discount for observed production competition

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — worldwake-ai ranking pipeline
**Deps**: S35OBSACTSIG-002 (activity_awareness_weight), S35OBSACTSIG-004 (GoalBeliefView.agents_active_at), S35OBSACTSIG-005 (CompetitionDiscount struct)

## Problem

Agents still cannot factor observed production competition into goal ranking. The observable-activity belief path and trace surface are already live, but `rank_candidates()` never consumes them, so production and restock opportunities still rank as if co-located workers are not visibly occupying the same place.

## Assumption Reassessment (2026-03-29)

Shared abstraction boundary under audit: `GroundedGoal` opportunity identity in `worldwake-ai` (`key`, `anchor`, `evidence_entities`, `evidence_places`) as consumed by `rank_candidates()` in [`crates/worldwake-ai/src/ranking.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/ranking.rs).

1. The S35 prerequisite surfaces are already live, not pending:
   - `UtilityProfile.activity_awareness_weight` exists in [`crates/worldwake-core/src/utility_profile.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/utility_profile.rs).
   - `BelievedActivity` and `BelievedEntityState.believed_activity` exist in [`crates/worldwake-core/src/belief.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/belief.rs).
   - `GoalBeliefView::believed_activity_of` and `GoalBeliefView::agents_active_at` exist in [`crates/worldwake-sim/src/belief_view.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/belief_view.rs), with concrete runtime implementations in [`crates/worldwake-sim/src/per_agent_belief_view.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/per_agent_belief_view.rs) and [`crates/worldwake-ai/src/planning_state.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/planning_state.rs).
   - `CompetitionDiscount` and `RankedGoal.competition_discount` already exist in [`crates/worldwake-ai/src/decision_trace.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/decision_trace.rs) and [`crates/worldwake-ai/src/goal_model.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/goal_model.rs).
2. `rank_candidates()` in [`crates/worldwake-ai/src/ranking.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/ranking.rs) currently computes `priority_class`, `motive_score`, and `provenance`, but always writes `competition_discount: None`. The missing work is ranking consumption, not prerequisite plumbing.
3. The live ranking score scale is not the one described in the original ticket. `score_product(weight, pressure)` currently returns the raw permille product (`u32::from(weight.value()) * u32::from(pressure.value())`) in [`crates/worldwake-ai/src/ranking.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/ranking.rs), so motive scores live on a `0..=1_000_000` scale. Any discount expectations must be written against that scale.
4. `ProduceCommodity` and `RestockCommodity` opportunities are place-anchored today and align cleanly with place-scoped observed production activity:
   - `emit_production_goals()` emits `GoalKind::ProduceCommodity` with `OpportunityAnchor::Place(candidate_place)` in [`crates/worldwake-ai/src/candidate_generation.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/candidate_generation.rs).
   - `emit_restock_goals()` emits `GoalKind::RestockCommodity` with `OpportunityAnchor::Place(candidate_place)` in [`crates/worldwake-ai/src/candidate_generation.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/candidate_generation.rs).
5. The original ticket's `AcquireCommodity` assumption is stale. Live acquisition opportunities are aggregated by place via `acquisition_path_opportunities_inner()` in [`crates/worldwake-ai/src/candidate_generation.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/candidate_generation.rs): a single `AcquireCommodity` opportunity at one place can lawfully combine seller, loose-lot, corpse, resource-source, and recipe-path evidence. There is no canonical merchant-specific `anchor.entity()` or acquisition-mode discriminator available to ranking.
6. Because of that mixed opportunity model, applying a Trade-domain competition discount to `AcquireCommodity` in this ticket would force ranking to infer a narrower path than the candidate identity actually represents. That would be architectural drift, not a clean extension.
7. The perception-side observable-activity projection is already implemented in [`crates/worldwake-systems/src/perception.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/perception.rs), including focused tests for set/clear behavior. No perception refactor belongs in this ticket.
8. Existing focused test coverage already proves the prerequisite layers:
   - perception activity belief set/clear tests in [`crates/worldwake-systems/src/perception.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/perception.rs)
   - belief-view activity query tests in [`crates/worldwake-sim/src/per_agent_belief_view.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/per_agent_belief_view.rs)
   - planning snapshot activity query tests in [`crates/worldwake-ai/src/planning_state.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/planning_state.rs)
9. Corrected scope:
   - In scope: apply observed competition discount to `ProduceCommodity` and `RestockCommodity`, both using `ActionDomain::Production` at the grounded opportunity place.
   - Out of scope for this ticket: `AcquireCommodity` competition discount. That requires a follow-up ticket to split or annotate acquisition opportunities by canonical path before ranking can cleanly distinguish trade from non-trade acquisition.

## Architecture Check

1. Applying the discount inside ranking after motive computation and before `RankedGoal` construction is still the right insertion point. It changes only ranking arithmetic and trace payload, leaving candidate generation, suppression, and planner contracts untouched.
2. The clean end-state for this ticket is narrower than the original plan: production-side goals only. `ProduceCommodity` and `RestockCommodity` already have place-scoped opportunity identity, and `agents_active_at(place, ActionDomain::Production, None)` matches that identity exactly.
3. Extending the discount to `AcquireCommodity` now would be less robust than the current architecture because ranking would have to reverse-engineer trade intent from a place-aggregated candidate that can also represent loose-lot pickup, corpse loot, resource harvesting, or recipe-path acquisition. That is a false precision problem under P3/P24.
4. The cleaner long-term architecture for trade competition is a follow-up that gives acquisition opportunities a canonical path discriminator or finer opportunity identity at candidate-generation time. Once ranking receives that clean data contract, trade competition can be added without aliasing or heuristics.
5. Capping observed competitors at 3 and flooring post-discount motive at 1 remain good dampeners. They reduce dogpiling pressure without creating invisible hard suppression.

## Verification Layers

1. Production/restock motive discount arithmetic -> focused `ranking.rs` unit tests
2. Competitor cap at 3 -> focused `ranking.rs` unit test
3. Zero-weight and motive-floor behavior -> focused `ranking.rs` unit tests
4. `CompetitionDiscount` trace payload population/absence -> focused `ranking.rs` unit tests
5. Belief-only read path -> compile-time/runtime boundary already enforced by `GoalBeliefView` usage in [`crates/worldwake-ai/src/ranking.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/ranking.rs)
6. Prerequisite observable-activity belief/query path -> existing focused tests in perception and belief-view layers; no new mixed-layer proof required for this ranking-only ticket

## What to Change

### 1. Add production-competition discount logic to `rank_candidates()`

In [`crates/worldwake-ai/src/ranking.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/ranking.rs), after motive score computation for each candidate and before `RankedGoal` construction:

```rust
let discount_place = match candidate.key.kind {
    GoalKind::ProduceCommodity { .. } | GoalKind::RestockCommodity { .. } => match candidate.anchor {
        OpportunityAnchor::Place(place) => Some(place),
        _ => None,
    },
    _ => None,
};

if let Some(place) = discount_place {
    let competitors = view.agents_active_at(place, ActionDomain::Production, None);
    if !competitors.is_empty() {
        let count = (competitors.len() as u32).min(3);
        let weight = u32::from(utility.activity_awareness_weight.value());
        let factor = 1000u32.saturating_sub(weight.saturating_mul(count));
        let pre_discount = motive_score;
        motive_score = (motive_score.saturating_mul(factor) / 1000).max(1);
        competition_discount = Some(CompetitionDiscount {
            observed_competitors: competitors,
            domain: ActionDomain::Production,
            effective_discount: Permille::new((1000 - factor) as u16).unwrap(),
            pre_discount_motive: pre_discount,
            post_discount_motive: motive_score,
        });
    }
}
```

### 2. Keep `AcquireCommodity` out of scope until opportunity identity is explicit

Open a follow-up ticket for trade competition once acquisition opportunities carry a canonical transport-path discriminator or finer-grained anchor contract.

## Files to Touch

- `crates/worldwake-ai/src/ranking.rs` (modify — add discount logic and focused tests)

## Out of Scope

- Perception system changes (S35OBSACTSIG-003)
- Any perception-pipeline refactor that unifies passive/entity and active/activity direct-observation bookkeeping
- `GoalBeliefView` activity query implementation (already shipped)
- `BelievedActivity` type / belief-store plumbing (already shipped)
- `UtilityProfile.activity_awareness_weight` definition (already shipped)
- `CompetitionDiscount` struct definition (already shipped)
- Golden tests (S35OBSACTSIG-007 or follow-up)
- Discount for `AcquireCommodity`, Combat, Travel, or Needs domains
- Candidate-generation refactor to split acquisition opportunities by canonical path

## Acceptance Criteria

### Tests That Must Pass

1. Ranking discount applied proportionally: 1 competitor with default weight -> 20% discount, 2 -> 40%, 3 -> 60%.
2. Discount capped at 3 competitors: 4 competitors produces same discount as 3.
3. `activity_awareness_weight = Permille(0)` -> zero discount regardless of competitor count.
4. `activity_awareness_weight = Permille(500)` -> 50% discount per competitor (capped at 3).
5. Motive score never drops below 1.
6. `ProduceCommodity` goal discounted when Production competitors observed.
7. `RestockCommodity` goal discounted when Production competitors observed.
8. `AcquireCommodity(SelfConsume)`, `AcquireCommodity(Restock)`, `Sleep`, and `TreatWounds` goals are NOT discounted by this ticket.
9. `CompetitionDiscount` trace populated with correct fields when discount applied.
10. `CompetitionDiscount` is `None` when no competitors observed.
11. Existing focused prerequisite suites still pass unchanged.

### Invariants

1. Discount only reads from `GoalBeliefView` — never from authoritative scheduler state (P12).
2. Discount never suppresses a goal entirely — `.max(1)` floor (P10).
3. Priority class is unaffected by competition discount — only motive score changes.
4. Competitor count cap at 3 prevents extreme discounting.
5. Ranking does not infer merchant-specific trade competition from place-aggregated `AcquireCommodity` opportunities.

## Test Plan

### New/Modified Tests

1. `ranking::tests::production_competition_discount_applies_to_restock_goals`
   Rationale: proves the main shipped behavior on a real ranked `RestockCommodity` opportunity, including the populated `CompetitionDiscount` trace payload.
2. `ranking::tests::production_competition_discount_caps_at_three_competitors`
   Rationale: proves the dampener cap holds even when more than three observed competitors are present.
3. `ranking::tests::production_competition_discount_respects_zero_awareness_weight`
   Rationale: proves observed competition can be recorded while producing no score change for agents configured to ignore it.
4. `ranking::tests::production_competition_discount_floors_positive_motive_at_one`
   Rationale: proves the discount never erases a positive motive into zero.
5. `ranking::tests::acquire_commodity_is_not_discounted_by_observed_production_activity`
   Rationale: locks in the corrected scope and prevents future accidental heuristic discounting of place-aggregated acquisition opportunities.

### Commands

1. `cargo test -p worldwake-ai ranking::tests::`
2. `cargo test -p worldwake-ai`
3. `cargo test --workspace`
4. `cargo clippy --workspace`

## Outcome

- Completed: 2026-03-29
- What actually changed:
  - Implemented production-competition discount consumption in [`crates/worldwake-ai/src/ranking.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/ranking.rs) for `ProduceCommodity` and `RestockCommodity` when the grounded opportunity is place-anchored and the agent believes co-located actors are already active in `ActionDomain::Production`.
  - Populated `RankedGoal.competition_discount` from live ranking results instead of always leaving it `None`.
  - Added focused ranking tests and updated the ranking test double to expose believed activity through `agents_active_at`.
- Deviations from original plan:
  - The ticket was narrowed before implementation. `AcquireCommodity` discounting was not shipped because the live acquisition opportunity model is place-aggregated and lacks a canonical merchant/path discriminator; shipping trade discounting here would have introduced heuristic aliasing.
  - No perception, belief-store, or trace-model prerequisite work was needed because those surfaces were already implemented.
- Verification results:
  - `cargo test -p worldwake-ai ranking::tests::` ✅
  - `cargo test -p worldwake-ai` ✅
  - `cargo test --workspace` ✅
  - `cargo clippy --workspace` ✅
