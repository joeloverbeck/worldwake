# S91PLAPATGOL-002: Degenerate 0-step plan loop blocks actionable goals

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: None — test-only
**Deps**: None

## Problem

The `FreeCarryCapacity` goal returns `GoalSatisfied[steps=0]` every tick once Forager Lina's `cli-evaluation.ron` Eldergrove Forest run accumulates enough carried Waste. The 0-step plan produces no executable action, yet its priority (score 280000 in the observer report) blocks all other goals (eat, drink, sleep, relieve). The observer report shows ~708 consecutive idle ticks after the loop takes over, but no existing golden test reproduces that failure from the same scenario substrate.

## Assumption Reassessment (2026-04-11)

1. **Harness infrastructure exists**: `GoldenHarness` at `crates/worldwake-ai/tests/golden_harness/mod.rs:1089`, `step_once()` at line 1196, `seed_actor_local_beliefs()` at line 193, and `planning_trace_at()` in `crates/worldwake-ai/tests/golden_planner_pathology.rs`. All confirmed present.
2. **Goal and trace types exist**: `GoalKind::FreeCarryCapacity` at `crates/worldwake-core/src/goal.rs:30`. `SelectionTrace.selected_goal()` and `PlanSearchOutcome::Found { steps, .. }` are present in the AI decision trace surface. All confirmed present.
3. **Shared file boundary**: `crates/worldwake-ai/tests/golden_planner_pathology.rs` already owns the S91 planner-pathology goldens. This ticket adds one more independent test function in that file.
4. **Live scenario substrate differs from the original ticket sketch**: `scenarios/cli-evaluation.ron` does not start Lina in a one-place world with 18 carried Waste. The live motivating substrate is Forager Lina at `Eldergrove Forest` in the `cli-evaluation.ron` topology with exact profile values, 8 ground Apples, 5 ground Water, `carry_capacity: 20`, `disposal_profile.capacity_strain_threshold: 700`, and the `Eldergrove Orchard` Apple source. The observer report ties the loop onset to Waste accumulation over time, not to an initially pre-filled inventory. The ticket must therefore reproduce the failure from that scenario substrate instead of a shorter proxy setup.
5. **Scenario-locality check**: The observer report states Lina remains isolated at `Eldergrove Forest` for the full run and only knows local contents there. That makes an Eldergrove-focused `cli-evaluation.ron` slice lawful, but the test must preserve the exact scenario values and long-horizon accumulation behavior rather than approximating it with a pre-filled inventory shortcut.

## Architecture Check

1. Pure test addition — no production code changes. Uses existing golden harness patterns. Decision-trace assertions on `selected_goal()` and `PlanSearchOutcome::Found { steps }` remain the strongest proof surface for goal-selection pathologies per `docs/golden-e2e-testing.md`.
2. No backwards-compatibility shims introduced.

## Verification Layers

1. FreeCarryCapacity selected dominantly after Waste accumulation -> decision trace (`selection.selected_goal()` frequency count over the late-run window)
2. 0-step plan produced -> decision trace (`PlanAttemptTrace.outcome == Found { steps: [] }`)
3. Post-onset inactivity / no eating -> action trace and authoritative world state (`eat` never commits in the loop window, hunger continues rising)
4. Single-layer ticket (golden E2E with trace + authoritative state inspection). Additional layer mapping not applicable — no production code changes.

## What to Change

### 1. Add test to `golden_planner_pathology.rs`

Add the test function to the existing shared file, reusing the module import and `planning_trace_at()` helper already landed by S91PLAPATGOL-001.

### 2. Implement `degenerate_zero_step_loop_blocks_actionable_goals`

Build the test from the `scenarios/cli-evaluation.ron` Forager Lina substrate, not a simplified proxy:
- use the live `cli-evaluation.ron` place graph names/IDs already modeled in `golden_planner_pathology.rs`
- place Lina at `Eldergrove Forest`
- use the exact Lina startup values from `scenarios/cli-evaluation.ron` that matter to this pathology:
  - `HomeostaticNeeds { hunger: 200, thirst: 600, fatigue: 100, bladder: 100, dirtiness: 100 }`
  - the scenario's `UtilityProfile`, `MetabolismProfile`, `DriveThresholds`, `ExplorationProfile`, `DisposalProfile`, `PreferenceProfile`, `carry_capacity: 20`, and `KnownRecipes: ["Harvest Apples"]`
  - 8 ground Apples and 5 ground Water at `Eldergrove Forest`
  - facilities `ChoppingBlock` and named orchard `Eldergrove Orchard`
  - Apple `ResourceSource` at the orchard with `regeneration_ticks_per_unit: 2`, `capacity: 20`
