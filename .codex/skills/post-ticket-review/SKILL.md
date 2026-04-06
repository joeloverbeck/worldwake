---
name: post-ticket-review
description: "Review a just-implemented Worldwake ticket after coding is finished. Use when a ticket is complete and you want Codex to archive it if appropriate, assess the implemented work against `docs/FOUNDATIONS.md`, review nearby architecture/test/traceability quality, and create or update follow-up tickets in `tickets/` when warranted."
---

# Post-Ticket Review

Use this skill after implementation is finished for a ticket. The goal is to close the completed ticket cleanly, then inspect the implemented work for architectural follow-up work without muddying the completed change.

Read [AGENTS.md](../../../AGENTS.md), [docs/FOUNDATIONS.md](../../../docs/FOUNDATIONS.md), [tickets/README.md](../../../tickets/README.md), and [tickets/_TEMPLATE.md](../../../tickets/_TEMPLATE.md) before making changes.

This skill may:
- update the completed ticket's `Outcome` and verification notes when the correction is factual and unambiguous
- archive the completed ticket
- create new tickets in `tickets/`
- update existing active tickets in `tickets/` when that is better than opening a duplicate

This skill must not modify production code or tests. It is a post-implementation review and follow-up planning workflow.

## Workflow

### 1. Resolve the target ticket

1. Use the provided ticket name if one was supplied.
2. If the just-finished ticket was already archived in the current session, use that archived ticket directly.
3. Otherwise, search active tickets for the most recently touched candidate and use that ticket.
4. Confirm the implementation state for that ticket is present locally, whether committed or not.
5. If the ticket lives under `.claude/worktrees/<name>/`, treat that worktree root as the repository root for all reads, writes, searches, and archival actions.
6. If the target ticket cannot be identified confidently, stop and ask for clarification.

### 2. Check archival readiness

1. Read the target ticket at its current path and confirm its current status.
2. Confirm the ticket's `Outcome` section and verification notes accurately describe the implemented work and the verification already performed.
3. Fix factual, unambiguous ticket handoff issues directly:
   - missing or incomplete `Outcome`
   - inaccurate verification notes
   - archival mechanics required by [docs/archival-workflow.md](../../../docs/archival-workflow.md)
4. If the ticket still has unresolved in-scope deliverables relative to its current text, stop and report archival as blocked. Do not write completion handoff text yet; implementation must resume first.
   - This includes stale source-golden headers, generated scenario docs, or other owned proof-surface prose when they no longer match the implemented contract. Treat that as incomplete handoff work, not as a separate follow-up ticket by default.
5. If the ticket is still active and the scoped work is complete, apply the archival checks above before moving it to the archive.
6. If the ticket is already archived, validate the archived handoff content rather than reopening or moving it again.
7. Do not revise the completed ticket's problem statement, scope, or acceptance criteria except where archival mechanics require factual completion notes.
8. If archival readiness is ambiguous, use the 1-3-1 rule from [AGENTS.md](../../../AGENTS.md) before proceeding.
9. If the ticket is ready and still active, archive it.
10. If an earlier post-ticket review pass blocked archival, treat a later pass as the authoritative handoff step only after the remaining in-scope implementation has landed.

### 3. Establish the review surface

Review the actual local implementation state, not an idealized committed state.

Start with:
- the completed ticket
- the files changed for that work
- directly relevant tests, traces, and docs

Broaden the review when the implementation touches a known boundary or contract:
- shared abstractions across crates
- planner or authoritative-validation boundaries
- information-path contracts
- traceability surfaces
- test harness or golden proof surfaces

The review covers:
- production architecture
- test architecture
- traceability/debuggability
- ticket and documentation handoff quality
- active downstream ticket roadmap drift when the completed work changed what remains to be done

### 4. Audit against FOUNDATIONS and architecture quality

Assess whether the completed work:
- aligns with [docs/FOUNDATIONS.md](../../../docs/FOUNDATIONS.md)
- preserves repo invariants from [AGENTS.md](../../../AGENTS.md)
- leaves the touched subsystem clean, robust, and extensible

Look for:
- direct `FOUNDATIONS` contradictions
- architectural contradictions the completed ticket exposed but did not resolve
- duplicated or competing information paths
- weak abstraction boundaries
- fragile test architecture or proof surfaces
- missing traceability needed to debug the canonical path
- cleanup or design debt near the touched subsystem that the implementation surfaced

Separate findings into:
- problems solved by the completed ticket
- residual concerns left behind
- new concerns exposed by the implementation

