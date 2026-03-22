# S16S09GOLVAL-007: Golden Docs — Canonical Inventory and Docs Sync Validation

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes — docs/tooling workflow only; no simulation, planner, or authoritative runtime behavior changes
**Deps**: `specs/S16-s09-golden-validation.md`, `docs/golden-e2e-coverage.md`, `docs/golden-e2e-scenarios.md`, `docs/golden-e2e-testing.md`

## Problem

The golden E2E docs are still maintained mostly by hand. That was already risky when the suite was small; with the current suite shape, it now causes concrete drift between the authored docs and the live test inventory.

Reassessment found that the active S16/S09 goldens referenced by the spec are already implemented in the test suite, but the docs/tooling path did not keep pace. The scenario catalog already contains stale references, and there is still no canonical generated inventory artifact or validation command that checks whether the docs only reference real `golden_*` tests.

That leaves the project with an avoidable traceability gap: the architecture promises a stable golden catalog, but the docs can silently drift away from the compiled test binary.

## Assumption Reassessment (2026-03-21)

1. `specs/S16-s09-golden-validation.md` is no longer a "proposed work" reference for several items this ticket originally treated as future implementation. The current suite already contains `crates/worldwake-ai/tests/golden_combat.rs::golden_defend_changed_conditions`, `crates/worldwake-ai/tests/golden_ai_decisions.rs::golden_spatial_multi_hop_plan`, and their deterministic replay companions.
2. The original ticket assumption that "adding one new golden exposed stale counts and descriptions" was directionally right but underspecified. The deeper problem is not just counts; authored docs can reference stale test names. Current mismatch: `docs/golden-e2e-scenarios.md` still names `golden_materialized_output_theft_forces_replan`, while the current suite exposes `crates/worldwake-ai/tests/golden_production.rs::golden_materialized_output_ownership_prevents_theft`.
3. `cargo test -p worldwake-ai -- --list` currently succeeds and remains the authoritative compiled inventory cross-check. Reassessment confirmed it is the right binary-level verification surface for this ticket.
4. The current source-level inventory under `crates/worldwake-ai/tests/golden_*.rs` is 10 files total, 9 files contributing `golden_*` tests, and 129 `golden_*` functions overall. Those counts currently match the docs, but they are still duplicated manually and therefore still drift-prone.
5. No dedicated script currently owns this workflow. `scripts/` only contains `scripts/verify.sh`; there is no canonical inventory generator or docs-sync validator today.
6. This remains a docs/tooling ticket, not a production simulation ticket. Reassessment found no authoritative-world, action-handler, planner, or AI-runtime contradiction that would justify widening scope into engine code.
7. The clean architectural boundary is narrower than "rewrite the docs." Mechanical inventory should be derived from source plus compiled test listing. Narrative scenario explanations should stay authored. The ticket should add a canonical generated artifact and validation boundary, then repair the authored docs to align with current test names.
8. Verification layer note: this is a single-layer docs/tooling ticket. Decision traces, action traces, event-log ordering, and authoritative-world assertions are not the contract here; the contract is inventory derivation and docs-reference validity against the current golden suite.
9. Scenario-isolation guidance from `docs/golden-e2e-testing.md` remains normative and should stay authored. This ticket should support that guidance with a stable inventory source and validation command, not replace the document with generated prose.
10. Corrected scope: implement canonical golden inventory generation plus docs-reference validation, repair the stale doc references, and document the maintenance workflow. Do not implement or modify simulation goldens unless the validation work uncovers an actual missing test rather than a stale doc.

## Architecture Check

1. The cleaner long-term architecture is a split model:
   - generated mechanical truth: file inventory, per-file counts, concrete `golden_*` names
   - authored interpretation: coverage commentary, scenario explanations, testing conventions
   This is cleaner than either extreme of fully hand-maintained docs or fully generated prose.
2. Using the compiled test listing as a validation input is more robust than trusting source parsing alone. Source parsing tells us what is written; `cargo test -p worldwake-ai -- --list` tells us what the current binary actually exposes.
3. The right validation boundary for authored docs is reference validity, not brittle full-document regeneration. We should prove that every backticked `golden_*` name in the docs resolves to a real current test, while leaving authored narrative text intact.
4. No backwards-compatibility shims, aliases, or dual documentation paths should be introduced. The docs should move directly to the generated-inventory-plus-validation workflow.

## Verification Layers

