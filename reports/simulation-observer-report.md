# Simulation Observer Report

## Run Summary

- **Scenario**: `scenarios/cli-evaluation.ron`
- **Seed**: 7777
- **Ticks simulated**: 1440 (1 simulated day)
- **Total events**: 13415
- **Agents**: Kael (e5g0), Merchant Vara (e6g0), Forager Lina (e7g0), Guard Theron (e8g0)
- **Places**: Thornwall Village (e0g0), Eldergrove Forest (e1g0), Dusty Trail (e2g0), Hearthstone Inn (e3g0), Golden Fields (e4g0)
- **Deaths**: Guard Theron at tick 1342 (cause: NeedDeprivation { Hunger })

## Findings

### 1. Redundant Perception -- MEDIUM

**Agent(s)**: All four agents
**Evidence**: Kael observed Guard Theron (e8g0) 1043 times and Dusty Trail (e2g0) 990 times. Merchant Vara observed Guard Theron 1037 times. Guard Theron observed Dusty Trail 793 times. Even Forager Lina (isolated in Eldergrove Forest with only 103 total observations) observed a single entity (e27g0) 26 times.
**Root cause hypothesis**: The perception system fires on every tick where the agent is co-located with observable entities, regardless of whether the entity's state has actually changed. In cases where 3 agents share Dusty Trail for 1000+ ticks, this produces hundreds of redundant observations per entity. The massive jump in perception volume at tick 800+ (Kael goes from ~20 observations/bin to 135-195/bin) corresponds to Guard Theron's post_notice explosion flooding the location with SocialArtifacts, each of which becomes a new observable entity.
**Confidence**: HIGH — the observation counts far exceed any plausible rate of entity state change.

### 2. Action Loops -- CRITICAL

**Agent(s)**: Kael, Merchant Vara, Guard Theron
**Evidence**:
- **Kael**: From tick 400 onward, action repertoire collapses to only `sleep` + `relieve_wilderness`. The action timeline shows a rigid pattern of sleep×10, relieve_wilderness×1 for every 100-tick bin from 400-1439. This is a behavioral collapse lasting 1040 ticks (72% of the simulation).
- **Merchant Vara**: Identical collapse from tick 400 onward (sleep×10 + relieve_wilderness×1). Mechanically flagged as action loop: `[sleep → sleep]` repeated 3 times. Behavioral transition at tick 400: repertoire narrowed from 7 types to 2 types. By tick 1400, further narrowed to 1 type (sleep only).
- **Guard Theron**: Different pathology. From tick 800, the agent enters a `post_notice` loop: 68 notices at tick 800-899, 92 at 900-999, 100 at 1000-1099, continuing until death at tick 1342. During this period, all other actions cease except occasional relieve_wilderness. The agent posts 487 notices total while starving to death. Behavioral transitions at tick 900 (8→2 types) and tick 1000 (2→1 type).
**Root cause hypothesis**: For Kael and Merchant Vara, the collapse is driven by the AcquireCommodity budget-exhaustion spiral (see smell 5/10). They cannot find food and the planner's only successful goals become Sleep and Relieve. For Guard Theron, the PostNotice goal likely has a very high drive score (possibly from ThreatWarning obligations) and the action completes instantly (1 tick), causing it to dominate all other goals including survival needs.
**Confidence**: HIGH — the action timelines clearly show the collapse points and the monotonic patterns afterward.

### 3. Stuck Agents -- MEDIUM

**Agent(s)**: Guard Theron (97 consecutive idle ticks), Kael (22 consecutive idle ticks)
**Evidence**: Guard Theron's 97-tick idle stretch occurs post-death (tick 1342-1439), which is expected behavior for a dead agent. Kael's 22-tick idle stretch is borderline — it may represent a planning period where no goal could be found.
**Root cause hypothesis**: Guard Theron's stuck period is entirely post-mortem (died at tick 1342, 97 remaining ticks). Kael's 22-tick idle period likely coincides with repeated budget-exhausted plan searches for AcquireCommodity goals.
**Confidence**: HIGH for Guard Theron (post-death, expected). MEDIUM for Kael (short enough to be normal planning overhead, but correlated with planner failures).

### 4. Failed Action Spirals -- LOW

**Agent(s)**: Kael (9 StartFailed for tell), Merchant Vara (12 StartFailed for tell), Guard Theron (20 StartFailed for tell)
**Evidence**: All StartFailed events are on the `tell` action. No other actions have StartFailed counts. The failures are concentrated in the early ticks when agents attempt ShareBelief goals.
**Root cause hypothesis**: The tell action fails at start when the listener has already heard the same belief or isn't co-located. The frontier-exhausted pattern at depth 0 with `build_successor returned None` for ShareBelief goals suggests the planner cannot find a valid tell action because the preconditions (novel belief + co-located listener) aren't met. This is not a spiral — the agents move on to other goals after failure.
**Confidence**: HIGH — this is expected behavior for belief-sharing with saturation, not a pathological spiral.

