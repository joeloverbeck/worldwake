---
name: improve-loop
description: "Run a repository-local iterative improvement loop over a campaign in `campaigns/*`: measure a baseline, propose one bounded optimization at a time, run the fixed harness and correctness checks, accept or roll back based on the campaign metric, and keep an audit trail in the campaign files."
---

# Improve Loop

Use this skill when you want Codex to run an autonomous improvement campaign against a fixed evaluation harness defined in `campaigns/<campaign>/`.

This skill is repository-specific. It assumes the Worldwake campaign layout and the Git-based accept-or-rollback workflow already used in this repo.

Read [AGENTS.md](../../../AGENTS.md), [docs/FOUNDATIONS.md](../../../docs/FOUNDATIONS.md), the target campaign's `program.md`, `harness.sh`, and `results.tsv` before making changes.

## Purpose

The loop improves a mutable system against a fixed metric.

For Worldwake campaigns, that means:
- the campaign defines the objective and mutability boundary in `program.md`
- the harness in `harness.sh` is the fixed measurement surface
- `results.tsv` is the experiment log
- accepted changes stay committed in the campaign worktree branch
- rejected changes are rolled back completely

Do not treat this skill as open-ended repo hacking. The campaign is the contract.

## Preconditions

The target directory must exist at `campaigns/<campaign>/` and must contain:
- `program.md`
- `harness.sh`
- `results.tsv`

Optional files:
- `checks.sh`
- `musings.md`
- `next-idea.md`
- `lessons.jsonl`
- `sync-fixtures.sh`

The cross-campaign lesson store lives at:
- `campaigns/lessons-global.jsonl`

If the campaign or required files are missing, stop and report the problem.

## Worktree Requirement

This skill commits and rolls back frequently. Run it in a dedicated git worktree, not in the user's main working tree.

Preferred layout:
- `.codex/worktrees/improve-<campaign>/`

If a dedicated worktree for the campaign does not already exist:
1. create one from the current mainline branch the user wants to optimize from
2. use a branch name such as `improve/<campaign>`
3. perform all reads, edits, harness runs, commits, and rollbacks inside that worktree root

If the user explicitly asks to use `.claude/worktrees/<name>/`, respect that path instead. Otherwise prefer a Codex-owned worktree path.

Before creating or removing a preferred worktree path such as `.codex/worktrees/improve-<campaign>/`:
1. verify whether that path is tracked, ignored, or otherwise special in the current repo
2. if the preferred path is tracked or would dirty the main repo unexpectedly, choose a different untracked worktree location or explicitly plan the cleanup before proceeding
3. do not assume `.codex/worktrees/` is safe just because it is the preferred convention

Before destructive git operations such as `git reset --hard` or `git checkout -- <files>`:
1. run `git status`
2. verify the worktree is clean or the pending changes are only the current experiment
3. preserve accepted work in commits before rollback

## Campaign Contract

Treat `program.md` as the campaign constitution.

Extract and obey:
- objective and metric name
- metric direction
- mutable files
- immutable files
- conditionally mutable files
- accept/reject thresholds
- configuration values such as harness run count, plateau threshold, and check timeout
- experiment categories and seeded root causes

Never modify files declared immutable by the campaign.

Do not modify `harness.sh`, `checks.sh`, or `program.md` during the loop unless the user explicitly changes campaign scope. The evaluation harness must stay fixed during the campaign.

If the user explicitly changes the campaign constitution mid-loop:
1. update only the campaign file(s) required for that change
2. commit that constitution change separately from any experiment code
3. re-read `program.md`, `harness.sh`, `checks.sh`, and any affected campaign notes before resuming
4. restart the loop from the new constitution rather than treating the previous experiment state as still authoritative

## Workflow

### 1. Resolve and validate the campaign

