---
name: traceability-retrospective
description: Post-implementation reflection on traceability gaps and implementation difficulties — extracts lessons, checks FOUNDATIONS alignment, and creates tickets for warranted changes
---

# Traceability Retrospective Skill

Reflects on the current session's implementation difficulties to identify traceability gaps, edge cases, and documentation shortfalls. Checks each finding against `docs/FOUNDATIONS.md`, proposes tickets for warranted changes, and creates ticket files after user approval.

**Core principle:** DO NOTHING is the default. Not every implementation difficulty warrants a ticket. The burden of proof is on ticket creation.

**Project scope:** This skill is tuned for the Worldwake project's debugging surfaces (decision traces, action traces, event log, belief state, perception pipeline, conservation checks). Adapt the reflection lenses for other projects.

## Invocation

```
/traceability-retrospective <ticket-id>
```

Example: `/traceability-retrospective E20COMBEH-007`

The argument is the ticket identifier that was just implemented. The skill resolves the full ticket filename by matching against `tickets/`.

## Prerequisites

- **Same-session only**: This skill relies on conversation context — the issues, confusions, and failures encountered during implementation. It cannot work cross-session.
- **Ticket must exist**: The referenced ticket file must be present in `tickets/` (active) or `archive/tickets/` (archived).

## Phase 0 — Context Loading

Read ALL of the following files completely:

1. **Implemented ticket**: Find `tickets/*{arg}*` or `archive/tickets/*{arg}*`. If multiple matches, list them and ask the user to disambiguate.
2. **Foundations**: `docs/FOUNDATIONS.md` — the alignment reference for all proposed changes.
3. **Ticket authoring contract**: `tickets/README.md` — conventions that proposed tickets must follow.
4. **Ticket template**: `tickets/_TEMPLATE.md` — format for any tickets created.
5. **Existing tickets**: List all files in `tickets/` to check for duplicates in Phase 2.

## Phase 1 — Reflection

Perform structured self-examination of the current conversation through these lenses. Do not prescribe what should surface — capture whatever actually did:

1. **Difficulties**: What was hard? What took multiple attempts? What required backing out and retrying?
2. **Confusions**: Where did assumptions not match reality? Where was the codebase surprising?
3. **Failures**: What broke, failed to compile, or failed tests? What was the root cause?
4. **Traceability gaps**: Where did you expect trace or debug data (decision trace, action trace, event log, belief state, perception pipeline, conservation checks) but it was unavailable, insufficient, or misleading?
5. **Edge cases**: What surprising behaviors from prior specs or systems surfaced? What interactions between systems were unexpected?
6. **Documentation gaps**: Where was documentation missing, misleading, or out of date?

**Output**: A numbered list of distinct lessons. One lesson per item — no grab bags. Each lesson gets:
- **What happened**: Factual description of the issue
- **Why it was non-obvious**: What made it hard to anticipate
- **Impact**: How it affected implementation (time lost, incorrect code written, test failures)

## Phase 1b — Ticket Drift Check (optional)

Compare the ticket's stated setup (from "What to Change" and "Assumption Reassessment") against what was actually implemented. If significant drift exists — different initial values, restructured scenario, omitted or added agents, changed tick budgets — note each divergence briefly.

If drift is found, propose updating the ticket's relevant sections so the ticket remains an accurate record of what was delivered. Present proposed updates alongside Phase 2b doc notes or Phase 3 ticket proposals for user approval.

If no significant drift exists, skip this phase silently.

## Phase 2 — FOUNDATIONS Alignment & Triage

For each lesson from Phase 1, run this evaluation:

### Step 1: Recurrence Check

Will this issue come up again in future tickets? If this was a one-off mistake (typo, wrong variable, misread a function signature), it does not warrant a ticket. **Route to DO NOTHING.**

### Step 2: Already Addressed Check

Was this issue already fixed during the implementation itself? If the conversation shows the problem was identified and resolved in-session, no ticket is needed unless the fix was incomplete or a band-aid. **Route to DO NOTHING.**

Distinguish between fixing the immediate task (test setup, configuration, wrong argument) and fixing the underlying infrastructure gap. If the issue was only resolved for *this* session's task but the same silent failure mode remains for future tasks, the lesson survives this step.

### Step 3: Duplicate Check

Is there already an existing ticket in `tickets/` that covers this issue? Grep ticket titles and problem descriptions. **Route to DO NOTHING** if covered.

### Step 4: FOUNDATIONS Alignment

Check the lesson against `docs/FOUNDATIONS.md`:
- Does it reveal a principle violation? (cite the principle number and name)
- Does it reveal a gap where a principle should apply but the infrastructure doesn't support it yet?
- Does it reveal a missing causal hook, information path, or debuggability surface? (Principle 3 — Concrete State; Principle 7 — Locality)