### 5. Sustained Critical Needs -- CRITICAL

**Agent(s)**: All four agents

**Evidence**:
- **Kael**: Hunger above 750‰ for 671 ticks (769-1439), thirst above 750‰ for 922 ticks (518-1439), dirtiness above 750‰ for 790 ticks (650-1439). Max hunger and thirst both hit 1000‰. Only 5 eat and 5 drink actions total in 1440 ticks.
- **Merchant Vara**: Hunger above 750‰ for 1171 ticks (269-1439) — 81% of the simulation. Average hunger 892‰. ZERO eat actions ever attempted (flagged as Anomaly 17: UNADDRESSED_NEED). Thirst above 750‰ for 860 ticks. Only 5 drink and 3 harvest:Water actions.
- **Forager Lina**: Dirtiness above 750‰ for 810 ticks (630-1439). However, hunger and thirst are well-managed (hunger avg 34‰, thirst avg 68‰) — Lina is the only agent successfully feeding herself via harvest:Harvest Apples (28 harvests, 64 eats).
- **Guard Theron**: Hunger above 750‰ for 336 ticks (1104-1439), thirst above 750‰ for 370 ticks (1070-1439), fatigue above 750‰ for 410 ticks (1030-1439). All three needs reached 1000‰. Died at tick 1342 from hunger deprivation.

**Root cause hypothesis**: Kael, Merchant Vara, and Guard Theron are all located at Dusty Trail (e2g0) for most of the simulation (Kael: 1426 ticks, Vara: 1315 ticks, Theron: 874 ticks). Dusty Trail has NO food sources, NO water well, and NO harvest affordances. The place contains only SocialArtifacts, Waste, Coins, and weapons. All AcquireCommodity plans for food (Bread, Apple, Grain) fail with budget-exhausted at 300 expansions with 693-705 candidates at depth 9 — the multi-hop plan (travel to food source → harvest → pick up → consume) explodes the search space beyond the planner's 300-expansion budget. Forager Lina survives because she's at Eldergrove Forest which has OrchardRow (apples) and she never leaves, so her AcquireCommodity plans are simple 1-step plans that always succeed.
**Confidence**: HIGH — this is the dominant pathology driving most other smells. The evidence chain is clear: no local food → budget-exhausted AcquireCommodity → behavioral collapse → starvation.

### 6. Unaddressed Needs -- CRITICAL

**Agent(s)**: Merchant Vara (hunger), Kael (dirtiness), Merchant Vara (dirtiness late-sim), Forager Lina (dirtiness)

**Evidence**:
- **Merchant Vara**: Hunger average 892‰ with ZERO eat actions ever attempted in 1440 ticks. This is mechanically flagged as Anomaly 17. She has no food in inventory, no harvest affordances at Dusty Trail, and all AcquireCommodity food plans budget-exhausted (102 total budget-exhausted searches). She attempted travel to Thornwall Village (9 travel actions) but could only harvest water there, not food.
- **Kael**: Dirtiness above 750‰ for 790 ticks. Kael's final affordances at Dusty Trail include no `wash` action. Dusty Trail has no WashBasin (it's at Hearthstone Inn). The planner never generates a multi-hop wash plan.
- **Forager Lina**: Dirtiness above 750‰ for 810 ticks. Eldergrove Forest has no WashBasin either. Lina never traveled (stayed at e1g0 all 1440 ticks) and never attempted wash.

**Root cause hypothesis**: No wash facilities exist at Dusty Trail or Eldergrove Forest — WashBasin is only at Hearthstone Inn (e3g0). No agent ever traveled to Hearthstone Inn. For hunger, Merchant Vara's eat-action absence is the AcquireCommodity budget-exhaustion spiral: the planner generates food acquisition goals but cannot find plans within the 300-expansion budget because the plan requires cross-location travel.
**Confidence**: HIGH — directly confirmed by affordance analysis and plan search outcomes.

### 7. Impossible Knowledge -- NONE

No evidence of agents acting on information they couldn't have obtained through perception or social channels. All action targets correlate with entities in the agent's perception trace or co-located entities.

### 8. Belief Staleness -- MEDIUM

**Agent(s)**: Kael, Merchant Vara, Guard Theron
**Evidence**: Kael knows only 16 entities (2 agents, 1 place, 2 items, 11 SocialArtifacts). He believes he is at Dusty Trail with Merchant Vara and various SocialArtifacts and Waste — this matches the actual state. However, he has no beliefs about food locations (no knowledge of OrchardRow at Eldergrove, no knowledge of Well at Thornwall Village). Merchant Vara similarly knows only 12 entities with beliefs confined to Dusty Trail contents. Guard Theron knows 3 agents and 1 place but zero items despite having picked up 8 items during the simulation.

