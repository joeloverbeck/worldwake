# Simulation Observer Report

## Run Summary

- **Scenario**: `scenarios/cli-evaluation.ron`
- **Seed**: 7777
- **Ticks simulated**: 1440 (1 simulated day)
- **Total events**: 9876
- **Agents**: 4 (Kael, Merchant Vara, Forager Lina, Guard Theron)
- **Places**: 5 (Thornwall Village, Eldergrove Forest, Dusty Trail, Hearthstone Inn, Golden Fields)
- **Deaths**: Guard Theron died at tick 422 from NeedDeprivation { Hunger }

## Findings

### 1. Redundant Perception — MEDIUM

**Agent(s)**: All four agents
**Evidence**: Kael observed himself 112 times, Merchant Vara 105 times, Guard Theron 112 times (observing himself). Every agent also observed their current location 10-63 times and co-located agents dozens of times.
**Root cause hypothesis**: The perception system fires on every co-located entity each perception tick, regardless of whether the entity's state has meaningfully changed. Self-observation is especially suspect — an agent's own state changes are already authoritative and don't need repeated perception. Place entities with static features (Well, Forge) are also re-observed without change.
**Confidence**: HIGH — the sheer volume (e.g., 112 self-observations) strongly suggests the perception system lacks change-detection filtering.

### 2. Action Loops — HIGH

**Agent(s)**: Kael, Merchant Vara, Forager Lina
**Evidence**:
- **Kael**: From tick 500 onward (940 ticks), action repertoire collapsed to only `sleep` + `relieve_wilderness`. Behavioral transition flagged at tick 500 (4 types → 2 types). The planner continued selecting Sleep and Relieve goals every tick for the remaining 940 ticks — 51 budget-exhausted plan searches indicate the planner repeatedly failed to find plans for higher-priority goals (AcquireCommodity Water/Bread).
- **Merchant Vara**: Similar collapse after tick 900 to `sleep` + `relieve_wilderness` only (ate until tick 800 sporadically). Mechanical action loop flagged: `[sleep → sleep]` repeated 3x. 43 budget-exhausted searches for AcquireCommodity(Water).
- **Forager Lina**: From tick 732 onward (708 ticks), Lina stopped executing any actions entirely despite the planner continuously selecting `FreeCarryCapacity` (0-step plans). Her inventory filled with 12 Waste items; the planner kept finding `FreeCarryCapacity` as the top goal but the plan had `steps=0` and `next_index=None` — the plan was marked "GoalSatisfied" but no action was actually dispatched. This is a planning-execution gap: the planner believes the goal is satisfied (the item is droppable) but no drop action is actually started.

**Root cause hypothesis**: Two distinct issues:
1. **Resource starvation at Dusty Trail** (Kael, Vara): After moving to Dusty Trail, agents lost access to `eat`, `drink`, and `wash` affordances. The planner generates AcquireCommodity goals but budget-exhausts searching for multi-step plans (travel → acquire → return). The search space is too large (2000+ candidates, depth 4-9) for the expansion budget (150-300).
2. **FreeCarryCapacity loop** (Lina): The planner selects FreeCarryCapacity with a 0-step plan (GoalSatisfied) but no action is dispatched. This blocks all other goals since FreeCarryCapacity keeps winning the goal ranking with a motive score of 280,000. The agent is stuck planning to drop waste but never actually dropping it.

**Confidence**: HIGH — action timeline data and decision timeline both confirm the patterns clearly.

### 3. Stuck Agents — HIGH

**Agent(s)**: Forager Lina (708 ticks), Guard Theron (1019 ticks), Kael (34 ticks), Merchant Vara (27 ticks)
**Evidence**:
- **Forager Lina**: 708 consecutive idle ticks (tick ~732–1440). Despite the planner running every tick and selecting FreeCarryCapacity, no actions were dispatched. Lina had harvest, eat, sleep, and relieve_wilderness in her affordances but the FreeCarryCapacity goal (motive score 280,000) always won, and its 0-step plan produced no action.
- **Guard Theron**: 1019 consecutive idle ticks, but 1018 of those are post-death (died at tick 422). The real pathological idle window is ticks 200-422 where Theron was alive with rising hunger/thirst but only executed sleep and relieve_wilderness. From tick 200 onward, the planner produced `candidates=0, plans_found=0` on 56+ planning ticks — no goal candidates were generated at all.
- **Kael/Vara**: 34 and 27 consecutive idle ticks respectively. These are brief and coincide with sleep cycles — likely not pathological.

