# S148PORMOTBAC-FOLLOWUP-006: Reassess survival-baseline observer convergence calibration

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes - `crates/worldwake-cli/src/bin/observer.rs`
**Deps**: `archive/tickets/S148PORMOTBAC-FOLLOWUP-005.md`, `archive/specs/S148-portfolio-and-motive-backed-intentions.md`

## Problem

While closing `S148PORMOTBAC-FOLLOWUP-005`, the two ticket-owned observer anomaly fixtures passed, but the broader ignored observer-anomaly binary exposed a separate same-suite failure:

`cargo test --release -p worldwake-cli --test golden_observer_anomalies -- --ignored --test-threads=1`

Before this ticket, `golden_observer_anomalies::convergence_smell_stays_absent_on_survival_baseline` expected 0 `GEOGRAPHIC_CONVERGENCE` anomalies for `scenarios/survival-baseline.ron` and reported 3. The preserved observer report showed the three convergence windows at Riverside Camp:

1. Agent A + Agent B, ticks 474-1439.
2. Agent A + Agent B + Agent C, ticks 521-1105.
3. Agent A + Agent C, ticks 907-1115.

This failure was not caused by the `S148PORMOTBAC-FOLLOWUP-005` edits, which only recalibrated the wash-gap fixture count and made the recipe-monoculture fixture's alternative FieldPlot observable. This ticket reassessed the failure before changing the survival-baseline fixture, the observer assertion, or the detector.

## Assumption Reassessment (2026-05-18)

1. The motivating report is an observer calibration failure, not a direct S148 self-care probe/control failure.
2. The failing test uses `scenarios/survival-baseline.ron`, not an observer-only fixture under `crates/worldwake-cli/tests/fixtures/observer_anomalies/`.
3. The current report still includes other baseline observer noise (`STUCK_AGENT` for Agent A), so every anomaly in the report should be classified against the test's intended contract rather than treated as automatically owned by this ticket.
4. Per `docs/FOUNDATIONS.md`, the repair preserved FND-1 local causality, FND-7 locality, FND-8 action duration/occupancy, and FND-11 physical dampeners. It corrected concrete detector classification instead of hiding convergence with a threshold-only detector tweak.

## Reassessment Result

1. The fresh observer report was preserved at `/tmp/worldwake-s148-006-observer-report.md`.
2. The live survival-baseline run kept all agents healthy and cycling through lawful self-care actions; the geographic-convergence windows were a detector classification false positive, not a survival-baseline scenario loop or AI regression.
3. The detector classified Riverside Camp as a bundled survival-support hub because water was treated as food through `consumable_profile`, and because transient item lots at a place could be counted as structural food support.
4. The detector owner was the correct repair surface. `scenarios/survival-baseline.ron` and the observer assertion stayed unchanged.

## Verified Layers

1. Observer anomaly section for `survival-baseline.ron` before the fix showed the exact reported `GEOGRAPHIC_CONVERGENCE` windows and involved agents.
2. Per-agent summaries showed lawful survival behavior with no critical needs, continued eat/drink/sleep/relieve/wash actions, and no evidence that the scenario had become an unintended shared-place loop.
3. Focused detector tests prove the support classifier still fires for a structural bundled hub, still suppresses structural split-support convergence, and ignores carried food when classifying a place.
4. The focused ignored observer test proves `convergence_smell_stays_absent_on_survival_baseline` is green with the detector repair.

## Landed Changes

### 1. Reassessed the live survival-baseline observer report

Preserved a fresh observer report outside the test tempfile under `/tmp/worldwake-s148-006-observer-report.md` while reassessing Section 4 anomaly flags, per-agent summaries, and Riverside Camp support classification.

### 2. Repaired the observer detector owner

Updated the geographic-convergence detector's local survival-support summary so food support is classified through `TradeCategory::Food` instead of any consumable profile. This keeps water and food as distinct concrete support axes.

Also narrowed food support to structural `ResourceSource` entities instead of transient item lots at a place. A carried or leftover apple no longer turns Riverside Camp into a false bundled support hub.

### 3. Added focused detector regression coverage

Updated the bin-local geographic-convergence tests so they cover structural split-support, structural bundled-support, and the carried-food false-positive case.

## Landed Files

- `crates/worldwake-cli/src/bin/observer.rs`
- `archive/tickets/S148PORMOTBAC-FOLLOWUP-006.md`

## Out of Scope

- Reopening the `S148PORMOTBAC-FOLLOWUP-005` wash-gap or recipe-monoculture fixture fixes.
- Threshold-only changes that hide a real convergence without explaining the causal branch.
- Reintroducing pre-003 self-care probe escapes or loose-lot control bypasses.

## Acceptance Result

### Focused Tests

1. `cargo test --release -p worldwake-cli --test golden_observer_anomalies convergence_smell_stays_absent_on_survival_baseline -- --ignored --test-threads=1`
2. `cargo test -p worldwake-cli --bin observer geographic_convergence`
3. `cargo test -p worldwake-cli --bin observer`

### Invariant Results

1. The observer convergence expectation is backed by concrete location occupancy and detector state.
2. The repair does not hide lawful local convergence with plot logic or unexplained thresholds; it corrects the detector's food-vs-water and structural-support classification.
3. The repair does not undo the evidence-grounded self-care acquisition boundary from `S148PORMOTBAC-FOLLOWUP-003`.

## Test Plan Result

### Modified Tests

1. `crates/worldwake-cli/src/bin/observer.rs` - added/updated geographic-convergence detector unit coverage.
2. `crates/worldwake-cli/tests/golden_observer_anomalies.rs` stayed unchanged; the existing ignored survival-baseline assertion became true after the detector repair.
3. `scenarios/survival-baseline.ron` stayed unchanged; the scenario behavior was lawful.

### Commands Run

1. `cargo test --release -p worldwake-cli --test golden_observer_anomalies convergence_smell_stays_absent_on_survival_baseline -- --ignored --test-threads=1`
2. `cargo test --release -p worldwake-cli --test golden_observer_anomalies -- --ignored --test-threads=1`
3. `cargo test -p worldwake-cli --bin observer geographic_convergence`
4. `cargo test -p worldwake-cli --bin observer`

## Outcome

Completed on 2026-05-18.

Changed:

- Corrected the geographic-convergence detector so water no longer counts as food support.
- Corrected the detector's food-support axis to use structural resource sources rather than transient or carried item lots.
- Added focused observer-bin regression coverage for split support, bundled support, and carried-food false positives.

No scenario fixture, observer integration assertion, or AI behavior changed. The survival-baseline run still reports the separate `STUCK_AGENT` observer noise for Agent A, but the ticket-owned `GEOGRAPHIC_CONVERGENCE` count is now zero.

Deviations:

- The repair landed in `crates/worldwake-cli/src/bin/observer.rs`, not in `scenarios/survival-baseline.ron` or `crates/worldwake-cli/tests/golden_observer_anomalies.rs`, because live reassessment proved detector over-classification.
- The full ignored observer-anomaly binary is now green, so no successor ticket was created.

## Verification Result

- Passed `cargo test -p worldwake-cli --bin observer geographic_convergence`
- Passed `cargo test --release -p worldwake-cli --test golden_observer_anomalies convergence_smell_stays_absent_on_survival_baseline -- --ignored --test-threads=1`
- Passed `cargo test --release -p worldwake-cli --test golden_observer_anomalies -- --ignored --test-threads=1`
- Passed `cargo test -p worldwake-cli --bin observer`
