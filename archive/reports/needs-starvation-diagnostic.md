**Status**: ✅ COMPLETED

# Needs Starvation Diagnostic

## Run Summary
- **Scenario**: `scenarios/cli-evaluation.ron`
- **Seed**: 7777
- **Ticks simulated**: 1440
- **Agents**: Kael, Merchant Vara, Forager Lina, Guard Theron
- **Places**: Thornwall Village (e0g0), Eldergrove Forest (e1g0), Dusty Trail (e2g0), Hearthstone Inn (e3g0), Golden Fields (e4g0)
- **Deaths**: Guard Theron at tick 1342 (cause: NeedDeprivation { Hunger })

## Agent Needs Overview

| Agent | Need | Max Value | Ticks >750‰ | Death? | Root Cause Category |
|-------|------|-----------|-------------|--------|---------------------|
| Kael | Hunger | 1000 | 671 | No | Geographic Desert + Belief Blindness |
| Kael | Thirst | 1000 | 922 | No | Geographic Desert + Belief Blindness |
| Kael | Dirtiness | 1000 | 790 | No | Structural Impossibility (at location) |
| Merchant Vara | Hunger | 1000 | 1171 | No | Planner Budget Wall + Geographic Desert |
| Merchant Vara | Thirst | 1000 | 860 | No | Geographic Desert + Belief Blindness |
| Merchant Vara | Dirtiness | 1000 | 361 | No | Structural Impossibility (at location) |
| Forager Lina | Dirtiness | 1000 | 810 | No | Structural Impossibility (at location) + Exploration Failure |
| Guard Theron | Hunger | 1000 | 336 | Yes (tick 1342) | Priority Override + Geographic Desert |
| Guard Theron | Thirst | 1000 | 370 | Yes (tick 1342) | Priority Override + Geographic Desert |
| Guard Theron | Fatigue | 1000 | 410 | Yes (tick 1342) | Priority Override |

## Failure Classifications

### Kael
**Categories**: Geographic Desert, Belief Blindness
**Confidence**: HIGH

**Evidence**: Kael traveled from Thornwall Village (e0g0, resource-rich) to Dusty Trail (e2g0, barren) at tick 15 and never returned. Dusty Trail has NO facilities, NO resource sources. Kael consumed his 5 initial Water items by ~tick 518 (thirst went critical) and 5 Bread items early on. After that, no eat/drink affordances were available.

By end-state (tick 1439), Kael's beliefs only contain Dusty Trail entities — 11 SocialArtifacts, 2 Waste, 2 agents. He has forgotten Thornwall Village entirely (memory_retention_ticks: 64 decayed long ago). His belief system is saturated with SocialArtifacts posted by Guard Theron.

**Causal chain**: Kael travels to Dusty Trail → consumes initial Water/Bread inventory → Dusty Trail has no facilities/resources → memory of Thornwall Village decays within 64 ticks → no exploration_profile to drive rediscovery → beliefs fill with SocialArtifacts → no eat/drink affordances ever again → hunger/thirst/dirtiness escalate to max.

Kael has NO cognitive_profile, NO exploration_profile, NO metabolism_profile. He uses engine defaults for planning budget. He knows "Harvest Water" and "Harvest Grain" recipes but can't execute them at Dusty Trail (no Well, no FieldPlot). He also has no AcquireCommodity budget-exhaustion failures — the planner never even attempted food acquisition because there are no eat/drink affordances at Dusty Trail and no believed resource locations to plan toward.

### Merchant Vara
**Categories**: Planner Budget Wall, Geographic Desert, Belief Blindness
**Confidence**: HIGH

**Evidence**: Merchant Vara also migrated to Dusty Trail early (by tick 21). She made periodic trips back to Thornwall Village (ticks 100, 154, 223, 313) but NEVER ate at any location despite having hunger above 750‰ from tick 269 onward. Anomaly 17 confirms: "hunger avg 892‰ but no relief action (eat) was ever attempted."

