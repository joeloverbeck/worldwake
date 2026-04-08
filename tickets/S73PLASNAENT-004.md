# S73PLASNAENT-004: Add segmented planning telemetry to soak_seed_perf

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes — worldwake-ai soak benchmark instrumentation
**Deps**: S73PLASNAENT-002, S73PLASNAENT-003

## Problem

S73's active spec claims the planning snapshot filter should eliminate the late-game planning-cost spike, but the live `crates/worldwake-ai/src/bin/soak_seed_perf.rs` benchmark only emits aggregate `duration_ms`, `event_count`, and hashes. It cannot directly prove the spec's early-vs-late planning-cost claim or a late/early ratio threshold. We need a bounded instrumentation ticket that adds an honest measurement surface for segmented planning-time telemetry without changing production simulation behavior.

## Assumption Reassessment (2026-04-08)

1. `crates/worldwake-ai/src/bin/soak_seed_perf.rs` currently times the entire 10,080-tick soak with a single `Instant` and emits only `seed_id`, `duration_ms`, `event_count`, `world_hash`, and `event_log_hash`.
2. `tickets/S73PLASNAENT-003.md` was narrowed during reassessment to the strongest honest current proof surface: aggregate seed-0 runtime improvement plus unchanged soak behavior. The removed early/late slope-proof slice now has no live owner.
3. The spec claim that references early-game and late-game planning cost remains active in `specs/S73-planning-snapshot-entity-relevance.md` Validation item 3. This ticket owns adding a truthful measurement surface for that claim rather than silently treating aggregate runtime as equivalent proof.

## Architecture Check

1. Adding telemetry to the benchmark binary is cleaner than over-claiming from aggregate runtime; it improves measurement fidelity without changing production planning semantics.
2. No backward-compatibility shims are needed. The benchmark output contract can be updated directly because it is repo-local tooling.

## Verification Layers

1. Benchmark emits segmented planning telemetry for the declared windows -> focused run of `cargo run --release -p worldwake-ai --bin soak_seed_perf -- 0`
2. Existing aggregate hashes and event counts remain emitted -> focused run output
3. The benchmark still preserves soak correctness invariants -> rerun `cargo test -p worldwake-ai --features soak --test golden_soak` only if the instrumentation shares runtime code paths beyond the benchmark binary
4. Single-layer ticket: benchmark instrumentation only; no additional layer mapping applies.

## What to Change

### 1. Add segmented timing output to `soak_seed_perf`

Update `crates/worldwake-ai/src/bin/soak_seed_perf.rs` to measure and emit at least:

- an early planning/runtime window covering ticks near the start of the soak
- a late planning/runtime window covering ticks near the end of the soak
- a derived late/early ratio based on those emitted windows

The chosen windows must be explicit in the benchmark output and stable across runs.

### 2. Keep existing aggregate output intact

Preserve the current `duration_ms`, `event_count`, `world_hash`, and `event_log_hash` lines so existing campaign tooling and ticket 003's aggregate baseline workflow remain usable.

### 3. Update campaign expectations if needed

If campaign docs or scripts assume only aggregate output, update the narrowest owned campaign surface so the new telemetry is discoverable without breaking the existing aggregate parse path.

## Files to Touch

- `crates/worldwake-ai/src/bin/soak_seed_perf.rs` (modify)
- `campaigns/soak-seed-perf/` owned docs/scripts only if the emitted output contract needs factual documentation

## Out of Scope

- Changing planning behavior or CognitiveProfile defaults
- Re-running or updating aggregate seed baselines owned by `S73PLASNAENT-003`
- Multi-seed telemetry sweeps

## Acceptance Criteria

### Tests That Must Pass

1. `cargo run --release -p worldwake-ai --bin soak_seed_perf -- 0` emits explicit early/late telemetry and a ratio in addition to the existing aggregate lines
2. Existing aggregate lines (`duration_ms`, `event_count`, `world_hash`, `event_log_hash`) remain present

### Invariants

1. Benchmark instrumentation does not change production simulation behavior
2. The emitted telemetry windows are explicit and deterministic for a fixed seed

## Test Plan

### New/Modified Tests

None — verification is command-based and benchmark-output-based.

### Commands

1. `cargo run --release -p worldwake-ai --bin soak_seed_perf -- 0`
2. `cargo clippy --workspace --all-targets -- -D warnings`
