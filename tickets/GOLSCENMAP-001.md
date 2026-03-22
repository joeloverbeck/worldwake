# GOLSCENMAP-001: Generate and Validate Golden Scenario Metadata Inventory

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: None — docs/tooling/test metadata only
**Deps**: `docs/FOUNDATIONS.md`, `tickets/README.md`, `docs/precision-rules.md`, `docs/golden-e2e-testing.md`, `docs/golden-e2e-scenarios.md`, `docs/golden-e2e-coverage.md`, `docs/generated/golden-e2e-inventory.md`, `scripts/golden_inventory.py`, `archive/tickets/completed/S18TICKETDOC-001.md`, `archive/tickets/completed/S19INSRECCON-003.md`

## Problem

The repo already has a generated `golden_*` test inventory, but it does not have a canonical generated mapping from:

- scenario number
- scenario title
- owning golden file
- primary test name
- replay-companion test name
- spec/ticket references

That leaves a remaining drift hole between specs, active tickets, and already-implemented golden scenarios. `S19INSRECCON-003` was able to describe Scenario 33 even though Scenario 33 already existed in `golden_offices.rs`, because the current generated inventory tracks test names only, not scenario identity. This is a documentation/tooling architecture problem: the repo lacks a concrete, machine-checkable scenario map grounded in the real tests.

## Assumption Reassessment (2026-03-22)

1. The current generated inventory script at [`scripts/golden_inventory.py`](/home/joeloverbeck/projects/worldwake/scripts/golden_inventory.py) validates `golden_*` test names against `cargo test -p worldwake-ai -- --list` and doc references, but it does not track scenario numbers or scenario titles.
2. The current coverage/scenario docs already treat generated inventory as the canonical mechanical source for `golden_*` tests:
   - [`docs/golden-e2e-coverage.md`](/home/joeloverbeck/projects/worldwake/docs/golden-e2e-coverage.md)
   - [`docs/golden-e2e-scenarios.md`](/home/joeloverbeck/projects/worldwake/docs/golden-e2e-scenarios.md)
   The missing contract is scenario identity grounding, not raw test-name grounding.
