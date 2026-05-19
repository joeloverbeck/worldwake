---
name: implement-spec-tickets
description: "Run the standard Worldwake implementation loop for a spec: repeatedly select the next active ticket, invoke implement-ticket with the originating spec as authority, apply implement-ticket audit suggestions, post-review completed tickets, apply post-ticket-review audit suggestions when review creates follow-up work, commit each iteration, continue through follow-up tickets first, archive the originating spec, then create and push a final branch."
user-invocable: true
arguments:
  - name: spec_path
    description: "Path or glob for the originating Worldwake spec in specs/ that the ticket family implements."
    required: true
  - name: ticket_path
    description: "Optional first ticket path or glob. If omitted, choose the first active tickets/*.md entry that belongs to the originating spec family."
    required: false
---

# Implement Spec Tickets

Run the full Worldwake ticket-family loop without making the user manually reissue the same skill commands.

This is an orchestration skill. Do not reimplement `implement-ticket`, `skill-audit`, or `post-ticket-review` here. Load and obey those skills when each phase calls for them, and let their narrower Worldwake guardrails control the phase they own.

Require a reset checkpoint between ticket iterations. Each ticket should start from the live repo, current spec, current ticket, and current child-skill guidance rather than from assumptions accumulated during the previous ticket. After an iteration commit, write a repo-local state file, print a compact handoff summary, and request or perform context compaction / fresh-session restart when the Codex surface supports it. Continue in the same context only when the next target is an immediate same-seam follow-up, the context is still small, and the proof/review/audit output from the previous iteration was compact.

The persisted state file is the source of truth for resuming after `/new`; the printed handoff is only a readable mirror.

## Required Reads

Before the first loop iteration, read:

- `AGENTS.md`
- `docs/FOUNDATIONS.md`
- `docs/archival-workflow.md`
- `tickets/README.md`
- `tickets/_TEMPLATE.md`
- `.codex/skills/implement-ticket/SKILL.md`
- `.codex/skills/skill-audit/SKILL.md`
- `.codex/skills/post-ticket-review/SKILL.md`
- the resolved originating spec

When a phase invokes one of the child skills, read any focused references that child skill requires. Do not rely on this harness as a substitute for those reads.

## State File

Use `.codex/run-state/implement-spec-tickets.json` as the harness state file. Create `.codex/run-state/` if needed.

This file represents one active harness run. If it already exists and names a different `originating_spec` than the requested run, do not overwrite it silently. Treat it as a durable record for another spec family: either resume that family, or ask the user whether to archive/replace the state file before starting the new family. Do not mix queues from two spec families in one state file.

Keep the file small and machine-readable. Update it after intake, after every iteration commit, after blockers, and after final spec archival. A useful shape is:

```json
{
  "originating_spec": "specs/S123-example.md",
  "archived_spec": null,
  "worktree_root": "/home/joeloverbeck/projects/worldwake",
  "starting_branch": "main",
  "current_branch": "spec-123-example",
  "base_head": "0123456789abcdef0123456789abcdef01234567",
  "last_ticket": "tickets/S123EXAMPLE-001.md",
  "last_result": "completed_archived",
  "last_work_commit": "abc1234",
  "last_state_commit": "containing_commit",
  "last_state_commit_kind": "separate",
  "next_target": "tickets/S123EXAMPLE-002.md",
  "queue": [
    "tickets/S123EXAMPLE-002.md"
  ],
  "implement_ticket_audit": "pending",
  "post_ticket_review_audit": "not_required",
  "last_implement_ticket_audit": "done",
  "last_post_ticket_review_audit": "done",
  "blocked": false,
  "blocker": null,
  "dirty_state": "clean",
  "updated_at": "YYYY-MM-DD"
}
```

On resume after `/new`, read this state file first, then verify every important field against live repo state before continuing:

