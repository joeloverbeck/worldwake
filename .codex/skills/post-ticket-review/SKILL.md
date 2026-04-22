---
name: post-ticket-review
description: "Review a just-implemented Worldwake ticket after coding is finished. Use when a ticket is complete and you want Codex to archive it if appropriate, assess the implemented work against `docs/FOUNDATIONS.md`, review nearby architecture/test/traceability quality, and create or update follow-up tickets in `tickets/` when warranted."
---

# Post-Ticket Review

Post-implementation review and follow-up planning. Archives the completed ticket, inspects the work for architectural follow-up, and creates tickets when warranted.

Read [AGENTS.md](../../../AGENTS.md), [docs/FOUNDATIONS.md](../../../docs/FOUNDATIONS.md), [tickets/README.md](../../../tickets/README.md), and [tickets/_TEMPLATE.md](../../../tickets/_TEMPLATE.md) before making changes.

**Allowed actions**: update the completed ticket's `Outcome` and verification notes (factual, unambiguous only), apply mechanical path/link rewrites caused solely by moving the completed ticket into `archive/`, archive it, create new tickets in `tickets/`, update existing active tickets, factually update active specs under `specs/` when the completed ticket's landed contract makes the drift unambiguous.

**Forbidden**: modifying production code or tests.

## Workflow

### 1. Resolve the target ticket

1. Use the provided ticket name if supplied.
2. If the just-finished ticket was already archived this session, use that archived ticket.
3. Before attempting any archive move, check whether the worktree is already in the manual-archive fallback state for this ticket: `D tickets/<id>.md` plus `?? archive/tickets/<id>.md`, with the active path absent and the archive path present. If so, treat the ticket as already manually archived in the current worktree and use the archived path directly.
4. Otherwise, search active tickets for the most recently touched candidate.
5. Confirm the implementation state is present locally (committed or not).
6. Record whether the completed ticket is tracked or untracked in the current worktree so the report can describe archival state accurately.
7. If the ticket lives under `.claude/worktrees/<name>/`, treat that worktree root as the repository root for all operations.
8. If the target ticket cannot be identified confidently, stop and ask.

### 2. Check archival readiness

1. Read the target ticket and confirm its current status.
2. Confirm `Outcome` and verification notes accurately describe the implemented work.
   - Compare the ticket's completion notes against the full landed local touched-file set, not just the original planned file list. Include factual compile/lint fallout or local test-fixture updates in `Outcome` when they were part of the real implementation handoff.
3. Fix factual, unambiguous handoff issues directly: missing/incomplete `Outcome`, inaccurate verification notes, archival mechanics per [docs/archival-workflow.md](../../../docs/archival-workflow.md).
   - If the completed ticket is untracked, do not rely on ordinary `git diff` output for ticket-body validation. Read the live ticket file directly before archival, then confirm archival state with `git status` plus a direct existence check on the original path after the move.
   - If a repo-local archival helper you would normally use is missing in the current checkout, fall back to the manual move-and-verify workflow in [docs/archival-workflow.md](../../../docs/archival-workflow.md) and mention that fallback explicitly in the report.
   - If the active path is already absent while `git status` shows `D tickets/<id>.md` and `?? archive/tickets/<id>.md`, do not attempt a second move. Treat that as an already-completed manual fallback in the current worktree, validate the archived file's contents, and report the exact transition state explicitly.
   - For active tickets, compare the live ticket's `Problem`, `What to Change`, `Acceptance Criteria`, `Invariants`, and `Test Plan` against the landed diff before deciding archival readiness. If any of those sections still overclaim the result or describe a stronger end state than the code actually landed, treat that as an unresolved in-scope handoff failure and block archival.
4. If unresolved in-scope deliverables remain, stop and report archival as blocked — implementation must resume first.
   - This includes stale source-golden headers, generated scenario docs, or owned proof-surface prose that no longer matches the implemented contract. Treat as incomplete handoff, not a separate follow-up ticket.
   - For golden tickets, also check generated-doc spillover explicitly: confirm the regenerated golden inventory/docs touched the expected owning scenario surfaces, and note any broader generated churn that needs explanation or follow-up.
   - When a previously completed ticket was left unarchived until a later reconciliation/disposition ticket clarified its live contract, re-read the older ticket's `Outcome`, `Verification Result`, and any remaining blocker wording against that newer archived disposition before archiving the older ticket.
