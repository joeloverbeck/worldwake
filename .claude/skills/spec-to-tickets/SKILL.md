---
name: spec-to-tickets
description: Break a spec into actionable, detailed tickets aligned with FOUNDATIONS.md. Use when asked to decompose a spec into tickets.
---

# Spec to Tickets

Break a numbered spec into a series of small, actionable implementation tickets.

## Invocation

```
/spec-to-tickets <spec-path> <NAMESPACE>
```

**Arguments** (both required, positional):
- `<spec-path>` — path to the spec file (e.g., `specs/99-event-card-policy-surface.md`)
- `<NAMESPACE>` — ticket namespace prefix (e.g., `99EVECARPOLSUR`)

If either argument is missing, ask the user to provide it before proceeding.

## Process

Follow these steps in order. Do not skip any step.

### Step 1: Mandatory Reads

Read ALL of these files before any analysis:

1. **The spec file** (from argument 1) — read the entire file
2. **`tickets/_TEMPLATE.md`** — the canonical ticket structure; every ticket you produce must follow this template exactly
3. **`tickets/README.md`** — the ticket authoring contract; understand the required sections and checks
4. **`docs/FOUNDATIONS.md`** — architectural commandments; every ticket must align with these principles

### Step 2: Codebase Validation

Before decomposing, validate the spec's assumptions against the actual codebase:

- **Grep/Glob** for file paths mentioned in the spec — confirm they exist
- **Grep** for types, functions, and modules the spec references — confirm they are real and current
- **Flag** any stale assumptions, missing files, or renamed entities
- If you find discrepancies, present them to the user before proceeding

### Step 3: Decompose the Spec

Analyze the spec and identify discrete work units:

- Each ticket must represent a **reviewable diff** — small enough for comfortable manual review
- Map **dependencies** between tickets (which must be done before which)
- Determine **priority ordering** (what to implement first)
- Ensure **every spec deliverable is covered** — no silent skipping. If a deliverable seems wrong or unnecessary, flag it to the user using the 1-3-1 rule instead of omitting it
- Consider natural boundaries: type changes, new modules, test suites, integration points

### Step 4: Present Summary for Approval

**Before writing any ticket files**, present a numbered summary table:

```
| # | Ticket ID | Title | Effort | Deps |
|---|-----------|-------|--------|------|
| 1 | <NS>-001  | ...   | Small  | None |
| 2 | <NS>-002  | ...   | Medium | 001  |
| ...
```

Include a 1-line description of each ticket's scope.

**Wait for user approval or adjustments.** Do not write files until the user confirms.

### Step 5: Write Ticket Files

For each approved ticket, write a file to `tickets/<NAMESPACE>-<NNN>.md` using the **exact structure** from `tickets/_TEMPLATE.md`.

Every ticket MUST include:

- **Status**: PENDING
- **Priority**: HIGH / MEDIUM / LOW (based on dependency order and criticality)
- **Effort**: Small / Medium / Large
- **Engine Changes**: None or list of affected areas
- **Deps**: Other tickets or specs this depends on
- **Problem**: What user-facing or architecture problem this solves
- **Assumption Reassessment**: Assumptions validated against current code (use today's date)
- **Architecture Check**: Why this approach is clean, how it preserves agnostic boundaries
- **What to Change**: Numbered sections with specific implementation details
- **Files to Touch**: Exact paths validated against the codebase (new or modify)
- **Out of Scope**: Explicit non-goals — what this ticket must NOT change
- **Acceptance Criteria**:
  - **Tests That Must Pass**: Specific behavior tests
  - **Invariants**: Must-always-hold architectural and data contract invariants
- **Test Plan**:
  - **New/Modified Tests**: Paths with rationale
  - **Commands**: Targeted test commands and full suite verification

### Step 6: Final Summary

After writing all files, list:
- All ticket files created
- The dependency graph (which tickets block which)
- Suggested implementation order

Do NOT commit. Leave files for user review.

## Constraints

- **FOUNDATIONS alignment**: Every ticket must respect the principles in `docs/FOUNDATIONS.md` (engine agnosticism, evolution-first, visual separation, etc.)
- **Template fidelity**: Every ticket must use the `tickets/_TEMPLATE.md` structure exactly — no ad-hoc sections or missing required fields
- **Ticket fidelity**: Never silently skip a spec deliverable. If something seems wrong, use the 1-3-1 rule (1 problem, 3 options, 1 recommendation) and ask the user
- **Codebase truth**: File paths and type references in tickets must be validated against the actual codebase, not assumed from the spec
- **Reviewable size**: Each ticket should be small enough to review as a single diff. When in doubt, split further
- **Explicit dependencies**: Use the `Deps` field to declare inter-ticket dependencies; never leave implicit ordering
