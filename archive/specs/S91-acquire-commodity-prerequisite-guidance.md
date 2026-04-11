# S91: AcquireCommodity Prerequisite Guidance

**Status**: COMPLETED

## Summary

Investigate and fix the planner budget exhaustion reproduced by the Dusty Trail / Thornwall Village golden in `crates/worldwake-ai/tests/golden_planner_pathology.rs`. The live failure is not a missing water affordance or a raw search-budget tuning issue. Purely remote `AcquireCommodity { commodity: Water, purpose: SelfConsume }` currently enters tactical search as a broad goal-place/exploration problem, so the root expansion keeps the full acquire operator family alive at depth 0 instead of first committing to the known prerequisite destination. This spec makes remote-only known commodity acquisition use the same prerequisite-guided tactical contract already used by `ProduceCommodity` and `TreatWounds`, without disturbing local-vs-remote seller reranking, then flips the Dusty Trail golden into the permanent proof that the scenario stays fixed.

## Phase

Phase 7: Consequence Carriers

## Crates

- `worldwake-ai` (strategic planning contract, tactical candidate narrowing, focused planner coverage, golden proof)

## Dependencies

- `specs/S91-planner-pathology-golden-tests.md`
- `archive/tickets/S91PLAPATGOL-001.md`
- `docs/planner-contracts.md`

## Problem Statement

The Dusty Trail golden proves a lawful water source is known at `Thornwall Village`, but the planner still returns `PlanSearchOutcome::BudgetExhausted` for `AcquireCommodity(Water)`. The live root cause is architectural:

1. `search::strategic::plan()` only emits `StrategicStageKind::Acquire(..)` for `ProduceCommodity` recipe inputs and `TreatWounds` medicine prerequisites.
2. `AcquireCommodity` with known remote supply and no current-place acquisition route therefore falls back to a plain goal-place stage, producing `TacticalSubGoal::SatisfyGoal` / `TravelToGoal` instead of `AcquirePrerequisite`.
3. `apply_tactical_candidate_filter()` deliberately keeps root relevant non-travel operators alive for `TravelToGoal` at depth 0.
4. For `AcquireCommodity(SelfConsume)`, that means root search keeps `Trade`, `QueueForFacilityUse`, `Harvest`, `Craft`, and `MoveCargo` candidate families live before the actor has even traveled to the known remote place.
5. In the Dusty Trail scenario that broad root fan-out overwhelms `max_candidates_per_expansion`, causing budget exhaustion before the planner commits to the obvious Thornwall travel step.

The clean fix is to correct the strategic/tactical contract for known commodity acquisition, not to raise planner budgets or add a special-case Dusty Trail workaround.

## Design Goals

- Make purely remote known `AcquireCommodity` supply enter tactical search as `AcquirePrerequisite(commodity)` when the actor does not already possess the commodity.
- Preserve exploration/social-query fallback when no lawful acquisition place is known.
- Preserve existing local-vs-remote trade reranking when the actor already has a current-place acquisition candidate.
- Preserve belief-only planning and perceived-travel guidance.
- Convert the Dusty Trail golden from bug reproduction into the permanent regression proof for the fixed behavior.

## Non-Goals

- Increasing `CognitiveProfile.max_node_expansions` or `max_candidates_per_expansion`
- Adding Dusty Trail-specific heuristics or one-off candidate filters
- Broadly redefining `AcquireCommodity` operator families
- Changing retry semantics for `BudgetExhausted` in the runtime cache
- Implementing the other S91 planner-pathology goldens

## FOUNDATIONS Alignment

| Principle | Alignment |
|-----------|-----------|
| P3 (Concrete State) | The fix routes known commodity acquisition through concrete prerequisite-place state, not through a new abstract score or heuristic bucket |
| P7 (Locality) | Strategic and tactical guidance continues to use belief-backed `goal_relevant_places()` / perceived travel costs only |
| P8 (Preconditions) | The actor must still physically travel to the known water source before harvest/trade/queue actions become relevant |
| P12 (State-mediated systems) | The change stays inside planner state and search contracts; no direct system-to-system bypass is introduced |
| P28 (No Backward Compat) | Replace the mis-scoped strategic contract directly; do not add a parallel “special acquire guidance” path |

## Reassessed Boundary

- **Live goal kind under audit**: `GoalKind::AcquireCommodity { commodity, purpose: CommodityPurpose::SelfConsume }`
- **Strategic boundary under audit**: `crates/worldwake-ai/src/search/strategic.rs::plan()` and `missing_commodities()`
- **Tactical boundary under audit**: `crates/worldwake-ai/src/search/mod.rs::TacticalGoal`, `apply_tactical_candidate_filter()`, and root candidate admission before the candidate-cap guard
- **Permanent proof boundary**: the Dusty Trail golden in `crates/worldwake-ai/tests/golden_planner_pathology.rs`

## Deliverables

### 1. Treat known commodity acquisition as a prerequisite-guided strategic stage

Update `crates/worldwake-ai/src/search/strategic.rs` so `AcquireCommodity` contributes a missing-commodity prerequisite stage whenever the actor does not already hold the requested commodity, known acquisition places exist, and the acquisition remains purely remote from the actor's current place.

