# Golden E2E Suite: Coverage Analysis and Gap Report

**Date**: 2026-03-12 (updated 2026-03-12)
**Scope**: `crates/worldwake-ai/tests/golden_*.rs` (split across domain files, shared harness in `golden_harness/mod.rs`)
**Purpose**: Document proven emergent scenarios, identify coverage gaps, and prioritize missing tests.

---

## File Layout

```
crates/worldwake-ai/tests/
  golden_harness/
    mod.rs                    — GoldenHarness, helpers, recipe builders, world setup
  golden_ai_decisions.rs      — 4 tests (scenarios 1, 2, 5, 7)
  golden_production.rs        — 4 tests (scenarios 3, 4, 6b, 6c)
  golden_combat.rs            — 2 tests (scenario 8 + replay)
  golden_determinism.rs       — 1 test  (scenario 6)
```

---

## Part 1: Proven Emergent Scenarios

The golden suite contains 11 tests across 4 domain files. Every test uses the real AI loop (`AgentTickDriver` + `AutonomousControllerRuntime`) and real system dispatch — no manual action queueing. All behavior is emergent.

### Scenario 1: Goal Invalidation by Another Agent
**File**: `golden_ai_decisions.rs` | **Test**: `golden_goal_invalidation_by_another_agent`
**Systems exercised**: Needs, Production (resource source), Travel, AI (candidate generation, planning)
**Setup**: Two critically hungry agents (Alice, Bob) at Village Square. Alice has 1 bread. Orchard Farm has apples.
**Emergent behavior proven**:
- Alice eats the bread (ConsumeOwnedCommodity goal).
- Bob, finding no local food, travels to Orchard Farm to harvest apples (AcquireCommodity goal with travel sub-plan).
- Conservation: bread lots never increase.
**Cross-system chain**: Needs pressure → goal generation → plan search → action execution → resource consumption.

### Scenario 2: Priority-Based Interrupt
**File**: `golden_ai_decisions.rs` | **Test**: `golden_priority_based_interrupt`
**Systems exercised**: Needs (metabolism), AI (goal switching, interrupt evaluation)
**Setup**: Single agent (Cara) with high fatigue (pm(800)), low hunger (pm(300)), but extremely fast hunger metabolism (pm(50)/tick). Has 2 bread.
**Emergent behavior proven**:
- Agent starts sleeping (fatigue is the highest-priority need initially).
- Metabolism drives hunger past critical threshold (pm(900)) during sleep.
- AI interrupts sleep action and switches to eating bread.
**Cross-system chain**: Metabolism tick → need escalation → interrupt evaluation → goal switch → action termination → new action start.

### Scenario 3: Resource Contention with Conservation
**File**: `golden_production.rs` | **Test**: `golden_resource_contention_with_conservation`
**Systems exercised**: Needs, Production, Travel, Conservation verification
**Setup**: Two critically hungry agents at Village Square. Alice has 1 bread. Orchard Farm has apples.
**Emergent behavior proven**:
- Both agents act concurrently under the same tick loop.
- Authoritative commodity totals (apple, bread) never increase — only decrease via consumption.
- Alice eats her bread. Event log grows (non-trivial simulation).
**Invariant enforced**: Per-tick authoritative conservation for both apple and bread commodities.

### Scenario 4: Materialization Barrier Chain
**File**: `golden_production.rs` | **Test**: `golden_materialization_barrier_chain`
**Systems exercised**: Production (harvest), Transport (pick-up), Needs (eat), AI (multi-step replanning)
**Setup**: Agent (Dana) at Orchard Farm, critically hungry, no food. OrchardRow workstation with 20 apples in ResourceSource.
**Emergent behavior proven**:
- Agent plans and executes harvest action → apple lots materialize on the ground at workstation.
- Agent replans to pick up the materialized apples (transport action).
- Agent replans again to eat the acquired apples.
- Conservation: total apple authoritative quantity never exceeds initial 20.
**Cross-system chain**: Harvest (production output on ground) → replan → pick-up (transport) → replan → eat (needs). This is the longest emergent action chain in the suite.

