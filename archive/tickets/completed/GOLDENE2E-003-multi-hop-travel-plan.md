# GOLDENE2E-003: Multi-Hop Travel Plan

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Possible
**Deps**: None

## Problem

Current golden coverage only proves a local acquire path and an adjacent travel-to-food path. It does not prove that the AI runtime can sustain a multi-edge travel plan from a distant origin through several sequential travel actions and then complete the harvest/acquire chain at the destination. An agent placed at a distant location should find and execute a multi-hop route to reach food.

**Coverage gap filled**:
- GoalKind: `AcquireCommodity` via multi-hop travel (tests planner depth)
- Topology: BanditCamp, ForestPath, NorthCrossroads, EastFieldTrail (4 new places used)
- Cross-system chain: Needs pressure → goal generation → Dijkstra pathfinding → multi-edge sequential travel → harvest at destination

## Assumption Reassessment (2026-03-12)

1. `build_prototype_world()` creates edges connecting all 12 places (confirmed in `crates/worldwake-core/src/topology.rs`).
2. The current shortest route from `BanditCamp` to `OrchardFarm` is `BanditCamp -> ForestPath -> NorthCrossroads -> EastFieldTrail -> OrchardFarm` with total travel time 14 ticks. This is a 4-edge route, not a single-edge or adjacent-place case.
3. The planner/search layer already has unit coverage for adjacent travel-to-food and travel-to-trade plans, but there is no golden test proving a 4-edge acquire path through the real AI runtime.
4. `PrototypePlace` enum includes BanditCamp, ForestPath, NorthCrossroads, EastFieldTrail (confirmed).
5. `prototype_place_entity()` can produce `EntityId` for any `PrototypePlace` variant, so this test can use prototype places directly without adding a new alias layer to the harness.
6. The harness does not expose `ground_location()`; the stable place API in this codebase is `World::effective_place()`. Any location assertion in this ticket should use `effective_place()` and, where useful, `World::is_in_transit()`.

## Architecture Check

1. The architectural value here is not just Dijkstra reachability. The missing proof is that candidate generation, planning snapshot horizon, runtime replanning, travel execution, and harvest completion compose cleanly across a 4-edge route.
2. Adding a new harness alias or a `ground_location()` wrapper would be redundant surface area. Prefer existing prototype-place accessors and `effective_place()` unless a helper removes real duplication across multiple tickets.
3. This still fits naturally in `golden_ai_decisions.rs` because it is fundamentally an AI planning/runtime scenario.
4. No shims or compatibility layers. If the runtime cannot complete the route with the current planner horizon/depth, fix the engine directly rather than weakening the scenario.

## What to Change

### 1. Write golden test: `golden_multi_hop_travel_plan`

In `golden_ai_decisions.rs`:
- Agent at BanditCamp, critically hungry, no food locally.
- Orchard Farm has apples (workstation + resource source).
- No food at any intermediate location.
- Run simulation long enough to cover 14 travel ticks plus harvest/replan overhead.
- Assert: the agent leaves `BanditCamp` by observing either `effective_place(agent) != Some(BANDIT_CAMP)` or `is_in_transit(agent)`.
- Assert: the agent eventually reaches `OrchardFarm` and completes the acquisition chain there (for example by harvesting/materializing apples and/or reducing hunger after arrival).
- Keep the assertion focused on the emergent outcome, not the exact per-edge visitation order.

**Expected emergent chain**: Hunger pressure → `AcquireCommodity` goal → planner/runtime sustain sequential travel across 4 edges → arrival at Orchard Farm → harvest/materialization → follow-up acquisition/consumption behavior.

### 2. Add a focused planner-depth regression test if engine work is required

If implementation exposes a planner-horizon or search-depth bug, add a narrow unit test in the affected AI module that proves the engine fix independently of the golden scenario. Do not add speculative unit tests if the golden scenario passes without engine changes.

### 3. Update coverage report

Update `reports/golden-e2e-coverage-analysis.md`:
- Move P3 from Part 3 to Part 1.
- Update topology matrix: 4 new places used.

## Files to Touch

- `crates/worldwake-ai/tests/golden_ai_decisions.rs` (modify — add test)
- `crates/worldwake-ai/src/*` (modify only if an engine limitation is discovered while making the scenario pass)
- `reports/golden-e2e-coverage-analysis.md` (modify — update coverage matrices)

## Out of Scope

