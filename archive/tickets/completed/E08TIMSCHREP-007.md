# E08TIMSCHREP-007: Canonical hashing helpers

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — foundational hashing API in `worldwake-core`, with downstream use from `worldwake-sim`
**Deps**: E07 complete; `World` and `EventLog` are already serializable in `worldwake-core`
**Spec Ref**: `specs/E08-time-scheduler-replay.corrected.md` ("Canonical Hashing", "Replay Recording", "Replay Execution", "Save / Load")

## Problem

Replay verification and save/load integrity require canonical hashes of authoritative state. "Canonical" means derived from stable serialized bytes, not `Debug` output, pointer identity, or ad hoc per-type hash code. Without a single canonical hashing path, replay checkpoints are not trustworthy and save/load equality checks will drift into duplicated logic.

## Assumption Reassessment (2026-03-09)

1. `World` derives `Serialize + Deserialize` in `worldwake-core` — confirmed.
2. `EventLog` derives `Serialize + Deserialize` in `worldwake-core` — confirmed.
3. `bincode` is already a regular dependency of `worldwake-core` — confirmed.
4. The ticket's cited spec file `specs/E08TIMSCHREP-007.md` does not exist. E08 is currently specified in the consolidated epic file `specs/E08-time-scheduler-replay.corrected.md`.
5. Canonical serialization infrastructure already partially exists in `worldwake-core::test_utils::canonical_bytes` — confirmed.
6. An ad hoc hash already exists: `Topology::topology_hash() -> Result<u64, WorldError>` in `worldwake-core`. This means the "no canonical hashing exists yet" assumption was wrong; the real problem is that hashing is inconsistent, incomplete, and partly stranded in one type-specific API.
7. Deterministic-state policy is already enforced in `worldwake-core` (`BTreeMap`/`BTreeSet` policy tests, no `HashMap`/`HashSet` in authoritative state) — confirmed.
8. `worldwake-sim` already has deterministic scheduler/input/RNG infrastructure and tests. This ticket should provide the shared hashing foundation they depend on, not duplicate that architecture.

## Scope Correction

This ticket should not introduce a sim-only hashing subsystem. The canonical hash API belongs in `worldwake-core` because:

1. The primary hashed roots in scope now are `World` and `EventLog`, both defined in `worldwake-core`.
2. The existing canonical-byte helper and the existing ad hoc topology hash both already live in `worldwake-core`.
3. `SimulationState` hashing in E08TIMSCHREP-010 should reuse the same generic helper from `worldwake-core`, not define a second hashing layer in `worldwake-sim`.

## Architecture Reassessment

### Preferred design

Create a production `canonical` module in `worldwake-core` with:

- `StateHash([u8; 32])`
- `canonical_bytes<T: Serialize>(&T) -> Result<Vec<u8>, CanonicalError>`
- `hash_bytes(&[u8]) -> StateHash`
- `hash_serializable<T: Serialize>(&T) -> Result<StateHash, CanonicalError>`
- `hash_world(&World) -> Result<StateHash, CanonicalError>`
- `hash_event_log(&EventLog) -> Result<StateHash, CanonicalError>`

`worldwake-sim` should consume these helpers directly for replay/save-load work.

### Hash algorithm choice

Use `blake3` as the fixed canonical hash algorithm.

Rationale:

1. Deterministic and stable.
2. Fast enough for frequent checkpointing.
3. 256-bit output avoids committing Phase 1 determinism checks to a narrow 64-bit hash.
4. Cleaner than preserving the current one-off FNV-style topology hash.

### Existing architecture to retire

`Topology::topology_hash()` is a special-case hash API with a different output type and algorithm than the rest of the planned replay stack. That is the wrong long-term architecture.

This ticket should replace that duplication with the shared canonical hashing path rather than preserve both APIs.

## What to Change

### 1. Add `blake3` to `worldwake-core`

Add `blake3 = "1"` to `crates/worldwake-core/Cargo.toml`.

Do not add runtime `bincode` or hashing dependencies to `worldwake-sim` for this ticket.

### 2. Add `StateHash` and canonical hashing module in `worldwake-core`

