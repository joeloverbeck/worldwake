---
name: cli-improvement-evaluate
description: "Interactively use the CLI, run mandatory checklists, score 6 metrics, write evaluation to reports/cli-evaluation.md. Invoke after CLI changes to measure improvement."
user-invocable: true
---

# CLI Evaluation

Interactively use the Worldwake CLI like a real user, run mandatory pass/fail checklists for each metric, then score the experience and write a structured evaluation to the report.

## Invocation

```
/cli-improvement:evaluate
```

No arguments. Uses the dedicated evaluation scenario at `scenarios/cli-evaluation.ron`.

## Process

Follow these steps in order. Do not skip any step.

### Step 1: Build and Launch

1. Build the CLI: `cargo build -p worldwake-cli`
2. If the build fails, stop and report the error. Do not evaluate a broken build.
3. Verify the scenario loads: `cargo run -p worldwake-cli -- scenarios/cli-evaluation.ron --exec quit 2>&1`. If it fails with a parse error (missing field, type mismatch), this is a schema-drift bug, not a scenario design issue. Fix the minimal field addition to match the current struct definition, note the fix in the evaluation, and proceed. If the failure is a logic error (not schema drift), stop and report.

### Step 2: Interactive CLI Session

Use the CLI's `--exec` + `--state` mode to run one command per Bash call, reading output between commands and deciding the next command adaptively.

**Initialize the session** with the first command:

```bash
cargo run -p worldwake-cli -- scenarios/cli-evaluation.ron --exec "world" --state /tmp/cli-eval-session.bin
```

**Subsequent commands** reuse the state file:

```bash
cargo run -p worldwake-cli -- scenarios/cli-evaluation.ron --exec "<command>" --state /tmp/cli-eval-session.bin
```

Exercise **all commands** across these 4 workflow sequences. React naturally to output — explore what's interesting, test edge cases, try things that seem like they might break.

**Explore workflow**: `world` -> `places` -> `agents` -> `goods` -> `look` -> `inspect <agent>` -> `relations <agent>` -> `inventory` -> `needs` -> `inventory <other_agent>`

**Act workflow**: `actions` -> `do <N>` (choose an action with >1 tick duration) -> `tick 1` -> `status` -> `cancel` (while action is still running) -> `actions` -> `do <N>` (try a different action) -> `tick 5`

**Control workflow**: `switch <other_agent>` -> `status` -> `actions` -> `look` -> `observe` -> `tick 3` -> `switch <original_agent>`

**Debug workflow**: `events 10` -> `event <id>` (pick an interesting event) -> `trace <id>` -> `save /tmp/cli-eval-save.bin` -> `load /tmp/cli-eval-save.bin` -> `status` -> `help`

**Adaptive exploration**: Between commands, read the output and decide what to explore next. If `look` reveals an interesting entity, `inspect` it. If `actions` shows something suspicious, try `do`-ing it. If `tick` produces many events, `event <id>` the most interesting one.

If a command exits with a non-zero code in `--exec` mode, test the same command in interactive REPL mode to determine whether the issue is CLI-wide or `--exec`-specific. Score based on the CLI behavior, not `--exec` artifacts.

During the session, take notes focused on **informational completeness**:
- Does the tick output tell you what each agent decided and why?
- Does the tick output name the actions started and completed?
- Do event deltas say WHAT changed, not just WHICH component was modified?
- Are there actions listed that crash or error when selected?
- Are command arguments self-explanatory from errors and help text?
- Can you follow a causal chain through trace output?

**Clean up** session files when done: `rm /tmp/cli-eval-session.bin /tmp/cli-eval-save.bin`

### Step 3: Checklist Evaluation

Run each of the 6 mandatory checklists below. These are **binary pass/fail** checks. Record results as PASS or FAIL with a brief note for each item.

#### Checklist 1 — Decision Transparency

- [ ] After ticking at least 3 times, does the tick summary name the goal/intent for each agent that acted? (Not just "1 started" but what goal drove it.)
- [ ] Inspect a Decision event via `event <id>` — does the delta show the goal kind (e.g., "ShareBelief(listener=Kael)") not just "ActiveGoal: set on X"?
- [ ] Can you tell from the tick or event output what goal an agent chose?

#### Checklist 2 — Action Lifecycle Clarity

- [ ] Find an ActionStarted event — does it name the action type (e.g., "Tell", "Travel", "Produce") not just "ActionStarted"?
- [ ] Find an ActionCommitted event — does it name the action type?
- [ ] Run `do <N>` — does the confirmation show what action was requested?
- [ ] Run `status` while an action is in progress — does it show the action name and target? (To test this, use a multi-tick action like travel or tell, then check `status` before ticking to completion.)

#### Checklist 3 — Delta Semantics

