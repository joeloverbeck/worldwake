# S56PEREXP-006: Golden tests for context-modulated perception

**Status**: COMPLETED
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
6. Reassessment correction: the strongest current golden-proof surface for S56 modulation is witnessed-event perception tracing via `PerceptionTraceEvent.effective_fidelity`. Passive local observation does not yet expose an equally strong golden trace/debug carrier, so this ticket should stay on witnessed-event scenarios rather than broader local-observation count claims.
7. No existing `golden_*` file cleanly owns S56 perception-exposure coverage. A new `golden_perception_exposure.rs` file is appropriate.
8. `python3 scripts/golden_inventory.py --write --check-docs` updates both `docs/generated/golden-e2e-inventory.md` and `docs/generated/golden-scenario-map.md`; both generated artifacts are in scope once a new golden file lands.
9. The highest live source-declared golden scenario ID is `115`, so any new `// Scenario ...` metadata in this file should start at `116` or another non-colliding identifier.

## Architecture Check

1. Golden tests verify full-stack witnessed-event perception behavior (needs/combat requests + visibility + perception tracing) without mocking the modulation logic itself.
2. No backwards-compatibility shims — new test file alongside existing golden tests.

## Verification Layers

1. Concealment modulates witnessed-event fidelity -> perception trace `effective_fidelity` comparison for open vs concealed place
2. Fatigue modulates witnessed-event fidelity -> perception trace `effective_fidelity` comparison for rested vs fatigued observer
3. Attention cost modulates witnessed-event fidelity -> action trace proves active `defend`, perception trace proves lowered `effective_fidelity`
4. Multiplicative stacking composes correctly -> perception trace `effective_fidelity == 253`
5. Golden inventory/docs stay in sync -> `python3 scripts/golden_inventory.py --write --check-docs`

## What to Change

### 1. Create golden test file

Create `crates/worldwake-ai/tests/golden_perception_exposure.rs` with the following witnessed-event scenarios:

#### Scenario 116: Concealment Reduces Witnessed-Event Fidelity

Setup equivalent witnessed-event runs at the same outdoor place:
- Open place: no `PlaceVisibilityProfile` (effective fidelity = 800)
- Concealed place: `PlaceVisibilityProfile { base_concealment: Permille(400) }` (effective = 480)

Trigger a same-place `relieve_wilderness` event and assert the observer's `PerceptionTraceEvent.effective_fidelity` is lower in the concealed run (`480`) than the open run (`800`).

#### Scenario 117: Fatigue Reduces Witnessed-Event Fidelity

Setup an agent with `observation_fidelity: 1000`. Manually set `HomeostaticNeeds.fatigue` to 800 (penalty = 180, effective = 820). Place observable entities nearby. Run perception and verify reduced observation count compared to a rested agent.
Use a same-place witnessed event and assert the fatigued observer's `effective_fidelity` is `820`, lower than the rested baseline (`1000`).

#### Scenario 118: Attention Cost Reduces Witnessed-Event Fidelity

Setup an observer performing `defend` (`attention_cost: 400`, effective = 600) while another co-located actor triggers a same-place witnessed event. Use action trace to prove `defend` started and perception trace to prove the observer's `effective_fidelity` dropped to `600`.

#### Scenario 119: Multiplicative Stacking

An observer with fidelity `800`, fatigue `700` (penalty `120`), active `defend` (`attention_cost 400`), in a place with concealment `400`, witnesses a same-place event. Assert the perception trace records `effective_fidelity == 253`.

### 2. Update golden test inventory

After writing tests, run `python3 scripts/golden_inventory.py --write --check-docs` to update docs.

## Files to Touch

- `crates/worldwake-ai/tests/golden_perception_exposure.rs` (new)
- `docs/generated/golden-e2e-inventory.md` (modify, generated)
- `docs/generated/golden-scenario-map.md` (modify, generated)

## Out of Scope

- Active concealment actions (hiding, disguise)
- Entity-level concealment (`entity_concealment` stays zero)
- Topology-based range modifiers

## Acceptance Criteria

### Tests That Must Pass

1. Concealment scenario: perceived witnessed-event `effective_fidelity` is `480` in concealment and `800` in the open baseline
2. Fatigue scenario: fatigued observer's witnessed-event `effective_fidelity` is `820`, below the rested baseline
3. Attention-cost scenario: active `defend` is proven by action trace and the observer's witnessed-event `effective_fidelity` is `600`
4. Stacking scenario: witnessed-event `effective_fidelity` is `253`
5. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. Deterministic: same seed produces same observation counts every run
2. `effective_fidelity <= base_fidelity` in all scenarios
3. Existing golden tests pass unchanged (zero modifiers = pre-S56 behavior)

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_perception_exposure.rs` — 4 golden scenarios proving witnessed-event modulation via perception trace and action trace where needed

### Commands

1. `cargo test -p worldwake-ai --test golden_perception_exposure`
2. `python3 scripts/golden_inventory.py --write --check-docs`
3. `cargo test -p worldwake-ai`
4. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

Completed on 2026-04-06.

- Added `crates/worldwake-ai/tests/golden_perception_exposure.rs` with four new golden witnessed-event scenarios covering concealment, fatigue, attention-cost occupancy, and multiplicative stacking through `PerceptionTraceEvent.effective_fidelity`.
- Used human-driven `defend` and `relieve_wilderness` requests plus action/perception tracing to prove the live cross-system path instead of asserting against mocked helpers or broad observation-count side effects.
- Stabilized the new scenarios with zero-rate test metabolism so the asserted fidelity values reflect the intended S56 modifiers exactly rather than incidental per-tick drift during setup.
- Regenerated `docs/generated/golden-e2e-inventory.md` and `docs/generated/golden-scenario-map.md` so the new scenario IDs `116`-`119` are inventory-visible and documented.

## Verification Result

- Passed `cargo test -p worldwake-ai --test golden_perception_exposure`
- Passed `python3 scripts/golden_inventory.py --write --check-docs`
- Passed `cargo test -p worldwake-ai`
- Passed `cargo clippy --workspace --all-targets -- -D warnings`
