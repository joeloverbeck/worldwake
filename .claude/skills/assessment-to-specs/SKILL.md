---
name: assessment-to-specs
description: "Triages an external LLM assessment against the codebase and FOUNDATIONS.md, writes draft specs for accepted proposals, and updates IMPLEMENTATION-ORDER.md."
user-invocable: true
arguments:
  - name: assessment_path
    description: "Path to the assessment document (e.g., brainstorming/ai-architecture-assessment.md)"
    required: true
---

# Assessment to Specs

Converts an external LLM assessment (architecture review, feature proposals, game design doc) into a triaged set of draft specs and a fresh implementation order for the next project phase.

The external LLM does not have direct access to the codebase. Every proposal must be validated against the actual code and `docs/FOUNDATIONS.md` before acceptance.

## Invocation

```
/assessment-to-specs <assessment-path>
```

**Arguments** (required, positional):
- `<assessment-path>` — path to the assessment document (e.g., `brainstorming/ai-architecture-assessment.md`)

If the argument is missing, ask the user to provide it before proceeding.

## Worktree Awareness

If working inside a worktree (e.g., `.claude/worktrees/<name>/`), ALL file paths — reads, writes, globs, greps — must use the worktree root as the base path. The default working directory is the main repo root; paths without an explicit worktree prefix will silently operate on main.

## Process

Follow these 3 phases in order. Do not skip any phase.

---

### Phase 1 — Triage

#### Step 1: Mandatory Reads

Read ALL of these files before any analysis:

1. **The assessment document** (from the argument) — read the entire file
2. **`docs/FOUNDATIONS.md`** — architectural commandments. Skip if read earlier in this session and not modified since.
3. **`docs/spec-drafting-rules.md`** — spec format requirements. Skip if read earlier in this session and not modified since.
4. **Current `specs/IMPLEMENTATION-ORDER.md`** — understand what is already built, what phases are completed, what the current state of the project is. Also determine the highest completed phase number for use in Phase 3. If the file does not exist in `specs/`, check `archive/specs/IMPLEMENTATION-ORDER*.md` for the most recently archived version and read that instead.

**Temporal context**: After reading the assessment, determine its generation date (look for timestamps, headers, or metadata). If the document lacks an explicit date, check filesystem modification time (`stat`), then `git log` for the file's last commit date. If neither is available, ask the user for the assessment's generation date before proceeding. Cross-reference against completion dates of specs referenced in the assessment. If the assessment was generated *after* referenced completed specs, note this: "Assessment post-dates S{N} (completed YYYY-MM-DD) — observations reflect post-fix simulation state." Carry this forward to Step 3.2 — post-fix observations take precedence over "already addressed" claims.

**Re-processed assessment**: If `specs/IMPLEMENTATION-ORDER.md` references this same assessment file as the source for completed adjunct waves, the file may have been regenerated from a new simulation run. The generation date in the assessment header takes precedence — if it post-dates all completed specs derived from this file, treat the assessment as a fresh post-fix document. Do not default to "already processed." When in doubt, ask the user whether the assessment reflects a new simulation run.

**Pre-flight check**: If `specs/IMPLEMENTATION-ORDER.md` exists in `specs/`, note its presence for Step 10. Do not warn about overwriting yet — the write strategy (append vs. fresh) depends on the number and nature of accepted proposals, which is not known until after triage. If the file was already archived (read from `archive/`), note that no active file exists.

#### Step 2: Extract Proposals

From the assessment document, extract every distinct proposal. For each proposal, record:

- **Proposal ID**: Sequential number (PR-1, PR-2, PR-3, ...). If the assessment already uses structured proposal IDs (e.g., GT-1, SC-2, TK-3), reuse those IDs instead of renumbering. Note the mapping if the assessment IDs are non-sequential or ambiguous.
- **Title**: Short descriptive name
- **Priority**: As stated in the assessment (Priority 0, 1, 2, etc.), or "Unranked" if not prioritized
- **Claim**: What the assessment says is wrong, missing, or improvable
- **FOUNDATIONS references**: Which principles the assessment cites
- **Proposed change**: What the assessment recommends
- **Scope estimate**: Small (single component/module), Medium (cross-cutting but bounded), Large (architectural overhaul)

If the assessment contains heterogeneous proposal types (golden tests, tickets, tooling enhancements), group related proposals that naturally belong in a single spec. A golden-gaps spec may bundle multiple test proposals. Record the grouping in the extraction and carry it forward to classification.

#### Step 3: Codebase Validation

For each proposal, validate the assessment's assumptions against the actual codebase:

