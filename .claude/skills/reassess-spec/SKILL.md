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

## Invocation

```
/reassess-spec <spec-path>
```

**Arguments** (required, positional):
- `<spec-path>` — path to the spec file (e.g., `specs/S05-merchant-stock-storage-and-stalls.md`)

If the argument is missing, ask the user to provide it before proceeding.

## Worktree Awareness

If working inside a worktree (e.g., `.claude/worktrees/<name>/`), ALL file paths — reads, writes, globs, greps — must use the worktree root as the base path.

## Plan Mode Awareness

If plan mode is active, load `references/plan-mode.md`.

## Process

Follow these steps in order. Do not skip any step.

### Pre-Process: Spec Classification

Before beginning Steps 2-3, classify the spec:

- **(a) New system** — introduces new components, actions, goal kinds, or information paths. Full checklist applies.
- **(b) System extension** — extends existing components, actions, or enums without new systems. Steps 3.1-3.8, 4.4 apply. Skip 3.9 if no behavioral claims about runtime readers/writers. Section H updates only for new deliverable sections. For tooling-only specs, downstream consumer analysis (3.6) can be limited to the tooling binary.
- **(c) Structural refactor** — trait/module restructuring with no behavioral changes. Skip Steps 3.5, 3.9, 4.4; Section H is N/A. Focus on symbol existence, count accuracy, and blast radius.
- **(d) Test-only** — adds golden tests, benchmarks, or test infrastructure without modifying production code.
  - Steps 3.1-3.4 apply (validate referenced paths, types, functions, dependencies).
  - Skip 3.5-3.9 (no production code changes to trace).
  - Step 4 applies but 4.4 is N/A.
  - Section H updates are N/A unless the test reveals a missing causal hook.

- **(e) Investigation/bugfix** — proposes root cause hypotheses with conditional fixes, no new systems or components.
  - Steps 3.1-3.4 apply (validate all referenced paths, types, functions, dependencies).
  - Steps 3.5-3.8 apply only to proposed fix deliverables (not to hypothesis text).
  - Step 3.9 applies if claims about runtime behavior are made.
  - Step 4 applies; 4.4 applies if any proposed fix touches action preconditions.
  - Section H updates only if the fix changes causal hooks.

**Hybrid specs**: Apply the union of applicable steps — use the most rigorous classification's checklist for shared steps.

**Re-reassessment shortcut**: If the same spec was reassessed earlier in this session and not externally modified, Steps 2-3 may scope to only references affected by the triggering change. Step 1 still applies.

**Self-authored spec note**: Full validation is required even for specs authored earlier in this session — authoring may introduce unchecked assumptions.

### Step 1: Mandatory Reads

Read ALL of these before any analysis:

1. **The spec file** (from the argument) — entire file
2. **`docs/FOUNDATIONS.md`** — skip if read earlier in this session and unmodified
3. **`docs/spec-drafting-rules.md`** (if the spec contains or should contain Section H) — skip if read earlier and unmodified

Parse the spec's metadata: Phase, Status, Priority, Crates, Dependencies, Goals/Design Goals, Non-Goals, FOUNDATIONS Alignment, and all deliverable sections.

### Step 2: Extract References

Extract every concrete codebase reference from the spec:

- **File paths** mentioned or implied
- **Type names** (e.g., `GoalKind`, `SaleListing`, `PlannerOpKind`)
- **Function names** (e.g., `generate_candidates`, `enumerate_trade_payloads`)
- **Crate/module names**
- **Test file paths or test names**
- **Other specs or tickets** in Dependencies
- **Code examples** (inline code blocks showing API usage, precondition lists, struct definitions) — extract for fidelity checking against actual source

Build a validation checklist (internal). Prioritize references most likely to have drifted: dependency paths, function signatures, and types the spec extends. Stable types (`EntityId`, `Permille`, `Quantity`) can be spot-checked.

### Step 3: Codebase Validation

Load `references/codebase-validation.md`. Validate every reference from Step 2.

Do not present findings yet. Collect everything for Step 4.

### Step 4: FOUNDATIONS.md Alignment Check

Load `references/foundations-alignment.md`. Check spec alignment against all applicable principles.

### Steps 5-6: Classify and Present Findings

Load `references/findings-and-questions.md`. Classify all findings from Steps 3-4, then present to the user.

Wait for user response before proceeding to Step 7. (In plan mode, this wait is replaced by ExitPlanMode approval.)

### Step 7: Write the Updated Spec

Load `references/spec-writing-rules.md`. Apply all approved changes.

### Step 8: Final Summary

Present:

- Number of issues fixed, improvements applied, additions incorporated
- Change inventory: all changes grouped by finding type (mirroring Step 6 structure)
- Post-Apply Confirmation results (e.g., "Verified: zero matches for eliminated references, N matches for corrected references")
- Deferred items the user chose not to address
- Items excluded by reassessment-driven scope changes (distinct from user-deferred) — note why. Omit if none.
- 1-3 sections that changed most substantially, with a note to review before proceeding
- Suggested next step: `/spec-to-tickets <spec-path> <NAMESPACE>`

Do NOT commit. Leave the file for user review.

## Guardrails

- **FOUNDATIONS alignment is mandatory**: Never approve a spec change that violates a Foundation principle, even if requested — flag the conflict instead.
- **Codebase truth**: All references in the updated spec must be validated. Never propagate stale paths, renamed types, or removed functions.
- **No scope creep**: The deliverable is the updated spec file. Do not write design docs, create tickets, or start implementation.
- **No approach proposals**: Validate and refine the existing design, not greenfield alternatives. Exception: when the approach violates a crate boundary, FOUNDATIONS principle, or critical invariant, propose minimum viable alternatives as part of the Issue finding.
- **Substantial redesign flag**: If reassessment changes >50% of deliverables' approach, flag in Step 6: "This reassessment proposes substantial redesign of N/M deliverables. Goals preserved but implementation path changes significantly."
