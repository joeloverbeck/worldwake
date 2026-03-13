# Golden E2E Suite: Coverage Analysis and Gap Report

**Date**: 2026-03-12 (updated 2026-03-13)
**Scope**: `crates/worldwake-ai/tests/golden_*.rs` (split across domain files, shared harness in `golden_harness/mod.rs`)
**Purpose**: Document proven emergent scenarios, identify coverage gaps, and prioritize missing tests.

---

## File Layout

```
crates/worldwake-ai/tests/
  golden_harness/
    mod.rs                    — GoldenHarness, helpers, recipe builders, world setup
  golden_ai_decisions.rs      — 10 tests (scenarios 1, 2, 3b, 3c, 5, 7, 7a, 7b, 7d, 7e)
  golden_care.rs              — 2 tests (scenario 2c + replay)
  golden_production.rs        — 9 tests (scenarios 3, 3d, 4, 6b, 6c, 9, 9b + replays)
  golden_combat.rs            — 7 tests (living combat + defensive mitigation + death scenarios + replays)
  golden_determinism.rs       — 1 test  (scenario 6)
  golden_trade.rs             — 4 tests (scenarios 2b, 2d + replays)
```

---

## Part 1: Proven Emergent Scenarios

The golden suite contains 33 tests across 6 domain files. Every test uses the real AI loop (`AgentTickDriver` + `AutonomousControllerRuntime`) and real system dispatch — no manual action queueing. All behavior is emergent.

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

### Scenario 3d: Resource Exhaustion Race
**File**: `golden_production.rs` | **Tests**: `golden_resource_exhaustion_race`, `golden_resource_exhaustion_race_replays_deterministically`
**Systems exercised**: Needs, Production, AI planning/runtime, reservation handling, Conservation, deterministic replay
**Setup**: Four critically hungry agents start co-located at Orchard Farm with no food. One OrchardRow workstation exposes a finite `ResourceSource` with exactly `Quantity(4)` apples and no regeneration.
**Emergent behavior proven**:
- Multiple hungry agents can queue against the same finite source through the real AI loop without crashing the tick when same-tick reservation contention occurs.
- The orchard source is observed stepping `4 -> 2 -> 0`, proving exactly two harvest commits exhaust the stock.
- Apple lots materialize from the source and at least one agent completes the downstream harvest/pick-up/eat chain.
- Authoritative apple quantity never exceeds the initial 4, and the same-seed run replays to identical world and event-log hashes.
- The scenario exposed and fixed an engine gap: autonomous same-tick start requests were previously strict, so a second valid-from-snapshot harvest request could fail with `ReservationUnavailable` and abort the whole tick. The input path now distinguishes strict manual requests from best-effort autonomous requests, allowing the AI runtime to reconcile contention on the next tick without shims.
**Cross-system chain**: Shared hunger pressure at one place → multiple harvest plans from one snapshot → reservation-backed start contention → finite source depletion across two harvest commits → materialization/pick-up/eat → deterministic replay.

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
- Runtime establishes an active journey commitment to Orchard Farm, records travel progress after leg completion, and later exposes the same commitment as suspended during the local thirst detour.
- The penalty-interruptible travel action continues while thirst rises through the subcritical medium/high bands.
- The agent does not consume carried water before thirst reaches the critical threshold.
- Once thirst becomes critical, the running travel plan is interrupted and the agent consumes carried water at an intermediate concrete place on the route.
- After the detour, the runtime reactivates the original Orchard Farm commitment, unless the detour resolves at Orchard Farm itself.
- The journey is not treated as a rigid commitment to the original destination.
**Cross-system chain**: Hunger pressure → distant `AcquireCommodity` travel plan → journey commitment established → metabolism escalates thirst during journey → intermediate arrival triggers replanning and commitment suspension → `ConsumeOwnedCommodity { Water }` → commitment reactivation or arrival at destination.

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

