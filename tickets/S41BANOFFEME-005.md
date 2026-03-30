# S41BANOFFEME-005: Golden Inventory Update & Cross-Suite Verification

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: None
**Deps**: S41BANOFFEME-002 (Suite 1), S41BANOFFEME-003 (Suite 2), S41BANOFFEME-004 (Suite 3)

## Problem

After all three S41 golden suites are implemented, the golden test inventory and scenario map documentation must be regenerated to include Scenarios 47–49. The cross-suite verification step ensures no regressions exist and all suites meet the spec's coverage claims (each suite exercises >= 3 systems with causal depth >= 3).

## Assumption Reassessment (2026-03-30)

1. **`scripts/golden_inventory.py`** — confirmed at `scripts/golden_inventory.py`. The `--write --check-docs` flags regenerate `docs/generated/golden-e2e-inventory.md` and `docs/generated/golden-scenario-map.md`.
2. **Scenario IDs 47, 48, 49** — confirmed available. Current highest is Scenario 46 in `golden_emergent.rs`.
3. **Metadata annotations** — each suite's test file must include the `// Scenario NN:` comment block with Systems, GoalKinds, ActionDomains, Places, Principles metadata for the inventory script to parse.
4. **Coverage claim**: Spec says after implementation, E18-related golden coverage = 8 tests (2 existing T22 + 6 new). This is verified by counting `#[test]` functions matching `golden_*` patterns in `golden_t22_bandit_camp_destruction.rs`.
5. **New GoalKind coverage**: `RaidTarget` currently has 0 golden scenarios. After S41, it will have Scenarios 47 and 49.
6. **New principle coverage**: FND-10 (Physical Dampeners) gets its first golden validation via Scenario 49.

## Architecture Check

1. No code changes — this is a documentation regeneration and verification step.
2. No backwards-compatibility shims.

## Verification Layers

1. Golden inventory completeness → `python3 scripts/golden_inventory.py --write --check-docs` exits 0
2. Scenario count → grep `#[test]` in `golden_t22_bandit_camp_destruction.rs` shows 8 test functions (2 T22 + 6 S41)
3. Full regression → `cargo test -p worldwake-ai` passes with 0 failures
4. Workspace health → `cargo clippy --workspace` produces no warnings

## What to Change

### 1. Verify all three suites pass

Run the full AI test suite to confirm all 6 new tests (3 main + 3 replay) pass alongside the 2 existing T22 tests.

### 2. Regenerate golden inventory

Run `python3 scripts/golden_inventory.py --write --check-docs` to update:
- `docs/generated/golden-e2e-inventory.md`
- `docs/generated/golden-scenario-map.md`

### 3. Verify inventory includes Scenarios 47–49

Inspect the regenerated files to confirm Scenarios 47 (Pressure-Driven Raid Emergence), 48 (Raid-Belief Economic Cascade), and 49 (Wound-Dampened Raid Spiral) are present with correct metadata.

### 4. Verify coverage claims

Confirm:
- `RaidTarget` appears in at least 2 scenarios (47, 49)
- FND-10 (Physical Dampeners) is referenced in Scenario 49
- Each new suite spans >= 3 systems and >= 3 causal depth (per spec metadata annotations)

## Files to Touch

- `docs/generated/golden-e2e-inventory.md` (regenerated)
- `docs/generated/golden-scenario-map.md` (regenerated)

## Out of Scope

- Writing or modifying golden test code (completed in S41BANOFFEME-002 through S41BANOFFEME-004)
- Engine changes (completed in S41BANOFFEME-004)
- Modifying the inventory script itself
- Updating CLAUDE.md or other project documentation beyond the generated inventory files

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-ai` — all golden tests pass (8 in `golden_t22_bandit_camp_destruction.rs`)
2. `cargo clippy --workspace` — no warnings
3. `python3 scripts/golden_inventory.py --write --check-docs` — exits 0

### Invariants

1. Generated inventory includes all scenarios 1–49 without gaps or duplicates.
2. Scenario metadata annotations in test source match the generated inventory entries.
3. No existing golden tests regress from the S41 additions.

## Test Plan

### New/Modified Tests

None — documentation-only ticket; verification is command-based and existing runtime coverage is named in Assumption Reassessment.

### Commands

1. `cargo test -p worldwake-ai` — full AI crate regression
2. `cargo clippy --workspace` — no warnings
3. `python3 scripts/golden_inventory.py --write --check-docs` — regenerate and verify inventory
