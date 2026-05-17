# S147HTNMETDEC-016: Prove end-to-end HTN method-failure golden

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes - hybrid golden fixture for generated method selection, active-plan method carriage, and runtime typed failure production.
**Deps**: `archive/tickets/S147HTNMETDEC-015.md` (selected-method plan provenance and typed method-failure producer)

## Problem

`S147HTNMETDEC-015` landed generated-candidate selector goldens for `FulfillBountyDirect` and `EscortToHome`, and it added the runtime substrate that can emit `Discrepancy::MethodFailure(MethodFailureContext)` for an active method-selected plan failure.

This ticket closed the remaining S147 D10 gap with a hybrid golden that starts from a generated `EscortToSafety` candidate, selects `EscortToHome`, carries that selected method id on an active `PlannedPlan`, and observes the normal `handle_plan_failure` path recording `Discrepancy::MethodFailure(MethodFailureContext)`.

## Assumption Reassessment (2026-05-17)

1. `PlannedPlan.method_id` persists the selected `MethodSchemaId` from strategic planning, so runtime failure handling can attribute failures to the selected HTN method after planning.
2. `handle_plan_failure` maps otherwise-unclassified method-selected failures to `Discrepancy::MethodFailure(MethodFailureContext { kind: SubgoalUnachievable, subgoal_index: None, ... })`.
3. A fully autonomous action-start failure fixture was not needed to prove the remaining substrate boundary. The landed hybrid golden derives method selection from live generated candidate evidence, then exercises the same public runtime failure producer used by action/start/revalidation paths.
4. Shared abstraction boundary proven: generated method selection -> committed `PlannedPlan.method_id` -> runtime failure handling -> `DiscrepancyMemory`.
5. The fixture intentionally uses a targetless `Sleep` failed step after method selection so stronger lawful classifications such as target-gone, commodity, danger, trade, or execution-failure mappings do not supersede the method-failure classification.

## Architecture Check

1. The proof uses live generated candidate, snapshot-backed `MethodSelector`, active plan method carriage, and typed discrepancy recording. It does not directly insert a discrepancy or fabricate a method trace.
2. The hybrid shape is the strongest stable live boundary for D10 method-failure attribution without widening this ticket into broader action lifecycle fixture design.
3. No backwards-compatibility shim or method-only special action path was introduced.

## Verified Layers

1. Method selection -> generated `EscortToSafety` candidate evidence and selected method id `EscortToHome`.
2. Runtime carriage -> selected method id copied onto active `PlannedPlan.method_id`.
3. Failure attribution -> `handle_plan_failure` records `Discrepancy::MethodFailure(MethodFailureContext)` in `DiscrepancyMemory`.
4. Golden metadata -> regenerated inventory/docs include Scenario 438.
5. Affected AI behavior -> `cargo test -p worldwake-ai --test golden_htn_methods` and `cargo test -p worldwake-ai`.

## Landed Changes

### 1. Hybrid method-failure golden

`crates/worldwake-ai/tests/golden_htn_methods.rs` now has Scenario 438, `method_selected_failure_records_method_failure_discrepancy`. It starts from generated escort candidate evidence, selects `EscortToHome`, carries that method id on a `PlannedPlan`, and asserts the typed discrepancy recorded by the runtime failure handler.

### 2. Generated docs

The golden inventory/docs were regenerated. The generated totals are now `53 files, 53 contributing files, 254 tests, 196 scenario blocks`.

## Files Touched

- `crates/worldwake-ai/tests/golden_htn_methods.rs` (modified)
- `docs/generated/golden-e2e-inventory.md` (regenerated)
- `docs/generated/golden-scenario-index.md` (regenerated)
- `docs/generated/golden-scenario-details/htn-methods.md` (regenerated)
- `docs/generated/golden-coverage-matrix.md` (regenerated)
- `specs/S147-htn-method-decomposition.md` (truth-sync)
- `specs/IMPLEMENTATION-ORDER.md` (truth-sync)

## Out of Scope

- Reworking direct-bounty or escort selector coverage already landed by `S147HTNMETDEC-015`.
- Adding story-beat methods, fabricated traces, or direct discrepancy insertion.
- Broad observer formatting work.

## Completed Acceptance

### Tests That Passed

1. A hybrid golden proves a generated method-selected plan reaches the live failure producer and emits `Discrepancy::MethodFailure(MethodFailureContext)`.
2. The test asserts selected method id `EscortToHome` and intentionally excludes stronger lawful competing classifications with a targetless failed `Sleep` step.
3. `python3 scripts/golden_inventory.py --write --check-docs` passed after golden metadata changes.
4. `cargo test -p worldwake-ai --test golden_htn_methods` passed.
5. `cargo test -p worldwake-ai` passed.

### Invariants

1. The proof uses live generated candidate, method selection, plan-carriage, and failure paths.
2. Method failure remains a typed discrepancy produced by runtime failure handling, not a trace-only annotation.
3. Flat GOAP fallback and more specific failure classifications remain available.

## Test Plan Result

### Landed Tests

1. `crates/worldwake-ai/tests/golden_htn_methods.rs` - Scenario 438 hybrid method-failure golden.

### Observed Commands

1. `cargo test -p worldwake-ai --test golden_htn_methods`
2. `python3 scripts/golden_inventory.py --write --check-docs`
3. `cargo test -p worldwake-ai`

## Outcome

Completed on 2026-05-17.

- Landed Scenario 438, proving generated escort method selection, `PlannedPlan.method_id` carriage, and runtime typed method-failure recording.
- Regenerated golden inventory/docs for the new method-failure scenario metadata.
- This closes the active S147 D10 remainder created after `S147HTNMETDEC-015`.

## Deviations

- The proof is hybrid rather than a fully autonomous action-start golden. That is intentional: it starts from live generated candidate selection and then exercises the public runtime failure producer directly, avoiding a brittle action lifecycle fixture while still proving the remaining typed method-failure attribution boundary.

## Verification Result

- Passed `cargo test -p worldwake-ai --test golden_htn_methods` (39 tests).
- Passed `python3 scripts/golden_inventory.py --write --check-docs` (`53 files, 53 contributing files, 254 tests, 196 scenario blocks`).
- Passed `cargo test -p worldwake-ai`.
