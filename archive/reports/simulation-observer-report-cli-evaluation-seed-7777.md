# Simulation Observer Report

**Status**: ✅ COMPLETED

## Run Summary

- **Scenario**: `scenarios/cli-evaluation.ron`
- **Seed**: 7777
- **Ticks simulated**: 1440 (1 day)
- **Total events**: 9876
- **Agents**: Kael, Merchant Vara, Forager Lina, Guard Theron
- **Places**: Thornwall Village, Eldergrove Forest, Dusty Trail, Hearthstone Inn, Golden Fields
- **Agent death**: Guard Theron died at tick 422 (cause: NeedDeprivation { Hunger })

## Findings

### 1. Redundant Perception -- MEDIUM

**Agent(s)**: All four agents
**Evidence**: Kael observed himself 112 times, Merchant Vara 105 times, Guard Theron observed himself 112 times. Place entities observed 20-63 times. Total flagged: 16 redundant perception anomalies across all agents.
**Root cause hypothesis**: The perception system fires on every action event in the same location, causing agents to re-observe co-located entities (including themselves) and the place entity on every tick that generates an event. Since Kael and Merchant Vara were both at Dusty Trail for ~1380 ticks doing sleep+relieve every ~10 ticks, they observed each other and themselves on each cycle. The perception check itself passes (e.g., 950 permille for Kael, 875 permille for Merchant Vara), so it's not filtering — it's that the trigger fires too broadly.
**Confidence**: HIGH — the observation counts directly correlate with action frequency and co-location duration.

### 2. Action Loops -- HIGH

**Agent(s)**: Kael, Merchant Vara, Forager Lina
**Evidence**:
- **Kael**: From tick 500 onward, action repertoire collapsed to only sleep+relieve_wilderness. For 940 ticks (65% of the simulation), Kael did nothing but sleep ~10 times per 100-tick bin and relieve once. Behavioral transition flagged at tick 500 (4 types to 2 types).
- **Merchant Vara**: Mechanically flagged for sleep-sleep loop. From tick 900 onward, collapsed to sleep+relieve only. At tick 1400, further narrowed to sleep only. Behavioral transition at tick 1400 (2 types to 1 type).
- **Forager Lina**: From tick 800 onward, the planner entered a FreeCarryCapacity loop — selecting FreeCarryCapacity as the goal every tick (77+ times in bins 800-999 alone) with a 0-step GoalSatisfied plan. The plan was "found" instantly but never resulted in an action, causing 708 consecutive idle ticks. Her inventory was full of 12 Waste items, and the planner kept picking FreeCarryCapacity as the highest-priority goal without ever executing a drop action.

**Root cause hypothesis**:
- Kael/Merchant Vara: These agents are at Dusty Trail, which has no food or water source. Their only satisfiable needs are fatigue (sleep) and bladder (relieve). The planner generates AcquireCommodity goals for water/food but every plan search hits budget exhaustion (300 expansions, 2000+ candidates, 7-9 depth). The multi-step plan (travel to Thornwall Village -> find Well -> draw water -> drink) exceeds the planner's search budget. With no executable food/water plans, only sleep and relieve remain.
- Forager Lina: The FreeCarryCapacity goal produces a GoalSatisfied plan with steps=0, meaning the planner thinks the goal is already satisfied (possibly because a put_down affordance exists). But the agent never actually drops waste. This is likely a disconnect between the planner's world model and the action execution layer — the plan says "already done" but no action fires. This traps her in an infinite replanning loop with no progress.

**Confidence**: HIGH — the action timelines and decision traces clearly show the collapse pattern and its correlation with resource unavailability / inventory saturation.

### 3. Stuck Agents -- CRITICAL

**Agent(s)**: Forager Lina, Guard Theron
**Evidence**:
- **Forager Lina**: 708 consecutive idle ticks (tick ~730 to 1440). Despite active planning (1298 planning ticks, 1579 plans found, 0 failures), no action was executed after tick ~730. The planner selected FreeCarryCapacity every tick but the 0-step plan generated no executable action.
- **Guard Theron**: 1019 consecutive idle ticks (tick ~420 to 1440). However, Theron died at tick 422 — the post-death idle is expected. Pre-death analysis: From tick 200 onward (behavioral transition: 6 types to 2 types), Theron's repertoire collapsed to sleep+relieve only. His duty-focused goals (InvestigateViolation, Patrol) were all blocked because the violations were at Thornwall Village but he was at Dusty Trail. He never generated eat/drink goals despite critical hunger (915 avg permille) and thirst (943 avg permille).
- **Kael**: 34 consecutive idle ticks (borderline, mechanically flagged). Minor compared to the others.
- **Merchant Vara**: 27 consecutive idle ticks (borderline). Minor.

