# S16S09GOLVAL-004: Golden — Spatial Awareness Enables Default-Budget Multi-Hop Plan

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: None
**Deps**: `specs/S16-s09-golden-validation.md`, `docs/golden-e2e-testing.md`

## Problem

The active spec's fourth ticket is a spatial-planning golden, but the current ticket file was mis-scoped and duplicated the combat-to-non-combat domain-crossing scenario instead. That leaves the actual S09 spatial promise without a matching implementation ticket: a single-agent golden proving that default-budget AI planning can leave the 7-edge `VillageSquare` hub, travel three hops to `OrchardFarm`, and complete the distant food-acquisition chain.

Without that golden, current coverage proves the planner substrate and other multi-hop scenarios, but not the exact behavioral promise from the spec: needs-driven, belief-driven, default-budget reachability from the branchiest hub in the prototype topology.

## Assumption Reassessment (2026-03-21)

1. The current ticket content does not match the owning spec. `specs/S16-s09-golden-validation.md` defines `S16GOLDVAL-004` as "Spatial Awareness Enables Multi-Hop Plan", but `tickets/S16S09GOLVAL-004.md` was written as "Combat-to-Non-Combat Domain Crossing", which is the spec's ticket 3 scenario. This is a scope mismatch and must be corrected before implementation.
2. The "no existing coverage" assumption is false. Current coverage already includes:
   - focused planner coverage for the exact branchy hub route in `crates/worldwake-ai/src/search.rs::search_finds_restock_progress_barrier_from_branchy_market_hub`
   - a golden multi-hop travel scenario from `BanditCamp` in `crates/worldwake-ai/tests/golden_ai_decisions.rs::golden_multi_hop_travel_plan`
   - a larger supply-chain golden in `crates/worldwake-ai/tests/golden_supply_chain.rs::test_merchant_restock_with_traces`, which explicitly raises `PlanningBudget.max_node_expansions` to 1024 for a multi-agent return-trip through the `VillageSquare` hub
3. The missing gap is specifically golden/E2E coverage, not focused/unit coverage. The planner layer already proves the search substrate can find a branchy-hub route; what is missing is a single-agent golden that proves the live AI decision/runtime stack does so from `VillageSquare` at the default `PlanningBudget::default()` boundary.
4. This is an AI golden E2E ticket and requires full action registries plus decision tracing. The contract spans candidate generation / ranking (`DecisionOutcome::Planning`), plan selection (`selection.selected_plan`), and authoritative movement / harvest / hunger relief. A local needs-only harness is insufficient.
5. Ordering contract: this is primarily authoritative world-state ordering plus plan-selection proof, not strict tick separation. The semantic promises are:
   - initial selected plan is travel-led toward the remote food source
   - the agent leaves `VillageSquare`
   - the agent reaches `OrchardFarm`
   - the agent harvests there
   - hunger decreases afterward
   The test should not overfit to exact tick numbers beyond a bounded observation window.
6. Not removing or weakening any heuristic/filter. The A* heuristic and travel pruning already exist; this ticket only adds coverage that those current planner substrates remain effective through the full AI runtime path.
7. Not a stale-request, contested-affordance, political, or `ControlSource` ticket. No request-resolution boundary claims are needed.
8. Scenario isolation: remove all local edible commodities from `VillageSquare` and adjacent one-hop places, place the apple source at `OrchardFarm`, and seed broad world beliefs so the agent lawfully knows the remote source exists. This intentionally removes nearer food branches that would otherwise make the test about local competition among lawful alternatives rather than about the branchy-hub travel reachability promise.
9. `golden_death_while_traveling` is relevant but insufficient. It starts from `BanditCamp`, not `VillageSquare`, so it does not exercise the seven outgoing edges documented in `crates/worldwake-core/src/topology.rs`.
10. Corrected scope: this ticket should add a VillageSquare-based spatial golden in the AI decision suite, not a combat-domain golden in `golden_combat.rs`.

## Architecture Check

1. The cleaner architecture is to place this test alongside existing needs/travel AI goldens in `crates/worldwake-ai/tests/golden_ai_decisions.rs`, not in `golden_combat.rs`. The behavior under test is planner-guided acquisition travel, not combat.
2. Keeping this as a tests-only ticket remains correct after reassessment. The code already has focused planner coverage for the branchy hub and live multi-hop travel goldens for other origins. The contradiction is in coverage shape, not in production architecture.
3. No backwards-compatibility aliasing or shims. The fix is to align the ticket to the actual spec and add the missing test directly on current symbols and current behavior.

## Verification Layers

