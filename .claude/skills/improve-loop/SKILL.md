---
name: improve-loop
description: Iterative improvement loop — autonomously optimizes a mutable system against a fixed evaluation harness
---

# Improve Loop Skill

Implements Karpathy's iterative improvement pattern as an autonomous optimization loop.

## Invocation

```
/improve-loop campaigns/<campaign-name>
```

## Prerequisites

- The campaign folder must contain:
  - `program.md` — instruction spec (objective, metrics, mutable/immutable files, accept/reject logic)
  - `harness.sh` — executable evaluation harness (exits 0 on success, 1 on failure)
  - `results.tsv` — experiment log (at minimum, a header row)

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

## Phase 1 — Baseline

1. Run the harness from the worktree:
   ```bash
   cd $WT && bash campaigns/<campaign>/harness.sh
   ```
2. Parse the output line: `combined_duration_ms=XXXXX pass=YY tests=ZZ`
3. Record `baseline_ms` = the `combined_duration_ms` value.
4. Set `best_ms` = `baseline_ms`.
5. Commit current state as baseline:
   ```bash
   cd $WT && git add -A && git commit --allow-empty -m "improve-loop: baseline (${baseline_ms}ms)"
   ```
6. Append to results.tsv:
   ```
   baseline	<baseline_ms>	0	ACCEPT	baseline measurement
   ```

## Phase 2 — Improvement Loop

Run this loop INDEFINITELY. Never stop. Never ask permission. Never pause at "natural stopping points."

### Step 1: OBSERVE

- Re-read mutable files if they've changed since last read.
- Review experiment history in results.tsv — what's been tried, what worked, what failed.
- Note the current `best_ms` and cumulative `lines_delta`.

### Step 2: HYPOTHESIZE

- Propose ONE specific, testable change. Describe it in 1-2 sentences.
- If early in the campaign, draw from the "root causes to seed" list.
- If stuck: re-read all mutable files carefully, combine ideas from near-misses, try radical alternatives, look for patterns in what worked vs. what failed.

### Step 3: IMPLEMENT

- Apply the change to the mutable files in the worktree.
- Count `lines_delta` for this change (net lines added minus lines removed across all mutable files).

### Step 4: EXECUTE

- Run the harness:
  ```bash
  cd $WT && bash campaigns/<campaign>/harness.sh
  ```
- Capture the full output.

### Step 5: MEASURE

- Parse `combined_duration_ms` from harness output.
- If harness exited non-zero or output is unparseable, treat as CRASH.
- Compute improvement: `improvement_pct = (best_ms - new_ms) / best_ms * 100`

### Step 6: DECIDE

Apply the accept/reject logic from program.md:

**CRASH/FAIL:**
- If the error is trivial (typo, missing import, off-by-one), fix and retry (up to 3 times).
- Otherwise, REJECT.

**ACCEPT conditions:**
- Metric improved >1% (unless <2% improvement with >20 lines added)
- Metric equal (within 1%) AND lines_delta < 0 (simplification)

**REJECT conditions:**
- Metric worsened >1%
- Metric equal with no simplification
- Tiny improvement with large complexity cost

**On ACCEPT:**
```bash
cd $WT && git add <changed-files> && git commit -m "improve-loop: <description> (<old_ms> -> <new_ms> ms)"
```
Update `best_ms = new_ms`.

**On REJECT:**
```bash
cd $WT && git checkout -- <changed-files>
```

### Step 7: LOG

Append a row to `$WT/campaigns/<campaign>/results.tsv`:
```
<experiment_id>	<combined_duration_ms>	<lines_delta>	<ACCEPT|REJECT|CRASH>	<description>
```

Use a sequential experiment ID: `exp-001`, `exp-002`, etc. (continue from where results.tsv left off).

### Step 8: REPEAT

Go back to Step 1. Do NOT stop.

## Git Operations Summary

| Event | Action |
|-------|--------|
| ACCEPT | `git add <files>` + `git commit -m "improve-loop: ..."` |
| REJECT | `git checkout -- <files>` |
| CRASH (trivial) | Fix, retry (up to 3x) |
| CRASH (fundamental) | `git checkout -- <files>`, log, continue |

`results.tsv` and `run.log` are untracked (gitignored) — they persist across accepts and rejects but are not committed.

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
