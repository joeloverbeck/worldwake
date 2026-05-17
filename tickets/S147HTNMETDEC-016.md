# S147HTNMETDEC-016: Prove end-to-end HTN method-failure golden

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes - likely narrow golden fixture/action-failure wiring only if the live producer substrate needs one more reachable boundary.
**Deps**: `archive/tickets/S147HTNMETDEC-015.md` (selected-method plan provenance and typed method-failure producer)

## Problem

`S147HTNMETDEC-015` landed generated-candidate selector goldens for `FulfillBountyDirect` and `EscortToHome`, and it added the runtime substrate that can emit `Discrepancy::MethodFailure(MethodFailureContext)` for an active method-selected plan failure.

The remaining S147 D10 gap is an end-to-end or hybrid golden that drives a method-selected plan through the live action/start/revalidation failure boundary and observes the typed method-failure outcome without fabricating a method trace or discrepancy.

## Assumption Reassessment (2026-05-17)

1. `PlannedPlan.method_id` now persists the selected `MethodSchemaId` from strategic planning, so runtime failure handling can attribute failures to the selected HTN method after planning.
2. `handle_plan_failure` now maps otherwise-unclassified method-selected failures to `Discrepancy::MethodFailure(MethodFailureContext { kind: SubgoalUnachievable, subgoal_index: None, ... })`.
3. Existing proof is lower-layer producer coverage in `failure_handling`; it does not yet prove an autonomous golden path that reaches the producer through action execution or start-time revalidation.
4. Shared abstraction boundary under audit: generated method selection -> committed `PlannedPlan.method_id` -> action lifecycle failure -> discrepancy memory/trace observation.
5. The intended invariant is typed method-failure attribution for stale or unachievable method subgoals. Lawful competing branches such as generic target-gone, danger, commodity, or execution-failure classifications must be either intentionally excluded by fixture setup or accepted as the stronger specific classification.

## Architecture Check

1. The golden must use the live planning/action lifecycle and typed discrepancy path. It must not hand-construct `MethodPlanAttemptTrace.failure_mode` or directly insert a discrepancy.
2. If the strongest live proof remains below full golden level, update this ticket with the exact missing runtime observability boundary before closing it.
3. No backwards-compatibility shim or method-only special action path should be introduced.

## Verification Layers

1. Method selection -> generated candidate evidence and `MethodPlanAttemptTrace.method_id`.
2. Runtime carriage -> active `PlannedPlan.method_id` on the failing plan.
3. Failure attribution -> `Discrepancy::MethodFailure(MethodFailureContext)` observed through the strongest available live surface, preferably discrepancy memory or decision/action trace.
4. Golden metadata -> regenerated inventory/docs if `golden_htn_methods` scenario metadata changes.

## What to Change

### 1. Build a truthful failure fixture

Start from the direct-bounty or escort method path that can select deterministically. Create the smallest fixture that allows the method to be selected, then makes a later subgoal fail through live action/start/revalidation rather than through fabricated trace state.

### 2. Assert typed method failure

Assert that the observed failure is `Discrepancy::MethodFailure(MethodFailureContext)` with the selected method id. If a more specific live classification lawfully fires first, record that as reassessment and choose a fixture that isolates the method-subgoal failure without weakening production classification.

## Files to Touch

- `crates/worldwake-ai/tests/golden_htn_methods.rs` (modify)
- `crates/worldwake-ai/src/agent_tick/` (possible narrow failure-carriage fix)
- `crates/worldwake-ai/src/failure_handling.rs` (possible narrow classification fix)
- `docs/generated/golden-e2e-inventory.md` (regenerate if golden metadata changes)
- `docs/generated/golden-scenario-index.md` (regenerate if golden metadata changes)
- `docs/generated/golden-scenario-details/` (regenerate if golden metadata changes)
- `docs/generated/golden-coverage-matrix.md` (regenerate if golden metadata changes)
- `specs/S147-htn-method-decomposition.md` (truth-sync)
- `specs/IMPLEMENTATION-ORDER.md` (truth-sync)

## Out of Scope

- Reworking direct-bounty or escort selector coverage already landed by `S147HTNMETDEC-015`.
- Adding story-beat methods, fabricated traces, or direct discrepancy insertion.
- Broad observer formatting work unless it is the only truthful surface needed to observe the live typed discrepancy.

## Acceptance Criteria

### Tests That Must Pass

1. An end-to-end or hybrid golden proves a method-selected plan reaches the live failure producer and emits `Discrepancy::MethodFailure(MethodFailureContext)`.
2. The test asserts the selected method id and excludes or explains stronger lawful competing classifications.
3. `python3 scripts/golden_inventory.py --write --check-docs` passes if golden metadata changes.
4. `cargo test -p worldwake-ai --test golden_htn_methods` passes.
5. `cargo test -p worldwake-ai` passes.

### Invariants

1. The proof uses live generated candidate, method selection, plan-carriage, and failure paths.
2. Method failure remains a typed discrepancy produced by runtime failure handling, not a trace-only annotation.
3. Flat GOAP fallback and more specific failure classifications remain available.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_htn_methods.rs` - add the end-to-end or hybrid method-failure golden.
2. Focused unit/runtime tests only if the golden exposes a missing production failure-carriage edge.

### Commands

1. `cargo test -p worldwake-ai --test golden_htn_methods`
2. `python3 scripts/golden_inventory.py --write --check-docs`
3. `cargo test -p worldwake-ai`
