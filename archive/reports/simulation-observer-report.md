# Simulation Observer Report

**Status**: ✅ COMPLETED

## Run Summary

- **Scenario**: `scenarios/cli-evaluation.ron`
- **Seed**: 7777
- **Ticks simulated**: 1440 (1 simulated day)
- **Total events**: 9876
- **Agents**: Kael (e5g0), Merchant Vara (e6g0), Forager Lina (e7g0), Guard Theron (e8g0)
- **Places**: Thornwall Village, Eldergrove Forest, Dusty Trail, Hearthstone Inn, Golden Fields
- **Deaths**: Guard Theron died at tick 422 (cause: NeedDeprivation { Hunger })

## Findings

### 1. Redundant Perception -- MEDIUM

**Agent(s)**: All four agents
**Evidence**: Kael observed himself (e5g0) 112 times and Merchant Vara 102 times. Merchant Vara observed Kael 96 times and herself 105 times. Guard Theron observed himself 112 times. Forager Lina observed herself 11 times and entity e27g0 14 times. All agents observe co-located agents and places repeatedly across the full simulation.
**Root cause hypothesis**: The perception system fires on every action event at the agent's location, re-observing the same co-located entities each time. Since Kael and Merchant Vara are co-located at Dusty Trail for 1373/1381 ticks respectively and performing sleep+relieve continuously, each action triggers perception of the same unchanged agents. The entities (agents, place) don't change state between observations, making the re-observations truly redundant.
**Confidence**: HIGH that this is redundant (entities are not changing state between observations). The perception system lacks a "changed since last observed" gate.

### 2. Action Loops -- CRITICAL

**Agent(s)**: All four agents (Forager Lina most severely)
**Evidence**:
- **Forager Lina**: From tick ~730 onward (708 consecutive idle ticks), the planner selects `FreeCarryCapacity` every single tick with `GoalSatisfied[steps=0]` — a 0-step plan that produces no executable action. This repeats hundreds of times across ticks 800-1439 (visible in decision timeline: "selected=FreeCarryCapacity ... GoalSatisfied[steps=0]" appearing 50-100+ times per 100-tick bin). Her inventory holds 12 Waste items, and the FreeCarryCapacity goal is selected because Waste fills her carry capacity, but the 0-step plan never actually drops any items. Meanwhile her hunger, thirst, fatigue, and dirtiness all rise to critical levels. This is a textbook degenerate plan loop: plans "found" but 0 actions execute.
- **Kael**: Behavioral collapse at tick 500 — action repertoire narrows from 4 types (eat, tell, relieve, sleep) to 2 (sleep, relieve_wilderness). From tick 500-1439 (940 ticks), he performs only sleep (~10/bin) and relieve_wilderness (~1/bin). He never eats or drinks again after tick ~400, despite hunger reaching 1000 and thirst reaching 1000.
- **Merchant Vara**: Similar pattern — eats occasionally through tick 800 but never drinks (entire simulation). By tick 900, narrows to sleep+relieve only. Behavioral transition at tick 1400: only sleep.
- **Guard Theron**: Collapses from 6 action types to 2 (sleep+relieve) by tick 200, then to 1 (sleep) by tick 400. Dies at tick 422.

**Root cause hypothesis**: Two distinct mechanisms:
1. **Forager Lina**: The `FreeCarryCapacity` goal produces a `GoalSatisfied[steps=0]` plan — the planner considers the goal already satisfied or plans a no-op. Since this goal always "wins" the priority ranking (primary score 280000), it blocks all other goals (eat, drink, relieve, sleep). The agent is stuck in infinite replanning with no progress.
2. **Kael/Vara/Theron**: After consuming initial food/water supplies, no eat/drink affordances remain at Dusty Trail. The planner attempts `AcquireCommodity{Water}` but budget-exhausts every time (too many candidates, search space too deep). Without being able to plan food/water acquisition, the agents default to the only goals they can satisfy: Sleep and Relieve.

**Confidence**: VERY HIGH — decision timeline data directly shows the degenerate plan loop for Lina and the budget exhaustion preventing food/water plans for others.

### 3. Stuck Agents -- HIGH

