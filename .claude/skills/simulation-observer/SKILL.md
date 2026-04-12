---
name: simulation-observer
description: "Run a scenario headlessly with the observer binary, analyze behavioral smells in the simulation, and write a narrative report to reports/simulation-observer-report.md."
user-invocable: true
---

# Simulation Observer

Run a scenario headlessly via the observer binary, read the structured dump, perform behavioral analysis across 10 smell categories (6 mechanical + 4 LLM-only), and write a narrative report.

## Invocation

```
/simulation-observer scenarios/cli-evaluation.ron
/simulation-observer scenarios/cli-evaluation.ron --ticks 720
```

First argument: path to a `.ron` scenario file (required).
Optional `--ticks N` to override the default of 1440 ticks (1 simulated day).

If no scenario path is provided, glob for `scenarios/*.ron` and present the list to the user. If exactly one scenario file exists, confirm it before proceeding. If none exist, stop and report.

## Process

Follow these steps in order. Do not skip any step.

### Step 1: Build Observer

```bash
cargo build -p worldwake-cli --bin observer
```

**Hard gate**: If the build fails, stop and report the error. Do not proceed.

### Step 2: Run Observer

```bash
cargo run -p worldwake-cli --bin observer -- <scenario_path> --ticks <N> --output reports/simulation-observer-dump.md
```

- Use the scenario path provided by the user.
- Use the tick count provided by the user, or default to 1440.
- The observer may take several minutes to write the dump after the last tick completes (the dump-writing phase is CPU-intensive for large simulations). If using background execution, wait for the process to exit rather than checking for the output file — the file is written atomically at the end.
- If the binary exits with a non-zero code, diagnose the failure mode:
  - **Scenario parse error** (missing field, wrong type): Stop and report. The scenario needs updating first. If the parse error is schema drift (field renamed or added by a recent spec), note which field and stop.
  - **Runtime tick error** (simulation crashes mid-run, e.g., `PreconditionFailed`, missing component): Diagnose whether it is (a) a scenario data issue (wrong value, missing item) → stop and report, or (b) a code/loader bug (missing component not set during spawn, incorrect precondition) → fix the code, run the affected crate's tests to verify no regressions, rebuild the observer, and re-run. Note the fix in the report's Run Summary section (what was broken, what file was changed, what the fix was).
  - **Other errors** (permissions, I/O): Stop and report.

**Hard gate**: If `reports/simulation-observer-dump.md` does not exist or is empty after the run, stop and report.

### Step 3: Read the Dump

1. Read `reports/simulation-observer-dump.md`.
2. If the file exceeds 500 lines, read Section 1 (Run Metadata) and Section 2 (Per-Agent Summaries) first, then Section 3 (Anomaly Flags), then Sections 4-7 as needed for smell analysis. Section boundaries vary by dump size — use the section headers (`## Section N`) to orient within offset-based reads. Build an entity-name mapping from Section 1 (agents and places tables) — all subsequent sections reference entities by EntityId (e.g., `e5g0`). Use agent and place names (not EntityIds) throughout the report; when quoting raw dump data that uses EntityIds, translate to names in your analysis. Section 1 only maps agents and places. Item EntityIds appearing in failed plan attempts and blocked desires (e.g., `EntityId { slot: 19, generation: 0 }`) cannot be translated — leave them as-is but note they are item/entity references, not agents or places. Do not guess item names from EntityIds.

The dump has 7 sections:
- Section 1: Run Metadata (scenario, seed, ticks, agents, places)
- Section 2: Per-Agent Summary (actions, perception, needs, locations, idle ticks, behavioral transitions, death tick/cause if applicable)
- Section 3: Anomaly Flags (mechanically detected smells)
- Section 4: Raw Event Sample (first/last 100 events, per-agent action timeline histograms in 100-tick bins, per-agent perception timeline in 100-tick bins showing pass/fail/entity counts, raw tail traces)
- Section 5: Per-Agent Belief Summary (known entities, believed locations, social/told/heard/institutional beliefs). Note: Section 5 uses item type names (e.g., "Waste", "Apple") in believed entity locations, not EntityIds — cross-reference with Section 6's place contents for belief staleness analysis (smell 8).
- Section 6: End-State Inventory & Resources (agent possessions, place contents)
- Section 7: Per-Agent Decision Summary (planning outcomes, goal selection, failed plans, blocked desires, affordances)

