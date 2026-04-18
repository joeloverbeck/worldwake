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
- **(b) System extension** — extends existing components, actions, or enums without new systems. Steps 3.1-3.8 apply. 4.4 applies if any deliverable modifies action preconditions, validation functions, affordance generation, or candidate emission. Skip 3.9 if no behavioral claims about runtime readers/writers. Section H updates only for new deliverable sections.
  - **Tooling-only variant** (observer binary, CLI enhancements, diagnostic tools): Steps 3.1-3.4 fully apply. Steps 3.5-3.7 apply only if the spec extends existing types or enums; if the spec adds only new functions/structs local to the tooling binary, 3.5-3.7 are N/A. Step 3.8 (upstream spec references) still applies. Skip 3.9. Downstream consumer analysis (3.6) can be limited to the tooling binary.
- **(c) Structural refactor** — trait/module restructuring with no behavioral changes. Skip Steps 3.5, 3.9, 4.4; Section H is N/A. Focus on symbol existence, count accuracy, and blast radius.
- **(d) Test-only** — adds golden tests, benchmarks, or test infrastructure without modifying production code.
  - Steps 3.1-3.4 apply (validate referenced paths, types, functions, dependencies).
  - Skip 3.5-3.9 (no production code changes to trace).
  - Step 4 applies but 4.4 is N/A.
  - Section H updates are N/A unless the test reveals a missing causal hook.

- **(e) Investigation/bugfix/optimization** — diagnoses a root cause and proposes targeted fixes, no new systems or components. Covers both hypothesis-driven investigations (conditional fixes) and proven-diagnosis specs (single concrete fix confirmed by existing tests). Also covers computation-optimization specs that add new planner-internal algorithms, heuristics, or filter/pruning logic without new world-facing state, as well as storage-layer performance fixes (deduplication, indexing, amortization) that change how data is stored or iterated without altering what the data means.
  - Steps 3.1-3.4 apply (validate all referenced paths, types, functions, dependencies).
  - Steps 3.5-3.8: For investigation/bugfix specs, apply only to proposed fix deliverables (not to hypothesis text). For computation-optimization specs, apply to all deliverables (there is no "hypothesis text" — all deliverables are implementation targets).
  - Step 3.9 applies if claims about runtime behavior are made.
  - Step 4 applies; 4.4 applies if any proposed fix touches action preconditions.
  - Section H updates only if the change introduces new causal hooks.
  - **Root-cause tracing (Step 2)**: The structured root-cause tracing substeps (a-d) apply to investigation/bugfix specs. For computation-optimization specs, skip root-cause tracing — instead prioritize validating that the spec's referenced types, functions, and integration points exist and have the assumed signatures and semantics.

**Deliverable removal**: If validation reveals a deliverable should be removed entirely, skip remaining sub-steps for that deliverable and record the removal as a finding. Continue validation for surviving deliverables.

**Hybrid specs**: Apply the union of applicable steps — use the most rigorous classification's checklist for shared steps. Common hybrids:
  - **(d)+(e)** (test triage with a bugfix): Steps 3.1-3.4 from both; 3.5-3.8 for bugfix deliverables only; 4.4 if bugfix touches candidate emission/preconditions; Section H only for bugfix deliverables.
  - **(b)+(d)** (system extension with golden tests): Full (b) checklist for production deliverables; (d) rules for test deliverables; 4.4 if any production deliverable modifies validation/emission.
  - **(a)+(d)** (new system with test infrastructure): Full (a) checklist; test deliverables validated per 3.1-3.4 only.

**Re-reassessment shortcut**: If the same spec was reassessed earlier in this session and not externally modified, Steps 2-3 may scope to only references affected by the triggering change. Step 1 still applies.

**Self-authored spec note**: Full validation is required even for specs authored earlier in this session — authoring may introduce unchecked assumptions.

### Step 1: Mandatory Reads

Read ALL of these before any analysis:

1. **The spec file** (from the argument) — entire file
2. **`docs/FOUNDATIONS.md`** — skip if read earlier in this session and unmodified. If the file exceeds the Read tool's token limit, read in sections (e.g., 200 lines each) to cumulatively cover the full document, or target specific principle sections relevant to the spec's domain.
3. **`docs/spec-drafting-rules.md`** (if the spec contains or should contain Section H) — skip if read earlier and unmodified

Parse the spec's metadata: Phase, Status, Priority, Crates, Dependencies, Goals/Design Goals, Non-Goals, FOUNDATIONS Alignment, and all deliverable sections.

**Non-numbered deliverables**: If the spec uses phases or named sections instead of numbered deliverables (common in investigation/bugfix and test-infrastructure specs), treat each distinct implementation section as a deliverable for validation purposes. Adapt references to "deliverable numbers" throughout this skill to the spec's actual organizational scheme (phase labels, section headers).

### Step 2: Extract References

Extract every concrete codebase reference from the spec:

- **File paths** mentioned or implied
- **Type names** (e.g., `GoalKind`, `SaleListing`, `PlannerOpKind`)
- **Function names** (e.g., `generate_candidates`, `enumerate_trade_payloads`)
- **Crate/module names**
- **Test file paths or test names**
- **Other specs or tickets** in Dependencies
- **Code examples** (inline code blocks showing API usage, precondition lists, struct definitions) — extract for fidelity checking against actual source
- **Scenario and test configuration files** referenced by the spec (RON scenarios, test fixtures, seed values) — extract profile/parameter values the spec's claims depend on