- Goal switching during multi-hop travel (that's GOLDENE2E-006)
- Death during travel (that's GOLDENE2E-012)
- Verifying the exact path taken (Dijkstra may choose different routes)
- Travel time optimization

## Engine Discovery Protocol

This ticket is a golden e2e test that exercises emergent behavior through the real AI loop.
If implementation reveals that the engine cannot produce the expected emergent behavior,
the following protocol applies:

1. **Diagnose**: Identify the specific engine limitation (missing candidate generation path, planner op gap, action handler deficiency, belief view gap, etc.).
2. **Do not downgrade the test**: The test scenario defines the desired emergent behavior. Do not weaken assertions or remove expected behaviors to work around engine gaps.
3. **Fix forward**: Implement the minimal, architecturally sound engine change that enables the emergent behavior. Document the change in a new "Engine Changes Made" subsection under "What to Change". Each fix must:
   - Follow existing patterns in the affected module
   - Include focused unit tests for the engine change itself
   - Not introduce compatibility shims or special-case logic
4. **Scope guard**: If the required engine change exceeds this ticket's effort rating by more than one level (e.g., a Small ticket needs a Large engine change), stop and apply the 1-3-1 rule: describe the problem, present 3 options, recommend one, and wait for user confirmation before proceeding.
5. **Document**: Record all engine discoveries and fixes in the ticket's Outcome section upon completion, regardless of whether fixes were needed.

## Acceptance Criteria

### Tests That Must Pass

1. `golden_multi_hop_travel_plan` proves that an agent starting at `BanditCamp` can complete a real multi-hop food-acquisition chain to `OrchardFarm`
2. The scenario observes departure from the starting place via `effective_place()` and/or `is_in_transit()`
3. The scenario observes successful destination-side progress, not just route selection: arrival at `OrchardFarm`, harvest/materialization there, or downstream hunger relief after arrival
4. Simulation completes without deadlock within a bounded tick budget chosen to cover the 14-tick route plus harvest/replan overhead
5. Coverage report `reports/golden-e2e-coverage-analysis.md` updated to reflect explicit multi-hop travel coverage and the newly used places
6. Existing suite: `cargo test -p worldwake-ai --test golden_ai_decisions`
7. Full workspace: `cargo test --workspace` and `cargo clippy --workspace`

### Invariants

1. All behavior is emergent — no manual action queueing
2. Conservation: apple lots never exceed initial resource source quantity
3. Determinism: same seed produces same outcome

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_ai_decisions.rs::golden_multi_hop_travel_plan` — proves multi-hop travel planning
2. Additional focused engine regression test only if a concrete engine defect is discovered while implementing this ticket

## Outcome

### What changed vs originally planned

- Added `golden_multi_hop_travel_plan` in `crates/worldwake-ai/tests/golden_ai_decisions.rs`.
- Added focused AI unit coverage for the discovered engine gaps:
  - `crates/worldwake-ai/src/candidate_generation.rs::remote_harvest_source_within_travel_horizon_emits_acquire_goal`
  - `crates/worldwake-ai/src/plan_selection.rs::same_goal_replanning_replaces_stale_in_progress_plan`
- Updated `reports/golden-e2e-coverage-analysis.md` to record explicit multi-hop travel coverage and the newly exercised places.

### Engine changes made

1. `crates/worldwake-ai/src/candidate_generation.rs`
   - `AcquireCommodity`/`RestockCommodity` candidate generation now searches reachable places within a travel horizon instead of only the actor's current place.
   - This was required because the original ticket assumption was wrong: the planner could solve multi-hop routes, but the AI never emitted a distant acquire goal for the planner to solve.

2. `crates/worldwake-ai/src/plan_selection.rs`
   - Same-goal replanning now adopts the refreshed plan instead of preserving a stale in-progress plan.
   - The golden test exposed a concrete runtime bug here: after the first travel hop, the runtime was keeping the old route prefix and revalidating `travel -> ForestPath` while already standing in `ForestPath`, which produced a false `NoKnownPath` blocker.

### What was intentionally not changed

- No harness `ground_location()` helper was added. The codebase uses `World::effective_place()`, and the original helper proposal was based on a stale API assumption.
- No extra place aliases were added to the harness. `prototype_place_entity(PrototypePlace::...)` was sufficient and kept the test surface smaller.

### Commands

1. `cargo test -p worldwake-ai --test golden_ai_decisions golden_multi_hop_travel_plan`
2. `cargo test --workspace`
3. `cargo clippy --workspace`