- `originating_spec` still exists unless `archived_spec` is set
- `worktree_root` matches the current repository root
- `current_branch` matches the current branch, or the mismatch is intentional and explained before continuing
- `base_head` is reachable from `HEAD`
- `next_target` exists and is still active, unless the next action is final spec archival
- queued ticket paths still exist and still belong to the originating spec family
- `last_work_commit` is reachable from `HEAD`
- `last_state_commit` is either `null` / `"none"`, `"containing_commit"`, or a commit reachable from `HEAD`; older state files may contain legacy `"self"` here, but before invoking a child skill you must resolve legacy `"self"` to `"containing_commit"` or to the reachable commit that last changed `.codex/run-state/implement-spec-tickets.json`
- `last_state_commit_kind`, when present, is `amended`, `separate`, or `none`; if it is missing in an older state file, infer `separate` only when `last_state_commit` is a real reachable commit or `"containing_commit"` and infer `none` only when `last_state_commit` is `"none"` / `null`
- for `last_state_commit: "containing_commit"`, verify `git log -n 1 -- .codex/run-state/implement-spec-tickets.json` is reachable from `HEAD` and treat that commit as the actual state persistence commit for resume checks and handoff reporting
- child-audit markers, when present, are compatible with the last iteration state:
  - `implement_ticket_audit` is `done`, `pending`, or `skipped:<reason>`
  - `post_ticket_review_audit` is `done`, `pending`, `not_required`, or `skipped:<reason>`
  - `last_implement_ticket_audit`, when present, is `done`, `pending`, or `skipped:<reason>` and describes the completed `last_ticket`
  - `last_post_ticket_review_audit`, when present, is `done`, `pending`, `not_required`, or `skipped:<reason>` and describes the completed `last_ticket`
  - if a marker is missing for an in-flight iteration, infer it from live evidence only when the compact audit/review block or changed skill diff is visible; otherwise set it to `pending` before committing the iteration
- `git status --short` matches or safely supersedes `dirty_state`

After resume validation, state the checked fields compactly before invoking a child skill:

```text
Resume validation checked: spec, worktree_root, branch, base_head, next_target, queue, last_work_commit, last_state_commit, child-audit markers, dirty_state.
```

If the state file conflicts with the live repo, trust the live repo and patch the state file before continuing. If the conflict changes the next target or archival readiness, state that explicitly before invoking a child skill.

`last_work_commit` means the commit that contains the ticket implementation, review/archive move, follow-up creation, and any applied child-skill hardening for the iteration. Record full work commit SHAs in the JSON state file; short SHAs are acceptable in printed handoffs only. `last_state_commit` identifies the state persistence shape: the same sha as `last_work_commit` when amended into that commit, `"containing_commit"` when the state file is committed separately and the commit that contains the file is discovered with `git log -n 1 -- .codex/run-state/implement-spec-tickets.json`, or `"none"` when intentionally left uncommitted. Do not try to make a separate state commit contain its own final SHA; changing the file changes the commit SHA and creates a self-reference loop. `last_state_commit_kind` records the persistence shape (`amended`, `separate`, or `none`) so the state file remains resumable after `/new`; do not rely on the printed handoff as the only place that contains the separate state commit sha.

## Intake

1. Resolve `spec_path` to exactly one live file under `specs/`. If it is missing, ambiguous, or already archived, stop and ask for the exact active spec path.
2. Snapshot the worktree with `git status --short`.
3. Classify pre-existing dirty paths before doing any work:
   - ticket/spec family state for the active run
   - existing user work that the run must not absorb silently
   - unrelated noise
4. If the initial snapshot shows staged/index entries, inspect `git diff --cached --name-status` and classify them separately from unstaged dirt. Pre-existing staged unrelated work must not be absorbed by a harness commit.
5. If `.codex/run-state/implement-spec-tickets.json` already exists, read and validate it even on a normal first invocation.
   - If it names the same originating spec, treat the invocation as a resume or restart of that family and verify it against the live repo before invoking child skills.
   - If it names a different originating spec, stop before any child skill invocation and ask whether to preserve, replace, or explicitly retire the existing run state.
   - If it conflicts with live repo state for the same originating spec, trust the live repo and refresh the state file before invoking child skills.
