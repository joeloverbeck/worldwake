# S17WOULIFGOLSUI-003: Golden E2E Docs Catch-Up for Scenarios 29 and 30

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: None
**Deps**: S17WOULIFGOLSUI-001 (Scenario 29 must exist), S17WOULIFGOLSUI-002 (Scenario 30 must exist)

## Problem

After S17WOULIFGOLSUI-001 and S17WOULIFGOLSUI-002 land, the golden E2E documentation (`golden-e2e-coverage.md` and `golden-e2e-scenarios.md`) will be out of date. The coverage matrix will not reflect Scenarios 29 and 30, and the scenario catalog will not describe the new tests. The generated inventory (`generated/golden-e2e-inventory.md`) must also be regenerated.

## Assumption Reassessment (2026-03-21)

1. `crates/worldwake-ai/tests/golden_emergent.rs` already contains `golden_deprivation_wound_worsening_consolidates_not_duplicates` and its deterministic replay companion. `crates/worldwake-ai/tests/golden_combat.rs` already contains `golden_recovery_aware_boost_eats_before_wash` and its deterministic replay companion. The ticket therefore starts from docs drift, not missing test implementation.
2. `docs/golden-e2e-coverage.md` and `docs/golden-e2e-scenarios.md` both exist, contrary to older ticket drift elsewhere. The generated inventory path is also real: `docs/generated/golden-e2e-inventory.md`.
3. `docs/generated/golden-e2e-inventory.md` already lists both Scenario 29/30 tests, and `python3 scripts/golden_inventory.py --write --check-docs` currently passes. The remaining gap is the hand-maintained dashboard/catalog text, not the generated inventory pipeline.
4. The current hand-written docs are only partially aligned: the scenario catalog does not yet describe Scenarios 29 and 30, and the coverage dashboard summary counts still lag the current 133-test inventory.
5. Not an AI-runtime behavior ticket. Verification is documentation review plus docs-sync validation, with the real target tests named to prove the scenarios already exist.
6. Mismatch corrected: this ticket should not describe the docs as nonexistent or imply that Scenarios 29 and 30 still need test implementation.

## Architecture Check

1. Documentation-only ticket. Updates docs to reflect test reality. No code or behavior changes.
2. No backwards-compatibility aliasing/shims introduced.

## Verification Layers

1. Coverage matrix includes Scenarios 29 and 30 → manual doc review
2. Scenario catalog describes both new tests → manual doc review
3. Generated inventory remains in sync with real `golden_*` declarations → `python3 scripts/golden_inventory.py --write --check-docs`
4. Single-layer ticket (documentation); no additional runtime verification layers applicable.

## What to Change

### 1. Update `docs/golden-e2e-coverage.md`

- Add Scenario 29 and Scenario 30 to the relevant cross-system coverage rows and update any stale counts or dates so the dashboard matches the current 133-test inventory.
- Keep the dashboard interpretive rather than duplicating the generated inventory by hand.

### 2. Update `docs/golden-e2e-scenarios.md`

Add two new scenario entries following the existing format:

**Scenario 29: Deprivation Wound Worsening Consolidates Not Duplicates**
- File: `golden_emergent.rs`
- Test: `golden_deprivation_wound_worsening_consolidates_not_duplicates`
- Systems exercised, setup, emergent behavior proven, cross-system chain (per spec)

**Scenario 30: Recovery-Aware Priority Boost Eats Before Wash**
- File: `golden_combat.rs`
- Test: `golden_recovery_aware_boost_eats_before_wash`
- Systems exercised, setup, emergent behavior proven, cross-system chain (per spec)

Update the date stamp in the header.

### 3. Regenerate `docs/generated/golden-e2e-inventory.md`

Run `python3 scripts/golden_inventory.py --write --check-docs` to validate docs against the generated inventory and rewrite the artifact only if it changed.

## Files to Touch

- `docs/golden-e2e-coverage.md` (modify — add Scenarios 29, 30 to coverage matrix)
- `docs/golden-e2e-scenarios.md` (modify — add Scenario 29, 30 descriptions)
- `docs/generated/golden-e2e-inventory.md` (validate via script; regenerate only if changed)

## Out of Scope

- Any production code changes
- Any test code changes (tests were added in S17WOULIFGOLSUI-001 and S17WOULIFGOLSUI-002)
- Updating `docs/golden-e2e-testing.md` (no new assertion patterns introduced)
- Updating `specs/S17-wound-lifecycle-golden-suites.md` (spec is source of truth, docs reflect it)
- Updating any other spec or ticket files

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-ai golden_deprivation_wound_worsening_consolidates_not_duplicates -- --exact`
2. `cargo test -p worldwake-ai golden_recovery_aware_boost_eats_before_wash -- --exact`
3. `python3 scripts/golden_inventory.py --write --check-docs`
4. `cargo test --workspace`

### Invariants

1. `golden-e2e-coverage.md` coverage matrix includes both Scenario 29 and Scenario 30
2. `golden-e2e-scenarios.md` scenario catalog includes descriptions for both new tests
3. Generated inventory matches the actual `golden_*.rs` test declarations
4. No production or test code modified — this is a documentation-only ticket

## Test Plan

### New/Modified Tests

None — documentation-only ticket; verification is command-based and existing runtime coverage is named in Assumption Reassessment.

### Commands

1. `cargo test -p worldwake-ai golden_deprivation_wound_worsening_consolidates_not_duplicates -- --exact`
2. `cargo test -p worldwake-ai golden_recovery_aware_boost_eats_before_wash -- --exact`
3. `python3 scripts/golden_inventory.py --write --check-docs`
4. `cargo test --workspace`
