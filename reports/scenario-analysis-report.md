# Scenario Analysis Report

## Run Summary
- **Scenario**: `scenarios/survival-contested.ron`
- **Scenario purpose**: Prove that 4 AI agents can survive 1440 ticks under tight resource contention, dynamic depletion, and chokepoint topology — using only the profile set already exercised by `survival-baseline.ron` and `survival-scattered.ron`. Tier 3 stress-test above `survival-scattered`.
- **Seed**: 306006
- **Ticks simulated**: 1440
- **Agents**: Agent A (North Camp), Agent B (North Camp), Agent C (South Camp), Agent D (South Camp)
- **Places**: North Camp, South Camp, Forest Glade, Central Crossing, Stone Well, Spring Basin, East Orchard, West Grainfield
- **Total events**: 128969
- **Deaths**: None

### Pre-flight Warnings

1. **Social interaction disabled by design** — `tell_profile.max_tell_candidates = 0` for all four agents. Smell 9 (Social Isolation) is expected and will not be reported as a defect.
2. **Single wash facility** — only Spring Washbasin at Spring Basin. All 4 agents must traverse to South-side to wash. **Confirmed**: dominant failure mode of this run.
3. **Water contention** — 4 agents sharing 2 Wells (capacity 4 each, 8- and 10-tick regen). **Confirmed mild**: agent C hit thirst=900 briefly but overall drink counts (A=3, B=4, C=9, D=4) suggest wells kept up.
4. **Latrines co-located only with Camps** — North/South Camp have `Latrine` tag but agents abandon camps within first 3 ticks (1 tick each). **Confirmed**: agents substituted `relieve_wilderness` everywhere else (22-27 invocations each), which carries a 200 permille dirtiness penalty per use.

---

## Layer 1: Behavioral Smell Analysis

### 1. Redundant Perception — MEDIUM
**Agent(s)**: All four agents (severe at East Orchard)
**Evidence**: Last 50 perception events show ticks 1372-1433 dominated by all 4 agents observing the same events simultaneously at East Orchard (fidelity 900 permille). Belief stores show 40+ ItemLot entries and 10× Waste at East Orchard for each agent (Section 5). Agent A beliefs: 52 items out of 60 known entities; Agent D: 57/68.
**Root cause hypothesis**: End-state has 3 agents + OrchardRow + 3 Apple items + 10 Waste items at East Orchard. Each perception tick re-observes this dense cluster. Combined with `max_snapshot_entities_per_place=50` and `institutional_memory_capacity=16`, the belief store becomes dominated by Waste/ItemLot entries and spaces out legitimate resource-location beliefs (e.g., Agent A and C never retain beliefs about Spring Basin despite visiting it for 57 and 1 ticks respectively).

### 2. Action Loops — LOW
**Agent(s)**: All four (behavioral narrowing flagged for B, C, D)
**Evidence**: Section 2 flags behavioral transitions — Agent B tick 1400 (8->4 types), Agent C tick 100 (7->2) and tick 300 (6->2), Agent D tick 700 (9->3). However, none correspond to degenerate plan loops: `GoalSatisfied[steps=0]` never appears. Agent C tick 100 transition is the 26-tick travel+wash trip (see Stuck Agent below). Agent D tick 700 narrowing to 3 types (eat/sleep/relieve_wilderness) lasts ~100 ticks, then broadens again. Agent B tick 1400 is end-of-simulation artifact (needs well-managed).
**Root cause hypothesis**: Narrowing reflects end-of-travel/resource-run cycles, not pathological looping. Plan search health is uniformly high (255-282 plans found per agent, 0-7 frontier-exhausted, 0 budget-exhausted).

### 3. Stuck Agents — LOW
**Agent(s)**: Agent C (tick 59-84, 26 consecutive idle ticks)
**Evidence**: Section 3 anomaly 3 — needs at start: hunger=532, thirst=76. Section 2 location history shows Agent C at Spring Basin (e5g0) for exactly 1 tick — this window is the cross-map trek Spring Basin -> Central Crossing -> East Orchard. Agent C had 11 wash-active ticks in bin 0-99, consistent with the early wash+travel cycle. No action timeline gap in Section 7.
**Root cause hypothesis**: Not pathological — this is a long multi-tick wash action (wash_ticks=12) followed by travel. The stuck-agent detector counts idle ticks but multi-tick actions register as active. Apparent "26 consecutive idle" is an artifact of the window straddling travel legs that are recorded as ActionCommitted events rather than sustained active frames.

### 4. Failed Action Spirals — NONE
**Agent(s)**: None
**Evidence**: Aborted=0 for every action across all agents. StartFailed counts are low (≤2 per agent per action type), consistent with brief resource contention (e.g., Harvest Apples StartFailed=2 for B/C/D corresponds to the capacity-6 orchard briefly empty when 2-3 agents queue).

