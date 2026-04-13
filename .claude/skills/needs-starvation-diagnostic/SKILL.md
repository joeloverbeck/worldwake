---
name: needs-starvation-diagnostic
description: "Run a scenario headlessly, diagnose agents stuck unable to meet basic needs (hunger, thirst, dirtiness, etc.), capture exact failure states as golden test blueprints, and propose solutions."
user-invocable: true
---

# Needs Starvation Diagnostic

Run a scenario via the observer binary, then perform focused analysis on needs-satisfaction failures. Produces a report with root-cause classifications, "damning moment" snapshots formatted as golden test blueprints, and proposed solutions.

This skill is **narrowly focused** on needs starvation pathologies. It does NOT replace `simulation-observer` (which covers all 10 behavioral smells). Use this skill when the primary concern is agents unable to eat, drink, wash, or otherwise satisfy basic needs.

## Invocation

```
/needs-starvation-diagnostic scenarios/cli-evaluation.ron
/needs-starvation-diagnostic scenarios/cli-evaluation.ron --ticks 2880
```

First argument: path to a `.ron` scenario file (required).
Optional `--ticks N` to override the default of 1440 ticks (1 simulated day).

If no scenario path is provided, glob for `scenarios/*.ron` and present the list to the user. If exactly one scenario file exists, confirm it before proceeding. If none exist, stop and report.

## Process

Follow these steps in order. Do not skip any step.

### Step 1: Build & Run Observer

```bash
cargo build -p worldwake-cli --bin observer
```

**Hard gate**: If the build fails, stop and report the error. Do not proceed.

Read the scenario `.ron` file to extract: agent profiles (which optional profiles each agent has — especially `exploration_profile`, `obligation_satiation_profile`, `cognitive_profile`, `metabolism_profile`), place topology (edges and travel times), facilities, resource sources, and initial items. This context is used in Steps 3-5.

```bash
cargo run -p worldwake-cli --bin observer -- <scenario_path> --ticks <N> --output reports/needs-diagnostic-dump.md
```

- Use the scenario path provided by the user.
- Use the tick count provided by the user, or default to 1440.
- The observer may take several minutes. Wait for the process to exit rather than checking for the output file. If the Bash tool backgrounds the observer command, wait for the background task completion notification. Do not poll for the output file — the notification will arrive automatically when the process exits.
- If the binary exits with a non-zero code, diagnose using the same logic as the `simulation-observer` skill (schema drift, runtime error, I/O error). Fix code bugs if found, note fixes in the report.

**Common failure mode — mid-simulation tick error**: If the observer crashes during tick execution (e.g., `Action(PreconditionFailed(...))`) rather than during build or startup, the dump may not be written because the observer calls `std::process::exit(1)` before reaching the dump-writing code. In this case, check the observer's tick error handler in `observer.rs`. If it uses `std::process::exit(1)`, change it to `break` from the tick loop so the observer still writes a partial dump with data collected up to the crash tick. A partial dump (even 500 of 1440 ticks) is far more useful than no dump. Note the crash tick, error message, and any observer fix in the report's Observer Notes section.

**Hard gate**: If `reports/needs-diagnostic-dump.md` does not exist or is empty after the run, stop and report.

**Note**: This uses a separate dump file from `simulation-observer` (`needs-diagnostic-dump.md` vs `simulation-observer-dump.md`) to avoid conflicts if both skills are used.

### Step 2: Read Dump — Needs-Focused Extraction

Read the dump selectively, focusing on needs-related data only.

**Required sections** (read in this order):

1. **Section 1** (Run Metadata): Extract agent names, place names, entity ID mapping. Note scenario file, seed, tick count.

2. **Section 2** (Per-Agent Summary): For each agent extract:
   - Needs trajectory samples (hunger, thirst, fatigue, bladder, dirtiness as permille over time)
   - "Ticks above 750 permille" counts for each need
   - Death tick and cause (if applicable)
   - Location history (time at each place)
   - Behavioral transition markers

3. **Section 3** (Anomaly Flags): Extract only:
   - Smell 5 flags (sustained critical needs) — tick ranges per agent per need
   - Smell 6 flags (unaddressed needs) — needs with high average but no relief action