### Scenario 7e: Wash Action
**File**: `golden_ai_decisions.rs` | **Test**: `golden_wash_action`
**Systems exercised**: Needs, AI (candidate generation, planning), Needs actions (`wash`), Conservation
**Setup**: Agent at Village Square with high dirtiness and 1 controlled Water. All other needs are low.
**Emergent behavior proven**:
- High dirtiness emits the `Wash` goal through the real AI loop.
- Agent executes the real `wash` action without manual queueing.
- Dirtiness decreases and Water is consumed.
- Water lot totals never increase during the scenario.
**Cross-system chain**: Dirtiness pressure + local Water → `Wash` goal → wash action → dirtiness relief + Water consumption.

### Scenario 7b: Bladder Relief with Travel
**File**: `golden_ai_decisions.rs` | **Test**: `golden_bladder_relief_with_travel`
**Systems exercised**: Needs (bladder pressure), AI (candidate generation, planning), Travel, Needs actions (`toilet`)
**Setup**: Agent (Mira) starts at Village Square with elevated bladder pressure. Public Latrine is reachable in the prototype topology and `toilet` is only available at latrine-tagged places.
**Emergent behavior proven**:
- Bladder pressure emits the `Relieve` goal.
- Agent leaves Village Square and reaches Public Latrine instead of relieving locally.
- Relief completes at the latrine, reducing bladder pressure and materializing waste there without taking the bladder-accident path.
**Cross-system chain**: Bladder pressure → `Relieve` goal → travel to latrine-tagged place → `toilet` action → bladder relief + waste materialization.

### Scenario 7d: Three-Way Need Competition
**File**: `golden_ai_decisions.rs` | **Test**: `golden_three_way_need_competition`
**Systems exercised**: Needs, AI (candidate generation, ranking, replanning), Needs actions (`eat`, `drink`, `sleep`)
**Setup**: Single agent at Village Square with critical hunger, thirst, and fatigue. The agent carries 2 bread and 2 water, and its `UtilityProfile` weights hunger above thirst above fatigue.
**Emergent behavior proven**:
- The first started self-care action is `eat`, confirming that the runtime opens on the highest-weight hunger path under simultaneous local pressure.
- The agent also consumes water in the same local scenario rather than stalling on a single need.
- Fatigue is eventually reduced after thirst has been handled, proving the runtime keeps re-ranking instead of deadlocking.
- The scenario exposed a planner bug: hypothetical consume transitions were commodity-blind, so a `drink` step could satisfy a bread-consume goal in planning. Tightening consume transitions to require a commodity match fixed that without adding shims.
**Cross-system chain**: Simultaneous hunger + thirst + fatigue pressure → candidate generation → ranking picks hunger path first → interruptible local consume actions update needs → replanning handles thirst → later rest reduces fatigue.

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

### Scenario 7f: ReduceDanger Defensive Mitigation Under Active Threat
**File**: `golden_combat.rs` | **Test**: `golden_reduce_danger_defensive_mitigation`
**Systems exercised**: AI (danger pressure, candidate generation, planning), Combat (`attack`, `defend`), authoritative belief/runtime integration, Conservation
**Setup**: Two sated agents at Village Square. The attacker has a concrete hostility relation toward the defender and a stronger combat profile. The defender has no outgoing hostility relation and is therefore purely reactive.
**Emergent behavior proven**:
- The attacker opens combat through the normal `EngageHostile` path.
- The defender observes live attack pressure through the real runtime-aware belief view instead of relying on a test shim or queue injection.
- The defender does not alias incoming hostility into proactive `EngageHostile`; instead it enters a real `ReduceDanger` mitigation path and manifests concrete defensive behavior (`defend`/defending stance).
- Coin totals remain exactly conserved throughout the encounter.
**Cross-system chain**: Outgoing hostility → attack action start → current-attacker danger signal → `ReduceDanger` emission → defensive mitigation action.

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

