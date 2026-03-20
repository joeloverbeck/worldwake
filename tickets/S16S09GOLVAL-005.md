# S16S09GOLVAL-005: Golden — Spatial Awareness Enables Multi-Hop Plan at Hub Node

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: None
**Deps**: None (does not need helpers from S16S09GOLVAL-001)

## Problem

No existing golden test proves that the A* heuristic and travel pruning from S09 enable an agent at VillageSquare (7 outgoing edges) to find a multi-hop plan to a remote resource within the default planning budget. The existing `golden_death_while_traveling` test starts from BanditCamp (1 hop from food), bypassing the hub branching problem. Without this coverage, a regression in the spatial heuristic could silently break plan reachability at high-degree nodes — the plan literally cannot be found without spatial awareness because uniform-cost search exhausts the 512-node budget exploring all 7 directions equally.

## Assumption Reassessment (2026-03-20)

1. `golden_death_while_traveling` exists in `crates/worldwake-ai/tests/golden_combat.rs`. It exercises BanditCamp -> OrchardFarm travel (1 hop), which does NOT stress the VillageSquare hub branching. Verified by reading the test setup.
2. VillageSquare has 7 outgoing edges: GeneralStore, CommonHouse, RulersHall, GuardPost, PublicLatrine, SouthGate, NorthCrossroads. Verified in `crates/worldwake-core/src/topology.rs` (the `build_prototype_world` function).
3. OrchardFarm is 3 hops from VillageSquare: VS -> SouthGate -> EastFieldTrail -> OrchardFarm (7 travel ticks total). Verified in topology.
4. `ORCHARD_FARM` constant is available in `golden_harness/mod.rs:56`. `seed_actor_world_beliefs` is available for granting omniscient beliefs. `place_workstation_with_source` is available for placing resource sources.
5. `PerceptionProfile` is required on agents that need to observe post-production output (per CLAUDE.md golden test note). Available in harness imports.
6. The default planning budget is 512 nodes (verified in `crates/worldwake-ai/src/budget.rs`).
7. This is a golden E2E test. Verification layers: decision trace for plan selection (travel steps toward OrchardFarm), authoritative world state for location + hunger decrease, action trace for harvest action.
8. Ordering contract: The agent must leave VillageSquare, arrive at OrchardFarm, harvest, then eat. This is action lifecycle ordering (sequential plan execution). The divergence from a failed scenario is not ordering-based but reachability-based — without the heuristic, no plan is found at all.
9. Not removing/weakening any heuristic.
10. Not a stale-request, political, or ControlSource ticket.
11. **Scenario isolation**: No food at VillageSquare or adjacent 1-hop places — this forces the 3-hop plan. The only food source is at OrchardFarm. The agent has no other critical needs (thirst, fatigue, etc. are sated), so hunger is the sole pressure driver. This isolates the test to the travel->harvest->eat chain.
12. No mismatches found.

## Architecture Check

1. This test belongs in `golden_combat.rs` following the existing travel pattern from `golden_death_while_traveling`, OR in a new `golden_spatial.rs` file. The spec suggests `golden_combat.rs` since the travel pattern already exists there. However, this scenario is not combat-related — it's purely about spatial planning. A new `golden_spatial.rs` would be cleaner for discoverability. **Recommendation**: place in `golden_combat.rs` alongside the existing travel golden for consistency, unless the reviewer prefers a new file.
2. No backwards-compatibility shims.

## Verification Layers

1. Agent leaves VillageSquare within ~10 ticks -> authoritative world state (`effective_place(agent) != VILLAGE_SQUARE`)
2. Agent reaches OrchardFarm -> authoritative world state (`effective_place(agent) == ORCHARD_FARM`)
3. Agent performs harvest at OrchardFarm -> action trace (`ActionTraceKind::Committed` for "harvest") or active action name == "harvest"
4. Agent's hunger decreases -> authoritative world state (hunger delta)
5. Decision trace at early tick shows plan with travel steps toward OrchardFarm -> decision trace (`Planning` outcome, inspect `selected_plan` for travel steps)
6. Deterministic replay -> world hash + event log hash
7. Single-layer for plan reachability: decision trace proves plan was found (not `BudgetExhausted`)

## What to Change

### 1. Add `golden_spatial_multi_hop_plan` test to `golden_combat.rs`

Setup:
- `GoldenHarness::new(Seed([53; 32]))`
- Single agent (HungryTraveler) at VillageSquare:
  - Critical hunger pm(850). All other needs sated (thirst pm(0), fatigue pm(0), bladder pm(0), dirtiness pm(0)).
  - Default `MetabolismProfile`. Default `UtilityProfile` (hunger_weight dominates at critical level).
  - `PerceptionProfile` (to observe entities at destination after arrival).
  - Known recipes: harvest apples (`RecipeId(0)` via `build_recipes()`).
- NO food at VillageSquare or any 1-hop adjacent place.
- Place `OrchardRow` workstation with `ResourceSource { commodity: Apple, available_quantity: Quantity(10), .. }` at OrchardFarm via `place_workstation_with_source`.
- Seed world beliefs via `seed_actor_world_beliefs` so agent knows about the remote resource.
- Enable decision tracing.

Observation loop (up to 100 ticks):
- Track whether agent leaves VillageSquare.
- Track whether agent reaches OrchardFarm.
- Track whether agent harvests.
- Track whether hunger decreases.

Assertions:
1. Agent leaves VillageSquare within first ~10 ticks.
2. Agent reaches OrchardFarm.
3. Agent performs a harvest action at OrchardFarm.
4. Agent's hunger eventually decreases.
5. Decision trace at early tick shows `Planning` outcome (not `BudgetExhausted` or `FrontierExhausted`).
6. Deterministic replay.

### 2. Add `golden_spatial_multi_hop_plan_replays_deterministically` companion

Standard two-run hash comparison.

## Files to Touch

- `crates/worldwake-ai/tests/golden_combat.rs` (modify — add two tests)

## Out of Scope

- Any engine/production code changes
- Modifying existing tests
- Changes to the golden harness module
- Creating a new `golden_spatial.rs` file (unless reviewer requests)
- Testing performance/timing of the A* search (only reachability)
- Testing 1-hop travel (already covered by `golden_death_while_traveling`)
- Testing combat scenarios (this is purely spatial planning)

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-ai golden_spatial_multi_hop_plan` — new test passes
2. `cargo test -p worldwake-ai golden_spatial_multi_hop_plan_replays_deterministically` — replay passes
3. `cargo test -p worldwake-ai` — full suite, no regressions

### Invariants

1. Append-only event log is never mutated
2. Conservation invariants hold (apple lots created by harvest, consumed by eat)
3. Determinism: identical seed produces identical hashes
4. The A* heuristic does not change plan correctness — it only makes the plan reachable within budget
5. `effective_place` uniquely locates the agent at all times (unique location invariant)

## Test Plan

### New/Modified Tests

1. `golden_spatial_multi_hop_plan` in `crates/worldwake-ai/tests/golden_combat.rs` — proves S09 spatial awareness enables 3-hop plan at 7-edge hub node
2. `golden_spatial_multi_hop_plan_replays_deterministically` in `crates/worldwake-ai/tests/golden_combat.rs` — deterministic replay companion

### Commands

1. `cargo test -p worldwake-ai golden_spatial_multi_hop_plan`
2. `cargo test -p worldwake-ai`
3. `scripts/verify.sh`
