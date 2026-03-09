# E02WORTOP-006: Topology Serialization Round-Trip and Stable Hash

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: None
**Deps**: E02WORTOP-005 (prototype builder produces a complete Topology)

## Problem

The topology must serialize/deserialize via bincode for save/load and replay, and produce a stable hash for snapshot comparisons. The spec requires: "Prototype builder yields a stable topology hash across repeated runs in the same build."

## Assumption Reassessment (2026-03-09)

1. `Place`, `TravelEdge`, and `Route` already derive `Serialize + Deserialize`, but `Topology` does not yet. This ticket is specifically about closing that gap.
2. `canonical_bytes()` exists in `crates/worldwake-core/src/test_utils.rs` for deterministic test-side serialization assertions — confirmed.
3. `bincode` is already a direct dependency of `worldwake-core` — confirmed.
4. The original ticket overfit the implementation by suggesting `DefaultHasher` and a panic-on-failure API. For long-term robustness, topology hashing should use an explicit fixed algorithm over serialized bytes and return `Result<u64, WorldError>` instead of panicking.

## Architecture Check

1. Add `Serialize + Deserialize` derives to `Topology`. This keeps topology aligned with the rest of the authoritative graph types and is cleaner than inventing a snapshot-only wrapper around the internal maps.
2. Provide a `topology_hash(&self) -> Result<u64, WorldError>` method on `Topology`. It should serialize the topology and hash those bytes with a small fixed algorithm implemented locally in `topology.rs`.
3. The hash remains a value-level topology fingerprint, not a persistence format. It is primarily used for determinism checks and snapshot comparison.
4. Although the spec only requires stability within the same build, using an explicit fixed hash is better architecture than `DefaultHasher`: it avoids depending on std implementation details and makes the determinism contract obvious in code.

## What to Change

### 1. Ensure `Topology` derives `Serialize + Deserialize`

All fields (`BTreeMap<EntityId, Place>`, etc.) are already serializable. Just add the derives.

### 2. Add fallible `topology_hash()` method with a fixed hash algorithm

```rust
impl Topology {
    /// Deterministic hash of the full topology for snapshot comparison.
    pub fn topology_hash(&self) -> Result<u64, WorldError> {
        let bytes = bincode::serialize(self)
            .map_err(|err| WorldError::SerializationError(err.to_string()))?;
        Ok(fixed_hash64(&bytes))
    }
}
```

Use a small explicit 64-bit hash implementation in-module rather than `DefaultHasher`.

### 3. Round-trip test

Serialize a `Topology` (from builder), deserialize, re-serialize, and assert byte equality.

## Files to Touch

- `crates/worldwake-core/src/topology.rs` (modify — add derives to `Topology`, add `topology_hash()`, add fixed hash helper and tests)

## Out of Scope

- Cross-build hash stability (different Rust/bincode versions may change layout — acceptable per spec).
- Save/load file format design — that's E08.
- Compression or versioned serialization format.
- Any hash used for security purposes.

## Acceptance Criteria

### Tests That Must Pass

1. Topology serialization round-trip: `serialize → deserialize → re-serialize` produces identical bytes — spec test.
2. Prototype builder yields a stable topology hash across repeated calls — spec test.
3. Round-tripped topology has identical `place_count()` and `edge_count()`.
4. Round-tripped topology answers the same `shortest_path()` queries.
5. Round-tripped topology answers the same `is_reachable()` queries.
6. Existing suite: `cargo test -p worldwake-core`.

### Invariants

1. Serialization uses only deterministic types (`BTreeMap`, `Vec`, etc.) — no `HashMap`.
2. `topology_hash()` is a pure function of topology state and returns an error instead of panicking on serialization failure.
3. Two `Topology` values with identical content produce identical hashes.
4. The hash algorithm is explicit and stable in code, not delegated to `DefaultHasher`.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/topology.rs` — topology serialization round-trip, stable hash repeatability, equal hashes for equal topology values, and query equivalence after round-trip.

### Commands

1. `cargo test -p worldwake-core topology_hash`
2. `cargo test -p worldwake-core topology_roundtrip`
3. `cargo test -p worldwake-core`
4. `cargo clippy --workspace`
5. `cargo test --workspace`

## Outcome

- Completion date: 2026-03-09
- What actually changed:
  - Corrected the ticket first to match the current codebase: `Topology` was the only missing serde type in the topology stack, while `canonical_bytes()` already existed as a test utility.
  - Added `Serialize + Deserialize` derives to `Topology` in `crates/worldwake-core/src/topology.rs`.
  - Added `Topology::topology_hash(&self) -> Result<u64, WorldError>` using an explicit fixed 64-bit FNV-1a hash over serialized bytes.
  - Added focused topology tests for serialization round-trip, hash repeatability across identical values, and query/count preservation after round-trip.
- Deviations from original plan:
  - The original ticket proposed `DefaultHasher` and a panic-on-failure API. The implementation intentionally uses a fixed in-module hash and returns `Result` so serialization failures stay inside the engine error model.
  - The resulting hash is stable by construction beyond the spec's minimum \"same build\" requirement, but it is still defined as a topology fingerprint rather than a persisted file format contract.
- Verification results:
  - `cargo test -p worldwake-core topology_` passed.
  - `cargo test -p worldwake-core` passed.
  - `cargo fmt --all --check` passed.
  - `cargo clippy --workspace` passed.
  - `cargo test --workspace` passed.