### Scenario 8b: Death While Traveling
**File**: `golden_combat.rs` | **Test**: `golden_death_while_traveling`
**Companion replay test**: `golden_death_while_traveling_replays_deterministically`
**Systems exercised**: Needs (metabolism, deprivation exposure), AI (distant acquire planning), Travel, Combat (fatal wound resolution), Conservation, deterministic replay
**Setup**: Fragile traveler starts at Bandit Camp with critical hunger and 5 coins. Orchard Farm is the only food source. The route requires real multi-hop travel through Forest Path and beyond.
**Emergent behavior proven**:
- Traveler leaves Bandit Camp and enters real in-transit travel on the hunger-driven route.
- The traveler dies from deprivation before reaching Orchard Farm.
- In the deterministic scenario, death resolves at the first intermediate route place (`ForestPath`), proving the body remains at a concrete grounded location instead of vanishing or reaching the destination.
- After death resolves, the agent has no active action and no lingering in-transit state.
- Coin lot conservation holds every tick throughout the journey and death sequence.
- Two runs with the same seed produce identical world and event-log hashes for the death-while-traveling scenario.
**Cross-system chain**: Hunger pressure → distant acquire-goal emission → travel departure → continued metabolism on route → deprivation wound infliction → death before destination → concrete body placement on route.

### Scenario 9: Exclusive Facility Queue Contention
**File**: `golden_production.rs` | **Tests**: `golden_exclusive_queue_contention_uses_queue_grants_and_rotates_first_turns`, `golden_exclusive_queue_contention_replays_deterministically`
**Systems exercised**: Production (exclusive facility policy, resource source), AI (candidate generation, planning with queue barriers), FacilityQueue (queue_for_facility_use action, facility_queue_system tick), Conservation, deterministic replay
**Setup**: Four critically hungry agents (Aster, Bram, Cara, Dara) co-located at Orchard Farm. A single OrchardRow workstation with `ExclusiveFacilityPolicy` and `FacilityUseQueue`, containing `Quantity(4)` apples and no regeneration. Grant expiry window is 3 ticks.
**Emergent behavior proven**:
- Multiple agents generate harvest goals simultaneously, but the exclusive facility policy forces them through `queue_for_facility_use` before harvesting.
- A real waiting line materializes on the workstation (`max_waiting_len >= 2`), proving queue contention occurs under the AI loop.
- The `facility_queue_system()` tick system (prune → expire → promote) grants access to one agent at a time. At least two distinct agents receive grants and complete harvest turns.
- The first two promoted actors are different agents, proving fair queue rotation rather than one agent monopolizing the facility.
- The exclusive orchard source is fully exhausted (`Quantity(0)`) after the two granted harvest turns.
- Authoritative apple conservation holds every tick throughout the contention sequence.
- Two runs with the same seed produce identical world and event-log hashes.
**Cross-system chain**: Multi-agent hunger pressure at exclusive facility → queue_for_facility_use action → facility_queue_system promotion → granted harvest → resource depletion → queue rotation to next agent → deterministic replay + conservation.

### Scenario 9b: Facility Queue Patience Timeout
**File**: `golden_production.rs` | **Test**: `golden_facility_queue_patience_timeout`
**Systems exercised**: Production (exclusive facility policy, resource source), AI runtime (patience expiry, blocked-facility memory, replanning), FacilityQueue, Travel, Conservation
**Setup**: Hungry agent starts at Orchard Farm with a per-agent `FacilityQueueDispositionProfile`. Facility A is a local exclusive orchard already monopolized by another actor's long-lived grant. Facility B is a reachable alternative exclusive orchard at Village Square.
**Emergent behavior proven**:
- The hungry agent initially queues at the local facility A through the real `queue_for_facility_use` path.
- When the queue does not progress within the configured patience window, the runtime removes the agent from facility A's authoritative queue instead of merely marking a dirty replan hint.
- The queue disappearance flows through the existing blocked-facility pipeline, recording `ExclusiveFacilityUnavailable` for facility A rather than introducing a special-case alias path.
- The agent then replans to the alternative facility B, uses it, and reduces hunger there.
- The monopolized facility A remains unused by the waiting agent; the alternative facility B is the one whose stock decreases.
**Cross-system chain**: Local hunger pressure → queue_for_facility_use at local exclusive facility → patience-based authoritative dequeue → blocked-facility planning memory → alternative-facility travel/queue/use → harvest/materialization → hunger relief.

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
| Wash | Yes | 7e |
| EngageHostile | Yes | 7c |
| ReduceDanger | Yes | 7f |
| Heal | Yes | 2c |
| ProduceCommodity | Yes | 6b |
| SellCommodity | **No** | — |
| RestockCommodity | Yes | 2d |
| MoveCargo | Yes | 2d |
| LootCorpse | Yes | 8 |
| BuryCorpse | **No** | — |

