# E08TIMSCHREP-011: Versioned save/load persistence

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — new module in worldwake-sim
**Deps**: E08TIMSCHREP-010 (SimulationState)

## Problem

The simulation must support saving to and loading from disk with versioned binary format. Save/load is a semantic round-trip — not just "it deserializes" but "the simulation continues identically." Old or mismatched format versions must fail cleanly, not silently corrupt state (Spec 9.19).

## Assumption Reassessment (2026-03-09)

1. `SimulationState` from E08TIMSCHREP-010 is the single serializable root
2. `bincode` is available for binary serialization — confirmed
3. `serde` derives on all component types — confirmed throughout codebase
4. No save/load exists yet — confirmed

## Architecture Check

1. Save file format: `[magic bytes][format version u32][bincode payload]`
2. Magic bytes: fixed 4-byte identifier (e.g., `b"WWAK"`) to detect non-save files
3. Format version: `u32` starting at `1` — incremented on breaking schema changes
4. Loading checks magic + version before deserializing — mismatches produce clear errors, not panics
5. No compression for prototype scope — bincode is already compact

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
1. Serialize `SimulationState` with bincode
2. Write magic bytes + version u32 + serialized bytes to file

`load`:
1. Read file
2. Verify magic bytes
3. Read and check version
4. Deserialize remaining bytes as `SimulationState`

### 4. New function: `save_to_bytes` / `load_from_bytes`

In-memory variants for testing without filesystem:
```rust
pub fn save_to_bytes(state: &SimulationState) -> Result<Vec<u8>, SaveError>
pub fn load_from_bytes(bytes: &[u8]) -> Result<SimulationState, SaveError>
```

## Files to Touch

- `crates/worldwake-sim/src/save_load.rs` (new)
- `crates/worldwake-sim/src/lib.rs` (modify — add module + re-exports)

## Out of Scope

- Compression (not needed for prototype)
- Migration between format versions (deferred — version 1 only for now)
- Autosave or periodic save logic
- Save file browser / listing
- CLI integration for save/load (E21)

## Acceptance Criteria

### Tests That Must Pass

1. **T09 — Save/load round-trip**: save a populated `SimulationState`, load it back, verify exact equality
2. Save/load round-trip with active actions, queued inputs, replay checkpoints, and RNG state
3. After load, continuing the simulation produces identical results to uninterrupted execution (semantic round-trip)
4. `load` rejects files with wrong magic bytes — returns `SaveError::InvalidMagic`
5. `load` rejects files with wrong version — returns `SaveError::UnsupportedVersion`
6. `load` rejects truncated files — returns `SaveError::Deserialization`
7. `save_to_bytes` / `load_from_bytes` round-trip matches file-based round-trip
8. Saved file includes: world, events, scheduler, active actions, pending reservations, control state, RNG state, replay state (verify by checking loaded state fields are non-default)
9. Existing suite: `cargo test -p worldwake-sim`

### Invariants

1. Save file format version is stored explicitly (Spec requirement)
2. Old or mismatched versions fail cleanly (Spec requirement)
3. Save/load is a semantic round-trip — simulation continues identically (Spec 9.19)
4. No silent data loss on round-trip

## Test Plan

### New/Modified Tests

1. `crates/worldwake-sim/src/save_load.rs` (inline `#[cfg(test)]`) — round-trip, error cases, semantic continuation test

### Commands

1. `cargo test -p worldwake-sim save_load`
2. `cargo clippy --workspace && cargo test --workspace`