1. Canonical golden inventory reflects current source files and compiled test binary -> focused tooling test plus `cargo test -p worldwake-ai -- --list`
2. Generated inventory artifact is deterministic for the same repo state -> focused tooling test / repeated generation diff check
3. Authored golden docs only reference real current `golden_*` tests -> focused tooling test / explicit docs validation command
4. Maintenance workflow is documented on the normative docs path -> doc review
5. Additional AI/runtime/action/world-state layer mapping is not applicable because this ticket does not change simulation behavior

## What to Change

### 1. Add a canonical golden inventory tool

Add a narrow script under `scripts/` that:

- scans `crates/worldwake-ai/tests/golden_*.rs` for `golden_*` functions
- optionally cross-checks them against `cargo test -p worldwake-ai -- --list`
- writes a checked-in generated inventory artifact with:
  - total file count
  - count of files contributing `golden_*` tests
  - total `golden_*` count
  - per-file counts
  - per-file `golden_*` test-name lists

### 2. Add docs-sync validation

Teach the same tool to validate the authored golden docs by checking that every backticked `golden_*` test reference in:

- `docs/golden-e2e-coverage.md`
- `docs/golden-e2e-scenarios.md`
- `docs/golden-e2e-testing.md`

resolves to a real current test in the canonical inventory.

### 3. Repair the docs and integrate the generated artifact

Revise the golden docs so the mechanical inventory references point to the generated artifact / validation command, and repair any stale test-name references and workflow notes discovered during reassessment.

### 4. Add focused tooling coverage

Add focused tests for the inventory/validation layer so the docs workflow itself is not purely manual and untested.

## Files to Touch

- `scripts/` (new inventory/validation tool and focused tests)
- `docs/golden-e2e-coverage.md` (modify)
- `docs/golden-e2e-scenarios.md` (modify)
- `docs/golden-e2e-testing.md` (modify)
- optionally `docs/generated/` or equivalent checked-in artifact path (new)

## Out of Scope

- Changing any simulation, planner, AI-runtime, or action execution behavior
- Rewriting all scenario prose into generated output
- Adding heavyweight documentation dependencies
- Re-scoping already completed S16/S09 golden implementation tickets

## Acceptance Criteria

### Tests That Must Pass

1. Focused inventory/validation test command for the new tooling passes
2. The inventory tool successfully regenerates or validates the current golden inventory
3. `cargo test -p worldwake-ai -- --list` succeeds and matches the canonical inventory cross-check path
4. `scripts/verify.sh` passes

### Invariants

1. Mechanical golden-suite facts come from current source plus compiled test listing, not from manually duplicated counts alone
2. The generated inventory artifact is deterministic for the same repo state
3. Authored docs do not reference nonexistent or renamed `golden_*` tests
4. Narrative scenario explanations remain authored and reviewable

## Test Plan

### New/Modified Tests

1. `scripts/test_golden_inventory.py` — proves source inventory extraction stays deterministic, per-binary compiled inventory parsing works, generated summary output stays stable, and stale doc references are rejected

### Commands

1. `python3 -m unittest scripts/test_golden_inventory.py`
2. `python3 scripts/golden_inventory.py --write --check-docs`
3. `cargo test -p worldwake-ai -- --list`
4. `scripts/verify.sh`

## Outcome

- Completion date: 2026-03-21
- What actually changed:
  - added `scripts/golden_inventory.py` as the canonical golden inventory generator and docs-sync validator
  - added `scripts/test_golden_inventory.py` as focused tooling coverage for the generator/validator layer
  - generated and checked in `docs/generated/golden-e2e-inventory.md`
  - updated `docs/golden-e2e-coverage.md`, `docs/golden-e2e-scenarios.md`, and `docs/golden-e2e-testing.md` to use the generated inventory workflow instead of hand-maintained mechanical counts
  - repaired the stale `golden_materialized_output_theft_forces_replan` scenario reference to the current `golden_materialized_output_ownership_prevents_theft` test
  - added a targeted `#[allow(clippy::too_many_lines)]` on the pre-existing `run_spatial_multi_hop_plan_scenario` helper in `crates/worldwake-ai/tests/golden_ai_decisions.rs` so the required repo-wide clippy baseline passes without refactoring unrelated scenario logic
- Deviations from original plan:
  - reassessment confirmed the referenced S16/S09 goldens were already implemented, so the ticket stayed docs/tooling-scoped rather than widening into new golden implementation
  - instead of trying to regenerate the authored scenario prose, the final design validates that authored docs only reference real current `golden_*` tests and moves the mechanical inventory into a generated artifact
- Verification results:
  - `python3 -m unittest scripts/test_golden_inventory.py` passed
  - `python3 scripts/golden_inventory.py --write --check-docs` passed
  - `cargo test -p worldwake-ai -- --list` passed
  - `bash scripts/verify.sh` passed
