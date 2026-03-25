---
name: improve-loop
description: Iterative improvement loop — autonomously optimizes a mutable system against a fixed evaluation harness
---

# Improve Loop Skill

Implements Karpathy's iterative improvement pattern as an autonomous optimization loop, enhanced with UCB1 category selection, MAD confidence scoring, Goodhart's Law defenses, lightweight backtracking, intermediate metrics, cross-run lesson store, self-improving research strategy, near-miss tracking, plateau detection, structured reflection, multi-run averaging, human steering, condition drift anchoring, correctness guards, scope enforcement, lesson curation gating, and structured lesson categories.

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
  - `checks.sh` — correctness guard (tests, types, lint); blocks metric-improving changes that break correctness

### results.tsv Schema

```
experiment_id	combined_duration_ms	lines_delta	category	status	description
```

Status values: `ACCEPT`, `REJECT`, `NEAR_MISS`, `EARLY_ABORT`, `CRASH`, `SUSPICIOUS_ACCEPT`, `BACKTRACK`

**Backward compatibility:** If resuming an old campaign whose results.tsv lacks the `category` column, treat all existing rows as having category `other` and continue with the new schema for new rows.

### Runtime Files (created automatically, untracked by git)

| File | Purpose |
|------|---------|
| `checkpoints.jsonl` | Named restore points for backtracking |
| `lessons.jsonl` | Per-campaign extracted lessons |
| `intermediates.jsonl` | Per-experiment intermediate metric breakdowns |
| `program.md.backup` | Meta-loop rollback snapshot |

### Persistent Cross-Campaign Files (MUST be committed)

| File | Purpose |
|------|---------|
| `campaigns/lessons-global.jsonl` | Cross-campaign promoted lessons — persists across campaigns and worktrees |

**IMPORTANT**: `campaigns/lessons-global.jsonl` is NOT a per-campaign runtime file.
It accumulates lessons across ALL campaigns and MUST be committed to the repo
(not gitignored) so it survives worktree removal and squash-merges. The "After
Campaign Completes" step explicitly commits this file.

### Configuration Keys (read from program.md, all have defaults)

| Key | Default | Purpose |
|-----|---------|---------|
| `ABORT_THRESHOLD` | 0.05 | Early abort if metric exceeds best by this fraction |
| `PLATEAU_THRESHOLD` | 5 | Consecutive rejects before strategy shift |
| `HARNESS_RUNS` | 1 | Number of harness runs per experiment (median taken) |
| `UCB_EXPLORATION_C` | 1.414 | UCB1 exploration constant (higher = more exploration) |
| `MIN_CONFIDENCE_RUNS` | `HARNESS_RUNS * 2` | Extra runs required when improvement is within noise floor |
| `HARNESS_SEEDS` | 1 | Multi-seed evaluation (1 = disabled) |
| `MAX_IMPROVEMENT_PCT` | 30 | Suspicion gate — flag improvements larger than this |
| `REGRESSION_CHECK_INTERVAL` | 5 | Re-verify metric stability every N accepts |
| `meta_improvement` | false | Enable self-improving program.md meta-loop |
| `META_REVIEW_INTERVAL` | 20 | Experiments between meta-reviews |
| `META_TRIAL_WINDOW` | 10 | Trial period for meta-changes |
| `NOISE_TOLERANCE` | 0.01 | Assumed noise floor for single-run campaigns (1% as decimal) |
| `PIVOT_CHECK_INTERVAL` | 10 | Experiments between PROCEED/REFINE/PIVOT checks |
| `METRIC_DIRECTION` | `lower-is-better` | Optimization direction (`lower-is-better` or `higher-is-better`) |
| `MAX_ITERATIONS` | `unlimited` | Hard cap on total experiments (graceful stop when reached) |
| `CHECKS_TIMEOUT` | 120 | Timeout in seconds for correctness checks (`checks.sh`) |

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
6. Read all configuration keys from program.md (see table above). Apply defaults for any missing keys.
7. Ensure `$WT/campaigns/<campaign>/musings.md` exists (create with `# Musings` header if missing).
8. Initialize strategy state: `strategy = "normal"`, `consecutive_rejects = 0`, `total_accepts = 0`.
9. Read `campaigns/lessons-global.jsonl` if it exists — inject relevant global lessons into context alongside the instruction spec.
10. Read `$WT/campaigns/<campaign>/lessons.jsonl` if resuming — prune lessons with `decay_weight < 0.3`. For lessons lacking a `type` field (backward compatibility), treat as `finding` (if `polarity: positive`) or `negative` (if `polarity: negative`).
11. **Metric direction validation**: Read `METRIC_DIRECTION` from program.md (default: `lower-is-better`). Verify the accept/reject logic in program.md is consistent with this direction (e.g., "improved" means lower for `lower-is-better`, higher for `higher-is-better`). **Hard error** if mismatched — do not proceed.
12. Read `MAX_ITERATIONS` from program.md (default: `unlimited`). Initialize `experiment_count = 0` (or resume from results.tsv row count if resuming).