### 5. Sustained Critical Needs — HIGH
**Agent(s)**: All four (dirtiness); Agent C transiently (hunger, thirst, bladder — but below 100-tick threshold)
**Evidence**:
- Agent A: dirtiness >750 for **901 ticks** (anomaly ticks 687-1005 alone is 319 consecutive)
- Agent B: dirtiness >750 for **849 ticks** (anomaly ticks 1064-1378 = 315 consecutive)
- Agent C: dirtiness >750 for **703 ticks** (anomaly ticks 584-896 = 313 consecutive); hunger 97 ticks >750 (max 950); thirst 37 ticks (max 900); bladder 10 ticks (max 904)
- Agent D: dirtiness >750 for **842 ticks** (anomaly ticks 292-663 = 372 consecutive)

Every agent reached `max dirtiness = 1000 permille`. Wash action counts are low (A=3, B=4, C=5, D=4) vs. relieve_wilderness counts (A=22, B=22, C=27, D=22). At +200 permille per wilderness relief, agents accumulate dirt far faster than wash cycles can remove it.

**Root cause hypothesis**: **Priority Override** primary, **Geographic Desert** secondary. Wash goal is generated (listed in Section 7 "Goals selected" for all 4 agents) but consistently outranked in motive scoring: `dirtiness_weight=625 < hunger_weight=700-750`. When at East Orchard, any hunger pressure (≥ ~50 permille) produces an AcquireCommodity(Apple) motive score that dominates Wash. See Cross-Cutting Patterns for the wash-starvation cycle.

### 6. Unaddressed Needs — NONE
**Agent(s)**: None
**Evidence**: Wash action was attempted (3-5 times per agent). Relieve, Sleep, Eat, Drink all appear in final affordances for 3 of the 4 agents. The issue is insufficient frequency, not absence.

### 7. Impossible Knowledge — NONE
**Agent(s)**: None
**Evidence**: All belief entries (Section 5) correspond to entities the agent co-located with at some point per Section 2 location history. Social observations = 0 for all (tell disabled).

### 8. Belief Staleness — MEDIUM
**Agent(s)**: Agents A and C particularly
**Evidence**: Section 5 shows Agent A's believed places = {South Camp, Central Crossing, East Orchard} — missing North Camp (starting location), Stone Well (45 ticks visited), Spring Basin (57 ticks visited), West Grainfield (2 ticks), Forest Glade (1 tick). Agent C knows {Central Crossing, Stone Well, East Orchard} but not Spring Basin despite visiting it at tick ~60-84. Meanwhile, East Orchard belief entry contains 40+ ItemLot references and 10× Waste.
**Root cause hypothesis**: Belief memory capacity pressure. With `institutional_memory_capacity=16` and `max_snapshot_entities_per_place=50`, the high-entity-count East Orchard (3 agents + OrchardRow + 3 Apple + 10 Waste + 40+ consumed ItemLots) crowds out landmark beliefs about remote locations. This compounds the dirtiness problem: even if planning wants to go wash, agents whose Spring Basin belief has decayed treat the Wash goal as "unknown target" and the plan reverts to local options.

### 9. Social Isolation — EXPECTED (by scenario design)
**Agent(s)**: All four
**Evidence**: Section 5 — Social observations / Told / Heard beliefs all = 0 for every agent. Scenario pre-flight noted `tell_profile.max_tell_candidates = 0`. Not reported as a defect.

### 10. Economic Stagnation — NONE
**Agent(s)**: None
**Evidence**: Agents harvested 16 Apples each (total 64, at capacity 6 orchard = ~10 full orchard cycles over 1440 ticks). Water harvests: A=3, B=4, C=7, D=4. Agents are economically active; the contention is on wash access, not food/water production.

---

## Layer 2: Needs Diagnostics

### Agent Needs Overview

| Agent | Need | Max Value | Ticks >750 permille | Death? | Root Cause Category |
|-------|------|-----------|---------------------|--------|---------------------|
| Agent A | Dirtiness | 1000 | 901 | No | Priority Override + Belief Memory Pollution |
| Agent B | Dirtiness | 1000 | 849 | No | Priority Override + Belief Memory Pollution |
| Agent C | Dirtiness | 1000 | 703 | No | Priority Override + Belief Memory Pollution |
| Agent C | Hunger | 950 | 97 (sub-threshold) | No | Resource Contention (not classified — below 100-tick bar) |
| Agent C | Thirst | 900 | 37 (sub-threshold) | No | Travel lag during water trips (sub-threshold) |
| Agent C | Bladder | 904 | 10 (sub-threshold) | No | Transient bladder spike (sub-threshold) |
| Agent D | Dirtiness | 1000 | 842 | No | Priority Override + Belief Memory Pollution |

### Failure Classifications

