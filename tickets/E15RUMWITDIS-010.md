# E15RUMWITDIS-010: Add belief_confidence() Derivation Helper

**Status**: PENDING
**Priority**: LOW
**Effort**: Small
**Engine Changes**: Yes — pure function in worldwake-core
**Deps**: None (uses existing PerceptionSource and Permille types)

## Problem

The spec calls for a pure `belief_confidence()` function that derives a confidence value from `PerceptionSource` variant and staleness (current_tick - observed_tick). This is a derived read-model, not authoritative state (Principle 3 compliance). The AI planner and downstream systems need this to compare belief reliability when ranking goals or deciding whether to act on uncertain information.

## Assumption Reassessment (2026-03-14)

1. `PerceptionSource` in `crates/worldwake-core/src/belief.rs` — confirmed. Variants: DirectObservation, Report { from, chain_len }, Rumor { chain_len }, Inference.
2. `Permille` in `crates/worldwake-core/src/numerics.rs` — confirmed. Return type for the confidence function.
3. `Tick` in `crates/worldwake-core/src/ids.rs` — u64 newtype. Staleness = current_tick.0 - observed_tick.0.
4. No existing confidence derivation function in the codebase — this is new.

## Architecture Check

1. Pure function with no side effects — takes immutable references, returns Permille.
2. Placed in `crates/worldwake-core/src/belief.rs` alongside the types it operates on.
3. Confidence ordering must satisfy: DirectObservation > Report(1) > Rumor(1) > deeper chains. Staleness reduces confidence.
4. Exact formula is implementation detail — the spec does not prescribe specific values, only ordering.
5. No backwards-compatibility shims.

## What to Change

### 1. Add `belief_confidence()` function

In `crates/worldwake-core/src/belief.rs`:

```rust
/// Derives confidence from perception source and staleness.
/// This is a read-model derivation — never stored as authoritative state.
pub fn belief_confidence(source: &PerceptionSource, staleness_ticks: u64) -> Permille { ... }
```

Confidence ordering:
- `DirectObservation` → highest base (e.g. 950)
- `Report { chain_len: 1 }` → moderate base (e.g. 750)
- `Rumor { chain_len: 1 }` → lower base (e.g. 500)
- Deeper chains reduce base further
- `Inference` → low base (e.g. 400)
- Staleness applies a decay: each tick of staleness reduces confidence (exact formula TBD during implementation, must be deterministic, must use integer math only)
- Floor at Permille(0)

### 2. Export function

In `crates/worldwake-core/src/lib.rs`, add `belief_confidence` to public exports.

## Files to Touch

- `crates/worldwake-core/src/belief.rs` (modify — add belief_confidence function)
- `crates/worldwake-core/src/lib.rs` (modify — export belief_confidence)

## Out of Scope

- Storing confidence as authoritative state (forbidden by Principle 3)
- AI planner integration with confidence (future work)
- Modifying any existing belief update paths
- Tell action or mismatch detection

## Acceptance Criteria

### Tests That Must Pass

1. `belief_confidence(DirectObservation, 0)` returns highest confidence
2. `belief_confidence(Report { chain_len: 1 }, 0) < belief_confidence(DirectObservation, 0)`
3. `belief_confidence(Rumor { chain_len: 1 }, 0) < belief_confidence(Report { chain_len: 1 }, 0)`
4. `belief_confidence(Rumor { chain_len: 3 }, 0) < belief_confidence(Rumor { chain_len: 1 }, 0)`
5. Staleness reduces confidence: `belief_confidence(DirectObservation, 10) < belief_confidence(DirectObservation, 0)`
6. Confidence never exceeds Permille(1000)
7. Confidence never panics (floor at Permille(0))
8. Function is deterministic: same inputs → same output
9. No floating point math used
10. Existing suite: `cargo test --workspace`
11. `cargo clippy --workspace`

### Invariants

1. belief_confidence is a pure function — no side effects, no state mutation
2. Never stored as authoritative state — always derived at query time
3. Uses integer-only arithmetic (determinism requirement)
4. Ordering: DirectObservation > Report > Rumor > deeper chains (at same staleness)

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/belief.rs` — unit tests for confidence ordering, staleness decay, boundary conditions (zero staleness, very high staleness, deep chains)

### Commands

1. `cargo test -p worldwake-core`
2. `cargo clippy --workspace`
3. `cargo test --workspace`