1. **Grep/Glob** for types, functions, files, and components the proposal references. Confirm they exist and have the shape the assessment assumes. The external LLM may have outdated or inaccurate assumptions about the codebase.
2. **Check if already addressed**: Some proposals may describe problems that have already been fixed in recent work. Cross-reference against completed specs in `specs/IMPLEMENTATION-ORDER.md` (or its archived equivalent if already archived). **Important**: If the Temporal Context note from Step 1 indicates the assessment post-dates a completed spec, the assessment's observations take precedence over the completed spec's claims — the fix was demonstrably insufficient. Do not classify these as Reject with reason "already addressed." Instead, classify as Accept or Scope-Down with a note that the prior fix was insufficient and the new spec should reference it.
3. **Verify FOUNDATIONS alignment**: Confirm the proposal's cited FOUNDATIONS principles are correct (right number, right name). Check whether the proposal itself would violate any principles it doesn't cite.
4. **Assess benefit**: Would this change create meaningful downstream consequences (Principle 5)? Or is it "nice to have" without real emergent payoff?
5. **Check for overlap with active specs**: Glob `specs/S*.md` and check whether any existing active spec already covers the proposal's scope. If so, classify as Reject with reason "already covered by S{N}."
6. **Check root-cause accuracy**: If the assessment correctly identifies a problem but misdiagnoses the root cause, note the corrected diagnosis. This typically leads to Scope-Down classification where the spec addresses the real root cause, not the assessment's proposed fix.

When the proposal count is large (>5), use up to 3 Explore agents in parallel to validate different proposal groups simultaneously. Provide each agent with the proposals it should validate and the checklist above. Group proposals by codebase area (e.g., AI/planner proposals together, perception proposals together, ECS/core proposals together) so each agent can efficiently share grep context. If proposals span many areas, group by estimated validation complexity instead.

#### Step 4: Auto-Detect Next S-Number

Scan `specs/S*.md` and `archive/specs/S*.md` for the highest existing S-number. Increment by 1 for the first new spec. This is needed before presenting the triage report so that spec number assignments are concrete.

#### Step 5: Classify Each Proposal

Before classifying, identify causal dependencies between proposals. If proposal A's problem is a downstream symptom of proposal B's root cause, note this relationship. Root-cause proposals should be classified first; downstream proposals can be rejected with "downstream of PR-{N}" if the root-cause fix is expected to resolve them.

For each proposal, assign one of three classifications:

- **Accept**: The proposal's assumptions are valid, it aligns with FOUNDATIONS, and the change would be beneficial. Record: which spec(s) it maps to, estimated scope. If a prior spec addressed the same scope but the assessment demonstrates the problem persists (post-fix observation per Temporal Context from Step 1), classify as Accept with note: "Prior fix (S{N}) was insufficient — investigation required." The new spec should: (a) name the prior spec and what it attempted, (b) explain why it was insufficient based on the assessment's post-fix observations, and (c) describe how the new approach differs. This prevents the new spec from repeating the same narrow fix.
- **Reject**: The proposal's assumptions are wrong (already addressed, codebase differs from what assessment assumes), it violates FOUNDATIONS, or it fails YAGNI (no meaningful downstream consequences). Record: the specific reason for rejection.
- **Scope-Down**: The core idea is valuable but the proposal is too ambitious or mixes concerns. Record: what the reduced spec would cover, what is deferred to later.

**Tooling and debuggability proposals**: Proposals that improve diagnostic capability (observer enhancements, trace enrichment, dump format improvements) should not be rejected as YAGNI solely because they don't introduce new simulation components or systems. FND-29 (Debuggability Is a Product Feature) makes diagnostic capability a first-class architectural concern. If a tooling proposal concretely improves the ability to diagnose an identified architectural gap or behavioral pathology, it has meaningful downstream consequences and should be classified as Accept. The proposal must still cite a specific diagnostic gap it addresses — "generally useful" is not sufficient.

#### Step 6: Present Triage Report

Present the triage to the user in a structured format:

```
## Triage Report: <assessment title>

### Accepted (N proposals)
1. **PR-1: <title>** — <1-sentence rationale>. FOUNDATIONS: <aligned / Principle N misnumbered / Principle N violated>. Scope: <Small/Medium/Large>. Spec: S{next}-<name>.
2. ...

### Scoped Down (N proposals)  
1. **PR-3: <title>** — <1-sentence rationale for scope reduction>. FOUNDATIONS: <aligned / Principle N misnumbered>.
   - **Included**: <what the spec will cover>
   - **Deferred**: <what is left for later>
   - Spec: S{next}-<name>.
2. ...

### Rejected (N proposals)
1. **PR-5: <title>** — <specific reason for rejection>. FOUNDATIONS: <aligned / Principle N violated / N/A>.
2. ...

### Questions
[If any proposals are ambiguous, ask here. Max 3 questions.]
```

Omit classification sections that have 0 entries (e.g., skip the "Rejected" header entirely if nothing was rejected). For rejection lists with 5+ proposals, a table format is acceptable as an alternative to the numbered list. When a question has 2-4 discrete options, use `AskUserQuestion` with labeled options. When open-ended, present in the report.