1. Resolve the campaign path in `campaigns/<campaign>/`.
2. Read `program.md`, `harness.sh`, and `results.tsv` fully.
3. Read `checks.sh` if present.
4. Read `campaigns/lessons-global.jsonl` if present.
5. Read campaign-local `lessons.jsonl` and `musings.md` if present.
6. Validate that `results.tsv` headers match the campaign metric and logging schema the campaign expects.
7. Validate that the mutable and immutable paths named in `program.md` exist in the current repo.
8. If campaign assumptions diverge from live code or file layout, stop and surface the mismatch before running experiments.

### 2. Establish the baseline

1. Run the fixed harness from the campaign worktree.
2. If the campaign config requires multiple runs, collect them and use the campaign's comparison rule.
3. Parse the primary metric from harness output using the metric key declared in `program.md`.
4. Record the baseline in `results.tsv` if it is not already present for this campaign branch.
5. Commit the baseline state in the campaign worktree branch so rejected experiments always have a clean rollback target.
6. Create `musings.md` with a minimal heading if it does not exist.

### 3. Observe before each experiment

Before proposing a new change:
- re-read the current mutable files from disk
- review recent `results.tsv` rows
- review recent musings and local lessons
- consult relevant entries in `campaigns/lessons-global.jsonl`
- identify the current best accepted metric
- identify recent categories, accepts, rejects, and near-misses

If the campaign has drifted away from the stated objective, record that in musings and refocus before editing.

### 4. Generate one bounded hypothesis

Choose one experiment at a time.

A good hypothesis for this repo:
- names the suspected hot path or root cause
- cites current profiling or harness evidence when the campaign requires profile-first work
- fits one campaign category from `program.md`
- changes only the campaign's mutable surface
- is small enough to accept or reject cleanly

Prefer profile-first hypotheses. For Worldwake performance campaigns, do not guess blindly at bottlenecks when the campaign requires profiling evidence.

If the correct next move is unclear or risky, use the 1-3-1 rule from [AGENTS.md](../../../AGENTS.md) instead of improvising a broad change.

### 5. Implement the experiment

1. Edit only the mutable files needed for the hypothesis.
2. Keep the change minimal and reviewable.
3. If conditionally mutable files such as tests must move to preserve the intended contract after a production-side optimization, keep those edits narrow and explain why they remain faithful to the campaign goal.
4. If fixture regeneration is required and the campaign provides `sync-fixtures.sh`, run it. Otherwise regenerate the necessary fixtures manually and keep the regeneration steps recorded in musings.
5. Capture a concise experiment description before running the harness.
6. If temporary diagnostic instrumentation is needed to gather profiling evidence, gate it tightly, keep it behavior-neutral when disabled, and record that it is diagnostic-only.

### 6. Measure with the fixed harness

1. Run `harness.sh` from the campaign worktree.
2. Parse the primary metric and any useful intermediate outputs.
3. If the campaign defines multiple harness runs, execute them and compute the campaign's comparison value.
4. If the harness crashes, classify the result honestly:
   - obvious trivial fix in the current experiment: fix and retry within the same experiment
   - genuine failed idea or unstable change: reject the experiment
5. Do not change the harness to rescue an experiment.

### 7. Run correctness checks

If `checks.sh` exists, run it after a metric-improving or metric-preserving candidate before accepting.

For Worldwake campaigns, correctness checks are part of the acceptance gate. A faster result that breaks correctness is a rejection.

Run narrower targeted commands first when useful for local iteration, but do not mark an experiment accepted until the campaign's declared correctness gate passes.

### 8. Accept or reject atomically

Use the campaign's accept/reject logic from `program.md`.

At minimum:
- `ACCEPT`: keep the change, commit it in the campaign worktree branch, append a results row, and record the learning in musings
- `REJECT`: append a results row, capture the learning, then roll back completely to the last accepted commit
- `NEAR_MISS`: record partial signal, then roll back unless the campaign explicitly says near-misses persist
- `CRASH`: record the failure honestly; only keep code if the campaign's own logic allows it

On acceptance:
- make an intentional git commit describing the experiment and metric result
- update campaign-local lessons when the outcome teaches something reusable
- consider promoting strong lessons to `campaigns/lessons-global.jsonl`

