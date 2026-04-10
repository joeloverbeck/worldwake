# Simulation Observer Report

## Run Summary

- **Scenario**: `scenarios/cli-evaluation.ron`
- **Seed**: 7777
- **Ticks simulated**: 1440 (1 simulated day)
- **Total events**: 9876
- **Agents**: 4 (Kael, Merchant Vara, Forager Lina, Guard Theron)
- **Places**: 5 (Thornwall Village, Eldergrove Forest, Dusty Trail, Hearthstone Inn, Golden Fields)
- **Agent death**: Guard Theron died at tick 422 from NeedDeprivation(Hunger)

## Findings

### 1. Redundant Perception -- MEDIUM

**Agent(s)**: All four agents
**Evidence**: Kael observed himself 112 times, Merchant Vara 102 times. Merchant Vara observed Kael 96 times. Guard Theron observed himself 112 times. Forager Lina observed the same OrchardRow entity 14 times. These are entities at the agent's current location that are repeatedly observed on perception ticks regardless of state changes.
**Root cause hypothesis**: The perception system fires on all co-located entities each time it runs, not just entities whose state has changed since the last observation. Self-observation is particularly wasteful -- agents observe themselves every perception tick. This is a broad systemic pattern rather than an agent-specific issue.
**Confidence**: HIGH -- the high observation counts with low unique entity counts confirm repeated observation of the same entities. Whether this constitutes a "problem" depends on design intent; if perception is meant to track changing state (e.g., need levels of co-located agents), some redundancy is expected. The self-observation at 112 times is likely unnecessary.

### 2. Action Loops -- HIGH

**Agent(s)**: Kael, Merchant Vara, Guard Theron (post-death behavior aside)
**Evidence**:
- **Kael**: Behavioral transition at tick 500 from 4 action types to 2 (sleep + relieve_wilderness). From tick 500 onward, the action timeline shows exclusively `sleep×10, relieve_wilderness×1` per 100-tick bin for 940 consecutive ticks. The decision timeline from tick 600+ shows `selected=none, candidates=1, plans_found=0` dominating (78-82 occurrences per 100-tick bin), meaning the planner found only 1 candidate (Sleep) and nothing else viable.
- **Merchant Vara**: Flagged for a `[sleep -> sleep]` loop repeated 3 times. Similar pattern to Kael -- from tick 900 onward, only sleep + relieve_wilderness. Her eat actions persist longer (through tick 800) because she had Grain to consume, but she eventually runs out and collapses to the same minimal pattern.
- **Forager Lina**: Active and diverse through tick 700, then completely idle for 708 consecutive ticks. Her action repertoire did not narrow gradually -- it dropped to zero abruptly. Section 7 shows 0 budget-exhausted and 0 frontier-exhausted plans, suggesting the planner stopped finding any candidates at all after resources were depleted.

**Root cause hypothesis**: Agents at Dusty Trail (Kael, Merchant Vara) lack food/water affordances at their final location. Once owned consumables are depleted, the only plannable goals are Sleep and Relieve. The planner tries AcquireCommodity goals but they budget-exhaust (Kael: 51 budget-exhausted; Merchant Vara: 43 budget-exhausted), meaning multi-step plans to travel-and-acquire are too deep for the expansion budget. This traps agents in a sleep-only loop despite critical hunger/thirst. For Forager Lina, the complete stop at tick ~732 suggests all apples were consumed and the harvest action could no longer produce new ones (or carry capacity was full of Waste preventing new pickups).
**Confidence**: HIGH -- the decision timeline data directly shows the planner selecting only Sleep/Relieve, with acquisition goals consistently budget-exhausting.

### 3. Stuck Agents -- HIGH

**Agent(s)**: Forager Lina (708 consecutive idle ticks), Guard Theron (1019 consecutive idle ticks, but 1018 are post-death)
**Evidence**:
- **Forager Lina**: 708 consecutive idle ticks from ~tick 732 to 1440. The action timeline shows her last actions in the 700-799 bin (sleep x4, eat x1, harvest x1, pick_up x1), then nothing. Her plan search outcomes show 1579 plans found with 0 frontier-exhausted and 0 budget-exhausted -- the planner successfully finds plans when candidates exist, but after tick ~732, no actionable candidates remain. Her inventory at end-state is 12x Waste, suggesting carry capacity is completely consumed by waste, preventing new pickups.
- **Guard Theron**: 1019 consecutive idle ticks, but he died at tick 422. His stuck period is almost entirely post-death (expected behavior). The 1 tick pre-death of idle at the boundary is trivial. His pre-death behavior (ticks 200-422) already showed behavioral collapse to sleep + relieve_wilderness only.
- **Kael**: 34 consecutive idle ticks (flagged). Minor compared to other agents, likely normal planning gaps between action completions.
- **Merchant Vara**: 27 consecutive idle ticks (flagged). Same as Kael -- minor.