Section 8 snapshots reveal the mechanism: At tick 35 (Dusty Trail), AcquireCommodity for Bread/Apple/Grain all budget-exhausted with 300 expansions and 693 candidates. At tick 109 (Thornwall Village), the same goals budget-exhausted with 705 candidates — even though 8× Grain was physically present at her location. She believed the Grain was there (Thornwall Village: 8× Grain in beliefs), yet the planner couldn't find a plan to acquire it within 300 expansions.

The S94 commodity-relevance pruning is implemented but insufficient: 693-705 candidates remain after pruning. The planner reaches depth 9 (of max 10) but exhausts its 300-node expansion budget before finding a valid path. This is a classic Budget Wall — the search space is still too large even after S94's pruning.

She made 9 travel trips and 3 Harvest Water actions, 5 drink actions, 1 wash — showing she CAN execute multi-step plans for Water. But food acquisition plans consistently budget-exhaust.

**Causal chain**: Merchant Vara generates AcquireCommodity(Food) goals → planner produces 693-705 candidates → 300-expansion budget exhausted at depth 9 → food acquisition never succeeds → no eat affordance directly available → hunger hits max at tick 269 and never recovers → behavioral transition at tick 400 narrows to 2 action types → by tick 820, hunger=1000, thirst=1000.

### Forager Lina
**Categories**: Structural Impossibility (at location), Exploration Failure
**Confidence**: HIGH

**Evidence**: Forager Lina stayed at Eldergrove Forest for all 1440 ticks. She managed hunger excellently (max 264, 64 eat actions, 28 apple harvests) and thirst adequately (max 615, 5 drink actions). However, dirtiness reached 1000 with 810 ticks above 750‰ and 0 wash actions.

Eldergrove Forest has no WashBasin. The only WashBasin in the scenario is at Hearthstone Inn (4 travel-ticks via Thornwall Village, or no direct edge). Forager Lina has an exploration_profile (curiosity_weight: 650, need_activation_threshold: 350) but never left Eldergrove Forest. Her beliefs at end-state only contain Eldergrove Forest entities. She never attempted ExploreLocation (not in goals-selected list; 0 frontier/budget-exhausted for non-ShareBelief goals).

Her exploration_profile's `need_activation_threshold: 350` may be too high for dirtiness, or the exploration candidate generation may not be triggered by dirtiness (only hunger/thirst). Additionally, the dirtiness_weight in her utility_profile is only 200 — one of the lowest weights, potentially too low to generate ExploreLocation goals even when dirtiness is critical.

**Causal chain**: Forager Lina at Eldergrove Forest → no WashBasin → dirtiness accumulates → exploration_profile exists but dirtiness_weight (200) too low to trigger ExploreLocation → agent never explores → dirtiness saturates at 1000.

### Guard Theron
**Categories**: Priority Override, Geographic Desert
**Confidence**: HIGH

**Evidence**: Guard Theron **died** at tick 1342 from hunger. He executed **487 post_notice** actions — by far his dominant activity. His utility_profile has notice_posting_weight: 900 and bounty_posting_weight: 700 — the highest weights in his profile, exceeding danger_weight (800) and far exceeding hunger_weight (400) and thirst_weight (400).

He had a diverse action repertoire early (12 action types including eat, drink, harvest, patrol, investigate, travel) but behavioral transitions at ticks 900 and 1000 narrowed his repertoire dramatically. By tick 930 (Section 8 Snapshot 10), his needs were hunger=404, thirst=333, fatigue=552 — all moderate — but he was budget-exhausting on a Sleep goal at Dusty Trail.

Guard Theron **does not have an obligation_satiation_profile** in the scenario. S96 (obligation satiation) was implemented to prevent exactly this pathology — repeated obligation execution without decay. But without the profile on this agent, the satiation mechanism is inactive. The undamped obligation loop (post_notice → triggers reinspection → post_notice) ran unchecked.

