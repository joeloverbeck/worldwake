# S74INTCOMM-003: Golden test validation and soak regression check

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: None — test-only adjustments
**Deps**: S74INTCOMM-002

## Problem

With the margin-based plan continuation in place (S74INTCOMM-002), golden tests that depend on rapid goal switching under need pressure may fail if the default `planning_switch_margin = 150` suppresses the switch. Specifically, `golden_merchant_selling` (`loose_home_stock_is_staged_before_sell_goal_settles`) depends on the agent switching goals when needs shift priorities. This ticket validates all golden tests and adjusts margin values or scenario parameters where needed, then confirms soak-seed-perf regression bounds.

## Assumption Reassessment (2026-04-08)

1. `golden_merchant_selling` test at `crates/worldwake-ai/tests/golden_merchant_selling.rs`. The test exercises the seller-side lifecycle including stock staging and goal settlement under need pressure. If the priority shift between the merchant's current goal and the need-driven goal is less than 150 permille of `motive_score`, the margin will suppress the switch and the test may fail.
2. Soak-seed-perf campaign at `campaigns/soak-seed-perf/program.md` with binary harness at `crates/worldwake-ai/src/bin/soak_seed_perf.rs`. Seeds 0-4 rotation. The spec's motivation is reducing full planning passes from ~10,000 per agent over 10,000 ticks. The margin should significantly reduce this without introducing seed-specific regressions.
3. All golden tests in `crates/worldwake-ai/tests/golden_*.rs` must pass. The margin change is global (every agent with a CognitiveProfile gets it), so all golden tests are potentially affected, not just `golden_merchant_selling`.
12. If `golden_merchant_selling` needs margin adjustment: the scenario isolates merchant selling lifecycle under need pressure. The intended branch is goal switching when needs drive a genuinely higher-priority goal. Setting a lower `planning_switch_margin` (e.g., 50) for the test agent preserves the rapid-switching behavior the test proves while keeping the margin mechanism active. Alternatively, strengthening the need pressure in the scenario so the priority delta exceeds 150 would test the margin bypass path naturally.

## Architecture Check

1. Adjusting per-agent `planning_switch_margin` in golden test scenarios is the cleanest approach — it exercises the per-agent configurability that P22 mandates while keeping the test's intended invariant intact.
2. No backwards-compatibility aliasing/shims introduced.

## Verification Layers

1. All golden tests pass with correct behavioral outcomes -> golden test suite pass (`cargo test -p worldwake-ai -- golden_`)
2. Soak seeds 0-4 show reduced full planning passes -> soak-seed-perf campaign metrics (per-agent-tick planning cost and full GOAP search count)
3. No seed-specific regression -> all 5 seeds pass soak validation bounds
4. `golden_merchant_selling` preserves goal-switching invariant -> decision trace shows the merchant switching goals when need-driven priority exceeds the margin

## What to Change

### 1. Run all golden tests and identify failures

Run `cargo test -p worldwake-ai -- golden_` and note any failures introduced by the margin change.

### 2. Adjust `golden_merchant_selling` if needed

If the test fails because the margin suppresses a goal switch:

**Option A (preferred)**: Set a lower `planning_switch_margin` for the test's merchant agent in the scenario setup. This proves the agent CAN switch goals when the margin is low while the margin mechanism is still active.

**Option B**: Strengthen the need pressure in the scenario so the priority delta naturally exceeds 150, triggering the switch through the margin bypass path.

The choice depends on what the test is proving — document the rationale in a code comment.

### 3. Validate soak-seed-perf regression bounds

Run the soak-seed-perf binary with seeds 0-4 and verify:
- Full GOAP search count per agent is significantly reduced from the ~10,000 baseline
- No seed shows regression (more planning passes than baseline)
- Per-agent-tick planning cost does not exceed 5ms at tick 10,000 (rough bound — the spec cites 2-20ms per full pass, and the margin should eliminate most of them)

### 4. Adjust other golden tests if needed

If any other golden test fails, apply the same analysis: determine whether the test exercises rapid goal switching under need pressure, and adjust the test agent's `planning_switch_margin` accordingly.

## Files to Touch

- `crates/worldwake-ai/tests/golden_merchant_selling.rs` (modify — if margin adjustment needed)
- Other `crates/worldwake-ai/tests/golden_*.rs` files (modify — only if failures occur)

## Out of Scope

- Modifying the margin comparison logic itself (S74INTCOMM-002)
- Changing the default `planning_switch_margin` value (that's a CognitiveProfile default in worldwake-core)
- Performance optimization beyond what the margin provides (future work)
- Adjusting non-golden test helpers (already done in S74INTCOMM-001)

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-ai -- golden_` — all golden tests pass
2. `cargo test -p worldwake-ai -- golden_merchant_selling` — merchant selling lifecycle intact
3. Existing suite: `cargo test --workspace`

### Invariants

1. All golden tests produce identical behavioral outcomes to pre-S74 (or explicitly documented better outcomes where the margin prevents wasteful replanning)
2. Soak seeds 0-4 show improvement in full GOAP search count (no seed-specific regression)
3. Agents still switch goals when a genuinely higher-priority goal emerges — the margin is not infinite
4. `cargo clippy --workspace --all-targets -- -D warnings` passes

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_merchant_selling.rs` — adjust `planning_switch_margin` if the default margin suppresses the intended goal switch. Document rationale.
2. Other golden tests — adjust only if failures occur.

### Commands

1. `cargo test -p worldwake-ai -- golden_` — all golden tests
2. `cargo test -p worldwake-ai -- golden_merchant_selling` — targeted merchant test
3. `cargo clippy --workspace --all-targets -- -D warnings` — lint verification
4. `cargo test --workspace` — full suite