1. Initial remote-food plan is selected from the live AI pipeline -> decision trace (`DecisionOutcome::Planning`, `selection.selected`, `selection.selected_plan`, `selection.selected_plan_source`)
2. Selected plan begins with travel and targets the first step away from `VillageSquare` -> decision trace (`selection.selected_plan.next_step` / travel step summaries)
3. Agent leaves `VillageSquare` and later reaches `OrchardFarm` -> authoritative world state (`effective_place`, `is_in_transit`)
4. Agent performs the distant acquisition rather than satisfying hunger locally -> action trace and/or authoritative world state (`harvest` action lifecycle or orchard apple output appearing there)
5. Hunger relief occurs after the remote acquisition chain -> authoritative world state (`HomeostaticNeeds.hunger`)
6. Deterministic replay of the full scenario -> `hash_world` + `hash_event_log`
7. Delayed authoritative outcomes are not being used as a proxy for initial plan selection. The test must prove both the early planning boundary and the later durable movement/resource outcomes because the ticket is about the full E2E promise.

## What to Change

### 1. Add a VillageSquare multi-hop golden in `golden_ai_decisions.rs`

Create a scenario with:

- `GoldenHarness::new(Seed([53; 32]))`
- a single hungry AI agent at `VILLAGE_SQUARE`
- no edible commodities at `VILLAGE_SQUARE` or adjacent one-hop places
- an `OrchardRow` resource source at `ORCHARD_FARM`
- world-belief seeding so the agent lawfully knows the remote resource exists
- decision tracing enabled, plus action tracing if needed for harvest proof

The scenario should prove that default-budget planning selects a travel-led plan toward the remote source and completes the remote acquire->eat chain.

### 2. Add a deterministic replay companion

Add the standard two-run `(hash_world, hash_event_log)` replay companion for the same scenario.

### 3. Reuse existing multi-hop helpers where that keeps the edit small

Prefer extending the existing `golden_ai_decisions.rs` multi-hop travel helper pattern rather than introducing a new test module or large new harness abstraction.

## Files to Touch

- `crates/worldwake-ai/tests/golden_ai_decisions.rs` (modify)
- `tickets/S16S09GOLVAL-004.md` (modify)

## Out of Scope

- Production planner or runtime changes unless reassessment during TDD exposes a real architectural contradiction
- Rewriting unrelated multi-hop or combat goldens
- Moving existing combat-domain coverage into new tickets during this change
- Raising the default planning budget
- Asserting exact per-tick arrival timings when the contract is semantic reachability

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-ai golden_spatial_multi_hop_plan`
2. `cargo test -p worldwake-ai golden_spatial_multi_hop_plan_replays_deterministically`
3. `cargo test -p worldwake-ai`

### Invariants

1. The selected initial plan is a lawful travel-led plan toward the remote food source under current belief/planner rules.
2. The test proves default-budget reachability from `VillageSquare`; it does not silently rely on an expanded planning budget.
3. Append-only event-log and conservation invariants continue to hold.
4. Deterministic replay with the same seed produces identical world and event-log hashes.

## Test Plan

### New/Modified Tests

1. `golden_spatial_multi_hop_plan` in `crates/worldwake-ai/tests/golden_ai_decisions.rs` — proves the live AI pipeline can leave the 7-edge `VillageSquare` hub under the default planning budget, travel to `OrchardFarm`, harvest remote food, and reduce hunger
2. `golden_spatial_multi_hop_plan_replays_deterministically` in `crates/worldwake-ai/tests/golden_ai_decisions.rs` — proves the same spatial-planning scenario replays deterministically

### Commands

1. `cargo test -p worldwake-ai golden_spatial_multi_hop_plan`
2. `cargo test -p worldwake-ai golden_spatial_multi_hop_plan_replays_deterministically`
3. `cargo test -p worldwake-ai`

## Outcome

- Completion date: 2026-03-21
- What actually changed:
  - corrected the ticket from an incorrectly duplicated combat-domain scope to the spec-aligned spatial multi-hop scope
  - added `golden_spatial_multi_hop_plan`
  - added `golden_spatial_multi_hop_plan_replays_deterministically`
  - reused the existing multi-hop helper path in `crates/worldwake-ai/tests/golden_ai_decisions.rs` instead of adding a new combat test or new test module
- Deviations from original plan:
  - the original ticket was not implemented as written because its scope was wrong; it duplicated the spec's ticket 3 domain-crossing work instead of ticket 4 spatial planning
  - the new golden was placed in `golden_ai_decisions.rs`, not `golden_combat.rs`, because the behavior under test is planner/runtime travel selection rather than combat
- Verification results:
  - `cargo test -p worldwake-ai golden_spatial_multi_hop_plan` passed
  - `cargo test -p worldwake-ai golden_multi_hop_travel_plan` passed after helper generalization
  - `cargo test -p worldwake-ai` passed
  - `cargo clippy --workspace` passed