**Agent(s)**: Forager Lina (708 ticks), Guard Theron (1019 ticks, but 1018 post-death), Kael (34 ticks), Merchant Vara (27 ticks)
**Evidence**:
- **Forager Lina**: 708 consecutive idle ticks (ticks ~732-1439). No actions at all despite continuous planning. The planner was active (1298 planning ticks, 1579 plans found) but all plans from tick ~730 onward are 0-step FreeCarryCapacity degenerate plans.
- **Guard Theron**: 1019 consecutive idle ticks, but he died at tick 422 — the 1018 post-death idle ticks are expected. Pre-death, his action repertoire collapsed rapidly (6 types at tick 100, 2 types by tick 200, 1 type by tick 400).
- **Kael/Merchant Vara**: Short idle stretches (27-34 ticks) — these fall within normal sleep/relieve cycles and are borderline, not pathological.

**Root cause hypothesis**: Forager Lina's stuck state is caused by the degenerate FreeCarryCapacity loop (see smell 2). Guard Theron's pre-death idleness is caused by inability to plan food/water acquisition. Kael and Merchant Vara's short idle stretches are normal pauses between sleep cycles.
**Confidence**: HIGH for Forager Lina (clearly pathological). Guard Theron's post-death idle is expected; pre-death stuckness is HIGH confidence pathological. LOW concern for Kael/Vara.

### 4. Failed Action Spirals -- LOW

**Agent(s)**: Merchant Vara (staff_market), Guard Theron (tell)
**Evidence**: Merchant Vara had 5 StartFailed attempts for `staff_market`. Guard Theron had 21 StartFailed attempts for `tell` and 4 for `tell`. Kael had 4 StartFailed for `tell`.
**Root cause hypothesis**: `staff_market` failures likely indicate a precondition not met (no stock to sell, or market infrastructure not available at Dusty Trail). `tell` StartFailed events are likely targeting agents who are busy/unavailable or have moved away. These are not spirals — the agents don't repeatedly attempt the same failing action in rapid succession; they're interspersed with successful actions.
**Confidence**: MEDIUM — these are isolated failures, not spirals. The `staff_market` failures deserve investigation to confirm whether market infrastructure exists at Dusty Trail.

### 5. Sustained Critical Needs -- CRITICAL

**Agent(s)**: All four agents
**Evidence**:
- **Guard Theron**: Hunger above 750 for 1215 ticks (ticks 225-1439, though dead at 422). Thirst above 750 for 1290 ticks (ticks 150-1439). Average hunger 915, average thirst 943. These directly caused his death.
- **Merchant Vara**: Thirst above 750 for 1257 ticks (ticks 183-1439) — nearly the entire simulation. Average thirst 926. Hunger above 750 for 265 ticks (ticks 1175-1439). Dirtiness above 750 for 790 ticks.
- **Kael**: Thirst above 750 for 915 ticks (ticks 525-1439). Hunger above 750 for 674 ticks (ticks 766-1439). Dirtiness above 750 for 790 ticks.
- **Forager Lina**: Dirtiness above 750 for 810 ticks. Thirst above 750 for 584 ticks. Fatigue above 750 for 480 ticks. Hunger above 750 for 361 ticks.

**Root cause hypothesis**: The scenario places most agents at or quickly travelling to Dusty Trail, which has no food/water sources and no production facilities. Initial supplies are consumed by tick ~100-400, after which no replenishment path exists. Thornwall Village has a Well (water source) but agents at Dusty Trail can't successfully plan the multi-step "travel to village → drink → return" sequence because `AcquireCommodity{Water}` budget-exhausts. Forager Lina at Eldergrove Forest has apples but her carry capacity fills with Waste, blocking further harvest/eat cycles. No agent has access to a `wash` facility — Hearthstone Inn has a WashBasin but no agent ever travels there.
**Confidence**: VERY HIGH — the causal chain is clear: depleted consumables + inability to plan acquisition = sustained critical needs.

### 6. Unaddressed Needs -- CRITICAL

**Agent(s)**: Merchant Vara (thirst), Guard Theron (hunger, thirst)
**Evidence**:
- **Merchant Vara**: Average thirst 926 but `drink` action was never attempted across the entire 1440-tick simulation. She ate 10 times but never drank once. Her final affordances at Dusty Trail include no `drink` action.
- **Guard Theron**: Average hunger 915 with no `eat` action ever attempted. Average thirst 943 with no `drink` action ever attempted. His affordances at Dusty Trail show no `eat` or `drink` actions. His goals selected list includes no food/water-related goals — only InvestigateViolation, Patrol, ShareBelief, Relieve, Sleep.

