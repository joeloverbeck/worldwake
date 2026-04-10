# Simulation Observer Report

## Run Summary

- **Scenario**: `scenarios/cli-evaluation.ron`
- **Seed**: 7777
- **Ticks simulated**: 1440 (1 simulated day)
- **Total events**: 9484
- **Agents**: Kael (e5g0), Merchant Vara (e6g0), Forager Lina (e7g0), Guard Theron (e8g0)
- **Places**: Thornwall Village (e0g0), Eldergrove Forest (e1g0), Dusty Trail (e2g0), Hearthstone Inn (e3g0), Golden Fields (e4g0)
- **Note**: Guard Theron has 1 dead tick (died at approximately tick 1439 from starvation/dehydration). Observer was previously crashing at tick 189 due to a missing `ProductionOutputOwnershipPolicy` on facilities, which was fixed during this run.

## Findings

### 1. Redundant Perception -- LOW

**Agent(s)**: All agents
**Evidence**: Kael observed itself (e5g0) 112 times, Merchant Vara 105 times; Guard Theron observed itself 112 times. Agents also repeatedly observed co-located agents and places (e.g., Kael observed e6g0 103 times, Theron observed e0g0 63 times).
**Root cause hypothesis**: The perception system fires on every action event at the agent's location, and self-observations and co-located agent observations trigger each time. Given that agents perform sleep actions every ~10 ticks, each sleep generates events that nearby agents perceive. This is architecturally expected — perception fires broadly on events, not selectively on state changes.
**Confidence**: HIGH that this is expected behavior, not a bug. The agents' state (needs, inventory) changes continuously, so these observations may carry genuine new information even if the entity identity is the same.

### 2. Action Loops -- HIGH

**Agent(s)**: Kael, Merchant Vara, Guard Theron
**Evidence**:
- **Kael**: From tick ~500 onward, action repertoire collapsed to exclusively `sleep + relieve_wilderness`. The 0-99 bin shows diverse actions (tell×8, drink×2, eat×2, pick_up×1, travel×1), but by 500-599 it's only `sleep×10, relieve_wilderness×1` — a pattern sustained for 900+ ticks.
- **Merchant Vara**: Mechanical loop detected (sleep→sleep ×3). Similar collapse: diverse actions in 0-99 (tell×18, staff_market×2, eat×1, pick_up×1), but by 900-999 only `sleep×10, relieve_wilderness×1`. Eating stops after tick ~800.
- **Guard Theron**: Action timeline ends at tick 400-499 (sleep×3). After that, 1019 consecutive idle ticks with no actions at all — complete behavioral shutdown.

**Root cause hypothesis**: All three agents migrated to Dusty Trail (e2g0), which has no food, no water, no resource sources, and no production facilities. Once there, the GOAP planner cannot find plans for `AcquireCommodity{Water}` or food acquisition — these plans consistently fail with `budget-exhausted` (e.g., Vara: 300 expansions, max depth 9, 1483 candidates at e0g0; Theron: 224 expansions, max depth 6, 1589 candidates). The plan to acquire water from another location requires a multi-hop chain (travel → find source → harvest/pick_up → consume) that exceeds the planner's expansion budget. With no way to address hunger/thirst, agents fall into survival-mode loops (sleep to reduce fatigue, relieve for bladder) while critical needs spiral.

**Confidence**: HIGH — the evidence clearly shows behavioral collapse correlated with relocation to a resource-poor location and planner budget exhaustion on resource acquisition goals.

### 3. Stuck Agents -- CRITICAL

**Agent(s)**: Guard Theron (1019 consecutive idle ticks), Forager Lina (40 ticks), Kael (34 ticks), Merchant Vara (27 ticks)
**Evidence**:
- **Guard Theron**: 1019 consecutive idle ticks (from ~tick 421 to end of simulation). This is pathological — his needs (hunger avg 915‰, thirst avg 943‰) were critically high but he took no actions. His tick breakdown shows 338 planning + 85 active-action + 1 dead over only 424 decision ticks total (not 1440), meaning the planner itself stopped generating decisions for ~1016 ticks. He died at the end of the simulation.
- **Other agents**: 27-40 tick idle windows are less severe but still indicate periods where the planner found no executable plans.

