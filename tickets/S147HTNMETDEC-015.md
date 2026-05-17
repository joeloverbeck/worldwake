# S147HTNMETDEC-015: Complete remaining direct, escort, and method-failure HTN goldens

**Status**: PENDING
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: Yes - likely narrow selector/view, escort candidate, or typed failure producer fixes before goldens can land.
**Deps**: `archive/tickets/S147HTNMETDEC-014.md` (report-backed `FulfillBountyInvestigation` golden slice)

## Problem

`S147HTNMETDEC-014` landed the stable report-backed `FulfillBountyInvestigation` D10 seam. The remaining original S147 non-production D10 narratives are still not honestly proven: `FulfillBountyDirect`, escort method selection/failure, and typed `Discrepancy::MethodFailure(MethodFailureContext)` production through a reachable planning/action failure path.

These paths must not be forced through hand-constructed `GoalOffer` traces. Each must prove the live generated-candidate, selector, strategic trace, and action/failure boundary that actually exists on the branch.

## Assumption Reassessment (2026-05-17)

1. `crates/worldwake-ai/tests/golden_htn_methods.rs` now proves `ProduceWithGather` selector/fallback, autonomous production method trace, and report-backed `FulfillBountyInvestigation` selector/autonomous trace coverage.
2. Direct-bounty reassessment during `S147HTNMETDEC-014` showed a generated direct-observation bounty candidate can carry bounty/target/place evidence, but `FulfillBountyDirect` did not select through the current `TargetLastSeenKnown` selector/view boundary. The exact gap must be rechecked before coding because it may belong in `RuntimeBeliefView`, `PlanningState`, `htn::selector`, or the candidate evidence bridge.
3. Escort method coverage still needs live generated `EscortToSafety` candidate evidence and stable action/failure lifecycle proof before any golden can claim method selection or failure.
4. Typed method failure must be proved through a reachable producer of `Discrepancy::MethodFailure(MethodFailureContext)`, not by fabricating `MethodPlanAttemptTrace.failure_mode`.
5. Shared abstraction boundary under audit: generated `GoalOffer` evidence -> `MethodSelector` preconditions -> strategic `MethodPlanAttemptTrace` -> live action/failure producer when failure is asserted.

## Architecture Check

1. The remaining goldens must preserve FND-14/FND-20: methods read actor-relative beliefs and encode lawful pursuit patterns, not story beats.
2. Any selector/view fix must keep one canonical information path for target location, escortee state, and failure attribution.
3. No backwards-compatibility shim: if a method cannot be represented from current lawful evidence, add the missing canonical substrate or narrow the ticket again with a new owner.

## Verification Layers

1. `FulfillBountyDirect` selection -> generated candidate evidence plus snapshot-backed `MethodSelector` proof, then `PlanAttemptTrace.method_trace` if autonomous planning is stable.
2. Escort method selection/failure -> generated escort candidate evidence plus decision/action/failure trace at the live boundary.
3. Typed method failure -> event-log/discrepancy-memory or lower producer proof that emits `Discrepancy::MethodFailure(MethodFailureContext)` through reachable runtime behavior.
4. Golden metadata -> `python3 scripts/golden_inventory.py --write --check-docs`.
5. Affected AI behavior -> `cargo test -p worldwake-ai --test golden_htn_methods` and `cargo test -p worldwake-ai`.

## What to Change

### 1. Reassess direct bounty selection

Trace the generated `FulfillBounty` direct-observation fixture from candidate evidence through `TargetLastSeenKnown`. Fix the canonical selector/view/snapshot bridge only if the live belief path lawfully contains the target location.

### 2. Reassess escort method coverage

Verify whether generated `EscortToSafety` candidates carry the escortee/destination evidence needed by methods 12 and 13. Add goldens only for stable live paths.

### 3. Reassess typed method failure production

Find the live boundary that can emit `Discrepancy::MethodFailure(MethodFailureContext)` from a method-selected plan. If no reachable producer exists, add the missing producer substrate before golden assertions.

## Files to Touch

- `crates/worldwake-ai/tests/golden_htn_methods.rs` (modify)
- `crates/worldwake-ai/src/htn/selector.rs` (possible selector bridge)
- `crates/worldwake-ai/src/planning_state.rs` or `crates/worldwake-ai/src/planning_snapshot.rs` (possible snapshot/view bridge)
- `crates/worldwake-ai/src/candidate_generation.rs` (possible evidence propagation)
- `crates/worldwake-ai/src/search/strategic.rs` (possible trace propagation)
- `crates/worldwake-ai/src/agent_tick/` (possible typed failure producer)
- `docs/generated/golden-e2e-inventory.md` (regenerated if golden metadata changes)
- `docs/generated/golden-scenario-index.md` (regenerated if golden metadata changes)
- `docs/generated/golden-scenario-details/` (regenerated if golden metadata changes)
- `docs/generated/golden-coverage-matrix.md` (regenerated if golden metadata changes)
- `specs/S147-htn-method-decomposition.md` (truth-sync)
- `specs/IMPLEMENTATION-ORDER.md` (truth-sync)

## Out of Scope

- Reworking production-method and report-backed `FulfillBountyInvestigation` coverage already landed by earlier S147 tickets.
- Adding story-beat methods or method-only goals.
- Fabricating method failures directly in traces without a live producer.

## Acceptance Criteria

### Tests That Must Pass

1. Stable goldens land for every remaining direct/escort/failure method path that live substrate can honestly prove.
2. Any still-unlanded original D10 narrative is assigned to a named follow-up with the exact missing substrate.
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

1. `crates/worldwake-ai/tests/golden_htn_methods.rs` - remaining direct/escort/failure D10 scenario coverage where live substrate is stable.
2. Focused lower-layer tests near any evidence, selector, snapshot, or failure producer changed during implementation.

### Commands

1. `cargo test -p worldwake-ai --test golden_htn_methods`
2. `python3 scripts/golden_inventory.py --write --check-docs`
3. `cargo test -p worldwake-ai`
