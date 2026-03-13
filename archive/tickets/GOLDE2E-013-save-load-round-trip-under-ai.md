# GOLDE2E-013: Save/Load Round-Trip Under AI

**Status**: ✅ COMPLETED
**Priority**: LOW
**Effort**: Medium
**Engine Changes**: Unlikely — reassessment shows AI controller runtime is intentionally transient
**Deps**: None (save/load exists from E08, AI runtime from E13)

## Problem

Save/load is tested in `worldwake-sim`, including a generic continuation check, but not under the real golden AI loop. The missing coverage is specifically: run autonomous agents for some ticks, save authoritative simulation state, load it, rebuild the AI controller runtime, and prove continuation remains deterministic relative to an uninterrupted run.

## Report Reference

Backlog item **P18** in `reports/golden-e2e-coverage-analysis.md` (Tier 3, composite score 2).

## Assumption Reassessment (2026-03-13)

1. `save()` / `load()` and `save_to_bytes()` / `load_from_bytes()` exist in `worldwake-sim/src/save_load.rs` using bincode.
2. `SimulationState` is the serialized root and currently includes `World`, `EventLog`, `Scheduler`, `RecipeRegistry`, `ReplayState`, `ControllerState`, and deterministic RNG state.
3. `AutonomousControllerRuntime` is not serialized state; it is an ephemeral wrapper around controller references in `worldwake-sim/src/autonomous_controller.rs`.
4. `AgentDecisionRuntime` is also not serialized and is explicitly not registered as an authoritative component in `worldwake-ai/src/decision_runtime.rs`.
5. `BlockedIntentMemory` is authoritative component state on agents and already participates in save/load through the world component schema.
6. The golden harness will likely need a narrow helper to round-trip through `SimulationState` and resume with a fresh `AgentTickDriver`.

## Architecture Check

1. All authoritative state must survive save/load round-trips.
2. Transient AI controller/runtime state should be rebuilt from authoritative state after load rather than serialized.
3. The new test should validate that the current architecture is sufficient, not push the engine toward persisting non-authoritative planner cache state.

## Engine-First Mandate

If implementing this e2e suite reveals a real gap in authoritative state persistence, fix the authoritative model itself. Do not serialize `AutonomousControllerRuntime` or `AgentDecisionRuntime` just to preserve planner cache state. The cleaner architecture is: authoritative world/scheduler/RNG state persists; transient AI runtime is reconstructed on resume; broken reconstruction paths get fixed at their real source.

## What to Change

### 1. New golden test in `golden_determinism.rs`

**Setup**: A golden scenario that exercises real AI planning across multiple ticks. Run for `N` ticks, snapshot through `SimulationState`, load, continue with a fresh `AgentTickDriver` for `M` more ticks. Also run continuously for `N + M` ticks without save/load.

**Assertions**:
- Save at tick N succeeds.
- Load from save produces a valid simulation state.
- Continuing from load for M more ticks succeeds without crash.
- The final authoritative world and event-log state after save/load/continue matches the continuous `N + M` tick run.
- A fresh controller runtime after load is sufficient to resume coherent AI behavior; no serialized planner cache is required.

### 2. Harness support only if required

Add the smallest helper needed to convert a golden harness snapshot into `SimulationState` and resume it. Do not broaden the harness API unless the test genuinely needs it.

## Files to Touch

- `crates/worldwake-ai/tests/golden_determinism.rs` (modify)
- `crates/worldwake-ai/tests/golden_harness/mod.rs` (modify, if helpers needed)
- Engine files only if authoritative persistence or reconstruction is actually broken

## Out of Scope

- Save/load performance benchmarks
- Backwards compatibility with older save formats
- Persisting transient planner caches or controller reference wrappers
- Broad save/load engine redesign unless the new golden coverage exposes a real authoritative-state defect

## Acceptance Criteria

### Tests That Must Pass

1. `golden_save_load_round_trip_under_ai` — save mid-simulation, load, continue with a fresh AI runtime produces identical authoritative results
2. Existing suite: `cargo test -p worldwake-ai golden_`
3. Full workspace: `cargo test --workspace`

### Invariants

1. Determinism holds across the save/load boundary
2. Conservation holds in both segments
3. AI agents continue behaving coherently after load via runtime reconstruction, not serialized planner cache state

## Post-Implementation

After implementing this suite, update `reports/golden-e2e-coverage-analysis.md`:
- Add the new scenario to Part 1 (Proven Emergent Scenarios)
- Remove P18 from the Part 3 backlog
- Update Part 4 summary statistics

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_determinism.rs::golden_save_load_round_trip_under_ai` — proves deterministic AI continuation across save/load with reconstructed runtime

### Commands

1. `cargo test -p worldwake-ai golden_save_load`
2. `cargo test --workspace && cargo clippy --workspace`

## Outcome

- Completion date: 2026-03-13
- Actual changes:
  - Corrected the ticket assumptions before implementation: `SimulationState` persists authoritative simulation roots, while `AutonomousControllerRuntime` and `AgentDecisionRuntime` remain intentionally transient.
  - Added minimal golden-harness support to snapshot authoritative state through `SimulationState`, round-trip it through save/load, and resume with a fresh `AgentTickDriver`.
  - Added `golden_save_load_round_trip_under_ai` in `crates/worldwake-ai/tests/golden_determinism.rs`.
  - Updated `reports/golden-e2e-coverage-analysis.md` to record the new proven scenario and remove backlog item `P18`.
- Deviations from original plan:
  - No engine persistence redesign was needed.
  - The test now proves clean runtime reconstruction after load instead of trying to serialize transient AI planner/controller cache state.
- Verification results:
  - `cargo test -p worldwake-ai golden_save_load_round_trip_under_ai -- --nocapture`
  - `cargo test -p worldwake-ai golden_ -- --nocapture`
  - `cargo test --workspace`
  - `cargo clippy --workspace --all-targets -- -D warnings`