**Coverage: 13/17 GoalKinds tested (76.5%).**

### ActionDomain Coverage

| Domain | Tested? | How |
|--------|---------|-----|
| Generic | Implicit | — |
| Needs (eat, drink, sleep, relieve, wash) | Yes | eat + drink + sleep + relieve + wash |
| Production (harvest, craft) | Yes | 4, 5, 6b |
| FacilityQueue (queue_for_facility_use) | Yes | 9 |
| Trade | Yes | 2b |
| Travel | Yes | 1, 3 (implicit) |
| Transport (pick-up, put-down) | Partial | pick-up only (4, 6c) |
| Combat (attack, defend) | Yes | 7c, 7f |
| Care (heal) | Yes | 2c |
| Loot | Yes | 8 |

**Coverage: 9/10 domains fully tested, 1 partial.**

### Needs Coverage

| Need | Tested as driver? | Tested as interrupt? |
|------|-------------------|---------------------|
| Hunger | Yes (all scenarios) | Yes (2) |
| Thirst | Yes (7a, 3c) | Yes (3c) |
| Fatigue | Yes (2, initial) | **No** |
| Bladder | Yes (7b) | **No** |
| Dirtiness | Yes (7e) | **No** |

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
| Multi-agent reservation-backed resource exhaustion | Yes |
| Multiple competing needs (hunger + thirst + fatigue) | Yes |
| Penalty-interrupt threshold (subcritical continue, critical interrupt) | Yes |
| Dirtiness pressure → wash → water consumption | Yes |
| Active attack pressure → ReduceDanger → defensive mitigation | Yes |
| Death after departure on multi-hop travel | Yes |
| Multi-agent exclusive facility queue rotation → grant promotion → harvest | Yes |
| Queue patience timeout → authoritative dequeue → alternative facility recovery | Yes |
| Wound bleed → clotting → natural recovery | **No** |

---

## Part 3: Missing Scenarios — Prioritized Backlog

Each scenario is rated on three axes:
- **Emergence complexity** (1-5): How many cross-system interactions are chained.
- **Bug-catching value** (1-5): Likelihood of catching real bugs or regressions.
- **Implementation effort** (1-5): 1=trivial, 5=requires significant new harness/setup.

Sorted by composite score (emergence + bug-catching - effort) descending.

**Target files for new tests**: AI decision tests → `golden_ai_decisions.rs`, production/economy/transport → `golden_production.rs`, combat/death/loot → `golden_combat.rs`, determinism/replay → `golden_determinism.rs`. New domains (trade, care) may warrant new `golden_trade.rs` or `golden_care.rs` files.

### Tier 1: High Priority (score >= 5)

No remaining Tier 1 backlog items. The prior journey-commitment proof gap is now covered directly in Scenario 3c.

### Tier 2: Medium Priority (score 3-4)

`P-NEW-3 Goal-Switch Margin Boundary` was removed from the golden backlog on 2026-03-13. Reassessment showed the exact boundary is already covered by focused tests in `goal_switching.rs`, `interrupts.rs`, `plan_selection.rs`, and `journey_switch_policy.rs`, while existing golden scenarios already cover behavior-level switching. A new golden arithmetic-threshold scenario would duplicate lower-layer guarantees without adding durable cross-system coverage.

#### P-NEW-5. Grant Expiry Before Intended Action
**Score**: Emergence=3, Bug-catching=4, Effort=3 → **Composite: 4**
**Rationale**: Scenario 9 shows grants being used promptly. This gap tests what happens when a grant expires before the planner schedules the intended action (e.g., because of goal-switching delay). Agent must re-queue or replan.
**Proves**: `GrantedFacilityUse::expires_at`, `expire_stale_grant()`, re-queue after expiry.
**File**: `golden_production.rs`

