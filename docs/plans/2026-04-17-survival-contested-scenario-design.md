# Survival-Contested Scenario Design

## Brainstorm Context

**Original request**: Decide whether a third survival-baseline scenario is warranted before moving on to gameplay-feature scenarios.

**Key findings from codebase evidence**:
- Existing scenarios (`survival-baseline.ron`, `survival-scattered.ron`) prove 3-agent survival over 1440 ticks under (a) co-located resources and (b) scattered resources with travel metabolism.
- Scattered relaxed `MAX_CRITICAL_RUN_TICKS` 100 → 400 and `IDLE_THRESHOLD` 20 → 50, and excluded `Wash` from budget-exhaustion checks (`GOAPTRVLSCAL-001`).
- Neither scenario exercises: resource contention among agents, dynamic depletion forcing belief invalidation, or tolerance tighter than scattered's wide allowances.
- S107's own archive notes that in scattered, agents converge to a 2-location corridor and `Orchard Hollow` is never visited — diversification is not pressure-tested.

**Decision**: Create one third scenario before starting gameplay-feature scenarios, to close the contention/scarcity gap in survival-baseline proof.

**Scope refinement (user feedback)**: The scenario must use the **same profile set** as the existing two survival scenarios. Including `DiversificationProfile` (S107) or any other opt-in feature profile would turn this into a feature scenario and defeat the "prove baseline first" intent. If contention cannot be survived without such a profile, that finding itself is the architectural result — it is not masked by opting into the feature.

**Final confidence**: 95%. No remaining gaps at design level.

## Overview

A new scenario `scenarios/survival-contested.ron` and golden test `crates/worldwake-ai/tests/golden_survival_contested.rs` prove that 4 AI agents can survive 1440 ticks under tight resource contention, dynamic depletion, and chokepoint topology — using only the profile set already exercised by `survival-baseline` and `survival-scattered`.

## Scenario Design

### Topology (7 places, 4 agents)

```
          North Camp ──── Forest Glade ──── South Camp
              │                                 │
              │             (latrine)           │
              │                                 │
          Stone Well ──── Central Crossing ──── Spring Basin
                              │   │
                              │   │
                        East Orchard  West Grainfield
```

| Place | Tags | Purpose |
|---|---|---|
| North Camp | Camp, Latrine | Start for Agents A, B |
| South Camp | Camp, Latrine | Start for Agents C, D |
| Forest Glade | Forest | Alternate relief site, no resources |
| Central Crossing | Crossroads, Road | Hub chokepoint |
| Stone Well | Crossroads | Water source only (no wash) |
| Spring Basin | Crossroads | Water + WashBasin (contested multi-use) |
| East Orchard | Farm, Field | Apple source |
| West Grainfield | Farm, Field | Grain source |

**Edges** (bidirectional, ticks):
- North Camp ↔ Stone Well (3)
- North Camp ↔ Forest Glade (2)
- South Camp ↔ Spring Basin (3)
- South Camp ↔ Forest Glade (2)
- Stone Well ↔ Central Crossing (2)
- Spring Basin ↔ Central Crossing (2)
- Central Crossing ↔ East Orchard (3)
- Central Crossing ↔ West Grainfield (3)
- East Orchard ↔ West Grainfield (4)
- Forest Glade ↔ Central Crossing (3)

**Key topology properties**:
- Each starting camp has one nearby water source (3 ticks). To reach the *other* water source, an agent must cross the Central Crossing chokepoint.
- Wash is only at Spring Basin — North-side agents must travel through Central Crossing to wash.
- Food (Apple, Grain) is reachable only through Central Crossing from either starting camp.
- Forest Glade is a latrine-only secondary place that alleviates bladder load without travel through the hub.

### Agents (4, same-profile AI)

Four agents with profile parity to `survival-scattered.ron` except for:
- Travel metabolism multipliers kept identical to scattered (150/100/50, wilderness penalty 200) — contention, not travel, is the new stressor.
- Starting needs aligned so that all four reach moderate thirst pressure within a ~150-tick window, forcing overlap at water sources.