6. If unrelated dirty paths exist and this harness is expected to stage and commit work, stop and ask whether those paths should be included. Do not silently commit unrelated user work.
7. Create or switch to the family branch before the first harness commit. Derive a concise branch name from the spec id or filename, record `starting_branch`, `current_branch`, `base_head`, and `worktree_root` in state, and do not commit to the starting branch unless the user explicitly approves that branch as the work branch.
8. Resolve the first ticket:
   - if `ticket_path` is supplied, resolve it to exactly one active ticket under `tickets/`
   - otherwise inspect active `tickets/*.md`, choose the first ticket in lexical order whose filename, `Deps`, problem statement, or explicit active deliverable wording ties it to the originating spec, and state the selection
9. Build the initial pending queue from active tickets that clearly belong to the same originating spec family. Include a ticket only when it owns active implementation work for the spec: matching family id, explicit active spec dependency, or active deliverable wording. Treat incidental historical mentions, roadmap examples, and archived-context references as evidence to read, not queue membership. Keep the queue lexical and append-only; do not jump ahead of a follow-up created by the current iteration.
10. Decide how to handle pre-existing untracked same-family ticket/spec files before implementation. Include them only when they are required to define the active family queue, dependency chain, or truthful handoff for the current iteration.
11. Write or refresh `.codex/run-state/implement-spec-tickets.json` with the resolved spec, branch/worktree metadata, initial target, initial queue, child-audit markers initialized for the first iteration, dirty-state classification, and `blocked: false`.

When advancing from one ticket to the next, copy the completed ticket's audit markers into `last_implement_ticket_audit` and `last_post_ticket_review_audit`, then reset the current child-audit markers for the new `next_target` before invoking any child skill. A completed iteration's `done` markers describe the just-finished `last_ticket`; they must not be carried forward as if the new current target's child phases have already run. Use `implement_ticket_audit: "pending"` for the next target, and `post_ticket_review_audit: "not_required"` until review actually creates or updates handoff surfaces that trigger it.

If an old state file exists but no reset or blocker will occur before the first target is processed, a pre-work state refresh may be deferred until the first iteration state commit. This is allowed only when the next target, queue, ownership classification, and archival readiness have been validated against live repo state before invoking child skills.

If an existing same-family state file is missing branch/worktree/base metadata, child-audit markers, or other resume-critical fields, it is also acceptable to make a small state-only commit before implementation begins. Use this only to make the harness resumable and keep that commit limited to `.codex/run-state/implement-spec-tickets.json`.

## Loop

Repeat until there is no active ticket left in the queue and no newly created follow-up ticket takes priority.

Before invoking the next ticket, decide whether the current context is still suitable:

- If the next target is not an immediate same-seam follow-up from the just-finished ticket, stop after the persisted handoff and require a fresh invocation.
- If the previous iteration produced broad proof output, material post-review edits, nontrivial child-skill audit findings, or a noisy failure investigation, stop after the persisted handoff and require a fresh invocation.
- Continue in the same context only when the next target is an immediate same-seam follow-up and the current context still contains enough room to reload the live child-skill guidance, reassess the ticket, implement, verify, review, audit, commit, and persist the next handoff.

### 1. Implement The Target Ticket

Invoke the implementation phase as if the user had said:

```text
$implement-ticket <ticket> . Rely on <originating-spec>
```

Use the live Worldwake `implement-ticket` skill exactly. The child skill owns reassessment, implementation, proof, closeout wording, and any decision to create a follow-up ticket needed for an honest closeout.

