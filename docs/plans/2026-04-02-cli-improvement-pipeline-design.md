# CLI Improvement Pipeline — Design

## Problem

The Worldwake CLI app (`crates/worldwake-cli/`) is a 30-command REPL for manually exploring and experiencing the simulation. It currently suffers from:
- **Opaque output**: component data rendered via `{:?}` debug format, raw event deltas
- **Invalid action offerings**: agents are offered actions they can't execute due to missing profiles
- **Implicit rules**: must run `actions` before `do`, timing of action execution unclear
- **Low adoption**: the CLI is underused because the experience isn't clear enough to be valuable

The CLI needs to serve three purposes equally: (1) understand what's happening in the simulation, (2) test specific scenarios for debugging/validation, (3) experience the world as a participant.

## Solution

Three skills forming an evaluate-implement loop, inspired by the proven pipeline pattern in `reports/example-improvement-skill-pipelines.md`.

## Skill Architecture

```
.claude/skills/cli-improvement/
  evaluate/SKILL.md      — Interactively use CLI, score 6 metrics, append to report
  implement/SKILL.md     — Read latest eval, fix top issues, verify
  scenario/SKILL.md      — Maintain evaluation RON file as simulation evolves
```

### Invocation Flow

```
/cli-improvement:scenario   (when new sim features land)
    |
/cli-improvement:evaluate   (interactive CLI session, score output)
    |
/cli-improvement:implement  (fix top recommendations)
    |
/cli-improvement:evaluate   (verify improvements, detect regressions)
    |
... loop until graduation (8.0+ avg, no CRITICAL/HIGH) ...
```

### Key Files

- `reports/cli-evaluation.md` — growing evaluation report (rubric + evaluations)
- `scenarios/cli-evaluation.ron` — dedicated evaluation scenario
- `reports/cli-evaluation-transcripts/eval-N.txt` — captured CLI output per evaluation

## Evaluation Metrics

| # | Metric | What It Measures |
|---|--------|-----------------|
| 1 | Output Clarity | Human-readable output, no `{:?}` debug format, no raw IDs |
| 2 | Action Reliability | Listed actions work when selected, no missing-profile errors |
| 3 | State Legibility | World state is scannable and well-labeled from status/look/inspect/world |
| 4 | Causal Traceability | Can trace WHY something happened via events/trace with clear explanations |
| 5 | Session Flow | Command sequence feels natural, transitions smooth, timing clear |
| 6 | Error Recovery | Errors explain what went wrong and suggest what to do instead |

### Scoring Guide

- 1-3: Unusable — debug format, cryptic errors, incomprehensible
- 4-5: Poor — partially functional but confusing
- 6-7: Adequate — works but not intuitive
- 8-9: Good — clear, intuitive, well-organized
- 10: Excellent — a developer unfamiliar with the project could understand everything

## Evaluate Skill

Claude interactively uses the CLI like a real user (via Bash tool), exploring naturally and reacting to output. This is more authentic than a piped script.

### Checklist

1. Build the CLI: `cargo build -p worldwake-cli`
2. Launch the CLI with the evaluation scenario: `cargo run -p worldwake-cli -- scenarios/cli-evaluation.ron`
3. Interactively explore the CLI, exercising all 30 commands across 4 workflow sequences:
   - **Explore**: world, places, agents, goods, look, inspect, relations, inventory, needs
   - **Act**: actions, do, tick, status, cancel
   - **Control**: switch, observe
   - **Debug**: events, event, trace, save, load
4. Take notes on issues encountered — output clarity, action failures, confusing state, error messages
5. After quitting, save the session transcript to `reports/cli-evaluation-transcripts/eval-N.txt`
6. Read the previous evaluation from `reports/cli-evaluation.md`
7. Score all 6 metrics (1-10) with justification
8. Compute deltas, list resolved issues, write prioritized recommendations (CRITICAL/HIGH/MEDIUM/LOW)
9. Track recurring issues, detect stagnation/regression
10. Append complete evaluation to `reports/cli-evaluation.md`
11. Graduation check: avg >= 8.0 and no CRITICAL/HIGH

### Stagnation Detection

Same issue is top recommendation for 3+ consecutive evaluations AND avg hasn't improved by 0.5+ points.

### Report Archival

When report exceeds ~500 lines or ~10 evaluations, archive older evaluations to `reports/cli-evaluation-archive.md`.

## Implement Skill

### Checklist

1. Read latest evaluation from `reports/cli-evaluation.md`
2. Identify CRITICAL and HIGH recommendations (if none, top 2-3 MEDIUM)
3. Note lowest-scoring metrics as priority targets
4. Read relevant CLI source files
5. For top 2-3 recommendations, identify specific file and function before coding
6. If ambiguous, apply 1-3-1 rule
7. Implement changes, highest-impact first
8. Verify: `cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings`
9. Do NOT update the evaluation report

### Scope Constraints

- Only modify `crates/worldwake-cli/`
- If a fix requires core/sim/systems/ai changes, flag as separate spec/ticket
- No backward-compatibility wrappers — replace, don't wrap

## Scenario Skill

### Checklist

1. Read `scenarios/cli-evaluation.ron` (current evaluation scenario)
2. Read latest evaluation to understand current coverage
3. Check recent commits/active specs for new simulation features
4. If new features aren't exercised, update the RON file:
   - Add agents with relevant profiles
   - Add items, facilities, resource sources
   - Add places/edges if needed
5. Validate: launch CLI with updated scenario, immediately quit
6. Document changes in a comment at top of RON file

### When to Invoke

After implementing a new spec that adds simulation capabilities. Not part of the evaluate-implement loop.

## Graduation Criteria

- Average score >= 8.0 across all 6 metrics
- No CRITICAL or HIGH recommendations remaining
- Re-enter the loop when new simulation features add CLI surface area

## Implementation Order

1. Create `scenarios/cli-evaluation.ron` — dedicated evaluation scenario with diverse agents, places, items, recipes
2. Create `reports/cli-evaluation.md` — initial rubric and scoring guide
3. Write `cli-improvement/evaluate/SKILL.md`
4. Write `cli-improvement/implement/SKILL.md`
5. Write `cli-improvement/scenario/SKILL.md`
6. Run first evaluation cycle
