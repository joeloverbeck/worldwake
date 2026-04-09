**Status**: COMPLETED

# Simulation Observer Report

## Run Summary

- **Scenario**: `scenarios/cli-evaluation.ron`
- **Seed**: 7777
- **Ticks simulated**: 1440 (1 simulated day)
- **Total events**: 7654
- **Agents**: 4 (Kael, Merchant Vara, Forager Lina, Guard Theron)
- **Places**: 5 (Thornwall Village, Eldergrove Forest, Dusty Trail, Hearthstone Inn, Golden Fields)

## Findings

### 1. Redundant Perception -- MEDIUM

**Agent(s)**: All four agents
**Evidence**: Kael observed e5g0 (self) 92 times, e6g0 71 times; Merchant Vara observed e5g0 82 times, e8g0 71 times; Guard Theron observed e0g0 46 times, e5g0 47 times, e8g0 82 times. Every agent has multiple entities observed 20-90+ times.
**Root cause hypothesis**: The perception system fires on a regular cadence (roughly every 7-8 ticks based on the perception trace) and re-observes all entities at the agent's current location, regardless of whether their state has changed. Since 3 agents (Kael, Merchant Vara, Forager Lina) are co-located at Dusty Trail for 1000+ ticks doing only sleep/relieve_wilderness, they repeatedly perceive each other and the place without new information. This is architecturally expected behavior (perception fires on events, not state changes), but the sheer volume suggests the perception system may benefit from change-detection gating.
**Confidence**: HIGH -- this is expected given the co-location and long idle period, but the volume is notable.

### 2. Action Loops -- HIGH

**Agent(s)**: Kael, Merchant Vara, Forager Lina
**Evidence**: All three agents collapse into an identical `sleep×10 + relieve_wilderness×1` pattern per 100-tick bin starting around tick 400-500, continuing unchanged for 900+ ticks until simulation end. The action timeline shows a clear behavioral transition:
- Kael: active and varied (eat, drink, pick_up, tell, travel) ticks 0-499, then pure sleep+relieve from 500 onward
- Merchant Vara: tell+relieve ticks 0-99, then sleep+relieve from 100 onward (never ate or drank at all)
- Forager Lina: drink, eat, pick_up ticks 0-199, then sleep+relieve from 300 onward

This is not patrol behavior. The agents are stuck in a maintenance-only loop with no goal-directed action for the majority of the simulation.
**Root cause hypothesis**: The agents exhausted locally available food/water resources at Dusty Trail (which has no resource sources -- only Waste, Coins, weapons). Their planner cannot find valid plans for eat/drink because no consumables are in inventory or at the current location. Rather than traveling to locations with resources (Thornwall has Water, Eldergrove has Apples), the planner falls back to the only satisfiable actions: sleep and relieve_wilderness. This suggests either (a) the planner doesn't generate travel-to-resource plans, (b) belief staleness prevents agents from knowing resources exist elsewhere, or (c) the goal ranking prioritizes fatigue/bladder relief over hunger/thirst when no immediate solution is available.
**Confidence**: HIGH -- the pattern is unambiguous and pathological.

### 3. Stuck Agents -- CRITICAL

**Agent(s)**: Guard Theron (1019 ticks), Merchant Vara (43 ticks), Kael (33 ticks)
**Evidence**: Guard Theron has 1019 consecutive idle ticks -- from approximately tick 420 to 1439 with zero actions. His action timeline shows activity only in ticks 0-499: investigate×21, patrol×8, travel×4, tell×7, eat×5 in the first 400 ticks, then sleep×3 in 400-499, then nothing. Meanwhile his thirst sits at 943 avg (above 750 for 1290 ticks) and he never once attempts to drink. His needs (hunger avg 230, thirst avg 943, fatigue avg 281) show hunger is managed through eating but thirst is completely unaddressed.
**Root cause hypothesis**: Guard Theron's behavioral profile appears to lack a drinking affordance or the planner cannot synthesize a drink plan. He eats 5 times (hunger stays manageable) but never drinks despite critical thirst. After his active patrol/investigate phase exhausts around tick 420, he becomes completely inert. The 1019-tick idle streak is pathological -- no sleep, no relieve, nothing. This may indicate the planner enters a state where it cannot find any valid plan at all (not even sleep), possibly because a precondition failure blocks the entire planning cycle. Kael's 33-tick and Vara's 43-tick idle streaks are brief and likely just planning gaps between action cycles.
**Confidence**: CRITICAL for Theron (unambiguously broken), LOW concern for Kael/Vara (brief gaps).

