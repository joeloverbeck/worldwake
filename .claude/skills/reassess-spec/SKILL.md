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

## Plan Mode Awareness

If plan mode is active, Steps 1-6 proceed normally (all are read-only analysis). At the end of Step 6, after presenting findings and resolving all questions, write a condensed summary of approved changes to the plan file before calling ExitPlanMode. The plan file serves as the approval artifact; the conversational report provides detail. Step 7 (write the updated spec) and Step 8 (final summary) execute after the user approves the plan. When plan mode is active, the user's plan approval covers both question resolutions and the overall set of changes — there is no separate confirmation gate between resolution presentation and spec writing.

The plan file should use this structure:
- **Context**: Which spec, why it's being reassessed
- **Approved Changes**: Organized by Issues Fixed / Improvements Applied / Additions Incorporated, each with severity tag
- **Critical Files**: Paths of files to be modified by the updated spec
- **Verification**: How to confirm the updated spec is correct after writing

The conversational report (Step 6) is the decision artifact — the user approves or rejects based on it. The plan file is a condensed reference for the implementation phase (Steps 7-8).

If question resolution produces new findings or modifies existing ones (e.g., a crate boundary constraint discovered during resolution changes the recommended integration strategy), the plan file should reflect the final resolved state, not the initial Step 6 report. Sequence: present the resolution conversationally (so the user sees the reasoning), then write the plan file incorporating all resolved findings, then call ExitPlanMode.

## Process

Follow these steps in order. Do not skip any step.

**Re-reassessment shortcut**: If the same spec was reassessed earlier in this session and the spec file has not been modified by an external process since then, Steps 2-3 may be scoped to only validate references affected by the change that triggered re-reassessment (e.g., a new CLAUDE.md invariant, a codebase change, or user-requested corrections). Step 1 still applies — re-read the spec (it may have been updated by the prior reassessment). Steps 4-8 proceed normally.

### Step 1: Mandatory Reads

Read ALL of these files before any analysis:

1. **The spec file** (from the argument) — read the entire file
2. **`docs/FOUNDATIONS.md`** — architectural commandments; every spec must align with these principles. Skip if read earlier in this session and not modified since.
3. **`docs/spec-drafting-rules.md`** (if the spec contains or should contain an FND-01 Section H analysis) — defines the required format and checklist points for P30 compliance. Skip if read earlier in this session and not modified since.

Parse the spec's metadata — look for fields like Phase, Status, Priority, Crates, Dependencies, Goals/Design Goals, Non-Goals, FOUNDATIONS Alignment, and all implementation/deliverable sections. Not all specs have every field.

### Step 2: Extract References

From the spec, extract every concrete codebase reference:

- **File paths** mentioned or implied (e.g., `crates/worldwake-ai/src/candidate_generation.rs`, `crates/worldwake-systems/src/trade_actions.rs`)
- **Type names** (e.g., `GoalKind`, `SaleListing`, `HomeostaticNeeds`, `PlannerOpKind`)
- **Function names** (e.g., `generate_candidates`, `enumerate_trade_payloads`, `classify_band`)
- **Crate/module names** (e.g., `worldwake-core`, `worldwake-ai`, `worldwake-systems`, `worldwake-sim`)
- **Test file paths or test names** referenced (e.g., `golden_merchant_selling.rs`, `golden_emergent.rs`)
- **Other specs or tickets** listed in Dependencies (e.g., `S04`, `E19`, `specs/S04-merchant-selling*.md`)

Build a checklist of every reference to validate in Step 3 (internal working artifact — do not present to user). Prioritize references most likely to have drifted: dependency paths (specs may have been archived), function signatures (may have changed with recent implementations), and types the spec proposes to extend (may have new constraints). Stable types like `EntityId`, `Permille`, `Quantity` can be spot-checked rather than fully validated.

### Step 3: Codebase Validation

For every reference extracted in Step 2, validate against the actual codebase:

1. **File paths**: Glob/Grep to confirm they exist at the stated location. If a file was moved, renamed, or deleted, record the discrepancy and the actual location (if found).
2. **Types and interfaces**: Grep for each type name. Confirm it exists, check its current shape (fields, members). If the spec assumes a field that does not exist or has a different name/type, record the discrepancy. For types the spec uses in formulas or struct definitions (not just extends), verify that the assumed field types match the actual types. Pay particular attention to numeric types (`u32` vs `Permille` vs `i32`) — the spec may assume a different numeric representation than what exists.
3. **Functions and methods**: Grep for each function. Confirm signature, module location, and export status. Note any signature differences from what the spec assumes. For new functions the spec proposes, validate that the signature's parameters provide sufficient data at every intended call site. If a parameter type doesn't carry enough information (e.g., the function needs belief context but the proposed parameter is a payload-only type), flag it as an Issue. For existing functions the spec proposes to modify (adding code at specific lines), verify that the function's parameters and local scope include the variables the proposed code references. If a proposed code change uses a variable that is not in scope at that location, flag as an Issue and note what signature changes are needed to thread the variable through.
4. **Dependencies (specs/tickets)**: For each dependency, verify whether it lives in `specs/`, `archive/specs/`, `tickets/`, or `archive/tickets/`. Record the correct path. If a dependency is listed as incomplete but has since been implemented, note this.
5. **Component fields and ECS registrations**: If the spec does not propose adding fields to existing components, creating new components, or extending discriminator enums, skip sub-steps 5a-5g. Otherwise, validate component definitions and check for conflicts, following these sub-steps:
   - **5a. Shape validation**: Grep for component struct definitions in `worldwake-core`, verify field names and types match spec claims. Check `component_schema.rs` for registration.
   - **5b. Trait bounds**: For types or enums the spec proposes to extend (new variants, new fields), check the existing derive macros and trait bounds. Record any constraints that new additions must satisfy (e.g., `Copy`, `Serialize`, `Ord`).
   - **5c. Default and constructors**: For field additions to existing structs, focus on the `Default` impl and any builder/constructor functions.
   - **5d. Downstream consumers**: For field type changes or removals, perform full downstream consumer analysis (Step 3.6).
   - **5e. Scalar-to-collection migrations**: For scalar-to-collection field migrations (e.g., `EntityId` → `BTreeSet<EntityId>`), additionally grep for equality comparisons (`== field_value`, `!= field_value`) that would need to become containment checks (`.contains()`).
   - **5f. Semantic overlap**: For each field the spec proposes to add to a new or existing component, grep for semantically similar field names across all existing components (e.g., if the spec adds `switch_margin`, search for `margin`, `switch`, `commitment` across other profile types). Record any semantic overlaps and trace the runtime interaction between the overlapping fields. Also check for functional overlap — fields on a new component that serve the same purpose as fields on existing components, even if the names differ. Flag these as potential P28 migration candidates. For new components introducing a novel domain concept (e.g., a new contention substrate), semantic overlap checks focus on functional overlap with existing components rather than field name similarity. For components extending an existing domain, field name similarity checks remain important. **Novel-domain test**: A component introduces a novel domain if no existing component serves the same downstream consequence (P5). If the new component's primary effect could be achieved by extending an existing component, it is extending an existing domain, and field name similarity checks apply.
   - **5g. EntityKind variant overlap**: For new enum variants on discriminator enums like `EntityKind`, check whether existing variants overlap semantically with the proposed addition. Empty or unused variants that would fragment the same domain (e.g., separate entity kinds for things that should share a common substrate) should be flagged as P28 migration candidates.
6. **Downstream consumers**: For types or interfaces the spec proposes to modify, grep for all import sites and usage points. Record the blast radius — files that would need updating.
7. **Crate boundary validation**: For new functions or methods the spec proposes, verify that the parameter types and return types are accessible from the crate where the function would live. Check `Cargo.toml` dependencies. If a proposed function in crate A takes a type from crate B, and A does not depend on B, flag this as an Issue and note which crate the function must actually live in. This is especially important for the workspace layering (`core → sim → systems → ai → cli`) — a function in `worldwake-core` cannot take parameters typed in `worldwake-sim`.
8. **Impact scan — upstream spec references**: Grep active specs in `specs/` for references to the target spec's deliverables (type names, component names, interfaces it introduces). Note any active specs that would be affected by proposed changes. This step can be delegated to an Explore agent alongside Steps 3.1-3.6 validation — include "grep active specs in specs/ for references to [list proposed type/component names]" in the agent prompt.