For broad expected-pass proof commands that run inside the child phase, such as `scripts/verify.sh`, keep command output compact where the tool surface allows it. Preserve complete failure diagnostics when a command fails, but for passing broad gates prefer capped output plus a concise recorded gate list so successful verification does not saturate the context before the required handoff/reset boundary. For broad Cargo proof that is expected to pass, prefer capped tool output and quiet Cargo output when compatible with the command, such as `cargo test --workspace --quiet`; if the broad command fails or quiet output hides the useful cause, rerun the failing narrow command or same command without quiet/capping as needed to capture actionable diagnostics.

If implementation ends blocked:

- if a concrete follow-up ticket was created or named as the next owner, put that follow-up at the front of the queue and continue the loop
- if no follow-up exists, stop the harness and report the blocker, current ticket, proof gap, and next required action

### 2. Audit And Apply Implement-Ticket Suggestions

Run the audit phase as if the user had said:

```text
$skill-audit .codex/skills/implement-ticket
```

Apply every audit suggestion that is specific, evidence-backed, and compatible with `AGENTS.md` and `docs/FOUNDATIONS.md`. This harness is the user's explicit authorization to implement those suggestions; do not wait for a separate "Implement suggestions" prompt.

Reject or defer only suggestions that are clearly wrong, speculative, duplicate already-live guidance, or would weaken Worldwake's ticket truthing, proof integrity, FOUNDATIONS alignment, or Cargo discipline.

Before starting this phase, set or confirm `implement_ticket_audit: "pending"` in the run state for the current iteration. After the compact audit block is printed and any accepted skill edits are applied or rejected, update it to `done` or `skipped:<reason>`. Do not commit an iteration with this marker still `pending` unless the harness is intentionally stopping before the iteration commit.

Before applying or rejecting suggestions, print a compact visible audit result:

```text
Child skill audit:
- Target skill: .codex/skills/implement-ticket
- Findings: <N issues, N improvements, N features>
- Evidence basis: <one-line session evidence checked, especially when Findings is 0>
- Apply: <specific suggestions to patch, or "none">
- Reject/defer: <specific suggestions and reason, or "none">
```

For harness-internal child phases, this compact block is the required visible report. Apply the child skill's evidence standards, guardrails, and edit rules, but do not emit the full child audit report unless the phase blocks, creates material follow-up decisions, or needs the extra detail for a truthful handoff.

After editing the skill, rerun a focused hygiene check over changed skill files, usually `git diff --check -- .codex/skills/implement-ticket`.

### 3. Review Completed Tickets

If the target ticket is marked `COMPLETED`, run the review phase as if the user had said:

```text
$post-ticket-review <completed-ticket>
```

Use the live Worldwake `post-ticket-review` skill exactly. The child skill owns closeout truthing, archival, dependency/path repairs, and follow-up ticket creation.

After the review phase, print a compact visible review result:

```text
Post-ticket review:
- Target ticket: <ticket path or archived path>
- Archival status: <archived | already archived | blocked>
- Closeout truthing: <validated unchanged | factually corrected | blocked>
- Reference sweep: <paths repaired or "no stale active-path refs found">
- Follow-ups: <created/updated ticket paths or "none">
- Verification: <rerun proof command/result or why rerun was not needed>
```

If `post-ticket-review` blocks archival because same-seam work remains, put the active ticket back at the front of the queue and continue through `implement-ticket` unless the review says a user decision is required. Do not archive a blocked ticket.

### 4. Audit Post-Ticket Review When It Changes Handoff Surfaces

If `post-ticket-review` creates or materially updates a follow-up ticket, active spec, active ticket dependency, implementation-order entry, or current contract doc, run:

```text
$skill-audit .codex/skills/post-ticket-review
```

Apply every sound, evidence-backed suggestion under the same rules as the `implement-ticket` audit. Rerun focused hygiene over changed post-review skill files.

Archive-path and dependency repairs in active specs, implementation-order prose, or active sibling tickets count as material handoff updates for this trigger, even when the repairs are mechanical.

