# Layer 2: Needs Diagnostics (Step 5, Conditional)

**Skip this layer entirely if the triage checkpoint determined no agent has needs failures.**

## Step 5.1: Classify Each Agent's Needs Failure

For each agent with any need >750 permille for 100+ consecutive ticks, classify the root cause into one or more categories:

| Category | Signature | How to Detect |
|----------|-----------|---------------|
| **Geographic Desert** | Agent at location with no local affordance for the need; AcquireCommodity budget-exhausts | Section 7: no relevant facility. Section 8: budget-exhausted. Affordances lack eat/drink/wash/harvest. |
| **Planner Budget Wall** | Resource exists at reachable location but plan search exceeds expansion budget | Section 8: budget-exhausted with 500+ candidates, depth 5+. Section 7: resource exists elsewhere connected by travel edges. |
| **Belief Blindness** | Agent lacks beliefs about resource-rich locations | Section 6: agent doesn't know about resource locations. Also covers facility-specific blindness: recipe exists but facility location unknown. |
| **Priority Override** | Agent has affordances for need relief but another goal consistently outranks it, so relief fires rarely or never | Section 8: non-needs goal dominates during critical-need period. Affordances DO include the relief action but it fires at a frequency insufficient to keep the need below threshold (or never fires). Distinguish "never fires" from "fires too infrequently" — both are Priority Override. For "fires too infrequently," compare relief-action count vs. need-accumulation rate: if `relief_rate < need_accumulation_rate` across a rolling 200+ tick window, classify as Priority Override regardless of whether the action ever committed. |
| **Structural Impossibility** | No location in the entire scenario has the needed resource/facility | Scenario inspection: no wash-capable facility exists anywhere, etc. |
| **Exploration Failure** | Agent has exploration_profile but ExploreLocation never fires or fails | Section 8: ExploreLocation in failed plans/blocked desires, or never appears in goals-selected despite having the profile. |
| **Belief Memory Pollution** | Agent previously knew resource locations but beliefs displaced by irrelevant entities | Section 6: memory at capacity dominated by SocialArtifacts/Waste. Section 2: agent visited resource locations earlier. Distinct from Belief Blindness (never learned vs. crowded out). |
| **Knowledge Gap** | Agent believes location exists but lacks recipe to exploit it | Section 6: knows facility location. Scenario: agent's known_recipes missing required recipe. |
| **Profile Gap** | Agent missing profile component enabling survival behavior | Compare scenario agent definition against registered profiles. Common: no PerceptionProfile (blind), no CognitiveProfile (default budget), no ExplorationProfile (no exploration drive). |

An agent may have multiple categories. For agents that died, trace the causal chain backward from death.

**Profile gap cross-reference**: When classifying as Profile Gap, check:
- Priority Override without `ObligationSatiationProfile` → satiation mechanism inactive
- No exploration without `ExplorationProfile` → exploration drive inactive
- Default planner budget without `CognitiveProfile` → engine defaults may be too low
- No item observation without `PerceptionProfile` → agent can't see ground items

## Step 5.2: Capture Damning Moments

For each classified failure, extract a "damning moment" — the exact agent state at the point where the failure becomes irrecoverable or clearly pathological.

**Identifying the damning tick**: The most reliable source is Section 4 anomaly flag tick ranges (e.g., "hunger above 750 permille for 274 consecutive ticks (ticks 269–542)" → tick 269). Refine by category:

- **Geographic Desert**: Section 4 start tick + Section 2 location history
- **Planner Budget Wall**: First budget-exhausted attempt from Section 8 or Section 9
- **Belief Blindness**: Tick when needs crossed high threshold with no resource beliefs
- **Belief Memory Pollution**: Tick when belief memory became dominated by non-resource entities
- **Priority Override**: Tick when the overriding goal first outranked needs at critical levels
- **Exploration Failure**: Tick when ExploreLocation first failed or was needed but absent

**Consolidation**: One damning moment per agent, focused on the *primary* root cause (earliest breakpoint). Note secondary categories in the Breakpoint section.

