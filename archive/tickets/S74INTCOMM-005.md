# S74INTCOMM-005: Preserve side-benefit-selected branch during in-progress replanning

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — planner-visible current-opportunity continuity during in-progress replanning
**Deps**: S74INTCOMM-002

## Problem

`S74INTCOMM-002` broad verification exposed a production-path contradiction in the merchant side-benefit scenario `combined_market_trip_selected_for_side_benefit`: the opening search correctly selects the home-market bread seller because that branch also carries a lawful `SellCommodity(Firewood)` side benefit, but the home-market `AcquireCommodity(Bread)` opportunity disappears from planner-visible consideration during the first in-progress replan while travel is still underway. The planner then searches only the inn sibling and silently abandons the originally selected combined trip.

This is not a `PLAN_FINISHED` continuity issue. The live failure happens earlier, during re-evaluation of an already committed in-progress plan. Test-only margin overrides would hide a real planner-visibility contradiction instead of fixing it.

## Assumption Reassessment (2026-04-08)

1. The failing proof surface is `combined_market_trip_selected_for_side_benefit` and `combined_market_trip_selected_for_side_benefit_replays_deterministically` in `crates/worldwake-ai/tests/golden_merchant_selling.rs`. The scenario metadata says equal primary bread opportunities should prefer the home-market seller because that path also carries a lawful `SellCommodity(Firewood)` side benefit, and the merchant should materialize that combined trip without a second market journey.
2. The opening tick still proves the side-benefit selector works: the recorded decision trace at tick 0 selects `AcquireCommodity(SelfConsume)` anchored at the home market (`OpportunityAnchor::Place(general_store)`) and the selected plan reports a `SellCommodity(Firewood)` side benefit at that destination.
3. The branch loss happens at the first in-progress replan while the travel plan is still active. Live traces show the opening branch selected at tick 0, travel advancing at tick 1, and by tick 2 the planner re-searches with `SelectedPlanReplacementKind::SameGoalSiblingReplaced`, choosing the inn seller instead.
4. The observed replacement is still a same-goal sibling swap, but the critical upstream failure is that the committed home-market opportunity is no longer planner-visible when same-goal continuation runs. Once an interleaved `SellCommodity(Firewood)` goal sits between the inn sibling and the missing home-market sibling, `build_candidate_plans()` never evaluates the original branch.
5. The exact shared abstraction boundary under audit is in-progress planner continuity for a committed opportunity: read-phase candidate generation / ranking must preserve the current opportunity when its assumptions still hold, and same-goal planning must evaluate that committed sibling before interleaved other goals can terminate sibling continuation.
6. This is not the same contract as S74INTCOMM-003. That ticket owns goldens that are genuinely margin-sensitive under snapshot-only continuation plus soak validation. This ticket owns the production contradiction in the live in-progress replanning path.
7. The intended invariant is narrower than “never switch same-goal siblings.” The live contract should preserve the committed branch as a planner-visible option while its assumptions remain intact, then allow same-goal replacement only when the current planner-visible contract genuinely justifies a different sibling.

## Architecture Check

1. Fixing planner-visible continuity at the committed-opportunity boundary is cleaner than papering over the symptom in a golden. The failure is caused by the active branch dropping out of the planner-visible set during in-progress replanning, so only the read-phase / same-goal planning contract can resolve it honestly.
2. The fix should preserve existing side-benefit boundedness and avoid introducing a new persisted “preferred branch” shim. If continuity is needed, it should come from the already concrete current plan / active goal state, not from a golden-specific exception or a second memory path.

## Verification Layers

1. Opening search still prefers the home-market branch for side-benefit reasons -> focused merchant golden decision trace on tick 0
2. In-progress replanning keeps the committed home-market opportunity planner-visible and actually evaluates it before interleaved goals terminate same-goal continuation -> focused runtime/planning proof
3. Same-goal sibling replacement remains explainable at the planner boundary -> focused merchant golden decision trace across the first replan boundary
4. Workspace-level merchant goldens remain deterministic -> targeted replay golden plus `cargo test --workspace`

## What to Change

### 1. Reassess committed-opportunity visibility during in-progress replanning

Audit the in-progress replan path across:

- `crates/worldwake-ai/src/agent_tick/observation.rs`
- `crates/worldwake-ai/src/agent_tick/planning.rs`
- `crates/worldwake-ai/src/agent_tick/mod.rs`

Determine why the branch that won at tick 0 drops out of planner-visible consideration after travel progress even though its assumptions are still intact.

### 2. Fix the production-side committed-opportunity contradiction

Implement the narrowest lawful fix so that during in-progress replanning, the merchant either:

- keeps the home-market combined-trip branch planner-visible and eligible for same-goal evaluation while the current plan remains valid, or
- flips branches only when the current planner-visible contract genuinely justifies the change.

