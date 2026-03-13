# GOLDE2E-010: Seed Sensitivity (Different Seeds, Different Outcomes)

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: None expected
**Deps**: None (deterministic replay exists from E08)

## Problem

Scenario 6 proves same-seed determinism (identical seeds → identical outcomes). A complementary test proving that different seeds produce different outcomes strengthens confidence that the RNG is actually influencing simulation behavior, not being ignored or short-circuited.

## Report Reference

Backlog item **P17** in `reports/golden-e2e-coverage-analysis.md` (Tier 2, composite score 3).

## Assumption Reassessment (2026-03-13)

1. `DeterministicRng` wraps `ChaCha8Rng` in `worldwake-sim/src/deterministic_rng.rs`.
2. `StateHash` comparison is available from `worldwake-core/src/canonical.rs`.
3. The golden determinism test (`golden_deterministic_replay_fidelity`) already runs two same-seed simulations.
4. A different-seed variant only needs to assert `hash_a != hash_b` (at least one hash differs).

## Architecture Check

1. Trivial addition — no new architecture needed.
2. Uses existing harness and hash infrastructure.

## Engine-First Mandate

If implementing this e2e suite reveals that the RNG is not actually influencing simulation outcomes (i.e., different seeds produce identical results), this indicates a fundamental architectural problem where randomness is being bypassed. Do NOT ignore this — investigate and fix the root cause to ensure the RNG correctly drives stochastic behavior throughout the simulation. Document any engine changes in the ticket outcome.

## What to Change

### 1. New golden test in `golden_determinism.rs`

**Setup**: Same scenario as `golden_deterministic_replay_fidelity` (two agents, food sources) but run twice with different seeds.

**Assertions**:
- Run 1 with seed A produces `StateHash` pair (world_a, log_a).
- Run 2 with seed B (B ≠ A) produces `StateHash` pair (world_b, log_b).
- At least one of `world_a != world_b` or `log_a != log_b` holds.

## Files to Touch

- `crates/worldwake-ai/tests/golden_determinism.rs` (modify)
- `crates/worldwake-ai/tests/golden_harness/mod.rs` (modify, if helpers needed)

## Out of Scope

- Statistical analysis of RNG distribution
- Proving specific stochastic behaviors
- Fuzzing with many seeds

## Acceptance Criteria

### Tests That Must Pass

1. `golden_seed_sensitivity_different_outcomes` — different seeds produce different state hashes
2. Existing suite: `cargo test -p worldwake-ai golden_`
3. Full workspace: `cargo test --workspace`

### Invariants

1. Both runs individually satisfy all simulation invariants (conservation, determinism within each run)
2. The seeds are truly different (not accidentally equal)

## Post-Implementation

After implementing this suite, update `reports/golden-e2e-coverage-analysis.md`:
- Add the new scenario to Part 1 (Proven Emergent Scenarios)
- Remove P17 from the Part 3 backlog
- Update Part 4 summary statistics

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_determinism.rs::golden_seed_sensitivity_different_outcomes` — proves RNG influence

### Commands

1. `cargo test -p worldwake-ai golden_seed_sensitivity`
2. `cargo test --workspace && cargo clippy --workspace`
