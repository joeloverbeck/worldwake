# S148PORMOTBAC-FOLLOWUP-005: Reassess observer anomaly fixtures after self-care acquisition tightening

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes - likely `crates/worldwake-cli/tests/golden_observer_anomalies.rs`, observer fixture `.ron` files, or the observer anomaly detector
**Deps**: `archive/tickets/S148PORMOTBAC-FOLLOWUP-003.md`, `archive/specs/S148-portfolio-and-motive-backed-intentions.md`

## Problem

`S148PORMOTBAC-FOLLOWUP-003` tightened self-care acquisition admission and loose-lot control handling. The AI survival goldens stayed green, but two CLI observer anomaly calibration tests changed shape under the final implementation:

1. `golden_observer_anomalies::maintenance_starvation_fires_on_wash_gap` expected 3 `MAINTENANCE_STARVATION` anomalies and now reports 2.
2. `golden_observer_anomalies::recipe_monoculture_fires_on_single_food_dependency` expected 1 `RECIPE_MONOCULTURE` anomaly and now reports 0.

Clean `HEAD` before `S148PORMOTBAC-FOLLOWUP-003` passed both tests, so this is not an inherited red gate. The observed change is plausibly a legitimate downstream behavior change from admitting only planner-resolvable self-care acquisition paths and rejecting legally uncontrollable loose-lot support, but the observer fixtures still need a foundation-aligned reassessment before their expected anomaly counts or setup can be changed.

## Assumption Reassessment (2026-05-18)

1. The observer fixtures are calibration scenarios, not the core S148 survival proof surface.
2. The wash-gap report changed because one of Noor's previous maintenance-starvation windows no longer forms under the new self-care behavior.
3. The monoculture fixture no longer emits the expected recipe-monoculture anomaly under the new acquisition/search path.
4. Per `docs/FOUNDATIONS.md`, the repair must preserve FND-1 local causality, FND-8 explicit contention/duration, FND-14/FND-14A belief locality, and FND-23 planner formalism. Do not force an anomaly with hidden plot logic or threshold-only masking.

## Architecture Check

1. If the new behavior is lawful and the old anomaly expectation depended on stale planner failure, update the fixture or assertion to prove the intended observer contract.
2. If the new behavior hides a real anomaly that should still be detected, fix the observer detector or scenario instrumentation.
3. If the new behavior is an unintended AI regression, repair the AI behavior without reopening the pressure-only probe escape or rejected-slot bypass removed by `S148PORMOTBAC-FOLLOWUP-003`.

## Verification Layers

1. Observer report comparison for the current and pre-003 behavior.
2. Decision/action trace for the missing wash-gap maintenance-starvation window.
3. Decision/action trace for the missing recipe-monoculture anomaly.
4. Focused observer tests after the chosen repair.

## What to Change

### 1. Reassess the live observer reports

Preserve observer reports outside the test tempfile and compare anomaly sections, decision traces, and final state snapshots.

### 2. Repair the correct owner

Choose the narrowest foundation-aligned owner:

- fixture truthing if the calibration setup no longer creates the intended anomaly
- observer detector repair if the anomaly is present but no longer detected
- AI repair if the new behavior is not lawful under the current belief/control contract

## Files to Touch

- `crates/worldwake-cli/tests/golden_observer_anomalies.rs`
- `crates/worldwake-cli/tests/fixtures/observer_anomalies/*.ron`
- observer detector code, only if live reassessment proves detector ownership
- AI code, only if live reassessment proves the new behavior is unlawful

## Out of Scope

- Reintroducing pressure-only self-care acquisition probe admission.
- Reintroducing rejected-slot planning bypasses.
- Weakening the S148 survival baseline, ask/consult, scattered, or patrol goldens.
- Threshold-only changes that hide a real anomaly without explaining the causal branch.

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test --release -p worldwake-cli --test golden_observer_anomalies maintenance_starvation_fires_on_wash_gap -- --ignored --test-threads=1`
2. `cargo test --release -p worldwake-cli --test golden_observer_anomalies recipe_monoculture_fires_on_single_food_dependency -- --ignored --test-threads=1`
3. Any focused unit or runtime test added for the exact repaired owner.

### Invariants

1. Observer anomaly expectations are backed by concrete action, need, recipe, or detector state.
2. The fixture does not author an outcome; it authors only nouns, laws, institutions, and initial conditions.
3. The repair does not undo the evidence-grounded self-care acquisition boundary from `S148PORMOTBAC-FOLLOWUP-003`.

## Test Plan

### Commands

1. `cargo test --release -p worldwake-cli --test golden_observer_anomalies maintenance_starvation_fires_on_wash_gap -- --ignored --test-threads=1`
2. `cargo test --release -p worldwake-cli --test golden_observer_anomalies recipe_monoculture_fires_on_single_food_dependency -- --ignored --test-threads=1`