**Root cause hypothesis**:
- Forager Lina: FreeCarryCapacity planning trap (see smell 2). The planner is selecting a goal that produces a degenerate 0-step plan, preventing any other goal from being selected.
- Guard Theron: Goal generation never produced eat/drink goals. His GoalsSelected list shows InvestigateViolation, Patrol, Relieve, ShareBelief, Sleep — no AcquireCommodity for food or water. The guard role's goal generator apparently doesn't include survival needs in its goal repertoire, or they are ranked below duty goals that are perpetually blocked. This is a fatal gap in the goal generation system for this agent archetype.

**Confidence**: HIGH for Lina (clear planning trace), HIGH for Theron (goal list evidence is conclusive).

### 4. Failed Action Spirals -- LOW

**Agent(s)**: Merchant Vara, Guard Theron
**Evidence**:
- **Merchant Vara**: staff_market failed 5 times (all StartFailed). tell failed 3 times.
- **Guard Theron**: tell failed 21 times (StartFailed). investigate had 27 started but only 4 committed (23 uncommitted — possibly multi-tick actions interrupted by death, not classic spirals).
- **Kael**: tell failed 4 times.

**Root cause hypothesis**: The staff_market failures for Merchant Vara likely indicate a missing precondition at Dusty Trail (no market stall to staff). The tell failures across agents are likely due to the listener not being present or not being a valid target (possibly targeting Guard Theron after death, or before co-location). These are not spirals — the agents don't repeatedly retry the same failed action in rapid succession. They're isolated failures interspersed with successful actions.
**Confidence**: MEDIUM — the failures are real but don't constitute a "spiral" pattern. The investigate 23-uncommitted for Theron may be long-duration actions that were interrupted by death rather than validation failures.

### 5. Sustained Critical Needs -- CRITICAL

**Agent(s)**: All four agents
**Evidence**:
- **Guard Theron**: Hunger above 750 permille for 1215 ticks (ticks 225-1439), thirst above 750 permille for 1290 ticks (ticks 150-1439). Died at tick 422 from hunger deprivation.
- **Merchant Vara**: Thirst above 750 permille for 1257 ticks (ticks 183-1439, avg 926 permille), hunger above 750 permille for 265 ticks (ticks 1175-1439), dirtiness above 750 permille for 790 ticks.
- **Kael**: Thirst above 750 permille for 915 ticks (ticks 525-1439), hunger above 750 permille for 674 ticks (ticks 766-1439), dirtiness above 750 permille for 790 ticks.
- **Forager Lina**: Dirtiness above 750 permille for 810 ticks, thirst for 584 ticks, fatigue for 480 ticks, hunger for 361 ticks, bladder for 226 ticks.

**Root cause hypothesis**: The core issue is resource geography combined with planner budget limits.
- **Water**: The only Well is at Thornwall Village. Agents at Dusty Trail (Kael, Merchant Vara, Theron) repeatedly attempted AcquireCommodity(Water) plans but all hit budget exhaustion (300 expansions, 2000-2600 candidates, depth 6-9). The plan exists but is too deep for the search budget.
- **Food**: Kael's food goal is AcquireCommodity(Bread) — but no bread exists. Merchant Vara eats (10 times) but the food source is unclear. Theron never generated food goals at all.
- **Dirtiness**: The only WashBasin is at Hearthstone Inn. No agent traveled there. Wash affordance appeared for Kael at Dusty Trail at tick 15 but not at later snapshots — unclear if it was consumed or if the data source moved.
- **Forager Lina**: Had food (apples) and water goals that succeeded through tick ~730, then got trapped by FreeCarryCapacity. Her needs deteriorated because she stopped acting entirely.

**Confidence**: HIGH — the sustained need data is concrete, and Section 7 traces directly confirm budget-exhausted plan searches for water acquisition.

### 6. Unaddressed Needs -- CRITICAL

**Agent(s)**: Guard Theron, Merchant Vara
**Evidence**:
- **Guard Theron**: Hunger avg 915 permille but no eat action ever attempted. Thirst avg 943 permille but no drink action ever attempted. His goals list contains no AcquireCommodity or ConsumeOwnedCommodity goals for food or water.
- **Merchant Vara**: Thirst avg 926 permille but no drink action ever attempted. She did eat 10 times (addressing hunger partially). Her failed plans show AcquireCommodity(Water) hitting budget exhaustion, and AcquireCommodity(Apple) also budget-exhausted. Her initial affordance set at Thornwall Village had no drink affordance; after traveling to Dusty Trail at tick 60, she had eat but no drink.