**Wait for user response.** Do not proceed to Phase 2 until the user has approved or adjusted the triage. Treat classifications as approved unless the user explicitly changes them. The triage approval question must not be bundled with other decisions (e.g., append vs. fresh from Step 10). Present the triage report and wait for classification approval only. Defer implementation-order decisions to Phase 3.

If the user reclassifies proposals (e.g., "accept P5 too" or "reject P2"), update the triage accordingly and confirm the updated list.

If the user corrects a foundational assumption (e.g., temporal context, assessment provenance, codebase state) that invalidates the overall triage — not just individual classifications — restart from Step 3 (Codebase Validation) with the corrected assumption. Present the corrected triage as a fresh report, not an incremental update. Note which assumption was corrected and how it changed the analysis.

---

### Phase 2 — Spec Writing

After the triage is approved:

#### Step 7: Write Draft Specs

For each accepted or scoped-down proposal, write a draft spec to `specs/S{next}-{short-name}.md`. Use lowercase-kebab-case for `{short-name}`. Name should describe the deliverable, not the problem. Match existing spec naming patterns (e.g., `S42-per-agent-reasoning-style`, `S44-generalized-contention-substrate`). Avoid abstract names like `belief-improvement` — prefer concrete names like `entity-belief-claims`.

Each spec MUST follow project conventions from `docs/spec-drafting-rules.md`:

1. **Title and Summary**: One-paragraph description of what the spec delivers
2. **Phase and Status**: Phase name, Status: Draft
3. **Crates**: Which crates are affected
4. **Dependencies**: Which prior specs/epics this depends on
5. **Design Goals**: What the spec optimizes for
6. **Non-Goals**: What is explicitly out of scope
7. **FOUNDATIONS Alignment**: Table mapping principle numbers to how the spec satisfies them. Verify principle numbers match `docs/FOUNDATIONS.md` — misnumbered principles are a common error.
8. **Deliverables**: Concrete types, functions, components, with enough detail to understand the architectural shape. Include field definitions for new types. Use `Permille` for any [0,1] or [0,1000] range values.
9. **FND-01 Section H** (causal hooks declaration): Include where applicable, per `docs/spec-drafting-rules.md`
10. **SystemFn Integration**: How new systems integrate with the tick execution order
11. **Component Registration**: New components to register in `component_schema.rs`
12. **Cross-System Interactions**: How the spec interacts with existing systems through state (Principle 26 — never direct calls)
13. **Profile-Driven Parameters**: Per-agent profile structs instead of hardcoded constants

For golden-gaps specs (bundled test scenarios), use the project's golden-gaps convention: per-scenario blocks with Setup, Assertion, GoalKinds/ActionDomains exercised, emergence justification, and "Why it is not a duplicate." See existing archived golden-gaps specs (e.g., `archive/specs/S67-*.md`) for the format.

These are **draft specs**. They contain the architectural shape and key deliverables but expect a `/reassess-spec` pass before ticket decomposition. Do not attempt exhaustive codebase validation of every reference — that is reassess-spec's job. **Exception**: For type names, function names, and enum variants used in deliverable code blocks, grep the codebase to confirm the exact name. Draft quality exempts architectural shape, not concrete identifiers.

When writing multiple specs and the existing context from Phase 1 is insufficient to write them confidently, use Explore agents in parallel to trace additional codebase references for different specs simultaneously.

#### Step 8: Verify and Present Written Specs

After writing all specs, spot-check that each contains: FOUNDATIONS Alignment table (verify principle numbers match `docs/FOUNDATIONS.md` headings), Section H (where applicable), Deliverables with concrete types, and Component Registration section. If the spot-check finds missing sections or misnumbered principles, fix them before presenting the summary. The summary should confirm all mandatory sections are present.

```
## Specs Written

1. `specs/S50-<name>.md` — <1-sentence summary>
2. `specs/S51-<name>.md` — <1-sentence summary>
...

All specs are draft quality. Run `/reassess-spec <path>` on each before ticket decomposition.
```

---

### Phase 3 — Implementation Order

#### Step 9: Analyze Dependencies

For each new spec, determine:
- Which other new specs it depends on
- Which existing completed specs/epics it builds on
- Which specs are independent and can run in parallel

A spec depends on another if it: (a) references types or components the other spec introduces, (b) modifies code the other spec also modifies (merge conflict risk), or (c) the other spec's deliverables are preconditions in this spec's Section H. Soft dependencies (benefit but not blocking) should be noted but not treated as hard blockers for wave ordering.

#### Step 10: Write Implementation Order

Determine the next phase number from the completed phases in the old `specs/IMPLEMENTATION-ORDER.md` (read in Step 1). Increment the highest completed phase number by 1.

