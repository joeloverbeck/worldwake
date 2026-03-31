# S40REMPUR-004: Remote candidate generation for RaidTarget and EngageHostile

**Status**: ✅ COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Deps**: S40REMPUR-001 (PursuitProfile), S40REMPUR-002 (pursuit_target_belief), S40REMPUR-003 (Travel in combat ops)

## Problem

`emit_raid_target_goals()` and `emit_engage_hostile_goals()` in `candidate_generation.rs` only emit candidates for co-located targets (via `local_raid_targets()` and `local_hostility_targets()`). Agents with a lawful belief about a hostile target's remote location cannot plan pursuit. This ticket extends both functions to emit the same `RaidTarget`/`EngageHostile` goal kinds for remote targets when pursuit-profile constraints are satisfied.

## Assumption Reassessment (2026-03-30)

1. `emit_engage_hostile_goals()` at `candidate_generation.rs:~1395-1456` calls `local_hostility_targets()` which filters to `effective_place(*target) == Some(place)` (co-located only).
2. `emit_raid_target_goals()` at `candidate_generation.rs:~1458-1501` calls `local_raid_targets()` which does the same co-location filter.
3. `pursuit_target_belief()` (from S40REMPUR-002) will provide the believed remote place for a target.
4. `belief_confidence()` at `belief.rs:1168` derives confidence from `(source, staleness_ticks, policy)`.
5. `PursuitProfile.min_location_confidence` (S40REMPUR-001) is the threshold.
6. Route cost to believed place can be computed via `planning_snapshot.rs` travel cost methods or topology pathfinding.
7. `PursuitProfile.max_pursuit_travel_ticks` bounds the allowed route cost.
8. `BlockedIntentMemory` (`blocked_intent.rs`) must be checked — no pursuit if the target/place combination is blocked.
9. The GoalKind emitted is the SAME as the local variant (`RaidTarget { target }` or `EngageHostile { target }`). No new goal kind.
10. `GoalBeliefView::belief_confidence_policy()` is at `belief_view.rs:~145`.
11. No adjacent contradictions exposed.

## Architecture Check

1. Extending existing emission functions (or adding parallel remote-emission helpers called from the same dispatch point) is cleaner than creating new goal kinds. The spec explicitly requires reusing `RaidTarget`/`EngageHostile` — the difference is purely in how the goal is reached (prerequisite travel vs. local).
2. The confidence/range/blocker checks are candidate-generation-time filters. They do NOT add new state or new goal semantics.
3. No backwards-compatibility shims.

## Verification Layers

1. Remote candidate emission → focused unit test: set up remote target with high-confidence belief, verify goal emitted
2. Omission when confidence too low → focused unit test
3. Omission when route cost too high → focused unit test
4. Omission when target place unknown → focused unit test (already covered by `pursuit_target_belief` returning `None`)
5. Omission when target/place blocked → focused unit test with `BlockedIntentMemory`
6. Search produces `Travel + Attack` for remote candidate → focused search test
7. Co-located candidates unaffected → existing test suite

## What to Change

### 1. Extend `emit_raid_target_goals()` in `candidate_generation.rs`

After the existing local-target loop, add a remote-target path:
- Iterate `view.hostile_targets_of(actor)` (or relevant raid-target source)
- For each target not co-located, call `pursuit_target_belief(view, actor, target)`
- If `Some(belief)`, derive confidence via `belief_confidence(&belief.source, current_tick - belief.observed_tick, &view.belief_confidence_policy(actor))`
- Check confidence >= `pursuit_profile.min_location_confidence`
- Check route cost to `belief.believed_place` <= `pursuit_profile.max_pursuit_travel_ticks`
- Check no active blocker in `BlockedIntentMemory` for this target/place
- If all pass, emit `RaidTarget { target: belief.target }`

### 2. Extend `emit_engage_hostile_goals()` in `candidate_generation.rs`

Same pattern as above but for hostile-target sources.

### 3. Access `PursuitProfile` from belief view

The candidate generation context must be able to read the actor's `PursuitProfile`. This may require exposing it through the `GoalBeliefView` or `GenerationContext` — follow the existing pattern for how `CombatProfile`, `UtilityProfile`, etc. are accessed.