### Scenario 5: Blocked Intent Memory with TTL Expiry
**File**: `golden_ai_decisions.rs` | **Test**: `golden_blocked_intent_memory_with_ttl_expiry`
**Systems exercised**: Production (resource regeneration), AI (blocked intent memory, TTL expiry)
**Setup**: Agent (Eve) at Orchard Farm, critically hungry. ResourceSource is depleted (available_quantity=0) but regenerates at 1 unit per 5 ticks.
**Emergent behavior proven**:
- With depleted source, agent cannot harvest immediately.
- Resource regeneration system restores apples over time (5 ticks/unit → 10 ticks to reach Quantity(2)).
- After regeneration, agent successfully harvests.
- BlockedIntentMemory may be recorded if a plan fails (observational, not required).
**Cross-system chain**: Depleted resource → failed/deferred plan → resource regeneration ticks → successful harvest.

### Scenario 6: Deterministic Replay Fidelity
**File**: `golden_determinism.rs` | **Test**: `golden_deterministic_replay_fidelity`
**Systems exercised**: All (determinism across entire stack)
**Setup**: Two agents (Alice, Bob), both hungry, at Village Square. Alice has bread. Orchard Farm has apples. Run for 50 ticks with same seed twice.
**Emergent behavior proven**:
- Identical seeds produce identical `StateHash` for both world and event log.
- World state differs from initial (non-trivial simulation occurred).
**Invariant enforced**: Full-stack determinism (ChaCha8Rng, BTreeMap ordering, no floats, no wall-clock).

### Scenario 6b: Multi-Recipe Craft Path
**File**: `golden_production.rs` | **Test**: `golden_multi_recipe_craft_path`
**Systems exercised**: Production (craft with inputs), Transport, Needs, AI (recipe selection)
**Setup**: Agent (Miller) at Village Square with 1 firewood. Knows 3 recipes (harvest apples, harvest grain, bake bread). Mill workstation at Village Square.
**Emergent behavior proven**:
- Agent selects the bake bread recipe (requires firewood input, produces bread).
- Crafting consumes 1 firewood, produces 1 bread (conservation verified both ways).
- Agent consumes the crafted bread, reducing hunger.
- Deterministic replay: two runs with same seed produce identical hashes.
**Cross-system chain**: Recipe selection → craft action (input consumption + output materialization) → replan → eat.

### Scenario 6c: Capacity-Constrained Ground-Lot Pickup
**File**: `golden_production.rs` | **Test**: `golden_capacity_constrained_ground_lot_pickup`
**Systems exercised**: Production (harvest), Transport (partial pick-up split), Needs, AI (post-barrier replanning)
**Setup**: Agent (Porter) at Orchard Farm, critically hungry, carry capacity only 1 load unit. OrchardRow workstation has 10 apples; harvest outputs 2-apple ground lots.
**Emergent behavior proven**:
- Agent harvests apples, materializing a 2-apple ground lot.
- Because only 1 apple fits, the follow-up `pick_up` executes against the authoritative ground lot and materializes a split-off carried lot.
- Agent consumes one carried apple; one apple remains on the ground.
- Conservation checkpoints hold: 10 authoritative apples after harvest, then 9 after one apple is consumed.
- Deterministic replay: two runs with the same seed produce identical hashes.
**Cross-system chain**: Harvest materialization → replan → constrained pick-up split → consume.

### Scenario 7: Deprivation Cascade
**File**: `golden_ai_decisions.rs` | **Test**: `golden_deprivation_cascade`
**Systems exercised**: Needs (metabolism-driven escalation), AI (threshold-crossing detection)
**Setup**: Agent (Felix) starts with pm(0) hunger, fast metabolism (pm(20)/tick), has 1 bread.
**Emergent behavior proven**:
- Metabolism pushes hunger from 0 upward over time.
- When hunger crosses low threshold (pm(250)), AI generates a consume goal.
- Agent eats the bread.
**Cross-system chain**: Metabolism system → state change → AI threshold detection → goal generation → plan → action.

### Scenario 8: Death Cascade and Opportunistic Loot
**File**: `golden_combat.rs` | **Test**: `golden_death_cascade_and_opportunistic_loot`
**Companion replay test**: `golden_death_cascade_and_opportunistic_loot_replays_deterministically`
**Systems exercised**: Needs (deprivation wounds), Combat (wound accumulation, death), Loot, Conservation, deterministic replay
**Setup**: Fragile victim (wound_capacity pm(200), existing pm(150) starvation wound, fast hunger metabolism, 2 hunger-critical-exposure ticks) with 5 coins. Second agent (Looter) at same location, healthy.
**Emergent behavior proven**:
- Victim dies from deprivation wounds exceeding wound_capacity.
- Looter opportunistically loots the corpse within the 100-tick observation window (hard assertion).
- Coin lot conservation holds every tick throughout the death + loot sequence.
- Two runs with the same seed produce identical world and event-log hashes for the death-and-loot scenario.
**Cross-system chain**: Metabolism → deprivation exposure → wound infliction → wound accumulation → death → corpse creation → loot goal generation → loot action.