The per-agent action timeline in Section 4 shows action counts binned by 100-tick windows, making it easy to identify when behavioral transitions occur (e.g., when an agent stops eating and enters a sleep-only loop). Cross-reference these transition points with needs trajectory and anomaly tick ranges.

Section 7 contains per-agent decision summaries from the GOAP planner. For each agent it shows: planning/active/dead tick breakdown, plan search outcomes (found/frontier-exhausted/budget-exhausted), goals selected, a decision timeline in 100-tick bins using `DecisionOutcome::summary()` one-liners, failed plan attempts with the goal that couldn't be planned and why, fully blocked desires (goals generated but all opportunities blocked), and affordances available at the agent's location. This section directly answers "why didn't the agent do X?" — cross-reference failed plan attempts and blocked desires with the sustained/unaddressed need anomalies.

Section 7 lines are extremely dense — individual decision timeline rows or goals-selected lines can exceed 5000 tokens. Sequential offset reads will frequently hit the Read tool's token limit (10,000 tokens) even at 15-20 line slices. For Section 7, never use Read with `limit` > 10 lines. Decision timeline rows routinely exceed 1000 tokens each. Use `limit: 5` for tick-breakdown and plan-search-outcomes headers.

**Grep is effective for locating Section 7 subsection headers and extracting shorter lines** (failed plan tables, affordance lists, tick breakdowns, plan search outcomes). However, decision timeline rows will always show as `[Omitted long matching line]` due to Grep's line-length limits — Grep cannot read their content. For goals-selected lines that may also be truncated, use `bash grep` as a fallback. **For decision timeline content, use `bash sed -n 'Xp' <file> | head -c 3000`** to read individual rows with byte truncation. This is the only reliable method for rows that exceed 3000 tokens each. If even `Read` with `limit: 5` hits the token cap, fall back to `bash sed` unconditionally.

**Section 7 Extraction Sequence** — for each agent, extract in this order:

1. Grep `Tick breakdown` and `Plan search outcomes` — establishes planning health baseline
2. `bash grep 'Goals selected' <dump>` — reveals what goal types the planner considered (too long for Grep tool)
3. Grep `Failed plan attempts` with `-A 30` — shows planning failures and root causes
4. Grep `Blocked desires` with `-A 10` — shows goals that couldn't be attempted (subsection may be absent)
5. Grep `Affordances available`, `Affordances after travel`, and `Final affordances` with `-A 15` — shows what actions were structurally possible
6. For specific decision timeline rows: `bash sed -n 'Xp' <file> | head -c 3000` where X is the line number from a prior Grep hit

The blocked desires subsection may be absent if no desires were fully blocked. If absent, note this in the analysis rather than treating it as an error — check failed plan attempts and affordances instead as alternative evidence.

Affordances are shown at tick 0 and after each travel arrival, plus a final snapshot at simulation end. Use the latest relevant snapshot for analysis rather than hedging against tick-0 limitations. For agents that never traveled, only the tick-0 snapshot exists.

The binary's mechanical anomaly detection has thresholds — it may miss borderline cases. If you see evidence of a mechanical smell in the action summaries or traces that wasn't flagged in Section 3, analyze it anyway.

### Step 4: Behavioral Smell Analysis

Analyze the dump for all 10 smell categories. For each, state whether the smell was detected, its severity (CRITICAL / HIGH / MEDIUM / LOW / NONE / INCONCLUSIVE), and your reasoning. Use INCONCLUSIVE when insufficient trace data prevents confident assessment — explain the data limitation rather than guessing.

**Mechanically flagged smells** (already in Section 3 -- add narrative context and root-cause hypotheses):

1. **Redundant perception** -- Agent observes the same unchanged entity repeatedly. Why might this be happening? Is the perception system firing too broadly? Is the entity genuinely changing state each time?