**Root cause hypothesis**: Forager Lina's stuck behavior is caused by inventory saturation with Waste. She has 12 Waste items and no way to discard them (FreeCarryCapacity goal appears in her goals list but apparently cannot execute or doesn't clear enough space). With full inventory, she cannot pick_up harvested apples, cannot eat, and has no affordances left at Eldergrove Forest that don't require inventory space. Guard Theron's stuck period is expected post-death.
**Confidence**: HIGH for Forager Lina (inventory data + action timeline clearly show the transition). EXPECTED for Guard Theron (post-death).

### 4. Failed Action Spirals -- MEDIUM

**Agent(s)**: Merchant Vara (staff_market), Guard Theron (tell, investigate)
**Evidence**:
- **Merchant Vara**: 5 StartFailed for `staff_market` (0 committed out of 5 attempts). She has a `SellCommodity{Grain}` goal that plans through StaffMarket, but the action consistently fails to start. The plan shows `feasibility=Unlikely` for SellCommodity, suggesting a known precondition gap.
- **Guard Theron**: 21 StartFailed for `tell`, 27 started but only 4 committed for `investigate`. The tell failures occurred at Dusty Trail (e2g0) where the agent had targets but something prevented the action from starting. The investigate actions had a 27:4 start:commit ratio -- 23 investigations were interrupted (InterruptForReplan with HigherPriorityGoal or SuperiorSameClassPlan) before they could commit, suggesting the planner kept replacing the current plan mid-execution.
- **Kael**: 4 StartFailed for `tell`, relatively minor.

**Root cause hypothesis**: Merchant Vara's staff_market failures suggest a missing precondition at Dusty Trail (no market stall registered, or market not established). Guard Theron's investigate interruption cascade is caused by the planner continually finding higher-priority goals (AcquireCommodity for Water) that it cannot actually execute (budget-exhausted), creating a thrashing cycle: investigate -> interrupted for Water acquisition -> Water plan fails -> falls back to investigate -> interrupted again.
**Confidence**: MEDIUM -- the staff_market failure pattern is clear but the specific precondition is not in the dump. Guard Theron's thrashing hypothesis is well-supported by the decision timeline showing repeated AcquireCommodity interrupts of investigate actions.

### 5. Sustained Critical Needs -- CRITICAL

**Agent(s)**: All four agents
**Evidence**:
- **Guard Theron**: Hunger above 750 for 1215 ticks (225-1439, but died at 422). Thirst above 750 for 1290 ticks (150-1439). His hunger averaged 915 and thirst 943 -- near-maximum throughout his life. He never ate or drank. Death at tick 422 from hunger deprivation.
- **Kael**: Thirst above 750 for 915 ticks (525-1439). Hunger above 750 for 674 ticks (766-1439). Dirtiness above 750 for 790 ticks (650-1439). He ate 5 times and drank 5 times total -- all in the first 400 ticks. After moving to Dusty Trail at tick ~463, no further consumption.
- **Merchant Vara**: Thirst above 750 for 1257 ticks (183-1439). Never drank. Hunger above 750 for 265 ticks (1175-1439). Dirtiness above 750 for 790 ticks (650-1439). She ate 10 times (Grain) but thirst was never addressed.
- **Forager Lina**: Hunger above 750 for 361 ticks, thirst for 584, fatigue for 480, dirtiness for 810, bladder for 226. She actively ate and drank through tick ~700 but all needs escalated after she went idle.

