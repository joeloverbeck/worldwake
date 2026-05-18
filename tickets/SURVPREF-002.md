# SURVPREF-002: Reconcile survival-preferences experience-preference landing

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes - likely `crates/worldwake-ai/tests/golden_survival_preferences.rs`, `scenarios/survival-preferences.ron`, `docs/scenario-roadmap.md`, or AI/source-reliability code only if live reassessment proves a production gap
**Deps**: `archive/tickets/S148PORMOTBAC-FOLLOWUP-004.md`, `archive/tickets/SURVPREF-001.md`, `docs/scenario-roadmap.md` row 7 `survival-preferences`

## Problem

`archive/tickets/S148PORMOTBAC-FOLLOWUP-004.md` corrected the live `survival-preferences` golden contract: Scout Ilen proactively reaches `Novel Grove`, later uses it as a real apple source, and must not fabricate Familiar Orchard source-failure memory without a violated committed source expectation.

That correction makes the active roadmap's row-7 wording stale. `docs/scenario-roadmap.md` still says row 7 lands "Experience preferences + diversification / curiosity" by carrying durable Familiar Orchard failure memory that discounts a later retry. The current golden no longer proves that experience-preference half. The remaining work is to decide and implement the truthful row-7 experience-preference landing: either create a lawful survival-time preference-memory branch, or narrow the roadmap row and point experience-preference coverage to its true owner.

## Assumption Reassessment (2026-05-18)

1. `archive/tickets/S148PORMOTBAC-FOLLOWUP-004.md` passed `survival_preferences_keeps_proactive_diversification_alive_under_survival` after removing the stale Familiar Orchard failure-memory assertion and adding `failed_attempts == 0`.
2. The generated Scenario 171 docs now prove proactive Novel Grove discovery, later Novel Grove apple acquisition, and no false Familiar Orchard failed-source memory.
3. `docs/scenario-roadmap.md` still overclaims durable Familiar Orchard failure memory in the ordered roadmap row and both row-7 detail sections.
4. Existing non-row-7 goldens cover source-reliability and source-composite behavior, but they are not the authored `survival-preferences` survival-roadmap seam.
5. The shared boundary under audit is the survival-roadmap proof contract: authored scenario setup, golden assertions, generated golden docs, and roadmap status must agree about whether experience preferences are landed inside row 7.
6. The live `GoalKind` under test is still `AcquireCommodity { commodity: Apple, purpose: SelfConsume }` if this ticket reestablishes an experience-preference branch.
7. The exact operator/failure surface for any revived branch must be ordinary travel plus `harvest:Harvest Apples` or another real source-reliability write path; no hand-seeded failure memory or hidden survival-only trigger is allowed.
8. The first implementation question is whether the row should remain `Experience preferences + diversification / curiosity` with a new lawful proof, or be narrowed to proactive diversification while experience-preference coverage remains owned by source-reliability/source-composite suites.

## Architecture Check

1. The clean path is to keep one truthful source of row-7 status across roadmap prose, golden metadata, generated docs, and executable assertions.
2. Any experience-preference proof must enter through concrete source reliability or source-composite state, not through authored outcomes or memory seeding.
3. No backwards-compatibility shims or alias paths should be introduced.

## Verification Layers

1. Row-7 contract truth -> `docs/scenario-roadmap.md` row 7, Scenario 171 metadata, and generated golden docs agree.
2. Proactive diversification remains landed -> `survival_preferences_keeps_proactive_diversification_alive_under_survival` still proves Novel Grove arrival and later Novel Grove apple success.
3. If experience preferences are re-landed in this scenario -> decision trace / authoritative world state proves the exact preference-memory or source-composite state that causally changes later apple acquisition.
4. If experience preferences are explicitly moved out of row 7 -> roadmap prose cites the alternate owning golden/spec/ticket and stops claiming the survival-preferences row proves durable Familiar Orchard failure memory.

## What to Change

### 1. Reassess row-7 ownership

Compare `docs/scenario-roadmap.md`, `golden_survival_preferences.rs`, generated Scenario 171 docs, and existing source-reliability/source-composite goldens. Decide whether row 7 still owns an experience-preference proof inside the survival scenario.

### 2. Land the truthful contract

If row 7 still owns experience preferences, adjust scenario/golden/AI behavior only through lawful source-reliability or source-composite paths and prove the branch. If it does not, narrow the roadmap row and generated metadata to the diversification contract and cite the correct owner for experience-preference coverage.

## Files to Touch

- `docs/scenario-roadmap.md` (modify)
- `crates/worldwake-ai/tests/golden_survival_preferences.rs` (modify if row-7 metadata/assertions change)
- `scenarios/survival-preferences.ron` (modify only if authored inputs need a lawful preference branch)
- `docs/generated/golden-scenario-details/survival-preferences.md` and `docs/generated/golden-scenario-index.md` (regenerate if Scenario 171 metadata changes)
- AI/source-reliability code only if reassessment proves production behavior is the truthful owner

## Out of Scope

- Reintroducing pressure-only self-care acquisition probe admission.
- Reintroducing rejected-slot planning bypasses.
- Hand-seeding Familiar Orchard failure memory.
- Changing unrelated survival roadmap rows.

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test --release -p worldwake-ai --test golden_survival_preferences survival_preferences_keeps_proactive_diversification_alive_under_survival -- --ignored --test-threads=1`
2. `cargo test --release -p worldwake-ai --test golden_survival_preferences survival_preferences_replays_deterministically -- --ignored --test-threads=1`
3. `python3 scripts/golden_inventory.py --write --check-docs` if Scenario 171 metadata changes
4. Any focused unit, runtime, or golden test added for a revived experience-preference branch

### Invariants

1. Roadmap row 7, Scenario 171 metadata, generated docs, and golden assertions must not disagree about what `survival-preferences` proves.
2. Experience-preference learning must be backed by concrete source reliability or source-composite state if row 7 continues to claim it.
3. No source may be recorded as failed unless the actor had a real violated source expectation.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_survival_preferences.rs` - update only if the row-7 proof contract changes.
2. Focused lower-layer test if production AI/source-reliability behavior changes.

### Commands

1. `cargo test --release -p worldwake-ai --test golden_survival_preferences survival_preferences_keeps_proactive_diversification_alive_under_survival -- --ignored --test-threads=1`
2. `cargo test --release -p worldwake-ai --test golden_survival_preferences survival_preferences_replays_deterministically -- --ignored --test-threads=1`
3. `python3 scripts/golden_inventory.py --write --check-docs`
