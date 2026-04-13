# Needs Starvation Diagnostic

## Run Summary
- **Scenario**: `scenarios/cli-evaluation.ron`
- **Seed**: 7777
- **Ticks simulated**: 543 (observer errored at tick 543 due to `PreconditionFailed("violation 12 is no longer active at commit")` in `investigate_actions.rs:179` — partial dump used)
- **Agents**: Kael (e5g0), Merchant Vara (e6g0), Forager Lina (e7g0), Guard Theron (e8g0)
- **Places**: Thornwall Village (e0g0), Eldergrove Forest (e1g0), Dusty Trail (e2g0), Hearthstone Inn (e3g0), Golden Fields (e4g0)
- **Deaths**: None
- **Observer fix applied**: Changed observer `Err` handler from `std::process::exit(1)` to `break` so partial dumps are written on tick errors.

## Agent Needs Overview

| Agent | Need | Max Value | Ticks >750‰ | Death? | Root Cause Category |
|-------|------|-----------|-------------|--------|---------------------|
| Merchant Vara | Hunger | 1000‰ | 274 | No | Planner Budget Wall |
| Merchant Vara | Bladder | 1000‰ | 84 | No | (secondary to hunger starvation) |
| Merchant Vara | Dirtiness | 1000‰ | 79 | No | (secondary to hunger starvation) |
| Merchant Vara | Fatigue | 841‰ | 32 | No | (secondary to hunger starvation) |
| Forager Lina | Thirst | 1000‰ | 137 | No | Geographic Desert + Knowledge Gap + Profile Gap |
| Forager Lina | Bladder | 1000‰ | 84 | No | (secondary to thirst starvation) |
| Forager Lina | Fatigue | 836‰ | 43 | No | (secondary to thirst starvation) |
| Forager Lina | Dirtiness | 1000‰ | 6 | No | (secondary — no WashBasin at any visited location) |
| Kael | (all) | — | 0 | No | Healthy — all needs managed |
| Guard Theron | (all) | — | 0 | No | Healthy — all needs managed |

## Failure Classifications

### Merchant Vara
**Categories**: Planner Budget Wall
**Confidence**: HIGH

**Evidence**:
- 156 out of 366 plan searches budget-exhausted (42.6%)
- Every AcquireCommodity attempt for food (Bread, Apple, Grain) budget-exhausted: 300 expansions used, 693 candidates generated, max depth 9
- Zero eat actions in 543 ticks despite food items being physically present at her location
- At tick 35 (first budget-exhaustion snapshot): Merchant Vara is at Dusty Trail with 3× Bread on the ground. She believes Bread is there. But AcquireCommodity(Bread) exhausts the 300-expansion budget
- Her `cognitive_profile.max_node_expansions = 300` and `max_candidates_per_expansion = 150` create a search space that explodes before finding even a 2-step plan (pick_up → eat)
- Her final affordances at tick 543 show `pick_up` (6 targets) but no `eat` — she has no food in inventory because she never picks it up because the planner can't complete the search

**Causal chain**: High `max_candidates_per_expansion` (150) → 693 candidate operators generated → search branches explode at depth 9 → 300-expansion budget exhausted before finding pick_up→eat path → hunger accumulates unchecked → hunger reaches 1000‰ at tick 269 and stays there for 274+ ticks → all other needs cascade (bladder, dirtiness, fatigue) as the agent's behavior narrows to just water harvesting, travel, and tell

### Forager Lina
**Categories**: Geographic Desert, Knowledge Gap, Profile Gap
**Confidence**: HIGH

**Evidence**:

*Geographic Desert*:
- Started at Eldergrove Forest: has OrchardRow (apples) and ChoppingBlock but NO Well, NO water resource source
- Consumed initial 5× Water placed at Eldergrove Forest (5 drink actions committed)
- Traveled to Dusty Trail at tick 248: NO facilities, NO resource sources, NO water
- Neither location she visited has water production capability

*Knowledge Gap*:
- Known recipes: only `["Harvest Apples"]` — does NOT know `"Harvest Water"`
- Even if she traveled to Thornwall Village (where the Well is), she couldn't harvest water
- She knows the Well exists at Thornwall Village (from beliefs) but lacks the recipe to use it

*Profile Gap*:
- Forager Lina has NO `perception_profile` in the scenario
- Uses engine defaults which may severely limit observation capacity
- At Dusty Trail end-state, her beliefs show agents only (Kael, Guard Theron) — NO items despite 3× Grain, 20× Coin, 22× Waste, etc. on the ground
- Final affordances at tick 543: NO pick_up, NO eat, NO drink — she can't even see the items at her feet
- Compare to Kael and Guard Theron (both have `perception_profile`) who successfully observe and interact with items

