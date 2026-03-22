# GOLSCENMAP-001: Generate and Validate Golden Scenario Metadata Inventory

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: None — docs/tooling/test metadata only
**Deps**: `docs/FOUNDATIONS.md`, `tickets/README.md`, `docs/precision-rules.md`, `docs/golden-e2e-testing.md`, `docs/golden-e2e-scenarios.md`, `docs/golden-e2e-coverage.md`, `docs/generated/golden-e2e-inventory.md`, `scripts/golden_inventory.py`, `archive/tickets/completed/S18TICKETDOC-001.md`, `archive/tickets/completed/S19INSRECCON-003.md`

## Problem

The repo already has a generated `golden_*` test inventory, but it does not have a canonical generated mapping from explicit source-declared scenario identity to the real `golden_*` tests that implement that scenario. Today `docs/generated/golden-e2e-inventory.md` tracks test names only. That leaves a drift hole for hand-written coverage docs and scenario catalogs that talk about "Scenario 33", "Scenario 34", or similar identities without a machine-checkable link back to the owning test block.

This is a documentation/tooling architecture problem: the repo lacks a generated scenario map grounded in the real golden test sources. The fix should strengthen the existing generated-inventory architecture, not add a second hand-maintained registry.

## Assumption Reassessment (2026-03-22)

1. The current generated inventory script at [`scripts/golden_inventory.py`](/home/joeloverbeck/projects/worldwake/scripts/golden_inventory.py) validates `golden_*` test names against compiled test binaries and doc references, but it does not track scenario identifiers or titles declared in source comments.
2. The current coverage/scenario docs already treat generated inventory as the canonical mechanical source for `golden_*` tests:
   - [`docs/golden-e2e-coverage.md`](/home/joeloverbeck/projects/worldwake/docs/golden-e2e-coverage.md)
   - [`docs/golden-e2e-scenarios.md`](/home/joeloverbeck/projects/worldwake/docs/golden-e2e-scenarios.md)
   The missing contract is scenario identity grounding, not raw test-name grounding.
3. Golden source files already use explicit scenario header comments such as `// Scenario 33: ...` and `// Scenario 34: ...` in [`crates/worldwake-ai/tests/golden_offices.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_offices.rs). Those headers are the cleanest current substrate for a derived scenario map, but they are not parsed or validated today.
4. Scenario identifiers in the live repo are not uniformly plain integers. Current source headers include values such as `11b`, `2c-self`, and `S03a` in addition to `33` and `34`. The generated map therefore needs to model a general scenario identifier string rather than assuming every scenario identity is a bare number.
5. The live repo already has focused script-level regression tests in [`scripts/test_golden_inventory.py`](/home/joeloverbeck/projects/worldwake/scripts/test_golden_inventory.py). This ticket should extend that existing test surface rather than inventing a new testing pattern.
6. Not every spec-described scenario is implemented yet. In particular, [`specs/S19-institutional-record-consultation-golden-suites.md`](/home/joeloverbeck/projects/worldwake/specs/S19-institutional-record-consultation-golden-suites.md) still describes Scenario 32, but there is no corresponding live `golden_consult_record_prerequisite_political_action` test in [`crates/worldwake-ai/tests/golden_offices.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_offices.rs). The generated map must therefore reflect only source-declared live scenarios, not planned spec inventory.
7. This is not a planner-runtime or authoritative-engine ticket. The intended verification layer is documentation/tooling correctness. Additional runtime-layer mapping is not applicable beyond validating that the generated map is derived from the real golden test sources and remains docs-synced.
8. Ordering is not the contract here. The architectural requirement is source-of-truth grounding: generated scenario metadata should derive from real test files, and docs should consume that derived inventory instead of relying on hand-maintained scenario identity.
9. No heuristic is being weakened. This ticket introduces a missing documentation/tooling substrate so authors do not rely on memory or prose catalogs when identifying already-implemented numbered or letter-suffixed scenarios.
10. Mismatch + correction: the current repo is stronger than a naive docs-only process because the generated `golden_*` inventory and its focused script tests already exist, but they still do not cover source-declared scenario identity.
11. The clean architecture consistent with [`docs/FOUNDATIONS.md`](/home/joeloverbeck/projects/worldwake/docs/FOUNDATIONS.md) Principle 25 is to generate the scenario map from concrete source comments plus real `golden_*` functions. Hand-written scenario docs remain useful as downstream interpretation, but they should not be the canonical identity registry.

## Architecture Check

1. A generated scenario metadata map is cleaner than adding more prose rules alone. It grounds scenario identity in concrete test source, which is more robust than relying on reviewers to manually notice scenario-number drift.
2. The right design is derived-summary architecture, not another hand-maintained registry. Scenario identity should be extracted from explicit metadata already colocated with the owning tests, then rendered into generated docs and validated.
3. The cleanest substrate is the existing `// Scenario ...` source-header convention. Adding required spec/ticket-reference metadata to every test block would create a second copy of planning information inside test files and would be worse than the current architecture. Spec and ticket references should remain human-authored docs, not part of the canonical generated identity map.
4. No backwards-compatibility aliases or duplicate “shadow catalogs” should be introduced. The generated scenario map should become the canonical mechanical source for source-declared scenario identity, and existing docs should reference it rather than re-declaring the same mapping manually.