4. **Section 6** (End-State Inventory & Resources): For each place:
   - What facilities exist (Well, OrchardRow, Mill, WashBasin, etc.)
   - What consumable commodities are present
   - What resource sources exist
   - For each agent: current inventory

5. **Section 5** (Per-Agent Belief Summary): For each agent extract:
   - Believed entity locations (what places the agent knows about and what entities it believes are at each place)
   - Count of known entities and breakdown by type (agents, places, items, other)
   - Believed resource/facility locations (does the agent know about Wells, OrchardRows, WashBasins, food/water items?)
   - Skip: social observation counts, told/heard belief statistics, institutional belief details

6. **Section 7** (Decision Summary): For each agent extract:
   - Plan search outcomes (found / frontier-exhausted / budget-exhausted counts)
   - Failed plan attempts — specifically for AcquireCommodity, ConsumeOwnedCommodity, Wash, ExploreLocation goals
   - Blocked desires related to needs
   - Affordance snapshots (tick 0, travel arrivals, final) — specifically eat, drink, wash, harvest, travel affordances
   - Goals selected — specifically which needs-related goals were selected vs. which non-needs goals dominated

7. **Section 8** (Budget Exhaustion Snapshots, if present): For each snapshot:
   - Agent needs at moment of budget exhaustion
   - Agent location and inventory
   - Believed entity locations
   - Place contents and adjacent place contents
   - Cognitive profile (budget parameters)
   - Search metrics (candidates, depth, expansions)

**Skip entirely**: Perception analysis, social isolation details, impossible knowledge checks, tell/social action analysis, SocialArtifact enumeration. These are handled by `simulation-observer`.

**Section 7 reading protocol**: Section 7 lines are extremely dense. Use the same extraction sequence as `simulation-observer`:
- Grep `Tick breakdown` and `Plan search outcomes` first
- `bash grep 'Goals selected'` for goal type frequency
- Grep `Failed plan attempts` with `-A 30`
- Grep `Blocked desires` with `-A 10` (may not be present in all observer versions — skip if absent)
- Grep `Affordances available`, `Affordances after travel`, `Final affordances` with `-A 15`
- For dense rows: `bash sed -n 'Xp' <file> | head -c 3000`

### Step 3: Classify Each Agent's Needs Failure

For each agent with any need >750 permille for 100+ consecutive ticks, classify the root cause into one or more categories:

| Category | Signature | How to Detect |
|----------|-----------|---------------|
| **Geographic Desert** | Agent at location with no local affordance for the need; AcquireCommodity budget-exhausts | Section 6 shows no relevant facility at agent's location. Section 7 shows budget-exhausted AcquireCommodity. Affordance snapshots lack eat/drink/wash/harvest. |
| **Planner Budget Wall** | Resource exists at reachable location but plan search exceeds expansion budget | Section 7 shows budget-exhausted with high candidate count (500+) and depth (5+). Section 6 confirms the resource exists at another location connected by travel edges. |
| **Belief Blindness** | Agent lacks beliefs about resource-rich locations, so planner can't plan travel there | Section 5 (beliefs) shows agent doesn't know about locations with resources. Section 8 snapshots show empty believed-entity-locations for resource-producing places. |
| **Priority Override** | Agent has affordances for need relief but another goal consistently outranks it | Section 7 goals-selected shows a non-needs goal (PostNotice, Patrol, ShareBelief) dominating during the critical-need period. Affordance snapshots DO include eat/drink/wash but the action never fires. |
| **Structural Impossibility** | No location in the entire scenario has the needed resource/facility | Read scenario `.ron` file. Check all places and facilities. If no WashBasin exists anywhere, dirtiness is structurally unsatisfiable. |
| **Exploration Failure** | Agent has exploration_profile but ExploreLocation never fires or fails | Section 7 shows ExploreLocation in failed plans or blocked desires. Or ExploreLocation never appears in goals-selected despite the agent having an exploration_profile in the scenario. |
| **Belief Memory Pollution** | Agent previously knew resource locations but beliefs were displaced by irrelevant entities (SocialArtifacts, Waste) | Section 5 shows belief memory at capacity but dominated by non-resource entities. Cross-reference with Section 2 location history — agent visited resource locations earlier but beliefs decayed and were replaced by high-volume low-value observations. Distinct from Belief Blindness (never learned) — here the agent *had* beliefs that were crowded out. |
| **Knowledge Gap** | Agent believes resource location exists and is reachable, but lacks recipe or skill to exploit it | Section 5 shows agent knows about facility/resource location (e.g., believes Well exists at Thornwall Village). Scenario file shows agent's `known_recipes` does NOT include the required recipe (e.g., "Harvest Water"). Cross-reference available facilities with agent recipes. Distinct from Belief Blindness (the agent *knows* where resources are) and Structural Impossibility (the resources *exist*). |
| **Profile Gap** | Agent is missing a profile component that would enable survival-critical behavior | Compare scenario agent definition against engine-registered profile types. If a profile exists in the engine (grep for the type) but is absent from the agent, and that profile is needed for the agent's survival (e.g., `PerceptionProfile` for observing ground items, `CognitiveProfile` for planner budget, `ExplorationProfile` for finding resources), classify as Profile Gap. May co-occur with other categories. |