The belief summaries show agents are informationally trapped: they know about their immediate surroundings but have almost no knowledge of resource locations at other places. Kael and Merchant Vara each have 1 heard belief; Forager Lina has zero social beliefs (completely isolated).
**Root cause hypothesis**: Information locality (Principle 7) is working as designed — agents only know what they've observed. However, the lack of belief about remote resources prevents the planner from even attempting multi-location plans. The agents traveled early in the simulation but the beliefs from those travels may have been about entities (other agents, places) rather than resource locations. This creates a catch-22: agents can't plan to get food because they don't know where food is, and they can't learn where food is because they don't travel (since they're stuck in the sleep+relieve loop).
**Confidence**: MEDIUM — the belief data confirms informational poverty, but whether this causes the planning failure vs. the planning failure causing the stagnation is hard to disentangle from the budget-exhaustion evidence.

### 9. Social Isolation -- MEDIUM

**Agent(s)**: Forager Lina
**Evidence**: Forager Lina spent all 1440 ticks at Eldergrove Forest alone. She has 0 social observations, 0 told beliefs, 0 heard beliefs, 0 institutional beliefs, and knows 0 other agents. She performed 0 tell, 0 ask_witness, 0 trade actions. Meanwhile, the other three agents are co-located at Dusty Trail for most of the simulation and do engage in social actions (Kael: 16 tell, Merchant Vara: 55 tell, Guard Theron: 57 tell).

However, the social interaction among the Dusty Trail agents is also questionable: despite being co-located for 800+ ticks, Kael and Merchant Vara have no trade actions (Kael has 20 Coins, Vara has nothing). No agent engaged in AskWitness or Trade actions.
**Root cause hypothesis**: Forager Lina lacks travel goals or social goals — her planner is entirely focused on the eat/harvest/sleep cycle, which is successful but isolating. The other agents do communicate (tell), but the communication doesn't lead to useful information exchange about resources. Trade is not attempted despite Kael having 20 Coins and other agents having needs — likely because the scenario doesn't set up market conditions or trade affordances at Dusty Trail.
**Confidence**: HIGH for Forager Lina's isolation. MEDIUM for the broader social stagnation among Dusty Trail agents.

### 10. Economic Stagnation -- CRITICAL

**Agent(s)**: Kael, Merchant Vara, Guard Theron
**Evidence**:
- Kael has 20 Coins but no food. Cannot eat, cannot trade, cannot harvest at Dusty Trail.
- Merchant Vara has an empty inventory. Despite being a "Merchant," she performed 0 trade actions, 0 eat actions, and only 3 harvest:Water actions (all at Thornwall Village during early travels).
- Guard Theron has 1 Bow and 1 Sword but no food. He harvested water 7 times early on but stopped after tick 800 when the post_notice loop consumed all planning capacity.
- Dusty Trail (e2g0) where 3 agents are stranded contains: 487+ SocialArtifacts, 48 Waste items, 20 Coins, 1 Bow, 1 Sword. No food, no water, no production facilities.
- Thornwall Village has Mill, Loom, Well. Eldergrove Forest has OrchardRow, ChoppingBlock, plus 23 Waste. Hearthstone Inn has Forge, WashBasin, Firewood, Medicine. Golden Fields has FieldPlot, GravePlot. Resources exist but are geographically separated from the agents.
- The AcquireCommodity budget-exhaustion spiral prevents any agent from planning a cross-location resource acquisition. 300 expansions at depth 9 with 693-705 candidates means the search tree is too large.

**Root cause hypothesis**: The economy is structurally broken by the interaction between geography and planner budget. Resources are distributed across 5 places, but agents settle at the resource-poorest place (Dusty Trail). The planner's 300-expansion budget is insufficient for the multi-hop plans required to travel, harvest, and consume. Forager Lina's success demonstrates the system works when food is local — the failure is specifically about cross-location resource chains exceeding the planner budget.
**Confidence**: HIGH — this is the root cause of almost all other pathologies in this run.

## Cross-Cutting Patterns

**The AcquireCommodity Budget-Exhaustion Cascade**: This single planner limitation drives a cascading failure across smells 2, 3, 5, 6, 8, and 10. The causal chain:

1. Agents settle at Dusty Trail (resource-poor location)
2. AcquireCommodity plans for food require travel → harvest → pick_up → eat, generating 693+ candidates at depth 9
3. The 300-expansion budget is exhausted every time, so no food plan is ever found
4. Without food plans, agents collapse to sleep + relieve_wilderness loops (smell 2)
5. Needs rise unchecked (smell 5), hunger becomes unaddressed (smell 6)
6. Agents die (Guard Theron at tick 1342) or starve slowly (Kael, Merchant Vara at 1000‰ hunger)