**Root cause hypothesis**: The core problem is resource scarcity at agent locations combined with planner inability to find multi-step acquisition plans:
1. **Dusty Trail** (Kael, Merchant Vara, Guard Theron's final location) has no water source and limited food. The Well is in Thornwall Village but the planner cannot build travel-to-village-then-drink plans within the expansion budget.
2. **Eldergrove Forest** (Forager Lina) has no water source either. She has drink in her tick-0 affordances but this disappears after initial water is consumed.
3. Dirtiness goes unaddressed because the WashBasin is at Hearthstone Inn -- no agent ever travels there.
**Confidence**: HIGH -- needs trajectory data, action counts, and budget-exhausted plan failures all converge on the same story.

### 6. Unaddressed Needs -- CRITICAL

**Agent(s)**: Merchant Vara (thirst), Guard Theron (hunger + thirst)
**Evidence**:
- **Merchant Vara**: Thirst averaged 926 but drink was never attempted across 1440 ticks. Her tick-0 affordances at Thornwall Village did not include `drink` (unlike Kael who had drink at the same location). After traveling to Dusty Trail, drink does not appear in her affordances either. Failed plan attempts show AcquireCommodity{Water} consistently budget-exhausting (300 expansions, depth 9, 1483 candidates at Thornwall; 300 expansions, depth 4, 2611 candidates at Dusty Trail).
- **Guard Theron**: Hunger averaged 915, thirst 943. No eat or drink actions were ever attempted. His tick-0 affordances at Dusty Trail show no eat or drink actions. His goals list contains no food/thirst goals -- only InvestigateViolation, Patrol, Relieve, ShareBelief, and Sleep. His failed plan attempts show AcquireCommodity{Water} budget-exhausting.

**Root cause hypothesis**:
- **Merchant Vara**: No drink affordance at her starting location (Thornwall Village) despite having a Well there. This may be a scenario configuration issue (no water commodity available at the Well?) or a missing recipe/harvesting rule connecting the Well to water production. The planner generates AcquireCommodity{Water} goals but cannot find a viable plan.
- **Guard Theron**: Starting at Dusty Trail with no eat/drink affordances and no food/water items. His role profile (guard) generates investigation and patrol goals that dominated his early ticks, while survival needs were never generated as goals at all. His goals list shows no AcquireCommodity goals for food, suggesting the goal generator doesn't produce eat/drink goals for his profile, or they are ranked below duty goals and never reach planning.
**Confidence**: HIGH -- the affordance lists and goal selections directly confirm the absence of survival-oriented actions for these agents.

### 7. Impossible Knowledge -- NONE

No evidence of agents acting on information about entities they never perceived. All action targets correspond to entities within the agents' perception traces. Cross-referencing Kael's actions at Dusty Trail with his perception trace shows he only targeted entities he observed. Forager Lina stayed in Eldergrove and only interacted with local entities. Guard Theron's tell targets were Kael and Merchant Vara, both co-located at his locations.

### 8. Belief Staleness -- MEDIUM

**Agent(s)**: Kael, Merchant Vara
**Evidence**:
- **Kael**: Knows 16 entities but believes Dusty Trail contains himself, Merchant Vara, and 13 Waste items. He knows only 1 place (Dusty Trail). He previously visited Thornwall Village (ticks 0-64 and briefly at tick 411-463) but his belief summary doesn't mention it. He doesn't know about the Well (water source) at Thornwall Village despite having been there.
- **Merchant Vara**: Knows only 1 place (Dusty Trail) despite starting at Thornwall Village. Her belief about Dusty Trail contents lists Kael and 10 Waste -- she doesn't believe Guard Theron is there (he is, as a corpse) and doesn't know about Merchant Vara's own items.
- **Forager Lina**: Knows 0 agents despite being in a world with 3 others. She stayed entirely in Eldergrove Forest and never interacted socially. Her beliefs accurately reflect her local environment (ChoppingBlock, OrchardRow, Apples, Waste).
- **Guard Theron**: Most complete beliefs (3 agents, 1 place) but still only knows Dusty Trail despite starting there and visiting Thornwall Village.

**Root cause hypothesis**: Belief formation is tied to perception events, and agents only form beliefs about entities they observe. Since agents spent most of their time at Dusty Trail or Eldergrove, their knowledge is geographically limited. The more concerning issue is that agents who visited Thornwall Village early (Kael, Merchant Vara, Guard Theron) don't retain beliefs about it -- either beliefs about previously-visited places decay, or the belief summary only captures current/recent beliefs. This limits the planner's ability to plan travel-to-resource actions because the agent doesn't "know" a water source exists elsewhere.
**Confidence**: MEDIUM -- the belief data clearly shows limited knowledge, but whether this represents "staleness" (lost accurate beliefs) vs. "never formed" (perception didn't create place-knowledge beliefs) requires engine-level investigation.

### 9. Social Isolation -- MEDIUM

**Agent(s)**: Forager Lina (complete isolation), Kael + Merchant Vara (late-game isolation)
**Evidence**:
- **Forager Lina**: Zero social observations, zero told beliefs, zero heard beliefs. She was alone in Eldergrove Forest for all 1440 ticks with no tell, ask_witness, or trade actions. No other agent ever visited her location.
- **Kael and Merchant Vara**: Co-located at Dusty Trail from tick ~70 onward (1370+ ticks together). Kael told 19 times (with 4 StartFailed) and Merchant Vara told 15 times (with 3 StartFailed). However, all social actions occurred in the first ~400 ticks. From tick 500 onward, despite being co-located, zero social interactions occurred -- both agents collapsed into sleep+relieve loops.
- **Guard Theron**: Told 9 times and had 21 StartFailed for tell. His social activity was concentrated in ticks 0-200 before behavioral collapse.

**Root cause hypothesis**: Social actions (tell) are only planned when the planner selects ShareBelief goals. After behavioral collapse (around tick 400-500 for most agents), the planner stops generating ShareBelief goals because survival needs dominate goal ranking but cannot be satisfied, leaving only Sleep/Relieve as viable goals. Social interaction is an "extra" that only happens when basic needs are met or the planner has headroom.
**Confidence**: HIGH for Forager Lina's isolation (complete geographic separation with no travel capability to reach others). MEDIUM for late-game isolation of co-located agents (expected consequence of behavioral collapse).

### 10. Economic Stagnation -- CRITICAL

**Agent(s)**: All agents
**Evidence**:
- **Dusty Trail** (3 agents end-state): Contains 36 Waste, 20 Coins, 1 Bow, 1 Sword. No food, no water. Kael has 20 Coins but nothing to spend them on. Merchant Vara's inventory is empty despite her merchant role -- she tried staff_market 5 times but all failed.
- **Eldergrove Forest** (Forager Lina): Contains 2 Apples and 14 Waste. Forager Lina harvested 13 times and ate 32 times, producing 12 Waste. She consumed faster than she produced, and her inventory became saturated with Waste.
- **Thornwall Village**: Has Mill, Loom, Well -- production facilities that were never used by any agent.
- **Hearthstone Inn**: Has Forge, WashBasin, 3 Firewood, 2 Medicine -- never visited by any agent.
- **Golden Fields**: Has FieldPlot, GravePlot -- never visited.
- **Trade**: Zero trade actions across the entire simulation.
- **Crafting**: Zero craft actions. Forager Lina harvested but no agent crafted.
- **Market**: Merchant Vara attempted staff_market 5 times, all StartFailed. No market economy was established.

**Root cause hypothesis**: Multiple compounding failures:
1. **Budget-exhausted multi-step plans**: The planner cannot find plans that require traveling to a different location to acquire resources. AcquireCommodity goals consistently budget-exhaust with 300 expansions at depth 4-9, suggesting the action chain travel->pick_up->consume (or travel->harvest->pick_up->consume) branches too widely for the expansion budget.
2. **No resource distribution**: Agents concentrated at Dusty Trail (a trail, not a resource hub) and Eldergrove Forest. No agent traveled to Thornwall Village (with Well, Mill, Loom) or Hearthstone Inn (with WashBasin, Forge) after the initial ticks.
3. **Waste accumulation**: Consumption produces Waste that fills inventory, preventing new pickups. Forager Lina's FreeCarryCapacity goal appears in her goal list but apparently couldn't execute effectively.
4. **Market failure**: Merchant Vara's staff_market precondition failures prevented any market establishment, blocking the entire trade pathway.

**Confidence**: HIGH -- comprehensive end-state inventory, zero trade/craft counts, and budget-exhaustion patterns all confirm complete economic failure.

## Cross-Cutting Patterns

### Pattern 1: Budget-Exhaustion-Driven Behavioral Collapse
The dominant pattern across the simulation is a cascading failure: agents cannot plan multi-step resource acquisition (budget exhausts at 300 expansions), so they cannot eat/drink, leading to rising needs that dominate goal ranking, further crowding out any non-survival goals, trapping agents in sleep+relieve loops until death (Guard Theron) or simulation end. This affects Kael, Merchant Vara, and Guard Theron identically. The key bottleneck is the planner's inability to find travel+acquire plans within its expansion budget, with candidate counts of 1400-2600 suggesting the search space branches too widely at each step.

### Pattern 2: Geographic Isolation Prevents Resource Access
Resources are distributed across 5 places, but agents concentrate at 2 (Dusty Trail and Eldergrove Forest). Thornwall Village (Well, Mill, Loom), Hearthstone Inn (WashBasin, Forge, Firewood, Medicine), and Golden Fields (FieldPlot, GravePlot) are entirely unused after the initial ticks. The planner's budget limitation means agents cannot plan multi-location journeys, making them effectively stranded wherever they settle.

### Pattern 3: Guard Theron's Death Chain
Guard Theron died at tick 422 from hunger. The causal chain: started at Dusty Trail with no food affordances -> his guard role profile generated InvestigateViolation and Patrol goals that consumed ticks 0-200 -> AcquireCommodity{Water} goals budget-exhausted repeatedly -> behavioral transition at tick 200 to sleep+relieve only -> hunger hit 1000 by tick ~300 -> died at tick 422. He never generated an AcquireCommodity{food} goal at all -- his goals list shows no hunger-relief goals, suggesting his profile's goal generator doesn't prioritize survival.

### Pattern 4: Waste Accumulation Cascade (Forager Lina)
Forager Lina was the most economically active agent (13 harvests, 32 eats, 26 pickups) but was ultimately defeated by waste accumulation. Each consumption produces Waste, and with no way to discard it, her 12 Waste items filled her carry capacity, making harvest->pickup->eat cycles impossible. Her FreeCarryCapacity goal appeared in her goal list but couldn't resolve the issue, possibly because dropping waste or discarding it has no supporting action/affordance.

## Summary Statistics

- Total findings: 8 (categories with severity other than NONE)
- By severity: 3 CRITICAL, 2 HIGH, 3 MEDIUM
- Agents with issues: Kael (action loops, sustained needs, belief staleness), Merchant Vara (action loops, unaddressed thirst, failed market, economic stagnation), Forager Lina (stuck, waste saturation, social isolation), Guard Theron (death from unaddressed hunger/thirst)
- Clean agents: None

## Trace Quality Assessment

### Trace Sufficiency
The dump provides good coverage for mechanical smells (1-6) and reasonable data for LLM-only smells (7-10). The decision timeline in Section 7 is particularly valuable for root-cause analysis. The main limitation is the lack of per-tick need values (only min/max/avg and ticks-above-750 are provided), which makes it harder to pinpoint exact transition points.

### Limitations and Recommended Additions

| ID | Limitation | Classification | Rationale |
|----|-----------|----------------|-----------|
| TQ-1 | No per-agent goal generation log (what goals were considered but not selected) | Acceptable trade-off | Would improve smell 6 analysis (why Guard Theron never generated food goals) but the blocked desires absence + affordance data is sufficient for diagnosis. Adding this would significantly increase dump size. |
| TQ-2 | Belief summary doesn't distinguish "never formed" vs. "formed and decayed" beliefs | Actionable | Prevented confident assessment of smell 8 -- cannot determine if agents forgot about Thornwall Village resources or never formed those beliefs. This directly affects root-cause analysis for why agents don't travel to resource locations. |
| TQ-3 | No waste/inventory capacity tracking over time | Actionable | Would materially improve root-cause diagnosis for Forager Lina's stuck behavior (smell 3) and economic stagnation (smell 10). Currently we can only see end-state inventory, not when capacity was reached. |
| TQ-4 | staff_market StartFailed doesn't include the specific precondition that failed | Actionable | Would directly explain Merchant Vara's market failure (smell 4, smell 10). Currently the failure reason is opaque. |

**Actionable item details:**

- **TQ-2**: Recommended addition: Include a "belief history" subsection showing belief formation and decay events (e.g., "tick 5: formed belief about Well at Thornwall Village", "tick 200: belief decayed"). Scope: Observer-binary enhancement (aggregate belief events from the event log).
- **TQ-3**: Recommended addition: Track carry capacity utilization per agent over time in 100-tick bins (e.g., "0-99: 2/10 slots, 100-199: 5/10 slots"). Scope: Observer-binary enhancement (compute from inventory events).
- **TQ-4**: Recommended addition: Include the specific failed precondition in StartFailed action reporting (e.g., "staff_market StartFailed: no MarketStall component at location"). Scope: Observer-binary enhancement (capture the error reason from the action framework's validation response).