Avoid introducing a test-only override or a new persisted “preferred branch” memory shim that bypasses the live current-plan contract.

### 3. Strengthen proof around same-goal sibling replacement

Add or extend focused coverage so the first in-progress replan proves why the committed sibling remains stable or changes. If the existing trace surfaces are not sufficient to explain the branch choice, widen the bounded planner-owned provenance surface rather than relying only on missing/observed actions.

### 4. Restore the merchant golden contract honestly

Update `crates/worldwake-ai/tests/golden_merchant_selling.rs` only as needed to prove the corrected lawful behavior. Keep the scenario metadata aligned with the implemented contract and rerun the replay twin.

## Files to Touch

- `crates/worldwake-ai/src/agent_tick/observation.rs` (modified — preserve the committed opportunity in generated candidates only for lawful same-goal sibling continuity cases)
- `crates/worldwake-ai/src/agent_tick/planning.rs` (modified — prioritize the committed opportunity ahead of interleaved goals during same-goal replanning)
- `crates/worldwake-ai/src/agent_tick/active_action.rs` (modified — keep the non-committed call path explicit after narrowing the fix)

## Out of Scope

- Snapshot-only continuation margin logic from S74INTCOMM-002
- Golden/soak validation for other truly margin-sensitive scenarios (S74INTCOMM-003)
- Changing the default `planning_switch_margin` value
- Broad merchant-selling cleanup unrelated to the side-benefit combined-trip contradiction

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-ai --test golden_merchant_selling combined_market_trip_selected_for_side_benefit`
2. `cargo test -p worldwake-ai --test golden_merchant_selling combined_market_trip_selected_for_side_benefit_replays_deterministically`
3. `cargo test -p worldwake-ai --test golden_merchant_selling`
4. Existing suite: `cargo test --workspace`

### Invariants

1. The opening side-benefit selection still prefers the home-market bread opportunity for the lawful `SellCommodity(Firewood)` combined trip.
2. During the first in-progress replan, the committed home-market opportunity remains planner-visible and is evaluated before same-goal continuation stops on an interleaved different goal.
3. Any same-goal sibling replacement that still occurs after the fix is explainable from the planner-owned contract rather than only from downstream action outcomes.
4. `cargo clippy --workspace --all-targets -- -D warnings` passes.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/agent_tick/observation.rs` — prove reinstatement occurs only for lawful same-goal sibling continuity cases.
2. `crates/worldwake-ai/src/agent_tick/planning.rs` — prove the committed opportunity is evaluated ahead of interleaved other-goal candidates during the first in-progress replan.

### Commands

1. `cargo test -p worldwake-ai --test golden_merchant_selling combined_market_trip_selected_for_side_benefit`
2. `cargo test -p worldwake-ai --test golden_merchant_selling`
3. `cargo clippy --workspace --all-targets -- -D warnings`
4. `cargo test --workspace`

## Outcome

Completion date: 2026-04-08

Implemented the live-boundary fix in the in-progress replanning path instead of the stale `PLAN_FINISHED` path. `observation.rs` now reinstates the current plan's committed opportunity only for true same-goal sibling continuity cases where the active goal still matches, the exact committed opportunity has dropped out, and live sibling candidates for that goal still exist. `planning.rs` now carries the committed opportunity into candidate-plan building and prioritizes that committed sibling ahead of interleaved other-goal candidates during same-goal replanning. No new persisted "preferred branch" shim was added; the fix derives continuity from the existing `current_plan` contract.

Focused regression coverage was added in `observation.rs` and `planning.rs` for reinstatement gating and same-goal ordering. The merchant golden pair now passes without scenario edits, which confirmed the production contradiction was in planner-visible continuity rather than in the golden itself.

Deviation from original plan: the ticket was corrected during implementation from a stale `PLAN_FINISHED` replan narrative to the live in-progress replanning boundary before the final fix landed.

## Verification Notes

Passed:

1. `cargo test -p worldwake-ai reinstate_current_plan_candidate`
2. `cargo test -p worldwake-ai committed_opportunity_clusters_same_goal_siblings_ahead_of_interleaved_goals`
3. `cargo test -p worldwake-ai --test golden_merchant_selling combined_market_trip_selected_for_side_benefit`
4. `cargo test -p worldwake-ai --test golden_merchant_selling combined_market_trip_selected_for_side_benefit_replays_deterministically`
5. `cargo test -p worldwake-ai --test golden_merchant_selling`
6. `cargo test -p worldwake-ai --test golden_production golden_facility_queue_patience_timeout`
7. `cargo test -p worldwake-ai --test golden_production golden_local_detour_reuses_existing_grant_before_harvest`
8. `cargo test -p worldwake-ai`
9. `cargo clippy --workspace --all-targets -- -D warnings`
10. `cargo test --workspace`