#### Agent A
**Categories**: Priority Override (primary), Belief Memory Pollution (secondary)
**Evidence**: Dirtiness 901/1440 ticks >750. Wash goal appears in "Goals selected" but only 3 wash actions committed. Agent A believed places = {South Camp, Central Crossing, East Orchard} only — lost belief about Spring Basin (visited 57 ticks). In Section 7 tick 200-299 PLAN entry, AcquireCommodity(Apple)@e6g0 ranked Feasibility>Water@e4g0 because thirst score was 140000+ while hunger was ~78 pressure. Wash never appears as "selected=" in sampled plans.
**Confidence**: HIGH
**Causal chain**: Agent abandons North Camp (tick 1) -> travels to food hub East Orchard -> needs food frequently -> never gets ahead of hunger enough to let Wash outrank AcquireCommodity -> dirtiness accumulates -> wash occurs only when dirtiness >= ~900 (near critical threshold) -> two wash trips total (bins 500-599 and 1000-1099).

#### Agent B
**Categories**: Priority Override (primary), Belief Memory Pollution (secondary)
**Evidence**: Dirtiness 849/1440 ticks >750. 4 wash commits. Similar pattern to A. Agent B has 5 place-beliefs including Stone Well and West Grainfield — slightly better memory coverage but still missing Spring Basin in the sampled belief summary.
**Confidence**: HIGH
**Causal chain**: Same as Agent A — East Orchard convergence, dirtiness loses motive scoring to hunger/thirst.

#### Agent C
**Categories**: Priority Override (primary), Belief Memory Pollution (secondary), Resource Contention (transient secondary)
**Evidence**: Dirtiness 703 ticks >750. Also hit hunger=950 (97 ticks), thirst=900 (37 ticks), bladder=904 (10 ticks) — all sub-100-tick so not "sustained critical" by the detector threshold, but very close. Section 7 reports 7 frontier-exhausted plan attempts (highest of any agent) around tick 59-422 for AcquireCommodity(Apple/Water) at e6g0 — indicates brief contention for the Orchard when all 4 agents converged. Agent C ended with the least known place-beliefs (3 places).
**Confidence**: HIGH for dirtiness, MEDIUM for hunger/thirst spikes (sub-threshold)
**Causal chain**: Agent C had higher thirst_rate=4 (vs 3 for others) and lower thirst critical threshold=800. Early trip to Spring Basin (ticks ~60-80) covered thirst+wash simultaneously, then converged to East Orchard. Hunger spike at tick 100-200 coincided with 4-agent convergence on finite orchard capacity (7 frontier-exhausted plans at ticks 59, 131, 270, 291, 422).

#### Agent D
**Categories**: Priority Override (primary), Belief Memory Pollution (secondary)
**Evidence**: Dirtiness 842 ticks >750. 4 wash commits. 1 frontier-exhausted plan at tick 79 (Apple acquisition during early convergence).
**Confidence**: HIGH
**Causal chain**: Same structural pattern — East Orchard convergence, dirtiness subordinated to hunger/thirst.

### Damning Moments

#### Damning Moment DM-1: Agent A — Priority Override at tick 687

**Agent state at tick 687** (from Section 3 anomaly 1 range start):
- Location: East Orchard (e6g0)
- Needs: dirtiness=750+ (crossing threshold), hunger~80-190, thirst~140-200, fatigue~290 (interpolated from Section 2 avg/max + anomaly start), bladder~170
- Inventory: (probably 1 Apple — Agent A end-state is empty but mid-run held 1-2 Apples periodically based on harvest vs eat counts)
- Known recipes: Harvest Apples, Harvest Water, Harvest Grain

**Location state**:
- Facilities at East Orchard: OrchardRow (for Harvest Apples)
- Resource sources at East Orchard: Apple (capacity 6, regen 10/unit)
- Consumables at East Orchard: ~3 Apples on ground
- Adjacent places: Central Crossing (3 ticks); West Grainfield (4 ticks via direct edge)
- Nearest wash: Spring Basin (5 ticks: East Orchard -> Central Crossing -> Spring Basin)

**Agent beliefs about resources**:
- Believed locations: South Camp, Central Crossing, East Orchard only (per Section 5)
- Believed resources: OrchardRow at East Orchard
- Missing beliefs: Spring Basin (despite visiting for 57 ticks earlier), Stone Well, West Grainfield, Forest Glade

**Planner state**:
- Goal attempted (sampled from Section 7 bin 600-699): AcquireCommodity(Apple)@e6g0 selected; Wash appears in Goals selected list but was not selected in sampled PLAN entries at this bin.
- Outcome: Plans consistently found (0 frontier-exhausted, 0 budget-exhausted for Agent A).
- Candidates: 2-3 typical. Depth: 1. Expansions: 1.
- Competing goals: AcquireCommodity(Apple) with primary score ~140000 always dominated Wash (no Wash plan reaches the sampled PLAN entries through bin 600-699).

