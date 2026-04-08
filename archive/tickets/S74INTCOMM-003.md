# S74INTCOMM-003: Golden test validation and soak regression check

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: None — validation-only close-out and follow-up routing
**Deps**: S74INTCOMM-002, S74INTCOMM-005

## Problem

With the margin-based plan continuation in place (S74INTCOMM-002), golden tests that depend on rapid goal switching under need pressure may fail if the default `planning_switch_margin = 150` suppresses the switch. This ticket owns the post-implementation validation pass: rerun the golden suites, adjust per-agent margin values or scenario pressure only when a failure is genuinely margin-driven, and then confirm soak-seed-perf regression bounds.

## Assumption Reassessment (2026-04-08)

1. `golden_merchant_selling` at `crates/worldwake-ai/tests/golden_merchant_selling.rs` contains multiple scenarios. `loose_home_stock_is_staged_before_sell_goal_settles` is still plausibly margin-sensitive because it depends on need-driven switching, but `combined_market_trip_selected_for_side_benefit` is no longer owned here after S74INTCOMM-002 review.
2. Soak-seed-perf campaign at `campaigns/soak-seed-perf/program.md` with binary harness at `crates/worldwake-ai/src/bin/soak_seed_perf.rs`. Seeds 0-4 rotation. The spec's motivation is reducing full planning passes from ~10,000 per agent over 10,000 ticks. The margin should significantly reduce this without introducing seed-specific regressions.
3. All golden tests in `crates/worldwake-ai/tests/golden_*.rs` must pass. The margin change is global (every agent with a CognitiveProfile gets it), so all golden tests are potentially affected, not just `golden_merchant_selling`.
4. S74INTCOMM-002 broad verification exposed `combined_market_trip_selected_for_side_benefit` and its replay twin as a production-side branch-stability contradiction during in-progress replanning, not on snapshot-only continuation. S74INTCOMM-005 owns and resolves that engine path, so this ticket must not paper over it with test-only margin overrides.
5. If a remaining golden genuinely needs margin adjustment, prefer per-agent `planning_switch_margin` overrides or stronger need pressure only when the test is proving rapid goal switching under need pressure rather than a separate production contract.
6. Live validation after S74INTCOMM-005 shows no remaining margin-driven golden fallout. `cargo test -p worldwake-ai --test golden_merchant_selling loose_home_stock_is_staged_before_sell_goal_settles` passes unchanged, and `cargo test -p worldwake-ai -- golden_` passes across the full golden surface without any per-agent `planning_switch_margin` overrides.
7. The live soak runner at `crates/worldwake-ai/src/bin/soak_seed_perf.rs` does not emit "full GOAP search count" directly. It emits `plan_and_validate_next_step` timing and sample-count windows via `PlanningTelemetrySummary` in `crates/worldwake-ai/src/perf_telemetry.rs`, so the ticket must use that telemetry contract instead of stale metric prose.
8. Soak seeds `0..4` on the current post-S74 state show the intended collapse in late-window planning churn, but wall-clock duration remains above stored baselines for seeds `1`, `3`, and `4`. That is a production performance follow-up, not a golden-adjustment task for this ticket.

## Architecture Check

1. Closing this ticket as validation-only is cleaner than forcing test edits where no margin-driven golden failure remains. The golden surface is already green, so widening this ticket into a production performance investigation would blur a validation handoff with a new optimization boundary.
2. No backwards-compatibility aliasing/shims introduced.

## Verification Layers

1. All golden tests pass with correct behavioral outcomes -> golden test suite pass (`cargo test -p worldwake-ai -- golden_`)
2. The plausibly margin-sensitive merchant scenario still holds without overrides -> targeted golden proof (`cargo test -p worldwake-ai --test golden_merchant_selling loose_home_stock_is_staged_before_sell_goal_settles`)
3. Soak seeds 0-4 still show reduced late-window replanning activity -> `soak_seed_perf` planning telemetry (`early_planning_sample_count`, `late_planning_sample_count`, `late_to_early_planning_avg_ratio`)
4. Remaining seed-duration regressions are identified as a separate production concern -> seed-by-seed soak runner outputs compared against `campaigns/soak-seed-perf/seed-baselines.tsv`

## What to Change

### 1. Run the golden validation surface and confirm whether any margin-driven fallout remains

Run the full golden surface plus the merchant scenario singled out by the spec. Only touch test scenarios if a real margin-driven failure remains.

