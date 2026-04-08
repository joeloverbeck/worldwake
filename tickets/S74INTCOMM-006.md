# S74INTCOMM-006: Investigate post-S74 soak duration regressions

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — production performance investigation on the post-S74 planning path
**Deps**: S74INTCOMM-002, S74INTCOMM-003

## Problem

`S74INTCOMM-003` validated that the S74 planning margin removes the expected late-window replanning churn, but the full soak runner still regresses in wall-clock duration against stored campaign baselines on multiple seeds. The current post-S74 state is therefore only partially validated: the intended planning-commitment behavior is live, yet end-to-end soak duration remains above baseline on seeds `1`, `3`, and `4`.

This is not a golden-adjustment problem. It is a production performance investigation on the live soak path after the margin feature landed.

## Assumption Reassessment (2026-04-08)

1. The authoritative validation for S74INTCOMM-003 is green on the golden surface: `cargo test -p worldwake-ai -- golden_` passes, and the plausibly margin-sensitive merchant scenario `loose_home_stock_is_staged_before_sell_goal_settles` in `crates/worldwake-ai/tests/golden_merchant_selling.rs` passes unchanged.
2. The live soak runner is `crates/worldwake-ai/src/bin/soak_seed_perf.rs`, and the telemetry carrier is `PlanningTelemetrySummary` in `crates/worldwake-ai/src/perf_telemetry.rs`.
3. The live runner does not emit direct "full GOAP search count" metrics. It emits per-window `sample_count`, `total_duration`, `average_duration`, and the late-to-early average ratio for `plan_and_validate_next_step`.
4. Post-S74 soak runs on 2026-04-08 produced:
   - seed `0`: `194512 ms` vs baseline `194335`
   - seed `1`: `155112 ms` vs baseline `151469`
   - seed `2`: `134001 ms` vs baseline `242409`
   - seed `3`: `142952 ms` vs baseline `110567`
   - seed `4`: `167671 ms` vs baseline `109393`
5. The same runs still show the intended planning-margin effect. Late-window planning samples are `0` for seeds `0`, `1`, `3`, and `4`, and `36` for seed `2`, so the regression is not "margin failed to reduce replanning churn."
6. The exact abstraction boundary under audit is end-to-end soak duration after planning-margin stabilization: determine which remaining hot path dominates seeds `1`, `3`, and `4` once late-window replanning churn is mostly gone.
7. This ticket must follow the soak campaign rules in `campaigns/soak-seed-perf/program.md`, including profile-first investigation, seed-specific baseline comparison, and the correctness gate.
8. `specs/S74-intention-commitment-under-needs-fluctuation.md` still claims "Soak seeds 0–4 all show improvement (no seed-specific regression)." This ticket owns resolving that active-spec drift once the remaining regression is classified as a real hot path or a stale baseline/campaign-contract issue.

## Architecture Check

1. Separating this from S74INTCOMM-003 keeps the validation ticket honest and gives the remaining duration regression a clean production-performance boundary.
2. The fix should target the earliest concrete hot path that still dominates soak duration after replanning churn dropped. Do not weaken the S74 commitment semantics or reintroduce per-tick replanning just to match an old wall-clock number.

## Verification Layers

1. Golden behavior remains intact while optimizing -> `cargo test -p worldwake-ai -- golden_`
2. The suspected hot path is real -> fresh profiling evidence or bounded diagnostic telemetry recorded against the soak runner
3. Seed-specific wall-clock improvement is real -> `cargo run --release -p worldwake-ai --bin soak_seed_perf -- <seed>` compared against `campaigns/soak-seed-perf/seed-baselines.tsv`
4. Soak-path correctness remains intact -> campaign correctness gate from `campaigns/soak-seed-perf/program.md`

## What to Change

### 1. Re-profile the live post-S74 soak path

Gather fresh profiling evidence on the current accepted post-S74 state, with special attention to seeds `1`, `3`, and `4`, to determine what now dominates soak duration once late-window replanning churn is mostly suppressed.

### 2. Identify the real remaining hot path

Determine whether the remaining regression is in:

- early-window planning cost despite reduced late-window churn,
- another AI hot path adjacent to planning,
- perception/event-log overhead,
- ECS lookup or scheduler cost,
- or a stale/non-comparable baseline issue that needs campaign upkeep rather than production code changes.

### 3. Implement the narrowest lawful optimization or campaign correction

If profiling finds a real production hot path, optimize that path without changing world meaning or weakening S74 commitment semantics. If profiling instead proves the stored baseline is stale or no longer comparable under the current soak contract, update the campaign artifacts honestly per the campaign rules rather than inventing a fake engine fix.

### 4. Bring the active S74 validation prose back into alignment

Once the regression source is understood, update the active spec and any campaign-facing validation prose so they no longer claim seed-wide improvement that the live proof surface does not currently support.

## Files to Touch

- `crates/worldwake-ai/src/bin/soak_seed_perf.rs` (modify only if telemetry needs bounded extension for diagnosis)
- `crates/worldwake-ai/src/perf_telemetry.rs` (modify only if telemetry needs bounded extension for diagnosis)
- `crates/worldwake-ai/src/**/*.rs` (modify — only the profiled hot path once identified)
- `campaigns/soak-seed-perf/musings.md` (modify — record profiling findings before editing production code)
- `campaigns/soak-seed-perf/seed-baselines.tsv` (modify only if campaign rules justify a baseline update after accepted results)
- `specs/S74-intention-commitment-under-needs-fluctuation.md` (modify — bring validation claims into line with the resolved soak evidence)

## Out of Scope

- Reopening golden scenario contracts that already pass under S74
- Changing the default `planning_switch_margin` value without profiling proof that it is the real remaining performance bottleneck
- Papering over soak duration regressions by weakening correctness checks or campaign invariants
- Broad soak-campaign meta-loop work unrelated to the specific post-S74 regression

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-ai -- golden_`
2. `cargo clippy --workspace --all-targets -- -D warnings`
3. Campaign correctness gate per `campaigns/soak-seed-perf/program.md`

### Invariants

1. The optimization target is chosen from fresh profiling evidence, not from stale architectural guesses.
2. Any accepted change preserves the S74 planning-commitment semantics while improving or honestly reclassifying the remaining soak regression.
3. Seed-specific comparisons remain aligned with `campaigns/soak-seed-perf/seed-baselines.tsv`.

## Test Plan

### New/Modified Tests

1. None yet — start with profiling and campaign verification before deciding whether new focused tests are needed for the eventual hot-path fix.

### Commands

1. `cargo run --release -p worldwake-ai --bin soak_seed_perf -- 1`
2. `cargo run --release -p worldwake-ai --bin soak_seed_perf -- 3`
3. `cargo run --release -p worldwake-ai --bin soak_seed_perf -- 4`
4. `cargo test -p worldwake-ai -- golden_`
5. `cargo clippy --workspace --all-targets -- -D warnings`