Lessons that do not connect to any FOUNDATIONS principle are unlikely to warrant tickets. Apply skepticism.

### Step 5: Change Classification

For lessons that survive Steps 1-4, classify the warranted change:
- **Code**: Engine or system changes (new trace points, missing validation, broken causal chains)
- **Docs**: Spec updates, CLAUDE.md additions, guide corrections
- **Testing**: Missing golden tests, missing assertions, inadequate verification surfaces
- **Multiple**: Changes spanning more than one category

### Output

For each lesson, state:
- **Bucket**: DO NOTHING (with reason) | DOC NOTE (with target file and proposed text) | TICKET WARRANTED (with change type)
- **FOUNDATIONS reference**: Principle number(s) if applicable

**DOC NOTE** is for lessons that don't warrant a ticket but would benefit from a brief addition (1-3 lines) to an existing guide — typically `docs/golden-e2e-testing.md`, `CLAUDE.md`, or a spec. If no tickets are warranted, doc notes are presented in Phase 2b. If tickets are also warranted, present doc notes alongside ticket proposals in Phase 3.

If all lessons route to DO NOTHING, **report that explicitly and stop**. This is a valid outcome — it means the implementation was clean and the infrastructure is solid. Do not manufacture tickets.

If the only non-DO-NOTHING lessons are DOC NOTEs (no tickets warranted), proceed to Phase 2b. Otherwise, proceed to Phase 3 (which handles both tickets and any accompanying doc notes).

## Phase 2b — Doc Note Presentation (no tickets)

When all surviving lessons are DOC NOTEs with no TICKET WARRANTED items:

1. Present each doc note with its target file and proposed text.
2. **Wait for explicit approval** before applying any changes.
3. Apply approved notes directly to the target files. No ticket file creation needed.
4. **Stop** — do not proceed to Phase 3 or Phase 4.

## Phase 3 — Ticket Proposal

For each lesson routed to TICKET WARRANTED, present a summary:

```
### Proposed Ticket N: <Title>

- **Problem**: <What gap was exposed, in 2-3 sentences>
- **Proposed change type**: Code | Docs | Testing | Multiple
- **FOUNDATIONS alignment**: Principle N — <name> (<why>)
- **Likely affected files**: <best-guess file list>
- **Estimated effort**: Small | Medium | Large
```

Present ALL proposed tickets to the user in a single message. Then **wait for explicit approval**. The user may:
- Approve all
- Approve some (specify which)
- Reject all
- Request modifications

**Do not proceed to Phase 4 until the user responds.**

## Phase 4 — Ticket Creation

For each approved ticket:

1. **Determine ticket ID**: Follow the naming convention visible in existing `tickets/` files. If the pattern includes a prefix derived from the parent spec or epic, use that. Otherwise ask the user.
2. **Create ticket file**: Write to `tickets/<ID>.md` using the format from `tickets/_TEMPLATE.md`.
3. **Fill required sections**:
   - Status: PENDING
   - Problem: From the Phase 3 proposal
   - Assumption Reassessment: With today's date, referencing the current codebase state discovered during the retrospective
   - Architecture Check: Why the proposed change is cleaner than alternatives
   - Verification Layers: Map invariants to proof surfaces (decision trace, action trace, event-log delta, authoritative world state)
   - What to Change: Specific changes required
   - Files to Touch: List of files
   - Acceptance Criteria: Specific tests and invariants
   - Test Plan: New/modified tests and runnable commands
4. **Apply precision rules**: Follow `docs/precision-rules.md` for all technical claims.

## Important Rules

- **DO NOTHING is the default** — if no lessons warrant tickets, say so explicitly and stop. Do not create filler tickets.
- **Never auto-create tickets** — always present proposals in Phase 3 and wait for user approval before writing files.
- **Never duplicate** — check existing tickets in `tickets/` before proposing new ones.
- **FOUNDATIONS alignment is mandatory** — every proposed ticket must cite at least one principle from `docs/FOUNDATIONS.md`.
- **Same-session only** — this skill depends on conversation context for the reflection phase. Do not attempt to reconstruct implementation difficulties from git history alone.
- **Follow ticket conventions exactly** — use `tickets/README.md` and `tickets/_TEMPLATE.md`. Apply `docs/precision-rules.md`.
- **One lesson per ticket** — do not bundle unrelated issues into a single ticket. If two lessons are genuinely coupled, explain the coupling explicitly.
- **Honest assessment** — if the implementation went smoothly and no gaps surfaced, that is a valuable signal. Report it.