**Damning moment format**:

```markdown
#### Damning Moment DM-[N]: [Agent] — [Category] at tick [T]

**Agent state at tick [T]**:
- Location: [place name]
- Needs: hunger=[X] permille, thirst=[Y] permille, fatigue=[Z] permille, bladder=[W] permille, dirtiness=[V] permille
- Inventory: [commodity: count, ...]
- Known recipes: [from scenario file]

**Location state**:
- Facilities at [place]: [list, or "none relevant to [need]"]
- Resource sources at [place]: [list, or "none"]
- Consumables at [place]: [list, or "none"]
- Adjacent places (from scenario edges): [place (travel_ticks), ...]

**Agent beliefs about resources**:
- Believed locations: [what places the agent knows about]
- Believed resources: [what the agent thinks exists where]
- Missing beliefs: [resource-rich locations the agent doesn't know about]

**Planner state**:
- Goal attempted: [goal kind and target]
- Outcome: [budget-exhausted / frontier-exhausted / never attempted]
- Candidates: [count], Depth: [N], Expansions: [N/budget]
- Competing goals: [what goal was selected instead, if Priority Override]

**Expected behavior chain**:
1. [Goal that should have been generated]
2. [Action 1, e.g., "travel to Thornwall Village (2 ticks)"]
3. [Action 2, e.g., "harvest Water at Village Well"]
4. [Action 3, e.g., "drink Water"]

**Actual behavior**: [what the agent did instead. Source from Section 8 decision timeline bins and Section 2 action counts.]

**Breakpoint**: [specific point where expected chain broke]
- System: [which system failed — e.g., "GOAP planner budget exhaustion", "goal ranking", "affordance generation"]
- Code area: [crate and module — e.g., "worldwake-ai::search", "worldwake-ai::candidate_generation"]

**Golden test blueprint**:
- Harness setup: [what GoldenHarness needs — place graph, agent profiles, facilities, items, beliefs]
- Tick count: [how many ticks to run]
- Primary assertion: [what the test should check]
- Failure mode assertion: [what currently happens]
- Regression guard: [once fixed, what the test prevents from recurring]
```

**Quality criteria**:
- Every field must have a concrete value, not "unknown". If the dump doesn't provide a value, note it as "[not in dump — needs observer enhancement]".
- For needs values at the damning tick: if no per-tick snapshot exists, note as "approximately [X] permille (interpolated from trajectory and anomaly range)".
- Expected behavior chain must be a plausible action sequence the engine could execute if the breakpoint were fixed.
- Golden test blueprint must be specific enough to write the test without re-reading the dump.

## Step 5.3: Propose Solutions

For each root cause category found, propose concrete solutions grouped by category.

**For each solution, include**:
- **What**: Concrete change (code, config, or scenario)
- **Where**: Crate/module or scenario file
- **FOUNDATIONS alignment**: Which principles support this (cite by number)
- **Existing specs**: Check `specs/*.md` and `archive/specs/*.md` for prior attempts. Note whether implementations are present and whether they were sufficient.
- **Type**: Scenario fix / Engine fix / Profile tuning
- **Impact estimate**: Which damning moments (DM-[N]) this addresses

**Solution categories**:

- **Geographic Desert**: Add resources to barren locations, improve ExploreLocation, add travel-first planner heuristic
- **Planner Budget Wall**: Raise max_node_expansions, prune irrelevant candidates, hierarchical task decomposition, speculative planning
- **Belief Blindness**: Landmark belief retention, hearsay about resources, initial common-knowledge beliefs
- **Priority Override**: Survival-need escalation, obligation cooldown, goal ranking cap
- **Belief Memory Pollution**: Memory partitioning, priority-based belief retention, observation filtering
- **Knowledge Gap**: Add missing recipes to scenario, recipe learning, initial common-knowledge recipes
- **Profile Gap**: Add missing profile to scenario, make survival-critical profiles universal with defaults, scenario validation
- **Exploration Failure**: Lower activation threshold, need-directed exploration, separate budget, multi-hop exploration chaining
