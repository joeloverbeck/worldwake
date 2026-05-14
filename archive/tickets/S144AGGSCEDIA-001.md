# S144AGGSCEDIA-001: PercentileBucket core helper

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: None
**Deps**: None — foundation ticket (S144 D2)

## Problem

S144's `ScenarioDiagnosticsReport` needed deterministic integer percentile distributions (plan depth, repair budget consumed, queue wait ticks, search expansions). Before this ticket, there was no shared percentile helper in the workspace; without one, each metric category would re-implement percentile math, risking determinism drift and float introduction.

## Assumption Reassessment (2026-05-14)

1. Before this ticket, `crates/worldwake-core/src/percentile.rs` did not exist; no `PercentileBucket` type existed anywhere in the workspace. This was a net-new type with no existing focused/unit, runtime trace, or golden/E2E coverage.
2. S144 spec D2 (`archive/specs/S144-aggregate-scenario-diagnostics.md`) specifies the exact struct shape: `n, min, p50, p95, p99, max, mean` (all `u64`) plus `from_sorted(&[u64]) -> Self`. `worldwake-core`'s dependency set is `serde, bincode, blake3`; the `Serialize`/`Deserialize` derives `PercentileBucket` needs are already available.
3. Shared abstraction boundary: `PercentileBucket` is consumed by `worldwake-ai`'s `scenario_diagnostics` module (ticket 004) as a struct field type. The data contract under audit is the `from_sorted` determinism guarantee — integer-only math, no floats, `mean = sum / n` integer division.

## Architecture Check

1. A single shared helper in `worldwake-core` (the dependency root) lets every metric category in `ScenarioDiagnosticsReport` reuse identical percentile math, eliminating per-category drift. Integer-only computation over a sorted slice keeps the result deterministic and replay-stable (CLAUDE.md Determinism invariant: no floats).
2. No backwards-compatibility aliasing/shims — this is net-new. `from_sorted` takes a borrowed slice; the caller owns sorting, keeping the helper allocation-free.

## Verified Layers

1. `from_sorted` determinism (same input slice → identical bucket) -> focused unit test in `percentile.rs`.
2. Percentile-index correctness (p50/p95/p99 land on the right elements for known inputs) -> focused unit test in `percentile.rs`.
3. Single-layer ticket: this is a pure core helper with no decision-trace, action-trace, or event-log surface — additional layer mapping is not applicable.

## Landed Changes

### 1. `PercentileBucket` type

Added `crates/worldwake-core/src/percentile.rs` defining `pub struct PercentileBucket { n, min, p50, p95, p99, max, mean: u64 }` with derives `Clone, Debug, Eq, PartialEq, Serialize, Deserialize`. Implemented `pub fn from_sorted(values: &[u64]) -> Self` using integer-only percentile indexing over the already-sorted slice; `mean = sum / n` with integer division; the empty-slice case yields an all-zero bucket with `n == 0`.

### 2. Module registration

Added `pub mod percentile;` to `crates/worldwake-core/src/lib.rs` and re-exported `PercentileBucket`.

## Landed Files

- `crates/worldwake-core/src/percentile.rs` (added)
- `crates/worldwake-core/src/lib.rs` (modify — module declaration + re-export)

## Out of Scope

- Floating-point percentile interpolation — S144 mandates integer-only math.
- Configurable percentile sets (the `--diagnostics-percentiles` CLI override is ticket 006's scope).
- Any consumer wiring — `ScenarioDiagnosticsReport` field usage is ticket 004.

## Acceptance Result

### Tests Passed

1. `from_sorted` over a known sorted slice produces the documented p50/p95/p99/min/max/mean values.
2. `from_sorted(&[])` yields an all-zero bucket with `n == 0` and does not panic.
3. `from_sorted` is deterministic — the same slice produces byte-identical buckets across calls.
4. Existing suite passed: `cargo test -p worldwake-core`

### Invariants

1. No floating-point arithmetic anywhere in `percentile.rs` (CLAUDE.md Determinism invariant).
2. `PercentileBucket` carries no authoritative state — it is a pure derived computation (FND-27).

## Test Plan Result

### Added Tests

1. `crates/worldwake-core/src/percentile.rs` (inline `#[cfg(test)]`) — `from_sorted` correctness on a known slice, empty-slice edge case, determinism re-call.

### Commands Passed

1. `cargo test -p worldwake-core percentile`
2. `cargo clippy -p worldwake-core --all-targets -- -D warnings`
3. `cargo test -p worldwake-core` (narrow boundary — this ticket touches only `worldwake-core`)

## Outcome

Completed on 2026-05-14.

- Added the shared `PercentileBucket` helper in `worldwake-core` with integer-only p50/p95/p99, min, max, and mean aggregation over caller-sorted `u64` slices.
- Re-exported `PercentileBucket` from `worldwake_core` for later S144 diagnostics consumers.
- Kept consumer wiring, configurable percentile sets, and observer rendering out of scope for later S144 tickets.

## Verification Result

- Passed `cargo test -p worldwake-core percentile`
- Passed `cargo clippy -p worldwake-core --all-targets -- -D warnings`
- Passed `cargo test -p worldwake-core`
