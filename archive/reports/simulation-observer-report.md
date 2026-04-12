# Simulation Observer Report

## Run Summary

- **Scenario**: `scenarios/cli-evaluation.ron`
- **Seed**: 7777
- **Ticks simulated**: 1440 (1 simulated day)
- **Total events**: 9966
- **Agents**: 4 (Kael, Merchant Vara, Forager Lina, Guard Theron)
- **Places**: 5 (Thornwall Village, Eldergrove Forest, Dusty Trail, Hearthstone Inn, Golden Fields)
- **Deaths**: Guard Theron at tick 422 (cause: NeedDeprivation { Hunger })

## Findings

### 1. Redundant Perception -- MEDIUM

**Agent(s)**: All four agents
**Evidence**: Kael observed Dusty Trail 33 times, self 111 times, Merchant Vara 103 times. Merchant Vara observed Kael 98 times, self 105 times. Guard Theron observed self 112 times, Thornwall Village 63 times. Forager Lina observed entity e27g0 34 times, self 20 times.
**Root cause hypothesis**: The perception system fires on a regular schedule (every ~7-10 ticks based on the perception timeline bins) and re-observes all nearby entities regardless of whether their state changed. This is by-design periodic perception, not a bug — agents need to re-scan their environment to notice state changes. However, the volume of self-observations (111, 105, 112 times) is notable and may represent wasted processing since self-state is directly accessible.
**Confidence**: MEDIUM — most of these are expected periodic perception. Self-observation at 100+ times may warrant investigation into whether self-perception provides useful information that couldn't be derived directly.

### 2. Action Loops -- HIGH

**Agent(s)**: Kael, Merchant Vara
**Evidence**:
- **Kael**: After tick 500, action repertoire collapsed from 4 types to 2 types (sleep + relieve_wilderness). From tick 500 to 1440 (940 ticks), Kael executed only sleep (x94) and relieve_wilderness (x10). The decision timeline shows the planner selecting only Sleep and Relieve goals because AcquireCommodity(Water) budget-exhausted repeatedly and no eat/drink affordances existed at Dusty Trail.
- **Merchant Vara**: Mechanical action loop detected — [sleep -> sleep] repeated 3 times. Additionally, after tick 900, Merchant Vara's repertoire collapsed to sleep + relieve_wilderness only, and further narrowed to sleep-only at tick 1400 when hunger/thirst/dirtiness all hit 1000 permille. The planner continuously attempted TreatWounds for Merchant Vara's own wounds but budget-exhausted every time (5739-6233 candidates at depth 3).
- Merchant Vara also had 5 failed staff_market StartFailed events, indicating the market-staffing action was attempted but couldn't start.
**Root cause hypothesis**: Behavioral collapse is driven by resource starvation — both agents are at Dusty Trail with no food or water sources. The planner cannot find plans for AcquireCommodity because the multi-location acquisition plan (travel to a place with resources, pick up, consume) generates too many candidates and exceeds the expansion budget. With only Sleep and Relieve available as satisfiable goals, agents enter a survival-minimal loop.
**Confidence**: HIGH — the action timelines clearly show the transition from diverse behavior to minimal loop, correlated exactly with rising needs and budget-exhausted plan failures.

### 3. Stuck Agents -- HIGH

**Agent(s)**: Guard Theron (1019 consecutive idle ticks), Kael (34 consecutive idle ticks), Merchant Vara (27 consecutive idle ticks)
**Evidence**:
- **Guard Theron**: 1019 consecutive idle ticks — but 1018 of these are post-death (died at tick 422, simulation runs to 1440). Pre-death, Theron was active: investigate (27 started), patrol (7 started), tell (9 committed, 21 StartFailed), travel (4), sleep (33), relieve_wilderness (4). His behavioral transitions show repertoire narrowing from 6 types to 2 types at tick 200, then to 1 type at tick 400 — just 22 ticks before death. His planner had 338 planning ticks, 85 active-action ticks, and 1 dead tick (the tick of death).
- **Kael**: 34 consecutive idle ticks is borderline — likely between action cycles during the sleep+relieve loop.
- **Merchant Vara**: 27 consecutive idle ticks, similar pattern.
**Root cause hypothesis**: Guard Theron's pre-death stuckness (ticks 200-422) follows the same pattern as Kael and Merchant Vara: he was at Dusty Trail with no eat/drink affordances, and AcquireCommodity(Water) repeatedly budget-exhausted (224 expansions, depth 5-7, 1589-2657 candidates). He had travel affordances (1 target) but the planner couldn't complete the multi-hop plan to reach water. The 1019 idle ticks are expected (dead agent).
**Confidence**: HIGH for Guard Theron's pre-death stuck pattern. LOW for Kael/Merchant Vara — their 27-34 idle tick stretches are between actions, not pathological.