- [ ] Pick 3 different events with deltas. For each: does the delta say WHAT changed, not just WHICH component was modified?
- [ ] Is there at least one delta that shows meaningful field values (e.g., "learned location of X at Y") rather than just a component name (e.g., "AgentBeliefStore: set on X")?
- [ ] Are component-level deltas distinguishable from each other? (i.e., you can tell the difference between "set goal" vs "updated beliefs" vs "changed needs")

#### Checklist 4 — Action Reliability

- [ ] Run `actions` and record the count of listed actions.
- [ ] Try EVERY action via `do <N>`. Record which ones succeed and which crash/error. Action numbers may shift between ticks as affordances change — always re-run `actions` immediately before each `do <N>` to ensure the number matches the intended action. Record any numbering mismatches as an Action Reliability issue.
- [ ] Zero crashes or opaque errors? (Actions needing payloads the CLI can't construct must either not appear in the list or must give a clear error explaining what's needed.)

#### Checklist 5 — Command Self-Documentation

- [ ] Run `help` — does each command have a description?
- [ ] Run `trace` without arguments — does the error explain what kind of ID is needed (event ID)?
- [ ] Run `inspect <nonexistent_name>` — does the error suggest valid entity names?
- [ ] Run `switch <nonexistent_name>` — does the error suggest valid agent names?

#### Checklist 6 — Causal Chain Readability

- [ ] Pick an event with an interesting cause chain. Run `trace <id>`. Is the output human-readable? To find events with deeper cause chains, look for events whose cause field shows `Event(N)` rather than `system tick N` or `external input N`. If no such events exist in the first 50 events, note this as a simulation-level gap rather than a CLI display gap.
- [ ] Does each link in the trace show: who acted, what action, what triggered it?
- [ ] Can you follow the chain from consequence back to root cause without guessing?

### Step 4: Read Previous Evaluation

Read `reports/cli-evaluation.md` to determine the evaluation number and review previous scores:
- If the file doesn't exist yet, this is Evaluation #1 — create it with the rubric header (see Step 7).
- If the file exists and is under 300 lines, read it in full. Otherwise, count total lines and read from `offset = totalLines - 200` to get the last 2-3 evaluations.
- To build the Score Trend table (if 3+ evaluations exist), grep for `\*\*Average\*\*` in the report file.

Save the session transcript to `reports/cli-evaluation-transcripts/eval-N.txt`. Use this format:

```
> command
  key output lines (indented)
  ISSUE: description of problem observed
  OK: brief positive note (if command worked well)
  CHECKLIST: M.N PASS|FAIL — brief note
```

Group entries by workflow sequence (Explore, Act, Control, Debug, Checklists). Include key output snippets — especially problematic ones — not full dumps.

### Step 5: Score All 6 Metrics

Score each metric 1-10 with brief justification:

| # | Metric | What to Score |
|---|--------|--------------|
| 1 | Decision Transparency | Can you see WHY agents chose their goals? Does tick/event output name the goal kind? |
| 2 | Action Lifecycle Clarity | Can you see WHAT actions are happening? Named at start, commit, in status? |
| 3 | Delta Semantics | Do deltas say WHAT changed specifically, not just which component? |
| 4 | Action Reliability | Did every listed action work? No crashes, no opaque payload errors? |
| 5 | Command Self-Documentation | Are commands and arguments self-explanatory from help/errors? |
| 6 | Causal Chain Readability | Can you trace consequences back to causes with readable output? |

**Hard gate**: A metric CANNOT score above 5 if its checklist (Step 3) has ANY failures. Apply this ceiling after initial scoring.

Compute deltas from the previous evaluation (if any).

### Step 6: Write Recommendations

For each issue found, classify as CRITICAL / HIGH / MEDIUM / LOW:

- **CRITICAL**: Blocks basic usage (crashes, data loss, completely unusable output)
- **HIGH**: Major information gap (can't see what agents decided, deltas are opaque, actions crash)
- **MEDIUM**: Moderate friction (some deltas informative but others not, partial lifecycle visibility)
- **LOW**: Minor polish (cosmetic, nice-to-have improvements)

Check prior evaluations for recurring issues:
- If an issue appeared before, note "Recurring: N consecutive evaluations"
- Issues persisting 3+ evaluations should be considered for escalation based on impact

### Step 7: Detect Stagnation and Regression

**Stagnation**: Same issue is top recommendation for 3+ consecutive evaluations AND average score hasn't improved by 0.5+ points. If detected, note it and suggest shifting to the `cli-improvement:implement` skill.

**Regression**: Any metric drops by 2+ points = major regression. Drops by 1 = minor regression. Flag both.

**Oscillation**: If the Score Trend shows alternating +/- deltas for 4+ evaluations, note the pattern and recommend more cautious implementation.

### Step 8: Write Evaluation

If `reports/cli-evaluation.md` does not exist, create it with this rubric header:

```markdown
# CLI Readability & Usability Evaluation (v2)

Evaluation report for the Worldwake CLI app (`crates/worldwake-cli/`).

Each evaluation is produced by the `cli-improvement:evaluate` skill, which
interactively uses the CLI against `scenarios/cli-evaluation.ron`, runs
mandatory checklists, and scores 6 metrics.

## Metrics

| # | Metric | What It Measures |
|---|--------|-----------------|
| 1 | Decision Transparency | Can you see WHY agents chose goals? |
| 2 | Action Lifecycle Clarity | Can you see WHAT actions happen at each stage? |
| 3 | Delta Semantics | Do deltas say WHAT changed, not just WHICH component? |
| 4 | Action Reliability | Do listed actions actually work? |
| 5 | Command Self-Documentation | Are commands self-explanatory? |
| 6 | Causal Chain Readability | Can you trace consequences to causes? |

## Scoring Guide

- **1-3**: Unusable — no information, crashes, incomprehensible
- **4-5**: Poor — partial information, some crashes or opaque output
- **6-7**: Adequate — most information present but gaps remain
- **8-9**: Good — full lifecycle visible, actionable deltas, reliable commands
- **10**: Excellent — a developer unfamiliar with the project understands everything

## Checklist Gate

A metric cannot score above 5 if its mandatory checklist has any failures.

## Graduation

All 6 checklists fully pass, average score >= 8.0, and no CRITICAL or HIGH
recommendations remaining.
```

Append the complete evaluation using this template:

```markdown
---

## EVALUATION #N — YYYY-MM-DD

### Session Notes
<Brief description of what was explored and discovered>

### Checklist Results

| Checklist | Result | Notes |
|-----------|--------|-------|
| 1. Decision Transparency | PASS/FAIL (N/M) | <brief> |
| 2. Action Lifecycle Clarity | PASS/FAIL (N/M) | <brief> |
| 3. Delta Semantics | PASS/FAIL (N/M) | <brief> |
| 4. Action Reliability | PASS/FAIL (N/M) | <brief> |
| 5. Command Self-Documentation | PASS/FAIL (N/M) | <brief> |
| 6. Causal Chain Readability | PASS/FAIL (N/M) | <brief> |

### Per-Command Analysis
<Key observations organized by workflow>

### Resolved Since Previous
<Issues from prior eval that are now fixed, or "First evaluation.">

### Scores

| # | Metric | Score | Delta | Gate | Justification |
|---|--------|-------|-------|------|---------------|
| 1 | Decision Transparency | N | +/-N | PASS/FAIL | <brief> |
| 2 | Action Lifecycle Clarity | N | +/-N | PASS/FAIL | <brief> |
| 3 | Delta Semantics | N | +/-N | PASS/FAIL | <brief> |
| 4 | Action Reliability | N | +/-N | PASS/FAIL | <brief> |
| 5 | Command Self-Documentation | N | +/-N | PASS/FAIL | <brief> |
| 6 | Causal Chain Readability | N | +/-N | PASS/FAIL | <brief> |
| | **Average** | **N.N** | **+/-N.N** | | |

### Score Trend (if 3+ evaluations)
| Eval | Avg | Delta |
|------|-----|-------|
| ... | ... | ... |

### Prioritized Recommendations
1. **[SEVERITY]** <recommendation>
2. ...
```

### Step 9: Graduation Check

If average score >= 8.0 AND all 6 checklists fully pass AND no CRITICAL or HIGH recommendations remain, note graduation:

> The CLI has graduated to acceptable quality. Further evaluations are optional — invoke only after significant CLI changes or new simulation features.

### Step 10: Report Archival

If the report exceeds ~500 lines or ~10 evaluations, archive older evaluations:

1. Keep the rubric header + last 5 evaluations in the active file
2. Move older evaluations verbatim to `reports/cli-evaluation-archive.md`
3. Do not condense or summarize archived evaluations

## Guardrails

- **Authentic interaction**: Use the CLI naturally. Don't just run commands mechanically — react to output, explore interesting things, test edge cases.
- **All commands exercised**: Every command must be tried at least once across the 4 workflows. Don't skip commands even if they seem fine.
- **Checklists are mandatory**: Do not skip any checklist item. Every item must be explicitly evaluated and recorded.
- **Hard gate enforcement**: Never score a metric above 5 if its checklist has failures. This is non-negotiable.
- **Honest scoring**: Score what you observed, not what you know the code does. A developer unfamiliar with the project is the reference user.
- **No implementation**: This skill only evaluates. Do not fix issues — that's the implement skill's job.
- **No scenario changes**: Do not modify `scenarios/cli-evaluation.ron` — that's the scenario skill's job.
