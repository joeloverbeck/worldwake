# S73PLASNAENT-004: Add segmented planning telemetry to soak_seed_perf

**Status**: COMPLETED
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
4. Reassessment correction: the benchmark binary alone cannot observe planner-phase timings. The narrow honest hook point is `crates/worldwake-ai/src/agent_tick/planning.rs:580` (`plan_and_validate_next_step`), with `soak_seed_perf` only responsible for session setup and output emission.
5. `campaigns/soak-seed-perf/harness.sh` parses only `duration_ms`, so additional telemetry lines are safe as long as the aggregate keys remain unchanged.
6. Live verification on seed 0 shows explicit early-window planning samples but zero planning-phase samples in the configured late tick window (`9000..10000`). The benchmark therefore needs to emit `late_to_early_planning_avg_ratio=NA` honestly when the denominator or late window sample set is absent, rather than fabricating a numeric ratio.

## Architecture Check

1. Adding a narrow planner-phase telemetry hook and letting the benchmark binary report it is cleaner than over-claiming from aggregate runtime or timing unrelated soak work; it improves measurement fidelity without changing production planning semantics.
2. No backward-compatibility shims are needed. The benchmark output contract can be updated directly because it is repo-local tooling.

## Verification Layers

1. Benchmark emits segmented planning-phase telemetry for the declared windows -> focused run of `cargo run --release -p worldwake-ai --bin soak_seed_perf -- 0`
2. Existing aggregate hashes and event counts remain emitted -> focused run output
3. The telemetry helper and hook compile cleanly under test and CI surfaces -> `cargo test -p worldwake-ai perf_telemetry` and `cargo clippy --workspace --all-targets -- -D warnings`
4. Single-layer ticket: benchmark instrumentation only; no additional layer mapping applies.

## What to Change

### 1. Add segmented planning-phase telemetry

Add a narrow opt-in timing hook around `plan_and_validate_next_step(...)` and have `soak_seed_perf` collect and emit at least:

- an early planning window covering ticks near the start of the soak
- a late planning window covering ticks near the end of the soak
- a derived late/early ratio, or explicit `NA` when a configured window has zero planning samples

The chosen windows must be explicit in the benchmark output and stable across runs.

### 2. Keep existing aggregate output intact

Preserve the current `duration_ms`, `event_count`, `world_hash`, and `event_log_hash` lines so existing campaign tooling and ticket 003's aggregate baseline workflow remain usable.

### 3. Update campaign expectations if needed

If campaign docs or scripts assume only aggregate output, update the narrowest owned campaign surface so the new telemetry is discoverable without breaking the existing aggregate parse path.

## Files to Touch

- `crates/worldwake-ai/src/agent_tick/planning.rs` (modify)
- `crates/worldwake-ai/src/perf_telemetry.rs` (new)
- `crates/worldwake-ai/src/bin/soak_seed_perf.rs` (modify)
- `campaigns/soak-seed-perf/program.md` (modify — factual telemetry note)

## Out of Scope

- Changing planning behavior or CognitiveProfile defaults
- Re-running or updating aggregate seed baselines owned by `S73PLASNAENT-003`
- Multi-seed telemetry sweeps

## Acceptance Criteria

### Tests That Must Pass

1. `cargo run --release -p worldwake-ai --bin soak_seed_perf -- 0` emits explicit early/late planning telemetry and a ratio field in addition to the existing aggregate lines
2. Existing aggregate lines (`duration_ms`, `event_count`, `world_hash`, `event_log_hash`) remain present

### Invariants

1. Benchmark instrumentation does not change production simulation behavior
2. The emitted telemetry windows are explicit and deterministic for a fixed seed
3. Empty late-window samples are reported honestly as `NA`, not coerced into a numeric ratio

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/perf_telemetry.rs` — verifies window capture and ratio arithmetic

### Commands

1. `cargo run --release -p worldwake-ai --bin soak_seed_perf -- 0`
2. `cargo test -p worldwake-ai perf_telemetry`
3. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

Completed on 2026-04-08.

- Added a new `perf_telemetry` helper and a narrow timing hook around `plan_and_validate_next_step(...)` so the soak benchmark can report explicit planning-phase windows without changing planner behavior.
- Extended `soak_seed_perf` to emit early and late window bounds, per-window planning sample counts, total/average microseconds, and `late_to_early_planning_avg_ratio`.
- Preserved the existing aggregate output keys (`duration_ms`, `event_count`, `world_hash`, `event_log_hash`) and updated `campaigns/soak-seed-perf/program.md` to note that extra telemetry keys are diagnostic only.

## Deviations

- Reassessment corrected the owned boundary: benchmark-local telemetry required a narrow planner hook in `agent_tick/planning.rs`, not just changes inside `soak_seed_perf.rs`.
- On the verified seed-0 run, the configured late tick window (`9000..10000`) had zero planning-phase samples, so the benchmark now emits `late_to_early_planning_avg_ratio=NA` honestly instead of inventing a numeric ratio.

## Verification Result

- Passed `cargo test -p worldwake-ai perf_telemetry`
- Passed `cargo run --release -p worldwake-ai --bin soak_seed_perf -- 0`
- Passed `cargo clippy --workspace --all-targets -- -D warnings`
- Seed-0 benchmark output included:
  - `planning_metric=plan_and_validate_next_step`
  - `early_tick_start=100`
  - `early_tick_end_exclusive=500`
  - `early_planning_sample_count=5978`
  - `early_planning_avg_us=992`
  - `late_tick_start=9000`
  - `late_tick_end_exclusive=10000`
  - `late_planning_sample_count=0`
  - `late_to_early_planning_avg_ratio=NA`