**Root cause hypothesis**: Guard Theron at Dusty Trail has no affordances for food/water acquisition. His AcquireCommodity{Water} plans consistently budget-exhaust. With no executable goals remaining (patrol/investigation blocked, ShareBelief frontier-exhausted), the planner enters a complete decision stall. The 424 decision ticks vs 1440 simulated ticks suggests the agent stopped being scheduled for decisions after dying or becoming fully incapacitated.

**Confidence**: HIGH for Theron (clearly pathological). MEDIUM for others (shorter idle periods may reflect planner cooldown cycles).

### 4. Failed Action Spirals -- MEDIUM

**Agent(s)**: Forager Lina
**Evidence**: In ticks 1300-1399, Lina has `pick_up×19` but 27 total StartFailed pick_up attempts across the simulation. The raw trace shows repeated failures: "actor e7g0 has insufficient carry capacity for any Apple" at ticks 1363, 1366, 1368, 1370, 1375, 1381, 1388. She has 20× Waste in inventory consuming all carry capacity (carry_capacity not set, so likely default), preventing further pick_ups.
**Root cause hypothesis**: Lina harvests apples, eats them (generating Waste), but never drops waste. Her inventory fills with Waste, then pick_up fails on precondition. The planner keeps generating AcquireCommodity{Apple} → pick_up plans because it doesn't model the carry capacity constraint during planning, only at execution time.
**Confidence**: HIGH — the trace clearly shows repeated capacity failures with 20× Waste in final inventory.

### 5. Sustained Critical Needs -- CRITICAL

**Agent(s)**: All agents
**Evidence**:
- **Guard Theron**: hunger above 750‰ for 1215 ticks (225-1439), thirst above 750‰ for 1290 ticks (150-1439) — nearly the entire simulation
- **Merchant Vara**: thirst above 750‰ for 1257 ticks (183-1439), dirtiness 790 ticks, hunger 265 ticks
- **Kael**: thirst above 750‰ for 915 ticks (525-1439), dirtiness 790 ticks, hunger 674 ticks
- **Forager Lina**: dirtiness above 750‰ for 810 ticks (630-1439) — only dirtiness, as she successfully managed hunger/thirst

**Root cause hypothesis**: Two distinct causes:
1. **Hunger/thirst (Kael, Vara, Theron)**: Agents at Dusty Trail cannot plan resource acquisition across locations. AcquireCommodity{Water} repeatedly budget-exhausts with 1000-2000+ candidates at moderate depths (4-9). The search space branches too widely for the planner budget (224-300 expansions).
2. **Dirtiness (all agents)**: No agent ever performed a `wash` action. Kael had wash(1 target) as an affordance at e0g0 (Thornwall Village) but never used it. At Dusty Trail there is no WashBasin (only at Hearthstone Inn). The planner likely never generates a Wash goal, or the dirtiness_weight (200-500) is too low relative to other pressing needs.

**Confidence**: HIGH

### 6. Unaddressed Needs -- CRITICAL

**Agent(s)**: Merchant Vara (thirst), Guard Theron (hunger + thirst)
**Evidence**:
- **Merchant Vara**: thirst avg 926‰ but **no drink action was ever attempted**. At tick 0 (e0g0), her affordances don't include `drink` — she had no water in inventory and no pick_up affordance for water at the village. AcquireCommodity{Water} failed 16+ times with budget-exhausted.
- **Guard Theron**: hunger avg 915‰ and thirst avg 943‰, **no eat or drink actions ever attempted**. At tick 0 (e2g0), his affordances include only: sleep, relieve_wilderness, staff_market, declare_support, travel, put_down, attack, defend, patrol, fine, exile — no eat, no drink, no pick_up for consumables.

