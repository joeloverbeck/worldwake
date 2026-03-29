# S35OBSACTSIG-006: Implement ranking discount for observed competition

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — worldwake-ai ranking pipeline
**Deps**: S35OBSACTSIG-002 (activity_awareness_weight), S35OBSACTSIG-004 (GoalBeliefView.agents_active_at), S35OBSACTSIG-005 (CompetitionDiscount struct)

## Problem

Agents cannot factor observed competition into their goal ranking. When multiple agents are at the same resource source, they blindly compete rather than considering alternatives. This ticket implements the competition discount in `rank_candidates()`.

## Assumption Reassessment (2026-03-29)

1. `rank_candidates()` at `crates/worldwake-ai/src/ranking.rs:70` computes `motive_score` via `score_product(weight: Permille, pressure: Permille) -> u32` (returns `weight.value() as u32 * pressure.value() as u32 / 1000`). Motive scores are `u32`.
2. After motive score computation, candidates are collected into `RankedGoal` structs and sorted. The discount must be applied between motive computation and `RankedGoal` construction.
3. `GoalKind` variants relevant to discount: `ProduceCommodity`, `RestockCommodity` (Production domain), `AcquireCommodity` (Trade domain when trade-bound). Need to verify how `AcquireCommodity` is distinguished as trade-bound vs other acquisition.
4. `GroundedGoal` has `anchor: OpportunityAnchor` which provides `.place()` and `.entity()` for target resolution.
5. `UtilityProfile` will have `activity_awareness_weight: Permille` after S35OBSACTSIG-002.
6. `GoalBeliefView` will have `agents_active_at(place, domain, target)` after S35OBSACTSIG-004.
7. `RankedGoal` will have `competition_discount: Option<CompetitionDiscount>` after S35OBSACTSIG-005.
8. The discount formula per spec: `factor = 1000 - (weight * min(competitors, 3))`, `motive = (motive * factor / 1000).max(1)`.

## Architecture Check

1. Applying discount after motive computation and before `RankedGoal` construction is the cleanest insertion point — it modifies the score without altering the priority class or suppression logic.
2. The discount is a pure function of belief queries — no authoritative state access (P12).
3. Capping competitors at 3 prevents extreme suppression. `.max(1)` ensures goals are never fully eliminated (P10 dampener).
4. Domain-scoped: only Production and Trade goals are affected. Needs goals (eat, drink, sleep) are never discounted — survival cannot be discouraged by competition.
5. No backward compatibility issues — new code path, default weight produces default behavior.

## Verification Layers

1. Discount proportional to competitor count -> focused unit test (1, 2, 3 competitors)
2. No discount beyond 3 competitors -> focused unit test (4 competitors same as 3)
3. `activity_awareness_weight = Permille(0)` -> no discount -> focused unit test
4. Motive floor at 1 -> focused unit test (high weight, max competitors)
5. No discount on Needs goals -> focused unit test
6. `CompetitionDiscount` trace populated when discount applied -> focused unit test
7. `CompetitionDiscount` trace absent when no competitors -> focused unit test
8. Ranking discount applied only from belief data -> architectural invariant verified by trait boundary

## What to Change

### 1. Add competition discount logic to `rank_candidates()`

In `crates/worldwake-ai/src/ranking.rs`, after motive score computation for each candidate and before `RankedGoal` construction:

```rust
// Determine if this goal kind is discountable
let discount_params = match &candidate.goal_key.kind {
    GoalKind::ProduceCommodity { .. } | GoalKind::RestockCommodity { .. } => {
        Some((ActionDomain::Production, candidate.anchor.place(), None))
    }
    GoalKind::AcquireCommodity { .. } if is_trade_bound(candidate) => {
        Some((ActionDomain::Trade, agent_place, candidate.anchor.entity()))
    }
    _ => None,
};

if let Some((domain, place, target)) = discount_params {
    if let Some(place_id) = place {
        let competitors = view.agents_active_at(place_id, domain, target);
        if !competitors.is_empty() {
            let weight_val = u32::from(utility.activity_awareness_weight.value());
            let count = (competitors.len() as u32).min(3);
            let factor = 1000u32.saturating_sub(weight_val * count);
            let pre_discount = motive_score;
            motive_score = (motive_score * factor / 1000).max(1);
            competition_discount = Some(CompetitionDiscount {
                observed_competitors: competitors,
                domain,
                effective_discount: Permille((1000 - factor) as u16),
                pre_discount_motive: pre_discount,
                post_discount_motive: motive_score,
            });
        }
    }
}
```

### 2. Determine `is_trade_bound` classification

Need to identify how `AcquireCommodity` goals are classified as trade-bound. This likely involves checking `CommodityPurpose` or the anchor type. Verify during implementation.

## Files to Touch

- `crates/worldwake-ai/src/ranking.rs` (modify — add discount logic in `rank_candidates()`)

## Out of Scope

- Perception system changes (S35OBSACTSIG-003)
- `GoalBeliefView` implementation (S35OBSACTSIG-004, prerequisite)
- `BelievedActivity` type (S35OBSACTSIG-001)
- `UtilityProfile` field definition (S35OBSACTSIG-002, prerequisite)
- `CompetitionDiscount` struct definition (S35OBSACTSIG-005, prerequisite)
- Golden tests (S35OBSACTSIG-007)
- Discount for Combat, Travel, or Needs domains

## Acceptance Criteria

### Tests That Must Pass

1. Ranking discount applied proportionally: 1 competitor with default weight -> 20% discount, 2 -> 40%, 3 -> 60%.
2. Discount capped at 3 competitors: 4 competitors produces same discount as 3.
3. `activity_awareness_weight = Permille(0)` -> zero discount regardless of competitor count.
4. `activity_awareness_weight = Permille(500)` -> 50% discount per competitor (capped at 3).
5. Motive score never drops below 1.
6. `ProduceCommodity` goal discounted when Production competitors observed.
7. `RestockCommodity` goal discounted when Production competitors observed.
8. `AcquireCommodity` (trade-bound) goal discounted when Trade competitors observed for same target.
9. `SelfConsumeCommodity`, `Sleep`, `Heal` goals NOT discounted.
10. `CompetitionDiscount` trace populated with correct fields when discount applied.
11. `CompetitionDiscount` is `None` when no competitors observed.
12. Existing suite: `cargo test --workspace` (all golden tests pass unchanged)

### Invariants

1. Discount only reads from `GoalBeliefView` — never from authoritative scheduler state (P12).
2. Discount never suppresses a goal entirely — `.max(1)` floor (P10).
3. Priority class is unaffected by competition discount — only motive score changes.
4. Competitor count cap at 3 prevents extreme discounting.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/ranking.rs` — focused unit tests for discount arithmetic, domain scoping, weight variation, competitor capping, and motive floor.

### Commands

1. `cargo test -p worldwake-ai`
2. `cargo test --workspace`
3. `cargo clippy --workspace`
