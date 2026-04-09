# CLI Improvement Pipeline Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Create 3 skills (evaluate, implement, scenario) forming an improvement loop for the Worldwake CLI app, plus a dedicated evaluation scenario and initial report file.

**Architecture:** Three SKILL.md files in `.claude/skills/cli-improvement/`, a RON scenario at `scenarios/cli-evaluation.ron`, and a report template at `reports/cli-evaluation.md`. The evaluate skill has Claude interactively use the CLI and score 6 metrics. The implement skill fixes top issues. The scenario skill maintains the evaluation scenario as the simulation evolves.

**Tech Stack:** Claude Code skills (SKILL.md YAML frontmatter), RON scenario files, Rust CLI (`worldwake-cli`), Bash for verification.

---

### Task 1: Create the dedicated evaluation scenario

**Files:**
- Create: `scenarios/cli-evaluation.ron`

**Step 1: Write the evaluation scenario**

Create a RON scenario that exercises the widest range of CLI features. Must include:
- 4+ places with different tags, connected by edges (some bidirectional, some not)
- 4+ agents: 1 human-controlled, 1 merchant with trade/merchandise profiles, 1 forager, 1 AI agent with combat profile — agents at different locations
- Items distributed across places and agents (coins, food, trade goods, weapons)
- At least 1 workstation facility
- At least 1 resource source
- Homeostatic needs on agents (hunger, thirst at various levels) so actions are available
- A trade disposition profile on the merchant
- A utility profile on at least one agent

Use the existing `scenarios/default.ron` as the structural template. The evaluation scenario should be richer — more places, more agents, more items — to exercise travel, trade, combat affordances, needs, and production.

**Step 2: Validate the scenario loads**

Run: `cargo run -p worldwake-cli -- scenarios/cli-evaluation.ron <<< "quit"`
Expected: CLI launches, prints prompt, exits cleanly with no errors.

**Step 3: Commit**

```bash
git add scenarios/cli-evaluation.ron
git commit -m "Add dedicated CLI evaluation scenario for improvement pipeline"
```

---

### Task 2: Create the initial evaluation report with rubric

**Files:**
- Create: `reports/cli-evaluation.md`
- Create: `reports/cli-evaluation-transcripts/` (empty directory with .gitkeep)

**Step 1: Write the report rubric**