**Root cause hypothesis**:
- **Merchant Vara**: She had `eat` in her affordances (ate Grain 10 times) but `drink` was never available. At tick 0 in Thornwall Village, her affordances don't list `drink` (Kael had it, she didn't — possibly because she didn't have a water container). After travelling to Dusty Trail at tick 60, no drink affordance appeared. The planner repeatedly budget-exhausted on `AcquireCommodity{Water}` — the search space is too large (1483-2522 candidates, 9 depth levels).
- **Guard Theron**: Started at Dusty Trail (tick 0 affordances: no eat, no drink). His role profile (guard/patrol) generated InvestigateViolation and Patrol goals but no survival goals. He never acquired food or water items. His failed plans show only ShareBelief frontier-exhaustions and AcquireCommodity{Water} budget-exhaustions. No eat/hunger goal was ever generated.

**Confidence**: VERY HIGH — affordance data confirms eat/drink were structurally unavailable to these agents at their locations. Guard Theron's goal set completely lacks survival goals, which is a planner/goal-generation gap.

### 7. Impossible Knowledge -- NONE

No evidence of agents acting on information they couldn't have obtained through perception or social channels. Kael and Merchant Vara share beliefs through `tell` actions while co-located. Guard Theron had social observations and told beliefs. Forager Lina, isolated at Eldergrove Forest, acts only on locally observable entities.

### 8. Belief Staleness -- MEDIUM

**Agent(s)**: Kael, Merchant Vara, Guard Theron
**Evidence**:
- **Kael** believes Dusty Trail contains "13x Waste" and knows Merchant Vara's location — but doesn't know about places with food/water (his known entities include 0 food/water items besides what he consumed). He knew 1 place (Dusty Trail) at end of simulation despite starting at Thornwall Village.
- **Merchant Vara** knows only Dusty Trail and 10x Waste. Knows Kael but not Guard Theron (despite being co-located). Her knowledge is extremely limited — 12 known entities total.
- **Guard Theron** knew 3 agents and 1 place but 12x Waste. His beliefs about Thornwall Village didn't help him find food there.
- **Forager Lina** knows Eldergrove Forest contents (ChoppingBlock, OrchardRow, 2 Apples, 8 Waste) but 0 agents — she has been completely isolated the entire simulation.

**Root cause hypothesis**: Beliefs are not stale per se (they accurately reflect what agents have observed) but they are extremely narrow. Agents at Dusty Trail have no beliefs about resources at other locations, so they can't plan to travel to get food/water. This is more of an information poverty problem than staleness — the belief system is working as designed but the agents' exploration radius is too limited.
**Confidence**: MEDIUM — beliefs are accurate but impoverished. Whether this is a design problem (agents should explore more) or scenario problem (Dusty Trail is a dead-end trap) needs further investigation.

### 9. Social Isolation -- HIGH

**Agent(s)**: Forager Lina
**Evidence**:
- **Forager Lina**: Spent entire 1440 ticks at Eldergrove Forest. 0 social observations, 0 told beliefs, 0 heard beliefs. Never interacted with any other agent. Knows 0 agents. Completely isolated the entire simulation.
- **Kael and Merchant Vara**: Co-located at Dusty Trail for ~1370+ ticks each but social interaction dropped off. Kael had 24 tell attempts (19 committed), Merchant Vara had 20 (15 committed), but all social activity occurs in ticks 0-400. From tick 500 onward, neither agent performs any social actions despite being co-located. Their behavioral collapse to sleep+relieve eliminated social interaction.
- **Guard Theron**: Had social activity (tell, investigate) in ticks 0-200 before dying. 9 tells committed, but 21 tell StartFailed events suggest difficulty communicating.

**Root cause hypothesis**: Forager Lina starts at Eldergrove Forest (alone) and never travels — she has no `tell` targets and no social affordances. The other agents' social collapse is a downstream effect of behavioral collapse: once agents narrow to sleep+relieve, social goals are never selected because survival needs dominate. Additionally, the `tell` StartFailed rate (Theron: 21/30, Kael: 4/28, Vara: 3/23) suggests communication preconditions are difficult to satisfy.
**Confidence**: HIGH for Forager Lina's complete isolation (structural — no other agents at her location). MEDIUM for the social collapse of Dusty Trail agents (downstream of behavioral collapse).

### 10. Economic Stagnation -- CRITICAL