For specs with many references (>10), consider launching parallel Explore agents organized by theme (e.g., one for action/type references, one for AI/test references, one for dependencies and infrastructure). This is more efficient than sequential validation. Choose themes based on the spec's scope — the split should minimize cross-agent dependencies. Examples: one for action/type references, one for AI/test references, one for dependencies and infrastructure. After agent results arrive, cross-reference their findings against the spec's type assumptions and formulas. Agents validate existence; you must validate semantic compatibility (e.g., the spec says Permille but the codebase uses u32). For specs that propose static lookup tables indexed by a dispatch key or discriminator enum, verify that the key's granularity matches the lookup's discrimination needs. If the spec assumes per-payload-variant behavior but the key collapses payload variants into a single entry, flag as an Issue. Spot-check agent claims about existence/registration with direct Grep or Read before including them in findings. Agent results are approximate — treat them as leads, not facts. In plan mode, Explore agents are inherently compatible (read-only exploration) — no special handling needed.

Do not present findings yet. Collect everything for Step 4.

### Step 4: FOUNDATIONS.md Alignment Check

Review each section of the spec against `docs/FOUNDATIONS.md`:

0. Before checking FOUNDATIONS alignment, scan the spec for **internal contradictions** between its Design Goals, Non-Goals, FOUNDATIONS Alignment table, and Deliverables. Flag any case where one section's claims conflict with another (e.g., a Design Goal promising backward compatibility while the FOUNDATIONS table cites P28 No Backward Compat).
1. If the spec has a FOUNDATIONS.md Alignment table, verify each claimed alignment is accurate. For each entry, verify the principle number matches the principle name in `docs/FOUNDATIONS.md` — misnumbered principles are a common error (e.g., citing P20 for "Agent Diversity" when the correct number is P22). Flag misnumbered principles as Issues. Flag any principle the spec claims to satisfy but actually violates.
2. Identify any Foundation principle the spec does **not** address but should, given its scope. Pay particular attention to:
   - **Principle 1** (Maximal Emergence) — does the spec introduce authored sequences or magic triggers?
   - **Principle 7** (Locality) — does the spec have agents querying global state on behalf of a character?
   - **Principle 14** (World State Is Not Belief State) — does the spec let agents read authoritative world state directly?
   - **Principle 26** (Systems Interact Through State) — does the spec introduce cross-system direct calls instead of state-mediated interaction?
   - **Principle 28** (No Backward Compatibility) — does the spec leave compatibility shims or defer migration?
   - **Principle 30** (Causal Hooks Declaration) — does the spec declare its causal hooks per the required checklist in `docs/FOUNDATIONS.md` P30? Count the items from the source document each time — do not rely on a cached count, as the list may evolve. If the spec includes a Section H, verify it addresses all applicable P30 checklist items. Not all items apply to every spec — flag only genuinely missing items, not items correctly omitted as N/A. P30's full 18-item checklist applies to new system specs. Bugfix, lifecycle, or architecture-fix specs need only the subset of Section H analyses relevant to their scope (typically: information-path, positive-feedback, stored state). If the spec already includes these and introduces no new systems, P30 compliance is met.
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

Tag each finding by severity: CRITICAL (blocks ticket decomposition), HIGH (should fix before tickets), MEDIUM (improves quality), LOW (nice to fix). This helps users prioritize when the finding list is long.

### Step 6: Present Findings

Present all findings to the user in a structured report:

```
## Reassessment: <spec-name>

### Issues (must fix)
[If none: "No issues found."]
1. **[SEVERITY] <title>** — <what the spec says> vs. <what the codebase has>. Recommendation: <change>.

### Improvements (should fix)
[If none: "No improvements found."]
1. **[SEVERITY] <title>** — <current spec text> could be improved because <reason>. Recommendation: <change>.

### Additions (consider adding)
[If none: "No additions proposed."]
1. **[SEVERITY] <title>** — <what's missing> would be beneficial because <reason>. Recommendation: <new section or deliverable>.

### FOUNDATIONS.md Alignment
- <Foundation N>: <aligned | see Issue #N [SEVERITY]>

### Authoritative-to-AI Impact Rule
[If Step 4.4 was triggered (spec modifies action preconditions, validate_* functions, affordance generation, or can_exercise_control): list the 7 checklist points with pass / N-A / flag — <brief explanation> status. If not triggered: omit this section.]

### Questions
[If none: "No questions."]
1. <question>
```

**Question discipline**: Ask at most 3 questions in this initial report. If you have more than 3, prioritize the ones that block further reassessment and defer the rest to a follow-up round after the user responds. If two questions are interdependent (the answer to one constrains the other), present them as a single combined question with clearly labeled option combinations, rather than asking sequentially and potentially invalidating the first answer. When a question has 2-4 discrete options, use `AskUserQuestion` with labeled options and a recommended default. When a question is open-ended, present it as plain text in the report.

