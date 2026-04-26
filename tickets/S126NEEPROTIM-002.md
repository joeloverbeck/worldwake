# S126NEEPROTIM-002: populate_assumptions need-horizon extension

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — extends `populate_assumptions` to derive per-need `NeedSafeUntilTick` assumptions when the agent's projected need-high crossing falls before the plan's completion tick. Adds `current_tick: Tick` and `plan_completion_tick: Tick` to the function signature; updates 6 production call sites and 7 existing unit tests.
**Deps**: S126NEEPROTIM-001

## Problem

After ticket 001 lands, `FrameAssumption::NeedSafeUntilTick` exists and `HomeostaticNeeds::projected_tick_of` exists, but no caller produces the new assumption. This ticket wires `populate_assumptions` (`crates/worldwake-ai/src/agent_tick/frame.rs:280`) to derive per-need projection assumptions for the active intention frame, per spec D4. The result: an agent with a multi-step plan whose completion tick exceeds the projected hunger-high crossing now carries a concrete `NeedSafeUntilTick { need: Hunger, until_tick: <plan_completion_tick> }` entry on the frame, available to ticket 003's evaluator.

The signature change ripples to every `populate_assumptions` call site. The two new parameters (`current_tick`, `plan_completion_tick`) are computed at the call site from `tick` (already in scope at every site) and `runtime.current_plan.total_estimated_ticks` minus the duration of completed steps (`runtime.current_step_index`).

## Assumption Reassessment (2026-04-26)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. `populate_assumptions` (`crates/worldwake-ai/src/agent_tick/frame.rs:280-329`) currently has signature `(frame: &IntentionFrame, agent: EntityId, view: &dyn RuntimeBeliefView) -> Vec<FrameAssumption>`. The function iterates `IntentionDomain` variants and pushes domain-specific assumptions (`RouteExists`, `CommodityAvailableAt`, `TargetAlive`, `NoCriticalThreat`); after this ticket it also pushes per-need `NeedSafeUntilTick` entries. Existing unit tests in `agent_tick/frame.rs::tests`: `populate_travel_produces_route_exists` (line 981), `populate_care_produces_target_alive_and_route` (line 1004), `populate_escort_produces_target_alive_and_route` (line 1030), `populate_errand_produces_route_exists` (line 1061), `populate_generic_produces_no_critical_threat` (line 1084), `populate_travel_with_acquire_commodity_produces_route_and_commodity` (line 1094), and the assess-commodity-availability tests at lines 950 and 968 (these don't call `populate_assumptions` directly but are sibling tests that may need parameter context). All 6 unit tests calling `populate_assumptions` directly need the two new arguments.
2. Spec authority: `specs/S126-need-projection-time-budget.md` D4 and Design Goal 2.
3. Shared abstraction boundary: this ticket changes the `populate_assumptions` signature — both the function definition and the call-site contract. The contract now requires the caller to compute `plan_completion_tick` (from `runtime.current_plan.total_estimated_ticks` and `runtime.current_step_index`); the function itself remains a pure read against `RuntimeBeliefView` plus per-need projection arithmetic.
4. The 6 production call sites are: `agent_tick/mod.rs:1027`, `agent_tick/planning.rs:1680`, `agent_tick/planning.rs:2114`, `agent_tick/planning.rs:2751`, `agent_tick/planning.rs:2847` (5 distinct call sites, plus the import at `mod.rs:21` and the import at `planning.rs:43`). At every call site, `tick: Tick` (or `current_tick`) is in scope and `runtime: &mut AgentDecisionRuntime` is in scope — the caller can read `runtime.current_plan.as_ref().map(|p| p.total_estimated_ticks)` and `runtime.current_step_index` to compute remaining ticks.
5. `RuntimeBeliefView` (`crates/worldwake-sim/src/belief_view.rs:1223-1236`) inherits from `ProfileBeliefView` (line 1226), which exposes `homeostatic_needs(agent)` (line 742), `drive_thresholds(agent)` (line 743), and `metabolism_profile(agent)` (line 752). All three return `Option<T>` — when any is `None` for a given agent, this ticket's per-need population must skip the projection arms (the agent is missing required physiology profiles; reactive ranking continues to handle it).
6. The harness boundary for new behavioral tests: local needs-only harness is sufficient for `populate_assumptions` unit tests because the function reads only the belief view and frame; no action registries are required. The mock belief view used by existing tests (search `MockBeliefView` or equivalent in `agent_tick/frame.rs::tests`) needs `metabolism_profile`, `drive_thresholds`, and `homeostatic_needs` stubs returning realistic `Some(...)` values for the new arms.
7. Plan-completion-tick computation at call sites: when `runtime.current_plan` is `Some(plan)`, the caller computes `remaining_ticks = plan.total_estimated_ticks.saturating_sub(sum_of_completed_step_durations)` and passes `current_tick + remaining_ticks` as `plan_completion_tick`. When `runtime.current_plan` is `None`, the caller passes `current_tick` itself as `plan_completion_tick` — the per-need test `breach_tick < plan_completion_tick` (where `breach_tick >= current_tick`) trivially fails, suppressing all `NeedSafeUntilTick` population. Sum-of-completed-step-durations is the sum of `plan.steps[..runtime.current_step_index].iter().map(|s| u64::from(s.estimated_ticks)).sum::<u64>()`. `PlannedStep.estimated_ticks: u32` per `crates/worldwake-ai/src/planner_ops.rs:823`.
8. Adjacent contradictions: the spec says "When `runtime.current_plan` is `None`, the caller passes `current_tick` for `plan_completion_tick`, which trivially makes every `breach_tick < current_tick` test false and skips need-horizon assumption population." This is a required consequence of this ticket's contract, not a separate bug — the no-plan case is by design.

