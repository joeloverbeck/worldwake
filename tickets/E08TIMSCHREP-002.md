# E08TIMSCHREP-002: Deterministic RNG service

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — new module in worldwake-sim
**Deps**: E07 (action framework complete), `Seed` type in worldwake-core

## Problem

All authoritative randomness must come from a single scheduler-owned service wrapping `ChaCha8Rng`. Without this, subsystems could accidentally use `thread_rng`, OS randomness, or wall-clock seeding, breaking determinism (Spec 9.2). The RNG state must also be fully serializable for save/load and replay.

## Assumption Reassessment (2026-03-09)

1. `Seed([u8; 32])` exists in `worldwake-core::ids` — confirmed
2. `rand_chacha` is listed as an allowed dependency — confirmed in CLAUDE.md external deps
3. No RNG service exists yet in worldwake-sim — confirmed
4. `Cargo.toml` for worldwake-sim does not yet depend on `rand` or `rand_chacha` — confirmed

## Architecture Check

1. Wrapping `ChaCha8Rng` behind a `DeterministicRng` struct keeps the API small and prevents accidental direct access to the underlying RNG
2. Named substreams derived from `(master_seed, tick, subsystem_id, sequence_no)` reduce coupling between systems — if system A adds a random call, it won't shift system B's sequence
3. Serialization captures the full internal state, not just the seed — this is critical for mid-simulation save/load

## What to Change

### 1. Add `rand` and `rand_chacha` dependencies

Add `rand = "0.8"` and `rand_chacha = "0.3"` to `worldwake-sim/Cargo.toml`. Also add `serde` feature for `rand_chacha` if available, otherwise implement manual serialization of `ChaCha8Rng` state.

### 2. New type: `DeterministicRng`

```rust
pub struct DeterministicRng {
    master_seed: Seed,
    rng: ChaCha8Rng,  // or serializable wrapper
}
```

Methods:
- `new(seed: Seed) -> Self` — construct from master seed
- `next_u32(&mut self) -> u32`
- `next_u64(&mut self) -> u64`
- `next_range(&mut self, low: u32, high_exclusive: u32) -> u32`
- `substream(&self, tick: Tick, system_id: SystemId, seq: u64) -> ChaCha8Rng` — derive a deterministic child RNG without advancing the master
- `master_seed(&self) -> Seed`

The struct must implement `Clone + Debug + Eq + PartialEq + Serialize + Deserialize`.

Note: `ChaCha8Rng` does not natively implement `Eq`/`PartialEq`/`Serialize`. Serialize/deserialize the 256-bit state + stream position + counter manually via `ChaCha8Rng::get_seed()` and position methods if needed, or store the seed + number-of-calls-since-creation as the canonical serializable state.

### 3. Serialization strategy

Store the RNG state as `(Seed, u128)` where `u128` is the stream position / word position. This must round-trip exactly — calling `next_u32` N times on a restored RNG must produce the same sequence as calling it N times on the original after the save point.

## Files to Touch

- `crates/worldwake-sim/Cargo.toml` (modify — add `rand`, `rand_chacha`)
- `crates/worldwake-sim/src/deterministic_rng.rs` (new)
- `crates/worldwake-sim/src/lib.rs` (modify — add module + re-export)

## Out of Scope

- Integrating the RNG into the scheduler tick loop (that's E08TIMSCHREP-006)
- Using the RNG in any game system (Phase 2 epics)
- Substream caching or pooling optimizations
- The `SystemId` type itself (E08TIMSCHREP-001)

## Acceptance Criteria

### Tests That Must Pass

1. `DeterministicRng::new(seed)` produces deterministic output — same seed yields same first 100 values
2. Two `DeterministicRng` with different seeds produce different output
3. `substream(tick, system_id, seq)` is deterministic — same args yield same child RNG
4. `substream` with different `(tick, system_id, seq)` tuples produces different sequences
5. Substream creation does NOT advance the master RNG state
6. Full bincode round-trip: serialize, deserialize, then subsequent calls produce identical output to a non-serialized continuation
7. `DeterministicRng` satisfies `Clone + Debug + Eq + PartialEq + Serialize + Deserialize`
8. Existing suite: `cargo test -p worldwake-sim`

### Invariants

1. No `thread_rng`, `OsRng`, or wall-clock time anywhere in the module
2. All randomness derives from the master seed
3. Serialization captures complete state — no information loss on round-trip

## Test Plan

### New/Modified Tests

1. `crates/worldwake-sim/src/deterministic_rng.rs` (inline `#[cfg(test)]` module) — determinism, independence, serialization, substream isolation

### Commands

1. `cargo test -p worldwake-sim deterministic_rng`
2. `cargo clippy --workspace && cargo test --workspace`
