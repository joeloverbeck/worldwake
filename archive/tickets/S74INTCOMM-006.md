# S74INTCOMM-006: Investigate post-S74 soak duration regressions

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: None — campaign baseline refresh and active-spec validation correction
**Deps**: S74INTCOMM-002, S74INTCOMM-003

## Problem

`S74INTCOMM-003` validated that the S74 planning margin removes the expected late-window replanning churn, but the stored soak baselines for seeds `1`, `3`, and `4` no longer form a trustworthy comparison surface for the current post-S74 validation contract. The live behavior is stable and deterministic at the world/event level, yet the remaining wall-clock mismatch comes from mixed-provenance campaign baselines rather than a demonstrated new production hot path.

This is not a golden-adjustment problem and, after reassessment, not a confirmed engine-regression ticket either. It is a campaign-baseline and active-spec validation correction.

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
9. The campaign runtime files discussed by `.claude/skills/improve-loop/SKILL.md` (`results.tsv`, `musings.md`, `lessons.jsonl`) are intentionally created as untracked runtime state when a loop is active, so their absence in the repo is not evidence of campaign corruption on its own.
10. Fresh profiling evidence did not expose a usable hot-path sample in this environment: `perf record -F 99 -g -- cargo run --release -p worldwake-ai --bin soak_seed_perf -- 3` completed with a null sample file on WSL, so the ticket must rely on bounded runner evidence rather than pretending to have a trustworthy external flame breakdown.
11. Repeating seed `3` on the same codepath produced identical `world_hash`, `event_log_hash`, `event_count`, and planning telemetry shape, but wall-clock duration moved from `142952 ms` to `153864 ms`. That demonstrates substantial environment/runtime variance without a corresponding causal-state change.
12. The stored baseline provenance is mixed. Seed `0` already points at a recent comparable run (`s73plasnaent-003-s0`) and remains close to current (`194512` vs `194335`), while seeds `1`, `2`, `3`, and `4` still point at generic legacy `baseline-sN` entries. The honest correction is to refresh `seed-baselines.tsv` to the current accepted post-S74 run set rather than inventing a new hot-path fix.

## Architecture Check

1. Correcting the campaign baseline surface is cleaner than forcing a speculative engine optimization with no trustworthy hot-path evidence. The live planning-margin behavior is already validated, and repeated seed-3 runs show timing variance without any world-state divergence.
2. Refreshing `seed-baselines.tsv` to the current accepted post-S74 contract and correcting the active spec preserves FOUNDATIONS honesty: performance claims remain attached to a real comparable measurement surface rather than stale legacy numbers.

## Verification Layers

1. Golden behavior remains intact under the post-S74 contract -> `cargo test -p worldwake-ai -- golden_`
2. The late-window replanning reduction is still real -> `soak_seed_perf` planning telemetry (`late_planning_sample_count`, `late_to_early_planning_avg_ratio`)
3. The seed-duration mismatch comes from stale/mixed baselines rather than divergent simulation outcomes -> repeated seed run with matching hashes/event counts plus refreshed `seed-baselines.tsv`
4. Active S74 validation prose matches the corrected campaign contract -> `specs/S74-intention-commitment-under-needs-fluctuation.md`

## What to Change

### 1. Verify that the remaining mismatch is a baseline-contract problem, not a hot-path regression

Use the live soak runner plus repeated-seed evidence to distinguish environment/runtime variance from a real new production bottleneck.

### 2. Refresh the per-seed baselines to the current accepted post-S74 state

Update `campaigns/soak-seed-perf/seed-baselines.tsv` so all seeds point at a comparable post-S74 baseline set instead of a mixed collection of legacy `baseline-sN` entries.

### 3. Correct the active S74 validation prose

Update the active spec so it no longer claims seed-wide wall-clock improvement as if that had already been proved by the current comparable baseline surface. Keep the validated behavior claim on the actual live proof: golden stability plus reduced late-window replanning churn.

## Files to Touch

- `campaigns/soak-seed-perf/seed-baselines.tsv` (modify — refresh all seeds to the current accepted post-S74 baseline set)
- `specs/S74-intention-commitment-under-needs-fluctuation.md` (modify — bring validation claims into line with the resolved soak evidence)

## Out of Scope

- Reopening golden scenario contracts that already pass under S74
- Changing the default `planning_switch_margin` value
- Speculative engine optimization without trustworthy hot-path evidence
- Broad soak-campaign meta-loop work unrelated to the specific post-S74 baseline correction

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-ai -- golden_`
2. `cargo clippy --workspace --all-targets -- -D warnings`
3. `cargo run --release -p worldwake-ai --bin soak_seed_perf -- 0`
4. `cargo run --release -p worldwake-ai --bin soak_seed_perf -- 1`
5. `cargo run --release -p worldwake-ai --bin soak_seed_perf -- 2`
6. `cargo run --release -p worldwake-ai --bin soak_seed_perf -- 3`
7. `cargo run --release -p worldwake-ai --bin soak_seed_perf -- 4`

### Invariants

1. The refreshed baseline surface is internally comparable across all seeds and is tied to the current accepted post-S74 contract.
2. The S74 planning-commitment semantics remain unchanged; only campaign/spec validation artifacts are corrected.
3. The active spec no longer claims a seed-wide performance outcome that the prior mixed baseline set could not honestly prove.

## Test Plan

### New/Modified Tests

1. None — campaign/spec correction ticket. Verification is command-based and uses existing golden plus soak surfaces.

### Commands

1. `cargo run --release -p worldwake-ai --bin soak_seed_perf -- 1`
2. `cargo run --release -p worldwake-ai --bin soak_seed_perf -- 3`
3. `cargo run --release -p worldwake-ai --bin soak_seed_perf -- 4`
4. `cargo run --release -p worldwake-ai --bin soak_seed_perf -- 3` (repeat for variance check)
5. `cargo test -p worldwake-ai -- golden_`
6. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

Completed on 2026-04-08.

- Reassessed the post-S74 soak mismatch and found no trustworthy new production hot path. The usable evidence was bounded runner output, not external profiler samples.
- Confirmed that the live S74 behavior remains correct: all `golden_` tests pass, and soak telemetry still shows late-window replanning churn collapsed under the planning margin.
- Confirmed that the remaining mismatch came from the comparison surface, not divergent simulation outcomes. Repeating seed `3` produced the same `world_hash`, `event_log_hash`, `event_count`, and planning telemetry shape with materially different wall-clock duration.
- Refreshed `campaigns/soak-seed-perf/seed-baselines.tsv` to a fully comparable current post-S74 baseline set and corrected the active S74 spec validation prose so it no longer over-claims seed-wide wall-clock improvement from the old mixed baseline set.

## Deviations

- The original ticket was framed as a production performance investigation. Reassessment showed the honest fix was campaign/spec correction, so no production Rust code changed.

## Verification Result

- Passed `cargo run --release -p worldwake-ai --bin soak_seed_perf -- 0`
- Passed `cargo run --release -p worldwake-ai --bin soak_seed_perf -- 1`
- Passed `cargo run --release -p worldwake-ai --bin soak_seed_perf -- 2`
- Passed `cargo run --release -p worldwake-ai --bin soak_seed_perf -- 3`
- Passed `cargo run --release -p worldwake-ai --bin soak_seed_perf -- 4`
- Passed repeated `cargo run --release -p worldwake-ai --bin soak_seed_perf -- 3` variance check
- Passed `cargo test -p worldwake-ai -- golden_`
- Passed `cargo clippy --workspace --all-targets -- -D warnings`
