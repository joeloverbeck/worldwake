# S148PORMOTBAC-FOLLOWUP-005: Reassess observer anomaly fixtures after self-care acquisition tightening

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes - `crates/worldwake-cli/tests/golden_observer_anomalies.rs`, `crates/worldwake-cli/tests/fixtures/observer_anomalies/recipe_monoculture_apples_vs_grain.ron`
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

## Verified Layers

1. Observer report comparison showed `maintenance_starvation_wash_gap.ron` still produces concrete dirtiness starvation, but the detector now reports one strongest `MAINTENANCE_STARVATION` anomaly per affected agent (`Mira`, `Noor`) rather than the stale count of 3.
2. Observer report comparison showed `recipe_monoculture_apples_vs_grain.ron` still produces the intended recipe-monoculture behavior for Agent A (many apple harvest commits and zero grain commits), but the fixture's alternative `FieldPlot` was not retained in Agent A's belief store under the old observation budget, so the detector correctly suppressed the anomaly.
3. The focused ignored observer tests prove the recalibrated wash-gap count and the repaired recipe-monoculture fixture.
4. A broader ignored observer-anomaly run exposed a separate `survival-baseline.ron` geographic-convergence calibration failure, now tracked by `tickets/S148PORMOTBAC-FOLLOWUP-006.md`.

## Landed Changes

### 1. Reassessed the live observer reports

Preserved observer reports outside the test tempfile under `/tmp/worldwake-s148-005/` while reassessing the two ticket-owned fixtures.

### 2. Truth-adjusted the wash-gap calibration

Updated `maintenance_starvation_fires_on_wash_gap` to expect the two concrete `MAINTENANCE_STARVATION` anomalies now emitted by the live detector: one for `Mira` and one for `Noor`.

### 3. Repaired the recipe-monoculture fixture owner

Updated `recipe_monoculture_apples_vs_grain.ron` so Agent A can lawfully retain the alternative `FieldPlot` evidence:

- added an explicit Grain resource source on `Scout FieldPlot`
- raised Agent A's observation budget to 240, within the live `PerceptionProfile` bounds, so co-located facilities survive the budgeted observation pass despite the fixture's many apple lots

## Landed Files

- `crates/worldwake-cli/tests/golden_observer_anomalies.rs`
- `crates/worldwake-cli/tests/fixtures/observer_anomalies/recipe_monoculture_apples_vs_grain.ron`
- `tickets/S148PORMOTBAC-FOLLOWUP-006.md`

## Out of Scope

- Reintroducing pressure-only self-care acquisition probe admission.
- Reintroducing rejected-slot planning bypasses.
- Weakening the S148 survival baseline, ask/consult, scattered, or patrol goldens.
- Threshold-only changes that hide a real anomaly without explaining the causal branch.
- The separate survival-baseline geographic-convergence calibration failure exposed by the broader ignored observer suite; follow-up owner is `tickets/S148PORMOTBAC-FOLLOWUP-006.md`.

## Acceptance Result

### Focused Tests

1. `cargo test --release -p worldwake-cli --test golden_observer_anomalies maintenance_starvation_fires_on_wash_gap -- --ignored --test-threads=1`
2. `cargo test --release -p worldwake-cli --test golden_observer_anomalies recipe_monoculture_fires_on_single_food_dependency -- --ignored --test-threads=1`
3. No focused unit/runtime test was added because the observer detector code stayed unchanged; the repaired owner was fixture/test calibration.

### Invariant Results

1. Observer anomaly expectations are backed by concrete action, need, recipe, or detector state.
2. The fixture does not author an outcome; it authors only nouns, laws, institutions, and initial conditions.
3. The repair does not undo the evidence-grounded self-care acquisition boundary from `S148PORMOTBAC-FOLLOWUP-003`.

## Outcome

Completed on 2026-05-18.

Changed:

- Recalibrated the wash-gap observer assertion from 3 to 2 `MAINTENANCE_STARVATION` anomalies and asserted that both `Mira` and `Noor` remain represented.
- Made the recipe-monoculture fixture's alternative grain path observable through an explicit `Scout FieldPlot` Grain source and a bounded Agent A observation budget increase.
- Created `tickets/S148PORMOTBAC-FOLLOWUP-006.md` for the separate `survival-baseline.ron` geographic-convergence calibration failure found during broader same-suite verification.

No production AI or observer detector code changed. The live detector behavior was already honest for the two ticket-owned reports once the stale fixture/count assumptions were corrected.

Deviations:

- The wash-gap repair landed as fixture assertion truthing rather than detector or AI repair because the live report still contains concrete dirtiness starvation for both agents.
- The recipe-monoculture repair landed in fixture setup because the detector correctly requires final belief-store evidence of an alternative recipe facility before flagging monoculture.
- The full ignored observer-anomaly binary remains red on `convergence_smell_stays_absent_on_survival_baseline`; that broader calibration question is out of scope here and owned by `tickets/S148PORMOTBAC-FOLLOWUP-006.md`.

## Verification Result

- Passed `cargo test --release -p worldwake-cli --test golden_observer_anomalies maintenance_starvation_fires_on_wash_gap -- --ignored --test-threads=1`
- Passed `cargo test --release -p worldwake-cli --test golden_observer_anomalies recipe_monoculture_fires_on_single_food_dependency -- --ignored --test-threads=1`
