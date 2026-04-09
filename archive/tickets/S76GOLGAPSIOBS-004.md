# S76GOLGAPSIOBS-004: Align active S76 spec with landed S76-A/S76-B contract

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: None
**Deps**: S76GOLGAPSIOBS-001

## Problem

`specs/S76-golden-gaps-simulation-observer.md` still describes S76-B using a stale scenario shape (`BarrenOutpost`, `ResourceTown`, and “only relieve_wilderness available”) and still documents the old S76-001 task breakdown in terms that no longer match the landed tests. The completed ticket corrected the live contract to use the prototype world (`VillageSquare` / `OrchardFarm`) and an honest “remote resource scarcity with lawful local self-care fallback” boundary. Leaving the active spec stale reintroduces roadmap drift and weakens future ticketing.

## Assumption Reassessment (2026-04-09)

1. The completed implementation lives in `crates/worldwake-ai/tests/golden_simulation_gaps.rs` with Scenario 126 and Scenario 127, not with `// Scenario S76-A` / `// Scenario S76-B` headers.
2. The completed ticket at `archive/tickets/S76GOLGAPSIOBS-001.md` records the live correction: S76-B is no longer “total local unsatisfiability except relieve_wilderness”; the landed scenario uses `VillageSquare` plus remote `OrchardFarm` resources and proves a bounded idle streak under remote scarcity.
3. `specs/S76-golden-gaps-simulation-observer.md` still contains stale prose at the S76-B scenario section and in the S76-001 ticket breakdown (`BarrenOutpost`, `ResourceTown`, “all local needs unsatisfiable except relieve”).
4. This is active-spec drift only. No code or tests need to change; the owning fix is factual alignment of the active spec text with the already-landed golden contract.

## Architecture Check

1. Correcting the active spec is cleaner than leaving the completed ticket as the only authoritative description of the landed contract.
2. No backwards-compatibility shims or duplicate scenario narratives are needed; the spec should simply match the live tests and generated golden docs.

## Verification Layers

1. Active spec prose matches landed S76-A/S76-B tests -> document review against `crates/worldwake-ai/tests/golden_simulation_gaps.rs`
2. Scenario inventory naming matches live metadata -> generated docs review via `docs/generated/golden-e2e-inventory.md`
3. No code/test fallout -> no additional runtime verification layer required
4. Single-layer doc ticket. Additional layer mapping is not applicable.

## What to Change

### 1. Rewrite the active S76-B scenario prose

- Replace stale `BarrenOutpost` / `ResourceTown` / “only relieve_wilderness available” wording with the landed `VillageSquare` / `OrchardFarm` contract.
- State clearly that the scenario proves bounded idle under remote food/water scarcity while lawful local self-care branches remain available.

### 2. Align the S76-001 breakdown and replay/conservation notes

- Update the S76-001 subsection to match the landed file path, scenario numbering, and function/test naming.
- Correct the conservation note to the deterministic final arithmetic proven by the implemented scenario.

## Files to Touch

- `specs/S76-golden-gaps-simulation-observer.md` (modify)

## Out of Scope

- New golden tests
- Production or harness changes
- S76-C and S76-D implementation work

## Acceptance Criteria

### Tests That Must Pass

1. `python3 scripts/golden_inventory.py --write --check-docs` (docs remain internally consistent after spec alignment)

### Invariants

1. The active spec’s S76-A/S76-B prose matches the landed scenario contract in `crates/worldwake-ai/tests/golden_simulation_gaps.rs`
2. No production code or tests change

## Test Plan

### New/Modified Tests

1. None — documentation-only ticket; verification is command-based and existing runtime coverage is named in Assumption Reassessment.

### Commands

1. `python3 scripts/golden_inventory.py --write --check-docs`

## Outcome

Completed on 2026-04-09.

Updated `specs/S76-golden-gaps-simulation-observer.md` to match the landed S76-A/S76-B contract from `crates/worldwake-ai/tests/golden_simulation_gaps.rs`. The active spec now describes the live `VillageSquare` / `OrchardFarm` scenario shape, states S76-B as bounded idle under remote food/water scarcity with lawful local self-care fallback, and aligns the S76-001 breakdown with the implemented Scenario 126 / 127 coverage and deterministic replay companions. The replay/conservation note was also corrected to the deterministic end-state proven by the landed S76-A scenario.

## Verification Result

- `python3 scripts/golden_inventory.py --write --check-docs` (pass)
- `git status --short tickets/S76GOLGAPSIOBS-004.md specs/S76-golden-gaps-simulation-observer.md docs/generated/golden-e2e-inventory.md docs/generated/golden-scenario-details/simulation-gaps.md` showed only `specs/S76-golden-gaps-simulation-observer.md` modified within this ticket's owned surface; no production code or test files changed