**Expected behavior chain** (if Priority Override were fixed):
1. At dirtiness >= 750, a drive escalation or override ranks Wash above hunger/thirst-driven AcquireCommodity
2. Plan: Travel East Orchard -> Central Crossing (3 ticks)
3. Travel Central Crossing -> Spring Basin (2 ticks)
4. Harvest Water at Spring Well (~8 ticks including possession)
5. Wash at Spring Washbasin (12 ticks)
6. Return to East Orchard (5 ticks)

**Actual behavior**: Agent A stayed at East Orchard harvesting/eating Apples and relieving in wilderness. Dirtiness rose monotonically except for the two wash trips (bin 500-599 and 1000-1099). Between those trips (~500 ticks), dirtiness climbed past 900 and lingered critical until the second wash cycle.

**Breakpoint**: Motive scoring in `worldwake-ai::goal_ranking` (or equivalent) — Wash goal base score uses `dirtiness_weight=625`, which never exceeds hunger (750) or thirst (700-725) motive scores while at East Orchard. The planner honors the weights, but the scenario's weight ratio makes dirtiness structurally subordinate to food/water.
- System: Goal ranking / drive escalation
- Code area: `worldwake-ai` goal selection, `worldwake-systems::needs` drive calculation

**Golden test blueprint**:
- Harness setup: 2 AI agents, 4 places (camp with latrine, crossing, wash site, food source), `dirtiness_weight=625`, `hunger_weight=750`, `wilderness_relief_dirtiness_penalty=200`. Start at camp, drive toward food source.
- Tick count: 800
- Primary assertion: Each agent performs at least 4 wash cycles over 800 ticks when wash facility is 2-3 hops from food source.
- Failure mode assertion: Current code produces <=2 wash cycles with dirtiness spending >60% of ticks above 750 permille.
- Regression guard: Any change to drive escalation / wash goal ranking must keep wash-cycle count >= threshold under this weight ratio, or explicitly document the trade-off.

#### Damning Moment DM-2: Agent C — Resource Contention Hunger Spike at tick 59

**Agent state at tick 59** (Section 3 anomaly 3 window + hunger trajectory):
- Location: Spring Basin (e5g0) mid-wash / South Camp route
- Needs: hunger=532, thirst=76, fatigue=250, bladder=36, dirtiness=228 (from anomaly window start)
- Inventory: unknown at exact tick; Section 2 shows Agent C had early-run Apple/Water harvests (harvest apples=16, waters=7 lifetime)
- Known recipes: Harvest Apples, Harvest Water, Harvest Grain

**Location state**:
- Facilities at Spring Basin: Spring Well, Spring Washbasin
- Water source: Spring Well (capacity 4, regen 10 ticks/unit)
- Adjacent places: South Camp (3 ticks), Central Crossing (2 ticks)

**Agent beliefs about resources**: (sampled from final Section 5; mid-run may have been richer)
- Known places at tick 59: likely South Camp + Spring Basin + Central Crossing + East Orchard (inferred from later visitation)

**Planner state** (Section 7):
- Goal attempted: AcquireCommodity(Apple) at e6g0 — 7 failed plan attempts at ticks 59, 131, 270, 291, 422 for Apple/Water.
- Outcome: frontier-exhausted (608 expansions, 3448-11336 candidates, depth 11)
- Candidates: 3448 at tick 59, 11336 at tick 291 — abnormally high candidate counts for a simple harvest plan suggest planner is exploring combinatorial travel routes because direct path to East Orchard yields no Apple (capacity consumed by other agents).

**Expected behavior chain**:
1. Recognize orchard contention; plan travel to alternative food location (West Grainfield — Agent C knows Harvest Grain)
2. Travel East Orchard -> Central Crossing -> West Grainfield (6 ticks)
3. Harvest Grain at Grainfield Plot
4. Consume Grain to relieve hunger

**Actual behavior**: Agent C ended up committing 16 Harvest Apples and 0 Harvest Grain actions over the full run. Despite knowing the Harvest Grain recipe and having West Grainfield as a reachable destination (3 ticks from Central Crossing), Agent C never diversified to Grain when Apple contention was high.

**Breakpoint**: Plan search prefers Apple affordance even when it's contested and alternative food (Grain at West Grainfield) is reachable within the planning depth. Either the FF heuristic over-weights known near-term operators, or `max_travel_candidates_per_expansion=4` prunes the Grain route before evaluation.
- System: GOAP plan search / travel-candidate expansion
- Code area: `worldwake-ai::search`, `worldwake-ai::candidate_generation`

