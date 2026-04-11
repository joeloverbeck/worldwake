# S89UNITWOPHA-003: Universal two-phase planning tests

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: None
**Deps**: S89UNITWOPHA-001, S89UNITWOPHA-002

## Problem

The universal two-phase planning changes from S89UNITWOPHA-001 and S89UNITWOPHA-002 need dedicated test coverage. Without these tests, regressions to the `TravelToGoal` tactical scoping, barrier satisfaction, and candidate filtering would go undetected. The existing S88 tests cover only `TreatWounds` and `ProduceCommodity` — no focused test yet exercises `TravelToGoal` on representative non-whitelisted remote-goal families or locks in the intentional \"exploration fallback stays unscoped\" boundary.

## Assumption Reassessment (2026-04-11)

1. Test file: `crates/worldwake-ai/src/search/tests.rs` exists. S88 golden tests start at line 7882. Test infrastructure for search tests (helper functions for scenario setup, plan result assertions) is available in the same file.
2. `GoalKind::AcquireCommodity`, `GoalKind::Patrol`, `GoalKind::InvestigateViolation`, `GoalKind::Sleep` are all current variants (34 total, confirmed). Each has `goal_relevant_places()` implementations in `goal_model.rs`.
3. `TravelToGoal` variant, `progress_barrier_satisfied`, `apply_tactical_candidate_filter` with TravelToGoal arm, and `SearchTraceMetadata.tactical_goal` all exist after 001+002 are implemented.

## Architecture Check

1. Tests follow existing S88 test patterns: construct a minimal world state with places, travel connections, and goal-relevant affordances, then assert plan search succeeds with expected shape. No new test infrastructure required.
2. No backwards-compatibility aliasing/shims introduced.

## Verification Layers

1. Remote goal receives TravelToGoal scoping → behavioral search tests (tests 1-3): plan found without budget exhaustion, starts with Travel action toward destination
2. Local goal has no tactical scoping → behavioral search test (test 4): Sleep goal plans succeed, tactical_goal is None or barrier instantly satisfied
3. TravelToGoal barrier correctness → focused unit test (test 5): barrier true at destination, false elsewhere
4. TravelToGoal candidate filter correctness → focused unit test (test 6): only travel-advancing candidates retained when not at destination
5. S88 regression → existing golden tests pass unchanged
6. Single-layer ticket (tests only) — no cross-system mapping applicable

## What to Change

### 1. `search_acquire_commodity_uses_travel_to_goal`

Behavioral search test. Setup: Actor at place A with no water, water source at place B (connected by travel). Goal: `AcquireCommodity { commodity: Water, purpose: SelfConsume }`. Assert: plan is found (not `BudgetExhausted`), first step is Travel toward B. This is the primary regression test for the most common budget-exhaustion scenario observed in simulation.

### 2. `search_patrol_uses_travel_to_goal_for_remote_place`

Behavioral search test. Setup: Actor at place A, patrol target at place B (connected by travel). Goal: `Patrol { place: B }`. Assert: plan routes to B without budget exhaustion. Validates a goal kind that has no commodity prerequisites and purely spatial requirements.

### 3. `search_investigate_uses_travel_to_goal`

Behavioral search test. Setup: Actor at place A, violation at place B (connected by travel). Goal: `InvestigateViolation { violation_id, place: B }`. Assert: plan routes to B. Validates institutional/authority goal kinds that previously budget-exhausted.

### 4. `search_local_sleep_has_no_tactical_goal`

Behavioral search test. Setup: Actor at place with sleep affordance. Goal: `Sleep`. Assert: planning succeeds. Verify that strategic plan is empty or has destination at current place, resulting in `tactical_goal = None` or immediate barrier satisfaction. Validates local-only goals are unaffected by the whitelist removal.

### 5. `search_travel_to_goal_barrier_satisfied_at_destination`

Focused unit test. Construct `TravelToGoal { destination: X }`, construct `PlanningState` with actor at X. Assert `progress_barrier_satisfied` returns true. Construct state with actor at Y. Assert returns false.

### 6. `search_travel_to_goal_candidate_filter`

Focused unit test. Construct `TravelToGoal { destination: X }` with actor not at X. Provide a candidate set with travel and non-travel candidates. Call `apply_tactical_candidate_filter`. Assert only travel-advancing candidates are retained.

## Files to Touch

- `crates/worldwake-ai/src/search/tests.rs` (modify)

## Out of Scope

- Testing all 34 GoalKind variants individually — representative sampling (AcquireCommodity, Patrol, InvestigateViolation, Sleep) covers the key behavioral categories
- Simulation observer re-run — manual verification step, not an automated test
- Performance benchmarks for candidate count reduction
- Modifying existing S88 golden tests

## Acceptance Criteria

### Tests That Must Pass

1. `search_acquire_commodity_uses_travel_to_goal` — plan found, starts with Travel
2. `search_patrol_uses_travel_to_goal_for_remote_place` — plan found, routes to B
3. `search_investigate_uses_travel_to_goal` — plan found, routes to B
4. `search_local_sleep_has_no_tactical_goal` — plan found, no tactical scoping
5. `search_travel_to_goal_barrier_satisfied_at_destination` — barrier true at destination, false elsewhere
6. `search_travel_to_goal_candidate_filter` — only travel candidates retained
7. Existing S88 tests unchanged and passing:
   - `search_treat_wounds_uses_two_phase_pick_up_before_heal`
   - `search_treat_wounds_with_zero_landmarks_preserves_two_phase_plan_shape`
   - `search_produce_commodity_uses_two_phase_pick_up_before_craft`
   - `search_produce_commodity_with_zero_landmarks_preserves_two_phase_plan_shape`
   - `search_trace_metadata_records_two_phase_strategic_and_landmark_details`
8. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. No S88 test is modified — all existing assertions remain as-is
2. New behavioral tests assert plan success (not budget exhaustion) — the contract is that tactical scoping makes multi-location plans tractable
3. Unit tests for barrier and filter are isolated from full search infrastructure — they test `TravelToGoal` methods directly

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/search/tests.rs::search_acquire_commodity_uses_travel_to_goal` — primary regression for budget-exhaustion fix
2. `crates/worldwake-ai/src/search/tests.rs::search_patrol_uses_travel_to_goal_for_remote_place` — spatial-only goal coverage
3. `crates/worldwake-ai/src/search/tests.rs::search_investigate_uses_travel_to_goal` — institutional goal coverage
4. `crates/worldwake-ai/src/search/tests.rs::search_local_sleep_has_no_tactical_goal` — local goal regression guard
5. `crates/worldwake-ai/src/search/tests.rs::search_travel_to_goal_barrier_satisfied_at_destination` — unit: barrier logic
6. `crates/worldwake-ai/src/search/tests.rs::search_travel_to_goal_candidate_filter` — unit: candidate filter logic

### Commands

1. `cargo test -p worldwake-ai search_acquire_commodity_uses_travel_to_goal`
2. `cargo test -p worldwake-ai search_patrol_uses_travel_to_goal`
3. `cargo test -p worldwake-ai search_investigate_uses_travel_to_goal`
4. `cargo test -p worldwake-ai search_local_sleep_has_no_tactical_goal`
5. `cargo test -p worldwake-ai search_travel_to_goal_barrier`
6. `cargo test -p worldwake-ai search_travel_to_goal_candidate_filter`
7. `cargo test -p worldwake-ai` — full crate suite including S88 regressions
8. `cargo clippy --workspace --all-targets -- -D warnings` — no new warnings