5. If already archived, validate the archived handoff content rather than reopening.
6. Do not revise the ticket's problem statement, scope, or acceptance criteria except for factual completion notes required by archival mechanics.
7. If archival readiness is ambiguous, apply the 1-3-1 rule.
8. Archive if ready. If a prior review pass blocked archival, treat this pass as the authoritative handoff step only after remaining in-scope implementation has landed.
9. After moving the ticket, inspect `git status` for both the archived path and the original active path. Record which post-archive state occurred, and mention it explicitly in the report:
   - tracked rename / move into the archive path
   - tracked deletion at the active path plus untracked archive file
10. Re-read the archived ticket after the move and fix any purely mechanical path fallout caused by relocation (for example relative markdown links in `Deps`, archive-ticket references, or other moved-path links inside the archived file). Treat these edits as archival mechanics, not scope changes.

### 3. Establish the review surface

Review the actual local implementation state, not an idealized committed state.

**Starting surface**:
- The completed ticket, changed files, directly relevant tests/traces/docs

**Broaden when the implementation touches a known boundary**:
- Shared abstractions across crates
- Planner or authoritative-validation boundaries
- Information-path contracts
- Traceability surfaces
- Test harness or golden proof surfaces

**Review dimensions**:
- Production architecture
- Test architecture
- Traceability/debuggability
- Ticket and documentation handoff quality
- Active downstream ticket roadmap drift

### 4. Audit against FOUNDATIONS and architecture quality

Assess whether the completed work:
- Aligns with [docs/FOUNDATIONS.md](../../../docs/FOUNDATIONS.md)
- Preserves repo invariants from [AGENTS.md](../../../AGENTS.md)
- Leaves the touched subsystem clean, robust, and extensible

Look for:
- Direct `FOUNDATIONS` contradictions
- Architectural contradictions the ticket exposed but did not resolve
- Duplicated or competing information paths
- Weak abstraction boundaries
- Fragile test architecture or proof surfaces
- Missing traceability for the canonical path
- Cleanup or design debt surfaced by the implementation

Separate findings into: problems solved, residual concerns, newly exposed concerns.

### 5. Decide whether follow-up work is warranted

Create or update follow-up tickets only when findings are backed by concrete evidence and materially improve FOUNDATIONS alignment, cleanliness, robustness, extensibility, test architecture, or traceability. Do not create tickets for vague stylistic preferences or speculative cleanup.

Prefer small, distinct tickets split by architectural concern. Use the 1-3-1 rule when ticket splitting or dependency structure is genuinely unclear.

#### Create vs. update vs. cite decision

| Situation | Action |
|-----------|--------|
| Concern is still an unmet deliverable of the reviewed active ticket | Do not create a new ticket; report archival blocked and return ownership to the current ticket |
| Concern fully covered by an active ticket | Cite that ticket in the report; do not duplicate |
| Active ticket partially covers the concern | Update that ticket factually to capture it accurately |
| No active ticket covers the concern | Create a new follow-up ticket |

#### Active-ticket maintenance checks

Run these checks before archival to keep the active roadmap accurate:

- **Delivered work overlap**: When the completed ticket materially resolves assumptions owned by nearby active tickets, update those tickets to remove already-delivered work, stale failure claims, or obsolete roadmap ownership.
- **Substrate-only slices**: When the completed ticket landed only a shared type surface, reserved enum variant, or other non-live substrate, check nearby active tickets for confusion between "the symbol now exists" and "the behavior is now live." Cite if accurate; update if not.
- **Sibling handoff consistency**: When one active sibling already assumes another active sibling owns the next producer/consumer wiring step, but that supposed owner still describes the work as deferred or out of scope, reconcile the ownership mismatch before archival. Update the owning active ticket when the next-step handoff is factual and unambiguous.
- **Scope narrowing**: When the completed ticket was corrected or narrowed during implementation, check whether nearby active tickets still assume the older broader boundary. If the remaining slice is real and no active ticket owns it, create a follow-up and update adjacent `Deps`.
- **Same-session follow-ups**: When implementation created a new follow-up ticket earlier in the same session and the current review archives its prerequisite, re-check that new ticket's `Deps` after archival so it points at the archived ticket path rather than the just-deleted active path.
- **Active spec drift**: When the completed ticket falsifies or narrows a claim in an active spec under `specs/`, classify that as active spec drift. This includes narrowed implementations where the draft spec still describes the broader pre-reassessment plan. Update the spec factually if in scope for this handoff; otherwise create/update a follow-up ticket that owns bringing the spec into alignment.
- **Dependency chain impact**: When a new follow-up ticket changes architectural ordering or prerequisites, also check adjacent active tickets in the same subsystem sequence and update their scope or `Deps` factually.
- **Broader verification blockers**: When the completed ticket's broader verification surfaced a failure outside the ticket's owned surface, rerun the failing proof in isolation before deciding action. If the failure is real and still outside scope, first check nearby active tickets/specs for an existing owner, then record it in the archived handoff and create or update a bounded follow-up ticket instead of folding it silently into the completed ticket.