**Guard Theron's PostNotice Pathology**: Separate from the budget-exhaustion cascade, Guard Theron has a compulsive PostNotice loop starting at tick 800. He posts 487 ThreatWarning notices while starving to death. The post_notice action completes in 1 tick and likely has a very high drive score from the guard role's obligations, overwhelming survival needs in goal ranking. This is a goal-ranking failure rather than a planning failure.

**Forager Lina as Control Case**: Lina demonstrates the system works correctly when food is locally available: 64 eat actions, hunger avg 34‰, thirst avg 68‰, consistent harvest/eat/sleep cycle throughout. Her only issue is dirtiness (no WashBasin at Eldergrove Forest) and social isolation (never leaves, never meets other agents). She proves the core eat/harvest/sleep loop is functional — the problem is exclusively about cross-location planning.

**SocialArtifact Pollution**: Dusty Trail accumulated 487+ SocialArtifacts from Guard Theron's post_notice spam. This pollution inflates the perception system (each artifact is observed repeatedly), bloats the place inventory, and may contribute to the affordance explosion that makes the planner's search space worse.

## Planner Diagnostics

| Agent | Plans Found | Frontier Exhausted | Budget Exhausted | Top Failed Goal | Candidate Count (typical) | Max Depth |
|-------|------------|-------------------|-----------------|----------------|--------------------------|-----------|
| Kael | 187 | 25 | 17 | AcquireCommodity (Bread/Apple/Grain) | 693-705 | 9 |
| Merchant Vara | 243 | 22 | 102 | AcquireCommodity (Bread/Apple/Grain) | 693-705 | 9 |
| Forager Lina | 316 | 0 | 0 | (none) | n/a | n/a |
| Guard Theron | 784 | 86 | 17 | ShareBelief | 3 | 0 |

Assessment: Budget exhaustion is **structural** for AcquireCommodity goals. The search space is inherently too large when food requires cross-location travel: 693+ candidates at depth 9 means the branching factor makes it impossible to find a plan within 300 expansions. This is a design-level issue — either the planner budget needs increasing for multi-hop plans, or the search space needs pruning (e.g., heuristic-guided search, or decomposing multi-location goals into sub-goals like "travel to food location" + "harvest food").

## Summary Statistics

- Total findings: 8 (categories with severity other than NONE)
- By severity: 3 CRITICAL, 3 MEDIUM, 1 LOW, 1 NONE
- Agents with issues: Kael, Merchant Vara, Guard Theron
- Clean agents: Forager Lina (only dirtiness issue, which is a missing-facility problem, not a planner failure)

## Trace Quality Assessment

### Trace Sufficiency
The dump provides excellent coverage for all 10 smells. The Section 7 decision summaries with failed plan attempts, affordances, and tick breakdowns directly answer "why didn't the agent do X?" for every pathology.

### Limitations and Recommended Additions

| ID | Limitation | Classification | Rationale |
|----|-----------|----------------|-----------|
| TQ-1 | No per-tick needs trajectory (only min/max/avg and above-750 counts) | Acceptable trade-off | Min/max/avg plus the tick-range data in anomalies is sufficient to identify sustained critical needs and correlate them with behavioral transitions. Per-tick data would be massive and rarely needed. |
| TQ-2 | SocialArtifact entities in belief summaries are opaque (e.g., "SocialArtifact#671") with no indication of their content | Acceptable trade-off | Knowing artifact content could help assess belief quality, but the sheer volume (487+ artifacts) makes this impractical. The key diagnostic insight (perception inflation from artifact pollution) is derivable from counts alone. |
| TQ-3 | No explicit carry capacity tracking — cannot directly confirm inventory-full hypothesis for agents | Actionable | Carry capacity is relevant to understanding why agents don't pick up more items and whether FreeCarryCapacity degenerate loops could occur. Without it, we can only infer from inventory contents. |
| TQ-4 | No goal-ranking scores visible in the dump — cannot determine why PostNotice outranks survival needs for Guard Theron | Actionable | The PostNotice loop is a critical pathology. Seeing the drive scores that led PostNotice to outrank ConsumeOwnedCommodity or AcquireCommodity would directly identify whether the issue is goal weighting, motive scoring, or role obligation priority. |

For **TQ-3**: **Recommended addition**: Add per-agent carry capacity (current/max) to Section 2 agent summaries. **Scope**: Observer-binary enhancement.

For **TQ-4**: **Recommended addition**: When an agent has sustained critical needs AND is selecting a non-survival goal, include a drive-score comparison in the decision timeline (e.g., "PostNotice drive=X vs AcquireCommodity drive=Y"). **Scope**: Observer-binary enhancement (derive from existing decision data).
