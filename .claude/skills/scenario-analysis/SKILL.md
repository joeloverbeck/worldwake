---
name: scenario-analysis
description: "Run a scenario headlessly with the observer binary, perform comprehensive behavioral smell analysis, needs diagnostics, and meta-analysis of detection gaps and false positives. Writes report to reports/scenario-analysis-report.md."
user-invocable: true
---

# Scenario Analysis

Run a scenario headlessly via the observer binary, read the structured dump, perform three layers of analysis, and write a unified report. This skill subsumes both the former `simulation-observer` and `needs-starvation-diagnostic` skills.

**Three analysis layers**:
- **Layer 1 — Behavioral Smells**: All 10 smell categories (6 mechanical + 4 LLM-only) with severity ratings
- **Layer 2 — Needs Diagnostics**: Root-cause classification, damning moments, golden test blueprints, and proposed solutions (conditional — only when needs failures are detected)
- **Layer 3 — Detection Meta-Analysis**: Evaluates the anomaly detection system itself — false positives, detection gaps, threshold assessment, and new smell proposals

## Invocation

```
/scenario-analysis scenarios/cli-evaluation.ron
/scenario-analysis scenarios/cli-evaluation.ron --ticks 720
/scenario-analysis scenarios/cli-evaluation.ron --days 2
```

First argument: path to a `.ron` scenario file (required).
Optional `--ticks N` to override the default of 1440 ticks (1 simulated day).
Optional `--days N` as sugar for `--ticks N*1440` (e.g., `--days 2` = 2880 ticks).

If no scenario path is provided, glob for `scenarios/*.ron` and present the list to the user. If exactly one scenario file exists, confirm it before proceeding. If none exist, stop and report.

## Process

Follow these steps in order. Do not skip any step.

### Step 0: Pre-flight

#### Step 0.5: Scenario Pre-flight

Before running the observer, scan the `.ron` scenario file for obvious survival gaps. This is a quick sanity check that can catch trivially broken scenarios before spending minutes on an observer run.

Check for:
- **Agents without food recipes**: Any AI agent whose `known_recipes` contains no food-producing recipe (e.g., only "Harvest Water") will be unable to produce food. Flag as pre-flight warning.
- **Agents without perception_profile**: Agents without `perception_profile` have severely limited observation capacity and may be effectively blind to ground items.
- **Locations without reachable food/water**: For each place with agents, trace travel edges to check whether food and water sources are reachable within a reasonable hop count (2-3 hops). Flag isolated locations with no food/water access.
- **Agents without water access for washing**: Wash requires possessed Water (not a facility). Check whether agents can reach a water source (Well, River, or other water-producing facility) to harvest Water for washing. Flag agents with no reachable water source within 2-3 travel hops.
- **Agents with disabled social profiles**: Check `tell_profile.max_tell_candidates`. If zero for all agents, note as pre-flight observation: "Social interaction disabled — smell 9 (Social Isolation) is expected by design." This avoids false reporting during Layer 1.

Report findings as "Pre-flight Warnings" in the report's Run Summary section. Do not gate the observer run on pre-flight results — these are informational only.

#### Step 0.7: Extract Scenario Purpose

If the scenario `.ron` file contains a purpose comment (typically the first `//` comment block at the top of the file), extract it and include it in the report's Run Summary as **Scenario purpose**. In the report conclusion, note whether the scenario achieved its stated purpose.

Also read the scenario `.ron` file to extract: agent profiles (which optional profiles each agent has — especially `exploration_profile`, `obligation_satiation_profile`, `cognitive_profile`, `metabolism_profile`, `perception_profile`), place topology (edges and travel times), facilities, resource sources, and initial items. This context is used in Layer 2.

### Step 1: Build & Run Observer

```bash
cargo build -p worldwake-cli --bin observer
```

**Hard gate**: If the build fails, stop and report the error. Do not proceed.

```bash
cargo run -p worldwake-cli --bin observer -- <scenario_path> --ticks <N> --output reports/scenario-analysis-dump.md
```

