# E19GUAPAT-004: Implement patrol candidate generation in worldwake-ai

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — new candidate emission function in AI crate
**Deps**: E19GUAPAT-001 (PatrolRoute, PatrolProfile), E19GUAPAT-002 (GoalKind::Patrol, PlannerOpKind::Patrol), E19GUAPAT-003 (patrol action registered)

## Problem

Guards with `PatrolRoute` and `PatrolProfile` components need to generate `GoalKind::Patrol` candidates during the AI decision cycle. Patrol urgency must be belief-driven (Principle 14): modulated by unresolved violations in the guard's `ViolationMemory`, believed office vacancy (E16c institutional beliefs), and believed contested offices (E16b force-control beliefs).

## Assumption Reassessment (2026-03-30)

1. Candidate generation in `crates/worldwake-ai/src/candidate_generation.rs` uses a pattern of `emit_X_candidates()` functions called from `generate_candidates_with_travel_horizon()` (lines 235–244). New function `emit_patrol_candidates()` follows this pattern.
2. `ViolationMemory` is accessible via `ctx.view` belief view interface. Method `unresolved_records()` returns unresolved violations (confirmed in `crates/worldwake-core/src/violation.rs` line 60+).
3. `believed_office_holder()` is the E16c interface for querying institutional beliefs about office occupancy. Must verify this symbol exists in the belief view.
4. `InstitutionalClaim::ForceControl` (E16b) is the mechanism for contested-office awareness. Must verify this exists in the belief/institutional modules.
5. The live `GoalKind` under test is `GoalKind::Patrol { place: EntityId }` — new variant from E19GUAPAT-002.
6. Candidate emission should produce a `RankedGoal` with motive derived from `PatrolProfile.patrol_motive_weight` multiplied by urgency modifiers.
7. The spec says patrol uses `UtilityProfile` weighting system — `patrol_motive_weight` on `PatrolProfile` competes with other motive weights.
8. Guard at remote location must NOT increase urgency for crimes they haven't heard about (Principle 7/14 — locality).
9. No adjacent contradictions found. If `believed_office_holder()` or `InstitutionalClaim::ForceControl` do not yet exist, this ticket must note them as blockers from E16b/E16c.

## Architecture Check

1. Following the existing `emit_justice_candidates()` / `emit_need_candidates()` pattern ensures consistency. A new `emit_patrol_candidates()` function called from the main candidate generation entry point is the cleanest approach.
2. Belief-driven urgency avoids querying world state (Principle 14). All modifiers come from the guard's `ViolationMemory` and institutional beliefs — information that reached the guard through lawful channels.
3. No backwards-compatibility shims.

## Verification Layers

1. Candidate emission for agents with PatrolRoute → decision trace: `candidates.generated` includes `GoalKind::Patrol`
2. Candidate absence for agents without PatrolRoute → decision trace: no Patrol candidate
3. Urgency scaling with crime reports → focused unit test: compare motive values with 0 vs N unresolved violations
4. Belief-only guarantee → focused unit test: guard at remote location with no crime reports has base urgency despite world-state crimes existing
5. Cross-layer: candidate generation (AI) reads belief state (core) — verified via decision trace

## What to Change

### 1. New function `emit_patrol_candidates()` in `crates/worldwake-ai/src/candidate_generation.rs`

```rust
fn emit_patrol_candidates(
    candidates: &mut Vec<GoalCandidate>,
    diagnostics: &mut CandidateDiagnostics,
    ctx: &CandidateContext,
) {
    // 1. Check agent has PatrolRoute + PatrolProfile
    // 2. Get next waypoint from PatrolRoute
    // 3. Compute base motive from patrol_motive_weight
    // 4. Apply belief-driven urgency modifiers:
    //    - Count unresolved violations in ViolationMemory
    //    - Check believed office vacancy
    //    - Check believed contested office
    // 5. Emit GoalKind::Patrol { place: next_waypoint }
}
```

### 2. Call from main candidate generation

Add `emit_patrol_candidates(candidates, diagnostics, ctx);` to `generate_candidates_with_travel_horizon()` alongside existing emitters.

### 3. Planner support for Patrol goal → plan sequence

Ensure the goal model maps `GoalKind::Patrol { place }` to the plan sequence `[Travel(place), Patrol]`. This may require additions to `goal_model.rs` if not fully handled by E19GUAPAT-002.

## Files to Touch

- `crates/worldwake-ai/src/candidate_generation.rs` (modify — add `emit_patrol_candidates()` and call it)
- `crates/worldwake-ai/src/goal_model.rs` (modify — if plan sequence mapping needs refinement beyond E19GUAPAT-002)

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
4. Guard with 3 unresolved violations in ViolationMemory has higher patrol motive than guard with 0
5. Guard who believes office is vacant has higher patrol motive than guard with stable institution
6. Guard at remote location with no crime reports has base-level patrol motive (Principle 7 — no world-state leakage)
7. Next waypoint is correctly derived from `PatrolRoute.current_index` and `assigned_places`
8. Existing suite: `cargo test -p worldwake-ai`
9. `cargo clippy --workspace`

### Invariants

1. Patrol candidate generation reads ONLY from belief state (ViolationMemory, institutional beliefs), never from `World` directly (Principle 14)
2. No `HashMap` or `f32`/`f64` introduced
3. Candidate motive uses `Permille` arithmetic
4. Candidate generation does not mutate any state

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/candidate_generation.rs` or `crates/worldwake-ai/tests/` — focused tests for `emit_patrol_candidates()` with varying belief states
2. Decision trace tests verifying candidate presence/absence

### Commands

1. `cargo test -p worldwake-ai -- patrol`
2. `cargo test -p worldwake-ai -- candidate`
3. `cargo clippy --workspace && cargo test --workspace`