**Golden test blueprint**:
- Harness setup: 3 AI agents, 1 contested Apple source (capacity 2), 1 alternative Grain source (capacity 4), both reachable from a central hub.
- Tick count: 300
- Primary assertion: At least one agent diversifies to Grain harvesting when Apple source is exhausted.
- Failure mode assertion: Current code leaves all agents fixated on Apple source with frontier-exhausted plan failures and rising hunger.
- Regression guard: Planner must expand to alternative commodity sources when primary is contested.

### Proposed Solutions

#### Priority Override (applies to DM-1, all dirtiness anomalies)

1. **Scenario fix — rebalance utility weights**
   - What: Raise `dirtiness_weight` from 625 to 680-710 for all agents, or lower `hunger_weight` from 750 to 700.
   - Where: `scenarios/survival-contested.ron` agent `utility_profile` blocks.
   - FOUNDATIONS alignment: Principle 5 (no magic numbers) — current values are scenario magic numbers; justify them explicitly or unify toward a default.
   - Existing specs: None directly address dirtiness weight calibration. `archive/specs/` has S94 residual candidate inventory but not dirtiness escalation.
   - Type: Scenario tuning.
   - Impact: Addresses DM-1 and the three other dirtiness failures (agents B, C, D).

2. **Engine fix — drive escalation on sustained critical needs**
   - What: Add a drive-escalation rule: when any need (including dirtiness) exceeds its critical threshold (here 900 permille) AND has been >high for N ticks, inflate its motive score by a multiplier until relieved. Distinct from the existing `PriorityClass` interrupt — this reshapes motive scoring, not ranking category.
   - Where: `worldwake-systems::needs` motive calculation or `worldwake-ai::goal_ranking`.
   - FOUNDATIONS alignment: Principle 8 (feedback dampening) — need-pressure feedback currently does not escalate; agents remain indifferent to long-sustained critical needs if hunger is competing. Principle 10 (belief-only planning) preserved because escalation operates on agent's own need readings.
   - Existing specs: Review `specs/` for drive escalation proposals; absent at time of writing.
   - Type: Engine change.
   - Impact: Addresses DM-1 structurally; prevents future scenarios from regressing to the same dirtiness equilibrium.

3. **Scenario fix — add secondary latrine / wash proximity**
   - What: Add a latrine at Central Crossing (on the travel corridor) or a second wash basin. Reduces cost of maintenance behaviors so they fit between meal cycles.
   - Where: `scenarios/survival-contested.ron` facilities list.
   - FOUNDATIONS alignment: Principle 1 (maximal emergence) — the scenario is testing contention; resource scarcity for wash/latrine is a scenario design choice. Adjusting should be deliberate.
   - Type: Scenario change. Note: doing this reduces the scenario's stress-test value; preferred only if the engine fix (#2) is not adopted.
   - Impact: Palliative — lowers travel cost, not root cause.

#### Belief Memory Pollution (applies to DM-1 secondary)

4. **Engine fix — priority-based belief retention**
   - What: Belief store prefers landmark/resource-source entries over ItemLot and Waste entries when at capacity. Currently `institutional_memory_capacity=16` with `max_snapshot_entities_per_place=50` allows East Orchard's 50+ entity cluster to push out distant-place beliefs.
   - Where: `worldwake-core` belief store eviction policy or perception profile.
   - FOUNDATIONS alignment: Principle 7 (locality of information) — retained; priority-based eviction still operates on agent-local observation history. Principle 10 (belief-only planning) — strengthened, since planner gets more useful beliefs.
   - Existing specs: Check `specs/` for belief retention / landmark memory work (several spec IDs mention belief decay but not priority-based retention).
   - Type: Engine change.
   - Impact: Addresses secondary cause for all 4 dirtiness failures (loss of Spring Basin / wash-site belief).

#### Resource Contention (DM-2)

5. **Engine fix — alternative-commodity expansion under planning failure**
   - What: When plan search returns frontier-exhausted for a commodity, seed next-cycle candidate generation with alternatives from `known_recipes` that satisfy the same need. Agent C's 7 frontier-exhausted Apple plans never triggered a Grain exploration.
   - Where: `worldwake-ai::candidate_generation` (generate_candidates) post-failure handling.
   - FOUNDATIONS alignment: Principle 12 (system decoupling) — preserved; candidate expansion stays in the AI crate.
   - Existing specs: Review for recipe-failure fallback; likely absent.
   - Type: Engine change.
   - Impact: Addresses DM-2 and future scenarios where one resource type is contested.

### Golden Test Recommendations

| Priority | Damning Moment | Test Name Suggestion | What It Guards Against |
|----------|----------------|----------------------|-----------------------|
| 1 | DM-1 | `golden_dirtiness_wash_cycle_under_priority_override` | Wash cycles falling below the threshold needed to keep dirtiness out of sustained-critical when wash is multi-hop from food. |
| 2 | DM-2 | `golden_food_diversification_under_apple_contention` | Agents fixating on a single contested food source when an alternative (Grain) is within planning depth. |
| 3 | (belief pollution) | `golden_landmark_belief_retention_under_itemlot_flood` | ItemLot/Waste observations crowding out landmark-place beliefs under memory-capacity pressure. |

