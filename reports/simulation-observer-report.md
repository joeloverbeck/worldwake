# Simulation Observer Report

## Run Summary

- **Scenario**: `scenarios/cli-evaluation.ron`
- **Seed**: 7777
- **Ticks simulated**: 1440 (1 simulated day)
- **Total events**: 9009
- **Agents**: 4 (Kael [Human], Merchant Vara [AI], Forager Lina [AI], Guard Theron [AI])
- **Places**: 5 (Thornwall Village, Eldergrove Forest, Dusty Trail, Hearthstone Inn, Golden Fields)

## Findings

### 1. Ignored Urgent Needs: Universal Dehydration -- CRITICAL

**Agent(s)**: Merchant Vara, Forager Lina, Guard Theron (all AI agents)

**Evidence**: No agent executes a `drink` action across all 1440 ticks. Thirst averages:
- Merchant Vara: avg 926, max 1000
- Forager Lina: avg 981, max 1000
- Guard Theron: avg 943, max 1000

All three agents hit the thirst ceiling (1000) and stay pinned there for most of the simulation. Despite this, they perform unrelated actions (sleep, tell, patrol, eat).

**Root cause hypothesis**: Two possible causes, likely compounding:
1. **No water sources in the scenario**: Water items exist only on Kael (Human, inactive). No water resource source exists at any place. Once Kael's water is inaccessible (human-controlled, never acts), there is zero drinkable water in the simulation.
2. **Drink affordance generation**: Even if water were available, the planner may not be generating `drink` as a candidate action. The action traces show no `drink` attempts (not even failed ones), suggesting either no affordance is generated or the planner never considers it viable.

**Confidence**: HIGH -- the data is unambiguous. Every AI agent is critically dehydrated for 90%+ of the simulation while performing lower-priority actions.

---

### 2. Stuck Agent: Guard Theron -- HIGH

**Agent(s)**: Guard Theron

**Evidence**: 1024 consecutive idle ticks (71% of the simulation). Only 117 total action lifecycle events across 1440 ticks. Action breakdown: sleep(20), tell(18 committed / 24 failed), eat(3), patrol(1), pick_up(1), relieve_wilderness(2), travel(2). After early activity (patrol + tell burst in the first ~400 ticks), Theron becomes effectively inert on Dusty Trail.

**Root cause hypothesis**: Theron completes 1 patrol and 2 travels, arriving at Dusty Trail where he stays for 1326 ticks. With thirst at 943 avg, the planner may be stuck: the highest-priority need (thirst) has no viable plan (no water), and the planner fails to fall back to lower-priority actions. The 24 failed `tell` attempts suggest he periodically tries social actions but they fail (possibly no valid recipients or no new information to share). After exhausting tell candidates, he enters a long idle state.

**Confidence**: HIGH -- an AI guard with patrol duties being idle for 1024 straight ticks is pathological, not explainable as legitimate rest or low-priority waiting.

---

### 3. Economic Stagnation -- HIGH

**Agent(s)**: Merchant Vara, Forager Lina

**Evidence**:
- **No trade actions** by any agent across the entire simulation.
- **Merchant Vara**: `staff_market` failed 6 times (StartFailed), 0 successful. She is a merchant with a MerchandiseProfile selling Grain, Apple, Bread, but never successfully staffs her market. No craft actions attempted either.
- **Forager Lina**: Located at Eldergrove Forest (apple resource source, capacity 20) but only eats 8 times and picks up 1 item. No `harvest` actions in her trace.
- **No craft actions** by any agent -- the Mill (Thornwall Village) and Forge (Hearthstone Inn) are never used.

The economy is completely inert: no harvesting, no crafting, no trading, no market staffing.

**Root cause hypothesis**: Multiple compounding issues:
1. Vara's `staff_market` StartFailed 6 times -- the precondition is failing (possibly needs to be at her `home_facility` which is set to "Thornwall Village", but she's on Dusty Trail for 1314 ticks).
2. Without a staffed market, trade cannot happen.
3. Lina has no `harvest` in her action list -- possibly no harvest affordance is being generated, or the planner doesn't consider it.
4. Extreme thirst (all agents pinned at 1000) may be consuming all planning bandwidth, leaving no room for enterprise/economic goals.

