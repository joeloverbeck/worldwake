---
name: spec-to-tickets
description: "Break a Worldwake spec into small, reviewable implementation tickets aligned with `AGENTS.md`, `docs/FOUNDATIONS.md`, `tickets/README.md`, and `tickets/_TEMPLATE.md`. Use when you want Codex to decompose a spec into repository-ready ticket files."
---

# Spec To Tickets

Use this skill when a Worldwake spec is ready to be decomposed into actionable implementation tickets.

The goal is not to restate the spec. The goal is to turn the spec into a bounded sequence of reviewable diffs that match the live codebase, the ticket template, and the repository's architectural rules.

Read [AGENTS.md](../../../AGENTS.md), [docs/FOUNDATIONS.md](../../../docs/FOUNDATIONS.md), [tickets/README.md](../../../tickets/README.md), [tickets/_TEMPLATE.md](../../../tickets/_TEMPLATE.md), and [docs/precision-rules.md](../../../docs/precision-rules.md) before writing any tickets.

## Preconditions

- The target spec must exist in `specs/` or `archive/specs/`, unless the user explicitly provides another valid path.
- The user must provide enough information to identify:
  - the spec to decompose
  - the ticket namespace prefix to use
- If working inside `.claude/worktrees/<name>/`, treat that worktree root as the repository root for all reads, writes, searches, and archival actions.

## Workflow

### 1. Resolve the target and load context

1. Resolve the target spec path from the user's request.
2. Resolve the ticket namespace prefix.
3. If either is missing or ambiguous, stop and ask for clarification.
4. Read the spec completely.
5. Read:
   - [AGENTS.md](../../../AGENTS.md)
   - [docs/FOUNDATIONS.md](../../../docs/FOUNDATIONS.md)
   - [tickets/README.md](../../../tickets/README.md)
   - [tickets/_TEMPLATE.md](../../../tickets/_TEMPLATE.md)
   - [docs/precision-rules.md](../../../docs/precision-rules.md)
6. If the spec references active or archived tickets, specs, golden docs, or planner contracts that materially affect decomposition, read the specific referenced files rather than inferring from memory.

### 2. Validate the spec against the live codebase

Before decomposing work, verify the spec still matches the repository as it exists now.

Check:
- referenced files and directories
- referenced crates, modules, types, functions, traits, and tests
- spec dependencies and whether they are active or archived
- claimed architectural boundaries and proof surfaces
- whether the stated coverage gap is still real

For non-trivial claims, name exact symbols and files rather than paraphrasing a subsystem.

If the spec was already reassessed earlier in the current session and all findings were resolved, this validation may be abbreviated to a focused spot-check of the references most likely to drift. Do not assume prior reassessment happened unless the current session evidence is clear.

### 3. Surface mismatches before decomposition

If the spec and the live codebase disagree in a way that changes ticket scope, stop and report it before writing tickets.

For each mismatch, state:
- what the spec says
- what the live codebase has
- whether the decomposition should:
  - correct the spec assumptions first
  - adapt the ticket plan to the live code
  - stop because the scope is blocked

Use the 1-3-1 rule from [AGENTS.md](../../../AGENTS.md) when the right decomposition boundary or sequencing is genuinely unclear or risky.

Do not silently omit spec deliverables.

### 4. Decompose the work into ticket-sized units

Turn the validated spec into a sequence of bounded tickets.

Each ticket must:
- represent a reviewable diff
- own one coherent concern
- have explicit dependencies
- cover concrete deliverables from the spec
- stay honest about proof surfaces and architectural boundaries

Prefer boundaries such as:
- shared type or schema changes
- planner or affordance-surface changes
- action/runtime lifecycle changes
- system integration slices
- test or golden coverage slices
- traceability or documentation follow-up owned by the spec

Do not bundle unrelated concerns just because they appear in the same section of the spec.

If one stated deliverable is too large for a reviewable ticket, split it further and make the dependency chain explicit.

### 5. Reassess each candidate ticket before presenting it

Before presenting the decomposition, make sure each proposed ticket can honestly satisfy the ticket contract.

For each candidate ticket, determine:
- ticket id using `<NAMESPACE>-<NNN>`
- title
- problem it solves
- likely priority
- likely effort
- whether `Engine Changes` is `None` or a concrete list of affected areas
- exact dependencies
- likely proof surfaces for `Verification Layers`
- likely files to touch, validated against the current repo