**Causal chain**: No Harvest Water recipe → can only drink pre-placed Water → 5 units consumed by ~tick 100 → thirst_rate=5 (fastest in scenario) drains rapidly → travels to Dusty Trail at tick 248 (no water there either) → no perception_profile → can't observe ground items at Dusty Trail → thirst exceeds 750‰ at tick 406 → 217 consecutive idle ticks (completely stuck) → AcquireCommodity(Water) frontier-exhausts with only 2 candidates at tick 542

## Damning Moments

#### Damning Moment DM-1: Merchant Vara — Planner Budget Wall at tick 35

**Agent state at tick 35**:
- Location: Dusty Trail (e2g0)
- Needs: hunger=278‰, thirst=54‰, fatigue=182‰, bladder=32‰, dirtiness=138‰
- Inventory: 1× Water
- Known recipes: Harvest Water, Harvest Grain, Harvest Apples, Bake Bread

**Location state**:
- Facilities at Dusty Trail: none
- Resource sources at Dusty Trail: none
- Consumables at Dusty Trail: 3× Bread, 5× Water, 20× Coin
- Adjacent places: Thornwall Village (2 ticks), Eldergrove Forest (via one-way from forest only — not reachable from trail)

**Agent beliefs about resources**:
- Believed locations: Thornwall Village, Dusty Trail
- Believed at Dusty Trail: Kael, Merchant Vara, 4× Water, 3× Bread, 1× Waste
- Believed at Thornwall Village: Guard Theron, Mill, Loom, Well
- Missing beliefs: Eldergrove Forest (OrchardRow, Apples), Hearthstone Inn, Golden Fields

**Planner state**:
- Goal attempted: AcquireCommodity { commodity: Bread, purpose: SelfConsume }
- Outcome: budget-exhausted
- Candidates: 693, Depth: 9, Expansions: 300/300
- Competing goals: n/a — food acquisition was highest-priority but couldn't find a plan

**Expected behavior chain**:
1. Goal: AcquireCommodity(Bread, SelfConsume)
2. Action: pick_up(Bread) — Bread is on ground at Dusty Trail
3. Action: eat(Bread) — from inventory
4. Result: hunger reduced

**Actual behavior**: Harvest Water loop + relieve_wilderness + tell (the only goals that successfully find plans). Hunger climbs unchecked to 1000‰.

**Breakpoint**: GOAP search generates 693 candidate operators for a simple 2-step plan. With `max_candidates_per_expansion=150` and `beam_width=10`, the search tree explodes across 9 depth levels, exhausting the 300-expansion budget before reaching the pick_up→eat terminal state.
- System: GOAP planner search — candidate explosion
- Code area: `worldwake-ai::search` (search space explosion), `worldwake-ai::candidate_generation` (unbounded candidate count per expansion)

**Golden test blueprint**:
- Harness setup: Single place, one agent with `cognitive_profile` matching Vara's (max_node_expansions=300, max_candidates_per_expansion=150), 1× Bread on ground, agent knows recipe. Agent has hunger=500.
- Tick count: 50
- Primary assertion: agent picks up Bread and eats it within 20 ticks
- Failure mode assertion: AcquireCommodity(Bread) budget-exhausts; agent never eats; hunger increases monotonically
- Regression guard: ensures simple pick_up→eat plans are found within planner budget even with high candidate generation settings

#### Damning Moment DM-2: Forager Lina — Geographic Desert + Knowledge Gap at tick 406

**Agent state at tick ~406**:
- Location: Dusty Trail (e2g0) (arrived tick 248)
- Needs: hunger=~300‰ (estimated from avg), thirst=750‰ (crossing threshold), fatigue=~400‰, bladder=~400‰, dirtiness=~350‰
- Inventory: empty
- Known recipes: Harvest Apples (only)

**Location state**:
- Facilities at Dusty Trail: none
- Resource sources at Dusty Trail: none
- Consumables at Dusty Trail: various items (Grain, Water, Coin, etc.) — but Lina can't see them (no perception_profile)
- Adjacent places: Thornwall Village (2 ticks — has Well + Water resource source)

**Agent beliefs about resources**:
- Believed locations: Thornwall Village, Eldergrove Forest, Dusty Trail
- Believed at Thornwall Village: Merchant Vara, Mill, Loom, Well
- Believed at Eldergrove Forest: ChoppingBlock, OrchardRow
- Believed at Dusty Trail: Kael, Forager Lina, Guard Theron (agents only — no items)
- Missing beliefs: Items on ground at Dusty Trail, Hearthstone Inn, Golden Fields

