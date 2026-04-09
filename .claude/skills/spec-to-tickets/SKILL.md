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
- `<spec-path>` — path to the spec file (e.g., `specs/S05-merchant-stock-storage-and-stalls.md`)
- `<NAMESPACE>` — ticket namespace prefix (e.g., `S05MERSTOSTALL`)

If either argument is missing, ask the user to provide it before proceeding.

## Process

Follow these steps in order. Do not skip any step.

### Step 1: Mandatory Reads

Read ALL of these files before any analysis:

1. **The spec file** (from argument 1) — read the entire file
2. **`tickets/_TEMPLATE.md`** — the canonical ticket structure; every ticket you produce must follow this template exactly
3. **`tickets/README.md`** — the ticket authoring contract; understand the required sections and checks
4. **`docs/FOUNDATIONS.md`** — architectural commandments; every ticket must align with these principles. Skip if read earlier in this session and not modified since.
5. **`docs/precision-rules.md`** — precision rules for technical claims; governs Assumption Reassessment and Verification Layers sections. Skip if read earlier in this session and not modified since.

### Step 2: Codebase Validation

Before decomposing, validate the spec's assumptions against the actual codebase:

- **Grep/Glob** for file paths mentioned in the spec — confirm they exist
- **Grep** for types, functions, and modules the spec references — confirm they are real and current
- **Flag** any stale assumptions, missing files, or renamed entities
- If you find discrepancies, present them to the user before proceeding
- If `/reassess-spec` was run on this spec in the current session and all findings were resolved, Step 2 validation may be abbreviated to a spot-check of key references rather than a full re-validation. An abbreviated spot-check should verify at least: (a) the spec's primary type/function references still exist at the stated paths, (b) if the spec modifies serialized state or save/load paths, `SAVE_FORMAT_VERSION` hasn't changed since reassessment (skip this check for specs that don't touch serialization), (c) no new files in `specs/` reference the same types, and (d) for specs that add fields to existing structs, grep for struct literal construction sites (e.g., `StructName {`) to verify all construction sites are accounted for in the tickets' Files to Touch. If the construction site count is high (>20), note the count in the Step 4 summary table's Effort column justification and consider whether the field-addition ticket should be split by crate. A high construction site count typically elevates effort from Small to Medium. 3-5 targeted greps is sufficient.
- If `/reassess-spec` was run but some findings were deferred by the user, treat deferred items as out-of-scope for ticket decomposition. Note them in the Step 6 final summary as "deferred reassessment findings that may warrant separate tickets." Do not silently incorporate deferred findings into ticket scope.

### Step 3: Decompose the Spec

Analyze the spec and identify discrete work units:

- Each ticket must represent a **reviewable diff** — small enough for comfortable manual review
- Map **dependencies** between tickets (which must be done before which)
- Determine **priority ordering** (what to implement first)
- Ensure **every spec deliverable is covered** — no silent skipping. If a deliverable seems wrong or unnecessary, flag it to the user using the 1-3-1 rule instead of omitting it. Deliverables that explicitly state no changes are needed (e.g., "No new profile", "No new components") do not require tickets. Note their existence in the Step 4 summary if non-obvious
- Consider natural boundaries: type changes, new modules, test suites, integration points
- Use the spec's "What This Does NOT Change" or equivalent non-goals section to populate tickets' Out of Scope fields — these are pre-validated non-goals from reassessment
- Ensure **workspace builds after each ticket** — if removing types/functions from a shared crate, all consumers must be updated in the same ticket. Splitting a migration across tickets is only valid when intermediate states compile.
- For **mechanical refactoring specs** (trait decomposition, enum splitting, interface migrations), recognize that multiple tickets may share an identical file set. In the summary table (Step 4), note the shared file set once and reference it from each ticket rather than forcing each ticket to independently discover the same list. Individual tickets should list only *additional* files beyond the shared set.

### Step 4: Present Summary for Approval

**Before writing any ticket files**, present a numbered summary table:

```
| # | Ticket ID | Title | Scope | Effort | Deps | FND |
|---|-----------|-------|-------|--------|------|-----|
| 1 | <NS>-001  | ...   | <5-10 word scope> | Small  | None | — |
| 2 | <NS>-002  | ...   | <5-10 word scope> | Medium | 001  | P12,P27 |
| ...
```

Include a 1-line description of each ticket's scope. The FND column is optional — populate it only for tickets with notable FOUNDATIONS concerns (e.g., a ticket touching derived views should note P27). Use `—` for tickets with no specific concern.

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
- **Assumption Reassessment**: Assumptions validated against current code (use today's date). Include items 1-3 from the template (always required) plus any domain-specific items from items 4-15 that match the ticket's scope. Omit inapplicable items silently — do not pad with "N/A" boilerplate. For pure structural refactoring tickets (no behavioral changes, no new actions/components), items 1-3 may be satisfied concisely by confirming: (a) the symbols being moved exist at stated locations, (b) the impl block count matches the spec's claim, (c) the shared boundary is the trait/struct under edit. Items 4-15 are typically all inapplicable for structural refactors
- **Architecture Check**: Why this approach is clean, how it preserves agnostic boundaries
- **Verification Layers**: Map each invariant to its proof surface (for mixed-layer or cross-system tickets: decision trace, action trace, event-log delta, authoritative world state)
- **What to Change**: Numbered sections with specific implementation details
- **Files to Touch**: Exact paths validated against the codebase (new or modify). When a ticket adds fields to an existing struct, grep for struct literal construction sites across the workspace during ticket writing and include all affected files. See `tickets/README.md` check #13 for known macro expansion sites (`delta.rs`, `world.rs`, `component_tables.rs`)
- **Out of Scope**: Explicit non-goals — what this ticket must NOT change
- **Acceptance Criteria**:
  - **Tests That Must Pass**: Specific behavior tests
  - **Invariants**: Must-always-hold architectural and data contract invariants
- **Test Plan**:
  - **New/Modified Tests**: Paths with rationale
  - **Commands**: Targeted test commands and full suite verification

For decompositions producing 5+ tickets, batch ticket file writes in parallel where tickets are independent. Group by similarity (e.g., all extraction tickets in one batch, infrastructure tickets in another).

### Step 6: Final Summary

After writing all files, list:
- All ticket files created
- The dependency graph (which tickets block which)
- Suggested implementation order

Do NOT commit. Leave files for user review.

## Constraints

- **FOUNDATIONS alignment**: Every ticket must respect the principles in `docs/FOUNDATIONS.md` (maximal emergence, belief-only planning, system decoupling, no backward compatibility, etc.)
- **Template fidelity**: Every ticket must use the `tickets/_TEMPLATE.md` structure exactly — no ad-hoc sections or missing required fields
- **Ticket fidelity**: Never silently skip a spec deliverable. If something seems wrong, use the 1-3-1 rule (1 problem, 3 options, 1 recommendation) and ask the user
- **Codebase truth**: File paths and type references in tickets must be validated against the actual codebase, not assumed from the spec
- **Reviewable size**: Each ticket should be small enough to review as a single diff. When in doubt, split further
- **Explicit dependencies**: Use the `Deps` field to declare inter-ticket dependencies; never leave implicit ordering
- **Performance-optimization specs**: For P12-type specs that compress computation without changing world meaning, tickets should include benchmark acceptance criteria with concrete metric thresholds (e.g., "per-agent-tick planning cost does not exceed X ms at tick 10,080") and regression guard commands (e.g., soak seed runs). Acceptance criteria should distinguish correctness (golden tests pass) from performance (metric thresholds met)