An agent may have multiple categories (e.g., Geographic Desert + Belief Blindness + Planner Budget Wall often co-occur).

**For agents that died**: Trace the causal chain backward from death. Which need killed them? What was the proximate cause (priority override preventing relief? budget exhaustion? no affordance?)?

**For agents with no needs failures**: Note them as "healthy" with a brief explanation of why they succeeded (e.g., "self-sufficient at resource-rich location").

**Profile gap cross-reference**: When classifying as Profile Gap, check these common patterns:
- Priority Override without `ObligationSatiationProfile` → S96 satiation mechanism inactive
- No exploration without `ExplorationProfile` → S80 exploration drive inactive
- Default planner budget without `CognitiveProfile` → agent uses engine defaults which may be too low
- No item observation without `PerceptionProfile` → agent can't see ground items, effectively blind

### Step 4: Capture Damning Moments

This is the primary deliverable section. For each classified failure, extract a "damning moment" — the exact agent state at the point where the failure becomes irrecoverable or clearly pathological.

**Identifying the damning tick**: The most reliable source for the 750‰ crossing tick is the Section 3 anomaly flag tick ranges (e.g., "hunger above 750‰ for 274 consecutive ticks (ticks 269–542)" → the damning tick is tick 269). Use these heuristics to refine:
- For Geographic Desert: The start tick from Section 3 anomaly range, cross-referenced with Section 2 location history to confirm the agent was at a barren location. If the Section 2 action count shows a specific number of eat/drink actions, the last one approximately marks when supplies ran out.
- For Planner Budget Wall: The first budget-exhausted AcquireCommodity attempt (from Section 7 failed plan attempts, or Section 8 first snapshot for that agent)
- For Belief Blindness: The tick when needs crossed the "high" threshold while the agent had no beliefs about resource locations
- For Belief Memory Pollution: The tick when belief memory became dominated by non-resource entities (cross-reference Section 5 end-state beliefs with Section 2 location history — if the agent visited resource locations early but end-state beliefs show only SocialArtifacts/Waste, the pollution occurred between those points)
- For Priority Override: The tick when the overriding goal first outranked the needs goal during critical need levels
- For Exploration Failure: The tick when ExploreLocation first failed or was first needed but absent

**Consolidation**: Create one damning moment per agent, focused on the *primary* root cause — the earliest breakpoint in the causal chain. Note secondary categories in the Breakpoint section. If the primary and secondary causes manifest at different ticks (e.g., Geographic Desert at tick 15 but Belief Blindness at tick 80), capture the most diagnostically valuable tick.

**For each damning moment, capture all of the following**:

```markdown
#### Damning Moment DM-[N]: [Agent] — [Category] at tick [T]

**Agent state at tick [T]**:
- Location: [place name]
- Needs: hunger=[X]‰, thirst=[Y]‰, fatigue=[Z]‰, bladder=[W]‰, dirtiness=[V]‰
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
1. [Goal that should have been generated, e.g., "ExploreLocation(need=hunger)"]
2. [Action 1, e.g., "travel to Thornwall Village (2 ticks)"]
3. [Action 2, e.g., "harvest Water at Village Well"]
4. [Action 3, e.g., "drink Water"]

**Actual behavior**: [what the agent did instead — e.g., "sleep + relieve_wilderness loop"]

**Breakpoint**: [the specific point where the expected chain broke]
- System: [which system/component failed — e.g., "GOAP planner budget exhaustion", "goal ranking", "affordance generation", "belief formation"]
- Code area: [crate and approximate module — e.g., "worldwake-ai::search", "worldwake-ai::candidate_generation"]

**Golden test blueprint**:
- Harness setup: [what GoldenHarness needs — place graph, agent with specific needs/profiles, facilities, items, beliefs]
- Tick count: [how many ticks to run]
- Primary assertion: [what the test should check — e.g., "agent commits travel action within 20 ticks"]
- Failure mode assertion: [what currently happens that the test should initially capture as the bug — e.g., "agent never commits travel; only sleep+relieve for 100 ticks"]
- Regression guard: [once fixed, what the test prevents from recurring]
```

**Damning moment quality criteria**:
- Every field must have a concrete value, not "unknown" or "N/A". If the dump doesn't provide a value, note it as "[not in dump — needs observer enhancement]".
- The expected behavior chain must be a plausible action sequence that the engine could execute if the breakpoint were fixed.
- The golden test blueprint must be specific enough that someone could write the test from it without re-reading the dump.

### Step 5: Propose Solutions

For each root cause category found in the diagnostic, propose concrete solutions. Group by category.

**For each solution, include**:
- **What**: Concrete change (code, config, or scenario)
- **Where**: Crate/module or scenario file
- **FOUNDATIONS alignment**: Which principles support this solution (cite by number, e.g., FND-07 Information Locality)
- **Existing specs**: Check `specs/*.md` and `archive/specs/*.md` for specs that already attempted to address this. Note whether those specs' implementations are present in the codebase (grep for key types/functions they introduced). If a prior spec attempted this fix and the problem persists, note that the spec's implementation was insufficient and hypothesize why.
- **Type**: Scenario fix (change the .ron file) / Engine fix (change Rust code) / Profile tuning (adjust agent parameters)
- **Impact estimate**: Which damning moments this solution would address (reference DM-[N])

**Solution categories**:

**Geographic Desert solutions**:
- Add resource sources or facilities to barren locations in the scenario
- Improve ExploreLocation to drive agents toward resource-rich locations
- Add "travel-first" planner heuristic that narrows search to reachable-resource locations

**Planner Budget Wall solutions**:
- Raise max_node_expansions for specific goal types
- Prune irrelevant candidates before search (e.g., filter travel targets to only resource-relevant locations)
- Hierarchical task decomposition (plan "get food" as an abstract macro-action)
- Speculative planning (plan from believed state of destination, not current state)

