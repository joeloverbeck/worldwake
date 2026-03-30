# S46GOLGAP-002: Add replay companion for Scenario 57

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: None - test-only ticket
**Deps**: `archive/tickets/guard-patrol/S46GOLGAP-001.md`, `specs/S46-golden-gaps-E19.md`

## Problem

Scenario 57 already exists in `crates/worldwake-ai/tests/golden_patrol.rs` as `golden_patrol_driven_crime_discovery`, but unlike Scenario 52 it does not yet expose a shared runner plus replay companion that proves the scenario replays deterministically from the same seed. This ticket is only about adding that determinism proof without changing the live architecture or rewriting Scenario 57's assertions.

## Assumption Reassessment (2026-03-30)

1. **Shared abstraction boundary under audit**: this ticket stays at the golden determinism boundary for the already-implemented Scenario 57 runner in [`crates/worldwake-ai/tests/golden_patrol.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_patrol.rs). It does not reopen the production patrol -> perception -> investigation contract from S46GOLGAP-001.
2. **Scenario 57 already exists**: `cargo test -p worldwake-ai --test golden_patrol -- --list` shows `golden_patrol_driven_crime_discovery` is already present. The original draft incorrectly treated S46GOLGAP-001 as still pending.
3. **Live scenario outcome is `WitnessedAbsence`, not `SuspectedTheft`**: the existing Scenario 57 test in [`crates/worldwake-ai/tests/golden_patrol.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_patrol.rs) asserts a non-owner patrol investigator records `SocialObservationDetail::WitnessedAbsence` and does not mint `ViolationKind::SuspectedTheft`. Any replay helper must preserve those assertions exactly rather than reintroducing the earlier stale theft narrative.
4. **The current file does not yet provide a replay helper for Scenario 57**: `golden_patrol.rs` has a helper/replay pair for Scenario 52 via `run_patrol_cycle(...)`, but Scenario 57 is still a monolithic test body. The missing work is real.
5. **Hashing surface is live and stable**: `worldwake_core::hash_world`, `worldwake_core::hash_event_log`, and `worldwake_core::StateHash` already back existing replay companions across the golden suite. They remain the right determinism proof surface for this ticket.
6. **The seed is already live**: Scenario 57 currently runs with `Seed([0x56; 32])` in the existing golden. This ticket should keep that seed rather than renumbering scenarios or altering the scenario identity.
7. **Repo policy is narrower than the old ticket text claimed**: current `golden_patrol.rs` contains some replay companions but not one for every scenario. The sound claim here is that Scenario 57 lacks its replay companion, not that the repository enforces an unconditional replay twin for every golden test.
8. **No planner/operator scope belongs here**: S46GOLGAP-001 already delivered the production planner fix and the live goal surface. This ticket should not broaden back into `GoalKind`, affordance, or runtime-engine changes.

## Architecture Check

1. The cleaner architecture is to factor Scenario 57 behind a file-local runner that owns all correctness assertions and returns `(StateHash, StateHash)`, matching the established golden replay pattern already used elsewhere in the suite.
2. Reusing the exact Scenario 57 assertions in the runner is stronger than cloning part of the test into a second replay-only body because it guarantees both runs exercise the same patrol/perception/investigation chain.
3. No alias paths or backwards-compatibility shims are needed. The existing top-level Scenario 57 test should simply delegate to the shared runner, and the new replay test should call that same runner twice.

## Verification Layers

1. Scenario 57 correctness invariants -> existing decision-trace, action-trace, authoritative `ViolationMemory`, and authoritative `AgentBeliefStore` assertions inside the shared runner
2. Deterministic reproduction of final world state -> `worldwake_core::hash_world`
3. Deterministic reproduction of final event history -> `worldwake_core::hash_event_log`

## What to Change

### 1. Extract Scenario 57 into a shared runner

Refactor the existing `golden_patrol_driven_crime_discovery` body in `crates/worldwake-ai/tests/golden_patrol.rs` into:

```rust
fn run_patrol_driven_crime_discovery(seed: Seed) -> (StateHash, StateHash)
```