On rejection:
- use git to restore the exact last accepted state
- verify the worktree is clean after rollback
- do not leave half-reverted experiments behind

### 9. Track strategy, plateaus, and backtracking

The improvement loop is autonomous, but not mindless.

Track:
- consecutive rejects and near-misses
- which categories have been tried recently
- whether one category is clearly outperforming others
- whether the campaign has plateaued from the current accepted state

Use these escalation modes when progress stalls:
- `normal`: next bounded idea in the strongest category
- `combine`: revisit compatible near-misses and combine only if the interaction is concrete
- `ablation`: simplify previously accepted complexity that may no longer pay for itself
- `radical`: one larger rethink still within mutable scope and campaign rules
- `backtrack`: return to an earlier accepted checkpoint when the current branch of search is exhausted

If all strategies are exhausted and the campaign appears ceiling-limited by architecture, stop and report a ceiling summary to the user instead of grinding indefinitely.

### 10. Keep the audit trail current

Maintain these files as the loop runs:
- `results.tsv`: one row per experiment outcome
- `musings.md`: short factual notes on hypotheses, outcomes, and lessons
- `lessons.jsonl`: campaign-local reusable lessons when warranted
- `campaigns/lessons-global.jsonl`: only for lessons strong enough to help future campaigns

Keep entries factual. Avoid story-like narration.

### 11. Finish the campaign cleanly

When the user says the campaign is finished, do not improvise the close-out. Perform an explicit finish pass:

1. summarize the accepted experiments, current best metric state, and any important plateaus or remaining hotspots in `musings.md` if the campaign tracks notes there
2. remove temporary profiling or diagnostic instrumentation that was added only to guide experiments, unless the user explicitly wants it retained as a supported feature
3. re-run the campaign harness and the campaign correctness gate on the de-instrumented final state
4. verify the campaign worktree is clean except for the intended final landing changes
5. land the result using the user-requested mode:
   - keep the campaign branch as-is
   - squash-merge into the target branch
   - prepare the branch for a PR
6. before destructive branch moves or worktree removal, confirm the target worktree is clean and confirm whether removing the worktree path will dirty another repo worktree
7. report the final landed commit and any follow-up risks or cleanup still left for the user

## Report Format During a Live Loop

Use compact updates in the conversation. Typical update structure:

```markdown
# Improve Loop: <campaign>

**Worktree**: <path>
**Best metric**: <value>
**Current experiment**: <exp-id>
**Category**: <category>

## Hypothesis
- <one concise statement>

## Result
- <ACCEPT | REJECT | NEAR_MISS | CRASH>
- <metric summary>
- <checks summary>
- <rollback or commit summary>

## Next Move
- <next hypothesis or plateau note>
```

## Guardrails

- The campaign harness is the fixed evaluation surface. Do not modify it during the loop.
- Respect immutable files and mutability boundaries from `program.md`.
- Use a dedicated worktree. Do not churn commits and resets in the user's main working tree.
- Keep one experiment hypothesis per iteration.
- Accept or reject atomically. Never leave the branch between accepted states.
- Follow [AGENTS.md](../../../AGENTS.md): minimal changes, DRY, TDD for bug fixes, and 1-3-1 for unclear risky decisions.
- Follow [docs/FOUNDATIONS.md](../../../docs/FOUNDATIONS.md). Performance work must compress computation, never causality.
- Preserve determinism, conservation, append-only event history, belief-only planning, and system decoupling.
- If a campaign optimization changes behavior and requires golden test updates, preserve the original proof intent rather than weakening assertions.
- Temporary instrumentation is not an accepted final optimization by default. Remove diagnostic-only profiling code before campaign finish unless the user explicitly wants it retained.
- Promote cross-campaign lessons selectively. `campaigns/lessons-global.jsonl` should contain durable findings, not noisy one-off notes.
- If the loop is blocked by a compile failure or harness break unrelated to the current experiment, stop and report that blocker before continuing.