**Root cause hypothesis**:
- **Guard Theron**: The goal generation system for this agent archetype does not produce food/water acquisition goals. His goal set is entirely duty-focused (investigate, patrol, share beliefs) plus basic needs (sleep, relieve). This is a critical gap — the guard's survival needs are not represented in goal generation at all.
- **Merchant Vara**: The drink action requires a water source (Well) as a target. No Well exists at Dusty Trail. The planner tried to plan multi-step water acquisition but couldn't find a plan within budget. Unlike Theron, Vara did generate water goals — the issue is plan feasibility, not goal generation. Her eat affordance existed because a food target was available at Dusty Trail at her arrival.

**Confidence**: HIGH — Guard Theron's goal list conclusively proves no survival goals were generated. Merchant Vara's failed plans confirm budget exhaustion on water acquisition.

### 7. Impossible Knowledge -- NONE

No evidence of agents acting on information they couldn't have obtained. Action targets are consistent with perception traces. Kael and Merchant Vara were co-located at Dusty Trail and observed each other. Guard Theron observed Kael and Merchant Vara while co-located at Thornwall Village and later at Dusty Trail. Forager Lina acted only on locally-perceived entities (apples, facilities in Eldergrove Forest).

### 8. Belief Staleness -- MEDIUM

**Agent(s)**: Kael, Merchant Vara, Forager Lina
**Evidence**:
- **Kael**: Believes Merchant Vara is at Dusty Trail (correct) and knows 13 Waste items there (consistent with end-state 36 Waste at Dusty Trail, though count is stale — he observed a subset). Does not know about the Well at Thornwall Village despite having been there (ticks 0-15 and again 411-463). His known entities include only 1 place (Dusty Trail). He appears to have lost or never formed beliefs about Thornwall Village's contents.
- **Merchant Vara**: Knows only 1 place (Dusty Trail), 1 agent (Kael). Does not know about Guard Theron despite being co-located. Knows 10 Waste items. No knowledge of any food or water sources anywhere.
- **Forager Lina**: Knows 0 agents (despite being in a world with 3 others), 1 place (Eldergrove Forest). She never traveled and never received any social information. Her beliefs about Eldergrove are accurate (ChoppingBlock, OrchardRow, 2 Apples, 8 Waste — end-state shows 2 Apples and 14 Waste there).
- **Guard Theron**: Most socially aware (3 agents known, 2 social observations, 1 heard belief). Knew about Kael, Merchant Vara, and himself at Dusty Trail. Accurate at time of death.

**Root cause hypothesis**: Belief staleness is partly structural — agents don't form beliefs about place contents (like Wells) unless they specifically observe them, and the perception system may not surface facility entities as observable. Kael visited Thornwall Village twice but doesn't know about the Well. This means even if the planner could handle the multi-step plan, the agent wouldn't have the belief foundation to plan "go to Thornwall and use the Well."
**Confidence**: MEDIUM — we can see what agents believe vs. what exists, but the perception-to-belief pipeline details are not fully captured in the dump.

### 9. Social Isolation -- MEDIUM

**Agent(s)**: Forager Lina
**Evidence**:
- **Forager Lina**: Spent all 1440 ticks alone in Eldergrove Forest. Never interacted with another agent. 0 social observations, 0 told beliefs, 0 heard beliefs, 0 agents known. Complete social isolation.
- **Kael + Merchant Vara**: Co-located at Dusty Trail for ~1380 ticks. Kael had 24 tell actions (19 committed, 4 failed). Merchant Vara had 20 tell actions (15 committed, 3 failed). They did communicate. Guard Theron was also co-located and told 9 times (all committed) plus received 21 failed tell attempts. Social interaction occurred between the Dusty Trail agents.
- However, after tick 500, no agent attempted tell or any social action. Kael's last tell is in the 400-499 bin. Merchant Vara's last tell is in the 200-299 bin. For the final ~1000 ticks, all social interaction ceased despite co-location.

**Root cause hypothesis**: Forager Lina's isolation is geographic — she's alone in Eldergrove Forest with no one to talk to and never traveled. The late-simulation social cessation at Dusty Trail correlates with behavioral collapse — once agents fell into sleep+relieve loops, they stopped generating ShareBelief goals (or those goals were deprioritized below the survival loop). Guard Theron's death at tick 422 also reduced the social group.
**Confidence**: HIGH for Lina's isolation (geographic). MEDIUM for late-game social cessation (correlation with behavioral collapse but causation unclear).

### 10. Economic Stagnation -- CRITICAL