**Agent(s)**: All agents
**Evidence**:
- **No trade actions** occurred across the entire simulation.
- **Merchant Vara**: Had `staff_market` in affordances and attempted it 5 times (all StartFailed). Had `SellCommodity{Grain}` in goals selected. Had store_stock, collect_display_stock, stage_stock_for_sale affordances in final snapshot. But never successfully staffed a market or sold anything. Her inventory is empty at simulation end.
- **Kael**: Holds 20 Coins but never traded. No trade or buy affordances visible.
- **Forager Lina**: Harvested 13 apples and picked up 26 items, but her inventory is 12 Waste. She consumed all the apples, produced Waste, and now her carry capacity is full of Waste with no way to dispose of it (the FreeCarryCapacity degenerate plan loop).
- **Guard Theron**: Holds 1 Bow, 1 Sword — never traded these despite co-location with other agents.
- **Resource distribution**: Thornwall Village has a Well, Mill, Loom. Eldergrove Forest has apples and an OrchardRow. Hearthstone Inn has a Forge, WashBasin, Firewood, Medicine. Golden Fields has FieldPlot, GravePlot. Dusty Trail (where 3 of 4 agents spend most time) has nothing — only waste and items agents brought.

**Root cause hypothesis**: Multiple compounding failures:
1. **Location mismatch**: Agents cluster at Dusty Trail (a resource-poor waypoint) instead of spreading to resource-rich locations.
2. **AcquireCommodity budget exhaustion**: The plan search space for acquiring commodities from other locations is too large (1000-6000+ candidates, 5-9 depth levels), causing systematic budget exhaustion. Agents can't plan multi-step acquisition.
3. **Merchant Vara's market failures**: `staff_market` StartFailed 5 times — likely because there's no market stall at Dusty Trail or preconditions aren't met.
4. **No cross-location supply chains**: Agents don't travel to gather resources and bring them back for trade. The planner can't build these multi-step plans.
5. **FreeCarryCapacity trap**: Forager Lina's waste-filled inventory prevents further economic activity.

**Confidence**: VERY HIGH — zero trade across the entire simulation with a merchant agent present is definitive economic failure.

## Cross-Cutting Patterns

**Pattern 1: Resource Trap at Dusty Trail**
Three of four agents (Kael, Merchant Vara, Guard Theron) converge on Dusty Trail — a location with no food, water, or production facilities. Once initial supplies are consumed (by tick ~100-400), agents cannot plan acquisition from other locations because `AcquireCommodity` budget-exhausts. This creates a death spiral: rising needs → failed acquisition plans → behavioral collapse to sleep+relieve → eventual starvation. Guard Theron died from this; Kael and Merchant Vara survived the 1-day simulation but would die on day 2.

**Pattern 2: Degenerate Plan Loop (Forager Lina)**
Forager Lina demonstrates a distinct failure mode: the FreeCarryCapacity goal produces 0-step GoalSatisfied plans that never execute an action. This goal "wins" priority ranking every tick (score 280000), blocking all other goals (eat, drink, sleep, relieve). The agent plans continuously but does nothing for 708 ticks. Her Waste-filled inventory is the trigger: she needs to drop waste to free capacity, but the "plan" to do so has 0 steps. This is a planner architecture issue — GoalSatisfied with 0 steps should not block other actionable goals.

**Pattern 3: Guard Theron Death Chain**
Guard Theron's death at tick 422 follows a clear chain: no eat/drink affordances at any visited location → hunger/thirst rise from tick 0 → AcquireCommodity{Water} budget-exhausts repeatedly → goal set dominated by InvestigateViolation and Patrol (role duties) with no survival goals generated → behavioral collapse at tick 200 → death at tick 422 from hunger. His guard role profile appears to suppress or deprioritize survival goals fatally.

**Pattern 4: Universal Dirtiness Crisis**
All agents have dirtiness above 750 for 790-810 ticks. The only WashBasin is at Hearthstone Inn, which no agent ever visits. The `wash` affordance only appeared in Kael's early snapshots at Thornwall Village (where there is no WashBasin listed in Section 6 — the Well may have doubled as a wash source). Dirtiness is a universal unaddressed need.

## Planner Diagnostics

| Agent | Plans Found | Frontier Exhausted | Budget Exhausted | Top Failed Goal | Candidate Count (typical) | Max Depth |
|-------|------------|-------------------|-----------------|----------------|--------------------------|-----------|
| Kael | 191 | 16 | 51 | AcquireCommodity{Water} | 1350-2902 | 5-7 |
| Merchant Vara | 189 | 4 | 43 | AcquireCommodity{Water} / TreatWounds | 1483-6233 | 3-9 |
| Forager Lina | 1579 | 0 | 0 | (none — all 0-step degenerate) | 10-13 | 0 |
| Guard Theron | 119 | 14 | 15 | AcquireCommodity{Water} | 422-2808 | 5-7 |