#### P-NEW-6. Materialization Binding Failure
**Score**: Emergence=3, Bug-catching=4, Effort=3 → **Composite: 4**
**Rationale**: Scenario 4 proves the happy-path materialization barrier chain. This gap tests what happens when the chain breaks: agent plans craft → consume, but between craft completion and consume step, another agent picks up the crafted item. Materialization binding can't resolve → plan failure → blocked intent.
**Proves**: `MaterializationBindings` resolution failure, `handle_plan_failure()`, blocked intent creation.
**File**: `golden_production.rs`

#### P-NEW-8. Blocked Facility Use Avoidance in Planner
**Score**: Emergence=2, Bug-catching=4, Effort=2 → **Composite: 4**
**Rationale**: After a failed queue+execute cycle at facility A, the planner should avoid re-queueing at facility A (via `blocked_facility_uses` in `PlanningState`). If alternative facility B exists, agent routes there.
**Proves**: `candidate_uses_blocked_facility_use()` filter, `blocked_facility_uses: BTreeSet<(EntityId, ActionDefId)>`.
**File**: `golden_production.rs`

#### P-NEW-7. AcquireCommodity(RecipeInput) Goal
**Score**: Emergence=3, Bug-catching=3, Effort=3 → **Composite: 3**
**Rationale**: `AcquireCommodity { purpose: RecipeInput }` is the only untested AcquireCommodity variant besides Treatment. Agent wants to bake bread but has no firewood; must acquire firewood before crafting.
**Proves**: `AcquireCommodity { purpose: RecipeInput }` goal generation, multi-step plan: travel → acquire input → craft → consume.
**File**: `golden_production.rs`

#### P17. Seed Sensitivity (Different Seeds, Different Outcomes)
**Score**: Emergence=1, Bug-catching=3, Effort=1 → **Composite: 3**
**Rationale**: Scenario 6 proves same-seed determinism. A complementary test proving that different seeds produce different outcomes would strengthen confidence in the RNG integration.
**File**: `golden_determinism.rs`

### Tier 3: Lower Priority (score <= 2)

#### P15. Put-Down Action (Inventory Management)
**Score**: Emergence=2, Bug-catching=2, Effort=2 → **Composite: 2**
**Rationale**: Only pick-up is tested (Scenario 4). Put-down (dropping items) is untested. Lower priority since it's simpler and less likely to have cross-system bugs.

#### P-NEW-9. Dead Agent Pruned from Facility Queue
**Score**: Emergence=2, Bug-catching=3, Effort=3 → **Composite: 2**
**Rationale**: Agent dies while waiting in facility queue. `prune_invalid_waiters()` removes them. Next agent in line gets promoted.
**Proves**: `prune_invalid_waiters()`, queue position advancement after death.
**File**: `golden_combat.rs`

#### P18. Save/Load Round-Trip Under AI
**Score**: Emergence=2, Bug-catching=3, Effort=3 → **Composite: 2**
**Rationale**: Save/load is tested in worldwake-sim but not with the full AI loop. Saving mid-simulation, loading, and continuing should produce consistent outcomes.
**File**: `golden_determinism.rs`

#### P16. BuryCorpse Goal
**Score**: Emergence=2, Bug-catching=2, Effort=3 → **Composite: 1**
**Rationale**: BuryCorpse requires a corpse + burial site. This is a complete feature that's untested, but it's also a simpler action with fewer cross-system interactions.
**File**: `golden_combat.rs`

---

## Part 4: Summary Statistics

| Metric | Current | With Tier 1 | With All |
|--------|---------|-------------|----------|
| Proven tests | 33 | 33 | 39 |
| GoalKind coverage | 13/17 (76.5%) | 13/17 (76.5%) | 14/17 (82.4%) |
| ActionDomain coverage | 9/10 full | 9/10 full | 10/10 full |
| Needs tested | 5/5 | 5/5 | 5/5 |
| Places used | 9/12 | 9/12+ | 9/12+ |
| Cross-system chains | 23 | 23 | 25 |

### Recommended Implementation Order (Tier 1)

No Tier 1 backlog remains.
