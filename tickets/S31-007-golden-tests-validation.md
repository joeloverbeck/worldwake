# S31-007: Golden Tests for Exhaustion Invalidation End-State

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — tests plus any trace/debug proof surface required by S31-008
**Deps**: S31-004, S31-005, S31-006, S31-008

## Problem

The S31 spec needs integrated proof that the completed exhaustion invalidation architecture avoids both over-invalidation and under-invalidation, preserves save/load parity, and remains explainable under the live golden scenarios that already exposed indefinite-caching failures.

## Assumption Reassessment (2026-03-27)

1. Existing golden coverage already proves this is not merely a missing-test problem. With TTL removed, `cargo test -p worldwake-ai` fails `golden_goal_invalidation_by_another_agent`, `golden_wash_action`, `golden_three_way_need_competition`, and `golden_utility_weight_diversity_in_need_selection` in [golden_ai_decisions.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_ai_decisions.rs).
2. Those failures expose a production contradiction in the current invalidation contract, so this ticket can no longer honestly remain “tests only” in isolation. It depends on `S31-008` completing the missing substrate first.
3. `golden_save_load_round_trip_under_ai` already exists in `crates/worldwake-ai/tests/` and remains the live save/load parity surface for the AI runtime including `exhaustion_cache`.
4. The intended over-invalidation invariant is still valid: unrelated commodity changes must not clear exhausted goals whose search space did not materially change.
5. The intended under-invalidation invariant needs more precision than the old narrative. For needs-driven goals such as `GoalKind::Wash`, `GoalKind::Sleep`, and local `GoalKind::ConsumeOwnedCommodity`, the retry trigger must align with the live planner/candidate-generation decision surface, not merely an arbitrary fixed need delta.
6. Decision traces and action traces remain the preferred debugging surfaces, but the contract under test here is golden E2E because the question is whether the integrated world + planner + runtime retry chain behaves lawfully under live simulation.
7. Reassessment after the failed TTL-removal experiment shows this ticket must also guard against “architecturally clean but explanatorily weak” assertions. If the final implementation relies on richer invalidation facts, the tests should prove those facts through the strongest available trace or state surface rather than only by eventual downstream success.

## Architecture Check

1. This ticket should prove the final architecture, not compensate for missing substrate. That aligns with `docs/FOUNDATIONS.md`: tests should validate a concrete, local, explainable retry contract rather than justify keeping a workaround.
2. The production fix belongs in `S31-008`, and the TTL cleanup belongs in `S31-006`. This ticket should then certify that the combined end-state is robust.

## Verification Layers

1. no over-invalidation -> golden E2E plus decision-trace proof that irrelevant changes do not re-open unrelated exhausted searches
2. no under-invalidation for needs-driven goals -> golden E2E plus the strongest available decision-trace or lower-layer proof for the retry trigger
3. save/load parity -> `golden_save_load_round_trip_under_ai`
4. facility-tagged and blocker-tagged goals do not self-invalidate spuriously -> golden or focused integration coverage, depending on the final substrate

## What to Change

### 1. Add golden proof for no over-invalidation

Add a golden scenario showing that bread consumption does not clear an unrelated exhausted acquisition/search path.

### 2. Add golden proof for no under-invalidation

Add golden scenarios showing that needs-driven exhausted goals retry when their concrete planner-relevant local state changes enough to reopen the search space.

### 3. Keep parity and existing regressions in the required pass set

The existing goldens that previously failed under indefinite caching remain required verification, not optional background coverage.

## Files to Touch

- `crates/worldwake-ai/tests/golden_exhaustion.rs` (new or modify, depending on final placement)
- `crates/worldwake-ai/tests/golden_ai_decisions.rs` (modify if the existing regressions become the canonical proof surface)

## Out of Scope

- Implementing the invalidation substrate itself
- Removing the planner TTL gate directly
- Save-format compatibility cleanup unrelated to golden proof

## Acceptance Criteria

### Tests That Must Pass

1. `golden_goal_invalidation_by_another_agent`
2. `golden_wash_action`
3. `golden_three_way_need_competition`
4. `golden_utility_weight_diversity_in_need_selection`
5. new over-invalidation golden coverage
6. `golden_save_load_round_trip_under_ai`
7. Existing suite: `cargo test --workspace`

### Invariants

1. Irrelevant local changes do not clear unrelated exhausted goals
2. Needs-driven exhausted goals retry when the concrete local decision surface changes enough to make the search space materially different
3. Save/load round-trip preserves the canonical exhaustion runtime behavior under the current live format

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_exhaustion.rs` — over-invalidation proof for unrelated commodity changes
2. `crates/worldwake-ai/tests/golden_exhaustion.rs` or [golden_ai_decisions.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_ai_decisions.rs) — under-invalidation proof for needs-driven retries
3. `crates/worldwake-ai/tests/golden_ai_decisions.rs` — keep the four existing regression goldens in the required verification set

### Commands

1. `cargo test -p worldwake-ai`
2. `cargo test -p worldwake-ai --test golden_ai_decisions golden_wash_action -- --exact`
3. `cargo test -p worldwake-ai --test golden_ai_decisions golden_three_way_need_competition -- --exact`
4. `cargo test -p worldwake-ai --test golden_ai_decisions golden_goal_invalidation_by_another_agent -- --exact`
5. `cargo test -p worldwake-ai --test golden_ai_decisions golden_utility_weight_diversity_in_need_selection -- --exact`
