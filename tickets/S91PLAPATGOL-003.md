# S91PLAPATGOL-003: Role agent missing survival goal generation

**Status**: PENDING
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: None — test-only
**Deps**: None

## Problem

Guard agents with `PatrolProfile` and `PatrolRoute` never generate hunger or thirst goals despite needs reaching critical levels (900+). Role goals (Patrol, InvestigateViolation) dominate goal selection, leading to death from `NeedDeprivation{Hunger}` without a single eat or drink attempt. No existing golden test reproduces this pathology in isolation.

## Assumption Reassessment (2026-04-11)

1. **Harness infrastructure exists**: `GoldenHarness` at `crates/worldwake-ai/tests/golden_harness/mod.rs:1089`, `step_once()` at line 1196, `seed_actor_beliefs()` at line 158. All confirmed present.
2. **Goal and profile types exist**: `GoalKind::AcquireCommodity` at `crates/worldwake-core/src/goal.rs:22`, `GoalKind::ConsumeOwnedCommodity` at line 19. `PatrolProfile` at `crates/worldwake-core/src/patrol.rs`. `PatrolRoute` at `crates/worldwake-core/src/patrol.rs`. `CombatProfile` at `crates/worldwake-core/src/combat.rs`. `DeadAt` / `DeathCause::NeedDeprivation` at `crates/worldwake-core/src/combat.rs:60`. All confirmed present.
3. **Shared file boundary**: `crates/worldwake-ai/tests/golden_planner_pathology.rs` now exists from S91PLAPATGOL-001. This ticket adds an independent test function in that shared file.
4. **Candidate generation surface**: `emit_need_candidates()` in `crates/worldwake-ai/src/candidate_generation.rs` generates hunger/thirst goals. `emit_patrol_candidates()` generates patrol goals. The hypothesis is that guard profiles either suppress or fail to generate need-based candidates. This test will confirm whether survival goals appear in the goal selection trace at all.
5. **Scenario isolation**: 2-place scenario with a guard agent at Trail (no food/water). Food and water at Village (2 ticks away). Agent has patrol_profile, patrol_route, combat_profile. The only survival path is generating a hunger/thirst goal and planning acquisition from Village. No social targets, no combat targets, no theft affordances — isolates the question of whether survival goals are generated at all for role agents.

## Architecture Check

1. Pure test addition — no production code changes. Decision-trace assertions on goal selection are the strongest proof surface for goal generation gaps (precision rule §6). The test checks whether survival goals appear in the `selected_goal()` trace across 250 ticks — a comprehensive sweep.
2. No backwards-compatibility shims introduced.

## Verification Layers

1. No survival goals selected -> decision trace (`selection.selected_goal()` across 250 ticks, checking for AcquireCommodity/ConsumeOwnedCommodity for food/water)
2. Role goals dominate -> decision trace (Patrol/Sleep/Relieve account for >90% of selections)
3. Agent reaches critical needs or death -> authoritative world state (`HomeostaticNeeds.hunger > 900` or `DeadAt` component present)
4. Single-layer ticket (golden E2E with trace inspection). Additional layer mapping not applicable — no production code changes.

## What to Change

### 1. Add test to `golden_planner_pathology.rs`

Add the test function to the existing shared file, reusing the module import and `planning_trace_at()` helper already landed by S91PLAPATGOL-001.

### 2. Implement `role_agent_generates_survival_goals_under_critical_needs`

Build minimal scenario:
- 2 places: Trail (tags: `[Trail, Road]`), Village (tags: `[Village]`)
- 1 bidirectional edge: Trail ↔ Village, `travel_ticks: 2`
- 5 Bread items at Village, 5 Water items at Village
- 1 AI agent ("Guard") at Trail with:
  - `HomeostaticNeeds { hunger: 600, thirst: 600, fatigue: 100, bladder: 100, dirtiness: 100 }`
  - `CombatProfile` (standard: `attack_skill: 600`, `guard_skill: 550`, `wound_capacity: 900`, `incapacitation_threshold: 750`, etc.)
  - `PatrolProfile { base_dwell_ticks: 5, dwell_vigilance_scale_ticks: 3, vigilance: 700, route_adaptation_sensitivity: 400, patrol_motive_weight: 600 }`
  - `PatrolRoute { assigned_places: [Trail, Village] }`
  - `UtilityProfile` with `hunger_weight: 400, thirst_weight: 400, danger_weight: 800`
  - `MetabolismProfile` with `hunger_rate: 3, thirst_rate: 3, starvation_tolerance_ticks: 200, dehydration_tolerance_ticks: 150`
  - `DriveThresholds` (hunger/thirst: low 250, medium 500, high 750, critical 900)
  - `PerceptionProfile` (standard), `CognitiveProfile` (defaults)
  - Beliefs seeded: agent knows Village exists, knows Bread and Water at Village

Run 250 ticks. Assert Phase 1 (bug reproduction):
1. Across all 250 ticks, no selected goal matches `AcquireCommodity { commodity: Bread | Water, .. }` or `ConsumeOwnedCommodity { commodity: Bread | Water, .. }`
2. Patrol, Sleep, and Relieve account for >90% of selected goals
3. Agent either has `DeadAt` component (NeedDeprivation death) or hunger > 900 by tick 250

Include Phase 2 assertions as commented-out code blocks.

## Files to Touch

- `crates/worldwake-ai/tests/golden_planner_pathology.rs` (new or modify)

## Out of Scope

- Fixing the survival goal generation gap (separate spec/ticket)
- Phase 2 assertion activation (deferred until fix lands)
- Tests for budget exhaustion or 0-step loop (S91PLAPATGOL-001, -002)
- Any production code changes

## Acceptance Criteria

### Tests That Must Pass

1. `role_agent_generates_survival_goals_under_critical_needs` passes — confirms the pathology exists (Phase 1 assertions)
2. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. No production code modified — test-only change
2. Deterministic reproduction: same seed produces same trace outcome every run

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_planner_pathology.rs::role_agent_generates_survival_goals_under_critical_needs` — reproduces missing survival goal generation for guard agents under critical needs

### Commands

1. `cargo test -p worldwake-ai role_agent_generates_survival_goals_under_critical_needs`
2. `cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`
