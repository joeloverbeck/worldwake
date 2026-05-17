# S147HTNMETDEC-014: Remaining non-production HTN D10 goldens

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: Yes - likely golden fixtures plus narrow candidate/method/action substrate fixes if live proof exposes missing belief/evidence bridges.
**Deps**: `archive/tickets/S147HTNMETDEC-013.md` (autonomous production method trace bridge), `archive/tickets/S147HTNMETDEC-009.md` (trace + diagnostics), `archive/tickets/S147HTNMETDEC-010.md` (observer method rendering)

## Problem

`S147HTNMETDEC-013` landed the autonomous `ProduceCommodity` method-trace bridge: generated production candidates carry source evidence, snapshot-backed method selection resolves recipe-input preconditions through the strategic planner's recipe registry, and `golden_htn_methods.rs` records `ProduceWithGather` from an autonomous generated planning attempt.

Before this ticket, the remaining S147 D10 contract was the non-production method narrative set: `FulfillBountyDirect`, `FulfillBountyInvestigation`, escort/failure, and typed `Discrepancy::MethodFailure(MethodFailureContext)` goldens. The implementation kept the original constraint that each path needs live candidate, legal setup, action, and failure substrate proof before any golden may claim it.

This ticket landed the stable report-backed `FulfillBountyInvestigation` slice. The remaining direct-bounty, escort, and typed method-failure narratives later landed through `archive/tickets/S147HTNMETDEC-015.md` and `archive/tickets/S147HTNMETDEC-016.md`.

## Assumption Reassessment (2026-05-17)

1. `crates/worldwake-ai/tests/golden_htn_methods.rs` now proves selector-level `ProduceWithGather`, disabled-method flat fallback, autonomous generated production evidence, snapshot-backed production method selection, autonomous production method trace, and deterministic replay.
2. The production evidence bridge is no longer the blocker for D10. The remaining boundary is per-goal: bounty, investigation, escort, and method-failure paths each need their own generated-candidate evidence, lawful setup, and action/failure proof surface.
3. Shared abstraction boundary under audit: generated `GoalOffer` evidence and `MethodSelector` preconditions for each non-production `MethodSchema`, then strategic method trace propagation into `PlanAttemptTrace.method_trace`.
4. Typed method failure must be proved at the strongest live boundary. If method failure does not currently produce `Discrepancy::MethodFailure(MethodFailureContext)` through a reachable action/planning failure path, split the missing producer substrate rather than asserting it through a hand-made trace.
5. Adjacent contradictions should be classified separately: missing bounty claim legal setup, absent witness/ledger evidence, escort lifecycle instability, or method-failure emission gaps are in scope only when required for the selected D10 narrative; otherwise create bounded follow-up tickets.
6. Implementation reassessment narrowed this ticket to the stable report-backed `FulfillBountyInvestigation` seam. Generated `FulfillBounty` candidates from reported bounty artifact beliefs select method id `2`, and an autonomous tick records method id `2` in `MethodPlanAttemptTrace`. A direct-observation fixture carried bounty/target evidence but did not satisfy `FulfillBountyDirect` through the current `TargetLastSeenKnown` selector/view boundary; escort and typed method-failure producers likewise still required their own substrate proof. Those paths later landed through `archive/tickets/S147HTNMETDEC-015.md` and `archive/tickets/S147HTNMETDEC-016.md`.

## Architecture Check

1. Each golden must prove a lawful method path from actor beliefs and generated candidates, not a hand-authored story beat.
2. Method selection remains belief-only and actor-relative; no method may query global world state to compensate for missing evidence.
3. No backwards-compatibility shim: missing evidence or failure propagation should be fixed at the canonical candidate/method/failure boundary or split to a named follow-up.

## Verified Layers

1. Bounty investigation method selection -> generated/report-backed candidate evidence plus `PlanAttemptTrace.method_trace`.
2. Direct-bounty, escort, and typed method-failure paths -> split to `archive/tickets/S147HTNMETDEC-015.md` because live substrate was not yet honest enough for generated-candidate goldens; final method-failure golden coverage later landed in `archive/tickets/S147HTNMETDEC-016.md`.
3. Golden metadata -> `python3 scripts/golden_inventory.py --write --check-docs`.
4. Affected AI behavior -> `cargo test -p worldwake-ai --test golden_htn_methods` and `cargo test -p worldwake-ai`.