### 4. Failed Action Spirals -- LOW

**Agent(s)**: Guard Theron, Merchant Vara
**Evidence**:
- Guard Theron had 21 tell StartFailed events (out of 30 attempted). These occurred primarily at Thornwall Village (ticks 0-100), suggesting the tell action's preconditions (perhaps requiring a specific listener or belief state) were not met.
- Merchant Vara had 5 staff_market StartFailed events and 3 tell StartFailed events.
**Root cause hypothesis**: The tell failures likely result from the target listener not being present or the agent not having a belief to share that passes the tell action's preconditions (build_successor returned None for ShareBelief goals). The staff_market failures may indicate missing preconditions for market operation (perhaps no market entity or no stock).
**Confidence**: MEDIUM — the tell failures are high-count (21/30 for Theron) but the agent was still functioning at the time. These represent wasted planning effort but didn't cause the agent's collapse.

### 5. Sustained Critical Needs -- CRITICAL

**Agent(s)**: Guard Theron, Kael, Merchant Vara, Forager Lina (dirtiness only)
**Evidence**:
- **Guard Theron**: Hunger above 750 permille for 1215 ticks (225-1439, but died at 422). Thirst above 750 permille for 1290 ticks (150-1439). Min hunger was 302, min thirst was 303 — needs were already critical at simulation start.
- **Kael**: Hunger above 750 permille for 674 ticks (766-1439). Thirst above 750 permille for 915 ticks (525-1439). Dirtiness above 750 for 790 ticks (650-1439).
- **Merchant Vara**: Hunger above 750 permille for 265 ticks (1175-1439). Thirst above 750 permille for 1257 ticks (183-1439, avg 926). Dirtiness above 750 for 790 ticks (650-1439).
- **Forager Lina**: Dirtiness above 750 for 810 ticks (630-1439). All other needs well-managed (hunger avg 44, thirst avg 104).

Cross-reference with Section 7:
- **Guard Theron**: AcquireCommodity(Water) budget-exhausted repeatedly (224 expansions, depth 5-7, 1589-2657 candidates at Thornwall Village; frontier-exhausted at Dusty Trail). Never ate or drank.
- **Kael**: AcquireCommodity(Water) budget-exhausted at Thornwall Village (224 expansions, depth 6, 2657 candidates), frontier-exhausted at Dusty Trail (8 expansions, depth 7, 18 candidates). Ate 5 times and drank 5 times in the first 400 ticks, then stopped.
- **Merchant Vara**: AcquireCommodity(Water) budget-exhausted from tick 11 onward (300 expansions, depth 9, 1483 candidates). AcquireCommodity(Apple) budget-exhausted (300 expansions, depth 4, 2080-2511 candidates). Never drank despite 1257 ticks above 750.
**Root cause hypothesis**: The AcquireCommodity plan for water requires a multi-step chain (travel to Well location, draw water, consume) that generates 1400-2600+ candidates and exceeds the planner's expansion budget. This is structural — the plan exists but the search space is too large by design. Agents at Dusty Trail have no local water source, and the travel-acquire-consume chain is too deep for the budget. Guard Theron started with high needs (302/303 hunger/thirst) and died before finding a viable plan.
**Confidence**: HIGH — the causal chain is clear: no local food/water at Dusty Trail -> budget-exhausted AcquireCommodity plans -> sustained critical needs -> death (Theron) or slow starvation (others).

### 6. Unaddressed Needs -- CRITICAL

**Agent(s)**: Merchant Vara (thirst), Guard Theron (hunger and thirst)
**Evidence**:
- **Merchant Vara**: Thirst avg 926 permille, never attempted drink. Her affordances at every location never included drink: tick 0 at Thornwall Village had no drink/eat, tick 60 at Dusty Trail had eat but no drink. Even Kael (who also ended at Dusty Trail) managed 5 drinks early on — Merchant Vara never had the affordance.
- **Guard Theron**: Hunger avg 915 permille, never attempted eat. Thirst avg 943 permille, never attempted drink. His affordances at Dusty Trail (tick 0) and Thornwall Village (tick 9) showed no eat/drink. His final affordances at tick 422 also had no eat/drink.