## Phase 1 — Baseline

1. Run the harness from the worktree. If `HARNESS_RUNS > 1`, run it that many times and take the median:
   ```bash
   cd $WT && bash campaigns/<campaign>/harness.sh
   ```
2. Parse the output line: `combined_duration_ms=XXXXX pass=YY tests=ZZ`
3. If multi-run: collect all `combined_duration_ms` values, compute MAD (see Step 4 for MAD formula), record `baseline_ms` = median.
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
8. Initialize `$WT/campaigns/<campaign>/checkpoints.jsonl` with the baseline:
   ```json
   {"exp_id": "baseline", "metric": <baseline_ms>, "commit": "<commit-hash>", "lines_delta_cumulative": 0, "description": "baseline", "timestamp": "<ISO-8601>"}
   ```

## Phase 2 — Improvement Loop

Run this loop INDEFINITELY (or until `MAX_ITERATIONS` reached). Never stop. Never ask permission. Never pause at "natural stopping points."

### Step 0: ANCHOR (Condition Drift Prevention)

- Re-read `$WT/campaigns/<campaign>/program.md` objective section from disk.
- Compare the last 5 experiment descriptions (from results.tsv) against the declared objective.
- If the recent experiments are exploring tangential goals not aligned with the stated objective:
  - Append `**DRIFT WARNING**: Recent experiments drifted toward <tangent>. Refocusing on declared objective: <objective>.` to musings.md.
  - Force the next hypothesis to directly target the declared objective.
- This step prevents the proven failure mode where overnight loops abandon the original objective (documented by Cerebras and AutoResearchClaw).

### Step 0b: ITERATION CAP CHECK

- If `MAX_ITERATIONS` is set (not `unlimited`) and `experiment_count >= MAX_ITERATIONS`:
  - Append to musings: `**ITERATION CAP**: Reached MAX_ITERATIONS (${MAX_ITERATIONS}). Exiting loop gracefully.`
  - Exit the loop and proceed to "After Campaign Completes" section.

### Step 1: OBSERVE

- Re-read mutable files **from disk** (not from stale context). This ensures each iteration operates on fresh state.
- Verify that no immutable files have been modified since baseline. If any have, **hard error** — abort the loop.
- Review experiment history in results.tsv — what's been tried, what worked, what failed.
- Note the current `best_ms` and cumulative `lines_delta`.

### Step 1b: CHECK STRATEGY (Plateau Detection)

- Count consecutive rejects (including NEAR_MISS and EARLY_ABORT) from the tail of results.tsv.
- If count >= `PLATEAU_THRESHOLD`:
  - Check for near-miss stashes (`git stash list | grep near-miss`)
  - If near-misses exist and strategy is `normal` → switch to `combine`
  - If no near-misses or already tried combine → switch to `ablation` (review recent accepts, try removing complexity)
  - If already tried ablation → switch to `radical` (large structural changes, rethink approach)
  - If already tried radical → switch to `backtrack` (see Step 1d)
