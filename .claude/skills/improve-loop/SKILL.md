---
name: improve-loop
description: Iterative improvement loop — autonomously optimizes a mutable system against a fixed evaluation harness
---

# Improve Loop Skill

Implements Karpathy's iterative improvement pattern as an autonomous optimization loop, enhanced with early abort, near-miss tracking, experiment categorization, plateau detection, structured reflection, multi-run averaging, and human steering.

## Invocation

```
/improve-loop campaigns/<campaign-name>
```

## Prerequisites

- The campaign folder must contain:
  - `program.md` — instruction spec (objective, metrics, mutable/immutable files, accept/reject logic, experiment categories, thresholds)
  - `harness.sh` — executable evaluation harness (exits 0 on success, 1 on failure)
  - `results.tsv` — experiment log (at minimum, a header row; schema below)
- Optional campaign files:
  - `musings.md` — structured reflection log (created automatically if missing)
  - `next-idea.md` — human-provided hypothesis override (consumed and renamed after use)

### results.tsv Schema

```
experiment_id	combined_duration_ms	lines_delta	category	status	description
```

Status values: `ACCEPT`, `REJECT`, `NEAR_MISS`, `EARLY_ABORT`, `CRASH`

**Backward compatibility:** If resuming an old campaign whose results.tsv lacks the `category` column, treat all existing rows as having category `other` and continue with the new schema for new rows.

## Worktree Requirement (NON-NEGOTIABLE)

The improvement loop commits and rolls back frequently. It MUST run inside a dedicated git worktree to protect the main working tree.

1. Check if already in a worktree: `git rev-parse --show-toplevel`
2. If not in a worktree, create one:
   ```bash
   git worktree add .claude/worktrees/improve-<campaign> -b improve/<campaign> main
   ```
3. ALL subsequent file operations use the worktree root as the base path.

Set `WT` = the worktree root path. Every file path in every tool call below is prefixed with `$WT/`.

## Phase 0 — Setup

1. Read `$WT/campaigns/<campaign>/program.md` completely.
2. Verify `$WT/campaigns/<campaign>/harness.sh` exists and is executable.
3. Read `$WT/campaigns/<campaign>/results.tsv` — if it has data rows beyond the header, resume from the last accepted state (the current HEAD of the worktree branch IS the last accepted state).
4. Identify the **mutable files** from program.md. Read each one into context.
5. Identify the **root causes to seed** from program.md as the initial hypothesis queue.
6. Read configuration from program.md:
   - `ABORT_THRESHOLD` (default: 0.05)
   - `PLATEAU_THRESHOLD` (default: 5)
   - `HARNESS_RUNS` (default: 1)
   - Experiment categories list
7. Ensure `$WT/campaigns/<campaign>/musings.md` exists (create with `# Musings` header if missing).
8. Initialize strategy state: `strategy = "normal"`, `consecutive_rejects = 0`.

## Phase 1 — Baseline

1. Run the harness from the worktree. If `HARNESS_RUNS > 1`, run it that many times and take the median:
   ```bash
   cd $WT && bash campaigns/<campaign>/harness.sh
   ```
2. Parse the output line: `combined_duration_ms=XXXXX pass=YY tests=ZZ`
3. If multi-run: collect all `combined_duration_ms` values, record `baseline_ms` = median, note spread.
4. If single-run: record `baseline_ms` = the `combined_duration_ms` value.
5. Set `best_ms` = `baseline_ms`.
6. Commit current state as baseline:
   ```bash
   cd $WT && git add -A && git commit --allow-empty -m "improve-loop: baseline (${baseline_ms}ms)"
   ```
7. Append to results.tsv:
   ```
   baseline	<baseline_ms>	0	baseline	ACCEPT	baseline measurement
   ```

## Phase 2 — Improvement Loop

Run this loop INDEFINITELY. Never stop. Never ask permission. Never pause at "natural stopping points."

### Step 1: OBSERVE

- Re-read mutable files if they've changed since last read.
- Review experiment history in results.tsv — what's been tried, what worked, what failed.
- Note the current `best_ms` and cumulative `lines_delta`.

### Step 1b: CHECK STRATEGY (Plateau Detection)

- Count consecutive rejects (including NEAR_MISS and EARLY_ABORT) from the tail of results.tsv.
- If count >= `PLATEAU_THRESHOLD`:
  - Check for near-miss stashes (`git stash list | grep near-miss`)
  - If near-misses exist and strategy is `normal` → switch to `combine`
  - If no near-misses or already tried combine → switch to `ablation` (review recent accepts, try removing complexity)
  - If already tried ablation → switch to `radical` (large structural changes, rethink approach)
- After any ACCEPT, reset `strategy = "normal"` and `consecutive_rejects = 0`.

### Step 1c: COMPUTE CATEGORY SUCCESS RATES