**Planner state**:
- Goal attempted: AcquireCommodity(Water, SelfConsume)
- Outcome: frontier-exhausted at depth 1 with 2 candidates (tick 542)
- Candidates: 2, Depth: 1, Expansions: 2
- Competing goals: none succeeding — stuck in idle

**Expected behavior chain**:
1. Goal: AcquireCommodity(Water, SelfConsume)
2. Action: travel to Thornwall Village (2 ticks)
3. Action: harvest Water at Village Well
4. Action: drink Water

**Actual behavior**: 217 consecutive idle ticks. AcquireCommodity(Water) frontier-exhausts because: (a) no Water observable at current location (no perception → no pick_up candidates), (b) agent knows Well exists at Thornwall Village but doesn't know "Harvest Water" recipe, so planner can't plan travel→harvest→drink chain.

**Breakpoint**: Two concurrent failures:
1. **Recipe knowledge gap**: Lina knows only "Harvest Apples", not "Harvest Water". Even if she traveled to the Well, she couldn't use it.
2. **No perception_profile**: Without a perception_profile, Lina can't observe items on the ground at Dusty Trail (Water, Grain exist there). If she could perceive them, she might pick_up and drink/eat.
- System: Scenario configuration (missing recipe + missing profile) → planner has no viable operators
- Code area: Scenario file `scenarios/cli-evaluation.ron` (Forager Lina agent definition)

**Golden test blueprint**:
- Harness setup: Two places connected by travel edge (2 ticks). Place A: agent with thirst=600, thirst_rate=5, known_recipes=["Harvest Apples"] only, NO perception_profile. Place B: Well + Water resource source. Items on ground at Place A: 3× Water.
- Tick count: 200
- Primary assertion: agent either (a) perceives ground water and drinks it, or (b) travels to Place B and harvests water
- Failure mode assertion: agent idles for 100+ consecutive ticks; thirst reaches 1000‰; AcquireCommodity(Water) frontier-exhausts
- Regression guard: ensures agents without water recipes or with missing perception profiles are detected as scenario configuration errors or handled gracefully

## Proposed Solutions

### Planner Budget Wall

**Solution PBW-1: Reduce candidate explosion for simple plans** (Engine fix)
- **What**: When AcquireCommodity targets a commodity present at the agent's current location, fast-path the search: generate only pick_up + consume operators for that commodity before expanding the full candidate set.
- **Where**: `worldwake-ai::search` or `worldwake-ai::candidate_generation`
- **FOUNDATIONS alignment**: FND-01 (causal standard — agent should respond to proximate causes), FND-15 (maximal emergence — the planner should find obvious solutions without hard-coding)
- **Existing specs**: `specs/S88-*.md` (GOAP overhaul) introduced `max_candidates_per_expansion` and `preferred_operator_boost`. The `preferred_operator_boost=3` setting on Vara should help but appears insufficient when 150 candidates are generated per expansion.
- **Type**: Engine fix
- **Impact**: Addresses DM-1

**Solution PBW-2: Tune Merchant Vara's cognitive_profile** (Scenario fix)
- **What**: Reduce `max_candidates_per_expansion` from 150 to 30-50. Increase `max_node_expansions` from 300 to 500. This reduces branching factor while giving more budget.
- **Where**: `scenarios/cli-evaluation.ron` → Merchant Vara → cognitive_profile
- **FOUNDATIONS alignment**: FND-08 (agent diversity — different agents can have different planner parameters)
- **Type**: Scenario fix (profile tuning)
- **Impact**: Addresses DM-1

**Solution PBW-3: Hunger-escalation priority class** (Engine fix)
- **What**: When hunger is above critical threshold and food is believed present at current location, elevate AcquireCommodity(Food) to a priority class that gets expanded first with a dedicated mini-budget, before falling back to full search.
- **Where**: `worldwake-ai::candidate_generation` or `worldwake-ai::search`
- **FOUNDATIONS alignment**: FND-01 (causal standard), FND-15 (emergence). This is a heuristic that lets the planner find obvious local solutions before exploring the full search space.
- **Existing specs**: S96 (obligation satiation) addresses obligation-vs-need priority but not planner search efficiency for survival needs.
- **Type**: Engine fix
- **Impact**: Addresses DM-1

### Geographic Desert

**Solution GD-1: Add Water resource to Eldergrove Forest** (Scenario fix)
- **What**: Add a stream or pond at Eldergrove Forest so Lina's starting location has water access. Add a facility (e.g., a `Well` or new `Stream` workstation) plus resource source.
- **Where**: `scenarios/cli-evaluation.ron` → facilities + resource_sources
- **FOUNDATIONS alignment**: FND-07 (information locality — agents should have local access to survival resources)
- **Type**: Scenario fix
- **Impact**: Partially addresses DM-2 (only if Lina also gets Harvest Water recipe)