---

## Layer 3: Detection Meta-Analysis

### False Positives

| Smell | Agent(s) | Why It's False | Detector Improvement |
|-------|----------|----------------|----------------------|
| STUCK_AGENT (Section 3 anomaly 3) | Agent C | 26-tick window coincides with a wash+travel cycle (wash_ticks=12 + 2x travel_ticks=2 each way). Wash is an ActionStarted/ActionCommitted multi-tick frame that the detector apparently does not recognize as "in-progress". | Detector should exclude windows where any `ActionStarted` occurred at or after the window start but has no matching `ActionCommitted` yet — this is an active multi-tick action, not idleness. |
| Behavioral transition, Agent B tick 1400 | Agent B | End-of-simulation artifact: needs well-managed (hunger 237, thirst 59, fatigue 297). Repertoire narrows to sleep + low-cost actions because all non-sleep goals are below interrupt thresholds. | Behavioral transition detection should deprioritize transitions within the last 100 ticks of the run when all needs are below 400 permille. |

### Detection Gaps

#### Gap 1: Geographic Convergence (HIGH)
**Evidence**: Section 2 location history — all 4 agents spent 1120-1204 ticks (78-84% of simulation) at East Orchard. Remaining places saw <10% of any agent's time. Spring Basin — the only wash site in the scenario — had 0-86 ticks of occupancy per agent (Agent C=1, Agent D=20, Agent A=57, Agent B=86).
**Agent(s)**: All four
**Why current detectors miss it**: No smell evaluates inter-agent spatial distribution. Smell 3 (Stuck Agent) operates per-agent, per-tick-window. Smell 10 (Economic Stagnation) checks per-agent needs-vs-resources but not aggregate spatial collapse.
**Impact**: HIGH — convergence caused 4× sustained-dirtiness anomalies. Symptom is clear (ticks>750 dirtiness); root cause was not surfaced by mechanical detection.

#### Gap 2: Latrine Abandonment (MEDIUM)
**Evidence**: Both camps (North, South) have `Latrine` tag and provide toilet affordance. Agents spent exactly 1 tick each in their starting camps. Total toilet actions committed across all 4 agents and 1440 ticks: 0 (Section 2 action counts show no toilet action at all). 100% of bladder relief came from `relieve_wilderness`, which applies +200 permille dirtiness penalty.
**Agent(s)**: All four
**Why current detectors miss it**: Current Smell 10 checks for unmet needs in resource-rich locations but does not cross-reference affordance usage. Agents have the toilet affordance (see tick 0 affordances for Agent A) but never travel back to use it.
**Impact**: MEDIUM — compounds the dirtiness failure via the +200 permille penalty per wilderness use. Over 22-27 relieve_wilderness actions per agent, this accounts for ~+4400-5400 permille accumulated dirt per agent over the run.

#### Gap 3: Single-Source Food Monoculture (MEDIUM)
**Evidence**: All 4 agents have Harvest Apples AND Harvest Grain recipes. Action counts: Harvest Apples total = 64, Harvest Grain total = 0. West Grainfield received 1-4 visit-ticks per agent. The Grainfield Plot never had a Harvest Grain action against it.
**Agent(s)**: All four
**Why current detectors miss it**: Smell 10 (Economic Stagnation) checks whether harvest/craft/trade is attempted, not whether the diversity of known recipes is exercised. A scenario designer's intent to provide alternative food sources is invisible to the detector if one source satisfies the need.
**Impact**: MEDIUM — would have manifested as a HIGH issue if Apple regen had been slower. Agent C's 7 frontier-exhausted Apple plans (DM-2) demonstrate the failure mode.

#### Gap 4: Wash-Cycle Starvation (MEDIUM)
**Evidence**: Smell 5 detects sustained critical needs but does not identify the sub-pattern where the relieving action IS available (wash appears in Goals selected and Final affordances on wash-site ticks) but fires too infrequently. Agents A/B/D each have 3-4 wash cycles over 1440 ticks while dirtiness remains >750 for 703-901 ticks.
**Agent(s)**: All four (dirtiness); generalizable to any need with expensive relief
**Why current detectors miss it**: Smell 6 (Unaddressed Needs) requires 0 relief attempts. Smell 5 requires sustained critical. Neither flags "relief attempted but at frequency insufficient for equilibrium".
**Impact**: MEDIUM — surfaces the priority-override pathology at the ratio level, independent of whether the critical threshold is crossed.