He patrolled between Dusty Trail and Thornwall Village (patrol_route: ["Dusty Trail", "Thornwall Village"]) and did successfully eat 10 times and drink 12 times early on. But the 487 post_notices consumed the vast majority of his action budget, leaving insufficient ticks for survival actions.

**Causal chain**: Guard Theron patrols to Dusty Trail → posts notices (notice_posting_weight: 900) → no obligation_satiation_profile → posting doesn't decay → notices dominate all decisions → hunger/thirst/fatigue escalate → by tick 1104, hunger above 750‰ → Dusty Trail has no food → insufficient time for travel+eat cycles → death at tick 1342.

## Damning Moments

#### Damning Moment DM-1: Guard Theron — Priority Override + Geographic Desert at tick 930

**Agent state at tick 930**:
- Location: Dusty Trail (e2g0)
- Needs: hunger=404‰, thirst=333‰, fatigue=552‰, bladder=404‰, dirtiness=112‰
- Inventory: 1× Bow, 1× Sword
- Known recipes: Harvest Water

**Location state**:
- Facilities at Dusty Trail: none
- Resource sources at Dusty Trail: none
- Consumables at Dusty Trail: 20× Coin, 34× Waste (no food or water)
- Adjacent places: Thornwall Village (2 ticks — has Well, Mill, Loom, but no food items remaining)

**Agent beliefs about resources**:
- Believed locations: Dusty Trail only
- Believed resources: Kael, Merchant Vara, Guard Theron + 12 SocialArtifacts at Dusty Trail
- Missing beliefs: Thornwall Village (Well), Eldergrove Forest (Apples, Water, OrchardRow), Hearthstone Inn (WashBasin)

**Planner state**:
- Goal attempted: Sleep
- Outcome: budget-exhausted (224 expansions, depth 7, 706 candidates)
- Competing goals: PostNotice dominated 487 times over entire simulation; 40 patrol, 19 investigate, 57 tell
- notice_posting_weight (900) >> hunger_weight (400), thirst_weight (400)

**Expected behavior chain**:
1. Goal ranking should prioritize hunger (404‰) or thirst (333‰) over PostNotice when needs are moderate
2. Generate AcquireCommodity(Grain, SelfConsume) or AcquireCommodity(Water, SelfConsume)
3. Travel to Thornwall Village (2 ticks)
4. Harvest Water at Village Well (or pick up Grain if available)
5. Drink Water / Eat Grain

**Actual behavior**: Continued posting ThreatWarning notices at Dusty Trail in an undamped loop until death.

**Breakpoint**: Goal ranking — PostNotice with notice_posting_weight 900 consistently outranks survival needs even at moderate-to-high levels. No obligation_satiation_profile to decay posting drive.
- System: worldwake-ai::ranking — `post_notice_motive` score unreduced by satiation
- Code area: `crates/worldwake-ai/src/ranking.rs` (obligation satiation application), `crates/worldwake-core/src/obligation.rs`

**Golden test blueprint**:
- Harness setup: 2-place graph (Trail, Village). Trail has no facilities. Village has Well + Water source. Agent at Trail with hunger=400, thirst=300, notice_posting_weight=900, hunger_weight=400, patrol_profile, NO obligation_satiation_profile. Place SocialArtifacts at Trail.
- Tick count: 200
- Primary assertion: Agent should travel to Village and eat/drink within 50 ticks when needs are moderate
- Failure mode assertion: Agent posts notices continuously at Trail without ever traveling for food, eventually dying. Specifically: agent commits >100 post_notice actions while hunger exceeds 500‰.
- Regression guard: Once obligation_satiation_profile is added, agent should prioritize survival over obligation spam.

---

#### Damning Moment DM-2: Merchant Vara — Planner Budget Wall at tick 109

**Agent state at tick 109**:
- Location: Thornwall Village (e0g0)
- Needs: hunger=426‰, thirst=75‰, fatigue=290‰, bladder=68‰, dirtiness=212‰
- Inventory: empty
- Known recipes: Harvest Water, Harvest Grain, Harvest Apples, Bake Bread

