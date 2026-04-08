---
name: assessment-to-specs
description: "Triages an external LLM assessment against the codebase and FOUNDATIONS.md, writes draft specs for accepted proposals, and creates a fresh IMPLEMENTATION-ORDER.md."
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

**Pre-flight check**: If `specs/IMPLEMENTATION-ORDER.md` exists in `specs/`, note its presence for Step 10. Do not warn about overwriting yet — the write strategy (append vs. fresh) depends on the number and nature of accepted proposals, which is not known until after triage. If the file was already archived (read from `archive/`), note that no active file exists.

#### Step 2: Extract Proposals

From the assessment document, extract every distinct proposal. For each proposal, record:

- **Proposal ID**: Sequential number (P1, P2, P3, ...)
- **Title**: Short descriptive name
- **Priority**: As stated in the assessment (Priority 0, 1, 2, etc.), or "Unranked" if not prioritized
- **Claim**: What the assessment says is wrong, missing, or improvable
- **FOUNDATIONS references**: Which principles the assessment cites
- **Proposed change**: What the assessment recommends
- **Scope estimate**: Small (single component/module), Medium (cross-cutting but bounded), Large (architectural overhaul)

#### Step 3: Codebase Validation

For each proposal, validate the assessment's assumptions against the actual codebase:

1. **Grep/Glob** for types, functions, files, and components the proposal references. Confirm they exist and have the shape the assessment assumes. The external LLM may have outdated or inaccurate assumptions about the codebase.
2. **Check if already addressed**: Some proposals may describe problems that have already been fixed in recent work. Cross-reference against completed specs in `specs/IMPLEMENTATION-ORDER.md` (or its archived equivalent if already archived).
3. **Verify FOUNDATIONS alignment**: Confirm the proposal's cited FOUNDATIONS principles are correct (right number, right name). Check whether the proposal itself would violate any principles it doesn't cite.
4. **Assess benefit**: Would this change create meaningful downstream consequences (Principle 5)? Or is it "nice to have" without real emergent payoff?
5. **Check for overlap with active specs**: Glob `specs/S*.md` and check whether any existing active spec already covers the proposal's scope. If so, classify as Reject with reason "already covered by S{N}."

When the proposal count is large (>5), use up to 3 Explore agents in parallel to validate different proposal groups simultaneously. Provide each agent with the proposals it should validate and the checklist above. Group proposals by codebase area (e.g., AI/planner proposals together, perception proposals together, ECS/core proposals together) so each agent can efficiently share grep context. If proposals span many areas, group by estimated validation complexity instead.

#### Step 4: Auto-Detect Next S-Number

Scan `specs/S*.md` and `archive/specs/S*.md` for the highest existing S-number. Increment by 1 for the first new spec. This is needed before presenting the triage report so that spec number assignments are concrete.

#### Step 5: Classify Each Proposal

For each proposal, assign one of three classifications:

- **Accept**: The proposal's assumptions are valid, it aligns with FOUNDATIONS, and the change would be beneficial. Record: which spec(s) it maps to, estimated scope.
- **Reject**: The proposal's assumptions are wrong (already addressed, codebase differs from what assessment assumes), it violates FOUNDATIONS, or it fails YAGNI (no meaningful downstream consequences). Record: the specific reason for rejection.
- **Scope-Down**: The core idea is valuable but the proposal is too ambitious or mixes concerns. Record: what the reduced spec would cover, what is deferred to later.

#### Step 6: Present Triage Report

Present the triage to the user in a structured format:

```
## Triage Report: <assessment title>

### Accepted (N proposals)
1. **P1: <title>** — <1-sentence rationale>. FOUNDATIONS: <aligned / P{N} misnumbered / P{N} violated>. Scope: <Small/Medium/Large>. Spec: S{next}-<name>.
2. ...

### Scoped Down (N proposals)  
1. **P3: <title>** — <1-sentence rationale for scope reduction>. FOUNDATIONS: <aligned / P{N} misnumbered>.
   - **Included**: <what the spec will cover>
   - **Deferred**: <what is left for later>
   - Spec: S{next}-<name>.
2. ...

### Rejected (N proposals)
1. **P5: <title>** — <specific reason for rejection>. FOUNDATIONS: <aligned / P{N} violated / N/A>.
2. ...

### Questions
[If any proposals are ambiguous, ask here. Max 3 questions.]
```

Omit classification sections that have 0 entries (e.g., skip the "Rejected" header entirely if nothing was rejected). When a question has 2-4 discrete options, use `AskUserQuestion` with labeled options. When open-ended, present in the report.

**Wait for user response.** Do not proceed to Phase 2 until the user has approved or adjusted the triage. Treat classifications as approved unless the user explicitly changes them.

If the user reclassifies proposals (e.g., "accept P5 too" or "reject P2"), update the triage accordingly and confirm the updated list.

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

These are **draft specs**. They contain the architectural shape and key deliverables but expect a `/reassess-spec` pass before ticket decomposition. Do not attempt exhaustive codebase validation of every reference — that is reassess-spec's job.

When writing multiple specs (>3) and the existing context from Phase 1 is insufficient to write them confidently, use Explore agents in parallel to trace additional codebase references for different specs simultaneously.

#### Step 8: Verify and Present Written Specs

After writing all specs, spot-check that each contains: FOUNDATIONS Alignment table, Section H (where applicable), Deliverables with concrete types, and Component Registration section. Report any missing mandatory sections before presenting the summary.

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

**Append vs. fresh**: If only 1-2 independent specs were accepted and they have no dependencies on unreleased specs in the current active phase, offer the user a choice: (a) append to the existing phase as parallel Wave 1 items (recommended for small additions), or (b) create a new phase. For single-spec results, appending is the default recommendation. If appending, edit the existing `specs/IMPLEMENTATION-ORDER.md` to add the new spec(s) to the dependency graph and Wave 1 list — do not overwrite. If creating a new phase or if no active IMPLEMENTATION-ORDER.md exists, warn the user that a fresh file will be written and suggest archiving the old one (see `docs/archival-workflow.md`).

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
1. Archive the old `specs/IMPLEMENTATION-ORDER.md` if not already done
2. Run `/reassess-spec <path>` on each new spec before ticket decomposition
3. Begin implementation with Wave 1
```

Do NOT commit. Leave all files for user review.

---

## Guardrails

- **FOUNDATIONS alignment is mandatory**: Every accepted proposal must align with `docs/FOUNDATIONS.md`. Reject proposals that violate principles, even if the external LLM recommends them — flag the conflict instead.
- **Codebase truth over external claims**: The external LLM does not have repo access. Always verify assumptions against the actual codebase before accepting a proposal.
- **YAGNI**: Reject proposals that do not create meaningful downstream consequences (Principle 5). "It would be nice" or "it feels more complete" is not sufficient justification.
- **No backward compatibility layers**: New specs must not introduce shims, redirects, or compatibility wrappers (Principle 28). When a design changes, update or remove the old path.
- **Draft quality**: Specs are drafts intended for `/reassess-spec` before ticket decomposition. Do not attempt exhaustive codebase validation — that is reassess-spec's job.
- **Spec-drafting-rules compliance**: All specs use `Permille` for ranges, profile-driven parameters, FND-01 Section H where applicable, and follow `docs/spec-drafting-rules.md` format.
- **No archival**: The skill does not archive old specs or the old IMPLEMENTATION-ORDER.md. The user handles archival separately via `docs/archival-workflow.md`.
- **No commit**: Write all files and stop. The user handles the file lifecycle.
- **Worktree discipline**: If working in a worktree, ALL file operations use the worktree root path.
- **Preserve spec voice**: Match the existing writing style of specs in `specs/`. Do not introduce a different tone or structure.
