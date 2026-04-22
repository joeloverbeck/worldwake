**Status**: COMPLETED

# Build a repo-local Claude Code handoff system

Your job is to design and implement the best possible **repo-local Claude Code handoff skill** for continuing work in a fresh session with minimal token waste and minimal re-discovery.

I do **not** want vague advice. I want a concrete design and the exact files needed.

## Objective

Create a manual Claude Code skill called `/handoff` that produces a **copy-paste-ready continuation packet** from the **current live session**. The packet must preserve everything that matters to resume the task in a fresh Claude Code session, while staying compact enough that it does not become its own context bomb.

The skill must work for:
- coding sessions
- debugging sessions
- research/exploration sessions
- mixed sessions with repo state + conversational decisions
- sessions with no code changes
- sessions outside a git repo

## Non-negotiable design constraints

1. Make this a **repo-local skill** at:
   - `.claude/skills/handoff/SKILL.md`

2. Make it **manual-only**:
   - Use `disable-model-invocation: true`

3. **Do not use `context: fork`**
   - This skill must see the current conversation history, so it must run inline, not in a forked subagent.

4. Keep `SKILL.md` **small and focused**
   - Put templates, examples, and checklists in supporting files.
   - Do not let `SKILL.md` become a giant blob.

5. Use **live workspace state** at invocation time
   - Add a helper script and/or dynamic shell injection so the skill can see current repo state when invoked.

6. Optimize for **low token burn**
   - Default to a compact, high-signal output.
   - Do not enable ultrathink.
   - Prefer low effort unless there is a very strong reason not to.

7. Do **not** rely on `/compact`
   - The skill must work independently and be useful right before `/clear`.

8. The summary must be **operational, not literary**
   - It should help the next session continue immediately.
   - It should not read like a narrative recap.

9. The summary must **separate facts from guesses**
   - Never invent user instructions.
   - Explicitly distinguish:
     - user-explicit requirements
     - evidence-backed facts
     - hypotheses / things not yet verified

10. Assume that in a fresh session:
   - root `CLAUDE.md` and auto memory will reload
   - nested `CLAUDE.md`, path-scoped rules, and previously invoked skills may need explicit reloading/reinvocation

The handoff packet must account for that instead of duplicating everything blindly.

## Deliverables

Return all of the following:

1. Recommended architecture
2. Exact file tree
3. Full contents of every file
4. Short rationale for major design choices
5. Usage examples

### Expected files

At minimum:

- `.claude/skills/handoff/SKILL.md`
- `.claude/skills/handoff/template.md`
- `.claude/skills/handoff/checklist.md`
- `.claude/skills/handoff/examples.md`
- `.claude/skills/handoff/scripts/workspace_snapshot.sh`

Optional if justified:

- `.claude/handoffs/README.md`
- hook companion files or settings snippets

## Required frontmatter direction

Start from something close to this unless you have a better reason:

```yaml
---
name: handoff
description: Generate a minimal continuation packet for restarting work in a fresh Claude Code session
disable-model-invocation: true
effort: low
---
```

Do **not** add `context: fork`.

## Required behavior of `/handoff`

When `/handoff` runs, it must:

1. Read the **current session context** and extract:
   - current objective
   - latest explicit user ask
   - hard constraints
   - decisions already made
   - dead ends / rejected approaches
   - blockers
   - next concrete action

2. Inject **live workspace state** such as:
   - repo root
   - working directory
   - current branch
   - HEAD commit
   - staged / unstaged / untracked summary
   - diff stats
   - recent commits
   - note if not in git repo

3. Produce a markdown `RESUME PACKET` that is:
   - compact
   - high signal
   - copy-paste ready
   - immediately actionable in a fresh session

4. Prefer **listing exact file paths, symbols, IDs, commands, and status**
   over verbose prose.

5. If practical and permission-safe, also support saving the same packet to:
   - `.claude/handoffs/latest.md`
   - and a timestamped file

But if writing files would complicate the design too much, prioritize a great chat output first.

## Required output schema

The generated handoff packet must use this structure in this exact order:

1. `Objective`
2. `Latest explicit user request`
3. `Hard constraints`
4. `Current status`
5. `Relevant files and symbols`
6. `Workspace state`
7. `Decisions already made`
8. `Dead ends / do not retry`
9. `Evidence-backed facts`
10. `Hypotheses / things to verify`
11. `Open blockers`
12. `Tests / commands already run`
13. `Things that will NOT survive a fresh session`
14. `Reinvoke / reread on next session`
15. `Ordered next steps`
16. `Paste into new session`

## Content rules

The packet must obey all of these rules:

- No giant code excerpts
- No raw logs except tiny excerpts when absolutely necessary
- If something is unknown, write `Unknown`
- If no tests were run, say so explicitly
- If there are no file changes, say so explicitly
- Do not restate broad project conventions already covered by root `CLAUDE.md` unless the current session overrode them
- Explicitly call out nested rules, nested `CLAUDE.md`, or skills that need to be reloaded manually
- Include dead ends so the next session does not waste tokens rediscovering failed approaches
- Include the exact next prompt the new session should receive
- Optimize for “continue immediately,” not for completeness theater

## Required live snapshot data

Your helper script or dynamic shell block should try to gather:

- absolute working directory
- git repo root if present
- current branch
- current HEAD short SHA
- `git status --short`
- staged diff stat
- unstaged diff stat
- untracked files summary
- recent commits (short list)
- whether there are merge/rebase/cherry-pick states in progress, if easy to detect

If shell execution is disabled by policy, the skill must still degrade gracefully:
- still produce the packet
- clearly mark workspace data as unavailable/unverified

## Anti-goals

Do not do any of the following:

- build a giant “memory bank”
- turn the skill into a general project manager
- rely on `/compact`
- use a forked context
- add broad unsafe auto-approval
- output only a generic summary with no operational details
- duplicate root `CLAUDE.md` content unnecessarily
- blur facts and guesses

## Optional companion hook design

If and only if it materially improves reliability without adding steady-state token bloat, add an optional companion design:

- `PostCompact` hook:
  - save Claude’s generated `compact_summary` to a timestamped file

- `SessionStart` hook with matcher `compact`:
  - inject only a **tiny pointer** to the latest saved handoff file
  - do **not** inject the whole saved summary

If you add hooks, explain why they help and keep them lightweight.

## Outcome

- Completion date: 2026-04-22
- What actually changed: This brainstorming brief was used as source input and is no longer an active planning document.
- Deviations from original plan: None recorded here. Any implementation or adaptation work happened outside this document.
- Verification results: User confirmed the brief has already been exploited and requested archival.

## Acceptance tests

Your solution is not done until you show how the design handles:

1. A coding session with modified files and failed tests
2. A debugging/research session with many dead ends and no code changes
3. A session where nested rules or extra skills matter
4. A non-git directory
5. A fresh session restarted from the generated packet

## Final answer format

Return your answer in this order:

1. **Architecture recommendation**
2. **Exact file tree**
3. **Full contents of every file** (no diffs)
4. **Short design rationale**
5. **Usage guide**
6. **Why this beats “just use /compact”**

Make decisions. Do not ask me follow-up questions unless something is genuinely impossible.