**Wait for user response.** Do not proceed to Step 7 until the user has answered all questions. Treat findings as approved unless the user explicitly objects or modifies them. If the user's response addresses only questions and does not mention specific findings, implicit approval is assumed.

If the user delegates question resolution (e.g., "you decide based on FOUNDATIONS," "reassess and determine the best choice"), resolve each question by reasoning against the referenced constraint (typically `docs/FOUNDATIONS.md`). If resolution requires additional codebase investigation (e.g., verifying a crate boundary constraint that wasn't checked in Step 3), perform the investigation and incorporate findings before presenting the resolution — this is a mini Step 3 scoped to the question at hand. If reasoning reveals that none of the original options are ideal, propose a new option with justification. Scope any additional investigation to the minimum codebase checks needed to confirm or reject the novel option — typically 1-3 targeted greps or reads. If the investigation reveals the novel option requires changes to the spec's dependency graph or crate boundaries, present this as a new finding before resolving. Present the resolution (including any novel option) with justification for each question and wait for user confirmation before proceeding to Step 7. In plan mode, the novel option is included in the plan file and ExitPlanMode approval covers it — no separate confirmation round is needed, consistent with the Plan Mode Awareness section.

If the user's answers raise new questions or invalidate previous findings, present a follow-up round (same format, same question limit). Repeat until all findings are resolved.

### Step 7: Write the Updated Spec

After all findings are resolved and the user has approved the changes:

If the user's plan approval (in plan mode) or question responses include corrections or additional feedback, incorporate them before writing. The ExitPlanMode result may contain user comments — treat these as binding modifications to the approved changes.

**Write the updated spec** incorporating all approved changes. Preserve the spec's existing structure and voice. Do not rewrite sections that have no findings — change only what was agreed upon. When changes are numerous and spread throughout the spec, a full Write is acceptable. The intent is to avoid gratuitous rewrites of prose that has no findings — not to mandate Edit over Write as the tool choice.

If the reassessment adds new deliverable sections that introduce actions, components, or system functions, verify that the FND-01 Section H is updated to cover them (P30 compliance extends to additions, not just the original spec content).

If the user requests corrections after reviewing, apply them and re-present the affected sections.

### Step 8: Final Summary

After writing the updated spec, present:

- Number of issues fixed, improvements applied, and additions incorporated
- Change inventory: enumerate all changes applied, grouped by finding type (issues fixed, improvements applied, additions incorporated) to mirror the Step 6 report structure
- Any deferred items the user chose not to address now
- Items excluded by reassessment-driven scope changes (distinct from user-deferred items) — note why
- 1-3 sections that changed most substantially, with a note to review them before proceeding
- Suggested next step: `/spec-to-tickets <spec-path> <NAMESPACE>` to decompose into tickets

Do NOT commit. Leave the file for user review.

## Guardrails

- **FOUNDATIONS alignment is mandatory**: Every change to the spec must respect `docs/FOUNDATIONS.md`. Never approve a spec change that violates a Foundation principle, even if the user requests it — flag the conflict instead.
- **Codebase truth**: All references in the updated spec must be validated against the actual codebase. Never propagate stale file paths, renamed types, or removed functions.
- **One question at a time in follow-ups**: After the initial report (which may have up to 3 questions), follow-up rounds ask one question at a time to avoid overwhelming the user.
- **YAGNI ruthlessly**: Additions must be natural extensions of the spec's scope. Do not propose features that "might be nice" but are not aligned with the spec's stated goals.
- **No scope creep**: The deliverable is the updated spec file. Do not write design docs, create tickets, or start implementation.
- **No approach proposals**: This is reassessment, not greenfield design. Do not propose 2-3 alternative architectures. The spec already has a design — validate and refine it. Exception: when the spec's proposed approach violates a crate boundary, FOUNDATIONS principle, or critical invariant, propose the minimum number of viable alternatives needed for the user to make an informed choice. Present these as part of the Issue finding, not as a separate design exercise.
- **Preserve spec voice**: When editing, match the spec's existing writing style. Do not rewrite unchanged sections for stylistic preferences.
- **Worktree discipline**: If working in a worktree, ALL file operations use the worktree root path.
