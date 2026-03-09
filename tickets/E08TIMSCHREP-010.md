# E08TIMSCHREP-010: SimulationState root struct

**Status**: PENDING
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — new root type in worldwake-sim
**Deps**: E08TIMSCHREP-002 (DeterministicRng), E08TIMSCHREP-004 (ControllerState), E08TIMSCHREP-005 (Scheduler), E08TIMSCHREP-008 (ReplayState)

## Problem

The spec requires a single authoritative simulation root that bundles all state needed for save/load and replay: world, event log, scheduler, replay state, controller state, and RNG. Without this, save/load must manually gather scattered state, risking omissions that break round-trip integrity (Spec 9.19).

## Assumption Reassessment (2026-03-09)

1. `World` exists in worldwake-core, derives `Serialize + Deserialize` — confirmed
2. `EventLog` exists in worldwake-core, derives `Serialize + Deserialize` — confirmed
3. `Scheduler` from E08TIMSCHREP-005 — will be serializable
4. `ReplayState` from E08TIMSCHREP-008 — will be serializable
5. `ControllerState` from E08TIMSCHREP-004 — will be serializable
6. `DeterministicRng` from E08TIMSCHREP-002 — will be serializable
7. No `SimulationState` exists yet — confirmed

## Architecture Check

1. `SimulationState` is a flat struct — no inheritance, no trait objects, no dynamic dispatch
2. All fields are fully owned and serializable — this is the single source of truth
3. The struct is the unit of save/load — you save one `SimulationState`, you load one back

## What to Change

### 1. New type: `SimulationState`

```rust
pub struct SimulationState {
    pub world: World,
    pub event_log: EventLog,
    pub scheduler: Scheduler,
    pub replay_state: ReplayState,
    pub controller_state: ControllerState,
    pub rng_state: DeterministicRng,
}
```

Derive: `Clone, Debug, Eq, PartialEq, Serialize, Deserialize`.

### 2. Constructor and accessors

- `new(world, event_log, scheduler, replay_state, controller_state, rng_state) -> Self`
- Individual field accessors (both `&` and `&mut` where needed for tick stepping)
- `hash(&self) -> StateHash` — canonical hash of the full simulation state

## Files to Touch

- `crates/worldwake-sim/src/simulation_state.rs` (new)
- `crates/worldwake-sim/src/lib.rs` (modify — add module + re-export)

## Out of Scope

- Save/load to disk (E08TIMSCHREP-011)
- Replay execution (E08TIMSCHREP-009)
- Construction helpers that build a "default" simulation (that's scenario setup, later epics)
- The `SystemRegistry` is NOT part of `SimulationState` — it's code, not data

## Acceptance Criteria

### Tests That Must Pass

1. `SimulationState` satisfies `Clone + Debug + Eq + PartialEq + Serialize + Deserialize`
2. Construction with all fields populates correctly (accessor round-trip)
3. Bincode round-trip: serialize a populated `SimulationState`, deserialize, verify equality
4. `hash()` returns identical `StateHash` for identical states
5. `hash()` returns different `StateHash` when any field changes
6. Existing suite: `cargo test -p worldwake-sim`

### Invariants

1. All fields are serializable — no `#[serde(skip)]` on any field
2. `SimulationState` is the complete authoritative state — nothing outside it is needed for save/load
3. `SystemRegistry` (code/functions) is NOT included — it's reconstructed at load time

## Test Plan

### New/Modified Tests

1. `crates/worldwake-sim/src/simulation_state.rs` (inline `#[cfg(test)]`) — construction, hashing, serialization

### Commands

1. `cargo test -p worldwake-sim simulation_state`
2. `cargo clippy --workspace && cargo test --workspace`