**Agent(s)**: Merchant Vara, Kael, Guard Theron
**Evidence**:
- **Merchant Vara**: SellCommodity(Grain) was blocked 698 times — every planning tick. She's a merchant who never sold anything. No harvest, craft, or trade actions attempted. Her only economic activity was eating (consuming commodities). staff_market failed 5 times. She has no inventory at end-state.
- **Kael**: Holds 20 Coins at end-state but no trade actions. No craft or harvest actions. His only acquisition was 1 pick_up. His ExploreLocation goal (motivating_need: Thirst) suggests he was trying to find water sources but couldn't.
- **Guard Theron**: No economic activity at all (holds Bow + Sword from spawn, never traded or produced).
- **Forager Lina**: The exception — she harvested 13 times, ate 32 times, drank 5 times, picked up 26 items. She had a functioning economic loop (harvest apples -> eat apples -> manage waste) until the FreeCarryCapacity trap at tick ~730.
- **Dusty Trail** end-state: 36 Waste, 20 Coins, 1 Bow, 1 Sword — no food or water.
- **Thornwall Village**: Mill, Loom, Well — production facilities unused by any agent.
- **Hearthstone Inn**: Forge, WashBasin, 3 Firewood, 2 Medicine — entirely unvisited.

**Root cause hypothesis**: Multiple compounding failures prevent economic activity:
1. **Resource geography**: Water/food sources are at Thornwall Village and Eldergrove Forest. Three agents are stuck at Dusty Trail with no resources.
2. **Planner budget**: Multi-step economic plans (travel -> acquire resource -> produce -> sell) far exceed the GOAP planner's 300-expansion budget. Even simple water acquisition (travel -> draw -> drink) generates 2000+ candidates at depth 7-9.
3. **Missing affordances**: Merchant Vara had no sell/buy affordances at Dusty Trail (SellCommodity blocked 698 times). The market infrastructure requirements aren't met.
4. **Goal generation gaps**: Guard Theron's archetype doesn't generate survival/economic goals. Kael's food goal targets Bread (which doesn't exist in the world).
5. **Inventory saturation**: Forager Lina's waste buildup blocked her productive loop. The FreeCarryCapacity goal failed to actually clear inventory.

**Confidence**: HIGH — the evidence is comprehensive across affordances, failed plans, blocked desires, and end-state inventory.

## Cross-Cutting Patterns

### Pattern A: Dusty Trail Death Trap
Three of four agents (Kael, Merchant Vara, Guard Theron) ended up at Dusty Trail — a location with no food, water, or production facilities. Once there, the GOAP planner couldn't find plans to acquire water (budget-exhausted at 2000+ candidates) or food. This created a cascading failure: rising needs -> behavioral collapse to sleep+relieve -> further need escalation -> death (Theron) or sustained critical needs at 1000 permille (Kael, Vara). The fundamental issue is that the planner's 300-expansion budget cannot handle plans requiring travel + resource interaction at a distant location.

### Pattern B: Goal Generation vs. Survival
Guard Theron's death reveals a critical architectural gap: duty-focused agent archetypes can generate goals for their role (investigate, patrol) but not for biological survival (eat, drink). His hunger/thirst reached critical levels from the very start (min hunger 302, min thirst 303) but no food/water goal was ever generated. The goal generation system treats role-based goals and survival goals as separate systems, and for the guard archetype, survival goals are missing entirely.

### Pattern C: Forager Lina's FreeCarryCapacity Trap
Lina had the most successful first half — harvesting, eating, drinking, managing needs. But waste accumulation from consumption filled her inventory, and the planner produced a degenerate FreeCarryCapacity plan (0 steps, GoalSatisfied) that never resulted in an actual drop action. This trapped her in an infinite planning loop from tick ~730 to end, during which all her needs deteriorated to critical levels. The planner found 1579 plans with 0 failures, yet the agent was stuck — the plans were technically "valid" but produced no actions.

### Pattern D: Universal Dirtiness Crisis
All agents had dirtiness above 750 permille for 790-810 ticks. The only WashBasin is at Hearthstone Inn, which no agent visited. No wash affordance appears in any agent's final affordance snapshot except Kael's initial Dusty Trail arrival (tick 15, then disappeared). Washing is effectively impossible for all agents in this scenario.

