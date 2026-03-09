# E02WORTOP-006: Topology Serialization Round-Trip and Stable Hash

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: None
**Deps**: E02WORTOP-005 (prototype builder produces a complete Topology)

## Problem

The topology must serialize/deserialize via bincode for save/load and replay, and produce a stable hash for snapshot comparisons. The spec requires: "Prototype builder yields a stable topology hash across repeated runs in the same build."

## Assumption Reassessment (2026-03-09)

1. `Topology` struct and all contained types (`Place`, `TravelEdge`, etc.) derive `Serialize + Deserialize` from prior tickets — dependency.
2. `canonical_bytes()` exists in `test_utils.rs` for deterministic serialization — confirmed.
3. `bincode` is already a dependency — confirmed in E01 tests.
4. The stable hash does not need to be cryptographic — a simple hash of canonical bytes (e.g., using `std::collections::hash_map::DefaultHasher` or a fixed algorithm) suffices for snapshot comparison within a build.

## Architecture Check

1. Add `Serialize + Deserialize` derives to `Topology` struct (if not already present from E02WORTOP-003).
2. Provide a `topology_hash(&self) -> u64` method that serializes to canonical bytes and hashes them. This is deterministic within a build because bincode + BTreeMap iteration is deterministic.
3. The hash is for testing/validation only — not for cross-build stability (bincode layout can change between versions, which is acceptable per spec: "same build").

## What to Change

### 1. Ensure `Topology` derives `Serialize + Deserialize`

All fields (`BTreeMap<EntityId, Place>`, etc.) are already serializable. Just add the derives.

### 2. Add `topology_hash()` method

```rust
impl Topology {
    /// Deterministic hash of the full topology for snapshot comparison.
    /// Stable within the same build (same bincode version + same data).
    pub fn topology_hash(&self) -> u64 {
        use std::hash::{Hash, Hasher};
        let bytes = bincode::serialize(self).expect("topology serialization must not fail");
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        bytes.hash(&mut hasher);
        hasher.finish()
    }
}
```

### 3. Round-trip test

Serialize a `Topology` (from builder), deserialize, re-serialize, and assert byte equality.

## Files to Touch

- `crates/worldwake-core/src/topology.rs` (modify — add derives to `Topology`, add `topology_hash()`)

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

1. Serialization uses only deterministic types (BTreeMap, Vec, etc.) — no HashMap.
2. `topology_hash()` is a pure function of topology state — no external dependencies.
3. Two `Topology` values with identical content produce identical hashes.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/topology.rs` — serialization round-trip, hash stability, query equivalence after round-trip.

### Commands

1. `cargo test -p worldwake-core topology_hash`
2. `cargo test -p worldwake-core serializ`
3. `cargo clippy --workspace && cargo test --workspace`