**Confidence**: HIGH -- zero economic activity in 1440 ticks is definitively stagnant.

---

### 4. Failed Action Spirals -- MEDIUM

**Agent(s)**: Merchant Vara, Guard Theron

**Evidence**:
- **Guard Theron**: `tell` 24 StartFailed vs 18 committed (57% failure rate). He repeatedly attempts tells that fail validation.
- **Merchant Vara**: `staff_market` 6 StartFailed, 0 committed (100% failure rate). `tell` 18 StartFailed vs 12 committed (60% failure rate).

**Root cause hypothesis**:
- `tell` failures: Likely no valid tell recipient co-located, or no new information to share. Once an agent has shared all known observations with co-located agents, further tell attempts fail. The planner may be re-generating `tell` goals without checking whether there's novel information to share.
- `staff_market` failures: Vara is at Dusty Trail (1314 ticks) but her home_facility is Thornwall Village. The `staff_market` precondition likely requires being at the home facility. The planner generates the goal (enterprise_weight: 800) but can't satisfy the co-location precondition. This explains both the market failure AND why Vara doesn't return to the village -- the planner may not chain travel + staff_market.

**Confidence**: MEDIUM -- the failure counts are clear, but root cause requires deeper planner trace inspection.

---

### 5. Action Loops: Sleep Dominance -- MEDIUM

**Agent(s)**: Merchant Vara, Forager Lina

**Evidence**: Anomaly 9 flags Vara with sleep->sleep loop repeated 3 times. Looking at the late-game trace (ticks 1300-1440), both Vara (e6g0) and Lina (e7g0) alternate almost exclusively between `sleep` and `relieve_wilderness`. Vara: 122 sleep actions. Lina: 75 sleep actions. These dominate their action budgets.

**Root cause hypothesis**: With thirst unsatisfiable and fatigue being the only addressable high-priority need, the planner correctly chooses sleep as the best available action. This is rational given the constraints, but the result is agents spending most of their time sleeping because they can't address their actual highest need (thirst). The loop is a symptom, not the disease -- the disease is the missing drink affordance / water supply.

**Confidence**: HIGH that the loop exists, MEDIUM that it's a distinct issue rather than a symptom of dehydration.

---

### 6. Redundant Perception -- LOW

**Agent(s)**: All agents

**Evidence**: 10 redundant perception anomalies across all 4 agents. Examples:
- Kael observes Vara (e6g0) 26 times and Theron (e8g0) 26 times
- Vara observes herself (e6g0) 59 times and Theron (e8g0) 47 times
- Theron observes Vara (e6g0) 47 times and himself (e8g0) 54 times

**Root cause hypothesis**: The perception system fires each tick for co-located entities, producing observation events even when the observed entity's state hasn't meaningfully changed. This is architecturally correct (agents should re-observe to catch changes), but in a low-activity simulation where most agents are idle or sleeping, it generates many observations of static state. The perception system doesn't filter for "no change since last observation."

**Confidence**: LOW severity -- this is expected behavior in a co-located idle scenario. The observations aren't causing harm, just noise. In a more active simulation with more agents, this could become a performance concern.

---

### 7. Stuck Agent: Kael -- NONE (Expected)

**Agent(s)**: Kael

**Evidence**: 1440 consecutive idle ticks, 0 actions, all needs at ceiling.

**Root cause hypothesis**: Kael is `ControlSource::Human` with no input. This is expected and explicitly called out in the skill as not a bug. His needs climb to 1000 across the board because he never acts.

**Confidence**: N/A -- expected behavior per design.

---

### 8. Social Isolation -- LOW

**Agent(s)**: Merchant Vara, Forager Lina

**Evidence**: In the late simulation (ticks 900+), Vara and Lina are both on Dusty Trail for extended periods. The trace shows no tell, trade, or social actions between them after tick ~400. They co-exist at the same location performing only sleep and relieve_wilderness.

