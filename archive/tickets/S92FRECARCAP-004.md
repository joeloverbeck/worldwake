# S92FRECARCAP-004: Flip Scenario 143 golden and refresh docs

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — directly possessed lot quantity parity in `PerAgentBeliefView`, shared carried-load helper alignment, plus Scenario 143 golden/doc refresh
**Deps**: `archive/tickets/S92FRECARCAP-001.md`, `archive/tickets/S92FRECARCAP-002.md`, `archive/tickets/S92FRECARCAP-003.md`, `archive/specs/S92-free-carry-capacity-zero-step-loop-fix.md`

## Problem

Scenario 143 (`degenerate_zero_step_loop_blocks_actionable_goals`) still proves the bug even after tickets 001-003: repeated zero-step `FreeCarryCapacity` plans, no late `eat` commit, and rising hunger. Reassessment shows the remaining contradiction is belief-view quantity parity for directly possessed lots: candidate emission/ranking can reconstruct carried strain from authoritative possessed-lot load, but planner snapshot load reconstruction was still reading stale `last_known_inventory` for directly possessed item lots through `PerAgentBeliefView::commodity_quantity()`. This ticket now owns that directly-possessed-quantity production fix, the shared carried-load helper alignment, and the downstream golden/doc flip.

## Assumption Reassessment (2026-04-11)

1. `degenerate_zero_step_loop_blocks_actionable_goals` exists at `crates/worldwake-ai/tests/golden_planner_pathology.rs:618`. Running `cargo test -p worldwake-ai degenerate_zero_step_loop_blocks_actionable_goals -- --nocapture` on 2026-04-11 still yields the failure-proof metrics `window_traces=120`, `window_selected=120`, `window_zero_step=120`, `hunger_start=238‰`, `hunger_after=476‰`, `late_eat_commit=false`. The ticket cannot remain a pure assertion flip. Corrected 2026-04-11.
2. Scenario substrate: `scenarios/cli-evaluation.ron`, seed `7777`, Eldergrove Forest, Forager Lina with `disposal_profile: (capacity_strain_threshold: 700)`. Confirmed 2026-04-11.
3. `scripts/golden_inventory.py` exists and accepts `--write --check-docs` flags. Confirmed 2026-04-11.
4. Snapshot-filter parity for `DropItem` was investigated on 2026-04-11 and proved non-causal for Scenario 143; the temporary hypothesis was removed after focused reruns showed the pathology unchanged. Corrected 2026-04-11.
5. Root cause: `PerAgentBeliefView::commodity_quantity(holder, kind)` returns authoritative quantities only for `holder == agent`, but directly possessed item lots are also physically accessible and should not fall back to stale `last_known_inventory`. Because planning snapshot commodity quantities are built through that view method, root `PlanningState` undercounted Lina's carried load and falsely satisfied `FreeCarryCapacity` at the root. Corrected 2026-04-11.
6. Live `GoalKind`: `FreeCarryCapacity`. After directly possessed lot quantities and the shared carried-load helper are aligned on concrete carried substrate, late-run planning should either stop selecting `FreeCarryCapacity` or produce executable `PlannerOpKind::DropItem` steps; the zero-step loop then becomes impossible on the real Scenario 143 substrate.

## Architecture Check

1. The remaining production contradiction is directly possessed lot quantity parity across `GoalBeliefView` consumers. The clean fix is to make `PerAgentBeliefView::commodity_quantity()` return authoritative quantities for directly possessed item lots and to keep the shared `FreeCarryCapacity` carried-load helper grounded in concrete carried possessions rather than agent-level controlled totals.
2. Flipping assertions from failure-proof to fix-proof remains the correct golden-layer outcome once the snapshot filter is corrected. No scenario substrate changes are needed.
3. No backward-compatibility shims. The stale directly-possessed quantity path and the old failure assertions are replaced directly.

## Verification Layers

1. Directly possessed item lots expose authoritative commodity quantity -> focused unit test in `crates/worldwake-sim/src/per_agent_belief_view.rs`
2. Shared `FreeCarryCapacity` carried-load helper ignores inflated controlled totals and uses concrete carried possessions -> focused unit coverage in `crates/worldwake-ai/src/{goal_model,candidate_generation,ranking}.rs`
3. No repeated zero-step FreeCarryCapacity plans -> golden E2E assertion on `PlanSearchOutcome::Found` step count
4. When FreeCarryCapacity selected, produces DropItem steps or planner switches goal -> golden E2E assertion on operator surface or goal switch
5. Self-care recovery signal (late eat commit or hunger decrease) -> golden E2E assertion on downstream behavior
6. Mixed-layer ticket with one `worldwake-sim` belief-view fix, one shared-helper alignment in `worldwake-ai`, and golden/doc fallout

## What to Change

### 1. Fix directly possessed item-lot quantity parity

In `crates/worldwake-sim/src/per_agent_belief_view.rs`:

1. When `holder` is directly possessed by the querying agent and is an `ItemLot`, return the authoritative lot quantity for `commodity_quantity(holder, kind)` instead of falling back to stale `last_known_inventory`.
2. Add a focused unit test proving a directly possessed lot returns its live authoritative quantity even when the agent's stored belief is stale.

### 2. Keep the shared carried-load helper on concrete carried substrate

In `crates/worldwake-ai/src/goal_model.rs` and focused `FreeCarryCapacity` tests:

1. Derive `free_carry_capacity_contract_from_view()` load from concrete carried possessions rather than agent-level controlled totals.
2. Update focused candidate/ranking coverage so it proves the helper ignores inflated controlled totals when carried load is below threshold.

### 3. Replace failure assertions with fix assertions

In `degenerate_zero_step_loop_blocks_actionable_goals`, replace the current assertions with:

1. **No repeated zero-step loop**: Assert that during the late-run observation window, there are zero (or bounded very small) `FreeCarryCapacity` plans returning `PlanSearchOutcome::Found { steps: [] }`.
2. **Executable disposal or goal switch**: When `FreeCarryCapacity` is selected, assert it produces a plan with `PlannerOpKind::DropItem` step(s), or the planner switches to another actionable self-care goal.
3. **Self-care recovery**: Assert at least one of: late `eat` commit occurs, hunger decreases or stabilizes during the observation window, or bounded inactivity (the idle run is broken).

Keep the exact substrate unchanged:
- `scenarios/cli-evaluation.ron`
- seed `7777`
- Eldergrove / Forager Lina setup
- late-run observation window after real waste accumulation

### 4. Refresh generated golden docs

Run:

```bash
python3 scripts/golden_inventory.py --write --check-docs
```

Commit any changes under `docs/generated/golden-*` caused by the updated Scenario 143 assertions.

## Files to Touch

- `crates/worldwake-sim/src/per_agent_belief_view.rs` (modify — directly possessed lot quantity parity and focused coverage)
- `crates/worldwake-ai/src/goal_model.rs` (modify — carried-load helper alignment)
- `crates/worldwake-ai/src/candidate_generation.rs` (modify — focused contract coverage alignment)
- `crates/worldwake-ai/src/ranking.rs` (modify — focused contract coverage alignment)
- `crates/worldwake-ai/tests/golden_planner_pathology.rs` (modify)
- `docs/generated/golden-e2e-inventory.md` (modify — regenerated)
- `docs/generated/golden-scenario-index.md` (modify — regenerated)
- `docs/generated/golden-scenario-details/golden_planner_pathology.md` (modify — regenerated)

## Out of Scope

- Reworking goal satisfaction semantics beyond the carried-load helper parity needed to match the live planner substrate
- Changing the scenario substrate (seed, scenario file, agent setup)
- Changing unrelated S91 pathologies (`budget_exhaustion_blocks_cross_location_water_acquisition` or `role_agent_generates_survival_goals_under_critical_needs`)
- Rebalancing hunger, metabolism, or utility weights

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-sim directly_possessed_item_lot_quantity_uses_authoritative_quantity_over_stale_belief -- --nocapture` — directly possessed lots expose live authoritative quantity
2. `cargo test -p worldwake-ai free_carry_capacity_ -- --nocapture` — shared carried-load helper and focused contract coverage aligned
3. `cargo test -p worldwake-ai degenerate_zero_step_loop_blocks_actionable_goals -- --nocapture` — now proves fix, not failure
4. `cargo test -p worldwake-ai golden_waste_disposal_cycle -- --nocapture` — existing S82 disposal cycle preserved
5. `cargo test -p worldwake-ai` — no regressions
6. `python3 scripts/golden_inventory.py --write --check-docs` — generated docs consistent
7. `cargo clippy --workspace --all-targets -- -D warnings` — clean clippy

### Invariants

1. Scenario 143 uses exact same substrate (cli-evaluation.ron, seed 7777, Forager Lina, late-run window) — no substrate changes
2. Directly possessed item lots expose live carried quantities to planner snapshot rebuilds; stale `last_known_inventory` no longer undercounts carried load
3. The golden proves lawful planner behavior (real disposal work or goal switch), not merely "stopped picking FreeCarryCapacity"
4. The pathological idle run is broken, evidenced by zero zero-step late-run plans and downstream self-care recovery

## Test Plan

### New/Modified Tests

1. `crates/worldwake-sim/src/per_agent_belief_view.rs` — add directly possessed lot quantity parity coverage
2. `crates/worldwake-ai/src/{goal_model,candidate_generation,ranking}.rs` — align focused `FreeCarryCapacity` helper coverage
3. `crates/worldwake-ai/tests/golden_planner_pathology.rs::degenerate_zero_step_loop_blocks_actionable_goals` — flip from failure-proof to fix-proof assertions

### Commands

1. `cargo test -p worldwake-sim directly_possessed_item_lot_quantity_uses_authoritative_quantity_over_stale_belief -- --nocapture`
2. `cargo test -p worldwake-ai free_carry_capacity_ -- --nocapture`
3. `cargo test -p worldwake-ai degenerate_zero_step_loop_blocks_actionable_goals -- --nocapture`
4. `cargo test -p worldwake-ai golden_waste_disposal_cycle -- --nocapture`
5. `cargo test -p worldwake-ai`
6. `python3 scripts/golden_inventory.py --write --check-docs`
7. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

Completed 2026-04-11.

`PerAgentBeliefView::commodity_quantity()` now returns authoritative quantities for directly possessed item lots, so planner snapshot rebuilds no longer undercount carried load from stale `last_known_inventory`. `free_carry_capacity_contract_from_view()` now derives carried load from concrete direct possessions instead of agent-level controlled totals, focused `FreeCarryCapacity` tests were aligned to that contract, and Scenario 143 was flipped from failure-proof to fix-proof with late-run assertions for zero zero-step plans, lawful `DropItem` execution or goal switch, and downstream self-care recovery.

## Verification Result

Passed 2026-04-11:

1. `cargo test -p worldwake-sim directly_possessed_item_lot_quantity_uses_authoritative_quantity_over_stale_belief -- --nocapture`
2. `cargo test -p worldwake-ai free_carry_capacity_ -- --nocapture`
3. `cargo test -p worldwake-ai degenerate_zero_step_loop_blocks_actionable_goals -- --nocapture`
4. `cargo test -p worldwake-ai golden_waste_disposal_cycle -- --nocapture`
5. `cargo test -p worldwake-ai`
6. `python3 scripts/golden_inventory.py --write --check-docs`
7. `cargo clippy --workspace --all-targets -- -D warnings`