Cross-reference with Section 7:
- Merchant Vara's planner budget-exhausted on AcquireCommodity(Water) from tick 11 and AcquireCommodity(Apple) from tick 85. No blocked desires section present (none were fully blocked — the planner kept trying and failing).
- Guard Theron's planner similarly budget-exhausted on AcquireCommodity(Water) repeatedly. His goals were dominated by InvestigateViolation, Patrol, and ShareBelief — his role-specific duties crowded out survival planning.
**Root cause hypothesis**: Merchant Vara and Guard Theron spawned without food/water in inventory and at locations without those resources as affordances. Merchant Vara's tick-0 affordances at Thornwall Village didn't even include eat or drink despite there being a Well at Thornwall Village — this suggests either the Well requires a specific action (queue_for_facility_use?) rather than direct drink, or Merchant Vara lacks the preconditions to use it. The affordance gap is the root cause.
**Confidence**: HIGH — the missing eat/drink affordances directly explain the unaddressed needs.

### 7. Impossible Knowledge -- NONE

No evidence of agents acting on information they never perceived. All action targets correlate with entities in their perception traces. Kael and Merchant Vara's tell actions targeted co-located agents they had observed. Guard Theron's investigate actions targeted locations he knew about through perception.

### 8. Belief Staleness -- MEDIUM

**Agent(s)**: Kael, Merchant Vara, Guard Theron
**Evidence**:
- **Kael**: Believes Dusty Trail contains himself, Merchant Vara, and 13x Waste. Section 6 shows Dusty Trail actually contains Kael, Merchant Vara, Guard Theron, 1x Bow, 20x Coin, 1x Sword, 36x Waste. Kael doesn't know about Guard Theron's presence (despite Theron being there since tick 69) or the weapons/coins on the ground.
- **Merchant Vara**: Believes Dusty Trail contains Kael and 10x Waste. Doesn't know about Guard Theron or the 36 Waste actually present. Doesn't know about herself being at Dusty Trail in her own beliefs (knows only 1 agent: Kael).
- **Guard Theron**: Believes Dusty Trail contains Kael, Merchant Vara, Guard Theron, and 12x Waste. More accurate but still misses the 36 Waste and equipment.
- **Forager Lina**: Knows 0 agents. She's been at Eldergrove Forest for all 1440 ticks with no social contact. Doesn't know about any other agent in the simulation.
**Root cause hypothesis**: Belief staleness is expected in a locality-based perception system. Agents only update beliefs when perception fires and passes, and perception has a 525-950 permille pass rate that means not every scan succeeds. The Waste accumulation (from consumption byproducts) outpaces perception updates. Kael not knowing Theron is at Dusty Trail despite being co-located for 1000+ ticks is more concerning — this may indicate perception doesn't detect dead agents or that the perception check failed for Theron.
**Confidence**: MEDIUM — some staleness is expected by design, but the failure to perceive co-located agents after 1000+ ticks of co-location warrants investigation.

### 9. Social Isolation -- HIGH

