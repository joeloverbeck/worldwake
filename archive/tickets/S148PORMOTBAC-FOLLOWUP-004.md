# S148PORMOTBAC-FOLLOWUP-004: Truth survival-preferences familiar-source contract

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes - `crates/worldwake-ai/tests/golden_survival_preferences.rs`, generated golden docs, and scenario comment truthing only
**Deps**: `archive/tickets/S148PORMOTBAC-FOLLOWUP-003.md`, `archive/specs/S148-portfolio-and-motive-backed-intentions.md`

## Problem

`S148PORMOTBAC-FOLLOWUP-003` tightened self-care acquisition admission to concrete evidence and fixed the legal-control boundary for loose item lots. During broad regression, `golden_survival_preferences::survival_preferences_keeps_proactive_diversification_alive_under_survival` still failed because the golden retained a stale expectation that Familiar Orchard must become durable source-failure memory.

Live reassessment showed that the authored scenario's actual branch is different: Scout Ilen starts beside Familiar Orchard, proactively travels to Novel Grove before hunger requires harvesting, later uses Novel Grove as a real apple source, and never has a violated committed expectation for Familiar Orchard. Recording Familiar Orchard as failed would fabricate memory instead of preserving FND-1 local causality, FND-14/FND-14A belief boundaries, and FND-17 expectation violation.

## Assumption Reassessment (2026-05-18)

1. The motivating failing test is `cargo test --release -p worldwake-ai --test golden_survival_preferences survival_preferences_keeps_proactive_diversification_alive_under_survival -- --ignored --test-threads=1`.
2. The failing pre-fix run selected proactive Novel Grove at `Tick(1)`, reached it, and later harvested apples there; it never produced a Familiar Orchard `SourceReliability` record with `failed_attempts > 0`.
3. The live production source-reliability failure seam already requires a committed concrete-source expectation and then a local absent/depleted observation. Existing focused coverage in `agent_tick` proves that seam for real failures.
4. Scout Ilen's Familiar Orchard branch has no violated committed expectation in this scenario. Adding a production rule that records Familiar Orchard failure anyway would violate the source-expectation contract rather than repair it.
5. The safe correction was golden/scenario truthing: remove the stale failure-memory assertion, keep the Novel Grove discovery and later apple-source proof, and assert that Familiar Orchard is not falsely recorded as failed.
6. This ticket stayed scoped to the survival-preferences contract and did not reopen the S148 pressure-only probe escape or legal-control loose-lot fix.

## Architecture Check

1. The landed contract keeps learned source-failure memory tied to real expectation violations; it does not turn ordinary co-located familiarity or an unused source into failed memory.
2. The golden now proves the actual survival-preferences invariant: proactive exploration reaches Novel Grove, the discovered grove later becomes a concrete apple source, and no false Familiar Orchard failure memory is created.
3. No hidden "try familiar first" or authored outcome trigger was introduced.

## Verified Layers

1. Survival preferences golden -> `survival_preferences_keeps_proactive_diversification_alive_under_survival` now passes and asserts Novel Grove arrival, later Novel Grove apple success, survival contract health, required self-care families, no stuck idle windows, and zero Familiar Orchard failed attempts.
2. Deterministic replay -> `survival_preferences_replays_deterministically` passes after the observation struct was narrowed.
3. Generated golden docs -> `scripts/golden_inventory.py --write --check-docs` refreshed the scenario detail/index after metadata truthing.

## Landed Changes

### 1. Truth the golden assertion

`crates/worldwake-ai/tests/golden_survival_preferences.rs` no longer waits for impossible Familiar Orchard failure memory or a later source-reliability discount. It preserves the proactive Novel Grove and later apple-source checks and adds a zero-failed-attempts assertion for Familiar Orchard.

### 2. Truth scenario and generated prose

`scenarios/survival-preferences.ron` and Scenario 171 metadata now describe the real branch: proactive Novel Grove discovery and later Novel Grove apple acquisition inside the survival envelope. `docs/generated/golden-scenario-details/survival-preferences.md` and `docs/generated/golden-scenario-index.md` were regenerated from that metadata.

## Landed Files

- `crates/worldwake-ai/tests/golden_survival_preferences.rs`
- `scenarios/survival-preferences.ron`
- `docs/generated/golden-scenario-details/survival-preferences.md`
- `docs/generated/golden-scenario-index.md`

## Out of Scope

- Reintroducing pressure-only self-care acquisition probe admission.
- Reintroducing rejected-slot planning bypasses.
- Forcing familiar-source depletion with hidden quest/script logic.
- Changing unrelated survival goldens or observer thresholds.
- Changing source-reliability production code; live focused coverage already owns real committed-source failure memory.

## Acceptance Result

1. The survival-preferences golden passes on the live branch.
2. Familiar Orchard failure memory remains backed by concrete expectation violation semantics: this scenario has no violated Familiar Orchard source expectation, so the golden asserts `failed_attempts == 0` instead of fabricating a failure.
3. No preference/ranking behavior changed.
4. The repair does not depend on authored outcome triggers.

## Outcome

Completed on 2026-05-18.

Changed:

- Removed the stale Familiar Orchard failure-memory and failure-discount requirements from `survival_preferences_keeps_proactive_diversification_alive_under_survival`.
- Added an explicit no-false-failure assertion for Familiar Orchard.
- Updated survival-preferences scenario prose and Scenario 171 generated docs so the documented contract matches the live Novel Grove branch.

Deviations:

- The drafted premise said the familiar orchard did not deplete and should be repaired into durable failure memory. The observed live branch instead has no violated Familiar Orchard source expectation, so production AI code was left unchanged and the golden was narrowed to the truthful contract.

## Verification Result

- Passed `cargo test --release -p worldwake-ai --test golden_survival_preferences survival_preferences_keeps_proactive_diversification_alive_under_survival -- --ignored --test-threads=1`
- Passed `cargo test --release -p worldwake-ai --test golden_survival_preferences survival_preferences_replays_deterministically -- --ignored --test-threads=1`
- Passed `python3 scripts/golden_inventory.py --write --check-docs`