**Append vs. fresh**: If the accepted specs are adjunct to the current phase (small scope, no dependencies on unreleased Phase N specs, and they integrate naturally into the existing dependency graph), offer the user a choice: (a) append to the existing phase as an adjunct wave (recommended for small additions), or (b) create a new phase. Appending is the default recommendation when the specs don't interact with the existing wave structure. If the decision was already resolved during a planning phase (e.g., plan mode approval that specified append or fresh), the prior approval satisfies this requirement — do not re-ask. If appending, edit the existing `specs/IMPLEMENTATION-ORDER.md` to add the new spec(s) to the dependency graph and wave list — do not overwrite. If creating a new phase or if no active IMPLEMENTATION-ORDER.md exists, warn the user that a fresh file will be written and suggest archiving the old one (see `docs/archival-workflow.md`).

**When writing a fresh file**, use the following structure:

```markdown
# Implementation Order & Dependency Graph

## Completed Work

Phases 1-5 (E01-E22, FND-01, FND-02, S01-S49) completed.
See `archive/` for detailed completion records.

---

## Phase N: <Phase Name>

Derived from external assessment (`<assessment-path>`) validated against
the actual codebase and `docs/FOUNDATIONS.md`.

### Dependency Graph

\```text
S50 (independent)
S51 ──→ S52 (S52 depends on S51)
S50, S51 (parallel)
\```

### Active Execution Steps

**Wave 1** (parallel, no deps):
- **S50**: <title> — <1-line summary>
- **S51**: <title> — <1-line summary>

**Wave 2** (after Wave 1):
- **S52**: <title> — <1-line summary>
  - depends on S51

...

### Phase Gate
- [ ] All specs reassessed and ticket-decomposed
- [ ] All wave specs implemented and passing golden E2E tests
- [ ] Canonical regressions addressed by this phase fully producible
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean
- [ ] `cargo test --workspace` passing
- [ ] Golden E2E coverage for each new spec's core behavior
```

Match the existing style from the current `specs/IMPLEMENTATION-ORDER.md` but start fresh — do not carry forward completed work details beyond the one-line reference.

#### Step 11: Present Summary

After writing IMPLEMENTATION-ORDER.md:

```
## Implementation Order Written

`specs/IMPLEMENTATION-ORDER.md` — <N> specs across <M> waves.

Next steps:
1. Archive the old `specs/IMPLEMENTATION-ORDER.md` if a fresh file was written (skip if appended)
2. Run `/reassess-spec <path>` on each new spec before ticket decomposition
3. Begin implementation with Wave 1
```

Do NOT commit. Leave all files for user review.

---

## Guardrails

- **FOUNDATIONS alignment is mandatory**: Every accepted proposal must align with `docs/FOUNDATIONS.md`. Reject proposals that violate principles, even if the external LLM recommends them — flag the conflict instead.
- **Codebase truth over external claims**: The external LLM does not have repo access. Always verify assumptions against the actual codebase before accepting a proposal.
- **YAGNI**: Reject proposals that do not create meaningful downstream consequences (Principle 5). "It would be nice" or "it feels more complete" is not sufficient justification. **Exception**: FND-29 (Debuggability) makes diagnostic capability a first-class concern. Tooling proposals that address a specific identified diagnostic gap (e.g., "the observer cannot answer 'why did this agent not eat?'") have meaningful downstream consequences and should not be rejected as YAGNI. The proposal must name the specific diagnostic question it enables — vague debuggability claims do not qualify.
- **No backward compatibility layers**: New specs must not introduce shims, redirects, or compatibility wrappers (Principle 28). When a design changes, update or remove the old path.
- **Draft quality**: Specs are drafts intended for `/reassess-spec` before ticket decomposition. Do not attempt exhaustive codebase validation — that is reassess-spec's job.
- **Spec-drafting-rules compliance**: All specs use `Permille` for ranges, profile-driven parameters, FND-01 Section H where applicable, and follow `docs/spec-drafting-rules.md` format.
- **No archival**: The skill does not archive old specs or the old IMPLEMENTATION-ORDER.md. The user handles archival separately via `docs/archival-workflow.md`.
- **No commit**: Write all files and stop. The user handles the file lifecycle.
- **Worktree discipline**: If working in a worktree, ALL file operations use the worktree root path.
- **Preserve spec voice**: Match the existing writing style of specs in `specs/`. Do not introduce a different tone or structure.
- **Plan-mode interaction**: If invoked from plan mode, the skill's file writes (specs, IMPLEMENTATION-ORDER.md) are permitted as the plan's only editable files. The plan file itself serves as the execution summary — do not duplicate the triage report or spec summaries in both the plan file and the conversation. Defer implementation-order decisions (append vs. fresh) to Phase 3, not to the triage approval interaction in Step 6.