**Location state**:
- Facilities at Thornwall Village: Mill, Loom, Well
- Resource sources at Thornwall Village: Water (Village Well, regen 3 ticks/unit, capacity 15)
- Consumables at Thornwall Village: 8× Grain, 1× Bow, 1× Sword (Grain is edible)
- Adjacent places: Dusty Trail (2 ticks), Eldergrove Forest (3 ticks — apples, water), Golden Fields (5 ticks — FieldPlot), Hearthstone Inn (4 ticks — WashBasin)

**Agent beliefs about resources**:
- Believed locations: Thornwall Village, Dusty Trail
- Believed resources at Thornwall Village: Merchant Vara, Guard Theron, 8× Grain, 1× Sword, 1× Bow, Mill, Loom, Well
- Believed resources at Dusty Trail: Kael, 1× Waste
- Missing beliefs: Eldergrove Forest, Golden Fields, Hearthstone Inn (no beliefs about these places)

**Planner state**:
- Goal attempted: AcquireCommodity { commodity: Bread, purpose: SelfConsume }
- Outcome: budget-exhausted (300 expansions, depth 9, 705 candidates)
- Also attempted: AcquireCommodity(Apple), AcquireCommodity(Grain) — both budget-exhausted with same metrics
- S94 commodity-relevance pruning is active but 705 candidates still remain post-filter

**Expected behavior chain**:
1. Generate AcquireCommodity(Grain, SelfConsume)
2. Pick up Grain at current location (Thornwall Village)
3. Eat Grain

**Actual behavior**: Planner exhausted 300-node budget exploring 705 candidates at depth 9 without finding this 2-step plan. Merchant Vara never ate in 1440 ticks despite being at a location with food multiple times.

**Breakpoint**: Planner search — 705 candidates after S94 pruning still overwhelm the 300-expansion budget. The simple pick_up → eat sequence is buried under MoveCargo, Trade, QueueForFacility, Harvest, Travel candidates for all known entities. The planner explores deep (depth 9) but wide (705 candidates per expansion) rather than finding the shallow (depth 2) pick_up → eat solution.
- System: worldwake-ai::search — candidate explosion despite commodity relevance pruning
- Code area: `crates/worldwake-ai/src/search/mod.rs`, `crates/worldwake-ai/src/search/candidates.rs`

**Golden test blueprint**:
- Harness setup: 1-place graph (Village). Village has Mill, Well. Agent with cognitive_profile (max_node_expansions: 300, beam_width: 10). Place 8× Grain at Village. Agent hunger=400, knows "Harvest Grain" recipe, has substitute_preferences (Food: [Grain, Apple, Bread]).
- Tick count: 50
- Primary assertion: Agent should pick_up Grain and eat within 10 ticks
- Failure mode assertion: AcquireCommodity(Grain) budget-exhausts despite Grain being at the agent's location. Agent never eats.
- Regression guard: After fixing candidate explosion, agent reliably acquires local food within budget.

---

#### Damning Moment DM-3: Kael — Geographic Desert + Belief Blindness at tick 518

**Agent state at tick 518**:
- Location: Dusty Trail (e2g0)
- Needs: hunger=~600‰ (estimated from trajectory: hit 750 at tick 769), thirst=750‰ (thirst crossed 750 at tick 518), fatigue=~285‰, bladder=~188‰, dirtiness=~500‰
- Inventory: 20× Coin (never used)
- Known recipes: Harvest Water, Harvest Grain

**Location state**:
- Facilities at Dusty Trail: none
- Resource sources at Dusty Trail: none
- Consumables at Dusty Trail: 20× Coin, ~15× Waste, numerous SocialArtifacts (no food/water)
- Adjacent places: Thornwall Village (2 ticks — Well, Grain), Eldergrove Forest (no direct edge from Trail)

