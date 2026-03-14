# E15RUMWITDIS-010: Add belief_confidence() Derivation Helper

**Status**: ✅ COMPLETED
**Priority**: LOW
**Effort**: Small
**Engine Changes**: Yes — pure function in worldwake-core
**Deps**: None (uses existing PerceptionSource and Permille types)

## Problem

The spec calls for a pure `belief_confidence()` function that derives a confidence value from `PerceptionSource` variant and staleness (current_tick - observed_tick). This is a derived read-model, not authoritative state (Principle 3 compliance). The AI planner and downstream systems need this to compare belief reliability when ranking goals or deciding whether to act on uncertain information.

## Assumption Reassessment (2026-03-14)

1. `PerceptionSource` in `crates/worldwake-core/src/belief.rs` exists as assumed. Variants remain `DirectObservation`, `Report { from, chain_len }`, `Rumor { chain_len }`, and `Inference`.
2. `Permille` in `crates/worldwake-core/src/numerics.rs` exists as assumed and is still the correct return type for a derived confidence helper.
3. `Tick` in `crates/worldwake-core/src/ids.rs` is still the logical time source for staleness derivation. `current_tick.0.saturating_sub(observed_tick.0)` is the correct non-panicking staleness model.
4. No shared `belief_confidence()` helper currently exists in the codebase.
5. The broader E15 assumptions in this ticket are stale: the codebase already includes `TellProfile`, `MismatchKind`, `EventTag::Discovery`, `SocialObservationKind::WitnessedTelling`, `ActionDomain::Social`, `crates/worldwake-systems/src/tell_actions.rs`, and mismatch emission in `crates/worldwake-systems/src/perception.rs`.
6. The current test surface is much broader than the ticket assumed. `cargo test -p worldwake-core` already passes, and E15 tell/discovery behavior is already covered in `worldwake-systems` unit tests.

## Architecture Check

1. Pure function with no side effects — takes immutable references, returns Permille.
2. Placed in `crates/worldwake-core/src/belief.rs` alongside the types it operates on.
3. Confidence ordering must satisfy: `DirectObservation > Report(chain_len 1) > Rumor(chain_len 1) > deeper chains`, with `Inference` below direct observation and staleness monotonically reducing confidence.
4. Exact numeric values are an implementation detail, but the helper must encode a clear, centralized policy so downstream consumers do not invent divergent confidence ladders.
5. No backwards-compatibility shims.

## Scope Correction

This ticket is narrower than originally written:

- In scope:
  - Add `belief_confidence()` in `crates/worldwake-core/src/belief.rs`
  - Export it from `crates/worldwake-core/src/lib.rs`
  - Add focused `worldwake-core` tests for ordering, monotonic staleness decay, and saturation behavior
- Out of scope because already implemented elsewhere:
  - `TellProfile`
  - tell action registration/handler work
  - discovery/mismatch event plumbing
  - `EventTag::Social` / `EventTag::Discovery`
  - `SocialObservationKind::WitnessedTelling`
  - `ActionDomain::Social`
- Out of scope because it would be premature architectural coupling:
  - AI planner integration before a concrete call site requires confidence scoring
  - storing confidence in authoritative state

## What to Change

### 1. Add `belief_confidence()` function

In `crates/worldwake-core/src/belief.rs`:

```rust
/// Derives confidence from perception source and staleness.
/// This is a read-model derivation — never stored as authoritative state.
pub fn belief_confidence(source: &PerceptionSource, staleness_ticks: u64) -> Permille { ... }
```

Confidence ordering:
- `DirectObservation` → highest base
- `Report { chain_len: 1 }` → lower than direct observation
- `Rumor { chain_len: 1 }` → lower than report
- Deeper report/rumor chains reduce base further
- `Inference` → lower-confidence derived knowledge than direct observation, but exact placement relative to shallow report/rumor should follow a clear single policy in code and tests
- Staleness applies deterministic integer-only decay
- Floor at `Permille(0)`

### 2. Export function

In `crates/worldwake-core/src/lib.rs`, add `belief_confidence` to public exports.

## Files to Touch

- `crates/worldwake-core/src/belief.rs` (modify — add `belief_confidence()` and unit tests)
- `crates/worldwake-core/src/lib.rs` (modify — export `belief_confidence`)

## Out of Scope

- Storing confidence as authoritative state (forbidden by Principle 3)
- AI planner integration with confidence (future work, only when a concrete planner/system use needs it)
- Modifying existing belief update paths
- tell action or mismatch detection behavior

## Acceptance Criteria

### Tests That Must Pass

1. `belief_confidence(DirectObservation, 0)` returns the highest confidence among the source variants at zero staleness
2. `belief_confidence(Report { chain_len: 1 }, 0) < belief_confidence(DirectObservation, 0)`
3. `belief_confidence(Rumor { chain_len: 1 }, 0) < belief_confidence(Report { chain_len: 1 }, 0)`
4. Deeper chains lower confidence for both report/rumor-derived beliefs
5. Staleness monotonically reduces confidence for a fixed source
6. Confidence never exceeds `Permille(1000)`
7. Confidence floors at `Permille(0)` without panicking, including very large staleness values
8. Function is deterministic: same inputs yield same output
9. No floating point math used
10. Relevant narrow suite: `cargo test -p worldwake-core`
11. Broader verification: `cargo clippy --workspace`
12. Broader verification: `cargo test --workspace`

### Invariants

1. belief_confidence is a pure function — no side effects, no state mutation
2. Never stored as authoritative state — always derived at query time
3. Uses integer-only arithmetic (determinism requirement)
4. Ordering remains derived from source provenance and age, never a stored authority field
5. Confidence policy is centralized in one helper so future planner/system callers do not duplicate ad hoc formulas

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/belief.rs` — unit tests for confidence ordering, monotonic staleness decay, boundary conditions (zero staleness, very high staleness, deep chains), and deterministic saturation behavior

### Commands

1. `cargo test -p worldwake-core`
2. `cargo clippy --workspace`
3. `cargo test --workspace`

## Outcome

- Completed: 2026-03-14
- What actually changed:
  - Added `belief_confidence()` to `crates/worldwake-core/src/belief.rs`
  - Exported `belief_confidence` from `crates/worldwake-core/src/lib.rs`
  - Added focused `worldwake-core` unit tests covering provenance ordering, deeper-chain penalties, monotonic staleness decay, zero-floor saturation, and determinism
- Deviations from original plan:
  - Before implementation, the ticket was corrected to reflect that most of the originally referenced E15 architecture was already implemented and tested elsewhere
  - Scope was intentionally narrowed to the missing derived helper rather than revisiting already-correct tell/discovery architecture
- Verification results:
  - `cargo test -p worldwake-core` passed
  - `cargo clippy --workspace` passed
  - `cargo test --workspace` passed
