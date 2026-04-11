# S92FRECARCAP-004: Flip Scenario 143 golden and refresh docs

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: None
**Deps**: `archive/tickets/S92FRECARCAP-001.md`, `archive/tickets/S92FRECARCAP-002.md`, `specs/S92-free-carry-capacity-zero-step-loop-fix.md`

## Problem

Scenario 143 (`degenerate_zero_step_loop_blocks_actionable_goals`) currently proves the bug: repeated zero-step `FreeCarryCapacity` plans, no late `eat` commit, rising hunger. After tickets 001-002 fix the contract, this golden must flip from failure proof to fix proof, demonstrating that Lina breaks the loop and recovers self-care behavior.

## Assumption Reassessment (2026-04-11)

1. `degenerate_zero_step_loop_blocks_actionable_goals` exists at `crates/worldwake-ai/tests/golden_planner_pathology.rs:618`. Current assertions (lines 680-699): `window_selected >= 100` (FreeCarryCapacity dominates), `window_zero_step >= 100` (repeated 0-step plans), `!late_eat_commit` (no eat recovery), `hunger_after > hunger_at_window_start` (hunger rises). These are failure-proof assertions that must become fix-proof assertions. Confirmed 2026-04-11.
2. Scenario substrate: `scenarios/cli-evaluation.ron`, seed `7777`, Eldergrove Forest, Forager Lina with `disposal_profile: (capacity_strain_threshold: 700)`. Confirmed 2026-04-11.
3. `scripts/golden_inventory.py` exists and accepts `--write --check-docs` flags. Confirmed 2026-04-11.
5. Live `GoalKind`: `FreeCarryCapacity`. After tickets 001-002, the planner should produce executable `PlannerOpKind::DropItem` steps when disposal is actionable, or switch to another goal. The zero-step loop should be impossible.

## Architecture Check

1. Flipping assertions from failure-proof to fix-proof is the correct approach — it reuses the exact scenario substrate that reproduced the bug, now proving the fix on the same conditions. No test infrastructure changes needed.
2. No backward-compatibility shims. The old failure assertions are replaced entirely.

## Verification Layers

1. No repeated zero-step FreeCarryCapacity plans -> golden E2E assertion on `PlanSearchOutcome::Found` step count
2. When FreeCarryCapacity selected, produces DropItem steps or planner switches goal -> golden E2E assertion on operator surface or goal switch
3. Self-care recovery signal (late eat commit or hunger decrease) -> golden E2E assertion on downstream behavior
6. Single-layer ticket: golden E2E is the appropriate proof surface for this end-to-end behavioral contract.

## What to Change

### 1. Replace failure assertions with fix assertions

In `degenerate_zero_step_loop_blocks_actionable_goals`, replace the current assertions with:

1. **No repeated zero-step loop**: Assert that during the late-run observation window, there are zero (or bounded very small) `FreeCarryCapacity` plans returning `PlanSearchOutcome::Found { steps: [] }`.
2. **Executable disposal or goal switch**: When `FreeCarryCapacity` is selected, assert it produces a plan with `PlannerOpKind::DropItem` step(s), or the planner switches to another actionable self-care goal.
3. **Self-care recovery**: Assert at least one of: late `eat` commit occurs, hunger decreases or stabilizes during the observation window, or bounded inactivity (the idle run is broken).

Keep the exact substrate unchanged:
- `scenarios/cli-evaluation.ron`
- seed `7777`
- Eldergrove / Forager Lina setup
- late-run observation window after real waste accumulation

### 2. Refresh generated golden docs

Run:

```bash
python3 scripts/golden_inventory.py --write --check-docs
```

Commit any changes under `docs/generated/golden-*` caused by the updated Scenario 143 assertions.

## Files to Touch

- `crates/worldwake-ai/tests/golden_planner_pathology.rs` (modify)
- `docs/generated/golden-e2e-inventory.md` (modify — regenerated)
- `docs/generated/golden-scenario-index.md` (modify — regenerated)
- `docs/generated/golden-scenario-details/golden_planner_pathology.md` (modify — regenerated)

## Out of Scope

- Modifying the shared helper, satisfaction, emission, or ranking logic — done in 001-002
- Changing the scenario substrate (seed, scenario file, agent setup)
- Changing unrelated S91 pathologies (`budget_exhaustion_blocks_cross_location_water_acquisition` or `role_agent_generates_survival_goals_under_critical_needs`)
- Rebalancing hunger, metabolism, or utility weights

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-ai degenerate_zero_step_loop_blocks_actionable_goals` — now proves fix, not failure
2. `cargo test -p worldwake-ai golden_waste_disposal_cycle` — existing S82 disposal cycle preserved
3. `cargo test -p worldwake-ai` — no regressions
4. `python3 scripts/golden_inventory.py --write --check-docs` — generated docs consistent
5. `cargo clippy --workspace --all-targets -- -D warnings` — clean clippy

### Invariants

1. Scenario 143 uses exact same substrate (cli-evaluation.ron, seed 7777, Forager Lina, late-run window) — no substrate changes
2. The golden proves lawful planner behavior (real disposal work or goal switch), not merely "stopped picking FreeCarryCapacity"
3. The pathological idle run is broken, evidenced by bounded inactivity and downstream self-care recovery

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_planner_pathology.rs::degenerate_zero_step_loop_blocks_actionable_goals` — flip from failure-proof to fix-proof assertions

### Commands

1. `cargo test -p worldwake-ai degenerate_zero_step_loop_blocks_actionable_goals -- --nocapture`
2. `cargo test -p worldwake-ai golden_waste_disposal_cycle -- --nocapture`
3. `cargo test -p worldwake-ai`
4. `python3 scripts/golden_inventory.py --write --check-docs`
5. `cargo clippy --workspace --all-targets -- -D warnings`