- Use the scenario path provided by the user.
- Use the tick count provided by the user, or default to 1440.
- The observer may take several minutes to write the dump after the last tick completes (the dump-writing phase is CPU-intensive for large simulations). If using background execution, wait for the process to exit rather than checking for the output file — the file is written atomically at the end.
- If the binary exits with a non-zero code, diagnose the failure mode:
  - **Scenario parse error** (missing field, wrong type): Stop and report. The scenario needs updating first. If the parse error is schema drift (field renamed or added by a recent spec), note which field and stop.
  - **Runtime tick error** (simulation crashes mid-run, e.g., `PreconditionFailed`, missing component): Diagnose whether it is (a) a scenario data issue (wrong value, missing item) -> stop and report, or (b) a code/loader bug (missing component not set during spawn, incorrect precondition) -> fix the code, run the affected crate's tests to verify no regressions, rebuild the observer, and re-run. Note the fix in the report's Run Summary section.
  - **Mid-simulation crash with no dump**: If the observer crashes during tick execution and the dump is not written because the observer calls `std::process::exit(1)` before reaching the dump-writing code, check the observer's tick error handler in `observer.rs`. If it uses `std::process::exit(1)`, change it to `break` from the tick loop so the observer still writes a partial dump with data collected up to the crash tick. A partial dump is far more useful than no dump. Note the crash tick, error message, and any observer fix in the report's Observer Notes section.
  - **Other errors** (permissions, I/O): Stop and report.

**Hard gate**: If `reports/scenario-analysis-dump.md` does not exist or is empty after the run, stop and report.

### Step 2: Read the Dump

1. Read `reports/scenario-analysis-dump.md`.
2. If the file exceeds 500 lines, read section by section using headers (`## Section N`) to navigate with offset-based reads. Build an entity-name mapping from Section 1 (agents and places tables) — all subsequent sections reference entities by EntityId (e.g., `e5g0`). Use agent and place names (not EntityIds) throughout the report; when quoting raw dump data that uses EntityIds, translate to names in your analysis. Section 1 only maps agents and places. Item EntityIds appearing in failed plan attempts and blocked desires cannot be translated — leave them as-is.

The dump does not currently indicate ControlSource per agent. If the scenario file is accessible, check AgentDef entries for `control_source: Human`. Otherwise, note which agent has no AI-driven goal selection in Section 7 (no planning ticks) as a heuristic for human control. This matters for smell 3 (stuck agents) — human-controlled agents with no input will appear stuck, which is expected behavior.

The dump has 7 sections:
- **Section 1**: Run Metadata (scenario, seed, ticks, agents, places)
- **Section 2**: Per-Agent Summary (actions, perception, needs, locations, idle ticks, behavioral transitions, death tick/cause if applicable)
- **Section 3**: Anomaly Flags (mechanically detected smells)
- **Section 4**: Raw Event Sample (first/last 100 events)
- **Section 5**: Per-Agent Belief Summary (known entities, believed locations, social/told/heard/institutional beliefs). Uses item type names (e.g., "Waste", "Apple"), not EntityIds.
- **Section 6**: End-State Inventory & Resources (agent possessions, place contents). Places with 500+ SocialArtifacts from post_notice/tell spam appear as extremely long single lines — note the pollution count and skip individual enumeration.
- **Section 7**: Per-Agent Decision Summary (planning outcomes, goal selection, failed plans, blocked desires, affordances)

**Section 7 reading protocol**: Section 7 lines are extremely dense — individual rows can exceed 5000 tokens. Never use Read with `limit` > 10 lines for Section 7. For each agent, extract in this order:

1. Grep `Tick breakdown` and `Plan search outcomes` — establishes planning health baseline
2. `bash grep 'Goals selected' <dump>` — reveals goal types (too long for Grep tool)
3. Grep `Failed plan attempts` with `-A 30` — shows planning failures and root causes
4. Grep `Blocked desires` with `-A 10` — may be absent; skip if not found
5. Grep `Affordances available`, `Affordances after travel`, and `Final affordances` with `-A 15`
6. For specific decision timeline rows: `bash sed -n 'Xp' <file> | head -c 3000` where X is the line number from a prior Grep hit

### Step 3: Triage Checkpoint

After reading Sections 1, 2, and 3, evaluate whether any agent has any need above 750 permille for 100+ consecutive ticks (from Section 2 "Ticks above 750 permille" and Section 3 Smell 5/6 flags).