**Root cause hypothesis**: These agents were never in possession of food/water items, and the locations they occupied (Dusty Trail) had no consumable items on the ground. The `drink` and `eat` actions require owning the consumable. The planner's AcquireCommodity goal was the only path to obtaining resources, but it consistently budget-exhausted. Guard Theron's failed plans show AcquireCommodity{Water} at depths 4-6 with 800-2000 candidates — the multi-location acquisition plan is simply too complex for the expansion budget.

**Confidence**: HIGH — the affordance data and failed plan data clearly explain the gap.

### 7. Impossible Knowledge -- NONE

No evidence of agents acting on information they never observed. Action targets are consistent with perception traces. Kael's believed locations match entities observed at Dusty Trail. Forager Lina's actions target entities at her location (Eldergrove Forest).

### 8. Belief Staleness -- MEDIUM

**Agent(s)**: Kael, Merchant Vara, Forager Lina
**Evidence**:
- All agents at Dusty Trail believe the location of Dusty Trail is "Unknown location" — a strange belief given they are physically present there.
- Kael believes Forager Lina is at Dusty Trail, but Lina spent most of the simulation at Eldergrove Forest (1389 of 1440 ticks) and only arrived at Dusty Trail at tick ~1389. This could be current at end-of-sim.
- Guard Theron knows 3 agents and 1 place but his believed locations show himself at Dusty Trail, which is correct. However, he believes 12× Waste are at Dusty Trail — this is current (57× Waste at Dusty Trail in Section 6, though the belief count is lower, suggesting stale partial knowledge).
- Forager Lina believes 8× Waste at Dusty Trail but the actual count is 57×.

**Root cause hypothesis**: Belief snapshots are captured at perception time with finite memory capacity (16 entities). As waste accumulates (57× at Dusty Trail by end-state), agents' beliefs about item counts become stale since they can only track a limited number of entities. The "Unknown location: Dusty Trail" belief likely means the place entity itself was observed but its location (being a place, it doesn't have a location in the same sense) is classified as unknown.
**Confidence**: MEDIUM — the "Unknown location" pattern may be an artifact of how place-entity locations are represented in beliefs rather than genuine staleness.

### 9. Social Isolation -- MEDIUM

**Agent(s)**: All agents (partially)
**Evidence**:
- Kael and Merchant Vara are co-located at Dusty Trail for ~1370 ticks. They performed tell actions early (Kael: 24 started/19 committed, Vara: 20 started/15 committed) but all social activity ceased after tick ~400. From tick 500 onward, both agents are co-located for 900+ ticks with zero social interaction.
- Guard Theron performed 9 tell actions (all committed) and 27 investigate starts (4 committed), but all activity ceased by tick ~420. He was co-located with Kael/Vara at Dusty Trail for the remainder with no interaction.
- Forager Lina had zero social observations, zero told beliefs, zero heard beliefs throughout the entire simulation. She was alone at Eldergrove Forest for most of the run.
- No Trade actions occurred anywhere in the simulation. Merchant Vara attempted `staff_market` 5 times (all StartFailed) and had SellCommodity{Grain} blocked 698 times.

**Root cause hypothesis**: Social actions require the planner to find plans, and once survival needs dominate (hunger/thirst critical), the planner prioritizes physiological goals. Since those goals can't be satisfied either (budget-exhausted), agents enter a planning deadlock where no goals succeed. ShareBelief goals are consistently frontier-exhausted (1 expansion, 0 depth, meaning no operators are available for the ShareBelief goal). Merchant Vara's trade was blocked because SellCommodity{Grain} was fully blocked 698 times — likely because no buyer was present or the market wasn't staffed.
**Confidence**: HIGH for the late-simulation isolation. The early social activity shows the system works when agents have capacity.

### 10. Economic Stagnation -- CRITICAL