**Assessment**: Budget exhaustion is structural for AcquireCommodity goals — the search space branches explosively (1000-6000+ candidates at depths 5-9) because multi-location acquisition requires travel+pickup+consume chains across the place graph. Raising `max_node_expansions` alone would not help at these candidate counts; the branching factor needs pruning (e.g., limiting candidate opportunities to believed-reachable locations, or decomposing acquisition into travel + local-acquire sub-goals). Merchant Vara's TreatWounds budget exhaustion (3-6233 candidates at depth 3) suggests the treatment planning space is also explosively branchy. Forager Lina's 0 budget/frontier exhaustions are deceptive — her planner "succeeds" every time but with 0-step no-op plans.

## Summary Statistics

- Total findings: 8 (categories with severity other than NONE)
- By severity: 3 CRITICAL, 2 HIGH, 2 MEDIUM, 1 LOW
- Agents with issues: Kael (5 smells), Merchant Vara (6 smells), Forager Lina (5 smells), Guard Theron (5 smells)
- Clean agents: none

## Trace Quality Assessment

### Trace Sufficiency
The dump provides excellent coverage for mechanically-detectable smells (1-6) and good coverage for LLM-analysis smells (7-10). Section 7's decision timeline is the most valuable diagnostic tool, directly revealing planner pathologies. The main limitation is the omission of per-tick need values (only trajectory min/max/avg and tick-above-750 counts), which makes precise cross-referencing of need levels with specific planning decisions approximate rather than exact.

### Limitations and Recommended Additions

| ID | Limitation | Classification | Rationale |
|----|-----------|----------------|-----------|
| TQ-1 | Waste accumulation visible only in end-state inventory; no per-tick inventory tracking | Actionable | Forager Lina's FreeCarryCapacity loop diagnosis required inferring that Waste accumulated from harvest byproducts. Per-tick inventory snapshots (even sampled every 100 ticks) would confirm exactly when carry capacity filled and when the degenerate loop began. |
| TQ-2 | GoalSatisfied[steps=0] plans not flagged as anomalous by mechanical detector | Actionable | The detector flagged STUCK_AGENT (708 idle ticks) but not the root cause: 0-step plans that the planner considers "found" but produce no action. This is a distinct pathology from frontier/budget exhaustion and should have its own anomaly category. |
| TQ-3 | Section 7 decision timeline lines exceed 5000 tokens, making offset-based reading impractical | Acceptable trade-off | The data is present and accessible via Grep and bash; the density is a consequence of rich decision data which is more valuable than formatting convenience. |
| TQ-4 | No per-tick need values in dump (only trajectory statistics) | Acceptable trade-off | Trajectory stats (min/max/avg/ticks-above-750) plus behavioral transition markers provide sufficient diagnostic power for need analysis. Per-tick values for 4 agents x 5 needs x 1440 ticks would add 28,800 data points with diminishing returns. |
| TQ-5 | `staff_market` StartFailed events lack failure reason | Actionable | Merchant Vara's 5 failed market-staffing attempts are unexplained — the dump shows StartFailed but not which precondition failed. Adding failure reasons to StartFailed events would clarify whether this is a location problem, stock problem, or infrastructure problem. |

**Actionable items:**

- **TQ-1 Recommended addition**: Add sampled inventory snapshots (every 100 ticks) to Section 2's per-agent summary. **Scope**: Observer-binary enhancement.
- **TQ-2 Recommended addition**: Add a `DEGENERATE_PLAN_LOOP` anomaly category in the mechanical detector: flag when an agent selects the same goal 50+ times in a 100-tick window with GoalSatisfied[steps=0]. **Scope**: Observer-binary enhancement.
- **TQ-5 Recommended addition**: Include the failed precondition name/reason in StartFailed action events. **Scope**: Engine instrumentation (the action framework's start-failure path should propagate the reason to the event log).

## Outcome

- **Completion date**: 2026-04-11
- **What actually changed**:
  - this report's findings were mined into follow-up golden/spec work, especially the S91 planner-pathology series
  - the AcquireCommodity budget-exhaustion and `FreeCarryCapacity` zero-step findings were converted into shipped proof/fix work
  - the Guard Theron survival-goal finding was later reassessed against the current branch and is no longer reproducible in the scenario-adjacent slice
- **Deviations from original plan**:
  - the report remains a historical observer artifact; not every original hypothesis remained live after remediation work landed
- **Verification results**:
  - report exploited by `archive/specs/S91-planner-pathology-golden-tests.md`, `archive/specs/S91-acquire-commodity-prerequisite-guidance.md`, and `archive/specs/S92-free-carry-capacity-zero-step-loop-fix.md`