**If NO agent meets this threshold** (healthy scenario):
1. Perform lightweight extraction using this protocol (a reduced version of the full Section 7 reading protocol):
   1. Grep `Tick breakdown` and `Plan search outcomes` — planning health baseline
   2. `bash grep 'Goals selected' <dump>` — goal types (too long for Grep tool)
   3. Grep `Failed plan attempts` with `-A 5` — any planning failures
   4. Grep `Blocked desires` with `-A 5` — may be absent; skip if not found
   5. Grep `Final affordances` with `-A 15` — available actions at end of sim
   6. Read Section 5 (beliefs) and Section 6 (end-state) in full
   7. Optionally read Section 4 last events — check whether Discovery events dominate (>50% of the last 100 events), which signals perception bloat from ground item accumulation (Waste, consumed item remnants). If so, note the affected location(s) and cross-reference with Section 6 place contents.
2. Run Layer 1 (lighter — smells unlikely to be severe) and Layer 3 (always runs — detection gaps matter even in healthy scenarios).
3. Skip Layer 2 entirely (no needs failures to diagnose).
4. Use the Healthy Scenario Report Template.

**If ANY agent meets the threshold**:
1. Continue full extraction (all sections including 5, 7 in full, and 8 if present).
2. Run all three layers.
3. Use the Standard Report Template.

### Step 4: Layer 1 — Behavioral Smell Analysis

Analyze the dump for all 10 smell categories. For each, state whether the smell was detected, its severity (CRITICAL / HIGH / MEDIUM / LOW / NONE / INCONCLUSIVE), and your reasoning. Use INCONCLUSIVE when insufficient trace data prevents confident assessment — explain the data limitation.

**Mechanically flagged smells** (already in Section 3 — add narrative context and root-cause hypotheses):

1. **Redundant Perception** — Agent observes the same unchanged entity repeatedly. Suggests overly broad perception or belief never updating.

2. **Action Loops** — Agent repeats the same action sequence (not patrol) without progress. Cross-reference with Section 7's decision timeline to see what the planner was selecting during the loop period. Also look for:
   - **Behavioral collapse**: agents settling into a minimal-action pattern (e.g., only sleep+relieve) for extended periods. Section 2 includes pre-computed behavioral transition markers — use these as starting points, then verify against Section 7 Decision timeline bins. Behavioral transitions in the last 100 ticks of the simulation with all needs below 300 permille are typically end-of-simulation artifacts — sleep is the correct low-urgency default. Note but do not escalate unless needs were rising at the time of the transition.
   - **Degenerate plan loops**: Section 7 shows the same goal selected repeatedly with plans found but 0 actions executed. Grep for `GoalSatisfied\[steps=0` — if an agent has hundreds of these across multiple 100-tick bins, it confirms a degenerate plan loop.
   - **Affordance-reporting gaps**: if an action type appears frequently in the action timeline but is absent from all affordance snapshots, note this discrepancy in Cross-Cutting Patterns.

3. **Stuck Agents** — No actions for many consecutive ticks. Distinguish explainable idle (human-controlled, needs satisfied, no affordances) from pathological (needs rising, agent does nothing). Check Section 7 for planner outcomes during the idle period. Also check whether candidate count dropped to 0 — the agent may be idle because no goal candidates were generated at all. If the agent has dead ticks, their idle status post-death is expected — focus on ticks leading to death. Note: the mechanical stuck-agent detector counts consecutive ticks with no action *started or in-progress*. Multi-tick actions like sleep *usually* occupy the agent and are not counted as idle, but travel+multi-tick-action sequences (e.g., travel→wash→travel, or travel→harvest→travel) can still register as stuck windows — the detector behavior is not 100% reliable for composite maintenance trips. Before classifying a flagged window as a false positive, verify against Section 7 decision timeline and Section 4 ActionStarted/ActionCommitted pairs inside the window: if continuous active frames or action-lifecycle pairs cover the window, it is genuinely a false positive; otherwise, investigate further. Therefore "max consecutive idle ticks" in Section 2 may exceed the detector's threshold without triggering an anomaly, and conversely an anomaly may fire on windows that contain active multi-tick work.

4. **Failed Action Spirals** — Agent keeps attempting actions that fail validation. What precondition is failing? Is the agent's belief stale?

