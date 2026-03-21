# S17WOULIFGOLSUI-003: Golden E2E Docs Catch-Up for Scenarios 29 and 30

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: None
**Deps**: S17WOULIFGOLSUI-001 (Scenario 29 must exist), S17WOULIFGOLSUI-002 (Scenario 30 must exist)

## Problem

After S17WOULIFGOLSUI-001 and S17WOULIFGOLSUI-002 land, the golden E2E documentation (`golden-e2e-coverage.md` and `golden-e2e-scenarios.md`) will be out of date. The coverage matrix will not reflect Scenarios 29 and 30, and the scenario catalog will not describe the new tests. The generated inventory (`generated/golden-e2e-inventory.md`) must also be regenerated.

## Assumption Reassessment (2026-03-21)

1. `docs/golden-e2e-coverage.md` contains the coverage matrix (GoalKind coverage, system coverage, cross-system chains). Scenarios 29 and 30 are not present — confirmed by reading the file.
2. `docs/golden-e2e-scenarios.md` contains detailed scenario descriptions. Scenarios 29 and 30 are not present — confirmed by reading the file.
3. `scripts/golden_inventory.py` exists and generates `docs/generated/golden-e2e-inventory.md`. The `--write --check-docs` flag regenerates the inventory and validates it against the docs.
4. Not an AI regression, ordering, heuristic, stale-request, political, or ControlSource ticket.
5. No mismatch found.

## Architecture Check

1. Documentation-only ticket. Updates docs to reflect test reality. No code or behavior changes.
2. No backwards-compatibility aliasing/shims introduced.

## Verification Layers

1. Coverage matrix includes Scenarios 29 and 30 → manual doc review
2. Scenario catalog describes both new tests → manual doc review
3. Generated inventory matches reality → `python3 scripts/golden_inventory.py --write --check-docs`
4. Single-layer ticket (documentation); no additional verification layers applicable.

## What to Change

### 1. Update `docs/golden-e2e-coverage.md`

- Add Scenario 29 to relevant GoalKind and system coverage rows:
  - Cross-system chain: Needs (deprivation exposure) → Wounds (consolidation) → Identity preservation
  - System coverage: Needs system (deprivation firing), WoundList (worsening), DeprivationExposure
- Add Scenario 30 to relevant rows:
  - GoalKind: ConsumeOwnedCommodity (eat), Wash — already covered, but add Scenario 30 reference
  - Cross-system chain: AI ranking (recovery-aware promotion) → Eat action → Recovery gate → Wound healing
  - System coverage: AI ranking (`promote_for_clotted_wound_recovery`), Combat (recovery gate)
- Update the date stamp in the header

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

Run `python3 scripts/golden_inventory.py --write --check-docs` to regenerate the generated inventory and validate it matches the updated docs.

## Files to Touch

- `docs/golden-e2e-coverage.md` (modify — add Scenarios 29, 30 to coverage matrix)
- `docs/golden-e2e-scenarios.md` (modify — add Scenario 29, 30 descriptions)
- `docs/generated/golden-e2e-inventory.md` (regenerate via script)

## Out of Scope

- Any production code changes
- Any test code changes (tests were added in S17WOULIFGOLSUI-001 and S17WOULIFGOLSUI-002)
- Updating `docs/golden-e2e-testing.md` (no new assertion patterns introduced)
- Updating `specs/S17-wound-lifecycle-golden-suites.md` (spec is source of truth, docs reflect it)
- Updating any other spec or ticket files

## Acceptance Criteria

### Tests That Must Pass

1. `python3 scripts/golden_inventory.py --write --check-docs` — inventory regeneration succeeds and docs validation passes
2. `cargo test --workspace` — no regressions (sanity check, no code changes expected)

### Invariants

1. `golden-e2e-coverage.md` coverage matrix includes both Scenario 29 and Scenario 30
2. `golden-e2e-scenarios.md` scenario catalog includes descriptions for both new tests
3. Generated inventory matches the actual `golden_*.rs` test declarations
4. No production or test code modified — this is a documentation-only ticket

## Test Plan

### New/Modified Tests

None — documentation-only ticket; verification is command-based and existing runtime coverage is named in Assumption Reassessment.

### Commands

1. `python3 scripts/golden_inventory.py --write --check-docs`
2. `cargo test --workspace` (sanity check)