### 4. Failed Action Spirals -- LOW

**Agent(s)**: Kael (4 tell StartFailed), Guard Theron (investigate 21 started / 4 committed, patrol 8 started / 1 committed)
**Evidence**: Kael had 4 tell actions fail to start. Guard Theron started 21 investigate actions but only 4 committed (17 failed or were interrupted), and started 8 patrols but only 1 committed. Forager Lina had 1 relieve_wilderness start that didn't commit (19 started vs 18 committed).
**Root cause hypothesis**: Theron's investigate/patrol failures likely relate to his early-phase behavior at Thornwall Village and Dusty Trail where targets may have moved or conditions changed during the action duration. The 21→4 investigate ratio is notable but concentrated in ticks 0-99 (all 21 investigate starts are in that bin), suggesting rapid-fire investigation attempts that mostly fail validation. This could be a target availability issue or a precondition that changes between start and commit. Kael's 4 tell failures are minor.
**Confidence**: MEDIUM for Theron's investigate pattern (warrants investigation but concentrated in early ticks), LOW for others.

### 5. Sustained Critical Needs -- CRITICAL

**Agent(s)**: All four agents
**Evidence**:
- **Thirst**: Theron 1290 ticks (90% of sim), Vara 1257 ticks (87%), Lina 1075 ticks (75%), Kael 915 ticks (64%)
- **Hunger**: Vara 1165 ticks (81%), Lina 850 ticks (59%), Kael 671 ticks (47%)
- **Dirtiness**: Kael, Vara, Lina all 790 ticks (55%) -- identical onset at tick 650
- **Fatigue**: 0 ticks for all (managed by constant sleeping)
- **Bladder**: 0 ticks for all (managed by relieve_wilderness)

Thirst is the most extreme -- every agent spends the majority of the simulation critically thirsty. Hunger is nearly as bad for Vara (who never eats). Dirtiness hits critical for 3 agents simultaneously at tick 650 and stays there.
**Root cause hypothesis**: The simulation has a systemic resource access failure. Water sources exist only at Thornwall Village, Apple sources at Eldergrove Forest. All 4 agents converge on Dusty Trail (which has no food/water sources) and never leave. The planner satisfies fatigue and bladder (which require no resources) but cannot satisfy hunger, thirst, or dirtiness at the current location. The dirtiness spike at tick 650 for all 3 agents simultaneously suggests dirtiness accumulates from a system tick and no wash action is available at Dusty Trail (WashBasin is at Hearthstone Inn).
**Confidence**: CRITICAL -- this is the simulation's central failure mode.

### 6. Unaddressed Needs -- CRITICAL

**Agent(s)**: Merchant Vara (hunger, thirst), Guard Theron (thirst)
**Evidence**: Merchant Vara never attempted eat or drink across 1440 ticks despite hunger avg 889 and thirst avg 926. Guard Theron never attempted drink despite thirst avg 943. These are not cases of "tried and failed" -- the actions were never even started.
**Root cause hypothesis**: For Vara, the planner never generates eat or drink goals, or generates them but cannot find plans. Vara's only actions across the entire simulation are tell, relieve_wilderness, sleep, and travel. She has no items in inventory at end-state and sits at Dusty Trail with no food/water sources. The planner likely lacks an affordance chain for "travel to resource location → pick up food → eat" or the goal ranking never selects hunger/thirst goals. For Theron, the same applies to drinking specifically -- he eats 5 times (suggesting he had food access early on at Thornwall) but never drinks, suggesting the drink affordance or water-resource knowledge is missing from his planning context.
**Confidence**: CRITICAL -- the planner is fundamentally failing to address survival needs for these agents.

