# S31-007: Golden Tests for Over/Under-Invalidation and Save/Load Parity

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: None — tests only
**Deps**: S31-004, S31-005, S31-006

## Problem

The S31 spec requires golden test proof that the new condition-based invalidation correctly avoids over-invalidation (irrelevant changes don't trigger re-search) and under-invalidation (relevant changes do trigger re-search). It also requires save/load parity proof that the current S31 exhaustion runtime shape survives round-trip under the live save format.

## Assumption Reassessment (2026-03-27)

1. `golden_save_load_round_trip_under_ai` exists in `crates/worldwake-ai/tests/` and tests save/load parity for the AI runtime including `exhaustion_cache`.
2. `golden_wash_action` exists and tests the Wash goal behavior — previously broke in exp-005 with indefinite caching.
3. `golden_three_way_need_competition` exists and tests need priority competition — previously broke in exp-005.
4. The spec calls for a golden test showing bread consumption does NOT clear Apple acquisition cache.
5. The spec calls for a golden test showing dirtiness crossing threshold DOES clear Wash exhaustion cache.
6. Decision traces (`h.driver.enable_tracing()`) and action traces (`h.enable_action_tracing()`) are available for debugging golden test failures.
7. Golden tests live in `crates/worldwake-ai/tests/` as integration tests.
8. Reassessment after S31-002: golden coverage should explicitly guard against accidental unconditional invalidation of facility-tagged and blocker-tagged goals. A cache that clears `Wash` or `ProduceCommodity` every tick would still satisfy some naive "eventually replans" assertions while violating the architecture.

## Architecture Check

1. Test-only changes. No production code modified.
2. Golden tests use the full simulation harness — they test the integrated behavior of S31-001 through S31-006.

## Verification Layers

1. No over-invalidation -> golden test with decision trace showing Apple cache retained after bread consumption
2. No under-invalidation -> golden test with decision trace showing Wash cache cleared after dirtiness crosses threshold
3. Save/load parity -> `golden_save_load_round_trip_under_ai` passes without driver reset
4. Facility-tagged goals stay exhausted until an actual facility dirty event occurs -> golden or focused integration test
5. Existing golden tests pass -> `cargo test -p worldwake-ai`

## What to Change

### 1. Add `golden_no_over_invalidation_commodity` golden test

Setup: Agent with exhausted `AcquireCommodity(Apple)` and bread in inventory.
Action: Agent eats bread (commodity change: bread consumed).
Assert: Apple acquisition goal remains in exhaustion cache (not re-searched). Use decision trace to verify no Apple-related search occurred.

### 2. Add `golden_no_under_invalidation_wash` golden test

Setup: Agent with exhausted `Wash` goal and low dirtiness.
Action: Run ticks until dirtiness crosses threshold (increases by 100+ permille).
Assert: Wash goal is removed from exhaustion cache and re-searched. Use decision trace to verify Wash search occurred.

### 2b. Add facility-signal retention proof

Setup: Agent with exhausted facility-tagged goal such as `Wash` or `ProduceCommodity`, with no facility-access change across the observed ticks.
Action: Advance at least one planning cycle without changing facility access.
Assert: The goal remains exhausted and is not re-searched merely because it carries `FacilitiesChanged`.

### 3. Verify existing golden tests pass

- `golden_save_load_round_trip_under_ai` — conditions survive serialization round-trip
- `golden_wash_action` — the test that broke in exp-005
- `golden_three_way_need_competition` — the test that broke in exp-005
- All other golden tests in `crates/worldwake-ai/tests/`

### 4. Keep save/load proof aligned with the current format only

Do not add old-format or empty-condition compatibility tests here. S31 should validate the current exhaustion runtime contract after S31-005 removes the stale serde-default fallback, not preserve a superseded shape.

## Files to Touch

- `crates/worldwake-ai/tests/golden_exhaustion.rs` (new — or add to existing golden test file)

## Out of Scope

- Production code changes (all done in S31-001 through S31-006)
- Profiling / performance benchmarks (documented in spec but not blocking acceptance)
- Changes to any non-test files

## Acceptance Criteria

### Tests That Must Pass

1. `golden_no_over_invalidation_commodity` — bread consumption does not clear Apple acquisition cache
2. `golden_no_under_invalidation_wash` — dirtiness threshold crossing clears Wash exhaustion cache
3. `golden_save_load_round_trip_under_ai` — passes without driver reset (S30 parity preserved)
4. `golden_wash_action` — passes (the test that broke in exp-005)
5. `golden_three_way_need_competition` — passes (the test that broke in exp-005)
6. Facility-tagged exhaustion is retained when no facility dirty event occurred
7. Full suite: `cargo test --workspace`

### Invariants

1. No over-invalidation: irrelevant commodity changes do not trigger re-searches (spec AC 2)
2. No under-invalidation: needs-driven goals re-search when relevant need crosses threshold (spec AC 3)
3. Save/load parity: exhaustion conditions survive serialization round-trip (spec AC 7)
4. Facility-tagged goals do not self-invalidate without an observed facility access change
5. All existing golden tests continue to pass

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_exhaustion.rs` — `golden_no_over_invalidation_commodity`
2. `crates/worldwake-ai/tests/golden_exhaustion.rs` — `golden_no_under_invalidation_wash`
3. `crates/worldwake-ai/tests/golden_exhaustion.rs` — facility-signal retention proof

### Commands

1. `cargo test -p worldwake-ai golden_exhaustion`
2. `cargo test -p worldwake-ai golden_save_load`
3. `cargo test -p worldwake-ai golden_wash`
4. `cargo test -p worldwake-ai golden_three_way`
5. `cargo clippy --workspace && cargo test --workspace`