### Knowledge Gap (Recipe)

**Solution KG-1: Add "Harvest Water" to Forager Lina's known recipes** (Scenario fix)
- **What**: Add `"Harvest Water"` to Forager Lina's `known_recipes` list. Water harvesting is basic survival knowledge.
- **Where**: `scenarios/cli-evaluation.ron` → Forager Lina → known_recipes
- **FOUNDATIONS alignment**: FND-08 (agent diversity — even foragers need basic survival recipes)
- **Type**: Scenario fix
- **Impact**: Addresses DM-2 together with GD-1 or alone (if Lina travels to Thornwall Village)

### Profile Gap

**Solution PG-1: Add perception_profile to Forager Lina** (Scenario fix)
- **What**: Add a `perception_profile` to Forager Lina similar to Kael's or Guard Theron's. Without it, she can't observe items on the ground, making her unable to interact with her environment at locations she hasn't been configured for.
- **Where**: `scenarios/cli-evaluation.ron` → Forager Lina
- **FOUNDATIONS alignment**: FND-07 (information locality — agents need perception to interact with local environment), CLAUDE.md critical invariant "Scenario profile completeness — every agent profile component registered on EntityKind::Agent must be scenario-definable"
- **Existing specs**: Check whether `PerceptionProfile` is a universal profile that should auto-apply. Per CLAUDE.md: "Golden production tests require PerceptionProfile on agents that need to observe post-production output."
- **Type**: Scenario fix
- **Impact**: Addresses DM-2 partially — even without Harvest Water recipe, Lina could pick_up and drink pre-existing Water items at her location

**Solution PG-2: Make PerceptionProfile a universal profile with defaults** (Engine fix)
- **What**: If PerceptionProfile isn't already applied universally with defaults, it should be. An agent without perception is essentially blind — this is a critical survival capability, not an optional enhancement.
- **Where**: `worldwake-core` or wherever universal profiles are registered
- **FOUNDATIONS alignment**: FND-15 (maximal emergence — agents need baseline perception to participate in the simulation), CLAUDE.md "Universal profiles are always applied with defaults"
- **Type**: Engine fix
- **Impact**: Prevents DM-2 class failures across all future scenarios

### Belief Memory Pollution

**Not observed in this run** — Dusty Trail has 22× Waste at end-state, but the primary failures are due to planner budget and missing profiles/recipes rather than belief displacement. Could become relevant in longer runs.

### Exploration Failure

**Solution EF-1: Lina's ExploreLocation should target water-bearing locations when thirsty** (Engine consideration)
- **What**: Lina has an `exploration_profile` with `need_activation_threshold: 350`. At tick 406 her thirst is at 750‰ (well above 350). She did select ExploreLocation(target=Dusty Trail, motivating_need=Dirtiness) earlier, but never explored for water.
- **Where**: `worldwake-ai::candidate_generation` (ExploreLocation goal generation)
- **FOUNDATIONS alignment**: FND-01 (causal standard — exploration should respond to survival pressures)
- **Existing specs**: S80 (exploration drive) — check if need-directed exploration considers all critical needs or just the first one encountered.
- **Type**: Engine investigation (may be working as designed — Lina explored for Dirtiness not Thirst, which suggests the goal generator doesn't prioritize the most critical need)
- **Impact**: Partially addresses DM-2

## Golden Test Recommendations

| Priority | Damning Moment | Test Name Suggestion | What It Guards Against |
|----------|---------------|---------------------|----------------------|
| 1 | DM-1 | `golden_planner_finds_local_pickup_eat` | Planner budget wall: agent with high `max_candidates_per_expansion` fails to find simple pick_up→eat plan for food at current location |
| 2 | DM-2 | `golden_agent_without_perception_profile_starves` | Profile gap: agent without PerceptionProfile can't observe ground items and starves despite resources being available |
| 3 | DM-2 | `golden_recipe_gap_prevents_water_harvest` | Knowledge gap: agent near Well but without Harvest Water recipe cannot satisfy thirst |

## Observer Error Note

The observer crashed at tick 543 with `PreconditionFailed("violation 12 is no longer active at commit")` in `investigate_actions.rs:179`. This is a separate bug: Guard Theron's investigation action committed against a violation that had already been resolved between plan execution and commit. The observer was patched to `break` instead of `exit(1)` so partial dumps are still written. The underlying violation-expiry race condition in `investigate_actions.rs` should be addressed separately.
