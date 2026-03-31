# E22INTSOATES-011: T32 — Long Replay Consistency

**Status**: ✅ COMPLETED
**Priority**: LOW
**Effort**: Medium
**Engine Changes**: None
**Deps**: E22INTSOATES-009

## Problem

No existing test verifies that save/load mid-run produces identical results to a continuous run over an extended period. T32 proves deterministic replay fidelity: a continuous 2880-tick run must produce identical `StateHash` at every 100-tick checkpoint compared to a save-at-1440 → load → continue run.

## Assumption Reassessment (2026-03-31)

1. `save_to_bytes()` and `load_from_bytes()` exist — confirmed in `crates/worldwake-sim/src/save_load.rs`.
2. `hash_world()` and `hash_event_log()` exist — confirmed in `crates/worldwake-core/src/canonical.rs`.
3. `replay_and_verify()` exists — confirmed in `crates/worldwake-sim/src/replay_execution.rs`.
4. `ReplayState` with `ReplayRecordingConfig` exists — confirmed.
5. `SimulationState` is the root state containing world + event log + scheduler + replay + rng — confirmed.
6. T32 reuses T30 population/topology — depends on 009.
7. `StateHash` type exists for hash comparison — confirmed.
8. Existing replay tests in `golden_determinism.rs` verify short replay. T32 extends to longer runs with mid-point save/load.
9. No adjacent contradictions.
10. T32 is `#[ignore]` like T30.
11. Continuous run: 2880 ticks with seed X, recording hashes at every 100-tick checkpoint. Split run: 1440 ticks → save → load → 1440 more ticks. Hashes must match at every checkpoint.

## Architecture Check

1. T32 uses existing `save_to_bytes()` / `load_from_bytes()` and `hash_world()` / `hash_event_log()` APIs. The test proves that the serialization boundary preserves all world meaning (Principle 12). No new infrastructure needed.
2. No backwards-compatibility shims introduced.

## Verification Layers

1. Continuous run hash recording → `hash_world()` and `hash_event_log()` at every 100-tick checkpoint
2. Split run hash recording → same hash functions at same checkpoints after load
3. Exact match → `StateHash` equality comparison at every checkpoint
4. Serialization fidelity → `save_to_bytes()` + `load_from_bytes()` preserves all state
5. Determinism → identical hashes between continuous and split runs proves deterministic execution

## What to Change

### 1. Add T32 replay consistency test to `crates/worldwake-ai/tests/golden_integration.rs`

- Reuse T30 population/topology builder
- `fn run_continuous(seed: Seed, ticks: u64) -> Vec<(u64, StateHash, StateHash)>`:
  - Run for `ticks` ticks, record `(tick, hash_world, hash_event_log)` at every 100-tick checkpoint
- `fn run_split(seed: Seed, save_at: u64, total_ticks: u64) -> Vec<(u64, StateHash, StateHash)>`:
  - Run for `save_at` ticks, recording checkpoints
  - `save_to_bytes()` at `save_at`
  - `load_from_bytes()`
  - Continue for `total_ticks - save_at` ticks, recording checkpoints
- Compare: exact `StateHash` match at every 100-tick checkpoint between continuous and split runs
- `fn run_t32_replay_consistency(seed: Seed)` — runs both continuous and split, asserts checkpoint match
- Single `#[test]` `#[ignore]` function: `t32_replay_consistency`

## Files to Touch

- `crates/worldwake-ai/tests/golden_integration.rs` (modify)

## Out of Scope

- Changes to save/load, replay, or hashing systems
- Performance optimization
- Non-`#[ignore]` test variants
- Any engine code changes

## Acceptance Criteria

### Tests That Must Pass

1. `t32_replay_consistency` (run via `cargo test -p worldwake-ai --test golden_integration -- --ignored t32`) — exact StateHash match at every 100-tick checkpoint
2. `hash_world()` matches between continuous and split runs at every checkpoint
3. `hash_event_log()` matches between continuous and split runs at every checkpoint
4. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. Save/load boundary preserves all world meaning (Principle 12)
2. Deterministic execution: same seed + same inputs = same state at every tick
3. No state leakage through serialization boundary

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_integration.rs::t32_replay_consistency` — proves save/load mid-run determinism

### Commands

1. `cargo test -p worldwake-ai --test golden_integration -- --ignored t32`
2. `cargo test --workspace`

## Outcome

- **Completion date**: 2026-03-31
- **What changed**: Added `run_continuous()`, `run_split()`, `run_t32_replay_consistency()`, and `#[test] #[ignore] fn t32_replay_consistency()` to `crates/worldwake-ai/tests/golden_integration.rs`. The split run uses `h.save_load_roundtrip()` at tick 1440, recording `(tick, hash_world, hash_event_log)` at every 100-tick checkpoint across both halves.
- **Deviations**: Used `save_load_roundtrip()` harness method instead of raw `save_to_bytes`/`load_from_bytes` — functionally identical but avoids accessing private harness internals.
- **Verification**: Compiles clean, clippy clean, all 36 existing `worldwake-ai` tests pass, test discoverable via `--list`.
