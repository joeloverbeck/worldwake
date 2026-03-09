# E08TIMSCHREP-011: Versioned save/load persistence

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — new module in worldwake-sim
**Deps**: E08TIMSCHREP-010 (SimulationState)

## Problem

The simulation must support saving to and loading from disk with versioned binary format. Save/load is a semantic round-trip — not just "it deserializes" but "the simulation continues identically." Old or mismatched format versions must fail cleanly, not silently corrupt state (Spec 9.19).

## Assumption Reassessment (2026-03-09)

1. `SimulationState` from E08TIMSCHREP-010 is the single serializable root
2. `bincode` serialization is already exercised heavily in tests across `worldwake-core` and `worldwake-sim`, but `worldwake-sim` currently declares it only as a `dev-dependency`; this ticket must promote it to a runtime dependency before save/load APIs can compile
3. `serde` derives on the runtime roots that compose `SimulationState`, and existing bincode round-trip tests already cover `World`, `EventLog`, `Scheduler`, `ReplayState`, `ControllerState`, and `DeterministicRng` — confirmed
4. No save/load exists yet — confirmed
5. Replay and checkpoint helpers now operate on `SimulationState` directly; save/load should preserve that root-first boundary instead of reintroducing separate root arguments — confirmed
6. Reservations are already authoritative world/action state, not a separate scheduler-owned save root; persistence must preserve them by saving the full `SimulationState`, not by introducing a parallel reservation snapshot path — confirmed

## Architecture Check

1. Save file format: `[magic bytes][format version u32][bincode payload]`
2. Magic bytes: fixed 4-byte identifier (e.g., `b"WWAK"`) to detect non-save files
3. Format version: `u32` starting at `1` — incremented on breaking schema changes
4. Loading checks magic + version before deserializing — mismatches produce clear errors, not panics
5. No compression for prototype scope — bincode is already compact
6. Save/load APIs should stay root-first: callers pass or receive `SimulationState`, and continuation tests should exercise the loaded root directly
7. `save_to_bytes` / `load_from_bytes` should be the primary persistence boundary; filesystem helpers should stay as thin wrappers over the byte-framing logic so tests can cover format behavior without coupling to disk

## What to Change

### 1. New constants

```rust
pub const SAVE_MAGIC: [u8; 4] = *b"WWAK";
pub const SAVE_FORMAT_VERSION: u32 = 1;
```

### 2. New type: `SaveError`

```rust
pub enum SaveError {
    Io(std::io::Error),
    Serialization(String),
    InvalidMagic,
    UnsupportedVersion { found: u32, expected: u32 },
    Deserialization(String),
}
```

### 3. New functions: `save` and `load`

```rust
pub fn save(state: &SimulationState, path: &Path) -> Result<(), SaveError>
pub fn load(path: &Path) -> Result<SimulationState, SaveError>
```

`save`:
1. Delegate to `save_to_bytes`
2. Write the framed bytes to disk without changing the payload

`load`:
1. Read file bytes
2. Delegate to `load_from_bytes`

### 4. New function: `save_to_bytes` / `load_from_bytes`

In-memory variants for testing without filesystem:
```rust
pub fn save_to_bytes(state: &SimulationState) -> Result<Vec<u8>, SaveError>
pub fn load_from_bytes(bytes: &[u8]) -> Result<SimulationState, SaveError>
```

`save_to_bytes` / `load_from_bytes` own the format framing:
1. Serialize or deserialize `SimulationState` with bincode
2. Prepend or validate magic bytes + version header
3. Reject truncated headers/payloads and mismatched versions without panicking

## Files to Touch

- `crates/worldwake-sim/Cargo.toml` (modify — promote `bincode` to runtime dependency)
- `crates/worldwake-sim/src/save_load.rs` (new)
- `crates/worldwake-sim/src/lib.rs` (modify — add module + re-exports)

## Out of Scope

- Compression (not needed for prototype)
- Migration between format versions (deferred — version 1 only for now)
- Autosave or periodic save logic
- Save file browser / listing
- CLI integration for save/load (E21)
- Refactoring runtime/replay APIs away from `SimulationState`; this ticket should follow the root-first architecture established in E08TIMSCHREP-010

## Acceptance Criteria

### Tests That Must Pass

1. **T09 — Save/load round-trip**: save a populated `SimulationState`, load it back, verify exact equality
2. Save/load round-trip with active actions, queued inputs, replay checkpoints, and RNG state
3. After load, continuing the simulation produces identical results to uninterrupted execution (semantic round-trip)
4. `load` rejects files with wrong magic bytes — returns `SaveError::InvalidMagic`
5. `load` rejects files with wrong version — returns `SaveError::UnsupportedVersion`
6. `load` rejects truncated files — returns `SaveError::Deserialization`
7. `save_to_bytes` / `load_from_bytes` round-trip matches file-based round-trip
8. Saved file includes the full authoritative `SimulationState` payload: world, event log, scheduler queue + active actions, controller state, RNG state, replay state, and reservation state carried by world/action data (verify with representative non-default loaded fields rather than a second hand-maintained inventory)
9. Existing suite: `cargo test -p worldwake-sim`

### Invariants

1. Save file format version is stored explicitly (Spec requirement)
2. Old or mismatched versions fail cleanly (Spec requirement)
3. Save/load is a semantic round-trip — simulation continues identically (Spec 9.19)
4. No silent data loss on round-trip
5. Save/load does not reintroduce exploded authoritative-root APIs where a single `SimulationState` root is sufficient

## Test Plan

### New/Modified Tests

1. `crates/worldwake-sim/src/save_load.rs` (inline `#[cfg(test)]`) — byte round-trip, file round-trip, header validation, truncated payload rejection, semantic continuation test

### Commands

1. `cargo test -p worldwake-sim save_load`
2. `cargo clippy --workspace && cargo test --workspace`

## Outcome

- Added versioned save/load persistence in `worldwake-sim` with fixed magic bytes, an explicit little-endian `u32` format version, and a `SimulationState` payload
- Added `save_to_bytes` / `load_from_bytes` as the primary persistence boundary and kept `save` / `load` as thin filesystem wrappers
- Promoted `bincode` from a `dev-dependency` to a runtime dependency in `worldwake-sim`; the original ticket understated this requirement
- Added focused save/load tests for full non-default state round-trip, file parity with the in-memory format, invalid magic, unsupported version, truncated header/payload rejection, and deterministic post-load continuation
- Kept the architecture root-first: persistence operates on `SimulationState` only, and reservation state is preserved through existing world/action data rather than a new compatibility layer or parallel snapshot path