### Pattern E: Guard Theron's Death Chain
Tick 0-150: Guard Theron starts at Dusty Trail, travels to Thornwall Village (tick 9), performs investigate/patrol/tell actions. Hunger min=302 and thirst min=303 at spawn — already elevated. Water acquisition plans fail (budget-exhausted, 2085 candidates at depth 5-6). Tick 150-200: Thirst crosses 750 permille. Tick 200: Behavioral transition (6 types to 2: sleep+relieve). Tick 225: Hunger crosses 750 permille. Tick 400: Second transition (2 types to 1: sleep only). Hunger and thirst both at 1000. Tick 422: Death from hunger deprivation. Contributing factors: no eat/drink goals in goal generator, no food/water at spawn location, budget-exhausted water plans even when at Thornwall Village (where the Well exists).

## Summary Statistics

- Total findings: 8 (categories with severity other than NONE)
- By severity: 3 CRITICAL, 2 HIGH, 2 MEDIUM, 1 LOW
- Agents with issues: Kael (action loops, sustained needs, belief staleness, economic stagnation), Merchant Vara (action loops, sustained needs, unaddressed thirst, economic stagnation), Forager Lina (stuck, action loops, sustained needs, social isolation), Guard Theron (stuck/dead, unaddressed hunger+thirst, economic stagnation)
- Clean agents: None

## Trace Quality Assessment

### Trace Sufficiency
The dump provides strong coverage for mechanical smells (1-6) and reasonable data for LLM smells (7-10). The decision timeline in Section 7 is particularly valuable for diagnosing planner behavior. The main gap is in perception-to-belief detail — we can see what agents believe and what they observed, but not why certain observable entities (like the Well at Thornwall Village) didn't become beliefs.

### Limitations and Recommended Additions

| ID | Limitation | Classification | Rationale |
|----|-----------|----------------|-----------|
| TQ-1 | No visibility into why Kael didn't form beliefs about Thornwall Village facilities (Well, Mill, Loom) despite visiting twice | Actionable | Prevents confident diagnosis of belief staleness (smell 8). The perception trace shows observations passed but doesn't show which entities were perceived and which beliefs formed. This is relevant to FND-07 (locality of information) — we can't verify whether the perception-to-belief pipeline respects locality correctly. |
| TQ-2 | Forager Lina's FreeCarryCapacity 0-step plan doesn't explain why no drop/put_down action fires | Actionable | The plan says GoalSatisfied with steps=0 but the agent has 12 Waste items. The dump shows plan selection but not plan execution outcome. This prevented confident root-cause diagnosis — is the plan degenerate (planner bug) or is action execution failing silently? |
| TQ-3 | No perception events after tick 820 in the raw trace tail — only 50 events shown | Acceptable trade-off | The binned perception timeline adequately covers perception frequency. More raw events would increase dump size substantially. |
| TQ-4 | Guard Theron's investigate action shows 27 started but only 4 committed, with no abort/fail breakdown for the remaining 23 | Actionable | Cannot distinguish between multi-tick actions interrupted by death and actual validation failures. This affects confidence in smell 4 (failed action spirals). |
| TQ-5 | No per-agent need trajectory over time (only min/max/avg and ticks-above-750) | Acceptable trade-off | The behavioral transition markers and tick ranges for sustained needs provide adequate temporal information. Full per-tick need values would be excessively verbose. |
| TQ-6 | Merchant Vara's eat affordance source unclear — she ate 10 times at Dusty Trail but Section 6 shows no food items at Dusty Trail at end-state | Acceptable trade-off | Could be consumed items. The per-event item tracking would be very verbose. The core finding (no water) is unaffected. |

**Actionable items detail:**

- **TQ-1** — Recommended addition: Add a "belief formation" trace to the perception section showing which entities became new beliefs and which were filtered/deduplicated. Scope: Observer-binary enhancement (log belief-formation events alongside perception events).
- **TQ-2** — Recommended addition: Add plan execution outcome to the decision timeline — after a plan is selected, show whether it produced an action, and if not, why (no steps, precondition failed at execution, etc.). Scope: Engine instrumentation (the decision runtime needs to emit a "plan execution result" event distinguishing "plan had 0 executable steps" from "plan step failed at execution").
- **TQ-4** — Recommended addition: Break down action lifecycle for multi-tick actions: started -> committed/aborted/expired/interrupted_by_death. Scope: Observer-binary enhancement (already has start/commit/abort tracking, needs to add death-interruption as a distinct category).

## Outcome

- Completion date: 2026-04-11
- What actually changed: this report's findings were consumed by the S90 mandatory tactical scoping implementation and by the archived remediation follow-up analysis in `archive/reports/simulation-remediation.md`.
- Deviations from original plan: none within the report itself; the report remained an investigative artifact and was archived once its findings had been acted on.
- Verification results: S90 follow-up implementation and closeout completed across `S90MANTACSCO-001` through `S90MANTACSCO-004`, and the remediation note already derived from this report remains archived.
