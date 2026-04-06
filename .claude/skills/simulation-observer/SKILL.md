---
name: simulation-observer
description: "Run a scenario headlessly with the observer binary, analyze behavioral smells in the simulation, and write a narrative report to reports/simulation-observer-report.md."
user-invocable: true
---

# Simulation Observer

Run a scenario headlessly via the observer binary, read the structured dump, perform LLM-driven behavioral analysis across 9 smell categories, and write a narrative report.

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
- If the binary exits with a non-zero code or the scenario fails to parse, stop and report. If the parse error is schema drift (missing field), note it and stop -- the scenario needs updating first.

**Hard gate**: If `reports/simulation-observer-dump.md` does not exist or is empty after the run, stop and report.

### Step 3: Read the Dump

1. Read `reports/simulation-observer-dump.md`.
2. If the file exceeds 500 lines, read Section 1 (Run Metadata) and Section 3 (Anomaly Flags) first, then selectively read per-agent summaries for agents mentioned in anomalies.

### Step 4: Behavioral Smell Analysis

Analyze the dump for all 9 smell categories. For each, state whether the smell was detected, its severity (CRITICAL / HIGH / MEDIUM / LOW / NONE), and your reasoning.

**Mechanically flagged smells** (already in Section 3 -- add narrative context and root-cause hypotheses):

1. **Redundant perception** -- Agent observes the same unchanged entity repeatedly. Why might this be happening? Is the perception system firing too broadly? Is the entity genuinely changing state each time?

2. **Action loops** -- Agent repeats the same action sequence (not patrol) without progress. Is this a planning failure? A missing affordance? A belief that never updates?

3. **Stuck agents** -- Agent has no actions for many consecutive ticks. Explainable idle (human-controlled agent with no input, all needs satisfied, no affordances) vs. pathological idle (needs rising but agent does nothing)?

4. **Failed action spirals** -- Agent keeps attempting actions that fail validation. What precondition is failing? Is the agent's belief stale about the precondition?

**LLM-only smells** (cross-reference dump sections to detect):

5. **Impossible knowledge** -- Cross-reference action traces with perception traces. Did an agent act on information about an entity they never observed and never heard about through Tell/AskWitness? Check: agent's action targets vs. entities in their perception trace.

6. **Ignored urgent needs** -- Cross-reference needs trajectory with action choices. If any need exceeds 750 permille for 5+ consecutive ticks while the agent is doing unrelated actions (not eating, not drinking, not resting, etc.), flag it.

7. **Belief staleness** -- Compare the agent's end-state behavior with events they witnessed. Are they making decisions based on outdated information when fresh data was available?

8. **Social isolation** -- Check location tracking: if agents are co-located for extended periods (20+ ticks) with no Tell, AskWitness, or Trade actions between them, flag it.

9. **Economic stagnation** -- Check for agents with unmet needs (hunger/thirst > 500 permille) in locations with resource sources or commodity stocks, but no harvest/craft/trade actions attempted.

### Step 5: Write Report

Write `reports/simulation-observer-report.md` with this structure:

```markdown
# Simulation Observer Report

## Run Summary
[Copy run metadata from dump: scenario, seed, ticks, agents, places, total events]

## Findings

### [Smell Category Name] -- [SEVERITY]
**Agent(s)**: [affected agents]
**Evidence**: [specific data from dump supporting this finding]
**Root cause hypothesis**: [your analysis of why this is happening]
**Confidence**: [how confident you are this is a real issue vs. expected behavior]

[Repeat for each detected smell]

## Cross-Cutting Patterns
[Patterns that span multiple smells or agents -- e.g., "Agent X has both stuck behavior AND ignored needs, suggesting a planning failure"]

## Summary Statistics
- Total findings: N
- By severity: N CRITICAL, N HIGH, N MEDIUM, N LOW
- Agents with issues: [list]
- Clean agents: [list]

## Trace Quality Assessment
[Does the dump provide enough information to confidently assess all 9 smells? What additional trace data would help? Are there blind spots?]
```

### Step 6: Clean Up

Delete `reports/simulation-observer-dump.md` -- the dump is an intermediate artifact. The report in `reports/simulation-observer-report.md` is the deliverable.

## Notes

- The observer binary outputs to stderr for progress; only the markdown dump file matters.
- Human-controlled agents (ControlSource::Human) with no input will always appear as "stuck" -- note this as expected behavior, not a bug.
- Patrol agents are excluded from action loop detection in the binary, but verify patrol behavior looks reasonable in the raw traces.
- 1440 ticks = 1 simulated day. For deeper analysis, try 2880 (2 days) or 4320 (3 days).