### 6. Author follow-up tickets

When a new ticket is warranted:
1. Create from [tickets/_TEMPLATE.md](../../../tickets/_TEMPLATE.md) per [tickets/README.md](../../../tickets/README.md).
2. Before drafting a new ticket, inspect adjacent active tickets in the same family plus nearby active specs to confirm the concern is not already owned. If a sibling already covers the exact remainder, cite or factually update it instead of duplicating the ticket.
3. If the new ticket is being exposed by a narrowed implementation or roadmap gap, inspect adjacent active tickets/specs first so the new ticket and any dependency updates are authored together.
4. Reassess against current code and docs before finalizing.
5. Name exact files, symbols, abstraction boundaries, invariants, and proof surfaces.
6. Keep bounded to one coherent concern.

Set fields using evidence, not placeholders:
- `Priority`: infer from impact and blast radius
- `Effort`: infer from likely implementation scope
- `Engine Changes`: reflect whether production architecture must change
- `Deps`: include strict blockers and strong sequencing dependencies (may point to newly created or existing active tickets)

Create high-confidence tickets directly. Ask before creating only when scope or dependency graph is uncertain.

### 7. Present the report

```markdown
# Post-Ticket Review: <ticket-id>

**Ticket**: <path>
**Review date**: YYYY-MM-DD
**Implementation state reviewed**: <worktree/index/committed summary, including tracked vs untracked ticket state when relevant; if manual archival was already in progress, name the exact `D tickets/...` + `?? archive/tickets/...` pattern>

## Archival Status

- <archived / already archived / blocked>
- <if manual fallback was already in progress, say so explicitly: e.g. "archived via manual fallback; worktree currently shows tracked deletion at the active path plus untracked archive file">
- <Outcome + verification notes check result>
- <validated unchanged / factually corrected>
- <any ticket updates made before archival>

## What This Ticket Solved

- <completed concerns>

## FOUNDATIONS Alignment

- <aligned summary or numbered findings with principle references>

## Architecture Findings

### Residual Concerns

1. **[SEVERITY]** <title>
   - **Evidence**: <code/test/ticket/doc evidence>
   - **Why it matters**: <cleanliness / robustness / extensibility impact>
   - **Recommended follow-up**: <ticket id or planned action>

### Newly Exposed Concerns

1. **[SEVERITY]** <title>
   - **Evidence**: <code/test/ticket/doc evidence>
   - **Why it matters**: <architectural impact>
   - **Recommended follow-up**: <ticket id or planned action>

## Test And Traceability Handoff

1. **[SEVERITY]** <title>
   - **Evidence**: <test/traceability/handoff evidence>
   - **Gap**: <what remains weak>
   - **Follow-up**: <ticket id or no ticket needed>

## Ticket Actions

- **Created**: <ticket ids with one-line rationale and deps>
- **Updated**: <ticket ids with one-line rationale>
- **Covered by existing tickets**: <ticket ids and why no new ticket was created>
- **Adjacent roadmap still valid**: <nearby active tickets that remain relevant context but were not updated>

## 1-3-1 Decisions

- <only include when used>

## Summary

**Result**: <ticket archived / archival blocked / already archived>
**Follow-up**: <N new tickets, N updated tickets, N covered by existing tickets>
```

If no follow-up tickets are warranted, still report reviewed areas and state that no new ticket was needed.

## Guardrails

- Do not modify production code or tests.
- Review local implementation state as it exists now, committed or not.
- Only change the completed ticket's `Outcome`, verification notes, and archival mechanics, including move-induced path rewrites inside the archived ticket.
- Every finding must be backed by concrete code, test, trace, ticket, or documentation evidence.
- Reject any follow-up suggestion that would violate [docs/FOUNDATIONS.md](../../../docs/FOUNDATIONS.md).
- Use 1-3-1 when archival readiness, ticket decomposition, or dependency ordering is genuinely ambiguous.
