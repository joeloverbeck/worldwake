# GOLDE2E-010: Seed Sensitivity (Different Seeds, Different Outcomes)

**Status**: ✅ COMPLETED
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: None expected
**Deps**: None (deterministic replay exists from E08)

## Problem

The existing golden determinism coverage proves same-seed determinism well, but the original ticket assumed the current Scenario 6 setup should also diverge under different seeds. That assumption does not hold in the current architecture: the bread/travel/harvest scenario is a valid fully deterministic chain even when seeded differently, because it does not rely on stochastic action resolution.

If we want a durable seed-sensitivity golden test, it must target a path that genuinely consumes `DeterministicRng` in production code. In the current stack, the cleanest golden candidate is living combat, where attack/guard rolls and body-part selection are resolved through RNG in `worldwake-systems/src/combat.rs`.

## Report Reference

Backlog item **P17** in `reports/golden-e2e-coverage-analysis.md` (Tier 2, composite score 3).

## Assumption Reassessment (2026-03-13)

1. `DeterministicRng` wraps `ChaCha8Rng` in `worldwake-sim/src/deterministic_rng.rs`.
2. `StateHash` comparison is available through `hash_world()` and `hash_event_log()` in the golden tests.
3. `golden_deterministic_replay_fidelity` is still the correct same-seed determinism proof, but it is not a valid basis for a different-seed divergence assertion because its scenario is not architecturally required to consume RNG.
4. The living-combat golden path already exercises real RNG through combat resolution, making it the correct place to prove seed sensitivity at the e2e level.
5. A single arbitrary seed pair is weaker than it first appears because different seeds can still legitimately converge to the same valid combat transcript. The test should therefore compare a small fixed seed set and assert that the scenario yields more than one valid outcome across that set.
6. A bare `hash_a != hash_b` assertion is too weak by itself. The test should also preserve scenario-level invariants so it proves meaningful stochastic divergence rather than incidental bookkeeping drift.

## Architecture Check

1. The original proposal was architecturally mis-scoped because it tried to prove RNG influence on a scenario that is allowed to be seed-insensitive.
2. The correct architecture is to prove seed sensitivity only on a scenario whose domain semantics are stochastic in the production engine.
3. No backward-compatibility alias should be introduced between deterministic-only and stochastic golden scenarios. Each test should state what kind of guarantee it provides.

## Engine-First Mandate

If implementing this e2e suite reveals that a genuinely stochastic golden scenario still produces identical outcomes across clearly different seeds, treat that as a real architectural bug in RNG plumbing or stochastic resolution. Do not paper over it with hash-only assertions or by switching to a noisier scenario; investigate the production path and document any engine changes in the outcome.

## What to Change

### 1. Add a stochastic seed-sensitivity golden test

**Preferred location**: `golden_combat.rs`

**Setup**: Reuse the existing living-combat golden scenario, which already goes through the real AI loop and real combat resolution.

**Assertions**:
- Run the same living-combat scenario across a small fixed set of distinct seeds.
- Each run must still satisfy the existing living-combat invariants and produce a `(world, log)` hash pair.
- The set of observed hash pairs must contain more than one distinct outcome.
- Both runs still satisfy the same combat scenario invariants already expected by the current golden helper: attack starts, combat events exist, defender is wounded, coin conservation holds, and both actors survive.

### 2. Keep same-seed determinism separate

Do not weaken or replace `golden_deterministic_replay_fidelity`. It proves a different property:
- same seed + same inputs => identical outcome

The new test should complement that by proving:
- different seed + same stochastic setup => at least one different valid outcome

## Files to Touch

- `crates/worldwake-ai/tests/golden_combat.rs` (modify)
- `crates/worldwake-ai/tests/golden_determinism.rs` (leave unchanged unless a small shared helper becomes clearly justified)
- `crates/worldwake-ai/tests/golden_harness/mod.rs` (modify only if a narrowly reusable helper is genuinely needed)

## Out of Scope

- Statistical analysis of RNG distribution
- Requiring every golden scenario to diverge under different seeds
- Fuzzing with many seeds
- Replacing deterministic scenarios with stochastic ones

## Acceptance Criteria

### Tests That Must Pass

1. `golden_seed_sensitivity_living_combat_different_outcomes` — a fixed set of distinct seeds produces more than one valid combat outcome
2. Existing related suite: `cargo test -p worldwake-ai golden_combat`
3. Existing same-seed determinism suite remains green: `cargo test -p worldwake-ai golden_deterministic_replay_fidelity -- --exact`
4. Full workspace: `cargo test --workspace`
5. Lint: `cargo clippy --workspace`

### Invariants

1. Both runs individually satisfy all scenario invariants for living combat.
2. The seeds are truly different.
3. Same-seed replay determinism for the combat scenario remains intact.

## Post-Implementation

After implementing this suite, update `reports/golden-e2e-coverage-analysis.md`:
- Add the new seed-sensitivity coverage note to the combat/determinism discussion in Part 1.
- Remove P17 from the Part 3 backlog.
- Update Part 4 summary statistics if the proven-test count changes.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_combat.rs::golden_seed_sensitivity_living_combat_different_outcomes` — proves RNG influence on a genuinely stochastic golden path

### Commands

1. `cargo test -p worldwake-ai golden_seed_sensitivity`
2. `cargo test -p worldwake-ai golden_combat`
3. `cargo test --workspace`
4. `cargo clippy --workspace`

## Outcome

- **Completion date**: 2026-03-13
- **What actually changed**:
  - Added `golden_seed_sensitivity_living_combat_different_outcomes` in `crates/worldwake-ai/tests/golden_combat.rs`.
  - Updated `reports/golden-e2e-coverage-analysis.md` to record the new combat seed-sensitivity coverage, remove backlog item P17, and refresh test counts.
- **Deviations from original plan**:
  - Did not implement the test in `golden_determinism.rs`.
  - Did not assert divergence on the original Scenario 6 bread/travel/harvest setup, because that scenario is not architecturally required to consume RNG.
  - Did not rely on a single seed pair. Initial implementation showed distinct seeds can still converge to the same valid combat outcome, so the final test uses a small fixed seed set and asserts that the set yields more than one valid outcome.
- **Verification results**:
  - `cargo test -p worldwake-ai golden_seed_sensitivity_living_combat_different_outcomes -- --exact`
  - `cargo test -p worldwake-ai golden_combat`
  - `cargo test -p worldwake-ai golden_deterministic_replay_fidelity -- --exact`
  - `cargo test --workspace`
  - `cargo clippy --workspace`
