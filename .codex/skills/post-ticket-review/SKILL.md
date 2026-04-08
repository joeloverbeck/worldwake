---
name: post-ticket-review
description: "Review a just-implemented Worldwake ticket after coding is finished. Use when a ticket is complete and you want Codex to archive it if appropriate, assess the implemented work against `docs/FOUNDATIONS.md`, review nearby architecture/test/traceability quality, and create or update follow-up tickets in `tickets/` when warranted."
---

# Post-Ticket Review

Post-implementation review and follow-up planning. Archives the completed ticket, inspects the work for architectural follow-up, and creates tickets when warranted.

Read [AGENTS.md](../../../AGENTS.md), [docs/FOUNDATIONS.md](../../../docs/FOUNDATIONS.md), [tickets/README.md](../../../tickets/README.md), and [tickets/_TEMPLATE.md](../../../tickets/_TEMPLATE.md) before making changes.

**Allowed actions**: update the completed ticket's `Outcome` and verification notes (factual, unambiguous only), archive it, create new tickets in `tickets/`, update existing active tickets.

**Forbidden**: modifying production code or tests.

## Workflow

### 1. Resolve the target ticket

1. Use the provided ticket name if supplied.
2. If the just-finished ticket was already archived this session, use that archived ticket.
3. Otherwise, search active tickets for the most recently touched candidate.
4. Confirm the implementation state is present locally (committed or not).
5. If the ticket lives under `.claude/worktrees/<name>/`, treat that worktree root as the repository root for all operations.
6. If the target ticket cannot be identified confidently, stop and ask.

### 2. Check archival readiness

1. Read the target ticket and confirm its current status.
2. Confirm `Outcome` and verification notes accurately describe the implemented work.
3. Fix factual, unambiguous handoff issues directly: missing/incomplete `Outcome`, inaccurate verification notes, archival mechanics per [docs/archival-workflow.md](../../../docs/archival-workflow.md).
4. If unresolved in-scope deliverables remain, stop and report archival as blocked — implementation must resume first.
   - This includes stale source-golden headers, generated scenario docs, or owned proof-surface prose that no longer matches the implemented contract. Treat as incomplete handoff, not a separate follow-up ticket.
5. If already archived, validate the archived handoff content rather than reopening.
6. Do not revise the ticket's problem statement, scope, or acceptance criteria except for factual completion notes required by archival mechanics.
7. If archival readiness is ambiguous, apply the 1-3-1 rule.
8. Archive if ready. If a prior review pass blocked archival, treat this pass as the authoritative handoff step only after remaining in-scope implementation has landed.

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
| Concern fully covered by an active ticket | Cite that ticket in the report; do not duplicate |
| Active ticket partially covers the concern | Update that ticket factually to capture it accurately |
| No active ticket covers the concern | Create a new follow-up ticket |

#### Active-ticket maintenance checks

Run these checks before archival to keep the active roadmap accurate:

- **Delivered work overlap**: When the completed ticket materially resolves assumptions owned by nearby active tickets, update those tickets to remove already-delivered work, stale failure claims, or obsolete roadmap ownership.
- **Substrate-only slices**: When the completed ticket landed only a shared type surface, reserved enum variant, or other non-live substrate, check nearby active tickets for confusion between "the symbol now exists" and "the behavior is now live." Cite if accurate; update if not.
- **Scope narrowing**: When the completed ticket was corrected or narrowed during implementation, check whether nearby active tickets still assume the older broader boundary. If the remaining slice is real and no active ticket owns it, create a follow-up and update adjacent `Deps`.
- **Active spec drift**: When the completed ticket falsifies or narrows a claim in an active spec under `specs/`, classify that as active spec drift. Update the spec factually if in scope for this handoff; otherwise create/update a follow-up ticket that owns bringing the spec into alignment.
- **Dependency chain impact**: When a new follow-up ticket changes architectural ordering or prerequisites, also check adjacent active tickets in the same subsystem sequence and update their scope or `Deps` factually.

### 6. Author follow-up tickets

When a new ticket is warranted:
1. Create from [tickets/_TEMPLATE.md](../../../tickets/_TEMPLATE.md) per [tickets/README.md](../../../tickets/README.md).
2. Reassess against current code and docs before finalizing.
3. Name exact files, symbols, abstraction boundaries, invariants, and proof surfaces.
4. Keep bounded to one coherent concern.

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
- Only change the completed ticket's `Outcome`, verification notes, and archival mechanics.
- Every finding must be backed by concrete code, test, trace, ticket, or documentation evidence.
- Reject any follow-up suggestion that would violate [docs/FOUNDATIONS.md](../../../docs/FOUNDATIONS.md).
- Use 1-3-1 when archival readiness, ticket decomposition, or dependency ordering is genuinely ambiguous.
