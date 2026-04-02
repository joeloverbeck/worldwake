---
name: reassess-spec
description: "Reassess a spec against the codebase and FOUNDATIONS.md. Validates assumptions, identifies issues/improvements/additions, asks clarifying questions, then writes the updated spec. Use when preparing a spec for ticket decomposition."
user-invocable: true
arguments:
  - name: spec_path
    description: "Path to the spec file (e.g., specs/S05-merchant-stock-storage-and-stalls.md)"
    required: true
---

# Reassess Spec

Validate a spec's proposed implementation against the actual codebase and FOUNDATIONS.md. Identify issues, improvements, and beneficial additions. Deliver an updated spec ready for ticket decomposition.

## Invocation

```
/reassess-spec <spec-path>
```

**Arguments** (required, positional):
- `<spec-path>` — path to the spec file (e.g., `specs/S05-merchant-stock-storage-and-stalls.md`)

If the argument is missing, ask the user to provide it before proceeding.

## Worktree Awareness

If working inside a worktree (e.g., `.claude/worktrees/<name>/`), **all file paths in this skill** — reads, writes, globs, greps — must be prefixed with the worktree root. The default working directory is the main repo root; paths without an explicit worktree prefix will silently operate on main, not the worktree. This applies to every path reference below.

## Process

Follow these steps in order. Do not skip any step.

### Step 1: Mandatory Reads

Read ALL of these files before any analysis:

1. **The spec file** (from the argument) — read the entire file
2. **`docs/FOUNDATIONS.md`** — architectural commandments; every spec must align with these principles
3. **`docs/spec-drafting-rules.md`** (if the spec contains or should contain an FND-01 Section H analysis) — defines the required format and checklist points for P30 compliance

Parse the spec's metadata: Status, Priority, Dependencies, Goals, Non-Goals, FOUNDATIONS.md Alignment table (if present), and all implementation sections.

### Step 2: Extract References

From the spec, extract every concrete codebase reference:

- **File paths** mentioned or implied (e.g., `crates/worldwake-ai/src/candidate_generation.rs`, `crates/worldwake-systems/src/trade_actions.rs`)
- **Type names** (e.g., `GoalKind`, `SaleListing`, `HomeostaticNeeds`, `PlannerOpKind`)
- **Function names** (e.g., `generate_candidates`, `enumerate_trade_payloads`, `classify_band`)
- **Crate/module names** (e.g., `worldwake-core`, `worldwake-ai`, `worldwake-systems`, `worldwake-sim`)
- **Test file paths or test names** referenced (e.g., `golden_merchant_selling.rs`, `golden_emergent.rs`)
- **Other specs or tickets** listed in Dependencies (e.g., `S04`, `E19`, `specs/S04-merchant-selling*.md`)

Build a checklist of every reference to validate in Step 3. Prioritize references most likely to have drifted: dependency paths (specs may have been archived), function signatures (may have changed with recent implementations), and types the spec proposes to extend (may have new constraints). Stable types like `EntityId`, `Permille`, `Quantity` can be spot-checked rather than fully validated.

### Step 3: Codebase Validation

For every reference extracted in Step 2, validate against the actual codebase:

1. **File paths**: Glob/Grep to confirm they exist at the stated location. If a file was moved, renamed, or deleted, record the discrepancy and the actual location (if found).
2. **Types and interfaces**: Grep for each type name. Confirm it exists, check its current shape (fields, members). If the spec assumes a field that does not exist or has a different name/type, record the discrepancy.
3. **Functions and methods**: Grep for each function. Confirm signature, module location, and export status. Note any signature differences from what the spec assumes.
4. **Dependencies (specs/tickets)**: For each dependency, verify whether it lives in `specs/`, `archive/specs/`, `tickets/`, or `archive/tickets/`. Record the correct path. If a dependency is listed as incomplete but has since been implemented, note this.
5. **Component fields and ECS registrations**: Grep for component struct definitions in `worldwake-core`, verify field names and types match spec claims. Check `component_schema.rs` for registration. For types or enums the spec proposes to extend (new variants, new fields), check the existing derive macros and trait bounds. Record any constraints that new additions must satisfy (e.g., `Copy`, `Serialize`, `Ord`).
6. **Downstream consumers**: For types or interfaces the spec proposes to modify, grep for all import sites and usage points. Record the blast radius — files that would need updating.
7. **Upstream spec references**: Grep active specs in `specs/` for references to the target spec's deliverables (type names, component names, interfaces it introduces). Note any active specs that would be affected by proposed changes.

For specs with many references (>10), consider launching parallel Explore agents organized by theme (e.g., one for action/type references, one for AI/test references, one for dependencies and infrastructure). This is more efficient than sequential validation.

Do not present findings yet. Collect everything for Step 4.

### Step 4: FOUNDATIONS.md Alignment Check

Review each section of the spec against `docs/FOUNDATIONS.md`:

1. If the spec has a FOUNDATIONS.md Alignment table, verify each claimed alignment is accurate. For each entry, verify the principle number matches the principle name in `docs/FOUNDATIONS.md` — misnumbered principles are a common error (e.g., citing P20 for "Agent Diversity" when the correct number is P22). Flag misnumbered principles as Issues. Flag any principle the spec claims to satisfy but actually violates.
2. Identify any Foundation principle the spec does **not** address but should, given its scope. Pay particular attention to:
   - **Principle 1** (Maximal Emergence) — does the spec introduce authored sequences or magic triggers?
   - **Principle 7** (Locality) — does the spec have agents querying global state on behalf of a character?
   - **Principle 14** (World State Is Not Belief State) — does the spec let agents read authoritative world state directly?
   - **Principle 26** (Systems Interact Through State) — does the spec introduce cross-system direct calls instead of state-mediated interaction?
   - **Principle 28** (No Backward Compatibility) — does the spec leave compatibility shims or defer migration?
   - **Principle 30** (Causal Hooks Declaration) — does the spec declare its causal hooks per the required checklist (14 unique points; items 14 and 15 in FOUNDATIONS.md are identical)?
3. Record each alignment issue with the specific Foundation number and what conflicts.
4. If the spec modifies action preconditions, `validate_*` functions, affordance generation (`enumerate_*_payloads`), or `can_exercise_control`, verify compliance with the Authoritative-to-AI Impact Rule checklist in CLAUDE.md. Check all 7 points: `get_affordances`, `generate_candidates`, `search_plan`, `BestEffort` action start, `handle_plan_failure`, payload revalidation (`with_payload_override_validator`), and golden test pass.

### Step 5: Classify Findings

Organize all findings from Steps 3 and 4 into three categories:

- **Issues**: Something in the spec is factually wrong, stale, or violates FOUNDATIONS.md. The spec cannot go to tickets without fixing this.
- **Improvements**: The spec is not wrong, but a refinement would make the implementation cleaner, safer, or more aligned with existing patterns.
- **Additions**: A feature or deliverable not in the spec that would be beneficial and aligns with the spec's stated goals. Apply YAGNI ruthlessly — only propose additions that are natural extensions of the spec's scope, not tangential features.

For each finding, record:
- What the spec says (or omits)
- What the codebase actually has (with file paths and line references)
- The recommended change to the spec

Optionally tag findings by severity: CRITICAL (blocks ticket decomposition), HIGH (should fix before tickets), MEDIUM (improves quality), LOW (nice to fix). This helps users prioritize when the finding list is long.

### Step 6: Present Findings

Present all findings to the user in a structured report:

```
## Reassessment: <spec-name>

### Issues (must fix)
[If none: "No issues found."]
1. **<title>** — <what the spec says> vs. <what the codebase has>. Recommendation: <change>.

### Improvements (should fix)
[If none: "No improvements found."]
1. **<title>** — <current spec text> could be improved because <reason>. Recommendation: <change>.

### Additions (consider adding)
[If none: "No additions proposed."]
1. **<title>** — <what's missing> would be beneficial because <reason>. Recommendation: <new section or deliverable>.

### FOUNDATIONS.md Alignment
- <Foundation N>: <aligned | issue description>

### Questions
[If none: "No questions."]
1. <question>
```

**Question discipline**: Ask at most 3 questions in this initial report. If you have more than 3, prioritize the ones that block further reassessment and defer the rest to a follow-up round after the user responds. If two questions are interdependent (the answer to one constrains the other), present them as a single combined question with clearly labeled option combinations, rather than asking sequentially and potentially invalidating the first answer.

**Wait for user response.** Do not proceed to Step 7 until the user has:
- Approved, rejected, or modified each finding
- Answered all questions

If the user delegates question resolution (e.g., "you decide based on FOUNDATIONS," "reassess and determine the best choice"), resolve each question by reasoning against the referenced constraint (typically `docs/FOUNDATIONS.md`). Present the resolution with justification for each question and wait for user confirmation before proceeding to Step 7.

If the user's answers raise new questions or invalidate previous findings, present a follow-up round (same format, same question limit). Repeat until all findings are resolved.

### Step 7: Write the Updated Spec

After all findings are resolved and the user has approved the changes:

**Write the updated spec** incorporating all approved changes. Preserve the spec's existing structure and voice. Do not rewrite sections that have no findings — change only what was agreed upon.

If the user requests corrections after reviewing, apply them and re-present the affected sections.

### Step 8: Final Summary

After writing the updated spec, present:

- Number of issues fixed, improvements applied, and additions incorporated
- Per-section change list: which sections changed and what each change was
- Any deferred items the user chose not to address now
- 1-3 sections that changed most substantially, with a note to review them before proceeding
- Suggested next step: `/spec-to-tickets <spec-path> <NAMESPACE>` to decompose into tickets

Do NOT commit. Leave the file for user review.

## Guardrails

- **FOUNDATIONS alignment is mandatory**: Every change to the spec must respect `docs/FOUNDATIONS.md`. Never approve a spec change that violates a Foundation principle, even if the user requests it — flag the conflict instead.
- **Codebase truth**: All references in the updated spec must be validated against the actual codebase. Never propagate stale file paths, renamed types, or removed functions.
- **One question at a time in follow-ups**: After the initial report (which may have up to 3 questions), follow-up rounds ask one question at a time to avoid overwhelming the user.
- **YAGNI ruthlessly**: Additions must be natural extensions of the spec's scope. Do not propose features that "might be nice" but are not aligned with the spec's stated goals.
- **No scope creep**: The deliverable is the updated spec file. Do not write design docs, create tickets, or start implementation.
- **No approach proposals**: This is reassessment, not greenfield design. Do not propose 2-3 alternative architectures. The spec already has a design — validate and refine it.
- **Preserve spec voice**: When editing, match the spec's existing writing style. Do not rewrite unchanged sections for stylistic preferences.
- **Worktree discipline**: If working in a worktree, ALL file operations use the worktree root path.
