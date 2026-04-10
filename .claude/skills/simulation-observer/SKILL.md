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
- If the binary exits with a non-zero code, diagnose the failure mode:
  - **Scenario parse error** (missing field, wrong type): Stop and report. The scenario needs updating first. If the parse error is schema drift (field renamed or added by a recent spec), note which field and stop.
  - **Runtime tick error** (simulation crashes mid-run, e.g., `PreconditionFailed`, missing component): Diagnose whether it is (a) a scenario data issue (wrong value, missing item) → stop and report, or (b) a code/loader bug (missing component not set during spawn, incorrect precondition) → fix the code, run the affected crate's tests to verify no regressions, rebuild the observer, and re-run. Note the fix in the report's Run Summary section (what was broken, what file was changed, what the fix was).
  - **Other errors** (permissions, I/O): Stop and report.

**Hard gate**: If `reports/simulation-observer-dump.md` does not exist or is empty after the run, stop and report.

### Step 3: Read the Dump

1. Read `reports/simulation-observer-dump.md`.
2. If the file exceeds 500 lines, read Section 1 (Run Metadata) and Section 2 (Per-Agent Summaries) first, then Section 3 (Anomaly Flags), then Sections 4-7 as needed for smell analysis. Section boundaries vary by dump size — use the section headers (`## Section N`) to orient within offset-based reads. Build an entity-name mapping from Section 1 (agents and places tables) — all subsequent sections reference entities by EntityId (e.g., `e5g0`). Use agent and place names (not EntityIds) throughout the report; when quoting raw dump data that uses EntityIds, translate to names in your analysis.

The dump has 7 sections:
- Section 1: Run Metadata (scenario, seed, ticks, agents, places)
- Section 2: Per-Agent Summary (actions, perception, needs, locations, idle ticks)
- Section 3: Anomaly Flags (mechanically detected smells)
- Section 4: Raw Event Sample (first/last 100 events, per-agent action timeline histograms in 100-tick bins, per-agent perception timeline in 100-tick bins showing pass/fail/entity counts, raw tail traces)
- Section 5: Per-Agent Belief Summary (known entities, believed locations, social/told/heard/institutional beliefs)
- Section 6: End-State Inventory & Resources (agent possessions, place contents)
- Section 7: Per-Agent Decision Summary (planning outcomes, goal selection, failed plans, blocked desires, affordances)

The per-agent action timeline in Section 4 shows action counts binned by 100-tick windows, making it easy to identify when behavioral transitions occur (e.g., when an agent stops eating and enters a sleep-only loop). Cross-reference these transition points with needs trajectory and anomaly tick ranges.

Section 7 contains per-agent decision summaries from the GOAP planner. For each agent it shows: planning/active/dead tick breakdown, plan search outcomes (found/frontier-exhausted/budget-exhausted), goals selected, a decision timeline in 100-tick bins using `DecisionOutcome::summary()` one-liners, failed plan attempts with the goal that couldn't be planned and why, fully blocked desires (goals generated but all opportunities blocked), and affordances available at the agent's location. This section directly answers "why didn't the agent do X?" — cross-reference failed plan attempts and blocked desires with the sustained/unaddressed need anomalies.

Section 7 is uniformly dense — decision timelines, failed plan tables, goals selected lines, and affordance lists all contain wide rows or long single lines. Use Grep for all targeted extraction in this section (e.g., `Failed plan attempts`, `Blocked desires`, `Affordances available`, `Goals selected`) rather than sequential offset reads. For goals selected lines that may be truncated by Grep's line-length limits, use `bash grep` as a fallback.

The blocked desires subsection may be absent if no desires were fully blocked. If absent, note this in the analysis rather than treating it as an error — check failed plan attempts and affordances instead as alternative evidence.

Affordances shown are a snapshot from the agent's starting location at tick 0. Agents that traveled will have different affordances at their current location. Qualify affordance-based conclusions accordingly — a "missing" affordance may reflect the tick-0 snapshot, not the agent's situation at the time of the unmet need.

The binary's mechanical anomaly detection has thresholds — it may miss borderline cases. If you see evidence of a mechanical smell in the action summaries or traces that wasn't flagged in Section 3, analyze it anyway.

### Step 4: Behavioral Smell Analysis

Analyze the dump for all 10 smell categories. For each, state whether the smell was detected, its severity (CRITICAL / HIGH / MEDIUM / LOW / NONE / INCONCLUSIVE), and your reasoning. Use INCONCLUSIVE when insufficient trace data prevents confident assessment — explain the data limitation rather than guessing.

**Mechanically flagged smells** (already in Section 3 -- add narrative context and root-cause hypotheses):

1. **Redundant perception** -- Agent observes the same unchanged entity repeatedly. Why might this be happening? Is the perception system firing too broadly? Is the entity genuinely changing state each time?

2. **Action loops** -- Agent repeats the same action sequence (not patrol) without progress. Is this a planning failure? A missing affordance? A belief that never updates? Cross-reference with Section 7's decision timeline to see what the planner was selecting during the loop period. Also look for behavioral collapse — agents settling into a minimal-action pattern (e.g., only sleep+relieve) for extended periods. Check the action timeline bins: if an agent's action repertoire narrows to 1-2 action types after an identifiable transition point, flag it even if the mechanical detector didn't. Cross-reference with smell 10 to determine whether resource starvation is causing the collapse.

