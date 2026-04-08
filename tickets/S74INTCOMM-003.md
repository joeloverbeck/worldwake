# S74INTCOMM-003: Golden test validation and soak regression check

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: None — test-only adjustments
**Deps**: S74INTCOMM-002, S74INTCOMM-005

## Problem

With the margin-based plan continuation in place (S74INTCOMM-002), golden tests that depend on rapid goal switching under need pressure may fail if the default `planning_switch_margin = 150` suppresses the switch. This ticket owns the post-implementation validation pass: rerun the golden suites, adjust per-agent margin values or scenario pressure only when a failure is genuinely margin-driven, and then confirm soak-seed-perf regression bounds.

## Assumption Reassessment (2026-04-08)

1. `golden_merchant_selling` at `crates/worldwake-ai/tests/golden_merchant_selling.rs` contains multiple scenarios. `loose_home_stock_is_staged_before_sell_goal_settles` is still plausibly margin-sensitive because it depends on need-driven switching, but `combined_market_trip_selected_for_side_benefit` is no longer owned here after S74INTCOMM-002 review.
2. Soak-seed-perf campaign at `campaigns/soak-seed-perf/program.md` with binary harness at `crates/worldwake-ai/src/bin/soak_seed_perf.rs`. Seeds 0-4 rotation. The spec's motivation is reducing full planning passes from ~10,000 per agent over 10,000 ticks. The margin should significantly reduce this without introducing seed-specific regressions.
3. All golden tests in `crates/worldwake-ai/tests/golden_*.rs` must pass. The margin change is global (every agent with a CognitiveProfile gets it), so all golden tests are potentially affected, not just `golden_merchant_selling`.
4. S74INTCOMM-002 broad verification exposed `combined_market_trip_selected_for_side_benefit` and its replay twin as failures on the `DirtySet::PLAN_FINISHED` full-replan path in `crates/worldwake-ai/src/agent_tick/active_action.rs`, not on snapshot-only continuation. That production-side branch-stability contradiction is owned by S74INTCOMM-005 and must not be papered over here with test-only margin overrides.
5. If a remaining golden genuinely needs margin adjustment, prefer per-agent `planning_switch_margin` overrides or stronger need pressure only when the test is proving rapid goal switching under need pressure rather than a separate production contract.

## Architecture Check

1. Adjusting per-agent `planning_switch_margin` in golden test scenarios is the cleanest approach only for scenarios whose intended invariant is rapid goal switching under need pressure. Production-path contradictions exposed on non-snapshot replanning belong in their own engine ticket.
2. No backwards-compatibility aliasing/shims introduced.

## Verification Layers

1. All golden tests pass with correct behavioral outcomes -> golden test suite pass (`cargo test -p worldwake-ai -- golden_`)
2. Soak seeds 0-4 show reduced full planning passes -> soak-seed-perf campaign metrics (per-agent-tick planning cost and full GOAP search count)
3. No seed-specific regression -> all 5 seeds pass soak validation bounds
4. Margin-sensitive merchant golden preserves its intended goal-switching invariant -> decision trace shows the merchant switching goals when need-driven priority exceeds the margin

## What to Change

### 1. Run all golden tests and identify failures

Run `cargo test -p worldwake-ai -- golden_` and note any failures introduced by the margin change.

### 2. Adjust margin-sensitive merchant goldens if needed

If a merchant golden fails because the margin suppresses a goal switch:

**Option A (preferred)**: Set a lower `planning_switch_margin` for the test's merchant agent in the scenario setup. This proves the agent CAN switch goals when the margin is low while the margin mechanism is still active.

**Option B**: Strengthen the need pressure in the scenario so the priority delta naturally exceeds 150, triggering the switch through the margin bypass path.

The choice depends on what the test is proving — document the rationale in a code comment.

### 3. Validate soak-seed-perf regression bounds

Run the soak-seed-perf binary with seeds 0-4 and verify:
- Full GOAP search count per agent is significantly reduced from the ~10,000 baseline
- No seed shows regression (more planning passes than baseline)
- Per-agent-tick planning cost does not exceed 5ms at tick 10,000 (rough bound — the spec cites 2-20ms per full pass, and the margin should eliminate most of them)

### 4. Adjust other golden tests if needed

If any other golden test fails, apply the same analysis: determine whether the test exercises rapid goal switching under need pressure, and adjust the test agent's `planning_switch_margin` accordingly. If the failure instead lands on a non-snapshot production path, stop and hand it to the owning engine ticket rather than widening this ticket.

## Files to Touch

- `crates/worldwake-ai/tests/golden_merchant_selling.rs` (modify — only for genuinely margin-driven scenarios such as `loose_home_stock_is_staged_before_sell_goal_settles`, not the side-benefit branch-stability regression owned by S74INTCOMM-005)
- Other `crates/worldwake-ai/tests/golden_*.rs` files (modify — only if failures occur)

## Out of Scope

- Modifying the margin comparison logic itself (S74INTCOMM-002)
- Fixing the `PLAN_FINISHED` side-benefit branch-stability regression in `combined_market_trip_selected_for_side_benefit` (S74INTCOMM-005)
- Changing the default `planning_switch_margin` value (that's a CognitiveProfile default in worldwake-core)
- Performance optimization beyond what the margin provides (future work)
- Adjusting non-golden test helpers (already done in S74INTCOMM-001)

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-ai -- golden_` — all golden tests pass
2. `cargo test -p worldwake-ai -- golden_merchant_selling` — margin-sensitive merchant goldens intact after S74INTCOMM-005 lands
3. Existing suite: `cargo test --workspace`

### Invariants

1. All golden tests produce identical behavioral outcomes to pre-S74 (or explicitly documented better outcomes where the margin prevents wasteful replanning)
2. Soak seeds 0-4 show improvement in full GOAP search count (no seed-specific regression)
3. Agents still switch goals when a genuinely higher-priority goal emerges — the margin is not infinite
4. `cargo clippy --workspace --all-targets -- -D warnings` passes

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_merchant_selling.rs` — adjust `planning_switch_margin` only if the remaining failing merchant scenario is genuinely margin-driven. Document rationale.
2. Other golden tests — adjust only if failures occur.

### Commands

1. `cargo test -p worldwake-ai -- golden_` — all golden tests
2. `cargo test -p worldwake-ai -- golden_merchant_selling` — targeted merchant test
3. `cargo clippy --workspace --all-targets -- -D warnings` — lint verification
4. `cargo test --workspace` — full suite
