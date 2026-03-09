# E08TIMSCHREP-002: Deterministic RNG service

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — new module in worldwake-sim
**Deps**: E07 (action framework complete), archived E08TIMSCHREP-001 (`SystemId`), `Seed` type in worldwake-core

## Problem

All authoritative randomness must come from a single scheduler-owned service wrapping `ChaCha8Rng`. Without this, subsystems could accidentally use `thread_rng`, OS randomness, or wall-clock seeding, breaking determinism (Spec 9.2). The RNG state must also be fully serializable for save/load and replay.

## Assumption Reassessment (2026-03-09)

1. `Seed([u8; 32])` exists in `worldwake-core::ids` and already derives the trait set this ticket needs — confirmed
2. Archived E08TIMSCHREP-001 already landed `SystemId` and `SystemManifest` in `crates/worldwake-sim/src/system_manifest.rs`
3. No RNG service exists yet in `worldwake-sim` — confirmed
4. `Cargo.toml` for `worldwake-sim` does not yet depend on `rand_chacha` — confirmed
5. `rand_chacha 0.3.x` already exposes the exact `ChaCha8Rng` capabilities this ticket needs:
   - stable `Eq` / `PartialEq`
   - `Serialize` / `Deserialize` behind the `serde1` feature
   - `get_seed`, `get_stream`, `set_stream`, `get_word_pos`, `set_word_pos`
6. Because those capabilities already exist in the dependency, this ticket should not add a bespoke serialized-state wrapper or duplicate the seed inside a second field unless a concrete gap appears

## Architecture Check

1. Wrapping `ChaCha8Rng` behind a `DeterministicRng` struct keeps the simulation API small and prevents scheduler code from depending directly on `rand_chacha` details.
2. The wrapper should not duplicate the seed as a stored field. `ChaCha8Rng` already exposes its seed and exact stream position; storing both wrapper-level and inner RNG state creates unnecessary sync risk.
3. Substreams are still the right architectural move, but they should return another `DeterministicRng`, not a raw `ChaCha8Rng`. Returning the raw engine would punch through the abstraction this ticket is supposed to establish.
4. Serialization should use `ChaCha8Rng`'s supported serde surface with `serde1`, which already preserves the semver-covered abstract state `(seed, stream, word_pos)`. Manual state shims are unnecessary complexity here.
5. The dependency footprint should stay minimal. Add `rand_chacha` with `serde1`; do not add `rand` unless the implementation genuinely requires more than `rand_core` traits.

## What to Change

### 1. Add the minimal RNG dependency

Add `rand_chacha = { version = "0.3", features = ["serde1"] }` to `worldwake-sim/Cargo.toml`.

Do not add `rand` unless the implementation proves it is needed. Prefer `rand_chacha::rand_core::{RngCore, SeedableRng}` and small local helpers over widening the dependency surface.

### 2. New type: `DeterministicRng`

```rust
pub struct DeterministicRng {
    rng: ChaCha8Rng,
}
```

Methods:
- `new(seed: Seed) -> Self` — construct from master seed
- `next_u32(&mut self) -> u32`
- `next_u64(&mut self) -> u64`
- `next_range(&mut self, low: u32, high_exclusive: u32) -> u32`
- `substream(&self, tick: Tick, system_id: SystemId, seq: u64) -> DeterministicRng` — derive a deterministic child RNG without advancing the parent
- `seed(&self) -> Seed`

The struct must implement `Clone + Debug + Eq + PartialEq + Serialize + Deserialize`.

Design note:
- `seed()` should be derived from the inner RNG's stable state rather than stored separately.
- `substream()` should derive a fresh seed from `(seed(), tick, system_id, seq)` using deterministic `ChaCha8Rng` state controls, then return a wrapped child RNG.

### 3. Serialization strategy

Rely on `ChaCha8Rng`'s `serde1` implementation rather than building a parallel state format in this ticket. The round-trip requirement remains the same: serializing, deserializing, and continuing must reproduce the exact same sequence from the save point.

## Files to Touch

- `crates/worldwake-sim/Cargo.toml` (modify — add `rand_chacha` with `serde1`)
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
8. `seed()` returns the canonical seed for the stream represented by that wrapper
9. Existing suite: `cargo test -p worldwake-sim`

### Invariants

1. No `thread_rng`, `OsRng`, or wall-clock time anywhere in the module
2. All randomness derives from the master seed
3. Serialization captures complete state — no information loss on round-trip
4. The abstraction boundary stays intact: simulation code uses `DeterministicRng`, not raw `ChaCha8Rng`

## Test Plan

### New/Modified Tests

1. `crates/worldwake-sim/src/deterministic_rng.rs` (inline `#[cfg(test)]` module) — determinism, independence, serialization, substream isolation

### Commands

1. `cargo test -p worldwake-sim deterministic_rng`
2. `cargo clippy --workspace && cargo test --workspace`

## Outcome

- Completed: 2026-03-09
- Changed vs. original plan:
  - Added `crates/worldwake-sim/src/deterministic_rng.rs` with a `DeterministicRng` wrapper over `ChaCha8Rng`.
  - Added `rand_chacha` with the `serde1` feature to `crates/worldwake-sim/Cargo.toml`.
  - Re-exported `DeterministicRng` from `crates/worldwake-sim/src/lib.rs`.
  - Added focused unit coverage for determinism, exact serde/bincode continuation, range bounds, and substream isolation.
- Deviations from original plan:
  - Did not add the broader `rand` crate; `rand_chacha::rand_core` is sufficient and keeps the dependency surface smaller.
  - Did not build a manual RNG-state serialization wrapper; `ChaCha8Rng` already exposes a stable serde surface for the needed abstract state.
  - Did not expose raw `ChaCha8Rng` from `substream`; child streams remain wrapped as `DeterministicRng` to preserve the abstraction boundary.
- Verification:
  - `cargo test -p worldwake-sim deterministic_rng`
  - `cargo test -p worldwake-sim`
  - `cargo clippy --workspace`
  - `cargo test --workspace`
