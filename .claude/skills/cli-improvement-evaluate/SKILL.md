---
name: cli-improvement-evaluate
description: "Interactively use the CLI, score 6 metrics, append evaluation to reports/cli-evaluation.md. Invoke after CLI changes to measure improvement."
user-invocable: true
---

# CLI Evaluation

Interactively use the Worldwake CLI like a real user, exercising all commands, then score the experience against 6 metrics and append a structured evaluation to the report.

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
3. Verify the scenario loads: `cargo run -p worldwake-cli -- scenarios/cli-evaluation.ron --exec quit 2>&1`. If it fails with a parse error (missing field, type mismatch), this is a schema-drift bug, not a scenario design issue. Fix the minimal field addition to match the current struct definition, note the fix in the evaluation, and proceed. This is distinct from the "no scenario changes" guardrail, which prohibits redesigning what the scenario exercises. If the failure is a logic error (not schema drift), stop and report.

### Step 2: Interactive CLI Session

Use the CLI's `--exec` + `--state` mode to run one command per Bash call, reading output between commands and deciding the next command adaptively. This is genuine interactive exploration — you can react to what you see.

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

**Adaptive exploration**: Between commands, read the output and decide what to explore next. If `look` reveals an interesting entity, `inspect` it. If `actions` shows something suspicious, try `do`-ing it. If `tick` produces many events, `event <id>` the most interesting one. This is the key advantage of `--exec` mode — you can react.

If a command exits with a non-zero code in `--exec` mode, test the same command in interactive REPL mode to determine whether the issue is CLI-wide or `--exec`-specific. Score based on the CLI behavior, not `--exec` artifacts.

During the session, take notes on:
- Output that uses `{:?}` debug format or raw internal identifiers
- Actions listed as available that fail when selected
- Output that is hard to parse or understand
- Missing context in event/trace output
- Confusing command flow or implicit rules
- Error messages that don't help you recover
- Anything surprising, confusing, or delightful

**Clean up** session files when done: `rm /tmp/cli-eval-session.bin /tmp/cli-eval-save.bin`

### Step 3: Read Previous Evaluation and Save Transcript

After the CLI session, read `reports/cli-evaluation.md` to determine the evaluation number and review previous scores:
- Read the first ~40 lines for the rubric and scoring guide
- Count total lines. If >200 lines, read from `offset = totalLines - 200` to get the last 2-3 evaluations
- To build the Score Trend table, grep for `\*\*Average\*\*` in the report file to get all historical averages

Then save the session transcript to `reports/cli-evaluation-transcripts/eval-N.txt`. The transcript may be written after scoring (Step 4) so that ISSUE/OK annotations can be informed by comparison with prior evaluations. Use this format:

```
> command
  key output lines (indented)
  ISSUE: description of problem observed
  OK: brief positive note (if command worked well)
```

Group entries by workflow sequence (Explore, Act, Control, Debug). Include key output snippets — especially problematic ones — not full dumps.

### Step 4: Score All 6 Metrics

Score each metric 1-10 with brief justification:

| # | Metric | What to Score |
|---|--------|--------------|
| 1 | Output Clarity | Human-readable? No `{:?}`, no raw IDs, no jargon? |
| 2 | Action Reliability | Did listed actions work? Any missing-profile errors? |
| 3 | State Legibility | Could you quickly understand world state? Scannable? |
| 4 | Causal Traceability | Could you trace why things happened? Clear event chains? |
| 5 | Session Flow | Did the command sequence feel natural? Timing clear? |
| 6 | Error Recovery | Did errors explain what went wrong? Suggest alternatives? |

Compute deltas from the previous evaluation (if any).

### Step 5: Write Recommendations

For each issue found, classify as CRITICAL / HIGH / MEDIUM / LOW:

- **CRITICAL**: Blocks basic usage (crashes, data loss, completely unusable output)
- **HIGH**: Major UX problem (actions that error, incomprehensible state display)
- **MEDIUM**: Moderate friction (debug format in some outputs, implicit rules)
- **LOW**: Minor polish (cosmetic, nice-to-have improvements)

Check prior evaluations for recurring issues:
- If an issue appeared before, note "Recurring: N consecutive evaluations"
- Issues persisting 3+ evaluations should be considered for escalation based on impact

### Step 6: Detect Stagnation and Regression

**Stagnation**: Same issue is top recommendation for 3+ consecutive evaluations AND average score hasn't improved by 0.5+ points. If detected, note it and suggest shifting to the `cli-improvement:implement` skill.

**Regression**: Any metric drops by 2+ points = major regression. Drops by 1 = minor regression. Flag both.

**Oscillation**: If the Score Trend shows alternating +/- deltas for 4+ evaluations, note the pattern and recommend more cautious implementation.

### Step 7: Append Evaluation

Append the complete evaluation to `reports/cli-evaluation.md` using the template from the rubric section. Include all sections:
- Session Notes
- Per-Command Analysis
- Resolved Since Previous
- Scores table with deltas
- Score Trend (if 5+ evaluations exist)
- Prioritized Recommendations

### Step 8: Graduation Check

If average score >= 8.0 AND no CRITICAL or HIGH recommendations remain, note graduation:

> The CLI has graduated to acceptable quality. Further evaluations are optional — invoke only after significant CLI changes or new simulation features.

### Step 9: Report Archival

If the report exceeds ~500 lines or ~10 evaluations, archive older evaluations:

1. Keep the rubric header + last 5 evaluations in the active file
2. Move older evaluations verbatim to `reports/cli-evaluation-archive.md`
3. Do not condense or summarize archived evaluations

## Guardrails

- **Authentic interaction**: Use the CLI naturally. Don't just run commands mechanically — react to output, explore interesting things, test edge cases.
- **All commands exercised**: Every command must be tried at least once across the 4 workflows. Don't skip commands even if they seem fine.
- **Honest scoring**: Score what you observed, not what you know the code does. A developer unfamiliar with the project is the reference user.
- **No implementation**: This skill only evaluates. Do not fix issues — that's the implement skill's job.
- **No scenario changes**: Do not modify `scenarios/cli-evaluation.ron` — that's the scenario skill's job.
