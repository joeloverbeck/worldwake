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

- **(f) Retroactive reassessment** — reassessment concludes (via Step 3 validation) that all deliverables already landed through downstream tickets. This classification is **not pre-selected**; it activates when every deliverable verifies as implemented in code. The user hint "I suspect this already landed" is a soft signal, not a classification by itself — only Step 3 evidence can confirm (f).
  - Steps 3.1-3.4 apply rigorously to prove landing of every deliverable; cite file paths + line numbers as evidence. Skip Steps 3.5-3.8 (ripple/root-cause substeps) — the work has already shipped, so there is no ripple to trace.
  - Step 4 applies for Outcome-section honesty (does the delivered implementation still align with FOUNDATIONS?).
  - **Step 7 output shape switches to Outcome population + archival**, not deliverable refinement (see Step 7's retroactive branch).
  - **Step 8 suggested next step becomes archival per `docs/archival-workflow.md` + IMPLEMENTATION-ORDER.md reconciliation**, not `/spec-to-tickets` (see Step 8's retroactive path).
  - Classification shift from (a)/(b)/(c)/(d)/(e) → (f) is a legitimate and common outcome when a spec is reassessed after the work already shipped through downstream tickets. Name the shift explicitly in Step 8.
  - (f) does not participate in hybrid combinations — it is outcome-based rather than deliverable-based, and it supersedes the originally-assumed classification once Step 3 confirms full landing.

**Deliverable removal**: If validation reveals a deliverable should be removed entirely, skip remaining sub-steps for that deliverable and record the removal as a finding. Continue validation for surviving deliverables.

**Hybrid specs**: Apply the union of applicable steps — use the most rigorous classification's checklist for shared steps. Common hybrids:
  - **(d)+(e)** (test triage with a bugfix): Steps 3.1-3.4 from both; 3.5-3.8 for bugfix deliverables only; 4.4 if bugfix touches candidate emission/preconditions; Section H only for bugfix deliverables.
  - **(b)+(d)** (system extension with golden tests): Full (b) checklist for production deliverables; (d) rules for test deliverables; 4.4 if any production deliverable modifies validation/emission.
  - **(b)-tooling-only + (d)** (tooling/report/observer enhancement with test-support helpers): Steps 3.1-3.4 apply fully; 3.5-3.7 apply only if the spec extends cross-crate types or enums; 3.3A applies if the spec proposes new observer/CLI output; 3.8 still applies; skip 3.9; Section H updates only for new causal hooks. Check the "Dual-Use Read-Model Types" pattern in `references/worldwake-validation-patterns.md` if the spec proposes types shared between tests and a non-test crate.
  - **(a)+(d)** (new system with test infrastructure): Full (a) checklist; test deliverables validated per 3.1-3.4 only.

**Re-reassessment shortcut**: If the same spec was reassessed earlier in this session and not externally modified, Steps 2-3 may scope to only references affected by the triggering change. Step 1 still applies.

**Self-authored spec note**: Full validation is required even for specs authored earlier in this session — authoring may introduce unchecked assumptions.

### Step 1: Mandatory Reads

Read ALL of these before any analysis:

1. **The spec file** (from the argument) — entire file
2. **`docs/FOUNDATIONS.md`** — skip if read earlier in this session and unmodified. If the file exceeds the Read tool's token limit, read in sections (e.g., 200 lines each) to cumulatively cover the full document, or target specific principle sections relevant to the spec's domain.
3. **`docs/spec-drafting-rules.md`** (if the spec contains or should contain Section H) — skip if read earlier and unmodified. Skip for classification (f) — deliverable refinement and Section H editing do not occur in the retroactive branch.

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

After reading each file, emit a **content-tied acknowledgment** — one per Read call, immediately after that Read returns. Each acknowledgment must quote a concrete anchor from the file just loaded, not a free-form "Loaded: …" string:

- `Loaded codebase-validation.md — top section is "3.0 Cross-Crate Scope Establishment"`
- `Loaded worldwake-validation-patterns.md — first pattern is "New GoalKind Variant"`

A generic "Loaded: codebase-validation.md, worldwake-validation-patterns.md" without a content anchor is treated as a skipped load, because it can be emitted without opening the file. Batching acknowledgments at report time defeats the audit trail. Then validate every reference from Step 2, applying any pattern-specific checklist the spec triggers.

Do not present findings yet. Collect everything for Step 4.

### Step 4: FOUNDATIONS.md Alignment Check

**Read `references/foundations-alignment.md` now, with the Read tool, before checking alignment.** Emit a content-tied acknowledgment immediately after the Read call — e.g., `Loaded foundations-alignment.md — opens with "4.0 Internal Contradictions"`. A bare "Loaded: foundations-alignment.md" is treated as a skipped load. Then check spec alignment against all applicable principles.

### Steps 5-6: Classify and Present Findings

**Read `references/findings-and-questions.md` now, with the Read tool, before classifying.** Emit a content-tied acknowledgment immediately after the Read call — e.g., `Loaded findings-and-questions.md — opens with "Step 5: Classify Findings"`. A bare "Loaded: findings-and-questions.md" is treated as a skipped load. The file prescribes the one-line finding format and the Step 6 presentation template; using your own format is not a substitute. Then classify all findings from Steps 3-4 and present to the user using that template.

**Redesign-count checkpoint**: Before presenting, count deliverables whose approach materially changed versus total deliverables. If the ratio exceeds 50%, the `### Substantial Redesign Flag` section is mandatory in the Step 6 output, placed immediately above `### Questions`. Emit the N/total counts in your pre-draft notes even when the ratio is below 50%, so the decision is auditable.

Wait for user response before proceeding to Step 7. (In plan mode: after question resolution, write the plan file per `references/plan-mode.md`, then call ExitPlanMode. Steps 7-8 execute after approval.)

**Auto mode interaction**: When auto mode is active and the findings contain no Issues (CRITICAL/HIGH severity or FOUNDATIONS violations) and no open Questions, proceed directly to Step 7. Report the auto-mode auto-approval inline in Step 6 presentation (e.g., "Auto mode: no Issues, proceeding to Step 7"). If any Issue is present or any Question is open, the wait-for-user gate still applies even in auto mode.

### Step 7: Write the Updated Spec

#### Pre-Apply Verification Table

Before editing, build a per-finding verification mini-table **and emit it in chat before calling Write/Edit**. For each finding (by its Step 6 key — `I1`, `I2`, `M1`, `F1`, etc.), run a targeted check (grep, count, path existence) and record both the command and the result. The table is the gate — a vague "I checked the findings" is not sufficient and will be treated as no verification.

Example:

| Finding | Check | Result |
|---------|-------|--------|
| I1 | `grep -n "pm(750)" crates/worldwake-ai/tests/golden_survival_*.rs` | 0 matches — confirms stale constant eliminated |
| I2 | `grep -rn "AnomalyKind::" crates/worldwake-cli/src/` | 17 matches, all in `bin/observer.rs` — no external consumers to migrate |
| M3 | `test -f specs/S118-stuck-agent-detector-active-frame-exclusion.md` | file exists — dependency path valid |

If a check reveals a mismatch with a finding, classify the mismatch and respond accordingly:

- **Recommendation-changing mismatch**: the pre-apply check invalidates the finding's *recommendation* — the fix that was approved no longer applies, the target text/symbol has moved, or a different fix is now warranted. Re-present the corrected finding to the user and wait for confirmation before applying any edits. Do not silently drop or modify the finding.
- **Evidence-refining mismatch**: the pre-apply check refines the finding's *supporting evidence* (e.g., a symbol the finding claimed was absent turns out to exist in a different location) but the recommendation still holds unchanged. Note the refinement inline in the Result column of the pre-apply table (e.g., "partial invalidation: symbol exists at <path>:<line>, not at spec-claimed location — recommendation unchanged") and proceed. The user sees the refinement in the emitted table, so this is not silent modification.

Example rows for each tier:

| Finding | Check | Result |
|---------|-------|--------|
| I5 (evidence-refining) | `grep -rn "NEEDS_LOW_CEILING"` | exists at `observer.rs:1931`, not at spec-claimed `golden_survival_contested.rs` — recommendation (cite scenario-authored contract field instead) unchanged |
| I3 (recommendation-changing) | `grep -n "#[cfg(test)]"` at claimed line | boundary has moved; the targeted function is now runtime, not test-only — re-present to user before applying |

The `Finding` column tier tag (`evidence-refining`, `recommendation-changing`) is required only when the pre-apply check detects a mismatch with the finding. Rows that confirm the finding exactly as written may use the compact descriptive form shown in the first example table (`I1`, `I2`, `M3`, optionally with a brief parenthetical anchor).

**Read `references/spec-writing-rules.md` now, with the Read tool, before writing.** Emit a content-tied acknowledgment immediately after the Read call — e.g., `Loaded spec-writing-rules.md — opens with "Pre-Apply Verification"`. A bare "Loaded: spec-writing-rules.md" is treated as a skipped load. The file carries the full pre-apply verification, apply-changes, and post-apply confirmation rules. Then apply all approved changes.

**Retroactive branch (classification (f))**: If Step 3 validation concluded all deliverables already landed, Step 7's output shape is **not** deliverable refinement. Instead:

1. Flip the spec's **Status** to `✅ COMPLETED`.
2. Populate the **Outcome** section with: completion date; landed changes (cite file paths + line numbers); delivering ticket(s); deviations from original plan (especially work absorbed by sibling specs); verification commands **re-run at reassessment time**, and their pass/fail status. Do not copy verification from the delivering ticket — rerun each command now to catch post-delivery regressions.
3. Mark historical **Motivating Evidence** as such — add a short parenthetical noting the drift described was resolved by the landed implementation, so future readers don't treat a stale condition as a live one.
4. Cross-reference any downstream specs that extended or absorbed original-spec scope (e.g., a later spec that added fields to the original's struct).
5. Do **not** apply structural refinements to deliverables that already shipped — the spec file is now a historical record, and editing D-sections to match current code would confuse the causal narrative.

After Step 7 completes for (f), Step 8 drives archival + IMPLEMENTATION-ORDER.md reconciliation rather than suggesting `/spec-to-tickets`.

### Step 8: Final Summary

Present:

- Number of issues fixed, improvements applied, additions incorporated
- Change inventory: all changes grouped by finding type (mirroring Step 6 structure)
- **Post-Apply Confirmation results**: for every finding that eliminated or renamed a reference, grep-prove it is gone and that corrected references resolve — e.g., "Verified: zero matches for eliminated references, N matches for corrected references". For retroactive reassessments (classification (f)), additionally grep every concrete artifact named in the spec's Motivating Evidence (symbols, constants, file-local numbers, old thresholds) and prove its absence or corrected form in the current codebase. This validates the Outcome section's claims are still true at archival time rather than at some earlier point.
- Deferred items the user chose not to address
- Items excluded by reassessment-driven scope changes (distinct from user-deferred) — note why. Omit if none.
- 1-3 sections that changed most substantially, with a note to review before proceeding
- **Classification shift note**: If the reassessment caused the spec's effective classification to shift, name the shift explicitly. Examples:
  - "(a) new system collapsed into (b) system extension after deliverable removal"
  - "(e) investigation was promoted to (a) after a new component proved necessary"
  - "(b) system extension shifted to (f) retroactive reassessment after Step 3 verified full landing"
  This surfaces the change so downstream handling is correct. Omit if the classification is unchanged.
- **Suggested next step**:
  - **Default path** (classifications (a)–(e)): `/spec-to-tickets <spec-path>` — the spec-to-tickets skill will prompt for the ticket namespace.
  - **Retroactive path** (classification (f)): `/spec-to-tickets` is **not** applicable. Instead, complete the archival flow:
    1. Archive the spec per `docs/archival-workflow.md` — move it from `specs/` to `archive/specs/`.
    2. **Reconcile `specs/IMPLEMENTATION-ORDER.md`**: find the spec's roadmap entry, verify it doesn't already say "✅ COMPLETED", and rewrite it using the canonical format used elsewhere in that file: `- **<ID>**: ✅ COMPLETED — archived at [archive/specs/<file>.md](...). <1–2 line summary of landed artifacts>.` Include delivering-ticket IDs and note any fallout absorbed by sibling specs.
    3. **Grep `specs/`, `archive/specs/`, `tickets/`, and `archive/tickets/`** for paths of the form `specs/<ID>-…` and rewrite them to `archive/specs/<ID>-…`. Include archive directories explicitly — prior archived specs and tickets often forward-reference the just-archived spec.

Do NOT commit. Leave the file for user review.

## Guardrails

- **FOUNDATIONS alignment is mandatory**: Never approve a spec change that violates a Foundation principle, even if requested — flag the conflict instead.
- **Codebase truth**: All references in the updated spec must be validated. Never propagate stale paths, renamed types, or removed functions.
- **No scope creep**: The deliverable is the updated spec file. Do not write design docs, create tickets, or start implementation.
- **No approach proposals**: Validate and refine the existing design, not greenfield alternatives. Exception: when the approach violates a crate boundary, FOUNDATIONS principle, or critical invariant, propose minimum viable alternatives as part of the Issue finding.
- **Substantial redesign flag**: If reassessment changes >50% of deliverables' approach, flag in Step 6: "This reassessment proposes substantial redesign of N/M deliverables. Goals preserved but implementation path changes significantly."