## Architecture Check

1. The signature change keeps the function pure with respect to authoritative state: it reads only the belief view and the per-call parameters, returning a `Vec<FrameAssumption>`. The plan-completion-tick stays out of `IntentionFrame` (no new authoritative field) per spec Design Goal 2 — passing it as a parameter avoids a parallel authoritative copy of plan timing that would violate FND-3.
2. Per-tick re-evaluation (the function is called every tick from `mod.rs:1027` and from each planning replan path) means the population is idempotent for stable goals and self-correcting when physiology shifts. No backward-compatibility shims around the old signature.
3. `RuntimeBeliefView` accessors are the locality-respecting read surface — using them rather than raw `world.get_component_*` calls keeps every other AI consumer's read pattern uniform with this one (FND-7, FND-14A).

## Verification Layers

1. `populate_assumptions` produces a `NeedSafeUntilTick` assumption when projected hunger high crosses before plan completion → focused unit test in `agent_tick/frame.rs::tests`.
2. `populate_assumptions` produces NO `NeedSafeUntilTick` assumption when projected hunger high crosses AFTER plan completion → focused unit test in `agent_tick/frame.rs::tests`.
3. `populate_assumptions` skips need-horizon population when `plan_completion_tick == current_tick` (no plan branch) → focused unit test in `agent_tick/frame.rs::tests`.
4. `populate_assumptions` skips need-horizon population when `metabolism_profile`, `drive_thresholds`, or `homeostatic_needs` returns `None` for the agent → focused unit test in `agent_tick/frame.rs::tests`.
5. `populate_assumptions` produces one `NeedSafeUntilTick` per breaching need across all 5 `HomeostaticNeedId` variants → focused unit test in `agent_tick/frame.rs::tests`.
6. Single-layer ticket (focused unit coverage) — additional layer mapping (action trace, event-log delta) is not applicable because this ticket changes a derived assumption-population helper that produces a `Vec<FrameAssumption>` value; no actions are committed and no event-log entries are emitted by this layer.

## What to Change

### 1. Update `populate_assumptions` signature