- Group results.tsv rows by `category` column.
- Compute `success_rate = accepts / attempts` per category.
- Use these rates to inform hypothesis generation (prefer high-yield categories, but don't completely ignore low-rate ones — exploration matters).

### Step 2: HYPOTHESIZE

- **Check for human override first:** Does `$WT/campaigns/<campaign>/next-idea.md` exist?
  - If yes: read its contents as the hypothesis. Rename to `next-idea.used-exp-NNN.md`. Skip normal generation.
  - If no: proceed with normal hypothesis generation below.

- **Strategy-specific generation:**
  - `normal`: Propose ONE specific, testable change. Prefer categories with higher success rates. If early in the campaign, draw from the "root causes to seed" list.
  - `combine`: Select 2-3 near-miss stashes (`git stash apply stash@{N}`), apply them together, test as one experiment.
  - `ablation`: Review recent accepted commits, propose removing complexity from one of them.
  - `radical`: Propose a fundamentally different approach — different algorithm, restructured data flow, etc.

- If stuck in `normal` mode: re-read all mutable files carefully, combine ideas from near-misses, try radical alternatives, look for patterns in what worked vs. what failed.

### Step 2.5: RECORD HYPOTHESIS (Structured Reflection)

Append to `$WT/campaigns/<campaign>/musings.md`:
```markdown
## exp-NNN: <description>
**Hypothesis**: <1-2 sentences on why this should improve the metric>
```

### Step 3: IMPLEMENT

- Apply the change to the mutable files in the worktree.
- Count `lines_delta` for this change (net lines added minus lines removed across all mutable files).
- Tag the change with a `category` from program.md's experiment categories list.

### Step 4: EXECUTE

- Read `HARNESS_RUNS` from program.md (default: 1).
- Run the harness:
  ```bash
  cd $WT && bash campaigns/<campaign>/harness.sh
  ```

**Early abort (per-run):** If the harness supports intermediate output (one line per target file), parse after each line. If the running `combined_duration_ms` already exceeds `best_ms * (1 + ABORT_THRESHOLD)`, kill the harness process. Log status as `EARLY_ABORT` and REJECT immediately (skip to Step 7).

**Multi-run averaging:** If `HARNESS_RUNS > 1`:
  - Run the harness N times, collecting all `combined_duration_ms` values.
  - Early abort still applies per-run (abort any single run that's clearly losing).
  - Take the **median** as the metric for the accept/reject decision.
  - Compute `spread = max - min`. If spread > 2% of median, consider adding one more run for confidence.
  - Report spread in the log description.

### Step 5: MEASURE

- Parse `combined_duration_ms` from harness output (or median if multi-run).
- If harness exited non-zero or output is unparseable, treat as CRASH.
- Compute improvement: `improvement_pct = (best_ms - new_ms) / best_ms * 100`

### Step 6: DECIDE

Apply the accept/reject logic from program.md:

**CRASH/FAIL:**
- If the error is trivial (typo, missing import, off-by-one), fix and retry (up to 3 times).
- Otherwise, REJECT.

**EARLY_ABORT:**
- Already handled in Step 4. Log and continue.

**ACCEPT conditions:**
- Metric improved >1% (unless <2% improvement with >20 lines added)
- Metric equal (within 1%) AND lines_delta < 0 (simplification)

**NEAR_MISS conditions:**
- Metric within 1% of best AND lines_delta >= 0 (not a simplification)
- On NEAR_MISS: create a named stash before rolling back:
  ```bash
  cd $WT && git stash push -m "near-miss-exp-NNN: <description>"
  ```

**REJECT conditions:**
- Metric worsened >1%
- Tiny improvement with large complexity cost

**On ACCEPT:**
```bash
cd $WT && git add <changed-files> && git commit -m "improve-loop: <description> (<old_ms> -> <new_ms> ms)"
```
Update `best_ms = new_ms`. Reset `strategy = "normal"`, `consecutive_rejects = 0`.

**On NEAR_MISS:**
```bash
cd $WT && git stash push -m "near-miss-exp-NNN: <description>"
```

**On REJECT / EARLY_ABORT:**
```bash
cd $WT && git checkout -- <changed-files>
```

### Step 7: LOG

Append a row to `$WT/campaigns/<campaign>/results.tsv`:
```
<experiment_id>	<combined_duration_ms>	<lines_delta>	<category>	<ACCEPT|REJECT|NEAR_MISS|EARLY_ABORT|CRASH>	<description>
```

Use a sequential experiment ID: `exp-001`, `exp-002`, etc. (continue from where results.tsv left off).

### Step 7.5: RECORD LEARNING (Structured Reflection)

Append to `$WT/campaigns/<campaign>/musings.md`:
```markdown
**Result**: <ACCEPT|REJECT|NEAR_MISS|EARLY_ABORT|CRASH> (<old_ms> -> <new_ms> ms)
**Learning**: <what was learned — confirmed/refuted hypothesis, surprising observations, what to try differently>
```

### Step 8: REPEAT

Go back to Step 1. Do NOT stop.

## Git Operations Summary

| Event | Action |
|-------|--------|
| ACCEPT | `git add <files>` + `git commit -m "improve-loop: ..."` |
| REJECT | `git checkout -- <files>` |
| NEAR_MISS | `git stash push -m "near-miss-exp-NNN: ..."` |
| EARLY_ABORT | `git checkout -- <files>` (or kill harness + checkout) |
| CRASH (trivial) | Fix, retry (up to 3x) |
| CRASH (fundamental) | `git checkout -- <files>`, log, continue |
| Combine strategy | `git stash apply stash@{N}` for 2-3 near-miss stashes |

`results.tsv`, `musings.md`, and `run.log` are untracked (gitignored) — they persist across accepts and rejects but are not committed.

## After Campaign Completes

When the human decides to stop the loop:
1. Review the worktree branch: `git log --oneline` shows all accepted improvements.
2. Squash-merge into main: `git merge --squash improve/<campaign>`
3. Remove the worktree: `git worktree remove .claude/worktrees/improve-<campaign>`

## Important Rules

- **Never modify immutable files** (harness.sh, engine source, game data, other tests).
- **Never weaken assertions** — the tests must remain equally rigorous.
- **Never add dependencies** — optimize with what's available.
- **Never stop the loop** — run until externally interrupted.
- **Always use worktree paths** — never operate on the main working tree.
- **Always tag experiments with a category** from program.md's taxonomy.
- **Always record hypothesis and learning** in musings.md.