Track this phase in the state file as well. Set `post_ticket_review_audit: "pending"` when the trigger is met, `done` after the compact audit block and any accepted edits are complete, `not_required` when the trigger is not met, or `skipped:<reason>` only when skipping is explicit and justified. Treat a missing marker on resume as `pending` if the review materially changed handoff surfaces and no evidence shows the audit ran.

Put any review-created follow-up ticket at the front of the queue, ahead of the original lexical next ticket. If review only truthed a spec, ticket dependency, or current contract doc and created no follow-up, keep the existing queue order.

### 5. Commit The Iteration

Before committing:

1. Refresh `git status --short`.
2. Inspect `git diff --cached --name-status` before staging owned paths. If pre-existing staged entries are unrelated to the current iteration, unstage those paths or stop for approval before committing.
3. Verify all dirty paths are either owned by this iteration, previously approved for inclusion, or generated/ignored artifacts that should remain unstaged.
4. Run `git diff --check` or the child skills' stronger equivalent over tracked and newly created owned files.
5. Stage only approved owned paths plus any pre-existing dirty paths the user explicitly allowed this harness to include.
   - If `.codex/run-state/implement-spec-tickets.json` is dirty from intake or resume refresh, do not stage it for the iteration work commit unless it already contains the final post-iteration state, including the correct `last_work_commit` shape. If it is already staged prematurely, unstage it before committing implementation, review, archive, follow-up, or skill-hardening changes.
6. Re-run `git diff --cached --name-status` after staging and confirm every staged path is owned by this iteration, explicitly approved, or intentional same-family state needed for the queue/handoff.
7. Commit with a concise message that names the ticket id and whether the iteration included implementation, review/archive, follow-up creation, and skill hardening.

When `post-ticket-review` archived a ticket with `git mv`, do not try to stage the now-missing active ticket path by name. Stage the archive destination and other edited owned paths, then confirm the source deletion or rename is staged with `git diff --cached --name-status`. If a staging command still fails with a missing-path pathspec for the old active ticket, do not retry with that source path; inspect `git status --short` or `git diff --cached --name-status` and continue only if the rename is already staged as `R old -> archive/...`.

If non-destructive git index commands needed for this harness step fail because Codex cannot write the git index or reports a sandbox/read-only filesystem error, rerun the same staging or commit command with the required approval/escalation and record the retry in the handoff or final report. Do not use this as permission for destructive commands or for staging unrelated paths.

If nothing changed after an iteration, do not create an empty commit. Record that there was no commit for that iteration and why.

### 6. Persist State And Prepare Context Reset

After each iteration work commit, update `.codex/run-state/implement-spec-tickets.json` before context compaction or a fresh-session restart. Include:

- originating spec path or archived spec path
- worktree root, starting branch, current branch, and base commit for the run
- last ticket processed and result
- `last_work_commit`: the ticket iteration work commit sha, or `"none"` if no work commit was created
- `last_state_commit`: the same sha as `last_work_commit` when amended into the work commit, `"containing_commit"` when the state file is committed separately, or `"none"` when the state file remains intentionally uncommitted
- `last_state_commit_kind`: `separate`, `amended`, or `none`
- next target, or `"final_spec_archive"` / `"blocked"`
- remaining queue
- child-audit markers for the completed or in-flight iteration:
  - `implement_ticket_audit`
  - `post_ticket_review_audit`
  - `last_implement_ticket_audit` and `last_post_ticket_review_audit`, when advancing to a new `next_target`
- blocker summary when blocked
- dirty-state classification
- `updated_at`

Before staging the state file, validate its schema against the allowed marker vocabulary in this skill. Audit markers must be exact values: `implement_ticket_audit` and `last_implement_ticket_audit` are `done`, `pending`, or `skipped:<reason>`; `post_ticket_review_audit` and `last_post_ticket_review_audit` are `done`, `pending`, `not_required`, or `skipped:<reason>`. Do not encode audit-result detail such as "completed_no_findings" in these enum fields; put that detail in the printed handoff or review/audit blocks. Also check that `last_state_commit` / `last_state_commit_kind`, `next_target`, queue paths, and `dirty_state` match the allowed shapes before committing the state file.