**Belief Blindness solutions**:
- Landmark belief retention (agents remember facilities at locations they've visited, with slow decay)
- Hearsay about resources (agents share knowledge about resource locations via tell)
- Initial common-knowledge beliefs (agents know the general layout of nearby locations)

**Priority Override solutions**:
- Survival-need escalation (critical needs gain increasing priority bonus)
- Obligation cooldown (fast-completing obligations get a mandatory cooldown period after N consecutive firings)
- Goal ranking cap (no non-survival goal can outrank a survival goal above a critical threshold)

**Belief Memory Pollution solutions**:
- Memory partitioning (SocialArtifacts, Waste, and other high-volume low-value entities use a separate memory pool that doesn't displace survival-relevant beliefs about facilities/resources)
- Priority-based belief retention (beliefs about facilities and resource sources decay slower or are protected from eviction)
- Observation filtering (reduce observation frequency for entity types that are not survival-relevant when needs are critical)

**Knowledge Gap solutions**:
- Add missing survival recipes to the agent's `known_recipes` in the scenario (e.g., "Harvest Water" for any agent that might need water)
- Implement recipe learning — agents can learn recipes by observing other agents performing them
- Initial common-knowledge recipes — basic survival recipes (Harvest Water, Harvest Grain) could be universal knowledge

**Profile Gap solutions**:
- Add the missing profile to the agent in the scenario file (e.g., `perception_profile` for agents that need to observe items)
- Make survival-critical profiles universal with defaults (e.g., PerceptionProfile should auto-apply if not explicitly configured)
- Add scenario validation that warns when agents lack profiles needed for basic survival behavior

**Exploration Failure solutions**:
- Lower ExploreLocation activation threshold when survival needs are critical
- Need-directed exploration (ExploreLocation targets locations believed to have resources, or unknown locations adjacent to resource-rich ones)
- Exploration budget separate from acquisition budget

### Step 6: Write Report

If `reports/needs-starvation-diagnostic.md` already exists, check `git status` for uncommitted changes. Warn before overwriting uncommitted work. If committed or untracked, overwrite directly.

Write `reports/needs-starvation-diagnostic.md`:

```markdown
# Needs Starvation Diagnostic

## Run Summary
- **Scenario**: `[path]`
- **Seed**: [N]
- **Ticks simulated**: [N]
- **Agents**: [names]
- **Places**: [names]
- **Deaths**: [agent at tick N (cause), or "None"]

## Observer Notes

[If the observer crashed mid-simulation, document: the tick error message, the crash tick, any code fixes applied (e.g., changing exit(1) to break), and whether the dump is partial. If no observer issues, omit this section.]

## Agent Needs Overview

| Agent | Need | Max Value | Ticks >750‰ | Death? | Root Cause Category |
|-------|------|-----------|-------------|--------|---------------------|
| ... | ... | ... | ... | ... | ... |

[One row per agent per need that exceeded 750‰ for 100+ ticks. Healthy agents get a single row: "all needs managed" with no category.]

## Failure Classifications

### [Agent Name]
**Categories**: [list]
**Evidence**: [specific data from dump]
**Confidence**: [HIGH/MEDIUM/LOW]
**Causal chain**: [A → B → C summary of how the failure developed]

[Repeat for each affected agent]

## Damning Moments

[All captured damning moments in the format specified in Step 4. This is the primary deliverable section.]

## Proposed Solutions

### Geographic Desert
[Solutions if this category was found]

### Planner Budget Wall
[Solutions if this category was found]

### Belief Blindness
[Solutions if this category was found]

### Priority Override
[Solutions if this category was found]

### Structural Impossibility
[Solutions if this category was found]

### Knowledge Gap
[Solutions if this category was found]

### Profile Gap
[Solutions if this category was found]

### Belief Memory Pollution
[Solutions if this category was found]

### Exploration Failure
[Solutions if this category was found]

[Omit categories not found in this run.]

## Golden Test Recommendations

| Priority | Damning Moment | Test Name Suggestion | What It Guards Against |
|----------|---------------|---------------------|----------------------|
| 1 | DM-[N] | golden_[descriptive_name] | [regression description] |
| ... | ... | ... | ... |

[Ordered by priority: deaths first, then sustained critical needs, then moderate failures. Include a suggested test file name following the project's `golden_*.rs` naming convention.]
```

### Step 7: Clean Up

Delete `reports/needs-diagnostic-dump.md` — the dump is an intermediate artifact. The report in `reports/needs-starvation-diagnostic.md` is the deliverable.

## Notes

- The observer binary outputs to stderr for progress; only the markdown dump file matters.
- 1440 ticks = 1 simulated day. For deeper analysis, try 2880 (2 days).
- If the dump lacks a Section 8 (Budget Exhaustion Snapshots), the observer binary may predate that feature. Fall back to Section 7 failed plan attempts and Section 2 needs trajectories for damning moment data. Note any "[not in dump]" fields in the damning moments.
- Human-controlled agents (ControlSource::Human) will not have needs-related planning. Skip them in the diagnostic unless they died from need deprivation (which would indicate a scenario issue).
- This skill focuses on needs starvation. It does NOT analyze: redundant perception, social isolation, impossible knowledge, economic stagnation (beyond its needs-related aspects), or action loops unrelated to needs. Use `simulation-observer` for comprehensive behavioral analysis.
- For before/after comparisons, run twice with different tick counts or after code changes, and compare the Agent Needs Overview tables and Damning Moments sections.