- After any ACCEPT, reset `strategy = "normal"` and `consecutive_rejects = 0`.

### Step 1c: COMPUTE UCB1 CATEGORY SCORES

- Group results.tsv rows by `category` column.
- Compute per-category:
  - `success_rate = accepts / attempts`
  - `ucb1_score = success_rate + UCB_EXPLORATION_C * sqrt(ln(total_experiments) / category_attempts)`
- Categories with 0 attempts get `ucb1_score = infinity` (always explored first).
- Rank categories by UCB1 score. The top-ranked category is the preferred target for hypothesis generation in `normal` mode.

### Step 1d: BACKTRACK CHECK

- If strategy is `backtrack`:
  1. Read `$WT/campaigns/<campaign>/checkpoints.jsonl`.
  2. Select the checkpoint with the best metric (lowest `metric` value). If metrics are within 1%, prefer the one with lower `lines_delta_cumulative`.
  3. Execute: `cd $WT && git reset --hard <commit>`
  4. Append to results.tsv: `backtrack-NNN	<checkpoint_metric>	0	backtrack	BACKTRACK	backtracked to <exp_id> (metric: <X>ms)`
  5. Append to musings: `## backtrack-NNN\n**Backtracked to <exp_id>** (metric: <X>ms). Previous HEAD was at exp-MMM (metric: <Y>ms). Reason: exhausted all strategies from current position.`
  6. Reset `strategy = "normal"`, `consecutive_rejects = 0`.
  7. Update `best_ms` to the checkpoint's metric.
  8. Cross-reference musings and results.tsv to identify experiments already tried from this checkpoint — avoid repeating them.

### Step 1e: PROCEED/REFINE/PIVOT CHECK

- Every `PIVOT_CHECK_INTERVAL` experiments, evaluate the campaign's trajectory:
  - **PROCEED** (accept rate in last N > 20%): Current approach is productive. Continue normally.
  - **REFINE** (accept rate >= 10% and <= 20%): Approach has potential but is underperforming. Adjust parameters — tighten/loosen thresholds, shift category priorities, re-read mutable files for missed angles.
  - **PIVOT** (accept rate < 10%): Approach is exhausted. Consult lessons (local and global) for alternative strategies. If lessons suggest a pattern, adopt it. If no relevant lessons, trigger `radical` strategy regardless of consecutive reject count.

### Step 1f: META-REVIEW (Self-Improving program.md)

- **Only if `meta_improvement: true` in program.md.**
- Every `META_REVIEW_INTERVAL` experiments:
  1. **SNAPSHOT**: Copy program.md to program.md.backup.
  2. **ANALYZE**: Read experiment log + musings. Compute:
     - Accept rate over last `META_REVIEW_INTERVAL` experiments
     - Category success rates and UCB1 scores
     - Average improvement per accept
     - Plateau frequency (how often strategy shifted)
     - Near-miss to combine conversion rate
  3. **HYPOTHESIZE META-CHANGE**: Propose ONE specific change to program.md. Allowed changes:
     - Threshold values: `ABORT_THRESHOLD`, `PLATEAU_THRESHOLD`, `NOISE_TOLERANCE`, `UCB_EXPLORATION_C`
     - Category weights/priorities and "root causes to seed" list
     - Strategy progression timing
     - Accept/reject thresholds (the complexity vs. improvement boundary)
     - `HARNESS_RUNS`
  4. **FORBIDDEN meta-changes** (hard-wired safety rails):
     - The evaluation harness (`harness.sh`)
     - The objective direction (lower-is-better vs higher-is-better)
     - The mutable file list
     - `META_REVIEW_INTERVAL` itself (prevents runaway self-modification)
     - Safety-critical config: `MAX_FIX_ATTEMPTS`, `HARD_TIMEOUT`, `MAX_IMPROVEMENT_PCT`
     - Lesson store and logging format
  5. **APPLY**: Edit program.md with the proposed change.
  6. **TRIAL**: Run the next `META_TRIAL_WINDOW` experiments under the new program.md.
  7. **EVALUATE**: Compare accept rate in trial window vs. the preceding window of the same size.
     - Better or equal → KEEP the program.md change
     - Worse → REVERT to program.md.backup
  8. **LOG** in musings.md:
     ```markdown
     ## meta-review-NNN
     **Changed**: <what was changed and from what to what>
     **Trial accept rate**: X/Y (was A/B)
     **Decision**: KEEP | REVERT
     **Learning**: <what was learned about the campaign's dynamics>
     ```

