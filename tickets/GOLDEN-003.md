# GOLDEN-003: Memory Retention Decay — Forgotten Resource Forces Local Discovery (Scenario 11)

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: None — test-only ticket; all perception/belief system code exists (E14)
**Deps**: None

## Problem

No golden test proves that `enforce_capacity()` — the belief retention decay mechanism — drives real behavioral divergence. An agent with a short `memory_retention_ticks` should forget about a distant resource after the retention window passes, and then discover and use a local resource instead of traveling to the forgotten distant one.

This is the last remaining perception/belief golden scenario (score 5). It validates that belief eviction is not just a data cleanup operation but actually changes agent planning outcomes.

## Assumption Reassessment (2026-03-14)

1. `enforce_capacity()` exists in `AgentBeliefStore` and is called by the perception system in `worldwake-systems/src/perception.rs:61,132`.
2. `PerceptionProfile` has `memory_retention_ticks: u64` field (`belief.rs:163`). Default is 48 ticks.
3. `within_retention_window()` in `belief.rs:179` checks `current_tick - observed_tick <= retention_ticks`.
4. The prototype topology has `VILLAGE_SQUARE` and `ORCHARD_FARM` connected by multi-hop route (3+ hops via `EAST_FIELD_TRAIL`).
5. `golden_perception.rs` does not exist yet. If this scenario still merits a golden test after implementation, this ticket can create that file directly.
6. The harness provides water at `VILLAGE_SQUARE` (or can be seeded). A local food source can be placed as a ground commodity lot or a resource source at `VILLAGE_SQUARE`.

## Architecture Check

1. Test-only change. No new engine code. This ticket may create `golden_perception.rs` if that remains the cleanest place for the scenario after implementation.
2. Uses a custom `PerceptionProfile` with short `memory_retention_ticks` (e.g., 15-20 ticks) to make retention decay observable within the test window.
3. No backwards-compatibility shims. The scenario is additive.

## What to Change

### 1. Add `run_memory_decay_scenario()` to `golden_perception.rs`

Setup:
- Single agent (Dana) at `VILLAGE_SQUARE` with short `PerceptionProfile` (`memory_retention_ticks: 20`, `memory_capacity: 4`, `observation_fidelity: pm(875)`).
- Dana has high thirst (pm(800)) and fast thirst metabolism (pm(15)/tick) — will drink first, consuming ticks.
- Water lot at `VILLAGE_SQUARE` (`Quantity(2)`) so Dana can drink locally.
- Dana seeded with belief that apples exist at distant `ORCHARD_FARM` (3+ hops away). This belief has `observed_tick` set to `Tick(0)`.
- Dana has moderate hunger (pm(400)) — not immediately dominant over thirst, but will become dominant after thirst is handled.
- A local food source at `VILLAGE_SQUARE`: ground apple lot with `Quantity(2)` placed at `VILLAGE_SQUARE` (discoverable via passive observation once Dana is present and looking for food).
- All other needs low (fatigue pm(0), bladder pm(0), dirtiness pm(0)).

Emergent behavior to prove (within 100-tick window):
1. Dana drinks water at `VILLAGE_SQUARE` (thirst dominant) — this consumes 10-15+ ticks.
2. During/after drinking, 20+ ticks pass from `Tick(0)`.
3. `enforce_capacity()` evicts the stale `ORCHARD_FARM` apple belief (observed_tick too old relative to current tick).
4. Dana gets hungry — cannot plan to go to `ORCHARD_FARM` (forgotten; no belief entry).
5. Dana discovers local apples at `VILLAGE_SQUARE` via passive observation.
6. Dana eats local apples.

### 2. Add assertions

- **Belief eviction**: After 20+ ticks, Dana's `AgentBeliefStore` no longer contains the `ORCHARD_FARM` apple source entry.
- **No distant travel**: Dana does NOT leave `VILLAGE_SQUARE` during the scenario. If she had retained the distant belief, she would travel to `ORCHARD_FARM`. Instead she stays local.
- **Local hunger relief**: Dana's hunger decreases by end of scenario (she ate local food).
- **Conservation**: Apple and water lot totals are consistent every tick.
- **Deterministic replay**: Two runs with same seed produce identical hashes.

### 3. Add test functions

- `golden_memory_decay_forgotten_resource_forces_local_discovery` — main scenario test.
- `golden_memory_decay_replays_deterministically` — deterministic replay companion.

### 4. Update `reports/golden-e2e-coverage-analysis.md`

- Move Scenario 11 from "Part 3: Missing Scenarios" to "Part 1: Proven Emergent Scenarios" with full writeup.
- Update Part 2 cross-system interaction coverage: mark "Memory retention decay → belief eviction → changed candidate generation → local discovery" as **Yes**.
- Update Part 4 summary statistics: proven tests count, cross-system chains count.
- Remove Scenario 11 from "Pending Backlog Summary" and "Recommended Implementation Order".
- If all 3 scenarios (10, 11, 12) are now proven, update the "Pending Backlog" section to reflect empty backlog and final coverage numbers.

## Files to Touch

- `crates/worldwake-ai/tests/golden_perception.rs` (new or modify — add scenario runner + 2 test functions if this scenario remains worth a dedicated golden file)
- `crates/worldwake-ai/tests/golden_harness/mod.rs` (modify only if a real helper gap exists)
- `reports/golden-e2e-coverage-analysis.md` (modify — move scenario, update stats, possibly mark backlog complete)

## Out of Scope

- No changes to `worldwake-core`, `worldwake-sim`, `worldwake-systems`, or `worldwake-ai/src/` production code.
- No changes to `enforce_capacity()` logic or `PerceptionProfile` struct.
- No changes to other golden test files (`golden_care.rs`, `golden_combat.rs`, etc.).
- Do not test multi-agent belief divergence (that's Scenario 10 / GOLDEN-002).
- Do not modify the perception pipeline or observation system.
- Do not add gossip/report-based belief propagation scenarios.

## Acceptance Criteria

### Tests That Must Pass

1. `golden_memory_decay_forgotten_resource_forces_local_discovery` — Dana forgets distant apples after retention window, discovers and eats local food instead of traveling.
2. `golden_memory_decay_replays_deterministically` — two runs with same seed produce identical world and event-log hashes.
3. Existing suite: `cargo test --workspace` passes (no regressions).

### Invariants

1. **Belief eviction**: Dana's `AgentBeliefStore` does not contain `ORCHARD_FARM` apple belief after `memory_retention_ticks` have elapsed since the belief's `observed_tick`.
2. **No distant travel**: Dana remains at `VILLAGE_SQUARE` throughout the scenario (proving the forgotten belief changed her plan).
3. **Conservation**: Apple and water lot totals never increase.
4. **No manual action queueing**: All behavior is emergent through `AgentTickDriver` + `AutonomousControllerRuntime`.
5. **Deterministic replay**: Identical seeds produce identical hashes.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_perception.rs::golden_memory_decay_forgotten_resource_forces_local_discovery` — proves `enforce_capacity()` drives behavioral divergence.
2. `crates/worldwake-ai/tests/golden_perception.rs::golden_memory_decay_replays_deterministically` — proves determinism.

### Commands

1. `cargo test -p worldwake-ai --test golden_perception`
2. `cargo test --workspace`
3. `cargo clippy --workspace`
