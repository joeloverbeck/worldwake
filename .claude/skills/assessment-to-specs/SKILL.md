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
4. **Current `specs/IMPLEMENTATION-ORDER.md`** — understand what is already built, what phases are completed, what the current state of the project is. (Step 4 will derive the next phase number from this file under the "highest existing phase + 1" rule, so a careful read here speeds that step.) If the file does not exist in `specs/`, check `archive/specs/IMPLEMENTATION-ORDER*.md` for the most recently archived version and read that instead. If `specs/IMPLEMENTATION-ORDER.md` exceeds the Read tool's token limit, read in chunks using offset/limit. The most recent phases (which inform Step 4's auto-detection) and the existing dependency-graph format are the highest-priority sections; older phases can be skimmed.

**Temporal context**: After reading the assessment, determine its generation date (look for timestamps, headers, or metadata). If the document lacks an explicit date, check filesystem modification time (`stat`), then `git log` for the file's last commit date. If neither is available, ask the user for the assessment's generation date before proceeding. Cross-reference against completion dates of specs referenced in the assessment. If the assessment was generated *after* referenced completed specs, note this: "Assessment post-dates S{N} (completed YYYY-MM-DD) — observations reflect post-fix simulation state." Carry this forward to Step 3.2 — post-fix observations take precedence over "already addressed" claims.

