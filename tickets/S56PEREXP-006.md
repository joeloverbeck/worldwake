# S56PEREXP-006: Golden tests for context-modulated perception

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: None
**Deps**: S56PEREXP-004, S56PEREXP-005

## Problem

S56 changes observation probability from a flat roll to a context-modulated function. Without golden E2E tests exercising concealment, fatigue, and attention cost modulation, regressions in the perception system would go undetected.

## Assumption Reassessment (2026-04-06)

1. Golden tests live in `crates/worldwake-ai/tests/golden_*.rs`. Naming convention: `golden_<domain>.rs`.
2. Golden tests use deterministic `ChaCha8Rng` with fixed seeds, `BTreeMap` for state, and scenario setup via `WorldTxn` or the scenario loading path.
3. `PerceptionProfile` must be set on observing agents (per CLAUDE.md: "Golden production tests require PerceptionProfile on agents that need to observe post-production output").
4. After S56PEREXP-004, agents with zero fatigue, no active action, and in places without `PlaceVisibilityProfile` behave identically to pre-S56 — existing golden tests should already pass.
5. New golden tests need scenarios with: (a) places that have `PlaceVisibilityProfile` set, (b) agents with elevated fatigue, (c) agents performing actions with non-zero `attention_cost`.

## Architecture Check

1. Golden tests verify emergent cross-system behavior (perception + needs + actions + topology). They exercise the full stack without mocking.
2. No backwards-compatibility shims — new test file alongside existing golden tests.

## Verification Layers

1. Concealment reduces observation rate -> golden test with high-concealment place
2. Fatigue reduces observation rate -> golden test with fatigued agent
3. Attention cost reduces observation rate -> golden test with combat action
4. Multiplicative stacking -> golden test combining all factors
5. Zero modifiers -> identical to baseline -> regression coverage from existing golden tests

## What to Change

### 1. Create golden test file

Create `crates/worldwake-ai/tests/golden_perception_exposure.rs` with the following test scenarios:

#### Scenario: Concealment Reduces Observation

Setup two identical agents (same `observation_fidelity: 800`) at two places:
- Open place: no `PlaceVisibilityProfile` (effective fidelity = 800)
- Concealed place: `PlaceVisibilityProfile { base_concealment: Permille(400) }` (effective = 480)

Place observable entities at both locations. Run perception ticks with deterministic RNG. Assert the agent in the concealed place observes fewer entities.

#### Scenario: Fatigue Reduces Observation

Setup an agent with `observation_fidelity: 1000`. Manually set `HomeostaticNeeds.fatigue` to 800 (penalty = 180, effective = 820). Place observable entities nearby. Run perception and verify reduced observation count compared to a rested agent.

#### Scenario: Attention Cost Reduces Observation

Setup an agent performing a combat action (`attention_cost: 400`, effective = 600) and an idle agent (effective = 1000), both with the same base fidelity. Verify the combat agent observes fewer entities.

#### Scenario: Multiplicative Stacking

An agent with fidelity 800, fatigue 700 (penalty 120), in combat (attention_cost 400), in a forest (concealment 400): effective = 253. Verify this severe reduction via deterministic observation counts.

### 2. Update golden test inventory

After writing tests, run `python3 scripts/golden_inventory.py --write --check-docs` to update docs.

## Files to Touch

- `crates/worldwake-ai/tests/golden_perception_exposure.rs` (new)

## Out of Scope

- Active concealment actions (hiding, disguise)
- Entity-level concealment (`entity_concealment` stays zero)
- Topology-based range modifiers

## Acceptance Criteria

### Tests That Must Pass

1. Concealment scenario: agent in concealed place observes fewer entities than agent in open place
2. Fatigue scenario: fatigued agent observes fewer entities than rested agent
3. Attention cost scenario: combat agent observes fewer entities than idle agent
4. Stacking scenario: all modifiers combined produce severely reduced observations
5. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. Deterministic: same seed produces same observation counts every run
2. `effective_fidelity <= base_fidelity` in all scenarios
3. Existing golden tests pass unchanged (zero modifiers = pre-S56 behavior)

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_perception_exposure.rs` — 4 golden scenarios testing concealment, fatigue, attention cost, and stacking

### Commands

1. `cargo test -p worldwake-ai -- golden_perception_exposure`
2. `cargo test -p worldwake-ai`
3. `cargo clippy --workspace --all-targets -- -D warnings`