**Agent(s)**: Forager Lina (complete isolation), Kael/Merchant Vara/Guard Theron (partial — social activity dropped to zero after tick 500)
**Evidence**:
- **Forager Lina**: 0 social observations, 0 told beliefs, 0 heard beliefs, 0 tell/ask_witness actions. Spent all 1440 ticks at Eldergrove Forest alone, never traveling. Knows 0 agents.
- **Kael**: Last tell action at tick ~400-499. After relocating to Dusty Trail with Merchant Vara (and later Guard Theron's corpse), Kael had tell affordances available (tell x1 target in final affordances) but the planner never selected ShareBelief goals after tick 500.
- **Merchant Vara**: Last tell action at tick ~200-299. Similar situation — tell affordances available but not selected.
- **Guard Theron**: Active social behavior before death (28 tell starts in first 100 ticks, 9 committed, 21 StartFailed). Died at tick 422.

All agents at Dusty Trail (Kael, Merchant Vara) were co-located for 1000+ ticks with no social interaction after the first 400 ticks. No trade actions occurred in the entire simulation.
**Root cause hypothesis**: Social goals (ShareBelief) are ranked lower than survival needs in the motive system. Once hunger/thirst became critical (after tick 400-500), the planner stopped selecting social goals in favor of Sleep, Relieve, and failed AcquireCommodity attempts. Forager Lina's complete isolation is structural — she spawned alone at Eldergrove Forest with no reason to travel (her food needs are met locally).
**Confidence**: HIGH — the social shutdown correlates exactly with the survival crisis onset.

### 10. Economic Stagnation -- CRITICAL

**Agent(s)**: Kael, Merchant Vara, Guard Theron
**Evidence**:
- **No trade actions** occurred in the entire simulation despite Merchant Vara being a merchant role.
- **No harvest actions** by anyone except Forager Lina (who harvested 28 apples at Eldergrove Forest).
- **Kael** holds 20 Coin but no food or water. Despite coins being available for potential trade, no trade goal was ever generated.
- **Merchant Vara** ended with empty inventory. Her 5 staff_market StartFailed events show she attempted market operations but couldn't succeed. Her planner selected SellCommodity(Grain) as a goal but never successfully planned or executed it.
- **Guard Theron** held Bow and Sword but no consumables. Died of hunger with weapons in hand.
- **Dusty Trail** (where 3 agents ended up) has 36 Waste, 20 Coin, 1 Bow, 1 Sword — no food or water. Thornwall Village has a Well and Mill but no raw materials. Hearthstone Inn has Firewood and Medicine but no food. Golden Fields has FieldPlot (agriculture?) but nobody visited.
- **Forager Lina** was self-sufficient at Eldergrove Forest (harvesting apples, eating, managing needs well) but produced only Waste as a byproduct, accumulating 13 Waste in inventory and 21 Waste at Eldergrove.

Cross-reference with Section 7: Merchant Vara's planner selected AcquireCommodity(Grain) and SellCommodity(Grain) goals, but these all budget-exhausted. Kael selected ExploreLocation(Thornwall Village, motivating_need: Thirst) — showing the planner understood water might be at the village — but the plan budget-exhausted.
**Root cause hypothesis**: The economy is structurally broken for this scenario. Resources are distributed across the place graph (Well at Thornwall Village, OrchardRow at Eldergrove, FieldPlot at Golden Fields) but agents can't plan multi-hop acquisition chains because the search space exceeds the expansion budget. The only agent who thrives (Forager Lina) is co-located with her resource source. Trade is impossible because no agent has both surplus goods and co-location with a buyer.
**Confidence**: HIGH — the economic failure is structural, driven by the same budget-exhaustion problem affecting survival needs.

## Cross-Cutting Patterns

### Pattern 1: Budget-Exhaustion Cascade
The dominant pathology across 3 of 4 agents is the same: AcquireCommodity budget-exhaustion. When agents need food or water but aren't co-located with the resource, the planner generates a multi-step plan (travel -> pick_up -> consume or travel -> use_facility -> consume) that branches into 1400-6200+ candidates and exceeds the expansion budget (112-300 expansions). This single failure mode cascades into:
- Sustained critical needs (smell 5)
- Unaddressed needs (smell 6)
- Behavioral collapse to sleep+relieve loops (smell 2)
- Social shutdown (smell 9)
- Economic stagnation (smell 10)
- Death (Guard Theron, smell 3)

### Pattern 2: Dusty Trail as a Death Trap
Dusty Trail has no food or water sources. Three agents ended up there (Kael, Merchant Vara, Guard Theron) and all experienced critical starvation. The place has travel affordances (1 target) but the planner can't complete the plan to travel and acquire resources. Dusty Trail accumulates Waste from consumption byproducts of the little food agents had early on, creating a growing pile of useless items (36 Waste at end).

### Pattern 3: Forager Lina as Control Group
Forager Lina demonstrates what happens when an agent IS co-located with resources: she thrives. Hunger avg 44, thirst avg 104 (though she drinks rarely — 5 times). She harvests, eats, manages needs, and her planner has 0 frontier-exhausted and 0 budget-exhausted outcomes. The contrast with the other 3 agents isolates the problem to resource accessibility, not planner logic.

### Pattern 4: Guard Theron's Death Chain
Guard Theron's death at tick 422 traces clearly:
1. Spawned at Dusty Trail with Bow+Sword but no food/water
2. Traveled to Thornwall Village (tick 9) — no eat/drink affordances there
3. Traveled between Thornwall Village and Dusty Trail, investigating violations and patrolling
4. AcquireCommodity(Water) budget-exhausted repeatedly (ticks 25-174)
5. Behavioral collapse: repertoire narrowed 6->2 types at tick 200, 2->1 at tick 400
6. Died tick 422 of hunger deprivation
7. Never ate or drank in 422 ticks of life

### Pattern 5: Dirtiness Universally Unaddressed
All 4 agents have dirtiness above 750 for 790-810 ticks. WashBasin exists at Hearthstone Inn but no agent ever traveled there. The `wash` affordance appeared only at Thornwall Village for Kael (tick 0) but was never used. This suggests either the wash action doesn't sufficiently address dirtiness, agents leave Thornwall Village before washing, or wash has preconditions that aren't met.

## Planner Diagnostics

| Agent | Plans Found | Frontier Exhausted | Budget Exhausted | Top Failed Goal | Candidate Count (typical) | Max Depth |
|-------|------------|-------------------|-----------------|----------------|--------------------------|-----------|
| Kael | 191 | 17 | 47 | AcquireCommodity(Water) | 1350-2657 | 5-7 |
| Merchant Vara | 189 | 5 | 40 | TreatWounds / AcquireCommodity(Water/Apple) | 1483-6233 | 3-9 |
| Forager Lina | 627 | 0 | 0 | (none) | n/a | n/a |
| Guard Theron | 120 | 16 | 9 | AcquireCommodity(Water) | 422-2085 | 5-7 |

**Assessment**: Budget exhaustion is structural, not parametric. The AcquireCommodity plan for multi-location resources generates thousands of candidates because each step in the chain (travel, pick_up, consume) branches into multiple affordance targets at the destination. Even at 300 max expansions, the planner cannot reach the goal. Simply raising the budget would help marginally but the combinatorial explosion at depth 4-9 with 1400-6200 candidates suggests the search space needs pruning or hierarchical decomposition, not just a bigger budget.

The TreatWounds budget exhaustion for Merchant Vara is also notable: 5739-6233 candidates at depth 3. The treatment plan likely requires acquiring Medicine (at Hearthstone Inn) which creates a similar multi-hop explosion.

## Summary Statistics

- Total findings: 8 (categories with severity other than NONE)
- By severity: 3 CRITICAL, 2 HIGH, 2 MEDIUM, 1 LOW
- Agents with issues: Kael (6 findings), Merchant Vara (7 findings), Guard Theron (7 findings), Forager Lina (2 findings — dirtiness + isolation)
- Clean agents: None (all agents have at least MEDIUM findings)

## Trace Quality Assessment

### Trace Sufficiency
The dump provides strong coverage for all 10 smells. Section 7's decision timeline, failed plan attempts, and affordance snapshots are particularly valuable for diagnosing the budget-exhaustion cascade. The entity-name mapping in Section 1 covers agents and places well.

### Limitations and Recommended Additions

| ID | Limitation | Classification | Rationale |
|----|-----------|----------------|-----------|
| TQ-1 | No explicit "why no eat/drink affordance" trace — we can see eat/drink is missing from affordances but not why (missing item? missing facility? wrong entity kind?) | Actionable | Would improve root-cause diagnosis for smells 5, 6, 10 from "missing affordance" to specific precondition failure |
| TQ-2 | Perception pass/fail only shows probability threshold, not what state change triggered the observation | Acceptable trade-off | Would help assess smell 1 (redundant perception) but the current data is sufficient to identify the pattern |
| TQ-3 | No inventory timeline — we see end-state inventory (Section 6) but not how it changed over time | Actionable | Would show when agents ran out of food/water items, enabling precise correlation with behavioral collapse timing (smell 2, 5) |
| TQ-4 | Dead agent affordances show tick-of-death snapshot but no indication of whether perception still fires for dead entities | Acceptable trade-off | Section 2 already notes death tick and post-death idle is expected behavior |
| TQ-5 | Item EntityIds in failed plan attempts can't be translated to names | Acceptable trade-off | Items appear in plan context but the primary diagnostic value comes from goal names and outcome metrics, not item identifiers |
| TQ-6 | No explicit "why staff_market StartFailed" reason — just the count | Actionable | 5 StartFailed events for Merchant Vara's core role action; knowing the specific precondition failure would improve smell 4 and 10 diagnosis |

**Actionable item details:**

- **TQ-1**: Recommended addition: When building affordances, log which action handlers were evaluated and which preconditions failed (e.g., "drink: no Water entity in agent inventory or at current place"). Scope: Observer-binary enhancement (add precondition-failure tracing to affordance dump).
- **TQ-3**: Recommended addition: Add a per-agent inventory timeline in 100-tick bins showing item counts by type (e.g., "tick 0-99: 2x Bread, 1x Water -> tick 100-199: 0x Bread, 0x Water"). Scope: Observer-binary enhancement.
- **TQ-6**: Recommended addition: When an action StartFailed, log the specific precondition or validation error. Scope: Engine instrumentation (action start handlers should report failure reason to the event log).