---

## Part 2: Coverage Matrix

### GoalKind Coverage

| GoalKind | Tested? | Scenarios |
|----------|---------|-----------|
| ConsumeOwnedCommodity | Yes | 1, 2, 3, 4, 5, 6b, 7 |
| AcquireCommodity (SelfConsume) | Yes | 1, 4, 5 |
| AcquireCommodity (Restock) | **No** | — |
| AcquireCommodity (RecipeInput) | **No** | — |
| AcquireCommodity (Treatment) | **No** | — |
| Sleep | Yes | 2 |
| Relieve | **No** | — |
| Wash | **No** | — |
| ReduceDanger | **No** | — |
| Heal | **No** | — |
| ProduceCommodity | Yes | 6b |
| SellCommodity | **No** | — |
| RestockCommodity | **No** | — |
| MoveCargo | **No** | — |
| LootCorpse | Yes | 8 |
| BuryCorpse | **No** | — |

**Coverage: 6/16 GoalKinds tested (37.5%), 1 partially tested.**

### ActionDomain Coverage

| Domain | Tested? | How |
|--------|---------|-----|
| Generic | Implicit | — |
| Needs (eat, drink, sleep, relieve, wash) | Partial | eat + sleep only |
| Production (harvest, craft) | Yes | 4, 5, 6b |
| Trade | **No** | — |
| Travel | Yes | 1, 3 (implicit) |
| Transport (pick-up, put-down) | Partial | pick-up only (4, 6c) |
| Combat (attack, defend) | **No** | — |
| Care (heal) | **No** | — |
| Loot | Yes | 8 |

**Coverage: 4/9 domains fully tested, 2 partially, 3 completely untested.**

### Needs Coverage

| Need | Tested as driver? | Tested as interrupt? |
|------|-------------------|---------------------|
| Hunger | Yes (all scenarios) | Yes (2) |
| Thirst | **No** | **No** |
| Fatigue | Yes (2, initial) | **No** |
| Bladder | **No** | **No** |
| Dirtiness | **No** | **No** |

### Topology Coverage

| Place | Used? | Scenarios |
|-------|-------|-----------|
| VillageSquare | Yes | All |
| OrchardFarm | Yes | 1, 3, 4, 5 |
| GeneralStore | **No** | — |
| CommonHouse | **No** | — |
| RulersHall | **No** | — |
| GuardPost | **No** | — |
| PublicLatrine | **No** | — |
| NorthCrossroads | **No** | — |
| ForestPath | **No** | — |
| BanditCamp | **No** | — |
| SouthGate | **No** | — |
| EastFieldTrail | **No** | — |

**Only 2/12 places used. No multi-hop travel tested. Shortest path VillageSquare→OrchardFarm is 7 ticks (VillageSquare→SouthGate(2)→EastFieldTrail(3)→OrchardFarm(2)), never explicitly verified.**

### Cross-System Interaction Coverage

| Interaction | Tested? |
|-------------|---------|
| Needs → AI goal generation | Yes |
| Metabolism → need escalation → eating | Yes |
| Production → materialization → transport → consumption | Yes |
| Resource depletion → regeneration → re-harvest | Yes |
| Deprivation → wounds → death | Yes |
| Death → loot | Yes |
| Trade negotiation between two agents | **No** |
| Combat between two living agents | **No** |
| Healing a wounded agent with medicine | **No** |
| Merchant restock → travel → acquire → return → sell | **No** |
| Goal switching mid-travel | **No** |
| Carry capacity exhaustion forcing inventory decisions | Yes |
| Multiple competing needs (hunger + thirst + fatigue) | **No** |
| Wound bleed → clotting → natural recovery | **No** |

---

## Part 3: Missing Scenarios — Prioritized Backlog

Each scenario is rated on three axes:
- **Emergence complexity** (1-5): How many cross-system interactions are chained.
- **Bug-catching value** (1-5): Likelihood of catching real bugs or regressions.
- **Implementation effort** (1-5): 1=trivial, 5=requires significant new harness/setup.

Sorted by composite score (emergence + bug-catching - effort) descending.