**Agent beliefs about resources**:
- Believed locations: Dusty Trail only (memory of Thornwall Village decayed >454 ticks ago; retention=64)
- Missing beliefs: Thornwall Village, Eldergrove Forest, all other places

**Planner state**:
- Goal attempted: Only Sleep, Relieve, ShareBelief visible in goal set — no AcquireCommodity
- Outcome: No food acquisition attempted because no eat/drink affordances exist at Dusty Trail AND no believed resource locations to plan toward
- No budget-exhaustion failures for AcquireCommodity (planner never generates the goal without affordances or beliefs about targets)

**Expected behavior chain**:
1. Need-driven exploration should trigger: hunger/thirst high + no believed food/water location
2. Generate ExploreLocation targeting adjacent unknown places
3. Travel to Thornwall Village (2 ticks)
4. Observe Well, discover Water source
5. Harvest Water → Drink

**Actual behavior**: Sleep and relieve_wilderness loop for 922 consecutive ticks. No travel, no exploration, no food acquisition attempted.

**Breakpoint**: Kael has NO exploration_profile. Without it, the ExploreLocation goal is never generated. S80 (exploration drive) requires an ExplorationProfile component on the agent. The candidate generation pathway in `crates/worldwake-ai/src/candidate_generation.rs` checks for the profile and skips exploration if absent.
- System: worldwake-ai::candidate_generation — ExploreLocation requires explicit opt-in via ExplorationProfile
- Code area: `crates/worldwake-ai/src/candidate_generation.rs`

**Golden test blueprint**:
- Harness setup: 2-place graph (Trail, Village). Trail has no food/water. Village has Well + Water source + Grain. Agent at Trail with hunger=500, thirst=300, NO exploration_profile, memory_retention_ticks=64. Give agent initial Water (consumed by ~tick 300).
- Tick count: 500
- Primary assertion: After initial supplies are consumed, agent should have a mechanism to travel to Village for food/water
- Failure mode assertion: Agent remains at Trail in sleep+relieve loop for 200+ ticks while hunger/thirst exceed 750‰, never traveling to Village despite it being 2 ticks away.
- Regression guard: Once agents either (a) have exploration profiles by default or (b) have a survival-travel fallback, they should seek resources.

---

#### Damning Moment DM-4: Forager Lina — Structural Impossibility + Exploration Failure at tick 630

**Agent state at tick 630**:
- Location: Eldergrove Forest (e1g0)
- Needs: hunger=~34‰, thirst=~68‰, fatigue=~287‰, bladder=~199‰, dirtiness=750‰ (crossed threshold at tick 630)
- Inventory: empty (cycles through pick_up → eat)
- Known recipes: Harvest Apples

**Location state**:
- Facilities at Eldergrove Forest: ChoppingBlock, OrchardRow (no WashBasin)
- Resource sources: Apple (OrchardRow, regen 2 ticks/unit, capacity 20)
- Consumables: Apples (regenerating), Water (initial 5 items, likely consumed)
- Adjacent places: Thornwall Village (3 ticks — no WashBasin there either), Dusty Trail (2 ticks, one-way — no WashBasin)

**Agent beliefs about resources**:
- Believed locations: Eldergrove Forest only
- Believed resources: ChoppingBlock, OrchardRow, Waste items
- Missing beliefs: Hearthstone Inn (has WashBasin, 7+ ticks away via Thornwall Village)

**Planner state**:
- Goal attempted: only AcquireCommodity(Apple/Water), ConsumeOwnedCommodity, Relieve, Sleep
- ExploreLocation: never appeared in goals-selected despite having exploration_profile
- Exploration profile: curiosity_weight=650, need_activation_threshold=350
- dirtiness_weight in utility_profile: 200 (lowest survival weight)

**Expected behavior chain**:
1. Dirtiness exceeds need_activation_threshold (350) at ~tick 350 (dirtiness grows at rate 1/tick)
2. No wash affordance at current location → ExploreLocation should trigger
3. Travel to Thornwall Village (3 ticks) → observe → no WashBasin
4. Travel to Hearthstone Inn (4 more ticks) → find WashBasin → Wash