**Root cause hypothesis**:
- Lina: FreeCarryCapacity goal blocks all other goals. The planner thinks the goal is satisfied (GoalSatisfied with 0 steps) but no action executes, so the inventory never frees up, and the cycle repeats.
- Theron: After moving to Dusty Trail (tick ~166), he had no eat/drink affordances. His goal set was dominated by InvestigateViolation and Patrol (which required Thornwall Village), but AcquireCommodity(Water) kept budget-exhausting. From tick 200+ the planner generated 0 candidates — all his goals were either location-locked to Thornwall Village (investigate/patrol with no travel affordance to that place?) or budget-exhausted (acquire water). With 0 candidates, he could only sleep and relieve.

**Confidence**: HIGH for Lina and Theron (pathological), LOW for Kael/Vara (likely benign sleep gaps).

### 4. Failed Action Spirals — LOW

**Agent(s)**: Merchant Vara (staff_market), Guard Theron (tell, investigate)
**Evidence**:
- Merchant Vara attempted `staff_market` 5 times — all StartFailed (0 committed). This suggests a precondition consistently fails (possibly no market stall present, or wrong location).
- Guard Theron had 21 `tell` StartFailed events. Likely targeting agents who moved away or have a state preventing reception.
- Guard Theron had 27 `investigate` starts but only 4 committed — most were interrupted by the planner finding higher-priority goals (AcquireCommodity/ShareBelief), causing constant replanning churn rather than completion.

**Root cause hypothesis**: `staff_market` preconditions are not met at any of Vara's locations — neither Thornwall Village nor Dusty Trail have a market stall configured in the scenario, or Vara lacks the required component. Theron's investigate interruptions are caused by the planner's goal-ranking oscillation between duty-based goals (investigate/patrol) and survival goals (acquire water) — neither can be planned successfully, so the planner thrashes between them.
**Confidence**: MEDIUM — staff_market failures are consistent; investigate interruptions are visible in the decision timeline.

### 5. Sustained Critical Needs — CRITICAL

**Agent(s)**: All four agents
**Evidence**:
- **Guard Theron**: Hunger above 750‰ for 1215 ticks (tick 225–1439, died at 422). Thirst above 750‰ for 1290 ticks (tick 150–1439).
- **Kael**: Thirst above 750‰ for 915 ticks (tick 525–1439). Hunger above 750‰ for 674 ticks (tick 766–1439). Dirtiness above 750‰ for 790 ticks (tick 650–1439).
- **Merchant Vara**: Thirst above 750‰ for 1257 ticks (tick 183–1439). Hunger above 750‰ for 265 ticks (tick 1175–1439). Dirtiness above 750‰ for 790 ticks.
- **Forager Lina**: Thirst above 750‰ for 584 ticks (tick 856–1439). Hunger above 750‰ for 361 ticks. Fatigue above 750‰ for 480 ticks. Dirtiness above 750‰ for 810 ticks.

