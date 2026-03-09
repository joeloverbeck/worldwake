# E08TIMSCHREP-010: SimulationState root struct

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — new root type in worldwake-sim
**Deps**: E08TIMSCHREP-002 (DeterministicRng), E08TIMSCHREP-004 (ControllerState), E08TIMSCHREP-005 (Scheduler), E08TIMSCHREP-008 (ReplayState)

## Problem

The spec requires a single authoritative simulation root that bundles all state needed for save/load and replay: world, event log, scheduler, replay state, controller state, and RNG. Without this, save/load must manually gather scattered state, risking omissions that break round-trip integrity (Spec 9.19).

## Assumption Reassessment (2026-03-09)

1. `World` exists in `worldwake-core` and already derives `Clone + Debug + Eq + PartialEq + Serialize + Deserialize` — confirmed
2. `EventLog` exists in `worldwake-core` and already derives `Clone + Debug + Eq + PartialEq + Serialize + Deserialize` — confirmed
3. `Scheduler` from E08TIMSCHREP-005 already exists and already derives `Clone + Debug + Eq + PartialEq + Serialize + Deserialize` — confirmed
4. `ReplayState` from E08TIMSCHREP-008 already exists and already derives `Clone + Debug + Eq + PartialEq + Serialize + Deserialize` — confirmed
5. `ControllerState` from E08TIMSCHREP-004 already exists and already derives `Clone + Debug + Eq + PartialEq + Serialize + Deserialize` — confirmed
6. `DeterministicRng` from E08TIMSCHREP-002 already exists and already derives `Clone + Debug + Eq + PartialEq + Serialize + Deserialize` — confirmed
7. Canonical hashing helpers already exist in `worldwake-core` as `hash_serializable(...) -> Result<StateHash, CanonicalError>` — confirmed
8. `replay_execution.rs` already computes a narrower replay bootstrap hash over `(World, EventLog, Scheduler, ControllerState)` via a private helper; that is not the same thing as a full `SimulationState` hash — confirmed
9. No `SimulationState` exists yet — confirmed

## Architecture Check

1. `SimulationState` is a flat struct — no inheritance, no trait objects, no dynamic dispatch
2. All fields are fully owned and serializable — this is the single source of truth
3. The struct is the unit of save/load — you save one `SimulationState`, you load one back
4. Replay bootstrap hashing remains a separate concern because replay verification must compare the initial authoritative runtime roots without recursively folding in the evolving `ReplayState`
5. Fields stay private; mutation flows through explicit accessors so the root can gain invariants later without a breaking redesign

## What to Change

### 1. New type: `SimulationState`

```rust
pub struct SimulationState {
    world: World,
    event_log: EventLog,
    scheduler: Scheduler,
    replay_state: ReplayState,
    controller_state: ControllerState,
    rng_state: DeterministicRng,
}
```

Derive: `Clone, Debug, Eq, PartialEq, Serialize, Deserialize`.

### 2. Constructor and accessors

- `new(world, event_log, scheduler, replay_state, controller_state, rng_state) -> Self`
- Individual field accessors (both `&` and `&mut` where needed for tick stepping)
- `hash(&self) -> Result<StateHash, CanonicalError>` — canonical hash of the full simulation state using `worldwake_core::hash_serializable`

## Files to Touch

- `crates/worldwake-sim/src/simulation_state.rs` (new)
- `crates/worldwake-sim/src/lib.rs` (modify — add module + re-export)

## Out of Scope

- Save/load to disk (E08TIMSCHREP-011)
- Refactoring replay execution to accept `SimulationState` directly; that is a follow-up architectural cleanup, not required for introducing the authoritative root value
- Construction helpers that build a "default" simulation (that's scenario setup, later epics)
- The `SystemRegistry` is NOT part of `SimulationState` — it's code, not data
- Replacing replay's existing bootstrap hash helper with full-state hashing; replay verification still needs its narrower root hash

## Acceptance Criteria

### Tests That Must Pass

1. `SimulationState` satisfies `Clone + Debug + Eq + PartialEq + Serialize + Deserialize`
2. Construction with all fields populates correctly (accessor round-trip)
3. Bincode round-trip: serialize a populated `SimulationState`, deserialize, verify equality
4. `hash()` returns identical `StateHash` for identical states
5. `hash()` returns different `StateHash` when any owned field changes
6. Existing suite: `cargo test -p worldwake-sim`

### Invariants

1. All fields are serializable — no `#[serde(skip)]` on any field
2. `SimulationState` is the complete authoritative state — nothing outside it is needed for save/load
3. `SystemRegistry` (code/functions) is NOT included — it's reconstructed at load time
4. Full-state hashing is infallible only at the call site; the API itself must surface `CanonicalError` instead of panicking

## Test Plan

### New/Modified Tests

1. `crates/worldwake-sim/src/simulation_state.rs` (inline `#[cfg(test)]`) — construction, hashing, serialization

### Commands

1. `cargo test -p worldwake-sim simulation_state`
2. `cargo clippy --workspace && cargo test --workspace`

## Outcome

- Added `SimulationState` in `worldwake-sim` as the private-field authoritative root for `World`, `EventLog`, `Scheduler`, `ReplayState`, `ControllerState`, and `DeterministicRng`
- Added constructor, immutable accessors, mutable accessors, and `hash(&self) -> Result<StateHash, CanonicalError>`
- Exported `SimulationState` from `worldwake-sim`
- Added focused tests for trait bounds, accessor round-trips, mutation through accessors, bincode round-trip, and full-state hash stability/change detection
- Corrected the original ticket scope: replay bootstrap hashing remains a narrower helper and was not refactored to use `SimulationState` in this ticket