**Re-processed assessment**: If `specs/IMPLEMENTATION-ORDER.md` references this same assessment file as the source for completed adjunct waves, the file may have been regenerated from a new simulation run. The established generation date (from header if present, otherwise from mtime per the prior paragraph's fallback chain) takes precedence — if it post-dates all completed specs derived from this file, treat the assessment as a fresh post-fix document. Do not default to "already processed." When in doubt, ask the user whether the assessment reflects a new simulation run.

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

If the assessment contains non-proposal content (investigation signals, health observations, acceptable-architecture confirmations), note these separately. They do not enter the classification pipeline but should be summarized in the triage report under a "### Investigation Signals" or "### Deferred for Future Analysis" section after the classification sections.

#### Step 3: Codebase Validation

For each proposal, validate the assessment's assumptions against the actual codebase.

**Diagnosis vs. observation**: Distinguish between *behavioral observations* (what the simulation did — e.g., "agents starve," "agent posted 487 notices") and *factual claims about code state* (what value a constant has, whether a field exists, what a function does). Post-fix behavioral observations take precedence over "already addressed" claims (see Step 3.2). Factual claims about code state must always be verified against the actual codebase — if the assessment cites a value or structure that no longer matches (e.g., "300-expansion budget" when the default is now 224), note the discrepancy and base classification on the actual codebase state. The guardrail "Codebase truth over external claims" always takes precedence over temporal context for factual claims. The external LLM may correctly observe a symptom but misdiagnose the root cause because it assumes pre-fix codebase state. When this happens, classify the *symptom* as potentially valid for investigation but reject the *proposed fix* if it addresses an already-fixed root cause. If the symptom persists after prior fixes, the new spec should investigate the actual current root cause rather than re-implementing the assessment's proposed fix.

1. **Grep/Glob** for types, functions, files, and components the proposal references. Confirm they exist and have the shape the assessment assumes. The external LLM may have outdated or inaccurate assumptions about the codebase.
2. **Check if already addressed**: Some proposals may describe problems that have already been fixed in recent work. Cross-reference against completed specs in `specs/IMPLEMENTATION-ORDER.md` (or its archived equivalent if already archived). **Important**: If the Temporal Context note from Step 1 indicates the assessment post-dates a completed spec, the assessment's observations take precedence over the completed spec's claims — the fix was demonstrably insufficient. Do not classify these as Reject with reason "already addressed." Instead, classify as Accept or Scope-Down with a note that the prior fix was insufficient and the new spec should reference it.
3. **Verify FOUNDATIONS alignment**: Confirm the proposal's cited FOUNDATIONS principles are correct (right number, right name). Check whether the proposal itself would violate any principles it doesn't cite.
4. **Assess benefit**: Would this change create meaningful downstream consequences (Principle 5)? Or is it "nice to have" without real emergent payoff?
5. **Check for overlap with active specs**: Glob `specs/S*.md` and check whether any existing active spec already covers the proposal's scope. If so, classify as Reject with reason "already covered by S{N}."
6. **Check root-cause accuracy**: If the assessment correctly identifies a problem but misdiagnoses the root cause, note the corrected diagnosis. This typically leads to Scope-Down classification where the spec addresses the real root cause, not the assessment's proposed fix.

When the proposal count is large (>5), use up to 3 Explore agents in parallel to validate different proposal groups simultaneously. Provide each agent with the proposals it should validate and the checklist above. Group proposals by codebase area (e.g., AI/planner proposals together, perception proposals together, ECS/core proposals together) so each agent can efficiently share grep context. If proposals span many areas, group by estimated validation complexity instead.

**Verify critical agent negatives**: Explore agents can return confidently-wrong negatives — "type X does not exist," "field Y is missing," "no call sites for Z" — when the agent's grep strategy did not cover the call sites where those names actually live. Any agent negative that would flip a classification outcome ("reject as already addressed," "accept because missing," "reject as overlap") must be independently verified with a direct `Grep` before being used to classify the proposal. One or two verifications per audit is cheap; trusting a false negative can silently reject valid proposals or accept already-implemented ones. Agent positives (the type was found at file:line) are lower-risk but still warrant a spot-check when they drive a scope-down decision.

#### Step 4: Auto-Detect Next S-Number and Phase Number

**S-number**: Scan `specs/S*.md` and `archive/specs/S*.md` for the highest existing S-number. Increment by 1 for the first new spec. This is needed before presenting the triage report so that spec number assignments are concrete.

**Phase number**: Scan `specs/IMPLEMENTATION-ORDER.md` (or its archived equivalent) for the highest existing `## Phase N` heading regardless of completion status. Increment by 1 for the new phase. The "highest existing" rule (rather than "highest completed") avoids ambiguity when prior phases have outstanding work — Phase 7 with partial drafts still counts as Phase 7's slot occupied. This number is the tentative phase; the user may redirect to an adjunct-wave inside an existing phase during Step 10. Carry the tentative phase number into Phase 2's spec drafts so each spec's "Phase and Status" line can be written without later rework. If Step 10 ultimately resolves to an adjunct-wave inside a different phase, update the affected specs' "Phase and Status" lines in the same pass that writes IMPLEMENTATION-ORDER.md.

#### Step 5: Classify Each Proposal

Before classifying, identify causal dependencies between proposals. If proposal A's problem is a downstream symptom of proposal B's root cause, note this relationship. Root-cause proposals should be classified first; downstream proposals can be rejected with "downstream of PR-{N}" if the root-cause fix is expected to resolve them.

For each proposal, assign one of four classifications:

- **Accept**: The proposal's assumptions are valid, it aligns with FOUNDATIONS, and the change would be beneficial. Record: which spec(s) it maps to, estimated scope. If a prior spec addressed the same scope but the assessment demonstrates the problem persists (post-fix observation per Temporal Context from Step 1), classify as Accept with note: "Prior fix (S{N}) was insufficient — investigation required." The new spec should: (a) name the prior spec and what it attempted, (b) explain why it was insufficient based on the assessment's post-fix observations, and (c) describe how the new approach differs. This prevents the new spec from repeating the same narrow fix.
- **Scope-Down**: The core idea is valuable but the proposal is too ambitious or mixes concerns. Record: what the reduced spec would cover, what is deferred to later.
- **Fold**: The proposal's intent is preserved but absorbed into another accepted spec's deliverables — no standalone spec. Use when one proposal's core ask is naturally a feature of another proposal's spec (e.g., partial-failure outcomes folding into the action-handler spec; event-tag declarations folding into the spec that owns each tag's source-of-truth state). Record: which receiving spec absorbs it, and which aspect of the proposal lands there. Distinct from Reject because the intent survives; distinct from Scope-Down because no new spec is created. The receiving spec must cite the absorbed PR-N per Step 7.
- **Reject**: The proposal's assumptions are wrong (already addressed, codebase differs from what assessment assumes), it violates FOUNDATIONS, or it fails YAGNI (no meaningful downstream consequences). Record: the specific reason for rejection. Use *only* when the proposal's intent is discarded — fold-ins go in the Fold bucket above.

**Tooling and debuggability proposals**: Proposals that improve diagnostic capability (observer enhancements, trace enrichment, dump format improvements) should not be rejected as YAGNI solely because they don't introduce new simulation components or systems. FND-29 (Debuggability Is a Product Feature) makes diagnostic capability a first-class architectural concern. If a tooling proposal concretely improves the ability to diagnose an identified architectural gap or behavioral pathology, it has meaningful downstream consequences and should be classified as Accept. The proposal must still cite a specific diagnostic gap it addresses — "generally useful" is not sufficient.

#### Step 6: Present Triage Report

Present the triage to the user in a structured format:

```
## Triage Report: <assessment title>

**Temporal context**: Assessment generated YYYY-MM-DD; post-dates completed specs S{N1}, S{N2}, ... (cite the most recent ones whose claims the assessment may supersede). If the assessment pre-dates all completed specs, write "no post-dating concerns."

### Accepted (N proposals)
1. **PR-1: <title>** — <1-sentence rationale>. FOUNDATIONS: <aligned / Principle N misnumbered / Principle N violated>. Scope: <Small/Medium/Large>. Spec: S{next}-<name>.
2. ...

### Scoped Down (N proposals)  
1. **PR-3: <title>** — <1-sentence rationale for scope reduction>. FOUNDATIONS: <aligned / Principle N misnumbered>.
   - **Included**: <what the spec will cover>
   - **Deferred**: <what is left for later>
   - Spec: S{next}-<name>.
2. ...

### Folded Into Other Specs (N proposals)
1. **PR-9: <title>** — folded into S{N}-<name>. <1-sentence note on which aspect of the proposal lands in the receiving spec.>
2. ...

### Rejected (N proposals)
1. **PR-5: <title>** — <specific reason for rejection: already addressed / FOUNDATIONS violation / YAGNI>. FOUNDATIONS: <aligned / Principle N violated / N/A>.
2. ...

### Investigation Signals
[If the assessment contains non-proposal signals (areas needing further analysis, health observations), summarize them here. If none, omit this section.]
1. **<signal title>** — <1-sentence description>. Deferred for future analysis.

### Questions
[If any proposals are ambiguous, ask here. Max 3 questions.]
```

Omit classification sections that have 0 entries (e.g., skip the "Rejected" header entirely if nothing was rejected). For lists with 5+ entries, a table format is acceptable as an alternative to the numbered list (works equally well for Rejected and Folded sections). When a question has 2-4 discrete options, use `AskUserQuestion` with labeled options. When open-ended, present in the report.

**Wait for user response.** Do not proceed to Phase 2 until the user has approved or adjusted the triage. Treat classifications as approved unless the user explicitly changes them. The triage approval question must not be bundled with other decisions (e.g., append vs. fresh from Step 10). Present the triage report and wait for classification approval only. Defer implementation-order decisions to Phase 3.

If the user reclassifies proposals (e.g., "accept P5 too" or "reject P2"), update the triage accordingly and confirm the updated list.

If the user corrects a foundational assumption (e.g., temporal context, assessment provenance, codebase state) that invalidates the overall triage — not just individual classifications — restart from Step 3 (Codebase Validation) with the corrected assumption. Present the corrected triage as a fresh report, not an incremental update. Note which assumption was corrected and how it changed the analysis.

**Post-triage spec-granularity adjustments**: If the user responds to triage questions by changing the spec count — "fold spec S into a ticket under spec T," "split spec X into two," or "drop spec Y entirely" — apply the adjustment before Phase 2 begins and renumber sequentially from the auto-detected starting S-number (Step 4). A downgraded-to-ticket scope folds into whichever sibling spec best owns the same architectural surface, added as a concrete Deliverable block that names it as a ticket (not as a standalone spec). After renumbering: update the in-memory dependency graph that feeds Phase 3, update any spec-number references in pending question answers or scope-down notes, and re-print the final spec list with the new numbering so the user can confirm before spec writing begins. Do not leave gaps in the spec numbering (no S110 + S112 + S113 with S111 missing) — subsequent specs fill the vacated numbers.

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

For pure refactor specs (no new types, systems, or components), many sections will be "Not applicable." This is expected. Include the section header with "Not applicable" rather than omitting it, so reassess-spec can confirm the omission was deliberate.

For golden-gaps specs (bundled test scenarios), use the project's golden-gaps convention: per-scenario blocks with Setup, Assertion, GoalKinds/ActionDomains exercised, emergence justification, and "Why it is not a duplicate." See existing archived golden-gaps specs (e.g., `archive/specs/S67-*.md`) for the format.

These are **draft specs**. They contain the architectural shape and key deliverables but expect a `/reassess-spec` pass before ticket decomposition. Do not attempt exhaustive codebase validation of every reference — that is reassess-spec's job. **Exception**: For type names, function names, and enum variants used in deliverable code blocks, grep the codebase to confirm the exact name. Draft quality exempts architectural shape, not concrete identifiers.

**Fold-in attribution**: If the spec absorbs other proposals via the Fold classification (Step 5), the spec's Summary or Design Goals section MUST cite the absorbed PR-Ns and the source assessment file. This preserves traceability from the spec back to the original proposal — without it, future readers (reassess-spec, ticket-decomposition, the implementer) cannot reconstruct which proposal each piece of the spec satisfies. Example phrasing: "Folds in PR-9 sleep-quality, PR-11 interrupted-sleep, and PR-12 sleep events from `reports/proposed-gameplay-mechanic-changes.md`."

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

The phase number was tentatively assigned in Step 4 (highest existing phase + 1). This step decides the *write strategy* and may override the tentative phase if the user redirects to an adjunct-wave inside an existing phase.

**Three write strategies** (always use `AskUserQuestion` to choose, unless plan-mode approval already resolved the decision or no active file exists):

- **(A) Adjunct wave inside an existing phase**: Edit `specs/IMPLEMENTATION-ORDER.md` to add a new wave under an existing `## Phase N` heading. Use when the new specs share the existing phase's theme and naturally extend its dependency graph. Choosing (A) requires updating each spec's "Phase and Status" line written in Phase 2 (per Step 4's note) so it names the host phase rather than the tentative new phase.
- **(B) New phase appended to the file**: Edit `specs/IMPLEMENTATION-ORDER.md` to add a new `## Phase N` section after the last existing phase, leaving prior phases untouched. Use when the new specs form a coherent new phase but the existing file's plan is not obsolete (typical case). The tentative phase number from Step 4 stands.
- **(C) Fresh file**: Replace `specs/IMPLEMENTATION-ORDER.md` entirely. Use when the existing plan is obsolete or being archived. Warn the user that a fresh file will be written and suggest archiving the old one (see `docs/archival-workflow.md`).

**When to skip the AskUserQuestion**:
- Plan-mode approval earlier in the conversation specified the strategy.
- No active `specs/IMPLEMENTATION-ORDER.md` exists (read from `archive/` per Step 1) — (C) is the only option.

**When to ask**: All other cases. The decision affects file lifecycle and prior plan preservation; the user must own it.

**Recommended-default phrasing for the AskUserQuestion**: Mark (B) as recommended when the new specs share survival/AI/system surface with current phase work but form their own conceptual unit; mark (A) as recommended when fewer than ~3 specs are accepted and they slot into an existing phase's dependency graph; (C) is never recommended by default — it's user-driven archival.

**Strategy-(A) phase override**: If the user picks (A), update each spec's "Phase and Status" line in the same write pass. Grep for "Phase {tentative_N}" in the new specs and replace with the host phase name (e.g., "Phase 7: Consequence Carriers — Adjunct Wave: <name>").

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
- [ ] (If assessment was scenario-derived) Rerun of `<scenario-name>` narrative report shows the targeted behavioral changes — name the specific observable shifts (e.g., "no `sleep → sleep` artifact," "measurable variation in sleep-site preference across agents," "`desired_target > 1` acquisition under projected horizons")
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
- **Plan-mode interaction**: If invoked from plan mode, the skill's file writes (specs, IMPLEMENTATION-ORDER.md) are permitted as the plan's only editable files. The plan file itself serves as the execution summary — avoid duplicating spec summaries and implementation-order details in both the plan file and the conversation. **Exception**: The triage report (Step 6) must be summarized in the AskUserQuestion for triage approval, since the user cannot see the plan file until ExitPlanMode. This is the one permitted duplication. Post-approval, spec summaries and implementation-order summaries should go only in the plan file. Defer implementation-order decisions (append vs. fresh) to Phase 3, not to the triage approval interaction in Step 6. If plan mode exits during the skill (e.g., after AskUserQuestion approval), continue the remaining phases normally — the plan file still serves as the execution summary. Do not attempt to re-enter or re-exit plan mode.
