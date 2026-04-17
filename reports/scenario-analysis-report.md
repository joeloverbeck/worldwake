# Scenario Analysis Report

## Run Summary
- **Scenario**: `scenarios/survival-scattered.ron`
- **Scenario purpose**: Prove that agents can maintain basic needs under realistic adversity: spatially separated resources, travel metabolism costs, and isolated starting positions. Tier 2 stress-test above survival-baseline.
- **Seed**: 205005
- **Ticks simulated**: 1440
- **Agents**: Agent A (Hilltop Camp), Agent B (Ravine Shelter), Agent C (Woodland Clearing)
- **Places**: Hilltop Camp, Woodland Clearing, Ravine Shelter, River Crossing, Orchard Hollow, Lowland Farm
- **Total events**: 90401
- **Deaths**: None

### Pre-flight Warnings

1. **Ravine Shelter has NO local facilities** — Agent B starts isolated, nearest resources 4+ ticks away through Woodland Clearing. **Confirmed**: Agent B successfully explored out within the first ~30 ticks, spending only 1 tick at Ravine Shelter before finding resources.

2. **Only one wash facility** (Crossing Basin at River Crossing) — all agents must travel there for washing. **Confirmed**: All agents converged on River Crossing as their primary base (718-1044 ticks each), partly driven by wash access. All agents achieved 12-13 wash actions across the run.

3. **Orchard Hollow food never used** — All agents have "Harvest Apples" recipe and Orchard Hollow has an OrchardRow, but zero Harvest Apples actions were executed. All food came from Harvest Grain at Lowland Farm. Agents chose the River Crossing-Lowland Farm corridor exclusively. This is not a failure — Grain and Apples both satisfy hunger — but it means the scenario's food resource diversity was not exercised.

---

## Layer 1: Behavioral Smell Analysis