## Verification Layers

1. source-declared scenario headers are parsed into machine-readable scenario metadata -> source parser + focused script tests
2. generated scenario map reflects the real `golden_*.rs` inventory -> script output validated against source and compiled `cargo test -p worldwake-ai --test ... -- --list`
3. golden docs point readers at the generated scenario map and only reference existing `golden_*` names -> doc updates + existing test-name validation in inventory script
4. single-layer tooling/docs ticket; no additional runtime-layer mapping is applicable

## What to Change

### 1. Extend generated inventory tooling to build a scenario map

Extend `scripts/golden_inventory.py` or add a companion script so the repo can generate and validate a scenario metadata artifact, for example:

- `docs/generated/golden-scenario-map.md`

The generated artifact should include:

- scenario identifier and title
- owning file
- all `golden_*` tests found under that scenario block
- a primary/replay split when that can be derived mechanically from current naming conventions

Validation should fail when:

- the same scenario identifier is declared more than once in live source
- a declared scenario block contains no `golden_*` tests
- a scenario map entry points at `golden_*` tests that are not present in the compiled test inventory

### 2. Make the generated scenario map the canonical identity inventory

Update the golden docs and ticket authoring guidance so that source-declared scenario identity is taken from the generated scenario map, while high-level interpretation remains in the hand-written docs. This preserves the current “generated mechanics, written interpretation” split.

### 3. Add script-focused tests

Add focused regression tests for the scenario-metadata parser and validator so future formatting changes do not silently break the generated map.

## Files to Touch

- `scripts/golden_inventory.py` (modify) or a new companion script if separation is cleaner
- `docs/generated/` new generated scenario map artifact
- `docs/golden-e2e-coverage.md` (modify)
- `docs/golden-e2e-scenarios.md` (modify)
- `docs/golden-e2e-testing.md` (modify if the canonical inventory/source wording needs adjustment)
- `tickets/README.md` (modify to point golden ticket authors at the generated scenario map)
- `scripts/` focused script tests if the repo already has a pattern for them; otherwise add a minimal test file in an appropriate location

## Out of Scope

- Changing golden scenario behavior
- Renumbering existing scenarios
- Engine/runtime changes
- Building a generic repository-wide ticket/spec inventory system beyond golden scenarios
- Validating planned-but-unimplemented scenario identities from specs or tickets
- Encoding spec/ticket references as required source metadata in golden test files

## Acceptance Criteria

### Tests That Must Pass

1. Generated scenario map renders from live golden test sources and includes numbered scenario metadata
2. Validation fails on duplicate or missing scenario-block metadata
3. `python3 scripts/golden_inventory.py --write --check-docs` or the new equivalent scenario-map command passes with updated docs
4. Focused script tests for the scenario-map parser/validator pass
5. Existing suite: `cargo test -p worldwake-ai -- --list`

### Invariants

1. Scenario identity inventory is derived from concrete test sources, not maintained as a parallel hand-written catalog
2. Generated docs remain caches of source truth and can be regenerated without changing world meaning
3. No backwards-compatibility alias layer is introduced between old and new scenario references; stale docs should be corrected directly
4. The generated map only claims live source-declared scenario identity; it does not pretend planned spec inventory is already implemented

## Test Plan

### New/Modified Tests

1. `scripts/test_golden_inventory.py` — extend focused parser/validator coverage to prove the generated map catches duplicate scenario identifiers, empty scenario blocks, malformed scenario headers, and missing compiled tests.
   Rationale: this is the authoritative regression surface for the tooling change.
2. `None` at runtime — this ticket changes docs/tooling metadata only; existing golden/runtime coverage remains the behavior source of truth.

### Commands

1. `python3 scripts/test_golden_inventory.py`
2. `python3 scripts/golden_inventory.py --write --check-docs`
3. `cargo test -p worldwake-ai -- --list`
4. `scripts/verify.sh`

## Outcome

- Completion date: 2026-03-22
- What actually changed:
  - extended `scripts/golden_inventory.py` to generate `docs/generated/golden-scenario-map.md` from live `// Scenario ...` headers plus colocated `golden_*` tests
  - kept the architecture derived-source only; did not add required spec/ticket-reference metadata to test files
  - extended `scripts/test_golden_inventory.py` with scenario-map parser and validator coverage
  - updated golden docs and `tickets/README.md` to point authors at the generated scenario map alongside the existing generated test inventory
- Deviations from original plan:
  - narrowed the scope away from validating plain-text scenario mentions across all docs because live source headers are not yet standardized for every documented scenario block
  - dropped spec/ticket-reference fields from the generated map because duplicating planning metadata inside test source would be worse than the current architecture
- Verification results:
  - `python3 scripts/test_golden_inventory.py` passed
  - `python3 scripts/golden_inventory.py --write --check-docs` passed
  - `cargo test -p worldwake-ai -- --list` passed
  - `scripts/verify.sh` passed