**Target files for new tests**: AI decision tests → `golden_ai_decisions.rs`, production/economy/transport → `golden_production.rs`, combat/death/loot → `golden_combat.rs`, determinism/replay → `golden_determinism.rs`. New domains (trade, care) may warrant new `golden_trade.rs` or `golden_care.rs` files.

### Tier 1: High Priority (score >= 6)

#### P1. Thirst-Driven Acquisition (Drink Water)
**Score**: Emergence=2, Bug-catching=4, Effort=1 → **Composite: 5**
**Rationale**: Thirst is the only other hunger-like need with consume actions, but it's completely untested. The `relieves_thirst` predicate and Water commodity consumption path have zero golden coverage. Trivial to implement — clone Scenario 7 with thirst instead of hunger.
**Proves**: Thirst threshold crossing → AI generates consume goal for Water → agent drinks.

#### P2. Two-Agent Trade Negotiation
**Score**: Emergence=4, Bug-catching=5, Effort=3 → **Composite: 6**
**Rationale**: The Trade domain (trade_actions, trade_valuation, MerchandiseProfile, DemandMemory) has zero golden coverage. This is an entire system crate module untested at the E2E level. Requires setting up two agents with MerchandiseProfile, TradeDispositionProfile, and complementary inventory.
**Proves**: Merchant A has surplus apples, merchant B has surplus bread → trade negotiation → goods exchange → conservation holds.

#### P3. Multi-Hop Travel Plan
**Score**: Emergence=3, Bug-catching=4, Effort=2 → **Composite: 5**
**Rationale**: All current travel is single-edge (VillageSquare ↔ OrchardFarm via shortest path). The planner's multi-step travel capability (Dijkstra pathfinding → sequential travel actions) is untested. Place agent at BanditCamp with food at OrchardFarm — requires traversing ForestPath → NorthCrossroads → EastFieldTrail → OrchardFarm (4 edges, 14 ticks).
**Proves**: GOAP search finds multi-hop travel plan → agent traverses 4 edges sequentially → arrives and harvests.

#### P4. Healing a Wounded Agent with Medicine
**Score**: Emergence=3, Bug-catching=5, Effort=2 → **Composite: 6**
**Rationale**: The Heal goal, Care action domain, and medicine consumption path are completely untested. The candidate_generation logic for `emit_heal_goals` requires medicine in inventory + wounded targets at same location.
**Proves**: Agent with medicine + wounded co-located agent → Heal goal generated → heal action executed → wound severity reduced.

#### P5. Bladder Relief with Travel to Latrine
**Score**: Emergence=3, Bug-catching=4, Effort=2 → **Composite: 5**
**Rationale**: The Relieve goal and PublicLatrine place are untested. Agent must recognize bladder pressure, travel to latrine, and relieve. Tests the bladder need pathway end-to-end.
**Proves**: Bladder crosses threshold → Relieve goal → travel to PublicLatrine → relieve action → bladder decreases.

#### P6. Goal Switching Mid-Travel
**Score**: Emergence=4, Bug-catching=5, Effort=2 → **Composite: 7**
**Rationale**: No test verifies that an agent can abandon a travel action mid-journey when a higher-priority need emerges. Agent traveling to distant food source has hunger spike to critical — should interrupt travel for available local alternative.
**Proves**: Agent starts traveling → hunger escalates during travel → interrupt evaluation fires → agent abandons travel → replans for local food.

#### P7. Combat Between Two Living Agents (ReduceDanger)
**Score**: Emergence=4, Bug-catching=5, Effort=3 → **Composite: 6**
**Rationale**: The Combat domain (attack, defend) and ReduceDanger goal are completely untested at E2E. This tests the combat system's wound resolution, attack/guard skill interaction, and the AI's decision to fight or flee.
**Proves**: Two agents, one attacks the other → wound infliction → defender generates ReduceDanger goal → combat resolution → conservation of items.

### Tier 2: Medium Priority (score 3-5)

#### P8. Merchant Restock-Travel-Sell Loop
**Score**: Emergence=5, Bug-catching=4, Effort=4 → **Composite: 5**
**Rationale**: The full merchant enterprise loop (RestockCommodity → travel to source → acquire → MoveCargo back to market → SellCommodity) is the most complex emergent chain possible. Tests EnterpriseSignals, restock_gap, deliverable_quantity, and the sell action.
**Proves**: Merchant with empty stock → restock goal → travel to resource → harvest/acquire → MoveCargo to home market → sell to buyer.