3. Golden files already use human-readable scenario headers such as `// Scenario 33: ...` in [`crates/worldwake-ai/tests/golden_offices.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_offices.rs), but that metadata is not currently parsed or validated by tooling.
4. Existing ticket/doc precision guidance already requires live goal/operator reassessment and divergence-first correction, per [`tickets/README.md`](/home/joeloverbeck/projects/worldwake/tickets/README.md), [`docs/precision-rules.md`](/home/joeloverbeck/projects/worldwake/docs/precision-rules.md), and archived [`S18TICKETDOC-001`](/home/joeloverbeck/projects/worldwake/archive/tickets/completed/S18TICKETDOC-001.md). The remaining gap is not narrative precision alone; it is the absence of a concrete generated inventory for scenario identity.
5. This is not a planner-runtime or authoritative-engine ticket. The intended verification layer is documentation/tooling correctness. Additional runtime-layer mapping is not applicable beyond validating that the generated map is derived from the real golden test sources and remains docs-synced.
6. Ordering is not the contract here. The architectural requirement is source-of-truth grounding: generated scenario metadata should derive from real test files, and docs should consume that derived inventory instead of relying on hand-maintained scenario identity.
7. No heuristic is being weakened. This ticket introduces a missing documentation/tooling substrate so ticket/spec authors do not rely on memory or narrative cross-reference when identifying already-implemented scenarios.
8. Golden scenario isolation remains untouched. This ticket does not change scenario behavior; it changes how scenario identity is represented and validated.
9. Mismatch + correction: the current repo is stronger than a naive docs-only process because the generated `golden_*` inventory already exists, but it still does not cover the scenario-number/title dimension that drifted in `S19INSRECCON-003`.
10. The clean architecture consistent with [`docs/FOUNDATIONS.md`](/home/joeloverbeck/projects/worldwake/docs/FOUNDATIONS.md) Principle 25 is a generated scenario map derived from concrete test metadata. Hand-maintained scenario catalogs remain useful as summaries, but they should be downstream caches, not the source of identity truth.

## Architecture Check

1. A generated scenario metadata map is cleaner than adding more prose rules alone. It grounds scenario identity in concrete test source, which is more robust than relying on reviewers to manually notice scenario-number drift.
2. The right design is derived-summary architecture, not another hand-maintained registry. Scenario identity should be extracted from explicit metadata colocated with the owning tests, then rendered into generated docs and validated.
3. No backwards-compatibility aliases or duplicate “shadow catalogs” should be introduced. The generated inventory should become the canonical mechanical source, and existing docs should reference it rather than re-declaring the same mapping manually.

## Verification Layers

1. every active golden scenario has machine-readable scenario metadata -> source parser + focused script tests
2. generated scenario map reflects the real `golden_*.rs` inventory -> script output validated against source and `cargo test -p worldwake-ai -- --list`
3. docs only reference existing generated scenario identities and test names -> docs validation in inventory script
4. single-layer tooling/docs ticket; no additional runtime-layer mapping is applicable

## What to Change

### 1. Standardize explicit scenario metadata in golden test sources

Introduce a minimal, parseable metadata convention in `crates/worldwake-ai/tests/golden_*.rs` for numbered scenarios. The convention should encode at least:

- scenario number
- scenario title
- primary test name
- replay-companion test name when present

This metadata must be colocated with the owning test source so the generated map derives from concrete test definitions, not a separate hand-maintained manifest.

### 2. Extend generated inventory tooling to build a scenario map

Extend `scripts/golden_inventory.py` or add a companion script so the repo can generate and validate a scenario metadata artifact, for example:

- `docs/generated/golden-scenario-map.md`

The generated artifact should include:

- scenario number and title
- owning file
- primary and replay test names
- whether the replay companion exists

Validation should fail when:

- scenario numbers are duplicated
- a numbered scenario lacks its declared test(s)
- docs reference scenario numbers or test names not present in the generated map

### 3. Make the generated scenario map the canonical identity inventory

Update the golden docs and ticket authoring guidance so that numbered-scenario identity is taken from the generated scenario map, while high-level interpretation remains in the hand-written docs. This preserves the current “generated mechanics, written interpretation” split.

### 4. Add script-focused tests

Add focused regression tests for the scenario-metadata parser and validator so future formatting changes do not silently break the generated map.

## Files to Touch

- `scripts/golden_inventory.py` (modify) or a new companion script if separation is cleaner
- `docs/generated/` new generated scenario map artifact
- `docs/golden-e2e-coverage.md` (modify)
- `docs/golden-e2e-scenarios.md` (modify)
- `docs/golden-e2e-testing.md` (modify if the canonical inventory/source wording needs adjustment)
- `tickets/README.md` (modify to point golden ticket authors at the generated scenario map)
- `crates/worldwake-ai/tests/golden_*.rs` (modify only as needed to add standardized scenario metadata comments)
- `scripts/` focused script tests if the repo already has a pattern for them; otherwise add a minimal test file in an appropriate location

## Out of Scope

- Changing golden scenario behavior
- Renumbering existing scenarios except where reassessment proves a live inconsistency
- Engine/runtime changes
- Building a generic repository-wide ticket/spec inventory system beyond golden scenarios

## Acceptance Criteria

### Tests That Must Pass

1. Generated scenario map renders from live golden test sources and includes numbered scenario metadata
2. Validation fails on duplicate or missing scenario-number metadata
3. `python3 scripts/golden_inventory.py --write --check-docs` or the new equivalent scenario-map command passes with updated docs
4. Existing suite: `cargo test -p worldwake-ai -- --list`

### Invariants

1. Scenario identity inventory is derived from concrete test sources, not maintained as a parallel hand-written catalog
2. Generated docs remain caches of source truth and can be regenerated without changing world meaning
3. No backwards-compatibility alias layer is introduced between old and new scenario references; stale docs/tickets should be corrected directly

## Test Plan

### New/Modified Tests

1. Focused parser/validator tests for the scenario-metadata inventory tooling — prove the generated map catches duplicates, missing tests, and malformed metadata.
2. `None` at runtime — this ticket changes docs/tooling metadata only; existing golden/runtime coverage remains the behavior source of truth.

### Commands

1. `python3 scripts/golden_inventory.py --write --check-docs`
2. `cargo test -p worldwake-ai -- --list`
3. `scripts/verify.sh`
