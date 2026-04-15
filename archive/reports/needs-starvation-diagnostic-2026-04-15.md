**Status**: ✅ COMPLETED

# Needs Starvation Diagnostic

## Run Summary
- **Scenario**: `scenarios/cli-evaluation.ron`
- **Seed**: 7777
- **Ticks simulated**: 1440
- **Agents**: Kael (AI), Merchant Vara (AI), Forager Lina (AI), Guard Theron (AI)
- **Places**: Thornwall Village, Eldergrove Forest, Dusty Trail, Hearthstone Inn, Golden Fields
- **Deaths**: Guard Theron at tick 769 (cause: NeedDeprivation { Hunger })

## Agent Needs Overview

| Agent | Need | Max Value | Ticks >750 | Death? | Root Cause Category |
|-------|------|-----------|------------|--------|---------------------|
| Guard Theron | Hunger | 1000 | 1221 (ticks 219-1439) | Yes (tick 769) | Knowledge Gap + Priority Override + Profile Gap |
| Kael | Hunger | 1000 | 683 (ticks 757-1439) | No | Geographic Desert + Belief Blindness + Planner Budget Wall |
| Forager Lina | Dirtiness | 1000 | 810 (ticks 630-1439) | No | Geographic Desert + Belief Blindness + Profile Gap |
| Merchant Vara | Thirst | 1000 | 305 (ticks 476-780) | No | Geographic Desert (transient, self-resolved) |
| Merchant Vara | Dirtiness | 899 | 147 (ticks 646-792) | No | Geographic Desert (transient, self-resolved) |

## Failure Classifications

### Guard Theron
**Categories**: Knowledge Gap, Priority Override, Profile Gap
**Evidence**:
- `known_recipes: ["Harvest Water"]` -- no food recipes whatsoever
- Goals selected show ZERO food-related goals: no AcquireCommodity for Grain/Bread/Apple, no ConsumeOwnedCommodity for food
- Anomaly 34 explicitly flags: "hunger avg 917 but no relief action (eat) was ever attempted"
- Goals dominated by InvestigateViolation (9), PostNotice (21+), Patrol (2), ShareBelief, Sleep -- duty goals outranked survival even at hunger=1000
- No `exploration_profile` to discover food sources; no `metabolism_profile` to tune starvation tolerance
- Stuck for 670 consecutive ticks after tick 400 (behavioral transition: repertoire narrowed to 3 types)
**Confidence**: HIGH
**Causal chain**: No food recipes -> planner never generates food acquisition goals -> duty goals fill all decision slots -> hunger rises unchecked -> starvation death at tick 769

### Kael
**Categories**: Geographic Desert, Belief Blindness, Planner Budget Wall
**Evidence**:
- Spent 1235 of 1440 ticks at Dusty Trail, which has zero food sources, no facilities, no resource sources
- Only 5 eat actions committed (consumed initial 5x Bread at Thornwall Village early)
- Knows "Harvest Grain" but FieldPlot is only at Golden Fields -- Kael never visited Golden Fields and has no beliefs about it
- ProduceCommodity(RecipeId(2)) budget-exhausted at ticks 424, 787, 1230, 1259, 1272 (224 expansions, 583-1652 candidates each time)
- End-state beliefs only include Thornwall Village and Eldergrove Forest; Golden Fields and Hearthstone Inn unknown
- At tick 1230: hunger=1000, inventory=20x Coin (no food), beliefs show no food items anywhere
**Confidence**: HIGH
**Causal chain**: Migrated to Dusty Trail early -> consumed initial Bread stock -> Harvest Grain needs FieldPlot at unknown Golden Fields -> ProduceCommodity budget-exhausts repeatedly -> no food production path available -> hunger saturates at 1000 from tick 757