2. **Action loops** -- Agent repeats the same action sequence (not patrol) without progress. Is this a planning failure? A missing affordance? A belief that never updates? Cross-reference with Section 7's decision timeline to see what the planner was selecting during the loop period. Also look for behavioral collapse — agents settling into a minimal-action pattern (e.g., only sleep+relieve) for extended periods. Section 2 includes pre-computed behavioral transition markers (e.g., "action repertoire narrowed from 4 types to 2 types at tick 500") — use these as starting points, then verify against the action timeline bins in Section 4. If an agent's action repertoire narrows to 1-2 action types after an identifiable transition point, flag it even if the mechanical detector didn't. Cross-reference with smell 10 to determine whether resource starvation is causing the collapse. Also watch for planning-level loops: if Section 7 shows the same goal selected repeatedly (e.g., FreeCarryCapacity appearing 50+ times in a 100-tick bin) with plans found but 0 actions executed, this indicates a degenerate plan loop — the planner thinks it found a plan but no action fires. Cross-reference with the action timeline: if plans are "found" but no actions appear, the plan is degenerate (0-step GoalSatisfied or ProgressBarrier with no executable step). This manifests as total action cessation despite continuous planning, and is distinct from smell 3 (stuck agents) where the planner itself fails to find plans. To detect degenerate plan loops, Grep Section 7 for `GoalSatisfied\[steps=0` — this pattern matches 0-step plans that produce no action. If an agent has hundreds of these across multiple 100-tick bins, it confirms a degenerate plan loop. Also check whether the same goal appears in consecutive bins via `bash grep 'selected=<GoalName>' <dump>` (substitute the suspected goal).

3. **Stuck agents** -- Agent has no actions for many consecutive ticks. Explainable idle (human-controlled agent with no input, all needs satisfied, no affordances) vs. pathological idle (needs rising but agent does nothing)? Cross-reference with Section 7 to determine if the planner was producing decisions at all during the idle period, and if so, what outcomes it found. Also check whether the planner's candidate count dropped to 0 — the agent may be idle not because plans failed, but because no goal candidates were generated at all (e.g., all goal preconditions unmet, carry capacity full, no affordances producing new goals). This is distinct from frontier/budget exhaustion and may indicate resource saturation or missing affordances rather than planner limitations. If the agent has dead ticks in Section 7, their idle status post-death is expected — focus analysis on the ticks leading to death and the causal chain that produced it.

4. **Failed action spirals** -- Agent keeps attempting actions that fail validation. What precondition is failing? Is the agent's belief stale about the precondition?