### 7. Impossible Knowledge -- NONE

**Evidence**: No instances found. Action targets align with perception traces. Agents act only on entities at their current location. Guard Theron's early investigate actions target entities at his known locations (Thornwall Village, Dusty Trail). Kael's tell actions target co-located agents. No agent acts on entities they haven't observed.
**Confidence**: HIGH -- perception traces are detailed enough to verify.

### 8. Belief Staleness -- HIGH

**Agent(s)**: All four agents
**Evidence**: All agents' belief summaries show they know only about Waste items (and for Theron: Bow, Grain, Sword). No agent has beliefs about food sources (Apple at Eldergrove), water sources (Water at Thornwall), or crafting facilities (Forge, WashBasin at Hearthstone Inn). Despite Forager Lina spending 381 ticks at Eldergrove Forest (where Apples exist), her known entities are 12 Waste items at Dusty Trail. Despite Kael and Merchant Vara starting at Thornwall Village (with Water source), their beliefs contain no water-related knowledge.

Critically, agents know 0 other agents and 0 places in their belief summary. This means agents have no beliefs about the existence of other locations or the resources there, making travel-to-resource planning impossible.
**Root cause hypothesis**: The perception system may not be creating beliefs about resource sources (infrastructure entities like Apple source, Water source) or place entities. Agents perceive events and discover entities, but the belief formation may be limited to item entities (Waste, Coin, etc.) and miss resource-producing infrastructure. With no beliefs about places or resources at other locations, the planner cannot generate multi-step plans involving travel.
**Confidence**: HIGH -- the belief summaries are strikingly empty of strategic information.

### 9. Social Isolation -- HIGH

**Agent(s)**: Forager Lina, Guard Theron (partial)
**Evidence**: Kael, Merchant Vara, and Forager Lina are co-located at Dusty Trail for 1000+ ticks. Kael has 16 tell actions and Vara has 11, but Lina has 0 tell actions and 0 social observations, 0 told beliefs, 0 heard beliefs. She exists alongside other agents for the entire simulation without any social interaction.

Guard Theron has 7 tell actions (early phase) and told beliefs from Kael and Vara, but after tick ~420 he is inert for 1019 ticks with no social activity despite being co-located with all 3 other agents.

All agents have 0 social observations in their belief summary. Kael and Vara exchanged tells early on but no agent engages in Trade or AskWitness actions.
**Root cause hypothesis**: Lina's agent profile may lack social action affordances (no tell capability configured). The broader absence of trade is likely because no agent has complementary resources to exchange (most have only Waste). The complete lack of AskWitness suggests the action isn't available in this scenario or isn't prioritized by the planner.
**Confidence**: HIGH for Lina's complete isolation, MEDIUM for the broader absence of trade/AskWitness (may be expected given resource scarcity).

### 10. Economic Stagnation -- CRITICAL

**Agent(s)**: All four agents
**Evidence**: Resources exist across the map: Apple source at Eldergrove, Water source at Thornwall, Forge/WashBasin at Hearthstone Inn, FieldPlot/GravePlot at Golden Fields, Mill/Loom at Thornwall. Yet all 4 agents are stuck at Dusty Trail, which has zero production infrastructure. No agent performs harvest, craft, or trade actions across the entire 1440 ticks. End-state inventories are minimal: Kael has 20 Coin + 15 Waste, Theron has 1 Bow + 5 Grain + 1 Sword, Vara and Lina have nothing. The place contents show 48 Waste accumulated at Dusty Trail (from relieve_wilderness).

Agents with critical hunger (Kael, Vara, Lina all above 596 avg) sit at a location with no food sources while Apples exist at Eldergrove (1 travel hop away). Agents with critical thirst sit where there's no water while Thornwall has a water source. No agent attempts to travel to resource locations after the initial early-phase movement.
**Root cause hypothesis**: This is the same root cause as smells 2, 5, 6, and 8 converging: agents lack beliefs about resources at other locations, so the planner cannot generate travel-to-harvest-to-consume plans. The economic system is completely dormant because the information locality principle is working (agents only know what they perceive) but the perception/belief pipeline isn't providing enough strategic information about the world topology and resource distribution to enable rational planning.
**Confidence**: CRITICAL -- the simulation has zero economic activity despite available resources.

