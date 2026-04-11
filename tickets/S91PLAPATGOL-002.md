# S91PLAPATGOL-002: Degenerate 0-step plan loop blocks actionable goals

**Status**: PENDING
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: None — test-only
**Deps**: None

## Problem

The `FreeCarryCapacity` goal returns `GoalSatisfied[steps=0]` every tick when an agent's inventory is full of Waste. The 0-step plan produces no executable action, yet its priority (score 280000) blocks all other goals (eat, drink, sleep). The agent idles for 700+ ticks while starving with food locally available. No existing golden test reproduces this pathology in isolation.

## Assumption Reassessment (2026-04-11)

1. **Harness infrastructure exists**: `GoldenHarness` at `crates/worldwake-ai/tests/golden_harness/mod.rs:1089`, `step_once()` at line 1196, `seed_actor_beliefs()` at line 158. All confirmed present.
2. **Goal and trace types exist**: `GoalKind::FreeCarryCapacity` at `crates/worldwake-core/src/goal.rs:30`. `SelectionTrace.selected_goal()` at `crates/worldwake-ai/src/decision_trace.rs:937`. `PlanSearchOutcome::Found { steps, terminal_kind }` at line 892. All confirmed present.
3. **Shared file boundary**: `crates/worldwake-ai/tests/golden_planner_pathology.rs` now exists from S91PLAPATGOL-001. This ticket adds an independent test function in that shared file; no compile-time conflict with -003.
4. **Component surface**: `DisposalProfile` at `crates/worldwake-core/src/disposal.rs`. `CarryCapacity` in `crates/worldwake-core/src/lib.rs`. `KnownRecipes` in `crates/worldwake-core/src/lib.rs`. All confirmed present.
5. **Scenario isolation**: Single-place scenario with one agent whose inventory is pre-filled with Waste. Only competing goals are eat (local apples available) and homeostatic needs. The test isolates FreeCarryCapacity's dominance by ensuring the only meaningful alternative is eating — if FreeCarryCapacity blocks eating, the pathology is confirmed.

## Architecture Check

1. Pure test addition — no production code changes. Uses existing golden harness patterns. Decision-trace assertions on `selected_goal()` and `PlanSearchOutcome::Found { steps }` are the strongest proof surface for goal selection pathologies (precision rule §6).
2. No backwards-compatibility shims introduced.

## Verification Layers

1. FreeCarryCapacity selected dominantly -> decision trace (`selection.selected_goal()` frequency count)
2. 0-step plan produced -> decision trace (`PlanAttemptTrace.outcome == Found { steps: [] }`)
3. Agent never eats -> authoritative world state (`HomeostaticNeeds.hunger` monotonically increasing)
4. Single-layer ticket (golden E2E with trace inspection). Additional layer mapping not applicable — no production code changes.

## What to Change

### 1. Add test to `golden_planner_pathology.rs`

Add the test function to the existing shared file, reusing the module import and `planning_trace_at()` helper already landed by S91PLAPATGOL-001.

### 2. Implement `degenerate_zero_step_loop_blocks_actionable_goals`

Build minimal scenario:
- 1 place: Forest (tags: `[Forest]`)
- 1 facility: OrchardRow at Forest (`WorkstationTag::OrchardRow`)
- 1 resource source: Apple at Forest (facility: OrchardRow, `regeneration_ticks_per_unit: 2`, `capacity: 20`)
- 5 Apple items at Forest
- 1 AI agent at Forest with:
  - `HomeostaticNeeds { hunger: 700, thirst: 100, fatigue: 100, bladder: 100, dirtiness: 100 }`
  - `CarryCapacity(20)`, 18 Waste items in inventory
  - `DisposalProfile { capacity_strain_threshold: Permille(700) }`
  - `UtilityProfile` with `hunger_weight: 600`
  - `MetabolismProfile` with `hunger_rate: 3`
  - `DriveThresholds` (hunger: low 200, medium 400, high 600, critical 800)
  - `PerceptionProfile` (standard), `CognitiveProfile` (defaults)
  - `KnownRecipes`: ["Harvest Apples"]
  - Beliefs seeded: agent knows Forest contents (apples, OrchardRow)

Run 60 ticks. Assert Phase 1 (bug reproduction):
1. Over first 50 ticks, FreeCarryCapacity is the selected goal for >= 40 ticks
2. For >= 30 of those ticks, `PlanSearchOutcome::Found { steps, .. }` where `steps.is_empty()` (0-step plan)
3. Agent's hunger at tick 50 is higher than at tick 0 (never ate)

Include Phase 2 assertions as commented-out code blocks.

## Files to Touch

- `crates/worldwake-ai/tests/golden_planner_pathology.rs` (new or modify)

## Out of Scope

- Fixing the FreeCarryCapacity 0-step loop bug (separate spec/ticket)
- Phase 2 assertion activation (deferred until fix lands)
- Tests for budget exhaustion or missing survival goals (S91PLAPATGOL-001, -003)
- Any production code changes

## Acceptance Criteria

### Tests That Must Pass

1. `degenerate_zero_step_loop_blocks_actionable_goals` passes — confirms the pathology exists (Phase 1 assertions)
2. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. No production code modified — test-only change
2. Deterministic reproduction: same seed produces same trace outcome every run

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_planner_pathology.rs::degenerate_zero_step_loop_blocks_actionable_goals` — reproduces FreeCarryCapacity 0-step degenerate plan loop blocking eat goals

### Commands

1. `cargo test -p worldwake-ai degenerate_zero_step_loop_blocks_actionable_goals`
2. `cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`