Create `reports/cli-evaluation.md` with the rubric header, 6 metric definitions, scoring guide, and evaluation template — everything before the first evaluation. Follow the structure from `reports/example-improvement-skill-pipelines.md` (the `train-operation-ui-evaluate` skill's template) adapted for CLI metrics:

```markdown
# CLI Readability & Usability Evaluation

Evaluation report for the Worldwake CLI app (`crates/worldwake-cli/`).

Each evaluation is produced by the `cli-improvement:evaluate` skill, which
interactively uses the CLI against `scenarios/cli-evaluation.ron` and scores
6 metrics. Evaluations are appended below the rubric.

## Metrics

| # | Metric | What It Measures |
|---|--------|-----------------|
| 1 | Output Clarity | Human-readable output, no {:?} debug format, no raw IDs |
| 2 | Action Reliability | Listed actions work when selected, no missing-profile errors |
| 3 | State Legibility | World state is scannable and well-labeled |
| 4 | Causal Traceability | Can trace WHY something happened with clear explanations |
| 5 | Session Flow | Command sequence feels natural, transitions smooth |
| 6 | Error Recovery | Errors explain what went wrong and suggest alternatives |

## Scoring Guide

- 1-3: Unusable — debug format, cryptic errors, incomprehensible
- 4-5: Poor — partially functional but confusing
- 6-7: Adequate — works but not intuitive
- 8-9: Good — clear, intuitive, well-organized
- 10: Excellent — developer unfamiliar with project could understand everything

## Graduation

Average score >= 8.0 and no CRITICAL or HIGH recommendations.
Re-enter the loop when new simulation features add CLI surface area.

## What to Look For

- Raw internal identifiers or debug format ({:?}) exposed to the user
- Actions listed as available that error when selected
- Component output that is machine-readable but not human-friendly
- Event deltas printed in raw format without context
- Implicit rules that the user must memorize (e.g., run `actions` before `do`)
- Missing error context or recovery suggestions
- Unclear timing of action execution vs. enqueuing
- Stale affordances after ticking
- Regressions from previous evaluations
```

**Step 2: Create transcript directory**

```bash
mkdir -p reports/cli-evaluation-transcripts
touch reports/cli-evaluation-transcripts/.gitkeep
```

**Step 3: Commit**

```bash
git add reports/cli-evaluation.md reports/cli-evaluation-transcripts/.gitkeep
git commit -m "Add CLI evaluation report rubric and transcript directory"
```

---

### Task 3: Write the evaluate skill

**Files:**
- Create: `.claude/skills/cli-improvement/evaluate/SKILL.md`

**Step 1: Write the skill file**

Create the SKILL.md with YAML frontmatter and the full checklist. The skill must:

- Frontmatter: `name: evaluate`, `description: "Interactively use the CLI, score 6 metrics, append evaluation to reports/cli-evaluation.md. Invoke after CLI changes to measure improvement."`, `user-invocable: true`
- Build the CLI binary first
- Launch the CLI interactively via Bash with the evaluation scenario
- Exercise all 30 commands across 4 workflow sequences (Explore, Act, Control, Debug)
- After the interactive session, save the session transcript
- Read the previous evaluation from the report
- Score all 6 metrics with justification and deltas
- List resolved issues, write prioritized recommendations (CRITICAL/HIGH/MEDIUM/LOW)
- Track recurring issues across evaluations
- Detect stagnation (same top issue for 3+ evals AND avg hasn't improved by 0.5+)
- Detect regressions (score drops)
- Include score trend table if 5+ evaluations exist
- Append the complete evaluation to the report
- Graduation check
- Report archival when >500 lines or >10 evaluations

Use the `train-operation-ui-evaluate` skill from the example report as the structural template — adapt the screenshot-reading steps to interactive CLI usage, and replace the 6 UI metrics with the 6 CLI metrics.

**Step 2: Verify skill appears in skill list**

Run: check that the skill file exists at the expected path and has valid YAML frontmatter.

**Step 3: Commit**

```bash
git add .claude/skills/cli-improvement/evaluate/SKILL.md
git commit -m "Add CLI evaluate skill — interactive CLI scoring with 6 metrics"
```

---

### Task 4: Write the implement skill

**Files:**
- Create: `.claude/skills/cli-improvement/implement/SKILL.md`

**Step 1: Write the skill file**

Create the SKILL.md with YAML frontmatter and the full checklist. The skill must:

- Frontmatter: `name: implement`, `description: "Read latest CLI evaluation, implement top recommendations within crates/worldwake-cli/. Invoke after evaluate to fix highest-priority issues."`, `user-invocable: true`
- Read latest evaluation from `reports/cli-evaluation.md`
- Identify CRITICAL and HIGH recommendations (if none, top 2-3 MEDIUM)
- Note lowest-scoring metrics as priority targets
- Read relevant CLI source files (key files reference: `handlers/actions.rs`, `handlers/inspect.rs`, `handlers/tick.rs`, `handlers/events.rs`, `handlers/control.rs`, `handlers/world_overview.rs`, `display.rs`, `repl.rs`, `commands.rs`)
- For top 2-3 recommendations, identify specific file and function before coding
- If ambiguous, apply 1-3-1 rule
- Implement changes, highest-impact first
- Scope constraint: only modify `crates/worldwake-cli/`. If core/sim/systems/ai changes are needed, flag as separate spec/ticket
- Verify: `cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings`
- Do NOT update the evaluation report

Use the `train-operation-ui-implement` skill from the example report as the structural template — adapt key files, data flow, scope constraints, and verification commands for the Rust CLI crate.

**Step 2: Commit**

```bash
git add .claude/skills/cli-improvement/implement/SKILL.md
git commit -m "Add CLI implement skill — fix top evaluation recommendations"
```

---

### Task 5: Write the scenario skill

**Files:**
- Create: `.claude/skills/cli-improvement/scenario/SKILL.md`

**Step 1: Write the skill file**

Create the SKILL.md with YAML frontmatter and the full checklist. The skill must:

- Frontmatter: `name: scenario`, `description: "Update the CLI evaluation scenario (scenarios/cli-evaluation.ron) when new simulation features land. Invoke after implementing specs that add new action types, systems, or components."`, `user-invocable: true`
- Read `scenarios/cli-evaluation.ron` (current evaluation scenario)
- Read latest evaluation from `reports/cli-evaluation.md` to understand current coverage
- Check recent commits or active specs for new simulation features
- If new features aren't exercised, update the RON file:
  - Add agents with relevant profiles
  - Add items, facilities, resource sources
  - Add places/edges if needed
- Validate: launch CLI with updated scenario, immediately quit
- Document changes in a comment at top of RON file
- Note: this is NOT part of the evaluate-implement loop. Invoke after implementing a new spec.

**Step 2: Commit**

```bash
git add .claude/skills/cli-improvement/scenario/SKILL.md
git commit -m "Add CLI scenario skill — maintain evaluation RON file"
```

---

### Task 6: Run first evaluation cycle

**Files:**
- Modify: `reports/cli-evaluation.md` (append first evaluation)
- Create: `reports/cli-evaluation-transcripts/eval-1.txt`

**Step 1: Invoke the evaluate skill**

Run: `/cli-improvement:evaluate`

This will interactively use the CLI, score the 6 metrics, and append EVALUATION #1 to the report. This establishes the baseline scores.

**Step 2: Review the evaluation**

Read the appended evaluation. Verify:
- All 6 metrics scored
- Recommendations are prioritized
- The evaluation captures the known pain points (opaque output, invalid actions)

**Step 3: Commit**

```bash
git add reports/cli-evaluation.md reports/cli-evaluation-transcripts/eval-1.txt
git commit -m "Baseline CLI evaluation #1"
```