### 4. Focused search test for `Travel + Attack` plan shape

Add a test that creates a remote target scenario and verifies `search_plan()` returns a plan with `Travel` then `Attack` steps.

## Files to Touch

- `crates/worldwake-ai/src/candidate_generation.rs` (modify) — extend emission functions
- `crates/worldwake-sim/src/belief_view.rs` (modify) — expose `PursuitProfile` access if needed
- `crates/worldwake-sim/src/per_agent_belief_view.rs` (modify) — implement profile access if needed
- `crates/worldwake-ai/src/planning_snapshot.rs` (modify) — route cost computation if not already available
- `crates/worldwake-ai/src/search/tests.rs` (modify) — add focused search test for Travel + Attack

## Out of Scope

- Invalidation of running pursuit plans (S40REMPUR-005)
- Decision trace extensions (S40REMPUR-006)
- Golden E2E tests (S40REMPUR-007)
- Guard/justice pursuit candidate generation (future ticket — same infrastructure, different goal kinds)
- New ranking or interrupt behavior — existing hierarchies apply
- Changes to `goal_relevant_places()` or `synthesized_root_candidate_targets()` (spec says no change needed)

## Acceptance Criteria

### Tests That Must Pass

1. Remote `RaidTarget` candidate emitted when target place is known, derived confidence >= `min_location_confidence`, and route cost <= `max_pursuit_travel_ticks`.
2. Remote `RaidTarget` NOT emitted when target place is unknown.
3. Remote pursuit NOT emitted when derived confidence < `min_location_confidence`.
4. Remote pursuit NOT emitted when route cost > `max_pursuit_travel_ticks`.
5. Remote pursuit NOT emitted when target/place is blocked in `BlockedIntentMemory`.
6. `RaidTarget` search returns `Travel + Attack` plan for remote believed target.
7. `EngageHostile` search returns `Travel + Attack` under same conditions.
8. `Attack` root synthesis remains `NoSynthesisPath` for remote hostile goals.
9. Co-located candidates still work as before.
10. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. No new `GoalKind` variant — remote pursuit uses existing `RaidTarget`/`EngageHostile`.
2. `Attack` remains lawful same-place terminal only.
3. Confidence is derived, never stored in the candidate or goal.
4. Pursuit is profile-driven, not hardcoded.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/candidate_generation.rs` (test module) — `test_remote_raid_target_emitted`, `test_remote_raid_target_omitted_low_confidence`, `test_remote_raid_target_omitted_over_range`, `test_remote_raid_target_omitted_blocked`
2. `crates/worldwake-ai/src/search/tests.rs` — `test_remote_pursuit_travel_then_attack`

### Commands

1. `cargo test -p worldwake-ai remote`
2. `cargo test -p worldwake-ai candidate`
3. `cargo clippy --workspace && cargo test --workspace`

## Outcome

**Completion date**: 2026-03-31

**What changed**:
- Added `pursuit_profile()` accessor to `GoalBeliefView`, `RuntimeBeliefView`, `impl_goal_belief_view!` macro, and `PerAgentBeliefView`
- Added `min_travel_ticks_via_view()` Dijkstra helper for route cost at candidate generation time
- Extended `emit_raid_target_goals()` with `emit_remote_raid_targets()` — iterates `known_entity_beliefs` for remote non-faction agents satisfying pursuit-profile constraints
- Extended `emit_engage_hostile_goals()` with `emit_remote_engage_hostile_targets()` — iterates `hostile_targets_of` for remote hostiles satisfying pursuit-profile constraints
- 7 focused candidate generation unit tests (emitted, omitted-low-confidence, omitted-over-range, omitted-blocked, omitted-unknown-place, engage-emitted, engage-omitted)
- 2 search tests verifying `Travel + Attack` plan shape for remote `RaidTarget` and `EngageHostile`

**Deviations**: None. All deliverables implemented as specified.

**Verification**: `cargo clippy --workspace` clean, `cargo test --workspace` 2,387 passed / 0 failed.
