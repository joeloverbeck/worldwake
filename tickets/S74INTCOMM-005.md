# S74INTCOMM-005: Preserve side-benefit-selected branch across PLAN_FINISHED replans

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — plan selection continuity across same-goal sibling replans
**Deps**: S74INTCOMM-002

## Problem

`S74INTCOMM-002` broad verification exposed a production-path contradiction in the merchant side-benefit scenario `combined_market_trip_selected_for_side_benefit`: the opening search correctly selects the home-market bread seller because that branch also carries a lawful `SellCommodity(Firewood)` side benefit, but after travel-step completion the runtime forces `DirtySet::PLAN_FINISHED` and a fresh replan switches to the sibling bread opportunity at the inn. The branch flip destroys the intended combined trip and the merchant never executes the originally selected home-market trade path.

This is not a snapshot-continuation issue. It happens on the post-step full-replan path after progress barriers, so test-only margin overrides would hide a real production contradiction instead of fixing it.

## Assumption Reassessment (2026-04-08)

1. The failing proof surface is `combined_market_trip_selected_for_side_benefit` and `combined_market_trip_selected_for_side_benefit_replays_deterministically` in `crates/worldwake-ai/tests/golden_merchant_selling.rs`. The scenario metadata says equal primary bread opportunities should prefer the home-market seller because that path also carries a lawful `SellCommodity(Firewood)` side benefit, and the merchant should materialize that combined trip without a second market journey.
2. The opening tick still proves the side-benefit selector works: the recorded decision trace at tick 0 selects `AcquireCommodity(SelfConsume)` anchored at the home market (`OpportunityAnchor::Place(general_store)`) and the selected plan reports a `SellCommodity(Firewood)` side benefit at that destination.
3. The branch loss happens later on the full replan path, not on snapshot-only continuation. In `crates/worldwake-ai/src/agent_tick/active_action.rs:210-238`, completing a `PlanTerminalKind::ProgressBarrier` or `GoalSatisfied` step clears `runtime.current_plan` and inserts `DirtySet::PLAN_FINISHED`, forcing fresh planning on the next tick.
4. The observed replacement at runtime is a same-goal sibling swap. The failing trace shows `SelectedPlanReplacementKind::SameGoalSiblingReplaced` in `crates/worldwake-ai/src/agent_tick/planning.rs` after travel legs, with the selected `AcquireCommodity(SelfConsume)` opportunity changing from the home-market seller to the inn seller.
5. The exact shared abstraction boundary under audit is plan-selection continuity for same-goal sibling opportunities after `PLAN_FINISHED`: `active_action.rs` emits the replan signal, `agent_tick/planning.rs` reconstructs candidate plans, `plan_selection.rs` chooses among same-goal siblings, and `side_benefit.rs` contributes bounded secondary value to those candidate plans.
6. This is not the same contract as S74INTCOMM-003. That ticket owns goldens that are genuinely margin-sensitive under snapshot-only continuation plus soak validation. This ticket owns the production contradiction on a non-snapshot replan path.
7. The intended invariant is narrower than “never switch same-goal siblings.” The live contract should preserve or re-select the lawful branch whose combined value remains best after progress, or otherwise make any branch flip explainable from the current authoritative side-benefit/value contract rather than silently discarding the earlier winning branch.

## Architecture Check

1. Fixing same-goal sibling branch stability at the production selection boundary is cleaner than papering over the symptom in a golden. The failure is caused by fresh replanning after `PLAN_FINISHED`, so only the plan-selection / value contract can resolve it honestly.
2. The fix should preserve existing side-benefit boundedness and avoid introducing a sticky ad hoc “remember the old branch” heuristic. If continuity is needed, it should come from the live value/selection contract for same-goal siblings, not from a golden-specific exception.

## Verification Layers

1. Opening search still prefers the home-market branch for side-benefit reasons -> focused merchant golden decision trace on tick 0
2. Post-progress full replanning preserves or lawfully re-selects the intended combined-trip branch -> focused merchant golden action trace + decision trace across the replan boundary
3. Same-goal sibling replacement remains explainable at the planner boundary -> focused runtime/planning proof around `SelectedPlanReplacementKind::SameGoalSiblingReplaced`
4. Workspace-level merchant goldens remain deterministic -> targeted replay golden plus `cargo test --workspace`

## What to Change

### 1. Reassess same-goal sibling selection after `PLAN_FINISHED`

Audit the post-progress replan path across:

- `crates/worldwake-ai/src/agent_tick/active_action.rs`
- `crates/worldwake-ai/src/agent_tick/planning.rs`
- `crates/worldwake-ai/src/plan_selection.rs`
- `crates/worldwake-ai/src/side_benefit.rs`

Determine why the branch that won at tick 0 loses after travel progress even though the scenario’s intended combined-trip value should still favor the home market.

### 2. Fix the production-side branch-stability contradiction

Implement the narrowest lawful fix so that after `PLAN_FINISHED` replanning, the merchant either:

- stays on the home-market combined-trip branch because its current plan value still wins, or
- flips branches only when the current side-benefit / value contract genuinely justifies the change.

Avoid introducing a test-only override or a sticky memory shim that bypasses the live value contract.

### 3. Strengthen proof around same-goal sibling replacement

Add or extend focused coverage so the replan boundary proves why the selected sibling remains stable or changes. If the existing trace surfaces are not sufficient to explain the branch choice, widen the bounded planner-owned provenance surface rather than relying only on missing/observed actions.

### 4. Restore the merchant golden contract honestly

Update `crates/worldwake-ai/tests/golden_merchant_selling.rs` only as needed to prove the corrected lawful behavior. Keep the scenario metadata aligned with the implemented contract and rerun the replay twin.

## Files to Touch

- `crates/worldwake-ai/src/agent_tick/active_action.rs` (modify — only if the replan handoff contract itself is wrong)
- `crates/worldwake-ai/src/agent_tick/planning.rs` (modify — same-goal sibling replan continuity)
- `crates/worldwake-ai/src/plan_selection.rs` (modify — same-goal sibling choice after fresh replanning)
- `crates/worldwake-ai/src/side_benefit.rs` (modify — only if the live value contract is incomplete)
- `crates/worldwake-ai/tests/golden_merchant_selling.rs` (modify — restore the scenario to the corrected production contract)

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
2. After travel-step `PLAN_FINISHED` replanning, the agent does not silently abandon that winning same-goal branch unless the live value/selection contract now justifies a different branch.
3. Any same-goal sibling replacement that still occurs after the fix is explainable from the planner-owned contract rather than only from downstream action outcomes.
4. `cargo clippy --workspace --all-targets -- -D warnings` passes.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_merchant_selling.rs` — restore `combined_market_trip_selected_for_side_benefit` and replay twin to the corrected production contract.
2. `crates/worldwake-ai/src/agent_tick/planning.rs` or nearby focused planner/runtime tests — prove same-goal sibling continuity or justified replacement across `PLAN_FINISHED` replanning.

### Commands

1. `cargo test -p worldwake-ai --test golden_merchant_selling combined_market_trip_selected_for_side_benefit`
2. `cargo test -p worldwake-ai --test golden_merchant_selling`
3. `cargo clippy --workspace --all-targets -- -D warnings`
4. `cargo test --workspace`