Normalize `dirty_state` after committing owned paths: refresh `git status --short` and record only remaining uncommitted paths. When Cargo or package/tool commands ran, or prior state already names ignored artifacts, also refresh ignored-aware status for affected paths before writing `dirty_state`. Classify remaining paths as `unrelated dirty`, `expected ignored artifacts`, or `blocked owned leftovers`.

If the state file itself changes after the work commit, either:

- amend it into the work commit before reporting the sha, then set `last_work_commit` and `last_state_commit` to that amended commit sha and set `last_state_commit_kind` to `amended`; or
- commit it separately as a harness-state commit, set `last_work_commit` to the implementation/archive commit, set `last_state_commit` to `"containing_commit"`, and set `last_state_commit_kind` to `separate`. After the commit succeeds, report the actual state commit sha from `git rev-parse HEAD` in the handoff; on resume, rediscover it with `git log -n 1 -- .codex/run-state/implement-spec-tickets.json`.

When the state file is committed separately, the committed `dirty_state` must describe the expected worktree state after that state commit succeeds, not the transient state-file dirt before the commit. In the normal successful case, record `clean`. If the state file intentionally remains uncommitted, only then mention the dirty state file in `dirty_state`, set `last_state_commit` to `"none"`, and set `last_state_commit_kind` to `none`.

Then print a short handoff that mirrors the state file:

```text
Harness handoff:
- Originating spec: <active or archived path>
- Branch: <current branch, with starting branch if different>
- Last ticket processed: <ticket id and result>
- Work commit: <sha or "none">
- State commit: <sha or "none" | same as work commit>
- Next target: <follow-up ticket path | next queued ticket path | final spec archive | blocked>
- Queue: <remaining active ticket paths>
- Dirty state: <clean | expected ignored artifacts | owned/unrelated paths still present>
- State file: .codex/run-state/implement-spec-tickets.json
- Required next invocation: $implement-spec-tickets <spec> <next-target-if-any>
```

For an intentional reset-boundary stop between non-follow-up tickets, this `Harness handoff` is the required interim report; reserve the full `Final Report` checklist for blocked or final-family exits.

Then prefer one of these reset paths:

- If Codex exposes context compaction or the user can issue `/new`, request it before starting the next ticket.
- If compaction is unavailable but the context is still small and the next target is an immediate follow-up from the just-finished ticket, continuing in the same context is acceptable.
- If the context is large, proof output was noisy, or the next queued ticket is not a direct same-seam follow-up, stop after the handoff instead of starting the next ticket in a saturated context.

The next session must reload this skill and the child skills from disk, then resume from the state file and live repo state.

## Queue And Follow-Up Rules

- A follow-up ticket created by `implement-ticket` or `post-ticket-review` is always the next target.
- If multiple follow-ups are created in one iteration, choose the one explicitly identified as the next owner. If none is identified, choose the lowest lexical path and record the ordering.
- Do not skip active tickets in the originating spec family unless their `Deps`, status, or review result proves they are no longer valid next work.
- If a sibling ticket is absorbed into the current ticket, update the queue after the child skill has made that sibling truthful.
- If a ticket is archived, remove its old active path from the queue and replace dependency references according to `post-ticket-review`.

## Final Spec Archive

When all originating-spec tickets are completed, reviewed, archived, and committed:

