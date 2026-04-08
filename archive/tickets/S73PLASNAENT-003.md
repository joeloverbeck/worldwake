# S73PLASNAENT-003: Soak performance regression guard

**Status**: COMPLETED
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
4. Reassessment correction: the live `soak_seed_perf` binary does not emit early/late per-agent-tick planning telemetry. It emits only `seed_id`, aggregate `duration_ms`, `event_count`, `world_hash`, and `event_log_hash`, and the campaign baseline file stores only total `best_duration_ms` per seed. The honest measurable contract for this ticket is therefore total seed-0 runtime improvement, with broader soak-behavior proof deferred when the full soak harness is not run locally.
5. `campaigns/soak-seed-perf/seed-baselines.tsv` currently records seed 0 baseline `best_duration_ms=341822` with `source_exp_id=baseline-s0`. If the post-filter run improves that total runtime while preserving soak behavior, updating the baseline file is an in-scope factual change.
6. The removed slope-proof slice now has no live owner. Follow-up ticket `S73PLASNAENT-004` is required to add honest early/late planning telemetry to `soak_seed_perf` (or another bounded benchmark surface) before the original per-phase cost-growth claim can be verified directly.

## Architecture Check

1. This is a verification-only ticket — no production code changes. The performance measurement uses the existing soak harness infrastructure.
2. No backward-compatibility concerns — we are measuring, not modifying.

## Verification Layers

1. Aggregate seed-0 soak runtime improves against the recorded baseline -> `soak_seed_perf` binary output + `seed-baselines.tsv`
2. Soak behavioral outcomes unchanged -> soak verification command(s) completed at the chosen local scope, or broader soak verification is explicitly deferred
3. Single-layer ticket: performance measurement only; additional layer mapping not applicable.

## What to Change

### 1. Run soak seed 0 performance measurement

Execute `cargo run --release -p worldwake-ai --bin soak_seed_perf -- 0` and record the emitted aggregate `duration_ms`, `event_count`, `world_hash`, and `event_log_hash`. Compare `duration_ms` against the current campaign baseline for seed 0 (`341822` ms).

**Correctness threshold**: Seed-0 aggregate `duration_ms` must improve relative to the recorded baseline (`341822` ms).

**Performance note**: The original early/late planning-cost growth claim is deferred to follow-up ticket `S73PLASNAENT-004`, which will add a truthful measurement surface for that finer-grained proof.

### 2. Run soak verification

Run the soak verification surface appropriate for the session. The full `cargo test -p worldwake-ai --features soak --test golden_soak` command remains the strongest proof, but local completion may rely on the aggregate seed-0 benchmark when the broader soak pass is intentionally waived or deferred.

### 3. Update seed baselines

If the aggregate runtime improves, update `campaigns/soak-seed-perf/seed-baselines.tsv` with the new seed-0 `best_duration_ms` and a new `source_exp_id` describing the post-filter run. Record any waived or deferred broader soak verification explicitly in the ticket outcome.

## Files to Touch

- `campaigns/soak-seed-perf/seed-baselines.tsv` (modify — update with post-filter measurements)

## Out of Scope

- Production code changes — this is verification only
- Tuning `max_snapshot_entities_per_place` default — 50 is the spec default; tuning is future work if needed
- Multi-seed performance sweep — seed 0 is the representative benchmark; full sweep is optional follow-up
- Belief store changes, perception changes, authoritative state changes — unchanged per S73

## Acceptance Criteria

### Tests That Must Pass

1. Seed-0 `soak_seed_perf` aggregate `duration_ms` improves relative to `341822` ms baseline
2. Updated seed-0 baseline row matches the measured post-filter aggregate runtime if criterion 1 passes
3. Any skipped broader soak verification is explicitly recorded in the ticket outcome

### Invariants

1. Seed-0 benchmark remains deterministic for the measured run surface
2. No claimed broader soak proof is recorded unless it was actually run

## Test Plan

### New/Modified Tests

None — verification is command-based and existing runtime coverage is named in Assumption Reassessment.

### Commands

1. `cargo run --release -p worldwake-ai --bin soak_seed_perf -- 0`
2. Optional broader soak verification: `cargo test -p worldwake-ai --features soak --test golden_soak`

## Outcome

Completion date: 2026-04-08

Seed-0 aggregate soak runtime improved from `341822` ms to `194335` ms after the S73 planning snapshot filter landed, and `campaigns/soak-seed-perf/seed-baselines.tsv` was updated to record that new best runtime as `s73plasnaent-003-s0`.

## Deviations

1. The original ticket assumed we could prove the spec's early/late planning-cost slope claim directly, but the live `soak_seed_perf` surface only exposes aggregate runtime. That finer-grained measurement work was split into follow-up ticket `S73PLASNAENT-004`.
2. The full multi-seed `golden_soak` verification was started locally but intentionally not completed after explicit user direction that the benchmark verification was sufficient for this ticket close-out.

## Verification Result

1. Passed: `cargo run --release -p worldwake-ai --bin soak_seed_perf -- 0`
2. Measured output:
   - `seed_id=0`
   - `duration_ms=194335`
   - `event_count=191539`
   - `world_hash=StateHash([142, 89, 207, 142, 96, 204, 173, 124, 37, 43, 16, 20, 234, 10, 35, 52, 238, 145, 151, 248, 185, 155, 214, 93, 94, 29, 237, 125, 63, 212, 115, 45])`
   - `event_log_hash=StateHash([32, 99, 154, 4, 188, 35, 223, 76, 161, 215, 169, 94, 47, 227, 49, 207, 109, 181, 183, 195, 233, 199, 23, 227, 111, 106, 231, 112, 145, 162, 63, 140])`
3. Not completed by user choice: `cargo test -p worldwake-ai --features soak --test golden_soak`