**Actual behavior**: Forager Lina never left Eldergrove Forest in 1440 ticks. Continued harvesting apples and sleeping. Dirtiness reached 1000.

**Breakpoint**: Either (a) ExploreLocation candidate generation doesn't trigger for dirtiness as a motivating need, (b) dirtiness_weight (200) is too low to make ExploreLocation competitive with food/sleep goals, or (c) the exploration system only considers hunger/thirst needs, not dirtiness. Since her hunger and thirst were well-managed, the need_activation_threshold (350) may never fire for those needs.
- System: worldwake-ai::candidate_generation — ExploreLocation need triggering may be limited to hunger/thirst
- Code area: `crates/worldwake-ai/src/candidate_generation.rs`, ExploreLocation generation logic

**Golden test blueprint**:
- Harness setup: 2-place graph (Forest, Inn). Forest has OrchardRow + Apple source but no WashBasin. Inn has WashBasin. Agent at Forest with exploration_profile, dirtiness=400, dirtiness_weight=200. Provide ample food.
- Tick count: 300
- Primary assertion: Agent should eventually travel to Inn and wash when dirtiness exceeds 750‰
- Failure mode assertion: Agent never leaves Forest despite critical dirtiness because ExploreLocation doesn't trigger for dirtiness need.
- Regression guard: Exploration should consider all needs, not just hunger/thirst.

## Proposed Solutions

### Geographic Desert

**Solution 1: Add food sources to Dusty Trail or make it a transit-only location**
- **What**: Add a resource source (e.g., Water from a stream) at Dusty Trail, or remove it as a viable long-term location by adding no facilities and ensuring agents treat it as a waypoint
- **Where**: `scenarios/cli-evaluation.ron` — add a resource source or rethink Dusty Trail's purpose
- **FOUNDATIONS alignment**: FND-03 (Maximal Emergence) — agents shouldn't be trapped at barren locations; the place graph should support emergent survival patterns
- **Existing specs**: None specifically address barren location traps
- **Type**: Scenario fix
- **Impact estimate**: DM-1, DM-3 partially (agents still need exploration/memory to return to resource-rich locations)

### Planner Budget Wall

**Solution 2: Further candidate pruning for at-location commodities**
- **What**: When AcquireCommodity targets a commodity that is physically present at the agent's current location, prioritize the simple pick_up → eat/drink sequence. Add a "local commodity shortcut" that generates only the minimal operator sequence before expanding the full candidate tree.
- **Where**: `crates/worldwake-ai/src/search/candidates.rs` — add priority injection for local-pickup candidates
- **FOUNDATIONS alignment**: FND-12 (Compress Computation Not Causality) — optimizing the search without removing valid causal paths
- **Existing specs**: S94 (commodity-relevance-candidate-pruning) implemented but insufficient — 705 candidates remain after pruning. S91 (acquire-commodity-prerequisite-guidance) and S95 (relaxed-plan-heuristic) also attempted to address this. The problem persists because even after pruning irrelevant commodities, too many entity-specific affordances (MoveCargo, Trade, QueueForFacility for each entity) remain.
- **Type**: Engine fix
- **Impact estimate**: DM-2 (Merchant Vara would successfully eat Grain at Thornwall Village)

**Solution 3: Raise Merchant Vara's expansion budget for food acquisition**
- **What**: Increase max_node_expansions from 300 to 500+ or add per-goal-kind budget overrides
- **Where**: `scenarios/cli-evaluation.ron` — Merchant Vara's cognitive_profile, OR engine-level per-goal budget in `crates/worldwake-ai/src/search/mod.rs`
- **FOUNDATIONS alignment**: FND-08 (No Magic Numbers) — budget should be profile-driven, which it is; but 300 is provably insufficient for AcquireCommodity at realistic entity densities
- **Type**: Profile tuning (short-term) / Engine fix (long-term)
- **Impact estimate**: DM-2