### Step 2: HYPOTHESIZE

- **Check for human override first:** Does `$WT/campaigns/<campaign>/next-idea.md` exist?
  - If yes: read its contents as the hypothesis. Rename to `next-idea.used-exp-NNN.md`. Skip normal generation.
  - If no: proceed with normal hypothesis generation below.

- **Strategy-specific generation:**
  - `normal`: Select the category with the highest UCB1 score (from Step 1c). Propose ONE specific, testable change within that category. If early in the campaign, draw from the "root causes to seed" list. Consult local and global lessons for patterns that have worked in similar contexts.
  - `combine`: Select 2-3 near-miss stashes (`git stash apply stash@{N}`), apply them together, test as one experiment.
  - `ablation`: Review recent accepted commits, propose removing complexity from one of them.
  - `radical`: Propose a fundamentally different approach — different algorithm, restructured data flow, etc.

- If stuck in `normal` mode: re-read all mutable files carefully, combine ideas from near-misses, try radical alternatives, look for patterns in what worked vs. what failed. Consult lessons for unexplored angles.

- **Partial signal guidance:** If recent experiments show partial signals in `intermediates.jsonl` (some intermediate metrics improved while others regressed), focus the hypothesis on extending the improvement to the regressing subset. Example: "Tests 1-5 got faster but tests 6-10 got slower — investigate what's different about tests 6-10."

### Step 2.5: RECORD HYPOTHESIS (Structured Reflection)

Append to `$WT/campaigns/<campaign>/musings.md`:
```markdown
## exp-NNN: <description>
**Category**: <UCB1-selected category> (UCB1 score: X.XX)
**Hypothesis**: <1-2 sentences on why this should improve the metric>
```

### Step 3: IMPLEMENT

- Apply the change to the mutable files in the worktree.
- **Scope check**: Verify that ONLY declared mutable files were modified (`git diff --name-only` against mutable file list from program.md). If any non-mutable file was changed:
  - Rollback: `cd $WT && git checkout -- <all-changed-files>`
  - Log as `REJECT` with description `"scope violation: touched immutable file <path>"`
  - Append to musings: `**SCOPE VIOLATION**: Attempted to modify <path>, which is not in the mutable file list.`
  - Skip to Step 8 (REPEAT).
- Count `lines_delta` for this change (net lines added minus lines removed across all mutable files).
- Tag the change with a `category` from program.md's experiment categories list.

### Step 4: EXECUTE

- Read `HARNESS_RUNS` from program.md (default: 1).
- Run the harness:
  ```bash
  cd $WT && bash campaigns/<campaign>/harness.sh
  ```

**Early abort (per-run):** If the harness supports intermediate output (one line per target file), parse after each line. If the running `combined_duration_ms` already exceeds `best_ms * (1 + ABORT_THRESHOLD)`, kill the harness process. Log status as `EARLY_ABORT` and REJECT immediately (skip to Step 7).

**Intermediate metric capture:** While parsing intermediate output for early abort, also record each checkpoint's label and metric value for `intermediates.jsonl` (see Step 5).