### 2. Validate the live soak telemetry surface

Run `soak_seed_perf` for seeds `0..4` and compare the emitted planning telemetry plus wall-clock duration against the ticket's claims.

### 3. Split any remaining production regression out of this ticket

If goldens are green but soak duration still regresses on some seeds, create a follow-up performance ticket instead of broadening this validation ticket into engine work.

## Files to Touch

- `tickets/S74INTCOMM-003.md` (modify — record the completed validation result honestly)
- `tickets/S74INTCOMM-006.md` (new — own the remaining soak duration regression investigation)

## Out of Scope

- Modifying the margin comparison logic itself (S74INTCOMM-002)
- Fixing the in-progress replanning side-benefit branch-stability regression in `combined_market_trip_selected_for_side_benefit` (S74INTCOMM-005)
- Fixing soak duration regressions on seeds `1`, `3`, and `4` (S74INTCOMM-006)
- Changing the default `planning_switch_margin` value (that's a CognitiveProfile default in worldwake-core)
- Performance optimization beyond what the margin provides (future work)
- Adjusting non-golden test helpers (already done in S74INTCOMM-001)

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-ai -- golden_` — all golden tests pass
2. `cargo test -p worldwake-ai --test golden_merchant_selling loose_home_stock_is_staged_before_sell_goal_settles` — the plausibly margin-sensitive merchant scenario still holds without overrides
3. `cargo clippy --workspace --all-targets -- -D warnings`

### Invariants

1. No golden scenario requires a per-agent `planning_switch_margin` override or stronger need pressure to preserve its existing intended contract.
2. The live soak telemetry proves that late-window replanning churn is reduced relative to the early window under the S74 margin path.
3. Any remaining soak duration regression is not hidden inside this ticket as a fake "test adjustment"; it is captured as a separate production follow-up ticket.
4. `cargo clippy --workspace --all-targets -- -D warnings` passes.

## Test Plan

### New/Modified Tests

1. None — validation-only ticket. Verification is command-based and uses existing golden plus soak surfaces.

### Commands

1. `cargo test -p worldwake-ai --test golden_merchant_selling loose_home_stock_is_staged_before_sell_goal_settles`
2. `cargo test -p worldwake-ai -- golden_`
3. `cargo run --release -p worldwake-ai --bin soak_seed_perf -- 0`
4. `cargo run --release -p worldwake-ai --bin soak_seed_perf -- 1`
5. `cargo run --release -p worldwake-ai --bin soak_seed_perf -- 2`
6. `cargo run --release -p worldwake-ai --bin soak_seed_perf -- 3`
7. `cargo run --release -p worldwake-ai --bin soak_seed_perf -- 4`
8. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

Completed on 2026-04-08.

- Verified that no margin-driven golden fallout remains after S74INTCOMM-005. The full golden surface passes, and `loose_home_stock_is_staged_before_sell_goal_settles` passes unchanged without any per-agent `planning_switch_margin` override.
- Verified the live soak telemetry contract for seeds `0..4`. The runner reports `plan_and_validate_next_step` window sample counts and timing, not direct "full GOAP search count" metrics.
- Confirmed that the S74 planning-margin behavior still suppresses late-window replanning churn, but identified remaining wall-clock duration regressions against stored baselines on seeds `1`, `3`, and `4`.
- Created S74INTCOMM-006 to own that production performance follow-up instead of broadening this validation ticket into engine work.

## Deviations

- The original ticket assumed this turn might need golden test adjustments. Live validation proved no test edits were required, so the ticket completed as a validation-only handoff plus a new performance follow-up.

## Verification Result

- Passed `cargo test -p worldwake-ai --test golden_merchant_selling loose_home_stock_is_staged_before_sell_goal_settles`
- Passed `cargo test -p worldwake-ai -- golden_`
- Passed `cargo run --release -p worldwake-ai --bin soak_seed_perf -- 0`
- Passed `cargo run --release -p worldwake-ai --bin soak_seed_perf -- 1`
- Passed `cargo run --release -p worldwake-ai --bin soak_seed_perf -- 2`
- Passed `cargo run --release -p worldwake-ai --bin soak_seed_perf -- 3`
- Passed `cargo run --release -p worldwake-ai --bin soak_seed_perf -- 4`
- Passed `cargo clippy --workspace --all-targets -- -D warnings`