### 1. Redundant Perception — LOW
**Agent(s)**: All agents (primarily Agent C at River Crossing)
**Evidence**: The last 100 events are dominated by Discovery events — Agent C generated ~30 Discovery events at tick 1438, Agent A generated ~30 at tick 1439, all at River Crossing. With 9 Waste items, 2 facilities, and multiple ItemLots on the ground at River Crossing, each perception tick produces a flood of observations. Agent A: 244 total observations, 212 passed, 56 unique entities. Agent C: 259 observations, 236 passed, 60 unique.
**Root cause hypothesis**: River Crossing accumulated 9 Waste items by end-state plus numerous consumed-item remnants. Each perception cycle re-observes all ground items including Waste. The perception system correctly observes them (they are present), but the volume creates event log bloat. Not pathological per se — observations pass — but Waste entities dominate belief stores (9 Waste in Agent A's River Crossing beliefs, 8 in Agent B's, 9 in Agent C's).

### 2. Action Loops — LOW
**Agent(s)**: Agent A
**Evidence**: Anomaly 1 flagged a sleep-sleep loop (length 2, repeated 3 times). Agent A has 147 sleep actions — the highest of any agent — but this is proportional to 1440 ticks. The behavioral transition at tick 1400 shows repertoire narrowing from 8 types to 1 type, but at tick 1400 the agent's needs are well-managed (hunger=126, thirst=81, fatigue=282) and only 40 ticks remain. The sleep loop at the end of simulation is the agent sleeping while waiting for needs to rise enough to trigger other goals.
**Root cause hypothesis**: End-of-simulation artifact. With all needs low, sleep is the default low-urgency action. Not pathological.

### 3. Stuck Agents — LOW
**Agent(s)**: Agent C
**Evidence**: Anomaly 2 flagged 22 consecutive idle ticks (ticks 63-84). At tick 63, needs were: hunger=160, thirst=40, fatigue=258, bladder=8, dirtiness=378. No needs were critical. Agent C's plan search outcomes show 300 found, 1 frontier-exhausted, 0 budget-exhausted — excellent planner health. The 22-tick idle window likely coincides with early exploration planning before the agent discovered resources.
**Root cause hypothesis**: Early-simulation exploration gap. Agent C started at Woodland Clearing with no local facilities. The idle window at ticks 63-84 likely corresponds to planning and transitioning between exploration goals. Dirtiness at 378 was the highest need but still well below threshold. Not pathological.

### 4. Failed Action Spirals — NONE
**Evidence**: Agent A: 0 StartFailed across all action types. Agent B: 1 ask_witness StartFailed, 1 Harvest Grain StartFailed — trivial counts. Agent C: 3 Harvest Water StartFailed — likely resource depletion at the well (capacity 14, regeneration 4 ticks/unit). No spiral pattern. All agents have >95% action success rates.

### 5. Sustained Critical Needs — NONE
**Evidence**: Zero ticks above 750 permille for any need for any agent. Maximum values: hunger peaked at 492 (Agent A), thirst at 329 (Agent B), fatigue at 350 (Agent A), bladder at 548 (Agent A), dirtiness at 587 (Agent A). All well below critical thresholds.

### 6. Unaddressed Needs — NONE
**Evidence**: Every need type has corresponding relief actions: eat (18-27 per agent), drink (17-27), sleep (147-151), relieve_wilderness (21-26), wash (12-13), toilet (1 each). All needs adequately addressed.

### 7. Impossible Knowledge — NONE
**Evidence**: All agents know only 2 places (River Crossing and Lowland Farm) — the places they actually visited and spent significant time at. Starting locations (Hilltop Camp, Ravine Shelter, Woodland Clearing) were visited briefly (1-16 ticks) during early exploration and apparently dropped from belief stores. No agent acted on locations they hadn't visited. Agent beliefs are consistent with their travel histories.

### 8. Belief Staleness — LOW
**Agent(s)**: All agents
**Evidence**: All agents believe only in River Crossing and Lowland Farm. They have no beliefs about Hilltop Camp (which has a Well), Orchard Hollow (which has an OrchardRow), or Ravine Shelter. These are locations they visited briefly early on. The staleness concern is that agents have "forgotten" about Hilltop Camp's well and Orchard Hollow's food source — but since their current two-base strategy (River Crossing + Lowland Farm) fully satisfies all needs, this staleness is non-impactful.
**Waste belief pollution**: Agent A knows 53 items, Agent B 56 items, Agent C 52 items. Of these, a significant portion are ItemLots and Waste at River Crossing. Belief stores are not at capacity but Waste entities are consuming belief slots that could represent more useful knowledge.

### 9. Social Isolation — NONE (by design)
**Evidence**: Zero social observations, told beliefs, heard beliefs, and institutional beliefs for all agents. All agents have `max_tell_candidates: 0` in their tell profiles — social interaction is explicitly disabled in this scenario. This is not a smell; the scenario intentionally tests survival without social cooperation.

### 10. Economic Stagnation — NONE
**Evidence**: All agents actively harvest (Grain: 9-14 per agent, Water: 15-20 per agent), consume, and travel between resource locations. Agent B even has 2 surplus Grain in inventory at end-state. Resource sources are being utilized effectively: Lowland Farm for Grain, River Crossing's Crossing Well for Water. No agent has unmet needs with available resources.

---

## Layer 2: Needs Diagnostics

*Not triggered — no agent exceeded 750 permille for 100+ consecutive ticks.*

### Agent Needs Overview

| Agent | Closest-to-Threshold Need | Max Value | Margin to 750 | Planner Health |
|-------|--------------------------|-----------|---------------|----------------|
| Agent A | Dirtiness | 587 | 163 | 268 found, 0 frontier-exhausted, 0 budget-exhausted |
| Agent B | Hunger | 576 | 174 | 310 found, 0 frontier-exhausted, 0 budget-exhausted |
| Agent C | Dirtiness | 555 | 195 | 300 found, 1 frontier-exhausted, 0 budget-exhausted |

### Survival Strategy Summary

**Agent A** (started Hilltop Camp): Explored outward through Woodland Clearing to River Crossing and Lowland Farm within the first ~30 ticks. Settled into a River Crossing primary base (949 ticks) with Lowland Farm runs (440 ticks) for Grain. 18 eat, 18 drink, 12 wash, 15 Harvest Water, 9 Harvest Grain, 18 travel. Efficient two-base cycle with all needs well-managed. Highest dirtiness peak (587) and bladder peak (548) among agents, suggesting slightly longer wash/relieve intervals.

**Agent B** (started Ravine Shelter — isolated): Successfully escaped isolation immediately, spending only 1 tick at Ravine Shelter. Explored through Woodland Clearing and Hilltop Camp (14 ticks each) before settling on River Crossing (718 ticks) and Lowland Farm (650 ticks). Most balanced time split between the two bases. Highest action count (640 lifecycle events) and most varied actions. 27 eat, 17 drink, 13 wash, 15 Harvest Water, 14 Harvest Grain, 26 travel. The most active traveler (26 trips). Carried 2 surplus Grain at end-state.

**Agent C** (started Woodland Clearing): Explored to River Crossing and Lowland Farm. Settled most heavily at River Crossing (1044 ticks — 73% of simulation), with shorter Lowland Farm runs (336 ticks). 18 eat, 27 drink, 13 wash, 20 Harvest Water, 9 Harvest Grain, 20 travel. Higher drink and Harvest Water counts reflect Agent C's higher thirst_rate (4 vs 2-3 for others).

### Margins and Risk Observations

- **Dirtiness is the tightest margin** for Agent A (163 permille to threshold) and Agent C (195). With only one wash facility (River Crossing), any disruption to wash access (e.g., facility queueing, extended travel) could push dirtiness into critical territory. In a longer run or with more agents competing for the wash basin, this margin could erode.

- **Hunger margin for Agent B** (174 permille) is the tightest food-related margin. Agent B has the highest hunger_rate (3 vs 2 for others) and the longest average travel distances from Ravine Shelter. The surplus 2 Grain in inventory provides a buffer.

- **Orchard Hollow is completely unused**. All food comes from Lowland Farm (Grain). If Lowland Farm's FieldPlot were disrupted (capacity reduction, travel edge removal), agents would need to discover and use Orchard Hollow — but their current beliefs don't include it. This is a fragility: the agents have found one food source and stopped exploring for alternatives.

- **Waste accumulation**: 9 Waste at River Crossing, 1 at Lowland Farm. In a multi-day run, Waste accumulation would increasingly pollute belief stores and create perception event bloat. Currently manageable (10 total Waste items), but growth is linear with relieve_wilderness actions (~70 total across all agents in 1440 ticks).

---

## Layer 3: Detection Meta-Analysis

### False Positives

| Smell | Agent(s) | Why It's False | Detector Improvement |
|-------|----------|----------------|---------------------|
| ACTION_LOOP (sleep-sleep) | Agent A | End-of-simulation artifact — needs well-managed, only 40 ticks remain, sleep is the correct low-urgency action | Add context: if all needs < 300 permille and remaining ticks < 100, suppress sleep-loop detection |
| STUCK_AGENT (22 idle ticks) | Agent C | Early exploration phase — agent was at Woodland Clearing (no facilities) with no critical needs, transitioning through planning to exploration goals | Consider idle-tick context: if the agent is at a facility-less location AND no need > 500 permille, raise the stuck threshold to 40 ticks to account for exploration planning time |

### Detection Gaps

#### Gap 1: Orchard Hollow Resource Abandonment
**Evidence**: No agent ever performed Harvest Apples despite all having the recipe and Orchard Hollow being 3 ticks from Woodland Clearing (Agent C's start). Agent A explored to Woodland Clearing (slot 1), River Crossing (slot 3), and Lowland Farm (slot 5) per goals selected. No agent selected ExploreLocation targeting Orchard Hollow (slot 4). Orchard Hollow does not appear in any agent's end-state beliefs.
**Agent(s)**: All agents
**Why current detectors miss it**: No smell category covers "reachable resource location never exploited." Smell 10 (Economic Stagnation) checks for unmet needs + available resources, but all needs are met via Grain. The issue is resource diversity, not starvation.
**Impact**: LOW — currently non-impactful since Grain satisfies hunger. However, this represents a fragility: if Lowland Farm were disrupted, agents lack beliefs about alternative food sources. In longer or more complex scenarios, single-source food dependency is a risk.

#### Gap 2: Belief Store Waste Pollution Trajectory
**Evidence**: 9 Waste at River Crossing in agent beliefs. Agent A knows 53 items, of which 9 are Waste at River Crossing (17% of item knowledge). Waste items provide zero utility but consume belief store capacity and generate perception events every tick. Over a multi-day run, this ratio would worsen.
**Agent(s)**: All agents
**Why current detectors miss it**: No detector tracks the ratio of useful-to-useless entities in belief stores. Smell 8 (Belief Staleness) checks belief-reality mismatches, not belief quality. The beliefs are accurate — they correctly reflect Waste presence — but they're low-value knowledge.
**Impact**: LOW (currently) / MEDIUM (in multi-day runs) — 10 Waste items across 1440 ticks is manageable, but the rate (~70 relieve_wilderness actions per 1440 ticks producing Waste) means a 3-day run would produce ~30 Waste at River Crossing, significantly degrading belief store utility.

#### Gap 3: Geographic Convergence
**Evidence**: All 3 agents independently converged on the same two locations (River Crossing: 718-1044 ticks, Lowland Farm: 336-650 ticks). 4 of 6 places are effectively abandoned after early exploration (Hilltop Camp: 14-16 ticks, Woodland Clearing: 1 tick, Ravine Shelter: 0-1 tick, Orchard Hollow: 0 ticks). The scenario designed 6 places but only 2 are used.
**Agent(s)**: All agents
**Why current detectors miss it**: No detector tracks place utilization across agents. Each individual agent's behavior is rational (River Crossing has water+wash, Lowland Farm has grain), but the emergent result is geographic monoculture. Smell 9 (Social Isolation) only checks for social actions, not spatial overlap.
**Impact**: MEDIUM — represents a scenario design signal: the River Crossing + Lowland Farm corridor is so dominant that other locations serve no purpose beyond initial exploration. For scenario designers, this means the spatial adversity was bypassed rather than overcome — agents found the optimal corridor and never needed the other 4 locations.

### Threshold Assessment

| Threshold | Current Value | Assessment | Recommendation |
|-----------|--------------|------------|----------------|
| Stuck agent idle ticks | 20 consecutive ticks | Too low | 30-40 ticks. Agent C's 22-tick idle was a false positive from normal exploration planning. In a 6-place map with 3-6 tick travel times, 20 ticks of idle during exploration is expected. |
| Redundant perception count | 10 observations | Appropriate | No change needed. River Crossing perception generates many events but they pass the observation filter correctly. |
| Critical need threshold | 750 permille | Appropriate | No change needed. Max needs (492-587) are well below threshold, confirming 750 is a reasonable critical line. |
| Sustained critical duration | 100 consecutive ticks | Appropriate | No change needed. No agent approached this threshold. |
| Failed action spiral rate | >75% failure with 5+ attempts | Appropriate | No change needed. Maximum failures were 3 (Agent C Harvest Water), well below detection threshold. |
| Unaddressed need average | 750 permille | Appropriate | No change needed. All needs have corresponding relief actions. |

### Proposed New Smell Categories

#### Proposed Smell 11: Single-Source Resource Dependency

**Detection logic**: For each consumable commodity type (food, water), count how many distinct resource source locations an agent exploits across the run. If all consumption of a commodity comes from a single source AND alternative sources exist in the scenario, flag it.
**Threshold**: 100% of a commodity from a single source over 500+ ticks, with at least one unused alternative reachable within the agent's travel horizon.
**Mechanical vs. LLM**: Mechanical — the observer already tracks harvest actions per facility. Compare harvest-action locations against scenario resource_sources.
**Implementation scope**: Observer binary enhancement — add a "Resource Source Diversity" subsection to Section 2 or Section 6.
**Example from this run**: All 3 agents obtain 100% of food from Lowland Farm (Harvest Grain), while Orchard Hollow (Harvest Apples) is reachable within 6 ticks from their primary base and is never used.
**False positive risk**: Scenarios with only one food source would always trigger. Filter: only flag when 2+ sources for the same consumption need exist. Also, if an agent's known recipes don't cover the alternative source, don't flag (in this case all agents know Harvest Apples, so it's a valid flag).

---

## Cross-Cutting Patterns

1. **River Crossing as universal attractor**: The presence of both water (Well) and wash (WashBasin) at River Crossing makes it the gravity center for all agents. Combined with Lowland Farm's food 3 ticks away, the River Crossing-Lowland Farm corridor becomes the only viable resource cycle. This is rational agent behavior but it reveals that the scenario's spatial complexity (6 places, 7 edges) collapses to a 2-place operational footprint.

2. **Exploration succeeded but didn't diversify**: All agents used ExploreLocation effectively to discover River Crossing and Lowland Farm from their starting positions. Agent B in particular escaped from isolated Ravine Shelter. However, exploration stopped once a viable survival cycle was found — no agent explored Orchard Hollow despite it being adjacent to Woodland Clearing. The exploration profile's `need_activation_threshold` (275-325) means exploration only fires when needs are pressing, and once a solution is found, the drive drops below threshold.

3. **Waste accumulation is the slow-burn risk**: At 10 Waste items in 1440 ticks, this is manageable. But the production rate (~70 relieve_wilderness per 1440 ticks) means longer runs will see significant belief pollution, perception bloat, and potentially pick_up affordance noise (pick_up already appears in final affordances with multiple targets, likely including Waste).

4. **Agent A's behavioral transitions are end-of-cycle artifacts**: The repertoire narrowing at tick 500 (9→4 types) and tick 1400 (8→1 types) correspond to transition points where the agent is in the middle of a resource run or settling into sleep. With needs all well-managed at those ticks, these transitions don't indicate behavioral collapse.

## Summary Statistics
- Layer 1 findings: 3 (categories with severity other than NONE)
- By severity: 0 CRITICAL, 0 HIGH, 0 MEDIUM, 3 LOW
- Layer 2: not triggered (healthy scenario)
- Layer 3: 2 false positives, 3 detection gaps, 1 new smell proposal
- Agents with issues: None (minor observations only)
- Clean agents: Agent A, Agent B, Agent C
- Scenario purpose achieved: **Yes** — all three agents maintain basic needs under spatially separated resources, travel metabolism costs, and isolated starting positions. Agent B successfully escaped isolation. No agent approached critical need levels. The scenario demonstrates Tier 2 survival competence.