## Landed Changes

### 1. Reassess each remaining D10 method family

Reassessed `FulfillBountyDirect`, `FulfillBountyInvestigation`, escort/failure, and typed method failure against live candidate generation, method preconditions, strategic trace, and action/failure boundaries before writing scenarios.

### 2. Add stable goldens only for honest live paths

Extended `crates/worldwake-ai/tests/golden_htn_methods.rs` with the stable report-backed `FulfillBountyInvestigation` selector and autonomous method-trace scenarios.

### 3. Split any missing substrate

Created the now-archived `archive/tickets/S147HTNMETDEC-015.md` for direct-bounty, escort, and typed method-failure substrate instead of forcing those narratives through fixtures.

## Landed Files

- `crates/worldwake-ai/tests/golden_htn_methods.rs` (modify)
- `docs/generated/golden-e2e-inventory.md` (regenerated)
- `docs/generated/golden-scenario-index.md` (regenerated)
- `docs/generated/golden-scenario-details/htn-methods.md` (regenerated)
- `docs/generated/golden-coverage-matrix.md` (regenerated)
- `archive/specs/S147-htn-method-decomposition.md` (truth-sync remaining D10 owner)
- `specs/IMPLEMENTATION-ORDER.md` (truth-sync remaining D10 owner)
- `archive/tickets/S147HTNMETDEC-015.md` (new follow-up for remaining direct/escort/failure substrate)

## Out of Scope

- Reworking production-method coverage already completed by `archive/tickets/S147HTNMETDEC-013.md`.
- Adding story-beat methods or method-only goals.
- Adding new `PlannerOpKind` variants unless reassessment proves a D10 method cannot be represented through existing lawful leaves.
- Performance regression gates.

## Acceptance Criteria

### Verification Result Summary

1. Stable non-production D10 goldens land for every remaining method/failure path that the live substrate can honestly prove in this pass: generated/report-backed `FulfillBountyInvestigation` selector and autonomous trace coverage.
2. Any unlanded original D10 narrative is assigned to a named follow-up with the exact missing substrate: `archive/tickets/S147HTNMETDEC-015.md`; final method-failure golden coverage later landed in `archive/tickets/S147HTNMETDEC-016.md`.
3. `python3 scripts/golden_inventory.py --write --check-docs` passes after golden metadata changes.
4. `cargo test -p worldwake-ai --test golden_htn_methods` passes.
5. `cargo test -p worldwake-ai` passes.

### Invariants

1. Methods remain lawful pursuit patterns, not story beats.
2. Generated candidates carry lawful evidence; methods do not query global world state to compensate.
3. Method failure proof uses a live typed failure producer, not a fabricated trace.
4. Flat GOAP fallback remains available when methods are disabled or no method preconditions match.

## Test Plan Result

### Modified Tests

1. `crates/worldwake-ai/tests/golden_htn_methods.rs` - generated/report-backed `FulfillBountyInvestigation` selector and autonomous method-trace coverage.
2. Focused lower-layer tests near any evidence or failure producer changed during implementation.

### Commands Run

1. `cargo test -p worldwake-ai --test golden_htn_methods`
2. `python3 scripts/golden_inventory.py --write --check-docs`
3. `cargo test -p worldwake-ai`

## Verification Result

1. Passed: `cargo test -p worldwake-ai --test golden_htn_methods`
2. Passed: `python3 scripts/golden_inventory.py --write --check-docs`
3. Passed: `cargo test -p worldwake-ai`

## Outcome

Completed: 2026-05-17

Landed two new S147 D10 scenario blocks in `crates/worldwake-ai/tests/golden_htn_methods.rs`:

1. Scenario 434 proves generated `FulfillBounty` candidate evidence from a reported bounty artifact selects `FulfillBountyInvestigation` through the snapshot-backed `MethodSelector`.
2. Scenario 435 proves the autonomous planning tick records `FulfillBountyInvestigation` in `MethodPlanAttemptTrace`.

The golden inventory/docs were regenerated. The original direct-bounty, escort, and typed `Discrepancy::MethodFailure(MethodFailureContext)` narratives remained real but needed stronger live selector/action/failure substrate; they were split to `archive/tickets/S147HTNMETDEC-015.md` instead of being forced through hand-constructed `GoalOffer` fixtures. Final method-failure golden coverage later landed in `archive/tickets/S147HTNMETDEC-016.md`.