#### Gap 5: Sub-Threshold Acute Spikes (LOW)
**Evidence**: Agent C had hunger=950 (for 97 consecutive ticks) and thirst=900 (37 ticks), both below the 100-tick sustained-critical threshold. These came close to death-inducing levels (dehydration_tolerance=220 ticks — thirst at 900 for 37 ticks is dangerous proximity).
**Agent(s)**: Agent C
**Why current detectors miss it**: 100-tick threshold is calibrated for chronic accumulation. Acute spikes that approach the critical threshold and nearly cross it go unflagged.
**Impact**: LOW in this run (no death) but HIGH in scenarios with shorter tolerance windows.

### Threshold Assessment

| Threshold | Current Value | Assessment | Recommendation |
|-----------|---------------|------------|----------------|
| Stuck agent idle ticks | 20 consecutive ticks | Too low for multi-tick actions | Raise to 24 OR fix detector to exclude windows containing active multi-tick ActionStarted events |
| Redundant perception count | 10 observations | Appropriate | Keep; but consider filtering Waste/ItemLot cluster observations in denominator |
| Critical need threshold | 750 permille | Appropriate | Keep |
| Sustained critical duration | 100 consecutive ticks | Too high for acute spikes | Consider adding a secondary "acute spike" detector: need>=critical for >=30 ticks |
| Failed action spiral rate | >75% failure with 5+ attempts | Not triggered this run | No data to recommend change |
| Unaddressed need average | 750 permille | Appropriate | Keep |

### Proposed New Smell Categories

#### Proposed Smell 11: Geographic Convergence

**Detection logic**: For each tick-bin (e.g., 200 ticks), compute the distribution of agent-ticks across places. If 2+ agents spend >60% of their ticks at the same place, flag as convergence.
**Threshold**: 2+ agents, >=60% overlap at a single place over a 200-tick window.
**Mechanical vs. LLM**: Mechanical — observer binary already records per-agent location ticks (Section 2).
**Implementation scope**: Observer binary — add a pass over Section 2 location ticks, cross-tabulate per-tick presence.
**Example from this run**: All 4 agents at East Orchard for >1120 ticks (78%+). Convergence present over 11 of 14 100-tick bins.
**False positive risk**: Trade-hub or event-driven convergence (market days, festivals) could be legitimate. Filter: only flag if convergence correlates with >=1 sustained-critical-need anomaly for an affected agent.

#### Proposed Smell 12: Wash-Cycle Starvation (generalized: Infrequent Maintenance)

**Detection logic**: For each maintenance need (dirtiness, bladder, fatigue) and each agent, compare relief-action rate to the metabolism rate. If a need's accumulation rate (from metabolism_profile) persistently exceeds relief rate in a rolling window, flag even if the need never crosses critical.
**Threshold**: relief_rate / accumulation_rate < 1.0 over 200+ ticks AND need avg > medium threshold.
**Mechanical vs. LLM**: Mechanical — all inputs are in the dump (action counts, metabolism rates, needs trajectories).
**Implementation scope**: Observer binary — new Section 9 or extend Section 3.
**Example from this run**: Agent A — dirtiness_rate=1 permille/tick plus 22 relieve_wilderness × 200 permille = 4400 permille added over 1440 ticks. Wash removes ~1000 permille per cycle × 3 cycles = 3000 removed. Net deficit = 1400 permille, exactly matching observed mean dirtiness avg=723.
**False positive risk**: Scenarios with intentional hardship (e.g., survival narrative) may expect this. Use severity calibration, not absolute flag.

#### Proposed Smell 13: Recipe Monoculture

**Detection logic**: For each agent, list known_recipes. Compute per-recipe action count. If any recipe accounts for 100% of food-satisfying or thirst-satisfying actions while agent knows alternatives, flag.
**Threshold**: Single-recipe share >= 95% of need-relevant actions AND agent knows >=2 recipes for that need category.
**Mechanical vs. LLM**: LLM (requires knowing which recipes satisfy which needs) — or mechanical if observer enriches with recipe-need mapping.
**Implementation scope**: Scenario-file introspection + per-agent action counts.
**Example from this run**: All 4 agents: Harvest Apples=16 each, Harvest Grain=0 each. 100% food monoculture.
**False positive risk**: Low — agent may legitimately prefer one recipe if the other's ingredients are scarcer. Filter: only flag if the alternative source location was visited AND affordance was available.

---

## Cross-Cutting Patterns

### The Wash-Starvation Cycle (dominant pattern)

The central dynamic of this run:

1. **Initial convergence** (ticks 1-50): All 4 agents leave starting camps within tick 1-3 and converge on the food axis (Forest Glade -> Central Crossing -> East Orchard).
2. **Food-anchored equilibrium** (ticks 50-500): Agents settle at East Orchard with easy Apple harvest and consumption loop. Hunger/thirst stay low. Dirtiness accumulates from wilderness relief (+200 per use).
3. **Wash-trigger threshold** (around tick 500 for A, 700-900 for others): Dirtiness approaches ~900 (near critical). Only at this near-critical pressure does Wash motive score begin to contest hunger/thirst.
4. **Single wash trip** (~20-30 ticks): Travel to Spring Basin, harvest Water, wash, return. Dirtiness drops to ~200-300.
5. **Return to step 2**, repeat. Net cycle = ~500-800 ticks per wash, which is too slow to keep dirtiness below 750.