In `crates/worldwake-ai/src/agent_tick/frame.rs:280-284`, extend the signature with two new parameters:

```rust
pub(super) fn populate_assumptions(
    frame: &IntentionFrame,
    agent: EntityId,
    view: &dyn RuntimeBeliefView,
    current_tick: Tick,
    plan_completion_tick: Tick,
) -> Vec<FrameAssumption>
```

### 2. Append per-need projection assumptions

After the existing domain-keyed assumption population (the `match *domain { ... }` arms returning `assumptions` at lines 287-328), but before the function returns, append per-need projection logic per spec D4:

```rust
let (Some(metabolism), Some(needs), Some(thresholds)) = (
    view.metabolism_profile(agent),
    view.homeostatic_needs(agent),
    view.drive_thresholds(agent),
) else {
    return assumptions;
};
for &need in &HomeostaticNeedId::ALL {
    let projected = needs.projected_tick_of(
        need,
        thresholds.high(need),
        metabolism.rate(need),
        current_tick,
    );
    if let Some(breach_tick) = projected
        && breach_tick < plan_completion_tick
    {
        assumptions.push(FrameAssumption::NeedSafeUntilTick {
            need,
            until_tick: plan_completion_tick,
        });
    }
}
```

The `current` arm structure (each domain branch returns a `Vec` directly) needs adjustment so the per-need block runs after every domain. Restructure to bind `let mut assumptions = match *domain { ... }` then run the per-need block on the bound vec, then return it.

### 3. Update production call sites

Update the 5 call sites to compute and pass the two new arguments. At each site, `tick` (or equivalent) and `runtime` are in scope:

- `crates/worldwake-ai/src/agent_tick/mod.rs:1027` — the per-tick frame-assumption refresh. Compute `plan_completion_tick = tick + runtime.current_plan.as_ref().map_or(0, |p| p.total_estimated_ticks.saturating_sub(completed_step_ticks(p, runtime.current_step_index)) as u64)`.
- `crates/worldwake-ai/src/agent_tick/planning.rs:1680, 2114, 2751, 2847` — apply the same computation pattern. `current_tick` is in scope at each site.

Add a small private helper at the top of `agent_tick/frame.rs` (or in `agent_tick/mod.rs` next to the call sites) to deduplicate the `completed_step_ticks` computation:

```rust
fn completed_step_ticks(plan: &PlannedPlan, current_step_index: u8) -> u64 {
    plan.steps
        .iter()
        .take(usize::from(current_step_index))
        .map(|step| u64::from(step.estimated_ticks))
        .sum()
}
```

Confirm the type of `current_step_index` against current code (`u8`, `u16`, or `usize`); adjust the `usize::from` accordingly.

### 4. Update existing unit tests (7 sites)

Update the 6 existing `populate_assumptions` test calls in `agent_tick/frame.rs::tests` (lines 993, 1016, 1047, 1073, 1089, 1115) and the 1 test that calls populate via the multi-arm path (line 993, the travel test). For each, append `current_tick: Tick(0), plan_completion_tick: Tick(0)` (or another safe pair where `plan_completion_tick <= breach_tick` so the per-need population trivially returns nothing) — this preserves the existing assertions on domain-specific assumptions while not affecting the test's intent.

### 5. Add new unit tests for need-horizon population

Add new focused unit tests in `agent_tick/frame.rs::tests` covering:
- `populate_produces_need_safe_until_tick_when_breach_before_plan_completion` — set hunger=400, hunger_rate=50, hunger.high()=700, current_tick=Tick(10), plan_completion_tick=Tick(20). Expected: `breach_tick = 10 + ⌈(700-400)/50⌉ = 10 + 6 = Tick(16)`, which is less than 20 → assumption pushed.
- `populate_omits_need_safe_until_tick_when_breach_after_plan_completion` — same setup but plan_completion_tick=Tick(15). Expected: `breach_tick = 16 >= 15` → no assumption.
- `populate_omits_need_safe_until_tick_when_no_plan` — plan_completion_tick == current_tick. Expected: no assumption regardless of physiology.
- `populate_skips_need_horizon_when_profile_missing` — mock view returns `None` for `metabolism_profile`. Expected: no need-horizon assumption (other domain assumptions still produced).
- `populate_produces_need_safe_until_tick_per_breaching_need` — multiple needs at high pressure with fast rates and a far-future plan_completion_tick. Expected: one `NeedSafeUntilTick` assumption per breaching need.