**Multi-run averaging with MAD:** If `HARNESS_RUNS > 1`:
  - Run the harness N times, collecting all `combined_duration_ms` values.
  - Early abort still applies per-run (abort any single run that's clearly losing).
  - Compute **median** as the metric for the accept/reject decision.
  - Compute **MAD** (Median Absolute Deviation):
    ```
    MAD = median(|x_i - median(x)|) for all runs
    normalized_MAD = 1.4826 * MAD
    noise_floor = normalized_MAD / median(x) * 100  (as percentage)
    ```
  - If `spread (max - min) > 3 * normalized_MAD`: flag as "unstable measurement" in musings, add tiebreaker runs up to `MIN_CONFIDENCE_RUNS`.
  - Report `noise_floor` alongside metric in the log description.

**Single-run noise floor:** If `HARNESS_RUNS == 1`, noise floor cannot be computed. The agent relies on the `NOISE_TOLERANCE` config value (default 1%) as the assumed noise floor.

### Step 4d: GOODHART CHECK

After a successful harness run (non-crash, non-abort), apply these guards:

**Multi-seed evaluation (if `HARNESS_SEEDS > 1`):**
- Re-run the harness with env var `HARNESS_SEED=N` for each additional seed (seeds 2 through `HARNESS_SEEDS`).
- Accept only if the improvement holds across ALL seeds (worst-case metric across all seeds must still beat `best_ms`).
- If any seed shows regression, classify as REJECT.

**Suspicion gate:**
- Compute `improvement_pct = (best_ms - new_ms) / best_ms * 100`.
- If `improvement_pct > MAX_IMPROVEMENT_PCT` (default 30%):
  - Log status as `SUSPICIOUS_ACCEPT` instead of `ACCEPT`.
  - Append to musings: `**WARNING**: Unusually large improvement (X%). Verify this is not metric gaming.`
  - The agent MUST attempt to explain WHY the improvement is so large before proceeding. If no plausible explanation, treat as REJECT.

**Periodic regression check:**
- Every `REGRESSION_CHECK_INTERVAL` accepts (tracked via `total_accepts` counter):
  - Re-run the harness WITHOUT any new changes.
  - If the measured metric has drifted from `best_ms` by more than `NOISE_TOLERANCE`:
    - Flag as "metric drift detected" in musings.
    - Update `best_ms` to the re-measured value (recalibrate).
    - Log: `regression-check-NNN	<measured_ms>	0	regression	ACCEPT	metric recalibrated from <old_best> to <new_best>`

### Step 5: MEASURE

- Parse `combined_duration_ms` from harness output (or median if multi-run).
- If harness exited non-zero or output is unparseable, treat as CRASH.
- Compute improvement: `improvement_pct = (best_ms - new_ms) / best_ms * 100`

**Intermediate metric parsing:**
- If the harness produced intermediate output lines (per-file or per-checkpoint), parse each one.
- Compare against the previous experiment's intermediates (from `intermediates.jsonl`).
- Identify **partial signals**: subsets where metrics improved vs. subsets where they regressed.
- Append to `$WT/campaigns/<campaign>/intermediates.jsonl`:
  ```json
  {"exp_id": "exp-NNN", "checkpoints": [{"label": "...", "metric": N}, ...], "primary_metric": N, "partial_signals": ["test-A improved 12%, test-B regressed 3%"]}
  ```

### Step 6: DECIDE

Apply the accept/reject logic from program.md:

**CRASH/FAIL:**
- If the error is trivial (typo, missing import, off-by-one), fix and retry (up to 3 times).
- Otherwise, REJECT.

**EARLY_ABORT:**
- Already handled in Step 4. Log and continue.

**Noise floor check (MAD-based):**
- If `improvement_pct > 0` but `improvement_pct < noise_floor` (from MAD computation or `NOISE_TOLERANCE` if single-run):
  - The improvement is within measurement noise. Require `MIN_CONFIDENCE_RUNS` additional harness runs to confirm.
  - If confirmed after additional runs: proceed to ACCEPT evaluation.
  - If NOT confirmed (median shifts back): classify as REJECT.

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

**On ACCEPT (or SUSPICIOUS_ACCEPT) — before committing, run Step 6b:**

### Step 6b: CORRECTNESS CHECK

- If `$WT/campaigns/<campaign>/checks.sh` exists:
  ```bash
  cd $WT && timeout $CHECKS_TIMEOUT bash campaigns/<campaign>/checks.sh
  ```
- If checks **fail** (non-zero exit) or **timeout**:
  - Downgrade ACCEPT to REJECT. Log description: `"correctness check failed after metric improvement (<improvement_pct>%)"`.
  - Append to musings: `**CORRECTNESS FAILURE**: Metric improved <improvement_pct>% but checks.sh failed. Change breaks correctness.`
  - Rollback: `cd $WT && git checkout -- <changed-files>`
  - Skip to Step 7 (LOG) with REJECT status.
- If checks **pass** (or `checks.sh` does not exist): proceed with ACCEPT.

**On ACCEPT (after passing Step 6b):**
```bash
cd $WT && git add <changed-files> && git commit -m "improve-loop: <description> (<old_ms> -> <new_ms> ms)"
```
Update `best_ms = new_ms`. Reset `strategy = "normal"`, `consecutive_rejects = 0`. Increment `total_accepts`.
Append to `$WT/campaigns/<campaign>/checkpoints.jsonl`:
```json
{"exp_id": "exp-NNN", "metric": <new_ms>, "commit": "<commit-hash>", "lines_delta_cumulative": <total>, "description": "...", "timestamp": "<ISO-8601>"}
```

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
<experiment_id>	<combined_duration_ms>	<lines_delta>	<category>	<ACCEPT|REJECT|NEAR_MISS|EARLY_ABORT|CRASH|SUSPICIOUS_ACCEPT|BACKTRACK>	<description>
```

Use a sequential experiment ID: `exp-001`, `exp-002`, etc. (continue from where results.tsv left off).

### Step 7.5: RECORD LEARNING (Structured Reflection)

Append to `$WT/campaigns/<campaign>/musings.md`:
```markdown
**Result**: <status> (<old_ms> -> <new_ms> ms, noise_floor: X%)
**Partial signals**: <if any intermediate metrics showed directional improvement/regression>
**Learning**: <what was learned — confirmed/refuted hypothesis, surprising observations, what to try differently>
```

### Step 7.6: EXTRACT LESSON (with Curation Gate)

Before persisting ANY lesson, apply the **curation gate** — answer all 3 questions:
1. **Generalizable?** Would this lesson apply to a different experiment in this category, or is it specific to this one change?
2. **Non-obvious?** Does this add information beyond what the accept/reject status already communicates?
3. **Actionable?** Could a fresh agent use this lesson to make a better hypothesis?

If ANY answer is NO, do not persist the lesson. Log in musings: `"Lesson suppressed (failed curation gate: <which question>)"`

**Lesson types** (replaces flat `polarity` field):
- `finding`: a reusable pattern or insight (replaces `polarity: positive`)
- `decision`: a choice between alternatives with rationale
- `experiment`: a tried approach and its outcome pattern
- `question`: an open problem identified during the experiment
- `negative`: a pattern that consistently fails (replaces `polarity: negative`)

**On ACCEPT** (if curation gate passes): Extract a typed lesson:
```json
{"lesson": "<what pattern worked and why>", "type": "finding", "confidence": 0.7, "source_exp": "exp-NNN", "category": "<category>", "timestamp": "<ISO-8601>", "decay_weight": 1.0}
```
Append to `$WT/campaigns/<campaign>/lessons.jsonl`.

**On 3+ consecutive REJECT in same category** (if curation gate passes): Extract a negative lesson:
```json
{"lesson": "<what approach consistently fails in this category and why>", "type": "negative", "confidence": 0.6, "source_exp": "exp-NNN", "category": "<category>", "timestamp": "<ISO-8601>", "decay_weight": 1.0}
```

**On surprising observations or open problems** (if curation gate passes): Extract a question:
```json
{"lesson": "<what remains unexplained or worth investigating>", "type": "question", "confidence": 0.5, "source_exp": "exp-NNN", "category": "<category>", "timestamp": "<ISO-8601>", "decay_weight": 1.0}
```

**On strategic choices** (if curation gate passes): Extract a decision:
```json
{"lesson": "<why X was chosen over Y and the outcome>", "type": "decision", "confidence": 0.7, "source_exp": "exp-NNN", "category": "<category>", "timestamp": "<ISO-8601>", "decay_weight": 1.0}
```

**On successful meta-review KEEP**: Extract a meta-lesson with `"category": "meta"` and `"type": "finding"`.

**Backward compatibility**: If resuming a campaign whose lessons.jsonl uses the old `polarity` field, treat `polarity: positive` as `type: finding` and `polarity: negative` as `type: negative`.

**Lesson decay**: Every 50 experiments, decrease `decay_weight` by 0.1 for all lessons in `lessons.jsonl`. Prune lessons with `decay_weight < 0.3`.

**Global promotion**: Every 50 experiments (or on campaign completion), promote lessons with `confidence >= 0.8` AND `decay_weight >= 0.5` to `$WT/campaigns/lessons-global.jsonl`. Skip duplicates (same `lesson` text).

### Step 8: REPEAT

Go back to Step 1. Do NOT stop.

## Git Operations Summary

| Event | Action |
|-------|--------|
| ACCEPT | `git add <files>` + `git commit -m "improve-loop: ..."` + append to checkpoints.jsonl |
| REJECT | `git checkout -- <files>` |
| NEAR_MISS | `git stash push -m "near-miss-exp-NNN: ..."` |
| EARLY_ABORT | `git checkout -- <files>` (or kill harness + checkout) |
| CRASH (trivial) | Fix, retry (up to 3x) |
| CRASH (fundamental) | `git checkout -- <files>`, log, continue |
| Combine strategy | `git stash apply stash@{N}` for 2-3 near-miss stashes |
| BACKTRACK | `git reset --hard <checkpoint-commit>` |
| SUSPICIOUS_ACCEPT | Same as ACCEPT but with warning in musings |
| META-REVIEW revert | Restore program.md from program.md.backup |

`results.tsv`, `musings.md`, `checkpoints.jsonl`, `lessons.jsonl`, `intermediates.jsonl`, and `run.log` are untracked (gitignored) — they persist across accepts and rejects but are not committed.

**Exception**: `campaigns/lessons-global.jsonl` is NOT gitignored — it MUST be committed during campaign completion (see below).

## After Campaign Completes

When the human decides to stop the loop (or `MAX_ITERATIONS` is reached):
1. Review the worktree branch: `git log --oneline` shows all accepted improvements.
2. Promote high-confidence lessons to global store (if not already done by Step 7.6).
3. **Commit `campaigns/lessons-global.jsonl`** with `git add -f campaigns/lessons-global.jsonl && git commit -m "chore: promote global lessons from <campaign>"`. This file persists across campaigns — without this commit, lessons are lost when the worktree is removed.
4. Squash-merge into main: `git merge --squash improve/<campaign>`
5. Remove the worktree: `git worktree remove .claude/worktrees/improve-<campaign>`

## Important Rules

- **Never modify immutable files** (harness.sh, engine source, game data, other tests).
- **Never weaken assertions** — the tests must remain equally rigorous.
- **Never add dependencies** — optimize with what's available.
- **Never stop the loop** — run until externally interrupted or `MAX_ITERATIONS` reached.
- **Always use worktree paths** — never operate on the main working tree.
- **Always tag experiments with a category** from program.md's taxonomy.
- **Always record hypothesis and learning** in musings.md.
- **Always extract lessons** on ACCEPT and on repeated category failures (subject to curation gate).
- **Never modify safety-critical config during meta-review** (see Step 1f forbidden list).
- **Never accept improvements within noise floor** without additional confirmation runs.
- **Always re-read program.md objective at each iteration** (Step 0: ANCHOR) — prevents condition drift.
- **Always verify scope after IMPLEMENT** (Step 3) — reject any change that touches immutable files.
- **Always run correctness checks** (Step 6b) if `checks.sh` exists — metric improvement that breaks correctness is not real improvement.
- **Always apply curation gate before persisting lessons** — not every observation is worth keeping.
- **Always validate metric direction at startup** (Phase 0, step 11) — optimizing the wrong direction is a silent, expensive bug.
