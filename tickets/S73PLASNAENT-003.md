# S73PLASNAENT-003: Soak performance regression guard

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: None
**Deps**: S73PLASNAENT-002

## Problem

S73's goal is to eliminate the 25x planning cost growth between early and late game in long-running soak tests. After implementing the entity filter (ticket 002), we need to verify the performance claim: soak seed 0 planning cost should not grow superlinearly with tick count, and the per-place cap (default 50) should not change soak behavioral outcomes.

## Assumption Reassessment (2026-04-08)

1. Soak performance harness exists at `crates/worldwake-ai/src/bin/soak_seed_perf.rs` — standalone binary invoked via `cargo run --release -p worldwake-ai --bin soak_seed_perf -- <seed-id>`.
2. Campaign infrastructure at `campaigns/soak-seed-perf/` includes `harness.sh` (immutable run harness), `checks.sh` (pre-optimization checks), and `seed-baselines.tsv` (baseline records).
3. Golden soak test at `crates/worldwake-ai/tests/golden_soak.rs:254` (`t30_seven_day_soak`) runs 10,080 ticks across 10 seeds, gated behind `#[cfg(feature = "soak")]`. Invoked via `cargo test -p worldwake-ai --features soak --test golden_soak`.
4. Profiling baseline from `soak-seed-perf` campaign: early game ~0.8ms/agent-tick, late game ~20.7ms/agent-tick. Post-filter, late game should be substantially reduced.

## Architecture Check

1. This is a verification-only ticket — no production code changes. The performance measurement uses the existing soak harness infrastructure.
2. No backward-compatibility concerns — we are measuring, not modifying.

## Verification Layers

1. Planning cost curve is sublinear (no superlinear growth) -> soak_seed_perf binary output for seed 0
2. Soak behavioral outcomes unchanged -> golden soak test passes with same seed determinism
3. Single-layer ticket: performance measurement only; additional layer mapping not applicable.

## What to Change

### 1. Run soak seed 0 performance measurement

Execute `cargo run --release -p worldwake-ai --bin soak_seed_perf -- 0` and record per-agent-tick planning cost at early game (tick ~100-500) and late game (tick ~9000-10000). Compare against baseline (0.8ms early, 20.7ms late).

**Correctness threshold**: Late-game per-agent-tick planning cost must not exceed 5x early-game cost (previously 25x). The exact ratio depends on entity accumulation patterns, but the filter should bring it well below 5x.

**Performance threshold**: Late-game per-agent-tick planning cost should be under 5ms (previously 20.7ms).

### 2. Run golden soak test

Execute `cargo test -p worldwake-ai --features soak --test golden_soak` and verify all seeds pass. This confirms the entity filter does not change behavioral outcomes.

### 3. Update seed baselines

If performance thresholds are met, update `campaigns/soak-seed-perf/seed-baselines.tsv` with new post-filter measurements for seed 0.

## Files to Touch

- `campaigns/soak-seed-perf/seed-baselines.tsv` (modify — update with post-filter measurements)

## Out of Scope

- Production code changes — this is verification only
- Tuning `max_snapshot_entities_per_place` default — 50 is the spec default; tuning is future work if needed
- Multi-seed performance sweep — seed 0 is the representative benchmark; full sweep is optional follow-up
- Belief store changes, perception changes, authoritative state changes — unchanged per S73

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-ai --features soak --test golden_soak` — all seeds pass
2. Soak seed 0 late-game planning cost < 5ms per agent-tick
3. Soak seed 0 late/early planning cost ratio < 5x

### Invariants

1. Golden soak test determinism preserved — same seed produces same outcome
2. No behavioral regressions — soak completion tick counts unchanged within noise margin

## Test Plan

### New/Modified Tests

None — verification is command-based and existing runtime coverage is named in Assumption Reassessment.

### Commands

1. `cargo run --release -p worldwake-ai --bin soak_seed_perf -- 0`
2. `cargo test -p worldwake-ai --features soak --test golden_soak`
3. `cargo test -p worldwake-ai`