1. Re-read the originating spec.
2. Confirm no active `tickets/*.md` still names the spec as active implementation work.
3. Update the spec status and `## Outcome` according to `docs/archival-workflow.md`.
4. Move the spec to `archive/specs/`, preferring `git mv` when tracked and plain `mv` when untracked.
5. Confirm the original `specs/` path no longer exists.
6. Sweep active tickets, docs, specs, `specs/IMPLEMENTATION-ORDER.md`, same-family archived tickets, and same-seam triage/report docs for stale active-spec path references. Repair actionable references to the archived path, including archived ticket `Deps`, current proof commands, and direct implementation-reference snippets that now point at the archived spec. Leave historical references only when clearly harmless or explicitly labelled as historical intake context.
7. Run hygiene over the spec archive move and reference repairs.
8. Commit the spec archive as its own finalization commit unless it is already included in the last ticket-family commit for a clear reason.
9. Update `.codex/run-state/implement-spec-tickets.json` with `archived_spec`, `next_target: null`, an empty queue, `blocked: false`, the final commit sha, branch/worktree metadata, terminal child-audit markers, and clean dirty-state classification. A completed run must not leave any current child-audit marker as `pending`; copy the last completed ticket's markers into `last_implement_ticket_audit` / `last_post_ticket_review_audit` when useful, and set current markers to `not_required` or another truthful terminal value because there is no active `next_target`.

If `git mv`, `git add`, or `git commit` fails during final archival because Codex cannot write the git index or reports a sandbox/read-only filesystem error, rerun the same non-destructive command with the required approval/escalation and record the first failure plus retry result. Do not widen the staged set while retrying.

## Branch And Push

After the final archive commit and any required final state-file persistence commit:

1. Refresh `git status --short`. Stop if uncommitted owned changes remain.
2. Confirm the current branch matches the recorded `current_branch` and is not the original starting branch unless the user explicitly approved using that branch for the harness work.
3. Run the repo's pre-PR verification gate before pushing: `./scripts/verify.sh` (space-conscious by default per `AGENTS.md`). If it cannot be run, stop before pushing unless the user explicitly approves a skip; record the skip reason in the final report and state file.
4. Push the recorded current branch to the configured remote.
5. Report the branch name, pushed remote, commits created by the harness, archived spec path, archived ticket paths, any follow-up tickets left active, and the pre-push verification command/result or explicitly approved skip.

If branch setup during intake or push during finalization fails because Codex cannot write the git ref in the sandbox, cannot resolve the remote host, or otherwise hits a clear sandbox/network restriction, rerun the same branch/push command with the required approval or escalation. Record both the first failure and the successful retry, or the remaining blocker if escalation still fails.

Do not create or push a branch if the implementation loop stopped blocked or if the worktree still contains unapproved dirty paths.

After a successful push, either leave the final state file as a durable run record or remove it in a final housekeeping commit only if the user wants ephemeral harness state excluded from the branch. Do not delete it silently if it is the only record of the harness queue and decisions.

## Hard Stops

- `docs/FOUNDATIONS.md` wins over spec prose, ticket prose, and this harness.
- Do not bypass `implement-ticket`, `skill-audit`, or `post-ticket-review` guardrails.
- Do not commit unrelated pre-existing dirty paths unless the user explicitly approves their inclusion.
- Do not treat a blocked ticket as completed just to let the loop continue.
- Do not archive the originating spec while any active ticket still owns required work for it.
- Do not push a branch with uncommitted owned changes or unresolved blockers.
- Do not start a new non-follow-up ticket in a context that is already carrying substantial implementation, proof, review, or audit history from a previous ticket; write the handoff and reset first.
- Do not use the printed handoff as the only resume record. Persist or refresh `.codex/run-state/implement-spec-tickets.json` before asking for `/new`.
- Do not run Cargo commands in `multi_tool_use.parallel`; Worldwake Cargo proof must stay sequential.

## Final Report

End with:

- originating spec path and archived path, if archived
- tickets implemented, blocked, archived, or left active
- follow-up retargeting decisions
- skill audit suggestions applied or rejected
- commits created
- final branch and push result, if reached
- verification commands or review surfaces that proved the final state
- final state-file status