5. **Sustained Critical Needs** — A need stays above 750 permille for 100+ consecutive ticks. Cross-reference with the agent's actions during that tick range and with Section 7's failed plan attempts. Distinguish `frontier-exhausted` (plan definitively not found) from `budget-exhausted` (search space too large — plan may exist but can't be found within budget). Note candidate counts and max depth.

6. **Unaddressed Needs** — Need average exceeds 750 permille but no corresponding relief action (eat/drink/sleep/toilet/wash) was ever attempted. Cross-reference with Section 7's blocked desires and affordances. If the relief action doesn't appear in the latest affordance snapshot, it's a missing affordance.

**LLM-only smells** (cross-reference dump sections to detect):

7. **Impossible Knowledge** — Did an agent act on information about an entity they never observed and never heard about through Tell/AskWitness? Cross-reference action targets vs. entities in perception trace.

8. **Belief Staleness** — Cross-reference belief summary (Section 5) with action traces, perception traces, and end-state inventory (Section 6). Does the agent believe resources exist at locations they haven't visited recently? Do believed entity locations match current placement?

9. **Social Isolation** — Agents co-located for 20+ ticks with no Tell, AskWitness, or Trade actions. Also flag: no Trade despite complementary needs/inventory, heavily unidirectional social actions, role-specific social actions unused, tell actions producing SocialArtifacts with no behavior change.

10. **Economic Stagnation** — Agents with unmet needs (hunger/thirst > 500 permille) in locations with resource sources (Section 6), but no harvest/craft/trade actions attempted. Cross-reference beliefs (Section 5) with place contents (Section 6). Section 7's failed plan attempts reveal whether agents tried economic actions and failed.

**Known Pathology Signatures** — recurring patterns for faster diagnosis:

- **FreeCarryCapacity degenerate loop**: Inventory fills with Waste, `GoalSatisfied[steps=0]` repeats 50+ times per bin, zero actions executed. Cross-reference Section 6 inventory and smell 10.
- **AcquireCommodity budget exhaustion spiral**: Multi-location plan generates 1000-6000+ candidates at depth 5-9, exceeding budget every time. Manifests as sustained critical needs (smell 5) despite commodity existing at reachable location.
- **Obligation spam loop**: Fast-completing obligation action (post_notice, investigate) fires 50+ times per bin while survival needs critical. The obligation goal's drive score overwhelms hunger/thirst/fatigue. Distinct from other signatures: plans succeed, actions execute, but the wrong goal is chosen.
- **Sleep+relieve behavioral collapse**: Action repertoire narrows to only sleep and relieve_wilderness for 500+ ticks. All non-trivial goals fail planning or lack local affordances. Often caused by geographic food desert.

After analyzing all 10 smells, note any cases where trace data was insufficient for confident assessment. Record which data gaps affected which smells — this feeds Layer 3.

### Step 5: Layer 2 — Needs Diagnostics (Conditional)

**Skip this step entirely if the triage checkpoint (Step 3) determined no agent has needs failures.**

#### Step 5.1: Classify Each Agent's Needs Failure

For each agent with any need >750 permille for 100+ consecutive ticks, classify the root cause into one or more categories:

| Category | Signature | How to Detect |
|----------|-----------|---------------|
| **Geographic Desert** | Agent at location with no local affordance for the need; AcquireCommodity budget-exhausts | Section 6: no relevant facility. Section 7: budget-exhausted. Affordances lack eat/drink/wash/harvest. |
| **Planner Budget Wall** | Resource exists at reachable location but plan search exceeds expansion budget | Section 7: budget-exhausted with 500+ candidates, depth 5+. Section 6: resource exists elsewhere connected by travel edges. |
| **Belief Blindness** | Agent lacks beliefs about resource-rich locations | Section 5: agent doesn't know about resource locations. Also covers facility-specific blindness: recipe exists but facility location unknown. |
| **Priority Override** | Agent has affordances for need relief but another goal consistently outranks it, so relief fires rarely or never | Section 7: non-needs goal dominates during critical-need period. Affordances DO include relief action but it fires at a frequency insufficient to keep the need below threshold (or never fires). Distinguish "never fires" from "fires too infrequently" — both are Priority Override. For "fires too infrequently," compare relief-action count vs. need-accumulation rate over the run: if `relief_rate < need_accumulation_rate` across a rolling 200+ tick window, classify as Priority Override regardless of whether the action ever committed. |
| **Structural Impossibility** | No location in the entire scenario has the needed resource/facility | Scenario inspection: no wash-capable facility exists anywhere, etc. |
| **Exploration Failure** | Agent has exploration_profile but ExploreLocation never fires or fails | Section 7: ExploreLocation in failed plans/blocked desires, or never appears in goals-selected despite having the profile. |
| **Belief Memory Pollution** | Agent previously knew resource locations but beliefs displaced by irrelevant entities | Section 5: memory at capacity dominated by SocialArtifacts/Waste. Section 2: agent visited resource locations earlier. Distinct from Belief Blindness (never learned vs. crowded out). |
| **Knowledge Gap** | Agent believes location exists but lacks recipe to exploit it | Section 5: knows facility location. Scenario: agent's known_recipes missing required recipe. |
| **Profile Gap** | Agent missing profile component enabling survival behavior | Compare scenario agent definition against registered profiles. Common: no PerceptionProfile (blind), no CognitiveProfile (default budget), no ExplorationProfile (no exploration drive). |

An agent may have multiple categories. For agents that died, trace the causal chain backward from death.

**Profile gap cross-reference**: When classifying as Profile Gap, check:
- Priority Override without `ObligationSatiationProfile` -> satiation mechanism inactive
- No exploration without `ExplorationProfile` -> exploration drive inactive
- Default planner budget without `CognitiveProfile` -> engine defaults may be too low
- No item observation without `PerceptionProfile` -> agent can't see ground items

#### Step 5.2: Capture Damning Moments

For each classified failure, extract a "damning moment" — the exact agent state at the point where the failure becomes irrecoverable or clearly pathological.

**Identifying the damning tick**: The most reliable source is Section 3 anomaly flag tick ranges (e.g., "hunger above 750 permille for 274 consecutive ticks (ticks 269-542)" -> tick 269). Refine with:
- Geographic Desert: Section 3 start tick + Section 2 location history
- Planner Budget Wall: First budget-exhausted attempt from Section 7 or Section 8
- Belief Blindness: Tick when needs crossed high threshold with no resource beliefs
- Belief Memory Pollution: Tick when belief memory became dominated by non-resource entities
- Priority Override: Tick when overriding goal first outranked needs during critical levels
- Exploration Failure: Tick when ExploreLocation first failed or was needed but absent

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

**Actual behavior**: [what the agent did instead. Source from Section 7 decision timeline bins and Section 2 action counts.]

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

#### Step 5.3: Propose Solutions

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

### Step 6: Layer 3 — Detection Meta-Analysis

**This step always runs**, regardless of whether the scenario is healthy or has failures. Detection quality matters in both cases — healthy scenarios may be masking problems that better detection would catch.

Layer 3 evaluates the anomaly detection system itself by cross-referencing raw trace data against what was flagged (and what wasn't).

#### Step 6.1: False Positive Assessment

Review every anomaly flagged in Section 3 (mechanical) and every smell identified in Layer 1 (LLM-only). For each, assess whether it is a **true positive** or a **false positive**.

A flagged anomaly is a **false positive** if:
- The behavior is expected given the agent's role, scenario design, or control source (e.g., human-controlled agent flagged as stuck)
- The behavior is a correct adaptation to the scenario constraints (e.g., agent loops sleep+relieve because that's genuinely the best available action at a resource-poor location — the problem is the scenario, not the agent)
- The threshold is too sensitive for this scenario type (e.g., 20-tick idle flagged as stuck in a scenario where agents need 30 ticks between resource runs)
- The detector pattern-matches on surface behavior without considering intent (e.g., redundant perception flagged for an entity that the agent *should* re-observe because it changes state between observations — the refinement missed it)

For each false positive, document:
- **Smell**: Which smell category
- **Agent(s)**: Affected agents
- **Why it's false**: Concrete reasoning
- **Detector improvement**: What change to the detector would prevent this false positive (threshold adjustment, additional filtering, context-awareness)

#### Step 6.2: Detection Gap Analysis

Scan the trace data for problematic behaviors that are NOT caught by any of the 6 mechanical anomaly kinds or 4 LLM-only smells. These are behaviors visible in the dump but that no current detector flags.

**Systematic scan approach**: For each agent, cross-reference:
- Section 7 Decision timeline vs. Section 2 needs trajectories: Are there periods where needs rise but the action pattern doesn't change?
- Section 7 goal selection vs. Section 7 affordances: Are affordances available that are never selected?
- Section 7 plan outcomes vs. Section 2 action counts: Are plans found but actions never committed?
- Section 5 beliefs vs. Section 6 reality: Are there belief-reality mismatches beyond what smell 8 covers?
- Section 2 location history vs. Section 7 goals: Is travel purposeful or aimless?
- Section 2 perception counts vs. Section 5 beliefs: Are observations being made but beliefs not forming?

**Common gap patterns to look for**:
- **Aimless travel**: Agent repeatedly travels between locations without executing any goal-relevant action at destinations. Travel serves no purpose.
- **Resource hoarding**: Agent acquires resources far beyond consumption rate while other agents starve. No sharing or trade despite co-location.
- **Goal oscillation**: Planner alternates between two goals every few ticks without making progress on either. Neither goal completes because the other keeps interrupting.
- **Perception without belief formation**: Agent observes entities but beliefs don't update (observations pass but belief store shows no corresponding entries).
- **Belief-action disconnect**: Agent has correct beliefs about resource locations but never plans actions toward those locations.
- **Silent plan degradation**: Plan quality drops over time (more budget exhaustions, fewer plans found) without any change in scenario conditions — suggests accumulating state pollution.
- **Asymmetric agent outcomes**: Agents with identical profiles and similar starting conditions have vastly different outcomes — suggests hidden sensitivity to initial placement or stochastic choices.
- **Dead-end exploration**: ExploreLocation succeeds (agent visits new places) but never leads to resource discovery because the explored places are also resource-poor.
- **Action timing pathology**: Agent executes correct actions but at wrong times (e.g., eating when hunger is low, sleeping when fatigue is low) suggesting priority inversion.
- **Geographic convergence**: All or most agents settle at the same subset of locations, leaving other places effectively unused. Compare Section 2 location ticks across agents — if 2+ agents spend >60% of ticks at the same location(s) while other places get <5%, the scenario's spatial design may be collapsing to a dominant corridor. This is a scenario design signal, not necessarily an agent bug.
- **Single-source resource dependency**: All consumption of a commodity type (food, water) comes from one resource source when alternatives exist in the scenario. Compare action counts (harvest types) against scenario resource_sources. If an entire commodity class is sourced from one facility while agents have recipes for unused alternatives, agents lack resilience to disruption.

For each detected gap, document:
- **Pattern name**: Short descriptive label
- **Evidence**: Specific data from the dump showing this behavior
- **Agent(s)**: Affected agents
- **Why current detectors miss it**: Which existing smell is closest and why it doesn't cover this case
- **Impact**: How this undetected behavior affects agent outcomes (severity equivalent: CRITICAL / HIGH / MEDIUM / LOW)

#### Step 6.3: Threshold Assessment

Evaluate whether the current mechanical anomaly thresholds are appropriate for this scenario:

| Threshold | Current Value | Assessment | Recommendation |
|-----------|--------------|------------|----------------|
| Stuck agent idle ticks | 20 consecutive ticks | [Too low / Appropriate / Too high] | [Suggested value if change needed, with reasoning] |
| Redundant perception count | 10 observations | [Too low / Appropriate / Too high] | [Suggested value] |
| Critical need threshold | 750 permille | [Too low / Appropriate / Too high] | [Suggested value] |
| Sustained critical duration | 100 consecutive ticks | [Too low / Appropriate / Too high] | [Suggested value] |
| Failed action spiral rate | >75% failure with 5+ attempts | [Too low / Appropriate / Too high] | [Suggested value] |
| Unaddressed need average | 750 permille | [Too low / Appropriate / Too high] | [Suggested value] |

Base the assessment on what this specific scenario reveals. A threshold that works for a survival-baseline scenario may be wrong for a trade-heavy or combat scenario.

#### Step 6.4: Proposed New Smell Categories

For each detection gap identified in Step 6.2 with MEDIUM or higher impact, propose a concrete new smell category:

```markdown
#### Proposed Smell [N]: [Name]

**Detection logic**: [How to detect this mechanically in the observer binary or via LLM analysis]
**Threshold**: [Specific values — e.g., "3+ consecutive travels with no non-travel action between them"]
**Mechanical vs. LLM**: [Can this be detected mechanically in the observer binary, or does it require LLM cross-referencing?]
**Implementation scope**: [Observer binary change / New dump section / LLM-only analysis instruction]
**Example from this run**: [Concrete instance from the current scenario showing the pattern]
**False positive risk**: [What benign behavior could trigger this detector, and how to filter it]
```

### Step 7: Write Report

If `reports/scenario-analysis-report.md` already exists, check `git status` for the file. If it has uncommitted changes, warn the user before overwriting. If committed (or untracked), overwrite directly — git history preserves the prior version.

#### Standard Report Template

Use when the triage checkpoint (Step 3) found needs failures.

```markdown
# Scenario Analysis Report

## Run Summary
- **Scenario**: `[path]`
- **Scenario purpose**: [extracted from comments, or "none stated"]
- **Seed**: [N]
- **Ticks simulated**: [N]
- **Agents**: [names and starting locations]
- **Places**: [names]
- **Total events**: [N]
- **Deaths**: [agent at tick N (cause), or "None"]

### Pre-flight Warnings
[From Step 0.5. For each, note whether the run confirmed or contradicted the warning.]

### Observer Notes
[If the observer crashed mid-simulation: crash tick, error message, code fixes applied, whether dump is partial. Omit if no issues.]

---

## Layer 1: Behavioral Smell Analysis

### 1. Redundant Perception — [SEVERITY]
**Agent(s)**: [affected agents]
**Evidence**: [specific data]
**Root cause hypothesis**: [analysis]

### 2. Action Loops — [SEVERITY]
[same structure]

### 3. Stuck Agents — [SEVERITY]
### 4. Failed Action Spirals — [SEVERITY]
### 5. Sustained Critical Needs — [SEVERITY]
### 6. Unaddressed Needs — [SEVERITY]
### 7. Impossible Knowledge — [SEVERITY]
### 8. Belief Staleness — [SEVERITY]
### 9. Social Isolation — [SEVERITY]
### 10. Economic Stagnation — [SEVERITY]

Report all 10 categories regardless of severity. NONE findings should be brief (1-2 sentences). INCONCLUSIVE findings should explain the data limitation.

---

## Layer 2: Needs Diagnostics

### Agent Needs Overview

| Agent | Need | Max Value | Ticks >750 permille | Death? | Root Cause Category |
|-------|------|-----------|---------------------|--------|---------------------|

[One row per agent per need that exceeded 750 permille for 100+ ticks. Healthy agents get a single row: "all needs managed".]

### Failure Classifications

#### [Agent Name]
**Categories**: [list]
**Evidence**: [specific data]
**Confidence**: [HIGH/MEDIUM/LOW]
**Causal chain**: [A -> B -> C summary]

[Repeat for each affected agent]

### Damning Moments

[All captured damning moments in the format specified in Step 5.2]

### Proposed Solutions

#### [Category Name]
[Solutions for each category found. Omit categories not found.]

### Golden Test Recommendations

| Priority | Damning Moment | Test Name Suggestion | What It Guards Against |
|----------|---------------|---------------------|----------------------|
| 1 | DM-[N] | golden_[descriptive_name] | [regression description] |

[Ordered: deaths first, then sustained critical needs, then moderate failures.]

---

## Layer 3: Detection Meta-Analysis

### False Positives

| Smell | Agent(s) | Why It's False | Detector Improvement |
|-------|----------|----------------|---------------------|

[One row per false positive. If none, state "No false positives identified."]

### Detection Gaps

#### Gap [N]: [Pattern Name]
**Evidence**: [specific data]
**Agent(s)**: [affected]
**Why current detectors miss it**: [analysis]
**Impact**: [CRITICAL / HIGH / MEDIUM / LOW]

[Repeat for each gap found. If none, state "No detection gaps identified — current detector coverage appears adequate for this scenario."]

### Threshold Assessment

| Threshold | Current Value | Assessment | Recommendation |
|-----------|--------------|------------|----------------|
| Stuck agent idle ticks | 20 | [assessment] | [recommendation] |
| Redundant perception count | 10 | [assessment] | [recommendation] |
| Critical need threshold | 750 permille | [assessment] | [recommendation] |
| Sustained critical duration | 100 ticks | [assessment] | [recommendation] |
| Failed action spiral rate | >75% / 5+ attempts | [assessment] | [recommendation] |
| Unaddressed need average | 750 permille | [assessment] | [recommendation] |

### Proposed New Smell Categories

[For each MEDIUM+ gap, a concrete proposal as specified in Step 6.4. If no new smells warranted, state "No new smell categories proposed — current coverage is adequate."]

---

## Cross-Cutting Patterns
[Patterns spanning multiple smells, layers, or agents. Entity pollution notes. Interactions between Layer 1 findings and Layer 2 root causes.]

## Planner Diagnostics
[Include only when any agent has budget-exhausted > 0.]

| Agent | Plans Found | Frontier Exhausted | Budget Exhausted | Top Failed Goal | Candidate Count | Max Depth |
|-------|------------|-------------------|-----------------|----------------|-----------------|-----------|

Assessment: [structural vs. parametric budget exhaustion]

## Trend Comparison
[Include only if a prior `scenario-analysis-report.md` exists in git history for the same scenario and seed.]

| Category | Prior Severity | Current Severity | Delta |
|----------|---------------|-----------------|-------|

[If no prior report, omit this section.]

## Summary Statistics
- Layer 1 findings: N (categories with severity other than NONE)
- By severity: N CRITICAL, N HIGH, N MEDIUM, N LOW
- Layer 2: [N damning moments / "not triggered (healthy scenario)"]
- Layer 3: [N false positives, N detection gaps, N new smell proposals]
- Agents with issues: [list]
- Clean agents: [list]
- Scenario purpose achieved: [Yes / No / Partially — brief explanation]
```

#### Healthy Scenario Report Template

Use when the triage checkpoint (Step 3) found no needs failures.

```markdown
# Scenario Analysis Report

## Run Summary
- **Scenario**: `[path]`
- **Scenario purpose**: [extracted or "none stated"]
- **Seed**: [N]
- **Ticks simulated**: [N]
- **Agents**: [names and starting locations]
- **Places**: [names]
- **Total events**: [N]
- **Deaths**: None

### Pre-flight Warnings
[From Step 0.5. For each, note whether run confirmed or contradicted.]

---

## Layer 1: Behavioral Smell Analysis

[Same 10-category structure as standard template, but most will be NONE or LOW.]

---

## Layer 2: Needs Diagnostics

*Not triggered — no agent exceeded 750 permille for 100+ consecutive ticks.*

### Agent Needs Overview

| Agent | Closest-to-Threshold Need | Max Value | Margin to 750 | Planner Health |
|-------|--------------------------|-----------|---------------|----------------|

[One row per agent. "Margin to 750" = 750 - max value.]

### Survival Strategy Summary

For each agent: where they spent time, how they obtained food/water, wash frequency, key action counts, primary survival bases.

### Margins and Risk Observations

[Which needs closest to 750 threshold. What scenario changes could push agents over. Structural observations about resource distribution.]

Note total waste items per location from Section 6. If any location has >30 Waste items, flag as "waste accumulation risk — belief stores may be polluted in longer runs" and note the count. Cross-reference with agent belief stores (Section 5) to check whether Waste entities dominate known-item counts.

---

## Layer 3: Detection Meta-Analysis

[Same structure as standard template — false positives, gaps, thresholds, proposals.]

---

## Cross-Cutting Patterns
## Summary Statistics
- Scenario purpose achieved: [Yes / No / Partially]
```

### Step 8: Clean Up

Delete `reports/scenario-analysis-dump.md` — the dump is an intermediate artifact. The report in `reports/scenario-analysis-report.md` is the deliverable.

## Comparison Mode

When re-running the analysis after a fix (code change, scenario edit, or profile tuning):

1. **Read the previous report** (`reports/scenario-analysis-report.md`) before running the observer.
2. **Run the analysis normally** (Steps 0-8).
3. **Add comparison metadata** to the Run Summary:
   - **Changes since last run**: Specific changes made
   - **Previous run deaths**: [N] -> **This run deaths**: [N]
4. **Add Delta column** to the Agent Needs Overview table:
   - `RESOLVED` — previously >750 for 100+ ticks, now below threshold
   - `IMPROVED` — still above threshold but fewer ticks or lower max
   - `UNCHANGED` — same or similar severity
   - `REGRESSED` — worse than previous run
   - `NEW` — failure not present in previous run
5. **Cross-reference prior DMs**: Note which prior damning moments were resolved, which persist, which are new.
6. **Layer 3 comparison**: Note which prior false positives still apply, which prior gaps are now detected, and whether threshold recommendations from the prior run were applied.

## Notes

- The observer binary outputs to stderr for progress; only the markdown dump file matters.
- Human-controlled agents (ControlSource::Human) with no input will always appear as "stuck" — note this as expected behavior.
- Patrol agents are excluded from action loop detection in the binary, but verify patrol behavior in raw traces.
- 1440 ticks = 1 simulated day. For deeper analysis, try 2880 (2 days) or 4320 (3 days).
- If an agent died, note it prominently in Run Summary. Focus smell analysis on ticks leading to death, not post-death idle.
- Section 8 (Budget Exhaustion Snapshots) may not be present in all observer versions. Fall back to Section 7 failed plan attempts.
- For before/after comparisons, use Comparison Mode (above) rather than running with different output paths.
