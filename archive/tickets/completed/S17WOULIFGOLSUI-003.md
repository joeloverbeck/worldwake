# S17WOULIFGOLSUI-003: Golden E2E Docs Catch-Up for Scenarios 29 and 30

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: None
**Deps**: S17WOULIFGOLSUI-001, S17WOULIFGOLSUI-002, `specs/S17-wound-lifecycle-golden-suites.md`

## Problem

After Scenario 29 and Scenario 30 landed, the golden E2E documentation needed to reflect the new coverage so the hand-maintained dashboard/catalog would stay aligned with the implemented suites and generated inventory.

## Assumption Reassessment (2026-03-21)

1. The scenario tests already exist in the live suite: `golden_deprivation_wound_worsening_consolidates_not_duplicates` in [crates/worldwake-ai/tests/golden_emergent.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_emergent.rs) and `golden_recovery_aware_boost_eats_before_wash` in [crates/worldwake-ai/tests/golden_combat.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_combat.rs). Their deterministic replay companions also exist.
2. The docs this ticket originally targeted already exist and are already updated: [docs/golden-e2e-coverage.md](/home/joeloverbeck/projects/worldwake/docs/golden-e2e-coverage.md) includes Scenario 30 in the coverage matrix and the current summary count of 133 tests, while [docs/golden-e2e-scenarios.md](/home/joeloverbeck/projects/worldwake/docs/golden-e2e-scenarios.md) already contains Scenario 29 and Scenario 30 entries.
3. The generated inventory path is real and already aligned: [docs/generated/golden-e2e-inventory.md](/home/joeloverbeck/projects/worldwake/docs/generated/golden-e2e-inventory.md) lists both scenario tests, and `python3 scripts/golden_inventory.py --write --check-docs` passes against the current repo state.
4. This is not an AI-runtime or production-architecture ticket. The intended deliverable was documentation parity with already-implemented tests. No candidate-generation, ranking, planner, runtime, or authoritative-system changes are warranted by the current architecture.
5. Mismatch corrected: the ticket can no longer truthfully claim a remaining docs gap. The repo state shows the docs work was effectively delivered already, so the correct scope is completion metadata plus archival, not further documentation or code edits.

## Architecture Check

1. The current architecture is the better one. Scenario behavior lives in the tests, the mechanical inventory lives in the generated artifact, and the hand-maintained docs stay interpretive instead of duplicating the full inventory. Adding more documentation machinery or moving coverage facts into another source of truth would be worse, not cleaner.
2. No backwards-compatibility aliasing or shims are needed. The correct response to a stale docs ticket is to update the ticket/spec record and archive them, not to introduce duplicate documentation paths.

## Verification Layers

1. Scenario 29 exists in the live golden suite -> focused test listing and exact targeted test run
2. Scenario 30 exists in the live golden suite -> focused test listing and exact targeted test run
3. Hand-maintained docs already describe Scenarios 29 and 30 -> manual document review
4. Generated inventory matches real `golden_*` declarations -> `python3 scripts/golden_inventory.py --write --check-docs`
5. Single-layer documentation/recordkeeping ticket; no additional runtime proof surfaces apply

## What to Change

### 1. Correct the ticket scope

Record that the docs catch-up is already present in the repository and that no production, test, or hand-maintained doc content changes are still required for S17.

### 2. Finalize and archive

Mark this ticket completed with an `Outcome` section, archive it under `archive/tickets/completed/`, and archive the completed S17 spec under `archive/specs/`.

## Files to Touch

- `tickets/S17WOULIFGOLSUI-003.md` (modify, then archive)
- `specs/S17-wound-lifecycle-golden-suites.md` (modify with completion metadata, then archive)

## Out of Scope

- Any production code changes
- Any test code changes
- Any new golden documentation content beyond what already exists
- Introducing new generated-doc or coverage-dashboard architecture

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-ai golden_deprivation_wound_worsening_consolidates_not_duplicates -- --exact`
2. `cargo test -p worldwake-ai golden_recovery_aware_boost_eats_before_wash -- --exact`
3. `python3 scripts/golden_inventory.py --write --check-docs`
4. `cargo test --workspace`
5. `cargo clippy --workspace --all-targets -- -D warnings`

### Invariants

1. Scenario 29 and Scenario 30 remain present in the live golden suite
2. The hand-maintained golden docs remain aligned with those scenarios
3. The generated inventory remains aligned with the actual `golden_*` declarations
4. No production or test code changes are required for this ticket

## Test Plan

### New/Modified Tests

1. `None — documentation/archival ticket; verification relies on existing golden tests and docs-sync validation.`

### Commands

1. `cargo test -p worldwake-ai golden_deprivation_wound_worsening_consolidates_not_duplicates -- --exact`
2. `cargo test -p worldwake-ai golden_recovery_aware_boost_eats_before_wash -- --exact`
3. `python3 scripts/golden_inventory.py --write --check-docs`
4. `cargo test --workspace`
5. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

- Completion date: 2026-03-21
- What changed:
  - Reassessed the live repo state against the ticket assumptions
  - Confirmed Scenario 29 and Scenario 30 already exist in the golden suite
  - Confirmed the hand-maintained docs and generated inventory were already aligned
  - Updated this ticket to reflect the corrected scope and prepared it for archival
- Deviations from original plan:
  - No additional documentation edits were needed because the repo had already absorbed the intended docs catch-up before this reassessment
  - No code or test changes were warranted; the architecture was already in the cleaner end state
- Verification results:
  - `cargo test -p worldwake-ai golden_deprivation_wound_worsening_consolidates_not_duplicates -- --exact` passed
  - `cargo test -p worldwake-ai golden_recovery_aware_boost_eats_before_wash -- --exact` passed
  - `python3 scripts/golden_inventory.py --write --check-docs` passed
