# S29-004: Benchmarking and Full Verification

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: None
**Deps**: S29-002, S29-003

## Problem

The spec requires before/after benchmarks measuring per-expansion clone cost and end-to-end `search_plan` time to confirm the structural sharing optimization delivers measurable improvement. This ticket captures the benchmark measurement and final workspace-wide verification.

## Assumption Reassessment (2026-03-27)

1. Rust's built-in `#[bench]` requires nightly. The project uses stable Rust (no `#![feature(...)]` in any crate). Benchmarks should use `cargo test` with `std::time::Instant` in dedicated `#[test]` functions, or the `criterion` crate if already present.
2. Checked: `criterion` is not in any `Cargo.toml`. Benchmarks should use wall-clock timing in `#[test]` + `#[ignore]` functions (run explicitly, not in CI), avoiding new dependencies per project convention (minimal deps).
3. The spec names two benchmark targets: (a) `golden_world_runs_without_observers` (default budget: beam_width=8, max_expansions=256), (b) a high-budget golden test from `golden_supply_chain.rs` or `golden_offices.rs` (max_expansions=1024 or beam_width=16).
4. Benchmarks measure wall-clock time for the full test scenario, not `search_plan` in isolation — the golden harness already provides the right invocation context.
5. This ticket does NOT change any code — it adds benchmark tests and documents measured results. The actual optimization was done in S29-002 and S29-003.
6. The "before" measurement must be taken on the commit before S29-002/003. This means the benchmark functions themselves can be written at any time, but the before/after comparison requires running them on both commits.

## Architecture Check

1. `#[test] #[ignore]` functions are the lightest approach: no new crate deps, no nightly, explicit opt-in via `cargo test -- --ignored`.
2. No backwards-compatibility shims. Benchmark tests are additive.

## Verification Layers

1. Benchmark results → wall-clock timing printed to stderr via `eprintln!`
2. Golden hash determinism → all golden tests pass (same hashes before and after)
3. Full workspace health → `cargo test --workspace` + `cargo clippy --workspace`
4. Single-layer ticket: benchmark-only, no behavioral changes.

## What to Change

### 1. Add benchmark tests

Create `#[test] #[ignore]` functions in a new file `crates/worldwake-ai/tests/bench_search_perf.rs` (integration test) or in `search/tests.rs` (unit test). Each function:
- Sets up the golden scenario harness.
- Runs `step_once()` N times (or the specific golden scenario).
- Measures wall-clock time via `std::time::Instant`.
- Prints elapsed time via `eprintln!`.

Scenarios:
- Default-budget: a scenario using beam_width=8, max_expansions=256.
- High-budget: a scenario using beam_width=16 or max_expansions=1024.

### 2. Run and document results

Run benchmarks on the commit before S29-002/003 (baseline) and after (optimized). Document the measured improvement in the spec status or a brief note committed alongside.

## Files to Touch

- `crates/worldwake-ai/tests/bench_search_perf.rs` (new — integration test file with `#[ignore]` benchmarks)

## Out of Scope

- Adding `criterion` or any other benchmark crate as a dependency.
- Changing any production code.
- Changing `planning_state.rs`, `search/*.rs`, or `shared_collections.rs`.
- CI integration of benchmarks (these are manual, opt-in).
- Performance targets that would gate the release (the spec says "measurable reduction", not a specific threshold).

## Acceptance Criteria

### Tests That Must Pass

1. Benchmark functions compile and run: `cargo test -p worldwake-ai bench_search_perf -- --ignored`
2. All golden tests pass unchanged: `cargo test -p worldwake-ai golden`
3. Full workspace: `cargo test --workspace`
4. `cargo clippy --workspace` — no new warnings.

### Invariants

1. Golden test hash values are identical before and after S29 (determinism preserved across the entire S29 series).
2. No new external dependencies introduced.
3. Benchmark tests do not run in the normal `cargo test` suite (they are `#[ignore]`).

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/bench_search_perf.rs` — `#[ignore]` benchmark tests measuring search_plan wall-clock time for default-budget and high-budget scenarios.

### Commands

1. `cargo test -p worldwake-ai bench_search_perf -- --ignored` (run benchmarks)
2. `cargo clippy --workspace && cargo test --workspace` (full verification)