**Root cause hypothesis**: Three agents (Kael, Vara, Theron) moved to Dusty Trail which lacks food/water sources. The planner's budget-exhaustion on AcquireCommodity goals (requiring multi-step travel + acquire plans) means they can never plan to get food/water. Lina's needs rise because FreeCarryCapacity blocks all other goal planning from tick 732 onward. Dirtiness is universally unaddressed — no `wash` affordance at Dusty Trail or Eldergrove Forest (WashBasin is at Hearthstone Inn, which no agent visits).
**Confidence**: HIGH — the sustained durations and outcomes (Theron's death) are unambiguous.

### 6. Unaddressed Needs — CRITICAL

**Agent(s)**: Merchant Vara (thirst), Guard Theron (hunger, thirst)
**Evidence**:
- Merchant Vara: Thirst averaged 926‰ but she never attempted a `drink` action across the entire simulation. Her tick-0 affordances at Thornwall Village had no `drink` affordance listed. After moving to Dusty Trail (tick 60), her affordances showed `eat` but not `drink`.
- Guard Theron: Hunger averaged 915‰ and thirst averaged 943‰ — neither `eat` nor `drink` was ever attempted. His affordances at Dusty Trail showed no food or water sources.

Cross-referencing Section 7: Vara's failed plans show repeated `AcquireCommodity { commodity: Water }` budget-exhausted at 300 expansions with 1483-2611 candidates. The plan exists in the search space but is too deep (depth 4-9) for the expansion budget. Theron's failed plans show the same pattern — AcquireCommodity(Water) budget-exhausted at 224 expansions, depth 4-6, 813-2085 candidates.

**Root cause hypothesis**: The core issue is **missing local drink/eat affordances** at Dusty Trail. The planner sees AcquireCommodity goals and tries to plan them, but the required action chain (travel to location with water source → drink → return) exceeds the search budget. The Well (water source) is at Thornwall Village; food sources appear limited. Without local resources, agents are structurally starved.
**Confidence**: HIGH — affordance data confirms no drink/eat actions available at agents' final locations.

### 7. Impossible Knowledge — NONE

No evidence found of agents acting on unobserved information. All action targets correspond to entities within agents' perception traces or told/heard beliefs. Kael and Merchant Vara both received told beliefs about entities, and their actions align with known entity sets.

### 8. Belief Staleness — MEDIUM

**Agent(s)**: Kael, Merchant Vara, Forager Lina
**Evidence**:
- **Kael**: Knows 16 entities but only 2 agents (doesn't know Forager Lina exists despite both being in the same simulation). Believes his location is Dusty Trail, which is accurate. His known items are 13 Waste — reflecting the waste-heavy state of Dusty Trail. He knows about Thornwall Village (from told beliefs via Guard Theron) but doesn't know about the Well there which could address his thirst.
- **Merchant Vara**: Knows only 1 agent (Kael), 1 place (Dusty Trail), 10 Waste items. She has 0 social observations despite being co-located with Kael for ~1380 ticks. Her heard beliefs (1) and told beliefs (0) suggest minimal information exchange despite proximity.
- **Forager Lina**: Knows 0 agents despite other agents being in the simulation. Knows only Eldergrove Forest. Her beliefs include Apple and ChoppingBlock/OrchardRow — accurate for her location. But she has no knowledge of other places or water sources, which means she cannot plan to address thirst even if FreeCarryCapacity weren't blocking.

**Root cause hypothesis**: Information does not propagate effectively between places. Agents at different locations have completely siloed belief systems. The tell action is used but primarily for sharing entity beliefs (about waste items), not for sharing resource location knowledge. No agent learns about the Well at Thornwall Village through social channels.
**Confidence**: MEDIUM — the belief summaries clearly show information silos, but whether this is "stale" vs. "never acquired" is a distinction. The beliefs are accurate for what agents have observed; the issue is that they've never observed or been told about key resources.

### 9. Social Isolation — MEDIUM

**Agent(s)**: Forager Lina, Guard Theron (post-death)
**Evidence**:
- **Forager Lina**: Spent all 1440 ticks at Eldergrove Forest alone. 0 social observations, 0 told beliefs, 0 heard beliefs. No Tell, AskWitness, or Trade actions. She is completely isolated from all other agents.
- **Kael and Merchant Vara**: Co-located at Dusty Trail for ~1380 ticks. Kael attempted 24 tell actions (19 committed, 4 StartFailed). Merchant Vara attempted 20 tell actions (15 committed, 3 StartFailed). However, all tells appear to be ShareBelief about Waste entities — no economically or strategically useful information was exchanged. No trade actions despite both being co-located with resources.
- **Guard Theron**: Had brief social interactions (9 tells, 21 attempted, all before tick ~200) before behavioral collapse and death.

**Root cause hypothesis**: Lina never encounters other agents because she never travels. Kael and Vara technically interact but only share trivial beliefs about Waste items — the tell system works mechanically but doesn't produce useful information exchange. No agent attempts trade despite co-location and complementary needs.
**Confidence**: HIGH for Lina's total isolation. MEDIUM for Kael/Vara's superficial interaction.

### 10. Economic Stagnation — CRITICAL

**Agent(s)**: Kael, Merchant Vara, Guard Theron
**Evidence**:
- **Dusty Trail has no production facilities**: No harvest sources, no crafting stations, no water. The only resources at end-state are Waste (36 items), Coins (20), weapons (Bow, Sword). Three agents are trapped at a location with zero economic potential.
- **Kael**: Holds 20 Coins but there is nothing to buy and no market. Never attempted harvest, craft, or trade.
- **Merchant Vara**: Empty inventory. Has a merchant role but `staff_market` failed 5 times. Never successfully conducted trade. Her goal set includes `SellCommodity { commodity: Grain }` but she has no grain and no way to acquire it at Dusty Trail.
- **Guard Theron**: Held weapons (Bow, Sword) but died from hunger while carrying them. His duty-focused goal set (InvestigateViolation, Patrol) consumed planning capacity while survival needs went unmet.
- **Forager Lina**: The only economically active agent — harvested 13 apple batches, picked up 26 items. However, her inventory filled with Waste (12 items at end-state, plus 14 Waste on the ground at Eldergrove Forest), blocking further productive activity. Only 2 Apples remain at Eldergrove at end-state.

Cross-referencing Section 7: The failed plans reveal that AcquireCommodity goals consistently budget-exhaust at depth 4-9 with thousands of candidates. The planner can see that resources exist elsewhere but cannot find plans within its expansion budget to reach them.

**Root cause hypothesis**: The scenario places 3 of 4 agents (or has them travel to) Dusty Trail, which is a resource desert. The planner's expansion budget (150-300) is insufficient for multi-location plans involving travel. The result is complete economic paralysis: agents can see goals but never plan paths to achieve them. Lina's productive economy at Eldergrove Forest is capped by carry capacity and Waste accumulation with no FreeCarryCapacity resolution.
**Confidence**: HIGH — the economic data is unambiguous. No trade, no cross-location resource flow, production capped by Waste.

## Cross-Cutting Patterns

### Pattern 1: Dusty Trail is a Death Trap
Three agents (Kael, Merchant Vara, Guard Theron) all end up at Dusty Trail, which has no food, no water, and no wash facilities. Theron died there. Kael and Vara are on a slow trajectory toward death. The scenario's topology creates a one-way trap: agents travel to Dusty Trail but the planner cannot find plans to travel back to resource-rich locations (budget-exhaustion on multi-step plans).

### Pattern 2: FreeCarryCapacity Deadlock (Lina)
Forager Lina represents a different failure mode. She has access to resources (Eldergrove Forest has apples, an orchard row) and was the most economically active agent for the first 730 ticks. But Waste accumulation filled her carry capacity, and the FreeCarryCapacity goal produces 0-step "GoalSatisfied" plans that never dispatch an action. This is a planner-execution gap: the planner marks the goal as achievable but no action fires, creating an infinite planning loop that blocks all other goals.

### Pattern 3: Planner Budget Exhaustion Cascade
The planner's expansion budget (150-300 per search) is structurally inadequate for multi-location plans. Every agent that needs resources from another location hits budget-exhaustion on AcquireCommodity goals. With 1483-2611 candidates at depth 4-9, the search space grows faster than the budget allows. This means the simulation's place graph, intended to create interesting travel dynamics, actually creates impassable planning barriers.

### Pattern 4: Guard Theron's Death Chain
Theron's death at tick 422 follows a clear causal chain:
- Tick 0-99: Active investigating violations and patrolling (duty goals), but AcquireCommodity(Water) budget-exhausted from the start
- Tick 100-199: Increasingly interrupted by survival-goal replanning (AcquireCommodity keeps winning priority class but can't be planned)
- Tick 200+: Planner generates 0 candidates on most ticks — all possible goals either location-blocked or budget-exhausted
- Tick 200-422: Only sleep and relieve_wilderness possible; hunger/thirst at 1000‰
- Tick 422: Death from hunger deprivation

## Summary Statistics

- Total findings: 8 (categories with severity other than NONE)
- By severity: 3 CRITICAL, 3 HIGH, 2 MEDIUM, 1 LOW
- Agents with issues: Kael (loops, stuck, sustained needs, belief staleness), Merchant Vara (loops, stuck, unaddressed needs, sustained needs, failed spirals, belief staleness, economic stagnation), Forager Lina (stuck 708 ticks, FreeCarryCapacity deadlock, sustained needs, isolation, belief staleness), Guard Theron (dead at tick 422, unaddressed needs, sustained needs, failed spirals, economic stagnation)
- Clean agents: None

## Trace Quality Assessment

### Trace Sufficiency
The dump provides comprehensive data for all 10 smell categories. Section 7's decision timeline is particularly valuable — it reveals the planner's internal state (goal selection, budget exhaustion, candidate counts) which is essential for diagnosing root causes. The main limitation is the absence of Blocked Desires subsections (no fully blocked desires detected), but failed plan attempts and affordances provide equivalent diagnostic value.

### Limitations and Recommended Additions

| ID | Limitation | Classification | Rationale |
|----|-----------|----------------|-----------|
| TQ-1 | FreeCarryCapacity 0-step plan produces no action — unclear whether this is a plan-execution dispatch bug or intentional behavior for already-satisfied goals | Actionable | Prevented confident root-cause for Lina's 708-tick idle period — is this a bug in action dispatch for 0-step plans, or should FreeCarryCapacity never produce a 0-step GoalSatisfied plan? |
| TQ-2 | No per-tick carry capacity or inventory fullness metric in the dump | Actionable | Would confirm the exact tick Lina's inventory became full and correlate with behavioral collapse. Currently inferred from end-state inventory + action timeline transition. |
| TQ-3 | Affordance snapshots don't explain *why* an affordance is absent | Actionable | At tick 463, Kael arrives at Dusty Trail and loses eat/drink/wash affordances. Without knowing *why* (no Well? no food items? precondition?), root-cause for resource starvation requires reading scenario + code. Misaligns with FOUNDATIONS Principle 7 (locality of information) — the observer should capture the reasoning. |
| TQ-4 | No explicit "goal blocked by capacity" annotation in the decision timeline | Acceptable trade-off | Would help identify when FreeCarryCapacity is blocking other goals, but the existing data (FreeCarryCapacity always selected, high motive score) is sufficient for diagnosis. |
| TQ-5 | Section 5 belief summaries don't include resource-type beliefs (e.g., "knows Water exists at Thornwall Village") | Actionable | Would help assess whether agents have the knowledge to plan multi-location resource acquisition but can't act on it (budget-exhaustion) vs. not having the knowledge at all. Currently this is ambiguous. |

For **Actionable** items:

**TQ-1**: Recommended addition: Add a diagnostic annotation when the planner selects a 0-step plan with `next_index=None` — either flag it as a dispatch anomaly or explain why no action was queued. Scope: Engine instrumentation (planner/action dispatch boundary).

**TQ-2**: Recommended addition: Include per-agent carry capacity utilization (current/max) in Section 2's needs trajectory or as a separate row. Scope: Observer-binary enhancement.

**TQ-3**: Recommended addition: For each affordance snapshot, include a "missing affordances" section listing common actions (eat, drink, wash, harvest) that are NOT available and the reason (no target entity, precondition failed, no resource at location). Scope: Observer-binary enhancement (query affordance system for explanations).

**TQ-5**: Recommended addition: Include commodity-awareness beliefs in Section 5 — specifically, which commodities the agent believes exist at which locations. Scope: Observer-binary enhancement (extract from belief store).