- seed only Lina's local Eldergrove beliefs at tick 0, matching the observer report's “knows Eldergrove Forest contents” boundary

Run a long-enough horizon to allow the real Waste accumulation path to trigger the failure (expected onset from the observer report is around tick 730, not tick 0). Assert Phase 1 (bug reproduction):
1. In a late-run observation window after the accumulation phase, `FreeCarryCapacity` is the selected goal for the overwhelming majority of ticks.
2. In that same late-run window, the selected `FreeCarryCapacity` attempt repeatedly returns `PlanSearchOutcome::Found { steps, .. }` with `steps.is_empty()`.
3. No `eat` commit occurs once the loop window begins, and Lina's hunger is higher at the end of the observation window than at its start.
4. The test comments should note that this is reproducing the observer-reported `cli-evaluation.ron` failure mode rather than a synthetic pre-filled-inventory shortcut.

Include Phase 2 assertions as commented-out code blocks.

## Files to Touch

- `crates/worldwake-ai/tests/golden_planner_pathology.rs` (new or modify)
- `docs/generated/golden-coverage-matrix.md` (inventory refresh fallout)
- `docs/generated/golden-e2e-inventory.md` (inventory refresh fallout)
- `docs/generated/golden-scenario-index.md` (inventory refresh fallout)
- `docs/generated/golden-scenario-details/planner-pathology.md` (inventory refresh fallout)

## Out of Scope

- Fixing the FreeCarryCapacity 0-step loop bug (separate spec/ticket)
- Phase 2 assertion activation (deferred until fix lands)
- Tests for budget exhaustion or missing survival goals (S91PLAPATGOL-001, -003)
- Any production code changes

## Acceptance Criteria

### Tests That Must Pass

1. `degenerate_zero_step_loop_blocks_actionable_goals` passes — confirms the `cli-evaluation.ron` Lina loop exists (Phase 1 assertions)
2. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. No production code modified — test-only change
2. Deterministic reproduction: same `cli-evaluation.ron` seed (`7777`) produces the same loop outcome every run

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_planner_pathology.rs::degenerate_zero_step_loop_blocks_actionable_goals` — reproduces the `cli-evaluation.ron` Forager Lina FreeCarryCapacity 0-step degenerate loop blocking actionable self-care

### Commands

1. `cargo test -p worldwake-ai degenerate_zero_step_loop_blocks_actionable_goals`
2. `cargo test -p worldwake-ai`
3. `python3 scripts/golden_inventory.py --write --check-docs`
4. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

Completed on 2026-04-11.

- Added `degenerate_zero_step_loop_blocks_actionable_goals` to `crates/worldwake-ai/tests/golden_planner_pathology.rs`.
- Reproduced the observer-reported Forager Lina loop from the `scenarios/cli-evaluation.ron` Eldergrove substrate instead of the ticket's original pre-filled-inventory proxy.
- Added a dedicated Lina setup helper that uses the live scenario seed (`7777`), place graph, profile values, local beliefs, and orchard/resource inputs needed for the real late-run Waste-accumulation failure.
- Refreshed golden inventory/docs after adding Scenario 143 metadata.

## Deviations

- Reassessed and corrected the original ticket scope before implementation: the landed golden does not use the earlier 60-tick one-place shortcut with pre-seeded Waste because that setup did not match the motivating `cli-evaluation.ron` failure described by the observer report.

## Verification Result

- Passed `cargo test -p worldwake-ai degenerate_zero_step_loop_blocks_actionable_goals -- --nocapture`
- Passed `cargo test -p worldwake-ai`
- Passed `python3 scripts/golden_inventory.py --write --check-docs`
- Passed `cargo clippy --workspace --all-targets -- -D warnings`