Add a new production module, for example `crates/worldwake-core/src/canonical.rs`, containing:

```rust
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Serialize, Deserialize)]
pub struct StateHash(pub [u8; 32]);
```

With `Display` implemented as lowercase hex.

Also add a small error type for serialization failures so the API does not leak raw `bincode` internals as the public contract.

### 3. Promote canonical bytes out of test-only infrastructure

Move the canonical-bytes helper into the new production module. Update tests to use the production helper rather than `worldwake-core::test_utils::canonical_bytes`.

`test_utils` may keep deterministic test seed helpers if still useful, but canonical serialization itself should no longer live only in test infrastructure.

### 4. Remove the ad hoc topology hash path

Replace `Topology::topology_hash()` and its private fixed-hash implementation with the shared canonical hashing API.

This is in scope because maintaining both the old type-specific hash and the new canonical hash would create two competing architectures for the same concern.

## Files to Touch

- `crates/worldwake-core/Cargo.toml` (modify — add `blake3`)
- `crates/worldwake-core/src/canonical.rs` (new)
- `crates/worldwake-core/src/lib.rs` (modify — export canonical hashing API)
- `crates/worldwake-core/src/topology.rs` (modify — remove one-off topology hash usage/tests, switch to shared canonical hash helper)
- `crates/worldwake-core/src/test_utils.rs` (modify — remove or shrink test-only canonical serialization helper)
- `crates/worldwake-core/tests/policy.rs` (modify — use production canonical helper)

## Out of Scope

- Defining `SimulationState` itself (E08TIMSCHREP-010)
- Replay checkpoint recording structure (E08TIMSCHREP-008)
- Replay execution/comparison logic (E08TIMSCHREP-009)
- Save/load persistence format (E08TIMSCHREP-011)
- Incremental hashing, caching, or Merkle-style structures

## Acceptance Criteria

### Tests That Must Pass

1. `StateHash` satisfies `Copy + Clone + Eq + Ord + Hash + Debug + Display + Serialize + Deserialize`
2. `canonical_bytes` returns identical bytes for identical values
3. `hash_serializable` returns identical hashes for identical values
4. `hash_serializable` changes when serialized content changes
5. `hash_world` is stable for identical `World` values
6. `hash_world` changes when the world changes
7. `hash_event_log` is stable for identical `EventLog` values
8. `hash_event_log` changes when an event is appended
9. `StateHash` round-trips through bincode
10. Existing `cargo test -p worldwake-core`
11. Existing `cargo test -p worldwake-sim`

### Invariants

1. Canonical hashes are derived from canonical serialized bytes, never from `Debug` output or per-type ad hoc logic.
2. Authoritative hashed state continues to rely only on deterministic data structures.
3. There is one shared canonical hashing architecture for Phase 1 state, not separate core and sim schemes.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/canonical.rs` — `StateHash` traits, stability, sensitivity, round-trip
2. `crates/worldwake-core/src/topology.rs` — update existing hash-related tests to use the shared canonical helper
3. `crates/worldwake-core/tests/policy.rs` — canonical-bytes stability test should exercise the production helper

### Commands

1. `cargo test -p worldwake-core canonical`
2. `cargo test -p worldwake-core topology`
3. `cargo test -p worldwake-sim`
4. `cargo clippy --workspace`
5. `cargo test --workspace`

## Outcome Notes For Later Archival

If implementation reveals that a broader rename is cleaner than `canonical`, capture that in the archive outcome section, but keep the architectural decision intact: canonical hashing is core infrastructure, and the one-off topology hash should not survive beside it.

## Outcome

Actual changes vs. original plan:

1. The implementation was moved into `worldwake-core`, not `worldwake-sim`, because the hashed authoritative roots (`World`, `EventLog`) and the pre-existing canonical-byte/topology-hash code already lived in core.
2. A shared production canonical hashing API was added in `worldwake-core` with `StateHash`, canonical byte serialization, BLAKE3 hashing, and typed helpers for `World` and `EventLog`.
3. The old ad hoc `Topology::topology_hash()` path was removed in favor of the shared canonical hashing path.
4. Existing tests were updated to exercise the production canonical helper rather than a test-only canonical serialization helper.