#### P9. Carry Capacity Exhaustion
**Score**: Emergence=3, Bug-catching=4, Effort=2 → **Composite: 5**
**Rationale**: No test verifies behavior when an agent's CarryCapacity (LoadUnits(50)) is reached. Agent should be unable to pick up more items and must either drop items or choose lighter alternatives.
**Proves**: Agent acquires items until load limit → transport action rejected → agent replans around capacity constraint.

#### P10. Three-Way Need Competition
**Score**: Emergence=3, Bug-catching=3, Effort=2 → **Composite: 4**
**Rationale**: No test exercises an agent with multiple simultaneous critical needs. Tests that the ranking system correctly prioritizes among hunger, thirst, and fatigue when all cross thresholds simultaneously.
**Proves**: Agent with critical hunger + critical thirst + critical fatigue → ranking selects highest-utility need → agent addresses it first → then re-ranks.

#### P11. Wash Action (Dirtiness + Water)
**Score**: Emergence=2, Bug-catching=3, Effort=2 → **Composite: 3**
**Rationale**: Wash goal requires dirtiness above threshold AND Water in inventory. This pathway and the dirtiness need are untested.
**Proves**: Dirtiness crosses threshold → agent with water → Wash goal → wash action → dirtiness decreases + water consumed.

#### P12. Death While Traveling
**Score**: Emergence=4, Bug-catching=4, Effort=3 → **Composite: 5**
**Rationale**: What happens when an agent dies from deprivation during a multi-tick travel action? The action should terminate, death should be processed, and the corpse should remain at the departure location (or mid-travel, depending on implementation).
**Proves**: Fragile agent starts traveling → deprivation wound accumulates during travel → death during travel action → action terminates → corpse location is consistent.

#### P13. Resource Exhaustion Race (Many Agents, Finite Resources)
**Score**: Emergence=3, Bug-catching=4, Effort=3 → **Composite: 4**
**Rationale**: Current tests use at most 2 agents. What happens when 4+ hungry agents compete for a limited resource source? Tests reservation/contention logic and graceful degradation when resources run out.
**Proves**: 4 agents, resource source with Quantity(4) → agents race to harvest → some succeed, some must seek alternatives → conservation holds → no deadlocks.

### Tier 3: Lower Priority (score <= 2)

#### P15. Put-Down Action (Inventory Management)
**Score**: Emergence=2, Bug-catching=2, Effort=2 → **Composite: 2**
**Rationale**: Only pick-up is tested (Scenario 4). Put-down (dropping items) is untested. Lower priority since it's simpler and less likely to have cross-system bugs.

#### P16. BuryCorpse Goal
**Score**: Emergence=2, Bug-catching=2, Effort=3 → **Composite: 1**
**Rationale**: BuryCorpse requires a corpse + burial site. This is a complete feature that's untested, but it's also a simpler action with fewer cross-system interactions.

#### P17. Seed Sensitivity (Different Seeds, Different Outcomes)
**Score**: Emergence=1, Bug-catching=3, Effort=1 → **Composite: 3**
**Rationale**: Scenario 6 proves same-seed determinism. A complementary test proving that different seeds produce different outcomes would strengthen confidence in the RNG integration.

#### P18. Save/Load Round-Trip Under AI
**Score**: Emergence=2, Bug-catching=3, Effort=3 → **Composite: 2**
**Rationale**: Save/load is tested in worldwake-sim but not with the full AI loop. Saving mid-simulation, loading, and continuing should produce consistent outcomes.

---

## Part 4: Summary Statistics

| Metric | Current | With Tier 1 | With All |
|--------|---------|-------------|----------|
| GoalKind coverage | 6/16 (37.5%) | 13/16 (81%) | 16/16 (100%) |
| ActionDomain coverage | 3/9 full | 7/9 full | 9/9 full |
| Needs tested | 2/5 | 4/5 | 5/5 |
| Places used | 2/12 | 5/12 | 5/12+ |
| Cross-system chains | 6 | 13 | 18 |

### Recommended Implementation Order (Tier 1)

1. **P1 (Thirst)** — trivial, fills a basic coverage gap
2. **P4 (Heal)** — new domain, moderate setup
3. **P5 (Bladder/Latrine)** — new need + new place
4. **P3 (Multi-Hop Travel)** — tests planner depth
5. **P6 (Goal Switch Mid-Travel)** — tests interrupt during travel
6. **P7 (Combat)** — complex new domain
7. **P2 (Trade)** — most complex setup (merchant profiles)