### Belief Blindness

**Solution 4: Landmark belief persistence for visited places**
- **What**: Agents who visit a place retain a permanent (or long-decay) belief about its facilities (Well, Mill, OrchardRow, WashBasin). Currently, memory_retention_ticks (64 for Kael) causes complete forgetting, including critical resource locations.
- **Where**: `crates/worldwake-core/src/` (belief/memory system), potentially new component `LandmarkMemory`
- **FOUNDATIONS alignment**: FND-07 (Information Locality) — agents should build mental maps through observation, retaining structural knowledge longer than transient entity observations. FND-10 (Agents Plan From Beliefs) — beliefs about facilities are fundamental planning inputs.
- **Existing specs**: S88 (two-phase-landmark-planning) added `landmark_extraction_depth` to CognitiveProfile. However, this doesn't address belief decay — landmarks are extracted during planning but the underlying facility beliefs still decay normally.
- **Type**: Engine fix
- **Impact estimate**: DM-1, DM-3 (agents would remember Thornwall Village has a Well even after leaving)

**Solution 5: SocialArtifact belief pollution mitigation**
- **What**: SocialArtifacts (notices, bounties) should not count toward entity_memory_capacity. Guard Theron's 487 posted notices flood the belief systems of all co-located agents, displacing survival-relevant beliefs about food and water locations.
- **Where**: `crates/worldwake-sim/src/per_agent_belief_view.rs` or perception system — exclude SocialArtifacts from entity memory capacity counting, or give them a separate, lower-priority memory pool
- **FOUNDATIONS alignment**: FND-07 (Information Locality) — agents should prioritize survival-relevant information in their limited memory. FND-11 (Every Positive Feedback Loop Needs a Physical Dampener) — notice spam creates a runaway memory pollution loop.
- **Existing specs**: S96 (obligation satiation) addresses the production side but not the consumption side (other agents' beliefs being polluted)
- **Type**: Engine fix
- **Impact estimate**: DM-1, DM-3 (belief memories wouldn't be displaced by SocialArtifacts)

### Priority Override

**Solution 6: Add obligation_satiation_profile to Guard Theron**
- **What**: Add `obligation_satiation_profile` with appropriate decay parameters to Guard Theron in the scenario. S96 implemented the satiation mechanism, but Guard Theron lacks the profile to activate it.
- **Where**: `scenarios/cli-evaluation.ron` — Guard Theron's agent definition
- **FOUNDATIONS alignment**: FND-11 (Every Positive Feedback Loop Needs a Physical Dampener) — the S96 dampener exists but is not applied to this agent
- **Existing specs**: S96 (obligation-satiation) implemented and tested, but the scenario doesn't use it for Guard Theron
- **Type**: Scenario fix
- **Impact estimate**: DM-1 (direct fix — obligation satiation would decay PostNotice drive after repeated posting)

**Solution 7: Survival need floor in goal ranking**
- **What**: When any survival need exceeds a critical threshold (e.g., 750‰), no non-survival goal should score higher than the highest survival goal, regardless of weight configuration. This is a safety net independent of obligation satiation.
- **Where**: `crates/worldwake-ai/src/ranking.rs` — add survival floor enforcement after per-goal scoring
- **FOUNDATIONS alignment**: FND-01 (Causal Standard) — agents dying from obligation spam violates the physical simulation's causal logic (an agent would never choose to post notices over eating when starving). FND-11 (Dampener) — this is a systemic dampener preventing any non-survival goal from killing an agent.
- **Existing specs**: No spec addresses a hard survival floor. S96 is per-profile; a floor would be universal.
- **Type**: Engine fix
- **Impact estimate**: DM-1 and any future priority override scenarios

### Structural Impossibility

**Solution 8: Add WashBasin at Thornwall Village or Eldergrove Forest**
- **What**: Add a WashBasin facility at Thornwall Village (the hub location). Currently the only WashBasin is at Hearthstone Inn, which is 4 ticks from Village and unreachable without specific knowledge.
- **Where**: `scenarios/cli-evaluation.ron` — add `(workstation: WashBasin, location: "Thornwall Village")` or `"Eldergrove Forest"`
- **FOUNDATIONS alignment**: FND-03 (Maximal Emergence) — agents should have realistic paths to satisfy all needs
- **Type**: Scenario fix
- **Impact estimate**: DM-4 partially (Forager Lina still needs to travel, but Village is 3 ticks away vs 7+ for Inn)

### Exploration Failure

**Solution 9: Make ExploreLocation trigger for all needs, not just hunger/thirst**
- **What**: Verify and fix that ExploreLocation candidate generation considers all unsatisfied needs (including dirtiness, fatigue, bladder, pain) when determining whether to explore, not just food/water needs.
- **Where**: `crates/worldwake-ai/src/candidate_generation.rs` — ExploreLocation generation logic
- **FOUNDATIONS alignment**: FND-03 (Maximal Emergence), FND-04 (Agent Diversity) — exploration should respond to any unmet need, not be hardcoded to specific need types
- **Existing specs**: S80 (exploration-drive) defines ExploreLocation as need-driven but may only implement hunger/thirst as motivating needs
- **Type**: Engine fix
- **Impact estimate**: DM-4 (Forager Lina would explore to find WashBasin)

**Solution 10: Default exploration profile for all AI agents**
- **What**: Make ExplorationProfile a universal profile with defaults (like PerceptionProfile), rather than opt-in. Agents without explicit exploration profiles would get conservative defaults (low curiosity_weight, moderate need_activation_threshold).
- **Where**: `crates/worldwake-core/src/exploration.rs` (Default impl), `crates/worldwake-cli/src/scenario/mod.rs` (spawn_agent), `crates/worldwake-core/src/component_schema.rs`
- **FOUNDATIONS alignment**: FND-04 (Agent Diversity) — all agents should have survival-level exploration capability. FND-10 (Agents Plan From Beliefs) — agents need a mechanism to acquire new beliefs when current beliefs are insufficient for survival.
- **Existing specs**: S80 registers ExplorationProfile as a universal profile with defaults, but CLAUDE.md critical invariant says "Universal profiles are always applied with defaults." Need to verify S80's implementation actually does this.
- **Type**: Engine fix
- **Impact estimate**: DM-3 (Kael would explore to find resources), DM-4 (if exploration triggers for dirtiness)

## Golden Test Recommendations

| Priority | Damning Moment | Test Name Suggestion | What It Guards Against |
|----------|---------------|---------------------|----------------------|
| 1 | DM-1 | golden_obligation_spam_kills_agent | Agent dies from PostNotice priority override — obligation spam prevents survival actions when no satiation profile present |
| 2 | DM-2 | golden_local_food_budget_exhaustion | AcquireCommodity budget-exhausts for food physically at agent's location — planner can't find pick_up→eat in 300 expansions |
| 3 | DM-3 | golden_stranded_agent_no_exploration | Agent without exploration_profile starves at barren location 2 ticks from resources — no mechanism to rediscover forgotten locations |
| 4 | DM-4 | golden_exploration_ignores_dirtiness | Agent with exploration_profile never explores for WashBasin despite critical dirtiness — ExploreLocation may not trigger for non-food needs |

## Outcome

- **Completion date**: 2026-04-13
- **What changed**: Diagnostic identified 4 damning moments (DM-1 through DM-4) across all 4 agents, classified root causes into 5 categories (Geographic Desert, Planner Budget Wall, Belief Blindness, Priority Override, Structural Impossibility/Exploration Failure), and proposed 10 concrete solutions with golden test blueprints for each.
- **Deviations**: None — report produced as designed by the needs-starvation-diagnostic skill.
- **Verification**: Findings exploited for subsequent remediation work; golden test blueprints informed spec and ticket creation.