3. **Stuck agents** -- Agent has no actions for many consecutive ticks. Explainable idle (human-controlled agent with no input, all needs satisfied, no affordances) vs. pathological idle (needs rising but agent does nothing)? Cross-reference with Section 7 to determine if the planner was producing decisions at all during the idle period, and if so, what outcomes it found. If the agent has dead ticks in Section 7, their idle status post-death is expected — focus analysis on the ticks leading to death and the causal chain that produced it.

4. **Failed action spirals** -- Agent keeps attempting actions that fail validation. What precondition is failing? Is the agent's belief stale about the precondition?

5. **Sustained critical needs** -- A need stays above 750 permille for 100+ consecutive ticks. The anomaly includes the tick range. Cross-reference with the agent's actions during that range to determine whether the need was truly ignored or simply unsatisfiable (no resource available). If the agent died, the sustained critical need likely contributed to death — trace the causal chain (rising need → failed plans → no relief → death). Cross-reference with Section 7's failed plan attempts — were plans for the corresponding relief action attempted and failed? Or was the goal never even generated (check blocked desires)? When cross-referencing failed plan attempts, distinguish between `frontier-exhausted` (plan definitively not found — likely a missing affordance or precondition) and `budget-exhausted` (search space too large — the plan may exist but can't be found within the expansion budget). Budget-exhaustion patterns suggest either the action chain is too deep, the search space branches too widely, or the planner budget needs tuning. Note the candidate counts and max depth to characterize the problem.

6. **Unaddressed needs** -- Need average exceeds 750 permille but no corresponding relief action (eat/drink/sleep/toilet/wash) was ever attempted. This strongly suggests a missing affordance or planner gap -- the agent never even tried to address the need. Cross-reference with Section 7's blocked desires and affordances. If the relief action doesn't appear in affordances, it's a missing affordance — but note that affordances are shown at tick 0 only, so an agent that traveled may have different affordances at their current location. If the action appears in affordances but the goal is in blocked desires, something is blocking the opportunity. If neither, the goal ranking may never select it.

**LLM-only smells** (cross-reference dump sections to detect):

7. **Impossible knowledge** -- Cross-reference action traces with perception traces. Did an agent act on information about an entity they never observed and never heard about through Tell/AskWitness? Check: agent's action targets vs. entities in their perception trace.

8. **Belief staleness** -- Cross-reference the agent's belief summary (Section 5) with their action traces, perception traces, and the end-state inventory (Section 6). Check: does the agent believe resources exist at locations they haven't visited recently? Do their believed entity locations match current placement? Are they failing to act on resource knowledge (e.g., believing food exists at a place but never traveling there)? Compare the agent's known entities with entities they could have observed -- are there gaps suggesting failed or missed perceptions? If the belief summary is sparse (few known entities), note the limitation rather than speculating. Section 7's affordances list shows what actions the planner could see at the agent's current location — if travel isn't in affordances, agents can't plan multi-location journeys. Note: affordances are a tick-0 snapshot at the starting location; agents that traveled may have different affordances later.

9. **Social isolation** -- Check location tracking: if agents are co-located for extended periods (20+ ticks) with no Tell, AskWitness, or Trade actions between them, flag it.

10. **Economic stagnation** -- Check for agents with unmet needs (hunger/thirst > 500 permille) in locations with resource sources or commodity stocks (use Section 6 to verify what resources actually exist at each place), but no harvest/craft/trade actions attempted. Cross-reference agent beliefs (Section 5) with actual place contents (Section 6) to determine whether the agent knows about available resources. Section 7's failed plan attempts and blocked desires directly reveal whether agents tried to plan economic actions (harvest, craft, trade) and failed, or never generated those goals at all. When failed plan attempts show `budget-exhausted` for acquisition goals, this indicates the plan may exist but the search space is too large — note candidate counts and max depth to characterize the bottleneck.

The per-agent summary also includes a "Ticks above 750‰" line for each need, providing concrete data for smells 5-6 and supporting LLM analysis of smells 8-10.

After analyzing all 10 smells, note any cases where trace data was insufficient to reach a confident assessment. Record which specific data gaps affected which smells -- this feeds the Trace Quality Assessment in Step 5.

### Step 5: Write Report

If `reports/simulation-observer-report.md` already exists from a previous run, rename it to `reports/simulation-observer-report-prev.md` before writing the new report. After writing the new report, if a previous report exists, add a brief "Changes From Previous Run" section at the end comparing total findings, severity counts, and per-agent status. Delete the previous report file after incorporating the comparison.

Write `reports/simulation-observer-report.md` with this structure:

```markdown
# Simulation Observer Report

## Run Summary
[Copy run metadata from dump: scenario, seed, ticks, agents, places, total events]

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
- If Section 7 shows `dead` ticks > 0 for an agent, note this prominently in the Run Summary and adjust smell analysis accordingly. A dead agent's "stuck" status is expected, not pathological. Focus analysis on what led to death: trace the unaddressed needs, missing affordances, and failed plans in the ticks before death. Report the approximate death tick and contributing factors in the Cross-Cutting Patterns section.
