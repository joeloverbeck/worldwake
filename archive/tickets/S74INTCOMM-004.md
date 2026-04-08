# S74INTCOMM-004: Refresh generated golden docs after Scenario 125 contract correction

**Status**: COMPLETED
**Priority**: LOW
**Effort**: Small
**Engine Changes**: None — generated docs handoff only
**Deps**: None

## Problem

The live `golden_report_found_after_search` scenario contract was corrected in `crates/worldwake-ai/tests/golden_expectation.rs` so reporting happens to a local `OfficeRegister` at OrchardFarm rather than by traveling back to VillageSquare. The generated golden docs still describe the old VillageSquare-return path, so the repository's canonical generated scenario references no longer match the implemented behavior.

## Assumption Reassessment (2026-04-08)

1. Live source scenario metadata in `crates/worldwake-ai/tests/golden_expectation.rs:1300-1327` now says the `OfficeRegister` exists at OrchardFarm and the chain ends with local `report_found` rather than return travel.
2. Generated docs are stale in both `docs/generated/golden-scenario-index.md:643-647` and `docs/generated/golden-scenario-details/expectation.md:92-96`; both still say the register is at VillageSquare and the planner travels back there before `report_found`.
3. The runtime proof surface is already green: `cargo test -p worldwake-ai --test golden_expectation` passes with the corrected local-report contract, so this ticket is documentation handoff only.
4. This is a golden-generated-docs ticket, not a production or golden-behavior ticket. The owned boundary is the generated inventory/detail output from `python3 scripts/golden_inventory.py --write --check-docs`, not `report_found` logic or scenario assertions.

## Architecture Check

1. Regenerating the canonical golden docs is cleaner than manually editing prose fragments because the generated files are derived artifacts and should stay aligned with the source scenario metadata.
2. No backwards-compatibility shims or duplicate documentation paths are introduced; the ticket restores consistency between the generator inputs and generated outputs.

## Verification Layers

1. Generated scenario prose matches live source scenario metadata -> regenerated `docs/generated/golden-scenario-index.md` and `docs/generated/golden-scenario-details/expectation.md`
2. Golden-doc generator remains healthy -> `python3 scripts/golden_inventory.py --write --check-docs`
3. Runtime contract already remains proven at the stronger layer -> existing `cargo test -p worldwake-ai --test golden_expectation`
4. Single-layer ticket: no additional production-layer mapping is applicable because behavior already landed and this ticket only repairs generated handoff artifacts.

## What to Change

### 1. Regenerate golden docs from the corrected Scenario 125 metadata

Run the golden inventory/doc generator so the generated scenario index and expectation detail pages pick up the corrected OrchardFarm-local `report_found` contract.

### 2. Verify the generated prose matches the live scenario

Check Scenario 125 in both generated docs and confirm they now describe:
- the `OfficeRegister` at OrchardFarm
- no return travel to VillageSquare
- the shortened cross-system chain ending with local `report_found`

## Files to Touch

- `docs/generated/golden-scenario-index.md` (modify)
- `docs/generated/golden-scenario-details/expectation.md` (modify)

## Out of Scope

- Any further changes to `report_found` production logic
- Any changes to `crates/worldwake-ai/tests/golden_expectation.rs`
- Regenerating unrelated generated docs whose content does not change

## Acceptance Criteria

### Tests That Must Pass

1. `python3 scripts/golden_inventory.py --write --check-docs`
2. Existing runtime proof remains green: `cargo test -p worldwake-ai --test golden_expectation`

### Invariants

1. Generated Scenario 125 prose matches the live source scenario metadata
2. Generated docs no longer claim a return trip to VillageSquare for `golden_report_found_after_search`

## Test Plan

### New/Modified Tests

1. None — documentation-only ticket; verification is command-based and existing runtime coverage is named in Assumption Reassessment.

### Commands

1. `python3 scripts/golden_inventory.py --write --check-docs`
2. `cargo test -p worldwake-ai --test golden_expectation`

## Outcome

Completed on 2026-04-08.

- Regenerated the canonical golden scenario docs so Scenario 125 now matches the corrected local-report contract in `golden_expectation.rs`.
- Updated `docs/generated/golden-scenario-index.md` and `docs/generated/golden-scenario-details/expectation.md` to describe the `OfficeRegister` at OrchardFarm, remove the stale return trip to VillageSquare, and shorten the cross-system chain accordingly.

## Verification Result

- Passed `python3 scripts/golden_inventory.py --write --check-docs`
- Passed `cargo test -p worldwake-ai --test golden_expectation`