**Root cause hypothesis**: Both agents have exhausted their tellable information (Vara's tell fails after early successes), and Lina has no TellProfile configured, so she cannot initiate tells. Without novel information to share or trade goods to exchange, co-location produces no social interaction. Lina also has no `communication_profile`, so she may not accept tells either.

**Confidence**: LOW -- some social isolation is expected when agents have nothing new to communicate. The early simulation shows healthy social interaction (Vara and Theron both tell actively in ticks 1-30).

---

### 9. Impossible Knowledge -- NONE

**Evidence**: Cross-referencing action traces with perception traces, no agent appears to act on information about entities they never observed. Guard Theron observes entities at Thornwall Village after traveling there (tick 8-9), then acts on them. Vara perceives at her locations. No evidence of omniscient behavior.

**Confidence**: HIGH -- no violations detected in the available traces.

---

### 10. Belief Staleness -- INCONCLUSIVE

**Evidence**: The dump does not include per-agent belief snapshots, making it impossible to compare agent beliefs against ground truth at specific decision points. Vara's persistence on Dusty Trail despite her home_facility being at Thornwall Village *could* indicate stale beliefs about facility location, but more likely reflects a planning limitation (not chaining travel + staff_market).

**Confidence**: LOW -- insufficient trace data to assess.

## Cross-Cutting Patterns

1. **Dehydration cascades into everything**: The universal thirst crisis (all AI agents pinned at 1000 permille) is the root cause of most other findings. Sleep dominance, economic stagnation, and even stuck behavior are downstream effects of agents being unable to satisfy their highest-priority need. Fix the water supply and many findings may resolve.

2. **Dusty Trail convergence**: By mid-simulation, 3 of 4 agents (Vara, Lina, Theron) are on Dusty Trail. Vara started at Thornwall Village but traveled there. Lina traveled between Forest and Trail. Theron patrols Trail<->Village but settles on Trail. This leaves Thornwall Village (the economic hub with Mill, Store, market) nearly empty, which compounds the economic stagnation.

3. **Early activity, late stasis**: The simulation shows healthy diverse behavior in ticks 0-200 (patrol, tell, travel, pick_up, eat) that degrades into sleep-only by tick 400+. This suggests initial conditions provide enough momentum for activity, but systemic deficiencies (no water, no economic circulation) cause the simulation to run down.

4. **Tell failure correlation**: Both Vara (18 failed) and Theron (24 failed) have high tell failure rates. Since tells require a valid recipient with novel information, this suggests the small agent count (4 agents, 1 inactive) exhausts social interaction opportunities quickly.

## Summary Statistics

- **Total findings**: 8 assessed (excluding Kael expected-idle)
- **By severity**: 1 CRITICAL, 2 HIGH, 2 MEDIUM, 2 LOW, 1 NONE
- **Agents with issues**: Merchant Vara (dehydration, economic stagnation, sleep loops, tell failures), Forager Lina (dehydration, economic stagnation, sleep loops, social isolation), Guard Theron (dehydration, stuck 1024 ticks, tell failures)
- **Clean agents**: Kael (expected idle -- Human with no input)

## Trace Quality Assessment

The dump provides strong evidence for mechanically-flagged smells (perception, loops, stuck, failed actions) and adequate evidence for needs analysis and economic stagnation. However:

- **Missing**: Per-agent belief state snapshots at decision points would enable confident assessment of belief staleness (smell 7) and impossible knowledge (smell 5). Currently these rely on cross-referencing action targets with perception traces, which is indirect.
- **Missing**: Planner decision traces (which goals were considered, which plans were searched, why plans failed) would greatly improve root-cause analysis for stuck agents and economic stagnation. Currently we can see *what* happened but not *why the planner chose it*.
- **Missing**: Affordance generation traces would clarify whether drink/harvest/craft are being generated as candidates at all, vs. being generated but failing plan search.
- **Adequate**: The action trace, perception trace, needs trajectory, and location tracking provide a solid behavioral picture for the 9 smell categories. The 100-event samples at start and end are sufficient for pattern detection.