This pattern explains all 4 dirtiness anomalies and is invisible to current detectors (they flag the symptom — sustained critical dirtiness — but not the cycle-frequency mismatch).

### Cross-Layer Interaction: Priority Override amplified by Belief Memory Pollution

DM-1 identifies Priority Override as the primary cause. Belief Memory Pollution (Smell 8) is a secondary amplifier: when dirtiness finally outranks hunger via crossing the critical threshold, the agent needs to plan to Spring Basin. Agent A's belief memory at that moment does not include Spring Basin (crowded out by East Orchard's 50+ items). Planner then has to generate the wash plan from weaker base beliefs, potentially increasing search cost and further delaying wash.

### Latrine abandonment as a compounding driver

Gap 2 (Latrine Abandonment) interacts with Gap 1 (Geographic Convergence): agents abandoned camps partly because the camps offer no food/water, and partly because their anchor point for all other behaviors became East Orchard. The scenario assumed agents would rotate back to camps for bladder/sleep; they did not. `wilderness_relief_dirtiness_penalty=200` is a Chekhov's gun — designed to penalize wilderness relief, effective only if camps are periodically visited.

## Planner Diagnostics

| Agent | Plans Found | Frontier Exhausted | Budget Exhausted | Top Failed Goal | Candidate Count | Max Depth |
|-------|-------------|--------------------|--------------------|-----------------|-----------------|-----------|
| Agent A | 255 | 0 | 0 | n/a | n/a | n/a |
| Agent B | 261 | 0 | 0 | n/a | n/a | n/a |
| Agent C | 282 | 7 | 0 | AcquireCommodity(Apple) at e6g0 | 3448-11336 | 11 |
| Agent D | 260 | 1 | 0 | AcquireCommodity(Apple) at e6g0 | 4464 | 11 |

Assessment: **Parametric, not structural**. Zero budget-exhausted failures across all agents — `max_node_expansions=640` is adequate. The frontier-exhausted failures (7 for C, 1 for D) occurred at peak contention moments (ticks 59-422) and indicate the planner did reach the goal-search frontier limit in those specific acquisition plans. This is scenario-specific (orchard capacity 6 with 4 agents queueing) and not a planner deficiency.

The high candidate counts (3448-11336) at Agent C's failures are worth noting — this is disproportionate for a single-commodity harvest plan. Likely cause: planner exploring combinatorial travel-candidate expansions when the direct Apple route is unavailable. `max_candidates_per_expansion=240` × depth 11 = large search space.

## Trend Comparison

Prior `reports/scenario-analysis-report.md` existed (just archived as `scenario-analysis-report-2026-04-17-exploited.md` and referred to `survival-scattered.ron` seed 205005). Since the prior report is on a different scenario/seed, a tick-comparison is not apples-to-apples. Notable observations carried forward:

- Prior (scattered, 3 agents): 0 critical/high/medium Layer 1 findings, no Layer 2 triggered.
- Current (contested, 4 agents): 1 HIGH Layer 1 finding (Sustained Critical Needs), Layer 2 triggered with 4 agents affected.
- Tier 3 scenario **is** more adversarial than Tier 2, as designed. The scenario achieves its stated purpose (stress-testing under contention) but surfaces an unintended failure mode (dirtiness cycle breakdown) rather than the intended ones (water contention, belief invalidation).

## Summary Statistics
- Layer 1 findings: 5 (categories with severity other than NONE)
- By severity: 0 CRITICAL, 1 HIGH (Sustained Critical Needs), 2 MEDIUM (Redundant Perception, Belief Staleness), 2 LOW (Action Loops, Stuck Agents)
- Layer 2: 4 failure classifications (all 4 agents dirtiness), 2 damning moments (DM-1 Priority Override, DM-2 Resource Contention)
- Layer 3: 2 false positives, 5 detection gaps, 3 new smell proposals
- Agents with issues: Agent A, Agent B, Agent C, Agent D (all)
- Clean agents: none
- Scenario purpose achieved: **Partially** — the scenario successfully stress-tests 4-agent coexistence and surfaces a real emergent failure (dirtiness cycle breakdown under priority override), but the *intended* stress axes (water contention, belief invalidation mid-plan on depleted wells) were not exercised because all four agents converged on East Orchard and the water sources were adequate. The dirtiness failure, while real, is driven by scenario-chosen utility weights and latrine geography rather than the contended-water design.