Expected contract:

- If the actor already has the commodity, no prerequisite stage is needed.
- If the actor lacks the commodity, known acquisition places exist, and none of those places is the actor's current place, the first strategic step becomes `TacticalSubGoal::AcquirePrerequisite(commodity)` at the chosen destination.
- If the actor already has a current-place acquisition candidate, strategic planning leaves `AcquireCommodity` as a direct tactical problem so seller reranking can still choose among local and remote opportunities after source-reliability updates.
- If no lawful acquisition place is known, existing `ExploreWithBarrier` / `SocialQuery` fallback behavior remains intact.

### 2. Keep tactical root search narrow until the actor reaches the prerequisite destination

Rely on the existing `AcquirePrerequisite` tactical filter path to keep only destination-advancing travel candidates alive before arrival, and only allow non-travel acquisition operators after the actor reaches the prerequisite destination.

This is a contract correction, not a new planner-only side path.

### 3. Add focused lower-layer planner coverage

Add or update focused planner tests proving:

- purely remote known `AcquireCommodity(SelfConsume)` uses `AcquirePrerequisite` rather than `SatisfyGoal`
- the traced tactical goal for known remote acquisition is `AcquirePrerequisite { commodity, destination }`
- a current-place acquire opportunity does not force prerequisite staging and therefore leaves local-vs-remote reranking intact
- fallback exploration/social-query behavior still holds when known acquisition places are absent

### 4. Flip the Dusty Trail golden into the permanent fixed-state proof

Update `crates/worldwake-ai/tests/golden_planner_pathology.rs` so the Dusty Trail scenario now proves the repaired behavior:

- an early `AcquireCommodity(Water)` attempt reaches `PlanSearchOutcome::Found`
- the resulting committed chain lawfully travels to Thornwall and commits `drink`
- thirst falls below the starting level within the test window
- the scenario no longer relies on `BudgetExhausted` as the passing condition

Update the scenario metadata comments and regenerate the golden inventory/docs.

## SystemFn Integration

- None. This spec changes planner-local search behavior and golden coverage only.

## Component Registration

- None. No new components or registration surfaces are introduced.

## Verification Plan

### Focused lower-layer coverage

1. `crates/worldwake-ai/src/search/strategic.rs` — strategic tests for known remote `AcquireCommodity(SelfConsume)` prerequisite staging
2. `crates/worldwake-ai/src/search/tests.rs` — trace/search tests proving `AcquirePrerequisite` tactical guidance for known remote acquisition

### Golden proof

1. `crates/worldwake-ai/tests/golden_planner_pathology.rs` — Dusty Trail scenario proves the fixed cross-location water plan

### Commands

1. `cargo test -p worldwake-ai --lib search::strategic`
2. `cargo test -p worldwake-ai --lib search::tests`
3. `cargo test -p worldwake-ai --test golden_planner_pathology`
4. `cargo test -p worldwake-ai`
5. `python3 scripts/golden_inventory.py --write --check-docs`
6. `cargo clippy --workspace --all-targets -- -D warnings`

## FND-01 Section H

### Information-path analysis

No new information path is introduced. The planner continues to derive acquisition places from belief-backed `PlanningState` / `PlanningSnapshot` data (`goal_relevant_places`, believed resource sources, believed sellers, perceived travel costs). The fix only changes how already-known commodity places are staged into strategic/tactical search.

### Positive-feedback analysis

No new positive-feedback loop is introduced. The change narrows planner branching for a known acquisition contract; it does not create a new world-state accumulation loop.

### Concrete dampeners

Not applicable. No new amplifying loop is added.

### Stored state vs. derived read-model list

**Stored state**

- existing belief-backed snapshot inputs: believed places, believed resource sources, believed merchandise, carried commodity quantities
- existing golden scenario state: topology, workstation/resource entities, homeostatic needs, patrol/intention profiles

**Derived**

- strategic stage selection for `AcquireCommodity`
- `TacticalGoal::AcquirePrerequisite { commodity, destination }`
- root candidate filtering before candidate-cap enforcement
- decision trace / golden assertions proving found plan, committed drink, and thirst reduction

## Outcome

- Completed: 2026-04-11
- What changed:
  - narrowed `AcquireCommodity(SelfConsume)` prerequisite staging so it activates only for purely remote acquisition problems
  - fixed the Dusty Trail / Thornwall Village planner pathology in production search code
  - rewrote the Dusty Trail golden from bug reproduction into permanent fixed-state proof
  - added focused planner coverage for remote prerequisite staging and preservation of local-vs-remote reranking
- Deviations from original plan:
  - the first implementation pass applied prerequisite staging too broadly and regressed trade rerouting; the landed fix narrowed the contract to remote-only acquisition instead of all known acquisition
- Verification results:
  - `cargo test -p worldwake-ai golden_trade_rejection_reroutes_to_reliable_seller --test golden_trade`
  - `cargo test -p worldwake-ai cross_location_water_acquisition_succeeds_without_budget_exhaustion --test golden_planner_pathology`
  - `cargo test -p worldwake-ai`
  - `python3 scripts/golden_inventory.py --write --check-docs`
  - `cargo clippy --workspace --all-targets -- -D warnings`