Apply [tickets/README.md](../../../tickets/README.md) and [docs/precision-rules.md](../../../docs/precision-rules.md) now, not after the files are written.

If a candidate ticket would require hand-wavy file references, vague proof surfaces, or unclear architecture ownership, refine or split it before presenting the plan.

### 6. Present the decomposition for approval

Before writing any ticket files, present a numbered decomposition summary in the conversation and wait for approval.

Include a compact table in this shape:

```markdown
| # | Ticket ID | Title | Effort | Deps |
|---|-----------|-------|--------|------|
| 1 | <NS>-001  | ...   | Small  | None |
| 2 | <NS>-002  | ...   | Medium | 001  |
```

Then include one short scope line per ticket explaining what that ticket owns.

Also include:
- any mismatches already resolved during decomposition
- any unresolved decision that still needs user direction
- the proposed implementation order

Do not write files yet.

### 7. Write approved ticket files

After the user approves the decomposition, write each ticket to:

`tickets/<NAMESPACE>-<NNN>.md`

Every ticket must follow [tickets/_TEMPLATE.md](../../../tickets/_TEMPLATE.md) exactly and satisfy [tickets/README.md](../../../tickets/README.md).

Every written ticket must include, with evidence-backed content rather than placeholders:
- `Status`: `PENDING`
- `Priority`
- `Effort`
- `Engine Changes`
- `Deps`
- `Problem`
- `Assumption Reassessment (<YYYY-MM-DD>)`
- `Architecture Check`
- `Verification Layers`
- `What to Change`
- `Files to Touch`
- `Out of Scope`
- `Acceptance Criteria`
- `Test Plan`

When writing tickets:
- apply [docs/precision-rules.md](../../../docs/precision-rules.md) to all technical claims
- name exact files, symbols, abstractions, and proof surfaces
- keep acceptance criteria aligned to the ticket's actual owned boundary
- keep test commands real and copy-paste runnable

Do not create filler tickets. If approval reveals that a proposed ticket is unnecessary, merge or remove it explicitly rather than preserving the original count.

### 8. Present the final handoff

After writing the approved tickets, summarize:
- all ticket files created
- the dependency graph
- the suggested implementation order
- any deferred or intentionally omitted work the user approved

Do not implement the tickets. Do not archive anything. Do not commit.

## Report Format

Use this structure for the approval step:

```markdown
# Spec To Tickets: <spec-id>

**Spec**: <path>
**Namespace**: <NAMESPACE>
**Decomposition date**: YYYY-MM-DD

## Validation Notes

- <validated references, resolved mismatches, or "no blocking mismatches found">

## Proposed Tickets

| # | Ticket ID | Title | Effort | Deps |
|---|-----------|-------|--------|------|
| 1 | <NS>-001  | ...   | Small  | None |

1. **<ticket-id>** — <one-line owned scope>

## Proposed Order

- <ordered ticket ids>

## Open Decisions

- <only when applicable>
```

Use this structure after writing files:

```markdown
# Spec To Tickets Complete: <spec-id>

**Spec**: <path>
**Namespace**: <NAMESPACE>
**Ticket files created**:
- `tickets/<ID>.md`

## Dependency Graph

- `<ID>` -> `<ID>`

## Suggested Implementation Order

- `<ID>`

## Notes

- <deferred or approved omissions, if any>
```

## Guardrails

- Never silently skip an explicit spec deliverable. If it seems wrong, blocked, or oversized, surface that explicitly.
- Validate ticket content against the live codebase, not against stale spec prose.
- Keep tickets small enough to review as single diffs whenever possible.
- Use explicit `Deps` fields. Never rely on implied ordering.
- Follow [AGENTS.md](../../../AGENTS.md), especially ticket fidelity, worktree discipline, TDD expectations for bug-fix tickets, and the 1-3-1 rule when needed.
- Follow [docs/FOUNDATIONS.md](../../../docs/FOUNDATIONS.md). Reject decompositions that would require shims, magic-number design, omniscient planning, or direct system-to-system coupling.
- Follow [tickets/README.md](../../../tickets/README.md) and [docs/precision-rules.md](../../../docs/precision-rules.md). Precision is part of the deliverable, not post-processing.
- Do not write ticket files until the user approves the decomposition summary.
- Do not commit or implement as part of this workflow.
