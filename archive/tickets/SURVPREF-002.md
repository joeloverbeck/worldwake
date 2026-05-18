# SURVPREF-002: Reconcile survival-preferences experience-preference landing

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: No - roadmap/ticket truthing only; live golden metadata, scenario inputs, generated docs, and production AI/source-reliability code stayed unchanged
**Deps**: `archive/tickets/S148PORMOTBAC-FOLLOWUP-004.md`, `archive/tickets/SURVPREF-001.md`, `docs/scenario-roadmap.md` row 7 `survival-preferences`

## Problem

Before this ticket, `docs/scenario-roadmap.md` still treated row 7 as a partial `Experience preferences + diversification / curiosity` landing after `archive/tickets/S148PORMOTBAC-FOLLOWUP-004.md` corrected the live `survival-preferences` golden contract.

That archived correction proved the truthful live branch: Scout Ilen proactively reaches `Novel Grove`, later uses it as a real apple source, and must not fabricate Familiar Orchard source-failure memory without a violated committed source expectation. The remaining row-7 decision was whether to revive a lawful survival-time experience-preference branch, or narrow row 7 to the behavior the survival scenario actually proves and cite the existing non-row-7 experience-preference owners.

## Assumption Reassessment (2026-05-18)

1. `archive/tickets/S148PORMOTBAC-FOLLOWUP-004.md` passed `survival_preferences_keeps_proactive_diversification_alive_under_survival` after removing the stale Familiar Orchard failure-memory assertion and adding `failed_attempts == 0`.
2. The live Scenario 171 metadata, generated docs, and golden assertions already agree: `survival-preferences` proves proactive Novel Grove discovery, later Novel Grove apple acquisition, and no false Familiar Orchard failed-source memory.
3. The live branch has no violated Familiar Orchard source expectation, so reintroducing durable Familiar Orchard failure memory would be a false causal claim unless the scenario or production behavior changed to create an ordinary source-reliability failure.
4. Existing non-row-7 goldens cover the relevant experience-preference behavior surfaces: `golden_experience_preferences.rs` proves learned route preference effects, `golden_source_reliability.rs` proves concrete source-reliability memory writes, and `golden_source_composite.rs` proves source-composite same-commodity reranking from those memories.
5. No production AI/source-reliability gap was found for row 7. The truthful contract is to land row 7 as diversification / curiosity only and leave experience-preference behavior as auxiliary evidence outside the survival-roadmap row.

## Architecture Check

1. Row 7 now has one truthful source of status across roadmap prose, golden assertions, Scenario 171 metadata, and generated golden docs.
2. The roadmap no longer treats structural `preference_profile` activation as behavioral experience-preference proof.
3. No source is recorded as failed unless the actor had a real violated source expectation.
4. No hidden survival-only trigger, hand-seeded failure memory, compatibility shim, or alternate source-reliability path was introduced.

## Verified Layers

1. Row-7 contract truth -> `docs/scenario-roadmap.md` now marks `survival-preferences` as a landed diversification / curiosity row and stops claiming durable Familiar Orchard failure memory.
2. Existing auxiliary experience-preference coverage -> roadmap catalog points to `golden_experience_preferences.rs`, `golden_source_reliability.rs`, and `golden_source_composite.rs` instead of assigning that behavior to row 7.
3. Proactive diversification remains landed -> `survival_preferences_keeps_proactive_diversification_alive_under_survival` still proves Novel Grove arrival, later Novel Grove apple success, and no false Familiar Orchard failure memory.
4. Deterministic replay remains stable -> `survival_preferences_replays_deterministically` still passes.
5. Generated golden docs remain current -> `python3 scripts/golden_inventory.py --write --check-docs` passed with no Scenario 171 metadata change required.

## Landed Changes

### 1. Narrowed row-7 ownership

`docs/scenario-roadmap.md` now treats `survival-preferences` as a landed diversification / curiosity row. The ordered roadmap, status summary, row-7 detail section, and generated-companion editorial section no longer keep row 7 partial for experience-preference reconciliation.

### 2. Moved experience-preference status to auxiliary proof owners

The gameplay feature catalog now states that experience-preference behavior is covered by auxiliary goldens outside the survival-roadmap row: learned route preferences in `golden_experience_preferences.rs`, source-reliability memory writes in `golden_source_reliability.rs`, and source-composite reranking in `golden_source_composite.rs`.

### 3. Preserved Scenario 171 metadata and generated docs

`crates/worldwake-ai/tests/golden_survival_preferences.rs`, `scenarios/survival-preferences.ron`, and `docs/generated/golden-*` stayed unchanged because their live contract was already truthful after `archive/tickets/S148PORMOTBAC-FOLLOWUP-004.md`.

## Landed Files

- `docs/scenario-roadmap.md`
- `archive/tickets/SURVPREF-002.md`

## No-Change Cited Files

- `crates/worldwake-ai/tests/golden_survival_preferences.rs`
- `scenarios/survival-preferences.ron`
- `docs/generated/golden-scenario-details/survival-preferences.md`
- `docs/generated/golden-scenario-index.md`

## Out of Scope

- Reintroducing pressure-only self-care acquisition probe admission.
- Reintroducing rejected-slot planning bypasses.
- Hand-seeding Familiar Orchard failure memory.
- Changing AI/source-reliability production behavior without a live production gap.
- Changing unrelated survival roadmap rows beyond removing the stale row-7 status leak from the row-11 detail status line.

## Acceptance Result

1. Roadmap row 7, Scenario 171 metadata, generated docs, and golden assertions now agree about what `survival-preferences` proves.
2. Experience-preference learning remains backed by concrete route-experience, source-reliability, and source-composite state in existing auxiliary goldens rather than by the survival-preferences row.
3. No source is recorded as failed unless the actor had a real violated source expectation.

## Outcome

Completed on 2026-05-18.

- Narrowed row 7 to the truthful `survival-preferences` behavioral contract: diversification / curiosity under a 1440-tick survival-health scenario.
- Removed the stale row-7 pending status and the accidental row-11 status leak from `docs/scenario-roadmap.md`.
- Cited the existing auxiliary experience-preference proof owners instead of creating a false survival-time Familiar Orchard failure-memory branch.
- Left source, scenario, golden metadata, and generated golden docs unchanged because live reassessment found them already truthful.

## Deviations

- The draft allowed a revived row-7 experience-preference branch if live reassessment proved a production gap. No such gap was found, so the implementation is roadmap truthing only.
- `python3 scripts/golden_inventory.py --write --check-docs` was run even though Scenario 171 metadata did not change, to confirm generated golden docs remained current.

## Verification Result

- Passed `cargo test --release -p worldwake-ai --test golden_survival_preferences survival_preferences_keeps_proactive_diversification_alive_under_survival -- --ignored --test-threads=1`
- Passed `cargo test --release -p worldwake-ai --test golden_survival_preferences survival_preferences_replays_deterministically -- --ignored --test-threads=1`
- Passed `python3 scripts/golden_inventory.py --write --check-docs`
