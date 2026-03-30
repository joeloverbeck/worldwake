# E19GUAPAT-004: Implement patrol candidate generation in worldwake-ai

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — new candidate emission function in AI crate
**Deps**: E19GUAPAT-001 (PatrolRoute, PatrolProfile), E19GUAPAT-002 (GoalKind::Patrol, PlannerOpKind::Patrol), E19GUAPAT-003 (patrol action registered)

## Problem

Guards with `PatrolRoute` and `PatrolProfile` components need to generate `GoalKind::Patrol` candidates during the AI decision cycle. Patrol urgency must be belief-driven (Principle 14): modulated by unresolved violations in the guard's `ViolationMemory`, believed office vacancy (E16c institutional beliefs), and believed contested offices (E16b force-control beliefs).

## Assumption Reassessment (2026-03-30)

1. Candidate generation in [`crates/worldwake-ai/src/candidate_generation.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/candidate_generation.rs) still follows the `emit_*_candidates()` orchestration pattern from `generate_candidates_with_travel_horizon()`, and there is currently no patrol emitter. Adding `emit_patrol_candidates()` at that boundary is consistent with the live architecture.
2. The live shared abstraction boundary under audit is the AI belief-view surface:
   `worldwake_sim::GoalBeliefView` / `RuntimeBeliefView` plus the snapshot-backed implementations in [`crates/worldwake-sim/src/belief_view.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/belief_view.rs), [`crates/worldwake-sim/src/per_agent_belief_view.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/per_agent_belief_view.rs), [`crates/worldwake-ai/src/planning_snapshot.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/planning_snapshot.rs), and [`crates/worldwake-ai/src/planning_state.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/planning_state.rs).
3. `GoalKind::Patrol { place: EntityId }`, `PlannerOpKind::Patrol`, patrol dispatch, and patrol action registration already exist in the live code. The intended planning surface remains `[Travel(place), Patrol]`; this ticket must not add an alternate goal-to-op shortcut.
4. The live placeholder patrol motive is in [`crates/worldwake-ai/src/ranking.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/ranking.rs), where `GoalKind::Patrol` currently scores `1`. `GroundedGoal` in [`crates/worldwake-ai/src/goal_model.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/goal_model.rs) carries no motive field, so candidate generation cannot “own” patrol scoring by itself. The real motive arithmetic must therefore replace the ranking placeholder, not sit beside it.
5. `believed_office_holder()` and `believed_force_controller()` already exist on the live belief-view surface. The clean contested-office read is `believed_force_controller()`, not ad hoc scanning for raw `InstitutionalClaim::ForceControl` values inside candidate generation.
6. `ViolationMemory::unresolved_records()` exists in [`crates/worldwake-core/src/violation.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/violation.rs), but unresolved violation records are not currently available on `GoalBeliefView`. `RuntimeBeliefView` already exposes `active_violation_records()`. If patrol ranking must depend on unresolved violations, this ticket must extend the shared AI read surface cleanly rather than hardcoding a second patrol motive path elsewhere.
7. `RuntimeBeliefView` already exposes `patrol_profile()`, but `GoalBeliefView` does not, and neither live trait exposes `patrol_route()`. Because patrol candidate emission needs the next waypoint and patrol ranking needs `patrol_motive_weight`, the current ticket scope is incomplete unless it explicitly adds the missing patrol-state reads to the AI belief boundary.
8. There are currently no patrol candidate-generation tests in [`crates/worldwake-ai/src/candidate_generation.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/candidate_generation.rs), no patrol ranking tests in [`crates/worldwake-ai/src/ranking.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/ranking.rs), and no patrol conformance coverage in [`crates/worldwake-ai/tests/planner_conformance.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/planner_conformance.rs). The original ticket understated the test gap.
9. Guard-locality still stands as a live invariant: patrol urgency must not depend on crimes or office instability absent from the guard’s belief/memory surface. This ticket should prove that at the belief-view / ranking layer, not by appealing to authoritative world facts.
10. Corrected scope: this ticket owns patrol candidate emission, the missing AI belief-view patrol reads required to support that emission cleanly, and replacement of the patrol ranking placeholder with patrol-profile-plus-belief arithmetic. Route adaptation, public-order feedback, and broader guard-presence architecture remain out of scope.

## Architecture Check

1. Following the existing `emit_justice_candidates()` / `emit_need_candidates()` pattern is still the cleanest candidate-emission architecture, but only if patrol route lookup is exposed through the same AI belief boundary instead of bypassing it with one-off world reads.
2. The robust design is:
   extend the shared belief-view surface with self-authoritative patrol route access and expose the already-existing self-authoritative patrol profile / active violation record data to `GoalBeliefView`, then keep candidate emission responsible only for emitting `GoalKind::Patrol { place }` while ranking remains the single owner of patrol motive arithmetic.
3. This is cleaner than encoding patrol motive inside `GroundedGoal`, threading sidecar patrol motive data through candidate generation, or duplicating the patrol score in both candidate generation and ranking. One motive path, one read boundary, no aliasing.
4. Vacancy and contest escalation should read the normalized institutional belief accessors (`believed_office_holder`, `believed_force_controller`) rather than scanning raw claim bags. That keeps the patrol feature aligned with the existing institutional-belief architecture.
5. No backwards-compatibility shim path: if patrol needs new self-authoritative reads on the AI surface, add the canonical methods and use them everywhere instead of inventing a temporary patrol-only helper.

## Verification Layers

1. Patrol candidate presence / absence by self-authoritative patrol state -> focused candidate-generation tests at the `GoalBeliefView` boundary
2. Patrol motive replacement of placeholder score -> focused ranking tests at the `GoalBeliefView` boundary
3. Belief-only urgency scaling from unresolved violations / vacancy / contest -> focused ranking tests against belief and memory inputs, not authoritative world state
4. Planner identity path `[Travel, Patrol]` remains lawful -> planner/goal-model conformance coverage
5. No hidden world-state dependency for patrol emission/ranking -> trait-boundary compilation plus focused belief-view tests proving patrol data is sourced from the AI read surface

## What to Change

### 1. New function `emit_patrol_candidates()` in `crates/worldwake-ai/src/candidate_generation.rs`

```rust
fn emit_patrol_candidates(
    candidates: &mut Vec<GoalCandidate>,
    diagnostics: &mut CandidateDiagnostics,
    ctx: &CandidateContext,
) {
    // 1. Check agent has PatrolRoute + PatrolProfile
    // 2. Get next waypoint from PatrolRoute.current_index
    // 3. Emit GoalKind::Patrol { place: next_waypoint }
}
```

Patrol candidate emission should not invent a parallel patrol-motive storage path. It only grounds the patrol opportunity from self-authoritative patrol state.

### 2. Extend the shared AI belief-view surface for patrol state

- Add canonical self-authoritative patrol route access to the relevant belief-view traits / implementations.
- Expose the already-existing self-authoritative patrol profile and active violation record reads on `GoalBeliefView` so ranking can compute patrol motive without bypassing the AI read boundary.
- Update snapshot-backed runtime/test belief views accordingly.

### 3. Call from main candidate generation

Add `emit_patrol_candidates(candidates, diagnostics, ctx);` to `generate_candidates_with_travel_horizon()` alongside existing emitters.

### 4. Replace placeholder patrol ranking with real patrol motive arithmetic

Update the live ranking surface so emitted patrol candidates score from `PatrolProfile.patrol_motive_weight` plus belief-driven urgency modifiers sourced from:

- unresolved active violation records / violation memory
- believed office vacancy
- believed contested force control

Do not leave patrol on the placeholder score from E19GUAPAT-002 once patrol candidates are active.

## Files to Touch

- `crates/worldwake-sim/src/belief_view.rs` (modify — add canonical patrol-state reads to the AI belief boundary)
- `crates/worldwake-sim/src/per_agent_belief_view.rs` (modify — implement patrol-state reads from authoritative self state)
- `crates/worldwake-ai/src/candidate_generation.rs` (modify — add `emit_patrol_candidates()` and call it)
- `crates/worldwake-ai/src/ranking.rs` (modify — replace placeholder patrol motive with real patrol motive arithmetic)

## Out of Scope

- Patrol action handler (E19GUAPAT-003 — already delivered)
- Route adaptation (E19GUAPAT-006)
- Guard presence factor / public_order extension (E19GUAPAT-005)
- Golden E2E tests (E19GUAPAT-007)
- Thief belief system / crime avoidance of guarded places (separate epic concern)
- Captain-mediated route reassignment (deferred per spec)

## Acceptance Criteria

### Tests That Must Pass

1. Agent with PatrolRoute + PatrolProfile generates `GoalKind::Patrol` candidate
2. Agent without PatrolRoute generates zero Patrol candidates
3. Agent without PatrolProfile generates zero Patrol candidates
4. Guard with unresolved active violation records has higher patrol motive than guard with none
5. Guard who believes an office on the patrol jurisdiction/path is vacant has higher patrol motive than guard with stable institutions
6. Guard who believes force control is contested on the patrol jurisdiction/path has higher patrol motive than guard with uncontested control
7. Guard at remote location with no crime or institutional belief input retains only base patrol motive (Principle 7 / 14)
8. Existing patrol planner identity coverage remains valid; this ticket must not change the existing `[Travel, Patrol]` path
9. Existing suite: `cargo test -p worldwake-ai`
10. Existing suite: `cargo test -p worldwake-systems patrol_actions`
11. `cargo clippy --workspace`

### Invariants

1. Patrol candidate generation reads patrol route/profile only through the canonical AI belief boundary and does not bypass it with direct world queries
2. No `HashMap` or `f32`/`f64` introduced
3. Patrol motive replaces the current placeholder patrol score with one canonical patrol-profile-driven arithmetic path
4. Candidate motive uses deterministic integer / `Permille` arithmetic
5. Candidate generation does not mutate any state

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/candidate_generation.rs` — focused patrol candidate tests for route/profile presence, next-waypoint derivation, and candidate absence when patrol state is incomplete
2. `crates/worldwake-ai/src/ranking.rs` — focused patrol motive tests for base weight, unresolved-violation escalation, vacancy escalation, contest escalation, and locality-preserving baseline behavior
3. `crates/worldwake-systems/src/patrol_actions.rs` — keep existing authoritative patrol lifecycle tests green as regression coverage for the leaf patrol operator

### Commands

1. `cargo test -p worldwake-ai patrol_candidates_`
2. `cargo test -p worldwake-ai patrol_goal_`
3. `cargo test -p worldwake-systems patrol_actions::tests::`
4. `cargo test -p worldwake-ai`
5. `cargo clippy --workspace`

## Outcome

Completed: 2026-03-30

What actually changed:
- Added canonical patrol-state reads to the AI belief surface in `worldwake_sim::GoalBeliefView` / `RuntimeBeliefView`, with authoritative implementation in `PerAgentBeliefView`.
- Added patrol candidate emission in `worldwake-ai` from self-authoritative `PatrolRoute` + `PatrolProfile`, using the next waypoint as the grounded patrol place.
- Replaced the patrol ranking placeholder score with one canonical patrol motive path in `ranking.rs`: base `patrol_motive_weight`, scaled by unresolved suspected-theft memory plus believed vacancy / believed contested force control for offices on the patrol route.
- Added focused patrol candidate-generation and patrol-ranking tests and kept the existing authoritative patrol action suite green.

Deviations from original plan:
- No `planning_snapshot` / `planning_state` changes were needed. The live runtime candidate/ranking path only required extending the shared belief trait plus `PerAgentBeliefView`.
- No new planner-conformance test was added. Existing patrol identity / dispatch coverage already held, and this ticket did not need to widen scope into planner transition coverage.

Verification results:
- Passed `cargo test -p worldwake-ai patrol_candidates_`
- Passed `cargo test -p worldwake-ai patrol_goal_`
- Passed `cargo test -p worldwake-systems patrol_actions::tests::`
- Passed `cargo test -p worldwake-ai`
- Passed `cargo clippy --workspace`
