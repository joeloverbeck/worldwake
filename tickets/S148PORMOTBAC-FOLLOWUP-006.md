# S148PORMOTBAC-FOLLOWUP-006: Reassess survival-baseline observer convergence calibration

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes - likely `crates/worldwake-cli/tests/golden_observer_anomalies.rs`, `scenarios/survival-baseline.ron`, or the observer geographic-convergence detector
**Deps**: `archive/tickets/S148PORMOTBAC-FOLLOWUP-005.md`, `archive/specs/S148-portfolio-and-motive-backed-intentions.md`

## Problem

While closing `S148PORMOTBAC-FOLLOWUP-005`, the two ticket-owned observer anomaly fixtures passed, but the broader ignored observer-anomaly binary exposed a separate same-suite failure:

`cargo test --release -p worldwake-cli --test golden_observer_anomalies -- --ignored --test-threads=1`

`golden_observer_anomalies::convergence_smell_stays_absent_on_survival_baseline` expected 0 `GEOGRAPHIC_CONVERGENCE` anomalies for `scenarios/survival-baseline.ron` and now reports 3. The preserved observer report showed the three convergence windows at Riverside Camp:

1. Agent A + Agent B, ticks 474-1439.
2. Agent A + Agent B + Agent C, ticks 521-1105.
3. Agent A + Agent C, ticks 907-1115.

This failure is not caused by the current `S148PORMOTBAC-FOLLOWUP-005` edits, which only recalibrated the wash-gap fixture count and made the recipe-monoculture fixture's alternative FieldPlot observable. It still needs a foundation-aligned reassessment before changing the survival-baseline fixture, the observer assertion, or the detector.

## Assumption Reassessment (2026-05-18)

1. The motivating report is an observer calibration failure, not a direct S148 self-care probe/control failure.
2. The failing test uses `scenarios/survival-baseline.ron`, not an observer-only fixture under `crates/worldwake-cli/tests/fixtures/observer_anomalies/`.
3. The current report still includes other baseline observer noise (`STUCK_AGENT` for Agent A), so every anomaly in the report should be classified against the test's intended contract rather than treated as automatically owned by this ticket.
4. Per `docs/FOUNDATIONS.md`, the repair must preserve FND-1 local causality, FND-7 locality, FND-8 action duration/occupancy, and FND-11 physical dampeners. Do not hide convergence with a threshold-only detector tweak unless the detector threshold itself is proved stale.

## Architecture Check

1. If survival-baseline agents lawfully converge at Riverside Camp under current survival behavior, update the observer calibration test or fixture expectation to stop treating that specific baseline convergence as a detector false positive.
2. If the convergence detector is over-counting overlapping windows or reporting the same causal convergence repeatedly, fix the detector and prove the dedup/window contract.
3. If survival-baseline now routes agents into an unintended shared-place loop, repair the scenario or AI behavior at the earliest concrete cause instead of masking the observer report.

## Verification Layers

1. Observer anomaly section for `survival-baseline.ron` -> exact reported `GEOGRAPHIC_CONVERGENCE` windows and involved agents.
2. Per-agent location timeline / decision summary -> whether convergence is lawful survival behavior, fixture drift, detector duplication, or AI regression.
3. Focused ignored observer test -> `convergence_smell_stays_absent_on_survival_baseline` or its reassessed replacement.

## What to Change

### 1. Reassess the live survival-baseline observer report

Preserve a fresh observer report outside the test tempfile and compare Section 4 anomaly flags, per-agent location timelines, and decision summaries for the Riverside Camp windows.

### 2. Repair the correct owner

Choose the narrowest foundation-aligned owner:

- fixture/test truthing if the baseline now lawfully converges under current survival behavior
- observer detector repair if the same convergence is duplicated or over-classified
- scenario or AI repair if the convergence exposes an unintended survival-baseline behavior regression

## Files to Touch

- `crates/worldwake-cli/tests/golden_observer_anomalies.rs`
- `scenarios/survival-baseline.ron`, only if live reassessment proves fixture ownership
- `crates/worldwake-cli/src/bin/observer.rs`, only if live reassessment proves detector ownership

## Out of Scope

- Reopening the `S148PORMOTBAC-FOLLOWUP-005` wash-gap or recipe-monoculture fixture fixes.
- Threshold-only changes that hide a real convergence without explaining the causal branch.
- Reintroducing pre-003 self-care probe escapes or loose-lot control bypasses.

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test --release -p worldwake-cli --test golden_observer_anomalies convergence_smell_stays_absent_on_survival_baseline -- --ignored --test-threads=1`
2. Any focused unit or runtime test added for the exact repaired owner.

### Invariants

1. The observer convergence expectation is backed by concrete location occupancy and detector state.
2. The repair does not hide lawful local convergence with plot logic or unexplained thresholds.
3. The repair does not undo the evidence-grounded self-care acquisition boundary from `S148PORMOTBAC-FOLLOWUP-003`.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-cli/tests/golden_observer_anomalies.rs` - update or preserve the convergence baseline assertion according to reassessment.

### Commands

1. `cargo test --release -p worldwake-cli --test golden_observer_anomalies convergence_smell_stays_absent_on_survival_baseline -- --ignored --test-threads=1`
2. `cargo test --release -p worldwake-cli --test golden_observer_anomalies -- --ignored --test-threads=1`
