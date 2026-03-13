# GOLDE2E-013: Save/Load Round-Trip Under AI

**Status**: PENDING
**Priority**: LOW
**Effort**: Medium
**Engine Changes**: Possible — AI runtime state may not fully serialize/deserialize
**Deps**: None (save/load exists from E08, AI runtime from E13)

## Problem

Save/load is tested in `worldwake-sim` but not with the full AI loop. Saving mid-simulation with active AI-controlled agents, loading the snapshot, and continuing should produce consistent outcomes. If AI runtime state (current goal, plan, decision runtime) is not correctly serialized, the simulation would diverge after load.

## Report Reference

Backlog item **P18** in `reports/golden-e2e-coverage-analysis.md` (Tier 3, composite score 2).

## Assumption Reassessment (2026-03-13)

1. `save()` / `load()` exist in `worldwake-sim/src/save_load.rs` using bincode.
2. `SimulationState` is the root state that gets serialized.
3. `AgentDecisionRuntime` and `AutonomousControllerRuntime` may or may not be included in the save state — verify.
4. The golden harness may need helpers for mid-simulation save/load.

## Architecture Check

1. All authoritative state (including AI decision runtime) must survive save/load round-trips.
2. If AI runtime state is transient/reconstructible, the test should prove that reconstruction produces equivalent behavior.

## Engine-First Mandate

If implementing this e2e suite reveals that AI runtime state (decision runtime, current goals, plans, blocked intents) does not correctly survive save/load round-trips — do NOT patch around it with workarounds. Instead, design and implement a comprehensive architectural solution that makes AI state persistence clean, robust, and extensible. Document any engine changes in the ticket outcome.

## What to Change

### 1. New golden test in `golden_determinism.rs`

**Setup**: Two AI-controlled agents in a standard scenario. Run for N ticks, save, load, continue for M more ticks. Also run continuously for N+M ticks without save/load.

**Assertions**:
- Save at tick N succeeds.
- Load from save produces a valid simulation state.
- Continuing from load for M more ticks succeeds without crash.
- The final state after save/load/continue matches (or is behaviorally equivalent to) the continuous N+M tick run.

## Files to Touch

- `crates/worldwake-ai/tests/golden_determinism.rs` (modify)
- `crates/worldwake-ai/tests/golden_harness/mod.rs` (modify, if helpers needed)
- Engine files TBD if AI state serialization is incomplete

## Out of Scope

- Save/load performance benchmarks
- Backwards compatibility with older save formats
- Save/load during active action (if this is a separate concern)

## Acceptance Criteria

### Tests That Must Pass

1. `golden_save_load_round_trip_under_ai` — save mid-simulation, load, continue produces consistent results
2. Existing suite: `cargo test -p worldwake-ai golden_`
3. Full workspace: `cargo test --workspace`

### Invariants

1. Determinism holds across save/load boundary
2. Conservation holds in both segments
3. AI agents continue behaving coherently after load

## Post-Implementation

After implementing this suite, update `reports/golden-e2e-coverage-analysis.md`:
- Add the new scenario to Part 1 (Proven Emergent Scenarios)
- Remove P18 from the Part 3 backlog
- Update Part 4 summary statistics

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_determinism.rs::golden_save_load_round_trip_under_ai` — proves AI state persistence

### Commands

1. `cargo test -p worldwake-ai golden_save_load`
2. `cargo test --workspace && cargo clippy --workspace`
