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
  golden_ai_decisions.rs      — 8 tests (scenarios 1, 2, 3b, 3c, 5, 7, 7a, 7b)
  golden_care.rs              — 2 tests (scenario 2c + replay)
  golden_production.rs        — 4 tests (scenarios 3, 4, 6b, 6c)
  golden_combat.rs            — 4 tests (living combat + scenario 8 + replay)
  golden_determinism.rs       — 1 test  (scenario 6)
  golden_trade.rs             — 4 tests (scenarios 2b, 2d + replays)
```

---

## Part 1: Proven Emergent Scenarios

The golden suite contains 23 tests across 6 domain files. Every test uses the real AI loop (`AgentTickDriver` + `AutonomousControllerRuntime`) and real system dispatch — no manual action queueing. All behavior is emergent.

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

### Scenario 2b: Buyer-Driven Trade Acquisition
**File**: `golden_trade.rs` | **Tests**: `golden_buyer_driven_trade_acquisition`, `golden_buyer_driven_trade_acquisition_replays_deterministically`
**Systems exercised**: Needs, AI (candidate generation, planning), Trade, Conservation, deterministic replay
**Setup**: Hungry buyer and sated seller co-located at Village Square. Seller advertises bread via `MerchandiseProfile`; buyer holds coins and a `TradeDispositionProfile`.
**Emergent behavior proven**:
- Buyer generates `AcquireCommodity { commodity: Bread, purpose: SelfConsume }` from hunger pressure.
- Planner resolves the acquire goal through a local trade barrier rather than unrelated travel branches.
- Trade executes through the real action handler: bread transfers to the buyer and coins transfer to the seller.
- Buyer then consumes the acquired bread, reducing hunger.
- Bread lots never increase and coin lots remain exactly conserved.
- Two runs with the same seed produce identical world and event-log hashes for the trade scenario.
**Cross-system chain**: Need pressure → seller discovery via `MerchandiseProfile` → planner trade barrier selection → trade valuation/exchange → consumption.

### Scenario 2c: Healing a Wounded Agent
**File**: `golden_care.rs` | **Tests**: `golden_healing_wounded_agent`, `golden_healing_wounded_agent_replays_deterministically`
**Systems exercised**: AI (candidate generation, planning), Care action domain, Combat/wound treatment, Conservation, deterministic replay
**Setup**: Healthy healer and wounded patient co-located at Village Square. Healer holds 1 medicine. Patient begins with a bleeding starvation wound.
**Emergent behavior proven**:
- Healer generates `Heal { target }` from the local wounded target plus medicine in inventory.
- Planner selects the care-domain heal action through the real action registry.
- Heal executes through the normal lifecycle: medicine is consumed and the patient's wound load decreases.
- Two runs with the same seed produce identical world and event-log hashes for the healing scenario.
**Cross-system chain**: Local wound state → heal-goal generation → planner care-step selection → medicine consumption → wound severity/bleed reduction.

### Scenario 2d: Merchant Restock and Return to Home Market
**File**: `golden_trade.rs` | **Tests**: `golden_merchant_restock_return_stock`, `golden_merchant_restock_return_stock_replays_deterministically`
**Systems exercised**: Enterprise AI signals, Travel, Production, Transport/cargo continuity, deterministic replay
**Setup**: Merchant starts at General Store with `MerchandiseProfile` advertising apples, zero apple stock, and remembered unmet apple demand at the home market. Orchard Farm has an apple resource source via OrchardRow workstation + `ResourceSource`.
**Emergent behavior proven**:
- Merchant generates the enterprise `RestockCommodity { Apple }` path from concrete remembered demand rather than from a magic stock threshold.
- Merchant leaves General Store, reaches Orchard Farm, and acquires apples through the real harvest path.
- Merchant controls apples away from the home market and later returns that stock to General Store, exercising `MoveCargo`.
- The scenario exposed a planner-budget gap: the default search node-expansion budget was too low for the branch-heavy restock route from Village Square. Raising the default node-expansion budget fixed the real runtime path without adding special cases.
- Two runs with the same seed produce identical world and event-log hashes for the merchant restock scenario.
**Cross-system chain**: Demand memory at home market → enterprise restock signal → multi-leg travel → harvest/materialization → cargo return to home market.

### Scenario 3: Resource Contention with Conservation
**File**: `golden_production.rs` | **Test**: `golden_resource_contention_with_conservation`
**Systems exercised**: Needs, Production, Travel, Conservation verification
**Setup**: Two critically hungry agents at Village Square. Alice has 1 bread. Orchard Farm has apples.
**Emergent behavior proven**:
- Both agents act concurrently under the same tick loop.
- Authoritative commodity totals (apple, bread) never increase — only decrease via consumption.
- Alice eats her bread. Event log grows (non-trivial simulation).
**Invariant enforced**: Per-tick authoritative conservation for both apple and bread commodities.

### Scenario 3b: Multi-Hop Travel Plan
**File**: `golden_ai_decisions.rs` | **Test**: `golden_multi_hop_travel_plan`
**Systems exercised**: Needs, AI (candidate generation, planning, replanning), Travel, Production
**Setup**: Critically hungry agent starts at Bandit Camp. Orchard Farm has apples via OrchardRow workstation + ResourceSource. The shortest route is `BanditCamp -> ForestPath -> NorthCrossroads -> EastFieldTrail -> OrchardFarm` (4 edges, 14 travel ticks).
**Emergent behavior proven**:
- Agent leaves Bandit Camp and traverses a real multi-edge route to the distant food source.
- AI replans cleanly after intermediate travel progress instead of reusing a stale pre-travel route prefix.
- Agent reaches Orchard Farm, harvests apples there, and reduces hunger.
**Cross-system chain**: Need pressure → distant acquire-goal emission → multi-hop plan search → sequential travel execution → harvest/materialization → downstream hunger relief.

### Scenario 3c: Goal Switching During Multi-Leg Travel
**File**: `golden_ai_decisions.rs` | **Test**: `golden_goal_switching_during_multi_leg_travel`
**Systems exercised**: Needs (metabolism), AI (candidate generation, ranking, replanning), Travel
**Setup**: Hungry agent starts at Bandit Camp with 1 carried water and no food. Orchard Farm remains the distant food source. Thirst starts low but escalates quickly enough to become critical only after the first travel leg completes.
**Emergent behavior proven**:
- Agent begins the distant hunger-driven journey and leaves Bandit Camp.
- Before reaching Orchard Farm, the agent reprioritizes to a different need and consumes carried water at an intermediate concrete place on the route.
- The journey is not treated as a rigid commitment to the original destination.
**Cross-system chain**: Hunger pressure → distant `AcquireCommodity` travel plan → metabolism escalates thirst during journey → intermediate arrival triggers replanning → `ConsumeOwnedCommodity { Water }`.

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

### Scenario 7a: Thirst-Driven Acquisition
**File**: `golden_ai_decisions.rs` | **Test**: `golden_thirst_driven_acquisition`
**Systems exercised**: Needs (metabolism-driven escalation), AI (threshold-crossing detection), Needs actions (drink)
**Setup**: Agent (Talia) starts with pm(0) thirst, fast thirst metabolism (pm(20)/tick), has 1 water.
**Emergent behavior proven**:
- Metabolism pushes thirst from 0 upward over time.
- When thirst crosses low threshold (pm(200)), AI generates a water consume goal.
- Agent drinks the water.
**Cross-system chain**: Metabolism system → thirst escalation → AI threshold detection → consume goal generation → drink action.

### Scenario 7b: Bladder Relief with Travel
**File**: `golden_ai_decisions.rs` | **Test**: `golden_bladder_relief_with_travel`
**Systems exercised**: Needs (bladder pressure), AI (candidate generation, planning), Travel, Needs actions (`toilet`)
**Setup**: Agent (Mira) starts at Village Square with elevated bladder pressure. Public Latrine is reachable in the prototype topology and `toilet` is only available at latrine-tagged places.
**Emergent behavior proven**:
- Bladder pressure emits the `Relieve` goal.
- Agent leaves Village Square and reaches Public Latrine instead of relieving locally.
- Relief completes at the latrine, reducing bladder pressure and materializing waste there without taking the bladder-accident path.
**Cross-system chain**: Bladder pressure → `Relieve` goal → travel to latrine-tagged place → `toilet` action → bladder relief + waste materialization.

### Scenario 7c: Hostility-Driven Living Combat
**File**: `golden_combat.rs` | **Tests**: `golden_combat_between_living_agents`, `golden_combat_between_living_agents_replays_deterministically`
**Systems exercised**: AI (hostility-driven candidate generation, planning), Combat (attack resolution, wounds), Conservation, deterministic replay
**Setup**: Two sated agents at Village Square with a concrete hostility relation. The attacker has a stronger combat profile; both carry coin so conservation can be checked across the fight.
**Emergent behavior proven**:
- The attacker generates a dedicated hostile-engagement goal from concrete local hostility and commits to the combat-domain `attack` action through the normal planner/runtime path.
- The defender responds through the live combat loop once the first attack is underway.
- At least one wound is inflicted on a living participant without either actor being pre-scripted via manual queueing.
- Coin totals remain exactly conserved throughout the encounter.
- Two runs with the same seed produce identical world and event-log hashes for the scenario.
**Cross-system chain**: Hostility relation → hostile-engagement goal → attack action start → wound infliction → defender response → deterministic replay + conservation checks.

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
| ConsumeOwnedCommodity | Yes | 1, 2, 3, 4, 5, 6b, 7, 7a |
| AcquireCommodity (SelfConsume) | Yes | 1, 2b, 4, 5 |
| AcquireCommodity (Restock) | Yes | 2d |
| AcquireCommodity (RecipeInput) | **No** | — |
| AcquireCommodity (Treatment) | **No** | — |
| Sleep | Yes | 2 |
| Relieve | Yes | 7b |
| Wash | **No** | — |
| EngageHostile | Yes | 7c |
| ReduceDanger | **No** | — |
| Heal | Yes | 2c |
| ProduceCommodity | Yes | 6b |
| SellCommodity | **No** | — |
| RestockCommodity | Yes | 2d |
| MoveCargo | Yes | 2d |
| LootCorpse | Yes | 8 |
| BuryCorpse | **No** | — |

**Coverage: 11/17 GoalKinds tested (64.7%).**

### ActionDomain Coverage

| Domain | Tested? | How |
|--------|---------|-----|
| Generic | Implicit | — |
| Needs (eat, drink, sleep, relieve, wash) | Partial | eat + drink + sleep |
| Production (harvest, craft) | Yes | 4, 5, 6b |
| Trade | Yes | 2b |
| Travel | Yes | 1, 3 (implicit) |
| Transport (pick-up, put-down) | Partial | pick-up only (4, 6c) |
| Combat (attack, defend) | Yes | 7c |
| Care (heal) | Yes | 2c |
| Loot | Yes | 8 |

**Coverage: 7/9 domains fully tested, 2 partially.**

### Needs Coverage

| Need | Tested as driver? | Tested as interrupt? |
|------|-------------------|---------------------|
| Hunger | Yes (all scenarios) | Yes (2) |
| Thirst | Yes (7a, 3c) | Yes (3c) |
| Fatigue | Yes (2, initial) | **No** |
| Bladder | Yes (7b) | **No** |
| Dirtiness | **No** | **No** |

### Topology Coverage

| Place | Used? | Scenarios |
|-------|-------|-----------|
| VillageSquare | Yes | All |
| OrchardFarm | Yes | 1, 2d, 3, 3b, 4, 5 |
| GeneralStore | Yes | 2d |
| CommonHouse | **No** | — |
| RulersHall | **No** | — |
| GuardPost | **No** | — |
| PublicLatrine | Yes | 7b |
| NorthCrossroads | Yes | 3b |
| ForestPath | Yes | 3b |
| BanditCamp | Yes | 3b |
| SouthGate | Yes | 2d |
| EastFieldTrail | Yes | 3b |

**9/12 places are now used. Multi-hop travel is explicitly tested via both the BanditCamp→OrchardFarm route and the GeneralStore→OrchardFarm merchant restock route.**

### Cross-System Interaction Coverage

| Interaction | Tested? |
|-------------|---------|
| Needs → AI goal generation | Yes |
| Metabolism → need escalation → eating | Yes |
| Metabolism → thirst escalation → drinking | Yes |
| Bladder pressure → travel → relief | Yes |
| Production → materialization → transport → consumption | Yes |
| Resource depletion → regeneration → re-harvest | Yes |
| Deprivation → wounds → death | Yes |
| Death → loot | Yes |
| Trade negotiation between two agents | Yes |
| Multi-hop travel to distant acquisition source | Yes |
| Combat between two living agents | Yes |
| Healing a wounded agent with medicine | Yes |
| Merchant restock → travel → acquire → return stock to home market | Yes |
| Goal switching during multi-leg travel | Yes |
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

#### P5. Bladder Relief with Travel to Latrine
**Score**: Emergence=3, Bug-catching=4, Effort=2 → **Composite: 5**
**Rationale**: The Relieve goal and PublicLatrine place are untested. Agent must recognize bladder pressure, travel to latrine, and relieve. Tests the bladder need pathway end-to-end.
**Proves**: Bladder crosses threshold → Relieve goal → travel to PublicLatrine → relieve action → bladder decreases.

#### P7. ReduceDanger Defensive Mitigation Under Active Threat
**Score**: Emergence=4, Bug-catching=4, Effort=3 → **Composite: 5**
**Rationale**: Living combat is now covered, but `ReduceDanger` itself is still not proven as a defensive end-to-end behavior. The architecture now keeps offensive hostility separate from defensive danger mitigation, so this remaining gap should test flight/defend behavior under active attack instead of reusing the living-combat scenario.
**Proves**: Agent comes under active attack pressure → `ReduceDanger` is emitted at high-or-above danger → planner selects a defensive mitigation (`defend`, reposition, or equivalent real path) → danger drops without manual action queueing.

### Tier 2: Medium Priority (score 3-5)

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
| GoalKind coverage | 11/17 (64.7%) | 12/17 (70.6%) | 13/17 (76.5%) |
| ActionDomain coverage | 7/9 full | 7/9 full | 9/9 full |
| Needs tested | 4/5 | 4/5 | 5/5 |
| Places used | 9/12 | 9/12+ | 9/12+ |
| Cross-system chains | 13 | 14 | 17 |

### Recommended Implementation Order (Tier 1)

1. **P7 (ReduceDanger defensive mitigation)** — still-open defensive combat gap
2. **P9 (Carry capacity exhaustion)** — transport/capacity decision gap with high regression value
3. **P10 (Three-way need competition)** — ranking/regression coverage without major harness expansion