## Cross-Cutting Patterns

### Pattern 1: The Dusty Trail Convergence
All 4 agents converge on Dusty Trail (a transit location with no resources) within the first ~100 ticks and never leave. This single fact drives most of the simulation's pathologies. Kael spends 1424/1440 ticks there, Vara 1376, Lina 1058, Theron 1315. The initial travel appears to be Theron patrolling from his starting location, and the others traveling from their starts (Kael/Vara from Thornwall, Lina from Eldergrove). Once at Dusty Trail, no agent has beliefs about other locations that would motivate return travel.

### Pattern 2: Sleep Dominance / Behavioral Collapse
By tick 500, all AI-controlled agents exhibit the same degenerate behavior: sleep every 10 ticks + relieve_wilderness every ~75 ticks. This represents a planning failure mode where only zero-resource-cost actions remain satisfiable. The planner correctly identifies these as achievable goals but cannot find plans for higher-priority needs (hunger, thirst, dirtiness) because the required resources/facilities don't exist at the current location.

### Pattern 3: Belief-Action Gap
Agents perceive events frequently (167 observations for Kael, 204 for Vara, 91 for Lina, 140 for Theron) but their belief summaries are impoverished -- only Waste items and a few starting possessions. No beliefs about places, agents, resource sources, or facilities. The perception system fires but doesn't build actionable world knowledge. This is the root cause of the economic stagnation: without beliefs about where resources exist, the planner cannot generate multi-location plans.

### Pattern 4: Guard Theron's Complete Shutdown
Theron is the most severely affected agent: 1019 consecutive idle ticks (71% of the simulation), never drinks despite critical thirst (943 avg), yet manages to eat 5 times. His early phase (ticks 0-100) is the most varied of any agent (investigate, patrol, travel, tell, eat, pick_up) suggesting his profile is well-configured for guard duties, but something causes complete behavioral shutdown around tick 420. The transition from "most active" to "completely inert" is abrupt and suggests a planner failure rather than gradual resource depletion.

## Summary Statistics

- **Total findings**: 10 smell categories assessed
- **By severity**: 4 CRITICAL, 2 HIGH, 1 MEDIUM, 1 LOW, 1 NONE, 1 (mixed CRITICAL/LOW)
- **Agents with issues**: Kael (action loop, sustained needs, belief staleness), Merchant Vara (action loop, sustained needs, unaddressed needs, belief staleness), Forager Lina (action loop, sustained needs, social isolation, belief staleness), Guard Theron (stuck 1019 ticks, unaddressed thirst, belief staleness)
- **Clean agents**: None

## Trace Quality Assessment

The dump provides good coverage for mechanical smells (1-6) with precise tick ranges, action counts, and need trajectories. The per-agent action timeline in 100-tick bins was essential for identifying the behavioral transition points.

**Limitations**:
- The perception trace shows only the last ~60 events (ticks 1212-1435), missing the critical early phase where agents form initial beliefs. Early perception data would help diagnose why agents don't build beliefs about resource locations.
- Belief summaries show only current state, not belief formation/decay over time. A belief timeline would clarify whether agents ever knew about resources and forgot, or never learned.
- No affordance trace -- we can't see what the planner considered and rejected. This is the biggest blind spot: we can see agents don't eat/drink/travel, but can't see whether the planner generated those goals and failed to find plans, or never generated the goals at all.
- No failed-plan trace -- when the planner attempts to find a plan and fails, there's no record of what it tried. This would directly explain smells 2, 3, 5, and 6.
- Guard Theron's abrupt shutdown at tick ~420 needs investigation. The dump shows he stops acting but doesn't show why the planner stops producing any output at all (not even sleep).

## Outcome

- Completion date: 2026-04-09
- What actually changed: Archived this observer report after its findings were exploited into follow-up remediation and planning material.
- Deviations from original plan: None. The report remains as historical analysis rather than active work.
- Verification results: Confirmed the report was marked completed before archival and moved into `archive/reports/`.