**Agent(s)**: Kael, Merchant Vara, Guard Theron
**Evidence**:
- **No trade occurred** in 1440 ticks. Merchant Vara's SellCommodity{Grain} was blocked 698 times. Her staff_market action failed 5 times (all StartFailed).
- **Kael** holds 20× Coin at end-state but never traded. Water was available at Thornwall Village (Well facility, regeneration 3 ticks/unit, capacity 15) but Kael left the village by tick ~67 and never returned.
- **Guard Theron** never attempted any economic action. He carried Sword and Bow but couldn't eat or drink.
- **Forager Lina** was the only economically active agent: 24 harvests, 55 eats, 5 drinks, 46 pick_ups — a functioning harvest→eat loop. However, she never traded and accumulated 20× Waste with no way to dispose of it.
- **Thornwall Village** resources (10× Grain, 5× Bread) were left untouched at end-state. The Village Well resource source was never harvested (water available at e0g0 not visible in end-state items — only facilities remain).
- **Dusty Trail** accumulated 57× Waste — the most item-rich location by end-state, but all waste.

**Root cause hypothesis**: The fundamental economic failure has three layers:
1. **Geographic mismatch**: 3 of 4 agents migrated to Dusty Trail (resource-poor) while resources remain at Thornwall Village and Eldergrove Forest.
2. **Planner budget exhaustion**: Multi-location economic plans (travel → acquire → consume or trade) exceed the GOAP planner's expansion budget. AcquireCommodity goals generate 1000-7000+ candidates but the budget is only 224-300 expansions.
3. **Missing waste disposal**: No `drop` or waste disposal action appears in affordances, so agents accumulate waste until carry capacity is full (Forager Lina's 27 failed pick_ups).

**Confidence**: HIGH — the end-state inventory and failed plan data clearly show the economic system is non-functional for multi-location resource chains.

## Cross-Cutting Patterns

### Pattern 1: Dusty Trail Death Trap
Three of four agents (Kael, Vara, Theron) migrated to Dusty Trail early in the simulation and became trapped there. Dusty Trail has no food, no water, no production facilities, and no resource sources. Once there, agents' survival needs escalated while the planner could not find resource acquisition plans within its budget. This created a cascading failure: critical needs → planning deadlock → behavioral collapse → starvation/dehydration. Guard Theron died; Kael and Vara were in slow decline at simulation end.

The migration itself was purposeful — Kael traveled there, Vara traveled there (both starting at Thornwall Village which has resources). The planner likely chose travel because ShareBelief or other social goals pointed toward agents at Dusty Trail (Theron's patrol route). But no return-journey plan was generated once the agents arrived and their needs became critical.

### Pattern 2: Planner Budget vs. Multi-Location Plans
The core systemic issue is that resource acquisition across locations requires plan chains too deep/wide for the current planner budget. AcquireCommodity{Water} at Dusty Trail generates 1000-7000+ candidates (reflecting all possible multi-hop paths to water sources) but the budget caps at 224-300 expansions. This affects every agent except Forager Lina (who was already at a resource-rich location).

### Pattern 3: Forager Lina as Control Case
Forager Lina demonstrates the system works correctly when an agent is at a resource-rich location. She maintained a functional harvest→pick_up→eat loop for the entire simulation, kept hunger at avg 62‰ and thirst at avg 141‰. Her only issues were: dirtiness (no WashBasin at Eldergrove Forest), carry capacity exhaustion (waste accumulation), and eventually she traveled to Dusty Trail at tick 1389 — potentially the beginning of the same death spiral that trapped the others.

### Pattern 4: Guard Theron Death
Guard Theron died at approximately tick 1439 (1 dead tick). Contributing factors:
- Started at Dusty Trail with no food/water items
- No eat/drink affordances at Dusty Trail
- AcquireCommodity{Water} budget-exhausted 12+ times (depths 4-6, 800-2000 candidates)
- Patrol route (Trail↔Village) should have brought him to resources, but patrol completed at tick ~6, then he traveled to Village and back, and after tick ~421 became completely idle
- Hunger above 750‰ for 1215 ticks, thirst above 750‰ for 1290 ticks

### Pattern 5: Waste Economy
57× Waste accumulated at Dusty Trail, 1× at Eldergrove Forest. Agents have no waste disposal mechanism. Forager Lina's carry capacity filled entirely with 20× Waste, blocking further pick_ups. This suggests a missing game mechanic (drop/discard action) or a missing planner goal (dispose of waste).

## Summary Statistics

- Total findings: 8 (categories with severity other than NONE)
- By severity: 3 CRITICAL, 2 HIGH, 2 MEDIUM, 1 LOW
- Agents with issues: Kael (action loops, stuck, sustained needs, belief staleness, social isolation, economic stagnation), Merchant Vara (action loops, stuck, sustained needs, unaddressed needs, social isolation, economic stagnation), Guard Theron (stuck, sustained needs, unaddressed needs, social isolation, economic stagnation, death), Forager Lina (failed action spirals, sustained dirtiness, social isolation)
- Clean agents: None

## Trace Quality Assessment

### Trace Sufficiency
The dump provides strong quantitative data for all 10 smell categories. Section 7 decision summaries are particularly valuable for diagnosing planner failures. The main limitation is the absence of per-tick need trajectories (only min/max/avg and ticks-above-750 are available), which would help pinpoint exactly when behavioral transitions occur.

### Limitations and Recommended Additions

| ID | Limitation | Classification | Rationale |
|----|-----------|----------------|-----------|
| TQ-1 | No per-agent need values at key transition points (e.g., when action repertoire narrows) | Actionable | Would enable precise correlation between need levels and behavioral collapse (smell 2). Currently we can infer timing from action timelines but can't confirm the need values at transition points |
| TQ-2 | Affordances shown only at tick 0, not at current location after travel | Actionable | Agents that traveled (all 4) have different affordances than shown. This affected confidence in smells 6 and 8 — we can't confirm whether eat/drink affordances exist at Dusty Trail without checking the current state |
| TQ-3 | No explicit death tick reported in Section 2 or 7 | Actionable | Guard Theron has 1 dead tick but the exact death tick isn't stated. We must infer it from the 424 decision ticks vs 1440 simulated ticks (dies around tick 424 + some offset). This affected the death analysis in Cross-Cutting Pattern 4 |
| TQ-4 | Waste item type not distinguished in belief summary counts | Acceptable trade-off | Beliefs show "12× Waste" but don't distinguish Waste from useful items. This is a minor clarity issue — we can cross-reference with Section 6 |
| TQ-5 | No planner budget configuration shown in dump | Acceptable trade-off | We can infer expansion budgets from the failed plan data (224-300) but having the configured values would confirm whether agents have different budgets |
| TQ-6 | ShareBelief frontier-exhausted plans show 1 expansion, 0 depth — no explanation of why | Actionable | 14 of 20 failed plans for Kael are frontier-exhausted at depth 0, meaning no operators were available. The dump doesn't explain why the tell action operator wasn't applicable (is the listener not co-located? is there a cooldown?). This data would improve diagnosis of social isolation (smell 9) |

For **Actionable** items:

**TQ-1**: Add a per-agent needs snapshot at detected behavioral transition points (when action type count drops by 50%+ between consecutive bins).
- **Scope**: Observer-binary enhancement

**TQ-2**: Capture affordances at end-of-simulation (or at the agent's current location) in addition to tick 0.
- **Scope**: Observer-binary enhancement

**TQ-3**: Report the exact death tick in the per-agent summary and Section 7 header.
- **Scope**: Observer-binary enhancement

**TQ-6**: Include the rejection reason for frontier-exhausted plans (e.g., "no applicable operators: tell requires co-located listener" or "cooldown active").
- **Scope**: Engine instrumentation (planner would need to record why no operators matched)