5. **Sustained critical needs** -- A need stays above 750 permille for 100+ consecutive ticks. The anomaly includes the tick range. Cross-reference with the agent's actions during that range to determine whether the need was truly ignored or simply unsatisfiable (no resource available). If the agent died, the sustained critical need likely contributed to death — trace the causal chain (rising need → failed plans → no relief → death). Cross-reference with Section 7's failed plan attempts — were plans for the corresponding relief action attempted and failed? Or was the goal never even generated (check blocked desires)? When cross-referencing failed plan attempts, distinguish between `frontier-exhausted` (plan definitively not found — likely a missing affordance or precondition) and `budget-exhausted` (search space too large — the plan may exist but can't be found within the expansion budget). Budget-exhaustion patterns suggest either the action chain is too deep, the search space branches too widely, or the planner budget needs tuning. Note the candidate counts and max depth to characterize the problem.

6. **Unaddressed needs** -- Need average exceeds 750 permille but no corresponding relief action (eat/drink/sleep/toilet/wash) was ever attempted. This strongly suggests a missing affordance or planner gap -- the agent never even tried to address the need. Cross-reference with Section 7's blocked desires (if present — this subsection may be absent; see failed plan attempts and affordances as alternatives) and affordances. If the relief action doesn't appear in the latest affordance snapshot, it's a missing affordance. If the action appears in affordances but the goal is in blocked desires, something is blocking the opportunity. If neither, the goal ranking may never select it.

**LLM-only smells** (cross-reference dump sections to detect):

7. **Impossible knowledge** -- Cross-reference action traces with perception traces. Did an agent act on information about an entity they never observed and never heard about through Tell/AskWitness? Check: agent's action targets vs. entities in their perception trace.

8. **Belief staleness** -- Cross-reference the agent's belief summary (Section 5) with their action traces, perception traces, and the end-state inventory (Section 6). Check: does the agent believe resources exist at locations they haven't visited recently? Do their believed entity locations match current placement? Are they failing to act on resource knowledge (e.g., believing food exists at a place but never traveling there)? Compare the agent's known entities with entities they could have observed -- are there gaps suggesting failed or missed perceptions? If the belief summary is sparse (few known entities), note the limitation rather than speculating. Section 7's affordances list shows what actions the planner could see at the agent's location — if travel isn't in affordances, agents can't plan multi-location journeys. Multiple affordance snapshots are available (tick 0, after each travel, and at simulation end); use the latest relevant snapshot for the time period being analyzed.

9. **Social isolation** -- Check location tracking: if agents are co-located for extended periods (20+ ticks) with no Tell, AskWitness, or Trade actions between them, flag it.

10. **Economic stagnation** -- Check for agents with unmet needs (hunger/thirst > 500 permille) in locations with resource sources or commodity stocks (use Section 6 to verify what resources actually exist at each place), but no harvest/craft/trade actions attempted. Cross-reference agent beliefs (Section 5) with actual place contents (Section 6) to determine whether the agent knows about available resources. Section 7's failed plan attempts and blocked desires directly reveal whether agents tried to plan economic actions (harvest, craft, trade) and failed, or never generated those goals at all. When failed plan attempts show `budget-exhausted` for acquisition goals, this indicates the plan may exist but the search space is too large — note candidate counts and max depth to characterize the bottleneck.

The per-agent summary also includes a "Ticks above 750‰" line for each need, providing concrete data for smells 5-6 and supporting LLM analysis of smells 8-10.

**Known Pathology Signatures** — recurring patterns from prior runs that speed diagnosis:

- **FreeCarryCapacity degenerate loop**: Agent inventory fills with low-value items (typically Waste from consumption byproducts). FreeCarryCapacity scores highest priority, but GoalSatisfied[steps=0] produces no action. Blocks all other goals indefinitely. Signature: `selected=FreeCarryCapacity ... GoalSatisfied[steps=0]` repeating 50+ times per 100-tick bin, with action timeline showing 0 actions. Cross-reference with Section 6 inventory (full of Waste) and smell 10.
- **AcquireCommodity budget exhaustion spiral**: Agent needs food/water but the multi-location acquisition plan (travel → pick up → consume) generates 1000-6000+ candidates at depth 5-9, exceeding the planner's expansion budget every time. Manifests as sustained critical needs (smell 5) despite the commodity existing at a reachable location. Signature: repeated `budget-exhausted` entries for `AcquireCommodity` with high candidate counts and depth.
- **Obligation spam loop**: Agent with role obligations (guard, merchant, official) executes a fast-completing obligation action (post_notice, investigate) hundreds of times while survival needs are critical. The obligation goal's drive score overwhelms hunger/thirst/fatigue because the action completes in 1 tick and re-triggers immediately. Signature: a single non-survival action type appearing 50+ times per 100-tick bin in the action timeline, while needs trajectory shows 1000‰ on multiple needs. Cross-reference with Section 7's goal selection — if the obligation goal consistently outranks AcquireCommodity/ConsumeOwnedCommodity despite critical needs, this is a goal-ranking priority failure, not a planning failure (plans succeed, but the wrong goal is chosen). Distinct from both other signatures: plans are found (not budget-exhausted) and actions do execute (not degenerate 0-step plans).

After analyzing all 10 smells, note any cases where trace data was insufficient to reach a confident assessment. Record which specific data gaps affected which smells -- this feeds the Trace Quality Assessment in Step 5.

### Step 5: Write Report

If `reports/simulation-observer-report.md` already exists, check `git status` for the file. If it has uncommitted changes, warn the user before overwriting. If it's committed (or untracked), overwrite directly — git history preserves the prior version.

Write `reports/simulation-observer-report.md` with this structure:

```markdown
# Simulation Observer Report

## Run Summary
[Copy run metadata from dump: scenario, seed, ticks, agents, places, total events]
**Deaths**: [list agent deaths with tick and cause, or "None"]

## Findings

### 1. Redundant Perception -- [SEVERITY]
### 2. Action Loops -- [SEVERITY]
### 3. Stuck Agents -- [SEVERITY]
### 4. Failed Action Spirals -- [SEVERITY]
### 5. Sustained Critical Needs -- [SEVERITY]
### 6. Unaddressed Needs -- [SEVERITY]
### 7. Impossible Knowledge -- [SEVERITY]
### 8. Belief Staleness -- [SEVERITY]
### 9. Social Isolation -- [SEVERITY]
### 10. Economic Stagnation -- [SEVERITY]

Each finding uses this structure:
**Agent(s)**: [affected agents]
**Evidence**: [specific data from dump supporting this finding]
**Root cause hypothesis**: [your analysis of why this is happening]
**Confidence**: [how confident you are this is a real issue vs. expected behavior]

Report all 10 categories regardless of severity. NONE findings should be brief (1-2 sentences confirming no detection). INCONCLUSIVE findings should explain the data limitation.

## Cross-Cutting Patterns
[Patterns that span multiple smells or agents -- e.g., "Agent X has both stuck behavior AND ignored needs, suggesting a planning failure"]

Check for entity pollution — actions that create persistent world entities (post_notice creating SocialArtifacts, tell creating SocialArtifacts) can flood a location with hundreds of entities, amplifying redundant perception (smell 1), bloating the planner's candidate space (smell 2/10), and obscuring meaningful inventory in Section 6. If a single action type produced 100+ entities at one location, note the pollution source, the affected location, and which smells it compounds.

## Planner Diagnostics
[Include this section only when any agent has budget-exhausted > 0 in Section 7's plan search outcomes.]

Per-agent planner summary:

| Agent | Plans Found | Frontier Exhausted | Budget Exhausted | Top Failed Goal | Candidate Count (typical) | Max Depth |
|-------|------------|-------------------|-----------------|----------------|--------------------------|-----------|
| ... | ... | ... | ... | ... | ... | ... |

Assessment: [1-2 sentences: Is budget exhaustion structural (design issue — the plan exists but the search space is too large by construction) or parametric (budget too low — raising max_node_expansions would help)?]

## Summary Statistics
- Total findings: N (count of categories with severity other than NONE)
- By severity: N CRITICAL, N HIGH, N MEDIUM, N LOW (NONE categories excluded)
- Agents with issues: [list]
- Clean agents: [list]

## Trace Quality Assessment

### Trace Sufficiency
[1-2 sentences: Does the dump provide enough information to confidently assess all 10 smells?]

### Limitations and Recommended Additions

For each limitation identified during analysis, classify it:

| ID | Limitation | Classification | Rationale |
|----|-----------|----------------|-----------|
| TQ-1 | [description] | Actionable / Acceptable trade-off | [why -- what analysis would improve, or why the current state is adequate] |

**Actionable** items are limitations where:
- The missing data prevented confident assessment of one or more smells (forced INCONCLUSIVE), OR
- The missing data would materially improve root-cause diagnosis for a MEDIUM+ finding, OR
- The limitation misaligns with a FOUNDATIONS principle (cite which one)

**Acceptable trade-off** items are limitations where:
- The data would be nice but didn't prevent any smell assessment
- The cost of capturing the data outweighs the diagnostic benefit
- The limitation is inherent to the dump format and not worth engineering around

For each **Actionable** item, also include:
- **Recommended addition**: What concrete change would address this limitation
- **Scope**: Observer-binary enhancement (change to the observer dump output) vs. Engine instrumentation (new component, event, or system in the simulation crates)
```

### Step 6: Clean Up

Delete `reports/simulation-observer-dump.md` -- the dump is an intermediate artifact. The report in `reports/simulation-observer-report.md` is the deliverable.

## Notes

- The observer binary outputs to stderr for progress; only the markdown dump file matters.
- Human-controlled agents (ControlSource::Human) with no input will always appear as "stuck" -- note this as expected behavior, not a bug.
- Patrol agents are excluded from action loop detection in the binary, but verify patrol behavior looks reasonable in the raw traces.
- 1440 ticks = 1 simulated day. For deeper analysis, try 2880 (2 days) or 4320 (3 days).
- If an agent died, Section 2 includes the death tick and cause (e.g., `**Death**: Tick 422 (cause: NeedDeprivation { Hunger })`). Note this prominently in the Run Summary. Section 7 will also show `dead` ticks > 0. Adjust smell analysis accordingly: a dead agent's "stuck" status post-death is expected, not pathological. Focus analysis on what led to death: trace the unaddressed needs, missing affordances, and failed plans in the ticks before death. Report the approximate death tick and contributing factors in the Cross-Cutting Patterns section.
- For before/after comparisons (e.g., validating a fix), run the observer twice with different `--output` paths (e.g., `reports/sim-dump-before.md` and `reports/sim-dump-after.md`). Compare Summary Statistics sections and specific smell severities across runs.