### 5. Decide whether follow-up work is warranted

Create or update follow-up tickets when findings are backed by concrete evidence and materially improve:
- `FOUNDATIONS` alignment
- cleanliness
- robustness
- extensibility
- test architecture
- traceability

Do not create tickets for vague stylistic preferences or speculative cleanup.

When a concern is already covered by an active ticket in [tickets/](../../../tickets):
- if the existing ticket fully covers it, cite that ticket in the report and do not duplicate it
- if the existing ticket should be expanded or clarified to capture the concern accurately, update that ticket factually

When the completed ticket materially resolves assumptions owned by nearby active tickets, update those tickets factually to remove already-delivered work, stale failure claims, or obsolete roadmap ownership.

When a completed ticket in a staged chain lands only a shared type surface, reserved enum variant, or other non-live substrate slice, explicitly check nearby active tickets for confusion between "the symbol now exists" and "the behavior is now live." If those tickets still accurately reserve the first live behavior for a later slice, cite them as covered; if not, update them factually before archival so the active roadmap does not imply already-live behavior.

When a completed ticket was corrected or narrowed during implementation, explicitly check whether nearby active tickets still assume the older broader boundary. If the remaining slice is still real and no active ticket cleanly owns it, create a new follow-up ticket and update adjacent `Deps` factually before archival so the roadmap still matches the implemented end-to-end activation path.

When a completed ticket falsifies or materially narrows a claim still present in an active spec under `specs/`, explicitly classify that as active spec drift during the review. If reconciling the spec is already in scope for the current handoff, update it factually. Otherwise, create or update the follow-up ticket that now owns bringing the active spec back into alignment with the live evidence, and name that ownership clearly in the report.

Prefer small, distinct tickets split by architectural concern.

When a new follow-up ticket changes architectural ordering or prerequisites, also check adjacent active tickets in the same subsystem sequence and update their scope or `Deps` factually if needed.

If ticket splitting or dependency structure is genuinely unclear, use the 1-3-1 rule before creating or editing tickets.

### 6. Author follow-up tickets

When a new ticket is warranted:
1. Create it from [tickets/_TEMPLATE.md](../../../tickets/_TEMPLATE.md).
2. Follow the contract in [tickets/README.md](../../../tickets/README.md).
3. Reassess the new ticket against current code and docs before finalizing its text.
4. Name exact files, symbols, abstraction boundaries, invariants, and proof surfaces.
5. Keep the ticket bounded to one coherent concern.

Set fields using evidence, not placeholders:
- `Priority`: infer from impact and blast radius
- `Effort`: infer from likely implementation scope
- `Engine Changes`: reflect whether production architecture must change
- `Deps`: include both strict blockers and strong sequencing dependencies

Dependencies may point to:
- newly created tickets from this review
- existing active tickets in `tickets/`

Create high-confidence tickets directly. Ask before creating a ticket only when the scope or dependency graph is uncertain.

### 7. Present the report

Return a structured report in the conversation with these sections:

```markdown
# Post-Ticket Review: <ticket-id>

**Ticket**: <path>
**Review date**: YYYY-MM-DD
**Implementation state reviewed**: <worktree/index/committed summary>

## Archival Status

- <archived / already archived / blocked>
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
- **Adjacent roadmap still valid**: <nearby active tickets that remain relevant context but were not updated and did not directly absorb a newly discovered concern>

## 1-3-1 Decisions

- <only include when used>

## Summary

**Result**: <ticket archived / archival blocked / already archived>
**Follow-up**: <N new tickets, N updated tickets, N covered by existing tickets>
```

If no follow-up tickets are warranted, still report the reviewed areas and state that no new ticket was needed.

## Guardrails

- Do not modify production code or tests in this workflow.
- Review the local implementation state as it exists now, committed or not.
- Archive the completed ticket when it is ready; do not leave it active just because follow-up work exists.
- Only change the completed ticket's `Outcome`, verification notes, and archival mechanics.
- Every finding must be backed by concrete code, test, trace, ticket, or documentation evidence.
- Reject any follow-up suggestion that would violate [docs/FOUNDATIONS.md](../../../docs/FOUNDATIONS.md).
- Prefer updating an existing active ticket over opening a duplicate when the concern is already substantially covered there.
- Keep follow-up tickets small and architecture-focused.
- Use 1-3-1 when archival readiness, ticket decomposition, or dependency ordering is genuinely ambiguous.