The mock belief view used by existing tests likely needs `metabolism_profile`, `drive_thresholds`, and `homeostatic_needs` stubs. Extend the existing mock in the test module to support `Some(...)` returns for new tests; preserve `None`-returning behavior for old tests where the per-need population should skip.

## Files to Touch

- `crates/worldwake-ai/src/agent_tick/frame.rs` (modify) — update `populate_assumptions` signature and body, update 7 existing tests, add 5 new tests, possibly add `completed_step_ticks` helper
- `crates/worldwake-ai/src/agent_tick/mod.rs` (modify) — update `populate_assumptions` call site at line 1027 + import line 21 if helper moves
- `crates/worldwake-ai/src/agent_tick/planning.rs` (modify) — update 4 `populate_assumptions` call sites at lines 1680, 2114, 2751, 2847 + import line 43 if helper moves

## Out of Scope

- `evaluate_assumptions` arm replacement (replaces ticket 001's placeholder) — ticket 003 (D5)
- `record_assumption_failure` extension to write `Discrepancy::NeedHorizonExceeded` — ticket 003 (D6 part 2)
- Golden test coverage for the full assumption lifecycle — ticket 004 (D8)
- Activity-multiplier-aware projection — explicit non-goal per spec Design Goal 7 and Non-Goals

## Acceptance Criteria

### Tests That Must Pass

1. New unit test: `populate_produces_need_safe_until_tick_when_breach_before_plan_completion`.
2. New unit test: `populate_omits_need_safe_until_tick_when_breach_after_plan_completion`.
3. New unit test: `populate_omits_need_safe_until_tick_when_no_plan`.
4. New unit test: `populate_skips_need_horizon_when_profile_missing`.
5. New unit test: `populate_produces_need_safe_until_tick_per_breaching_need`.
6. Existing tests: `populate_travel_produces_route_exists`, `populate_care_produces_target_alive_and_route`, `populate_escort_produces_target_alive_and_route`, `populate_errand_produces_route_exists`, `populate_generic_produces_no_critical_threat`, `populate_travel_with_acquire_commodity_produces_route_and_commodity` all still pass with the updated call signatures.
7. Existing suite: `cargo test -p worldwake-ai --lib agent_tick::frame::tests` passes.
8. Existing suite: `cargo test --workspace` passes.
9. Existing suite: `cargo clippy --workspace --all-targets -- -D warnings` passes.

### Invariants

1. `populate_assumptions` returns the same domain-keyed assumptions as before for callers that pass `plan_completion_tick == current_tick` (the no-plan branch) — backward behavior preserved when no plan is active.
2. `populate_assumptions` produces at most 1 `NeedSafeUntilTick` per `HomeostaticNeedId` variant (no duplicates).
3. The per-need population block does not call into authoritative world state directly — all reads go through `RuntimeBeliefView` accessors (FND-7, FND-14A locality).
4. The per-call cost remains O(N) in the number of homeostatic needs (5), bounded and deterministic.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/agent_tick/frame.rs` — 5 new tests under `mod tests` covering the projection-population branches; 7 existing test sites updated to pass the two new parameters.

### Commands

1. `cargo test -p worldwake-ai --lib agent_tick::frame::tests::populate`
2. `cargo test -p worldwake-ai --lib agent_tick::frame::tests`
3. `cargo test -p worldwake-ai`
4. `cargo build --workspace`
5. `./scripts/verify.sh`
