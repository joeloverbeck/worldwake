# E08TIMSCHREP-007: Canonical hashing helpers

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — new module in worldwake-sim, possible trait additions in worldwake-core
**Deps**: E07 (action framework complete), worldwake-core World and EventLog already serializable

## Problem

Replay verification and save/load integrity require canonical hashes of world state, event log, and full simulation state. "Canonical" means derived from stable serialized bytes (bincode), not `Debug` output or memory layout. Without stable hashing, replay checkpoint comparisons are meaningless.

## Assumption Reassessment (2026-03-09)

1. `World` derives `Serialize` — confirmed (serde derive on struct)
2. `EventLog` derives `Serialize` — confirmed in `event_log.rs`
3. `bincode` is available as a dev-dependency in worldwake-sim and as a dependency in worldwake-core — confirmed
4. No canonical hashing exists yet — confirmed
5. `BTreeMap`-based storage ensures deterministic serialization order — confirmed by deterministic data policy

## Architecture Check

1. Use `bincode::serialize` for canonical bytes, then hash with a fast non-cryptographic hash (e.g., `blake3` or a simple `sha2::Sha256` — or even a simpler approach: serialize to bytes and hash with a fixed algorithm)
2. For prototype scope, `std` doesn't provide a suitable hasher. Options:
   - Add `sha2` dependency (common, stable, no_std compatible)
   - Use `bincode` bytes length + simple checksum (too weak)
   - Recommendation: use `blake3` (fast, single dependency, 256-bit output, deterministic)
3. The hash output is a `[u8; 32]` wrapped in a `StateHash` newtype
4. Hashing must ignore transient caches — since we only hash serializable fields and our types don't have `#[serde(skip)]` transient fields, this is satisfied by construction

## What to Change

### 1. Add `blake3` dependency

Add `blake3 = "1"` to `worldwake-sim/Cargo.toml`.

### 2. New type: `StateHash`

```rust
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Serialize, Deserialize)]
pub struct StateHash(pub [u8; 32]);
```

With `Display` impl showing hex.

### 3. New module: `canonical_hash`

Free functions:
- `hash_world(world: &World) -> StateHash` — bincode serialize World, then blake3 hash
- `hash_event_log(log: &EventLog) -> StateHash` — bincode serialize EventLog, then blake3 hash
- `hash_bytes(data: &[u8]) -> StateHash` — low-level helper

A generic helper:
- `hash_serializable<T: Serialize>(value: &T) -> StateHash` — bincode serialize, then blake3

### 4. Add `bincode` as a regular dependency (not just dev)

Since canonical hashing is used at runtime (not just tests), `bincode` must be a regular dependency of worldwake-sim.

## Files to Touch

- `crates/worldwake-sim/Cargo.toml` (modify — add `blake3`, promote `bincode` to regular dep)
- `crates/worldwake-sim/src/canonical_hash.rs` (new)
- `crates/worldwake-sim/src/lib.rs` (modify — add module + re-exports)

## Out of Scope

- Hashing `SimulationState` (depends on E08TIMSCHREP-010 defining that struct)
- Per-tick checkpoint recording (E08TIMSCHREP-008)
- Replay hash comparison logic (E08TIMSCHREP-009)
- Optimizations like incremental hashing or caching

## Acceptance Criteria

### Tests That Must Pass

1. `StateHash` satisfies `Copy + Clone + Eq + Ord + Hash + Debug + Display + Serialize + Deserialize`
2. `hash_serializable` on identical values produces identical hashes
3. `hash_serializable` on different values produces different hashes (basic collision resistance)
4. `hash_world` produces stable output — same World state hashed twice yields same `StateHash`
5. `hash_event_log` produces stable output — same EventLog hashed twice yields same `StateHash`
6. `hash_world` changes when a component is added to the World
7. `hash_event_log` changes when an event is appended
8. `StateHash` bincode round-trip
9. Existing suite: `cargo test -p worldwake-sim`

### Invariants

1. Hashes are derived from canonical serialized bytes, not `Debug` output (Spec requirement)
2. Canonical serialization order is stable (guaranteed by `BTreeMap`-based storage + `bincode`)
3. No `HashMap`/`HashSet` in hashed state

## Test Plan

### New/Modified Tests

1. `crates/worldwake-sim/src/canonical_hash.rs` (inline `#[cfg(test)]`) — stability, sensitivity, round-trip

### Commands

1. `cargo test -p worldwake-sim canonical_hash`
2. `cargo clippy --workspace && cargo test --workspace`
