# S147HTNMETDEC-014: Remaining non-production HTN D10 goldens

**Status**: PENDING
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: Yes - likely golden fixtures plus narrow candidate/method/action substrate fixes if live proof exposes missing belief/evidence bridges.
**Deps**: `archive/tickets/S147HTNMETDEC-013.md` (autonomous production method trace bridge), `archive/tickets/S147HTNMETDEC-009.md` (trace + diagnostics), `archive/tickets/S147HTNMETDEC-010.md` (observer method rendering)

## Problem

`S147HTNMETDEC-013` landed the autonomous `ProduceCommodity` method-trace bridge: generated production candidates carry source evidence, snapshot-backed method selection resolves recipe-input preconditions through the strategic planner's recipe registry, and `golden_htn_methods.rs` records `ProduceWithGather` from an autonomous generated planning attempt.

The remaining S147 D10 contract is the non-production method narrative set: `FulfillBountyDirect`, `FulfillBountyInvestigation`, escort/failure, and typed `Discrepancy::MethodFailure(MethodFailureContext)` goldens. These should be implemented only where the live candidate, legal setup, action, and failure substrates can prove the method path without hand-constructed `GoalOffer` distortion.

## Assumption Reassessment (2026-05-17)

1. `crates/worldwake-ai/tests/golden_htn_methods.rs` now proves selector-level `ProduceWithGather`, disabled-method flat fallback, autonomous generated production evidence, snapshot-backed production method selection, autonomous production method trace, and deterministic replay.
2. The production evidence bridge is no longer the blocker for D10. The remaining boundary is per-goal: bounty, investigation, escort, and method-failure paths each need their own generated-candidate evidence, lawful setup, and action/failure proof surface.
3. Shared abstraction boundary under audit: generated `GoalOffer` evidence and `MethodSelector` preconditions for each non-production `MethodSchema`, then strategic method trace propagation into `PlanAttemptTrace.method_trace`.
4. Typed method failure must be proved at the strongest live boundary. If method failure does not currently produce `Discrepancy::MethodFailure(MethodFailureContext)` through a reachable action/planning failure path, split the missing producer substrate rather than asserting it through a hand-made trace.
5. Adjacent contradictions should be classified separately: missing bounty claim legal setup, absent witness/ledger evidence, escort lifecycle instability, or method-failure emission gaps are in scope only when required for the selected D10 narrative; otherwise create bounded follow-up tickets.

## Architecture Check

1. Each golden must prove a lawful method path from actor beliefs and generated candidates, not a hand-authored story beat.
2. Method selection remains belief-only and actor-relative; no method may query global world state to compensate for missing evidence.
3. No backwards-compatibility shim: missing evidence or failure propagation should be fixed at the canonical candidate/method/failure boundary or split to a named follow-up.

## Verification Layers

1. Bounty direct/investigation method selection -> generated candidate evidence plus `PlanAttemptTrace.method_trace`.
2. Escort method selection/failure -> generated escort candidate evidence plus decision trace or action/failure trace at the live boundary.
3. Typed method failure -> event-log/discrepancy-memory proof if the live execution path emits `Discrepancy::MethodFailure(MethodFailureContext)`.
4. Golden metadata -> `python3 scripts/golden_inventory.py --write --check-docs`.
5. Affected AI behavior -> `cargo test -p worldwake-ai --test golden_htn_methods` and `cargo test -p worldwake-ai`.

## What to Change

### 1. Reassess each remaining D10 method family

For `FulfillBountyDirect`, `FulfillBountyInvestigation`, escort/failure, and typed method failure, verify the live candidate generation, method preconditions, strategic trace, and action/failure boundary before writing scenarios.

### 2. Add stable goldens only for honest live paths

Extend `crates/worldwake-ai/tests/golden_htn_methods.rs` with the non-production scenarios that can be proven from generated candidates and lawful belief/evidence setup.

### 3. Split any missing substrate

If a D10 narrative depends on absent legal setup, missing evidence propagation, unstable action lifecycle, or no typed method-failure producer, create a narrower follow-up and truth-sync this ticket/spec rather than forcing the golden through fixtures.

## Files to Touch

- `crates/worldwake-ai/tests/golden_htn_methods.rs` (modify)
- `crates/worldwake-ai/src/candidate_generation.rs` (possible evidence propagation fix)
- `crates/worldwake-ai/src/htn/selector.rs` (possible precondition bridge fix)
- `crates/worldwake-ai/src/search/strategic.rs` (possible method trace propagation fix)
- `crates/worldwake-ai/src/agent_tick/` (possible typed failure propagation fix)
- `docs/generated/golden-e2e-inventory.md` (regenerated if golden metadata changes)
- `docs/generated/golden-scenario-index.md` (regenerated if golden metadata changes)
- `docs/generated/golden-scenario-details/` (regenerated if golden metadata changes)
- `docs/generated/golden-coverage-matrix.md` (regenerated if golden metadata changes)
- `specs/S147-htn-method-decomposition.md` (truth-sync if any D10 narrative splits again)
- `specs/IMPLEMENTATION-ORDER.md` (truth-sync if any D10 narrative splits again)

## Out of Scope

- Reworking production-method coverage already completed by `archive/tickets/S147HTNMETDEC-013.md`.
- Adding story-beat methods or method-only goals.
- Adding new `PlannerOpKind` variants unless reassessment proves a D10 method cannot be represented through existing lawful leaves.
- Performance regression gates.

## Acceptance Criteria

### Tests That Must Pass

1. Stable non-production D10 goldens land for every remaining method/failure path that the live substrate can honestly prove.
2. Any unlanded original D10 narrative is assigned to a named follow-up with the exact missing substrate.
3. `python3 scripts/golden_inventory.py --write --check-docs` passes after golden metadata changes.
4. `cargo test -p worldwake-ai --test golden_htn_methods` passes.
5. `cargo test -p worldwake-ai` passes.

### Invariants

1. Methods remain lawful pursuit patterns, not story beats.
2. Generated candidates carry lawful evidence; methods do not query global world state to compensate.
3. Method failure proof uses a live typed failure producer, not a fabricated trace.
4. Flat GOAP fallback remains available when methods are disabled or no method preconditions match.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_htn_methods.rs` - remaining non-production D10 scenario coverage where live substrate is stable.
2. Focused lower-layer tests near any evidence or failure producer changed during implementation.

### Commands

1. `cargo test -p worldwake-ai --test golden_htn_methods`
2. `python3 scripts/golden_inventory.py --write --check-docs`
3. `cargo test -p worldwake-ai`