The shared runner must:

- keep the current Scenario 57 setup and assertions intact
- preserve the live non-owner `WitnessedAbsence` / no-`SuspectedTheft` boundary
- return `(hash_world(&h.world).unwrap(), hash_event_log(&h.event_log).unwrap())` after all assertions

### 2. Keep the existing primary test as a thin wrapper

`golden_patrol_driven_crime_discovery` should become:

```rust
#[test]
fn golden_patrol_driven_crime_discovery() {
    let _ = run_patrol_driven_crime_discovery(Seed([0x56; 32]));
}
```

### 3. Add the replay companion

Add:

```rust
#[test]
fn golden_patrol_driven_crime_discovery_replays_deterministically() {
    let first = run_patrol_driven_crime_discovery(Seed([0x56; 32]));
    let second = run_patrol_driven_crime_discovery(Seed([0x56; 32]));
    assert_eq!(
        first,
        second,
        "patrol-driven crime discovery scenario should replay deterministically"
    );
}
```

## Files to Touch

- `crates/worldwake-ai/tests/golden_patrol.rs` (modify - extract shared runner, add replay companion)

## Out of Scope

- Any production code or planner changes
- Rewriting Scenario 57's live assertions into a different behavioral contract
- Reopening the owner-only theft-inference architecture settled in S46GOLGAP-001
- Regenerating golden inventory docs unless another ticket explicitly requests it

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-ai --test golden_patrol golden_patrol_driven_crime_discovery`
2. `cargo test -p worldwake-ai --test golden_patrol golden_patrol_driven_crime_discovery_replays_deterministically`
3. `cargo test -p worldwake-ai --test golden_patrol`
4. `cargo clippy -p worldwake-ai --test golden_patrol -- -D warnings`

### Invariants

1. Scenario 57 still proves the same patrol-driven local discovery chain and the same non-owner `WitnessedAbsence` boundary as before this refactor.
2. Two runs with the same seed produce identical final world and event-log hashes.
3. Existing patrol goldens continue to pass unchanged.

## Tests

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_patrol.rs::golden_patrol_driven_crime_discovery`
Rationale: stays as the human-readable primary Scenario 57 entry point while delegating to the shared runner.
2. `crates/worldwake-ai/tests/golden_patrol.rs::golden_patrol_driven_crime_discovery_replays_deterministically`
Rationale: adds the missing determinism proof for Scenario 57 by running the exact same asserted scenario twice with the same seed.

### Commands

1. `cargo test -p worldwake-ai --test golden_patrol golden_patrol_driven_crime_discovery`
2. `cargo test -p worldwake-ai --test golden_patrol golden_patrol_driven_crime_discovery_replays_deterministically`
3. `cargo test -p worldwake-ai --test golden_patrol`
4. `cargo clippy -p worldwake-ai --test golden_patrol -- -D warnings`

## Outcome

- Completed: 2026-03-30
- What actually changed:
  - Extracted Scenario 57 in [`crates/worldwake-ai/tests/golden_patrol.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_patrol.rs) into `run_patrol_driven_crime_discovery(seed)` so the scenario's existing correctness assertions now live in one shared runner.
  - Kept `golden_patrol_driven_crime_discovery` as the primary thin wrapper around that runner.
  - Added `golden_patrol_driven_crime_discovery_replays_deterministically` to prove deterministic replay via world-hash and event-log-hash equality for `Seed([0x56; 32])`.
- Deviations from original plan:
  - Corrected the stale ticket narrative before implementation. The live scenario remains a non-owner `WitnessedAbsence` investigation chain, not a `SuspectedTheft` escalation path.
  - No production or doc-regeneration work was needed; the final scope stayed test-only.
- Verification results:
  - `cargo test -p worldwake-ai --test golden_patrol golden_patrol_driven_crime_discovery`
  - `cargo test -p worldwake-ai --test golden_patrol golden_patrol_driven_crime_discovery_replays_deterministically`
  - `cargo test -p worldwake-ai --test golden_patrol`
  - `cargo clippy -p worldwake-ai --test golden_patrol -- -D warnings`