Build a validation checklist (internal). Prioritize references most likely to have drifted: dependency paths, function signatures, and types the spec extends. Stable types (`EntityId`, `Permille`, `Quantity`) can be spot-checked.

For investigation/bugfix specs (type e, investigation/bugfix subtype), also prioritize the root-cause hypothesis: trace the claimed failure path through actual code to confirm the spec's causal narrative, not just that the referenced symbols exist. Structured root-cause tracing:

- (a) identify each code path the spec claims participates in the bug
- (b) read the actual implementation of each path
- (c) verify the claimed divergence mechanism by comparing inputs, computation methods, and outputs across paths
- (d) check scenario/test configuration for parameter values that trigger the claimed failure

For computation-optimization specs (type e, optimization subtype), skip root-cause tracing — there is no bug hypothesis to validate. Instead prioritize: (a) that all referenced types, functions, and integration points exist with the assumed signatures, (b) that proposed integration sites have the structural shape the spec assumes (e.g., variable availability, loop structure, timing of data collection), and (c) that proposed new types satisfy existing trait bounds at their intended usage sites.

**Proven-diagnosis scoping**: If the spec's diagnosis is already confirmed by existing tests that demonstrate the specific failure mode or by profiling data that quantifies the specific bottleneck (e.g., golden tests asserting `BudgetExhausted` with concrete candidate counts, or profiling showing specific function hot-paths with measured growth rates), root-cause tracing may scope to verifying the fix's assumptions — that the proposed remedy targets the right code path and reads the right data — rather than re-proving the diagnosis from scratch. Sub-steps (a-b) still apply; (c-d) may be lighter-weight.

### Step 3: Codebase Validation

**Read `references/codebase-validation.md` and `references/worldwake-validation-patterns.md` now, with the Read tool, before any validation work.** These files carry the validation checklists and the pattern-specific triggers (new GoalKind variant, new component on Agent, new component read by AI crate, new action type, new cross-crate enum variant). Skipping these reads means pattern-specific checklists will be missed and findings produced in that state are incomplete.

After reading, acknowledge the load with a one-line "Loaded: codebase-validation.md, worldwake-validation-patterns.md" so the skip is auditable. Then validate every reference from Step 2, applying any pattern-specific checklist the spec triggers.

Do not present findings yet. Collect everything for Step 4.

### Step 4: FOUNDATIONS.md Alignment Check

**Read `references/foundations-alignment.md` now, with the Read tool, before checking alignment.** Acknowledge with "Loaded: foundations-alignment.md". Then check spec alignment against all applicable principles.

### Steps 5-6: Classify and Present Findings

**Read `references/findings-and-questions.md` now, with the Read tool, before classifying.** Acknowledge with "Loaded: findings-and-questions.md". The file prescribes the one-line finding format and the Step 6 presentation template; using your own format is not a substitute. Then classify all findings from Steps 3-4 and present to the user using that template.

Wait for user response before proceeding to Step 7. (In plan mode: after question resolution, write the plan file per `references/plan-mode.md`, then call ExitPlanMode. Steps 7-8 execute after approval.)

### Step 7: Write the Updated Spec

**Pre-Apply Verification**: Before editing, run targeted checks to confirm each finding still holds (e.g., grep confirming symbol presence/absence, count validation). If a finding is invalidated, re-present the corrected finding before applying.

**Read `references/spec-writing-rules.md` now, with the Read tool, before writing.** Acknowledge with "Loaded: spec-writing-rules.md". The file carries the full pre-apply verification, apply-changes, and post-apply confirmation rules. Then apply all approved changes.

### Step 8: Final Summary

Present:

- Number of issues fixed, improvements applied, additions incorporated
- Change inventory: all changes grouped by finding type (mirroring Step 6 structure)
- Post-Apply Confirmation results (e.g., "Verified: zero matches for eliminated references, N matches for corrected references")
- Deferred items the user chose not to address
- Items excluded by reassessment-driven scope changes (distinct from user-deferred) — note why. Omit if none.
- 1-3 sections that changed most substantially, with a note to review before proceeding
- **Classification shift note**: If the reassessment caused the spec's effective classification to shift (e.g., (a) new system collapsed into (b) system extension after deliverable removal, or (e) investigation was promoted to (a) after a new component proved necessary), name the shift explicitly — e.g., "Effective classification shifted (a) → (b) after D2/D3 elimination." This surfaces the change so `/spec-to-tickets` can plan ticket granularity accordingly. Omit if the classification is unchanged.
- Suggested next step: `/spec-to-tickets <spec-path>` (the spec-to-tickets skill will prompt for the ticket namespace)

Do NOT commit. Leave the file for user review.

## Guardrails

- **FOUNDATIONS alignment is mandatory**: Never approve a spec change that violates a Foundation principle, even if requested — flag the conflict instead.
- **Codebase truth**: All references in the updated spec must be validated. Never propagate stale paths, renamed types, or removed functions.
- **No scope creep**: The deliverable is the updated spec file. Do not write design docs, create tickets, or start implementation.
- **No approach proposals**: Validate and refine the existing design, not greenfield alternatives. Exception: when the approach violates a crate boundary, FOUNDATIONS principle, or critical invariant, propose minimum viable alternatives as part of the Issue finding.
- **Substantial redesign flag**: If reassessment changes >50% of deliverables' approach, flag in Step 6: "This reassessment proposes substantial redesign of N/M deliverables. Goals preserved but implementation path changes significantly."