| Agent | Start | Hunger | Thirst | Fatigue | Bladder | Dirtiness |
|---|---|---|---|---|---|---|
| A | North Camp | 380 | 360 | 120 | 150 | 130 |
| B | North Camp | 420 | 340 | 140 | 170 | 150 |
| C | South Camp | 400 | 370 | 110 | 140 | 120 |
| D | South Camp | 440 | 350 | 130 | 160 | 140 |

Same `known_recipes` as scattered: `["Harvest Apples", "Harvest Water", "Harvest Grain"]`. Same drive thresholds, cognitive profile, execution budget, exploration profile, perception profile, utility profile structure as scattered (with per-agent trait variation on curiosity/metabolism rates, same order as scattered's A/B/C).

**No new profiles**: no `DiversificationProfile`, no `ObligationSatiationProfile`, no `ArtifactPostingProfile`, no `DisposalProfile`.

### Facilities and resource sources

| Facility | Workstation | Location |
|---|---|---|
| Stone Well Pump | Well | Stone Well |
| Spring Well | Well | Spring Basin |
| Spring Washbasin | WashBasin | Spring Basin |
| Valley Orchard | OrchardRow | East Orchard |
| Grainfield Plot | FieldPlot | West Grainfield |

**Resource sources** (tighter than scattered):

| Commodity | Location | Facility | Regen ticks/unit | Capacity |
|---|---|---|---|---|
| Water | Stone Well | Stone Well Pump | 8 | 4 |
| Water | Spring Basin | Spring Well | 10 | 4 |
| Apple | East Orchard | Valley Orchard | 10 | 6 |
| Grain | West Grainfield | Grainfield Plot | 12 | 5 |

**Scarcity math** (rough):
- 4 agents × ~3 pm/tick thirst × 1440 ticks ≈ 17 water units consumed total.
- Aggregate water regen across both wells over 1440 ticks: ~320 units. Aggregate supply far exceeds demand; contention is *instantaneous* not cumulative.
- Capacity 4 per well means at most 4 concurrent draws before depletion — with 4 agents, any single well can saturate.

## Key Decisions

1. **4 agents, not 5** — balances real contention against test runtime and planner scaling. Scenarios 1-2 used 3; going to 4 is the minimum bump that creates genuine rivalry at capacity-4 wells.

2. **Two water sources, not one** — one would force either a fixed queue or starvation; two creates meaningful choice under pressure. Agents that initially know neither source must learn both via S102 frontier-aware exploration.

3. **Co-located wash + water at Spring Basin** — doubles contention at one node (agents wanting water *and* agents wanting wash compete for the same place). Stone Well offers a no-wash alternative.

4. **Forest Glade is resource-less** — it exists as an alternate latrine and a detour option, mirroring the `Woodland Clearing` role in scattered. Keeps at least one non-resource place in the graph for exploration proofs.

5. **No planner-budget increases from scattered** — same 640 expansions, beam_width 12, etc. If contention blows budget, that is an engine finding not a tuning move.

6. **Wash remains excluded from budget-exhaustion assertions**, with the same `GOAPTRVLSCAL-001` caveat as scattered. This scenario is not the right place to force that fix; a dedicated ticket/spec (existing) should own it.

## Test Assertions

All assertions are adaptations of the scattered golden test with tighter tolerances. The golden test structure matches `golden_survival_scattered.rs` closely (shared `NeedRunTracker`, same idle-window machinery).

| Assertion | Target | Vs scattered |
|---|---|---|
| All agents alive at tick 1440 | Same | Same |
| `MAX_CRITICAL_RUN_TICKS` per need | **≤ 250** | 400 (tighter) |
| `IDLE_THRESHOLD` | **40** | 50 (tighter) |
| `NEEDS_LOW_CEILING` | 300 | Same |
| Each agent commits eat, drink, sleep, relieve, wash | Same | Same |
| No budget exhaustion on non-Wash survival goals | Same | Same (Wash excluded via same caveat) |
| No stuck idle window ≥ 40 ticks with needs > 300 pm | Same | Tighter threshold |

**Contention-specific assertion** (new):
- Across the whole run, **both** water sources are drawn from by **at least one agent** (i.e., population-level does not fully monopolize a single well). Proves belief invalidation drives adaptation when a preferred source is contested.

**Contention-specific assertion** (new, optional — add only if straightforward):
- The isolated agents from each camp side (one from North, one from South) both reach **at least one food source** (East Orchard OR West Grainfield). Proves that both sides of the topology exercise the chokepoint.

**Tolerance-flex clause**: If 250 / 40 are infeasible on first run, the plan allows relaxation in 25/5 increments up to 325 / 50. Any looser than that signals a real behavioral gap and should trigger investigation rather than further relaxation — document in the golden test comments exactly as scattered documented its relaxations.

## FOUNDATIONS.md Alignment

| Principle | How the scenario respects it |
|---|---|
| FND-01 Maximal Emergence | Contention emerges from capacity + regen + concurrent need cycles, not from scripted rivalry. |
| FND-07 Information Locality | Agents plan from beliefs; no scenario seeds knowledge of opposite-side resources — discovery is required. |
| FND-10 Belief-Only Planning | Tested directly: when a target source is depleted by another agent, the planner must react to dirty beliefs. |
| FND-12 System Decoupling | Scenario references only `core`/`sim`/`systems`/`cli` surfaces already used by existing survival scenarios. No new component types. |
| FND-22 Need Dampening | Tighter `MAX_CRITICAL_RUN_TICKS=250` proves dampening remains bounded under contention. |
| FND-31 Agent Symmetry | All four agents use the same AI driver, same profile schema, parameterized per-agent (mirrors scattered). |

## Files to Create

1. `scenarios/survival-contested.ron` — scenario definition (~450 lines, same structure as scattered).
2. `crates/worldwake-ai/tests/golden_survival_contested.rs` — golden test (~500 lines, adapted from `golden_survival_scattered.rs`).
3. Regenerate `docs/generated/golden-scenario-index.md` and `docs/generated/golden-scenario-details/survival-contested.md` via `python3 scripts/golden_inventory.py --write --check-docs`.

## Files to Update

- `specs/IMPLEMENTATION-ORDER.md` — add a short note under "Adjunct Wave: Proactive Diversification" (or a sibling wave) that the third survival scenario landed and that Phase 7 gameplay-feature specs inherit a three-scenario survival floor.

## Verification Plan

Run in this order, narrowest first:

```bash
# 1. Scenario parses
cargo test -p worldwake-cli scenario::

# 2. New golden suite passes standalone
cargo test -p worldwake-ai golden_survival_contested

# 3. Existing survival goldens still green (no regression)
cargo test -p worldwake-ai golden_survival_baseline golden_survival_scattered

# 4. Full AI crate
cargo test -p worldwake-ai

# 5. Workspace-wide
cargo test --workspace

# 6. Clippy parity with CI
cargo clippy --workspace --all-targets -- -D warnings

# 7. Observer sanity run (optional but recommended)
cargo run -p worldwake-cli --bin observer -- scenarios/survival-contested.ron --ticks 1440
```

If tolerances fail on first run, use the scenario-analysis skill to produce a behavioral smell report before loosening assertions.

## Open Questions / Known Flex Items

1. **If 4 agents cannot survive without diversification**: that is an architectural finding. Options (document in a follow-up ticket, do not pre-solve here):
   - Tighten engine-level reactivity so baseline mechanisms cover contention without diversification.
   - Promote `DiversificationProfile` (or a slimmer proactive-replan primitive) to baseline-agent default and retrofit scenarios 1-2.
   - Accept that contention is out of scope for "survival baseline" and narrow the scenario's assertions.

2. **Wash-budget exhaustion closure**: this scenario deliberately inherits the scattered exclusion. Closing `GOAPTRVLSCAL-001` remains owned by its ticket and is unaffected by this scenario's acceptance.

3. **Tolerance precedent**: the 250 / 40 targets are set by the principle "tighter than scattered, aspirationally close to baseline". If empirical evidence after first runs justifies a different landing point, the golden test should document the numeric choice with the same care scattered's comments show.
