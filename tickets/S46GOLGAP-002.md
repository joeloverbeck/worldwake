# S46GOLGAP-002: Implement replay companion for Scenario 57

**Status**: PENDING
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: None — test-only ticket
**Deps**: S46GOLGAP-001 (primary test must exist first)

## Problem

Every golden scenario requires a replay companion that runs the same scenario twice with the same seed and asserts world hash + event log hash match. This validates the determinism invariant (FND-9, FND-12). Scenario 57 needs its replay companion.

## Assumption Reassessment (2026-03-30)

1. **Replay pattern**: Established by `run_patrol_cycle` in `golden_patrol.rs` (lines 190–287) and `golden_patrol_cycle_wraps_route_replays_deterministically` (lines 314–318). Pattern: extract shared setup into a function returning `(StateHash, StateHash)`, call twice with same seed, assert equality.
2. **StateHash / hash_world / hash_event_log**: Exist at `crates/worldwake-core/src/canonical.rs`. Used by all existing replay companions.
3. **Seed type**: `worldwake_core::Seed([u8; 32])`. Each scenario uses a unique seed byte pattern (S52 uses `0x51`, S53 uses `0x52`, etc.).
4. **S46GOLGAP-001 dependency**: The primary test function must exist before refactoring its setup into a shared helper. This ticket extracts the setup into a `run_patrol_driven_crime_discovery(seed: Seed)` function that both the primary test and this replay companion call.
5. **No live GoalKind or operator surface changes**: This ticket only restructures test code; no planner or candidate generation surface is relevant.

## Architecture Check

1. Follows the exact replay companion pattern used by all other patrol scenarios (S52 `run_patrol_cycle`). The refactored helper function centralizes setup and execution, reducing duplication between the primary and replay tests.
2. No backwards-compatibility shims.

## Verification Layers

1. Determinism: world hash equality across two runs → `hash_world` comparison
2. Determinism: event log hash equality across two runs → `hash_event_log` comparison
3. Single-layer ticket: only determinism is under test. The primary test (S46GOLGAP-001) proves correctness; this test proves reproducibility.

## What to Change

### 1. Extract shared setup helper

Refactor the setup and execution logic from `golden_patrol_driven_crime_discovery` (written in S46GOLGAP-001) into a new file-local function:

```rust
fn run_patrol_driven_crime_discovery(seed: Seed) -> (StateHash, StateHash) {
    // All setup, theft simulation, and stepping from S46GOLGAP-001.
    // Return (hash_world(&h.world), hash_event_log(&h.event_log)).
}
```

Update `golden_patrol_driven_crime_discovery` to call this helper:
```rust
#[test]
fn golden_patrol_driven_crime_discovery() {
    let _ = run_patrol_driven_crime_discovery(Seed([0x56; 32]));
}
```

(Seed byte `0x56` follows the existing sequence: S52=`0x51`, S53=`0x52`, S54=`0x53`, S55=`0x54`, S56=`0x55`.)

### 2. Add replay companion test

```rust
#[test]
fn golden_patrol_driven_crime_discovery_replays_deterministically() {
    let first = run_patrol_driven_crime_discovery(Seed([0x56; 32]));
    let second = run_patrol_driven_crime_discovery(Seed([0x56; 32]));
    assert_eq!(first, second, "patrol-driven crime discovery scenario should replay deterministically");
}
```

### 3. Move assertions into the shared helper

The shared helper must still perform the correctness assertions from S46GOLGAP-001 (decision trace, action trace, ViolationMemory checks). This ensures the replay companion validates both determinism AND correctness on each run. This follows the `run_patrol_cycle` pattern which also asserts inside the helper.

## Files to Touch

- `crates/worldwake-ai/tests/golden_patrol.rs` (modify — refactor setup into helper, add replay test)

## Out of Scope

- Any engine/production code changes.
- Doc regeneration (ticket S46GOLGAP-003).
- Changing the primary test's assertions or scope — only restructuring into a shared helper.
- Modifying existing scenarios S52–S56.

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-ai --test golden_patrol golden_patrol_driven_crime_discovery` — primary test still passes after refactor.
2. `cargo test -p worldwake-ai --test golden_patrol golden_patrol_driven_crime_discovery_replays_deterministically` — replay companion passes.
3. `cargo test -p worldwake-ai --test golden_patrol` — all existing patrol golden tests (S52–S56) still pass.

### Invariants

1. Same seed produces identical `StateHash` for world and event log across two runs (determinism, FND-9).
2. ChaCha8Rng seeding is the sole source of randomness — no wall-clock time, no HashMap iteration order (FND-9).
3. Existing patrol scenarios S52–S56 continue to pass unchanged.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_patrol.rs::golden_patrol_driven_crime_discovery_replays_deterministically` — proves deterministic replay of the patrol-driven crime discovery chain.
2. `crates/worldwake-ai/tests/golden_patrol.rs::golden_patrol_driven_crime_discovery` — modified to call shared helper (behavior unchanged).

### Commands

1. `cargo test -p worldwake-ai --test golden_patrol golden_patrol_driven_crime_discovery_replays_deterministically`
2. `cargo test -p worldwake-ai --test golden_patrol`
3. `cargo clippy -p worldwake-ai`