### Forager Lina
**Categories**: Geographic Desert, Belief Blindness, Profile Gap
**Evidence**:
- WashBasin exists only at Hearthstone Inn; Forager Lina spent 1252 ticks at Eldergrove Forest (no WashBasin)
- 0 wash actions committed in entire simulation
- ExploreLocation goals generated (Village e0g0 and Trail e2g0, motivating_need=Dirtiness) but never targeted Hearthstone Inn
- End-state beliefs only include Eldergrove Forest; no knowledge of Hearthstone Inn or WashBasin
- No `perception_profile` -> only 216 observations, 137 passed, 25 unique entities (vs Kael's 444/427/73)
- No `cognitive_profile` -> uses default planner budget
- Dirtiness reached 1000 and stayed above 750 for 810 consecutive ticks
**Confidence**: HIGH
**Causal chain**: No WashBasin at Forest -> ExploreLocation fires for Dirtiness but targets known-adjacent places (Village, Trail) which also lack WashBasin -> never discovers Hearthstone Inn -> dirtiness rises unchecked

### Merchant Vara
**Categories**: Geographic Desert (transient)
**Evidence**:
- Thirst >750 for 305 ticks (476-780) while primarily at Dusty Trail (928 total ticks at Trail)
- Eventually resolved: 11 drink, 6 harvest water by end of simulation
- Knows "Harvest Water" recipe and traveled to Village (143 ticks) and Forest (353 ticks) to satisfy thirst
- Budget-exhausted AcquireCommodity(Bread/Apple) at tick 32 (300 expansions, 693 candidates) but food was less critical
**Confidence**: MEDIUM (transient issue, self-resolved through travel)
**Causal chain**: Migrated to Dusty Trail (no water source) -> thirst rose during Trail residence -> eventually traveled to Village/Forest for water -> resolved

## Damning Moments

#### Damning Moment DM-1: Guard Theron -- Knowledge Gap at tick 219

**Agent state at tick 219**:
- Location: Dusty Trail (e2g0)
- Needs: hunger=750 (crossing threshold), thirst~150, fatigue~300, bladder~150, dirtiness~140
- Inventory: 1x Bow, 1x Sword
- Known recipes: ["Harvest Water"] -- NO food recipes

**Location state**:
- Facilities at Dusty Trail: none
- Resource sources at Dusty Trail: none
- Consumables at Dusty Trail: none relevant to hunger (Waste, SocialArtifacts, Coins)
- Adjacent places: Thornwall Village (2 ticks), Eldergrove Forest (via Village, 5+ ticks)

**Agent beliefs about resources**:
- Believed locations: Thornwall Village, Dusty Trail
- Believed resources: Thornwall Village has Mill, Loom, Well; Dusty Trail has various items/waste
- Missing beliefs: Eldergrove Forest (food), Golden Fields (FieldPlot for Grain), Hearthstone Inn

**Planner state**:
- Goal attempted: No food goal was ever generated (the planner never generated AcquireCommodity for any food)
- Outcome: never attempted -- the candidate generation system produces no food-related goals because the agent has no food recipes and no food in inventory
- Competing goals: InvestigateViolation, Patrol, PostNotice, ShareBelief, Sleep dominated all decision ticks

**Expected behavior chain**:
1. CandidateGeneration should produce AcquireCommodity(Grain/Apple/Bread, SelfConsume) when hunger is high
2. Travel to Thornwall Village (2 ticks) to pick up available Grain (10x) or Bread (5x)
3. Eat the picked-up food item

**Actual behavior**: Patrol, investigate violations, post notices, share beliefs, sleep -- zero food-seeking behavior for 550+ ticks until death

**Breakpoint**: Candidate generation never produces food acquisition goals for an agent without food recipes and without food in inventory. The system requires either known recipes for harvestable commodities or existing food items in reachable locations to generate food goals. Guard Theron has neither -- he only knows "Harvest Water" and has no food items.
- System: GOAP candidate generation -- `generate_candidates` in worldwake-ai
- Code area: worldwake-ai::candidate_generation (AcquireCommodity goal generation requires recipe match or item availability)
- Secondary: worldwake-ai::goal_ranking -- duty goals (investigate, patrol, post_notice) should not outrank critical survival needs

**Golden test blueprint**:
- Harness setup: 2 places (Village with Grain items, Trail with no food), 1 agent at Trail with hunger=700, known_recipes=["Harvest Water"] only, utility_profile with hunger_weight=400, patrol_profile + patrol_route to Trail
- Tick count: 100
- Primary assertion: agent either acquires food by picking up ground items OR travels to Village to pick up Grain within 50 ticks
- Failure mode assertion: agent never commits an eat action; only patrols, posts notices, and sleeps while hunger rises to 1000
- Regression guard: ensures agents with no food recipes can still pick up and consume available food items at reachable locations

#### Damning Moment DM-2: Kael -- Geographic Desert + Belief Blindness at tick 757

**Agent state at tick 757**:
- Location: Dusty Trail (e2g0)
- Needs: hunger=750 (crossing threshold), thirst~200, fatigue~300, bladder~250, dirtiness~30
- Inventory: 20x Coin (no food)
- Known recipes: ["Harvest Water", "Harvest Grain"]

**Location state**:
- Facilities at Dusty Trail: none
- Resource sources at Dusty Trail: none
- Consumables at Dusty Trail: Waste, SocialArtifacts, Coins -- no food
- Adjacent places: Thornwall Village (2 ticks -- has Grain and Well but no FieldPlot)

**Agent beliefs about resources**:
- Believed locations: Thornwall Village, Eldergrove Forest, Dusty Trail
- Believed resources: Thornwall Village has Mill, Loom, Well; Eldergrove Forest has OrchardRow, ChoppingBlock
- Missing beliefs: Golden Fields (has FieldPlot for Harvest Grain), Hearthstone Inn

**Planner state**:
- Goal attempted: ProduceCommodity(RecipeId(2)) -- Harvest Grain
- Outcome: budget-exhausted (224 expansions, 583-1652 candidates) -- at ticks 424, 787, 1230, 1259, 1272
- Candidates: high count indicates planner explores many paths but can't reach FieldPlot
- Competing goals: ShareBelief, Sleep dominate; AcquireCommodity(Bread) also selected but Bread is exhausted

**Expected behavior chain**:
1. ExploreLocation to discover Golden Fields (which has FieldPlot)
2. Travel to Golden Fields (5 ticks from Village)
3. Harvest Grain at FieldPlot
4. Eat Grain (or travel to Village to Bake Bread at Mill)

**Actual behavior**: Repeatedly budget-exhausts on ProduceCommodity(Harvest Grain) because FieldPlot is at Golden Fields which Kael doesn't know about. Falls back to sleep + tell + relieve at Dusty Trail. Occasionally travels to Village for water but never explores beyond known locations.

**Breakpoint**: Kael has no `exploration_profile` so ExploreLocation goals are never generated. He knows the Harvest Grain recipe but the required facility (FieldPlot) is at Golden Fields which he has never visited and has no beliefs about. The planner budget-exhausts trying to plan Harvest Grain from Trail/Village because it can't find a FieldPlot in believed locations.
- System: exploration system (missing profile) + GOAP planner (budget wall on multi-hop resource acquisition)
- Code area: worldwake-ai::candidate_generation (no ExploreLocation for agents without exploration_profile), worldwake-ai::search (budget exhaustion on deep food plans)

**Golden test blueprint**:
- Harness setup: 3 places (Trail, Village with Well+Mill, Fields with FieldPlot), agent at Trail with hunger=700, known_recipes=["Harvest Water", "Harvest Grain"], initial beliefs about Village only (not Fields), no exploration_profile
- Tick count: 200
- Primary assertion: agent travels to Village and either picks up available Grain or discovers path to FieldPlot within 100 ticks
- Failure mode assertion: agent budget-exhausts on ProduceCommodity repeatedly, never acquires food, hunger reaches 1000
- Regression guard: ensures agents with food recipes can locate required facilities when beliefs are incomplete

#### Damning Moment DM-3: Forager Lina -- Geographic Desert + Belief Blindness at tick 630

**Agent state at tick 630**:
- Location: Eldergrove Forest (e1g0)
- Needs: hunger~37, thirst~79, fatigue~288, bladder~211, dirtiness=750 (crossing threshold)
- Inventory: Apple (varying)
- Known recipes: ["Harvest Apples"]

**Location state**:
- Facilities at Eldergrove Forest: ChoppingBlock, OrchardRow -- NO WashBasin
- Resource sources at Eldergrove Forest: Apple (OrchardRow, regen 2 ticks, cap 20)
- Consumables at Eldergrove Forest: Apples, Water (initial 5, depleted over time)
- Adjacent places: Thornwall Village (3 ticks -- no WashBasin), Dusty Trail (2 ticks one-way -- no WashBasin)

**Agent beliefs about resources**:
- Believed locations: Eldergrove Forest only (end-state)
- Believed resources: ChoppingBlock, OrchardRow, Apples, Waste at Forest
- Missing beliefs: Hearthstone Inn (has WashBasin), Thornwall Village (no WashBasin either), Golden Fields

**Planner state**:
- Goal attempted: ExploreLocation(target=Village e0g0, motivating_need=Dirtiness) and ExploreLocation(target=Trail e2g0, motivating_need=Dirtiness)
- Outcome: ExploreLocation goals executed (29 travel actions committed), but target locations also lack WashBasin
- AcquireCommodity(Water) budget-exhausted at tick 835 (snapshot 8: 224 expansions, 987 candidates)

**Expected behavior chain**:
1. ExploreLocation targeting Hearthstone Inn (which has WashBasin)
2. Travel to Hearthstone Inn via Thornwall Village (3 + 4 = 7 ticks)
3. Wash at WashBasin

**Actual behavior**: ExploreLocation correctly fires for Dirtiness but targets Village (e0g0) and Trail (e2g0) -- neither has a WashBasin. Agent explores known-adjacent places but never discovers Hearthstone Inn. Dirtiness rises to 1000 and stays there.

**Breakpoint**: ExploreLocation targets are limited to places the agent has beliefs about or can infer from adjacency. Hearthstone Inn is 2 hops away from Forest (Forest -> Village -> Inn) and the agent has no beliefs about it. The exploration system explores adjacent known places but doesn't chain multi-hop exploration to discover distant facilities.
- System: exploration target selection -- ExploreLocation only targets places with existing beliefs or direct adjacency
- Code area: worldwake-ai::candidate_generation (ExploreLocation target selection), worldwake-systems or worldwake-ai exploration module
- Secondary: no `perception_profile` limits entity observation capacity (only 25 unique entities seen), reducing chance of learning about distant locations from other agents' tell actions

**Golden test blueprint**:
- Harness setup: 3 places (Forest with OrchardRow, Village as hub, Inn with WashBasin), agent at Forest with dirtiness=600, exploration_profile with need_activation_threshold=350, no initial beliefs about Inn
- Tick count: 300
- Primary assertion: agent discovers Hearthstone Inn via multi-hop exploration and washes within 200 ticks
- Failure mode assertion: agent explores Village and Trail repeatedly but never reaches Inn; dirtiness reaches 1000
- Regression guard: ensures exploration can chain through intermediate places to find distant facilities matching a need

## Proposed Solutions

### Knowledge Gap

**Solution KG-1: Add food recipes to Guard Theron in scenario**
- **What**: Add `"Harvest Grain"` (or `"Harvest Apples"`) to Guard Theron's `known_recipes`
- **Where**: `scenarios/cli-evaluation.ron`, Guard Theron agent definition
- **FOUNDATIONS alignment**: FND-07 (Information Locality) -- agents should have minimum viable knowledge for survival
- **Existing specs**: No spec explicitly addresses minimum recipe coverage for agent survival
- **Type**: Scenario fix
- **Impact estimate**: Addresses DM-1 partially (Theron could attempt food harvesting, but still needs facility access)

**Solution KG-2: Add pickup-and-eat as a recipe-free food acquisition path**
- **What**: Ensure AcquireCommodity(Food) candidate generation considers picking up ground food items even without harvest recipes. Currently candidate generation seems to require a recipe to produce food; agents should also consider picking up existing food items at their location or reachable locations.
- **Where**: worldwake-ai::candidate_generation -- AcquireCommodity goal generation
- **FOUNDATIONS alignment**: FND-01 (Causal Standard) -- picking up ground food is a causally valid action that doesn't require recipe knowledge
- **Existing specs**: Check if `AcquireCommodity` goal already handles ground-item pickup vs harvest-only paths
- **Type**: Engine fix
- **Impact estimate**: Addresses DM-1 (Theron could pick up Grain/Bread at Village without recipes)

### Geographic Desert

**Solution GD-1: Add food resource source or items to Dusty Trail**
- **What**: Add a food source (e.g., Berry Bush, small Apple tree) or initial food items at Dusty Trail so agents stationed there can eat
- **Where**: `scenarios/cli-evaluation.ron`, items/facilities/resource_sources
- **FOUNDATIONS alignment**: FND-03 (World Dynamics) -- locations should have minimum resource diversity for sustained habitation
- **Type**: Scenario fix
- **Impact estimate**: Addresses DM-1 and DM-2 (agents at Trail can access food locally)

**Solution GD-2: Add WashBasin to Eldergrove Forest or Thornwall Village**
- **What**: Add a WashBasin (or water feature usable for washing) to Forest or Village so agents don't need to travel 7+ ticks to Inn
- **Where**: `scenarios/cli-evaluation.ron`, facilities section
- **FOUNDATIONS alignment**: FND-03 -- hygiene facilities should be reachable within reasonable travel distances
- **Type**: Scenario fix
- **Impact estimate**: Addresses DM-3 (Lina can wash locally)

### Belief Blindness

**Solution BB-1: Grant initial common-knowledge beliefs about all places**
- **What**: Agents should start with beliefs that all named places exist (even if they don't know exact contents), enabling ExploreLocation to target unknown locations
- **Where**: worldwake-sim or worldwake-systems -- agent initialization / belief bootstrapping
- **FOUNDATIONS alignment**: FND-07 (Information Locality) -- agents in a small world would know about major landmarks even without visiting. FND-10 (Belief-only planning) -- beliefs must be sufficient for basic survival planning
- **Existing specs**: Check S80 (Exploration) and S101 (Activation-based belief decay) for interaction with initial beliefs
- **Type**: Engine fix or scenario fix (add initial beliefs to agent definitions)
- **Impact estimate**: Addresses DM-2 and DM-3 (agents could target Golden Fields and Hearthstone Inn for exploration)

**Solution BB-2: Add "common knowledge" landmark beliefs to scenario agent definitions**
- **What**: Add initial believed_locations for all agents covering the 5 places in the scenario (agents know the world has a Village, Forest, Trail, Inn, and Fields even if they haven't visited)
- **Where**: `scenarios/cli-evaluation.ron`, agent definitions (if scenario format supports initial beliefs)
- **FOUNDATIONS alignment**: FND-07 -- reasonable starting knowledge for a small community
- **Type**: Scenario fix
- **Impact estimate**: Addresses DM-2 and DM-3

### Planner Budget Wall

**Solution PBW-1: Add cognitive_profile to Kael**
- **What**: Give Kael a `cognitive_profile` with appropriate budget parameters (similar to Merchant Vara's) to improve planner reach
- **Where**: `scenarios/cli-evaluation.ron`, Kael agent definition
- **FOUNDATIONS alignment**: FND-12 (System Decoupling) -- planner budget is a per-agent parameter, not a global constant
- **Type**: Scenario fix (profile tuning)
- **Impact estimate**: Partially addresses DM-2 (higher budget won't help if Golden Fields is unknown, but could find alternative food paths)

**Solution PBW-2: Need-directed candidate pruning**
- **What**: When planning for a survival need at critical levels, prune candidates unrelated to need satisfaction before search to reduce wasted budget
- **Where**: worldwake-ai::search or worldwake-ai::candidate_generation
- **FOUNDATIONS alignment**: FND-01 (Causal Standard) -- focus planner effort on causally relevant actions
- **Existing specs**: Check S88 (GOAP overhaul), S90 for existing pruning mechanisms
- **Type**: Engine fix
- **Impact estimate**: Reduces budget exhaustion for survival-critical goals across all agents

### Priority Override

**Solution PO-1: Survival-need escalation in goal ranking**
- **What**: When a survival need (hunger/thirst) is at critical level (>750), apply an escalating bonus to survival goals that grows with need severity, eventually outranking any non-survival goal
- **Where**: worldwake-ai::goal_ranking or worldwake-ai::candidate_generation
- **FOUNDATIONS alignment**: FND-01 -- survival is causally prior to duty performance; a dead guard can't patrol
- **Existing specs**: Check S96 (Obligation Satiation) for existing need-priority mechanisms
- **Type**: Engine fix
- **Impact estimate**: Addresses DM-1 secondary cause (even without food recipes, escalating hunger should eventually override patrol/investigate/post_notice goals and trigger alternative food-seeking behavior)

### Profile Gap

**Solution PG-1: Add exploration_profile to Kael and Guard Theron**
- **What**: Both agents lack `exploration_profile`, preventing ExploreLocation goals from being generated. Adding it would allow them to discover unknown resource-rich locations.
- **Where**: `scenarios/cli-evaluation.ron`, agent definitions
- **FOUNDATIONS alignment**: FND-07 -- agents should be able to discover information through exploration
- **Type**: Scenario fix
- **Impact estimate**: Addresses DM-1 (Theron could discover food) and DM-2 (Kael could discover Golden Fields)

**Solution PG-2: Add perception_profile to Forager Lina**
- **What**: Lina lacks `perception_profile`, resulting in very limited observation (25 unique entities vs Kael's 73). Adding it would improve her ability to learn about resources and locations from other agents or environment.
- **Where**: `scenarios/cli-evaluation.ron`, Forager Lina agent definition
- **FOUNDATIONS alignment**: FND-07 -- perception is fundamental to information locality
- **Type**: Scenario fix
- **Impact estimate**: Partially addresses DM-3 (better perception could let Lina learn about Inn from other agents' tells)

**Solution PG-3: Add metabolism_profile to Guard Theron**
- **What**: Theron uses default metabolism rates. An explicit profile would allow tuning starvation_tolerance_ticks to match his duty-focused lifestyle (longer tolerance) or faster hunger_rate to make the problem more urgent for the planner.
- **Where**: `scenarios/cli-evaluation.ron`, Guard Theron agent definition
- **FOUNDATIONS alignment**: FND-12 -- per-agent parameters over global defaults
- **Type**: Scenario fix (profile tuning)
- **Impact estimate**: Does not fix root cause (Knowledge Gap) but could extend survival window

### Belief Memory Pollution

**Solution BMP-1: Partition belief memory by entity type**
- **What**: End-state beliefs for Kael show 48x Waste and 22+ SocialArtifacts at Dusty Trail, crowding out potentially useful entity beliefs. Partitioning memory so Waste/SocialArtifact entities don't displace facility/resource/food beliefs would preserve survival-critical knowledge.
- **Where**: worldwake-core or worldwake-ai belief memory management
- **FOUNDATIONS alignment**: FND-07 -- information locality should prioritize survival-relevant knowledge
- **Existing specs**: Check S101 (activation-based belief decay) for existing memory management
- **Type**: Engine fix
- **Impact estimate**: Partially addresses DM-2 (preserves food-location beliefs that might be displaced by Waste observations)

## Golden Test Recommendations

| Priority | Damning Moment | Test Name Suggestion | What It Guards Against |
|----------|---------------|---------------------|----------------------|
| 1 | DM-1 | golden_guard_starves_without_food_recipes | Agent with no food recipes starving to death despite food items existing at reachable locations |
| 2 | DM-2 | golden_agent_hunger_at_barren_location_with_distant_food | Agent with food recipe unable to locate required facility due to belief blindness and no exploration profile |
| 3 | DM-3 | golden_forager_dirtiness_no_washbasin_reachable | Agent with dirtiness at critical levels unable to discover WashBasin at multi-hop distant location |
