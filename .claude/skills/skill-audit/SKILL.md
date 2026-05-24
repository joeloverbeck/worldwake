---
name: skill-audit
description: "Use when a skill was exercised during the current session and you want to evaluate its quality, find gaps, or identify improvements. Triggers: end of session, after implementing with a skill, after encountering skill friction."
user-invocable: true
arguments:
  - name: skill-path
    description: "Path to skill directory (e.g., .claude/skills/improve-loop)"
    required: true
---

# Skill Audit

Analyze a skill file against the work done in the current Claude Code session to determine whether the skill has issues, could be improved, or needs new features. Report only — never modify the target skill.

## Invocation

```
/skill-audit <path-to-skill-directory>
```

Example: `/skill-audit .claude/skills/improve-loop`

The argument is the skill directory path. The framework automatically resolves `SKILL.md` within it.

## Checklist

1. **Read the target skill** — Read the SKILL.md file at the provided path. Parse its name, description, and full content. If the exact path does not resolve, glob for `<path>*/SKILL.md` (appending wildcard + `/SKILL.md`). If that also fails, try `<path>**/SKILL.md`. If exactly one match is found, use it and note the correction. If zero or multiple matches, stop and report the error.
2. **Read alignment documents** — Read `docs/FOUNDATIONS.md` — skip only if read earlier in this session (fully or via partial reads that cumulatively covered the document), not from memory or training knowledge. If the file exceeds the Read tool's token limit, read the first 200 lines (preamble + principle listing) using offset/limit, or read relevant sections targeted to the audit topic. Multiple partial reads that cumulatively cover the full document satisfy this requirement. System context injection of CLAUDE.md (which summarizes key principles) is acceptable as a supplement but not a replacement for reading FOUNDATIONS.md at least once per session. `CLAUDE.md` is always available via system context injection and does not need explicit reading. For meta-tooling skill targets (e.g., brainstorm, skill-audit, skill-extract-references, skill-consolidate, skill-rebalance-references, and similar process/tooling skills), this FOUNDATIONS.md read may be skipped — alignment will be N/A per Step 4. **Criterion**: a skill counts as meta-tooling for this purpose when its job is process orchestration over specs/tickets/code rather than directly authoring content that becomes part of the simulation. Skills like reassess-spec, spec-to-tickets, post-ticket-review, and learning-audit qualify even though they touch simulation-adjacent artifacts, because the skill's output is process guidance, not simulation behavior.
3. **Session reflection** — Review the current conversation context to identify the items below. If the target skill is skill-audit itself (self-audit), use session evidence from any prior audit invocation(s) in this session, including any follow-up implementation phases triggered by those audits — the implementation phase exercises the Follow-up implementation guardrails (tag interpretation, edit ordering, cascade handling, finding-key conventions, post-edit verification) and is valid evidence. The self-audit invocation itself provides no independent session evidence beyond confirming the skill's flow works. If no prior audit invocation exists in this session, report "No session evidence available — self-audit with no prior invocations produces no findings beyond confirming the skill's flow parses correctly." and skip steps 3-6.
   - Moments where the skill's instructions were unclear or ambiguous
   - Steps that were skipped, reordered, or worked around
   - Behaviors the skill didn't anticipate (edge cases, unexpected inputs)
   - Places where Claude had to improvise because the skill didn't provide guidance
   - Outcomes that diverged from what the skill intended
   - Steps that were not exercised this session (mark as "not exercised" — do not speculate about issues)
4. **Cross-check alignment** — For each finding from step 3, check whether the skill contradicts or fails to implement:
   - Principles from `docs/FOUNDATIONS.md` (reference by foundation number)
   - Conventions from `CLAUDE.md` (reference by section name)
   - For meta/tooling skills that do not touch simulation design (e.g., brainstorm, skill-audit, skill-extract-references, skill-consolidate, skill-rebalance-references, and similar process/tooling skills — see Step 2 criterion for the boundary), note "N/A — meta-tooling skill, FOUNDATIONS principles do not apply" and move on. Reserve detailed alignment analysis for skills that govern simulation code, specs, or tickets.
5. **Classify findings** — Categorize each finding into one of three buckets:
   - **Issue**: Something broken, misleading, or contradictory in the skill
   - **Improvement**: A refinement to existing behavior that would make the skill more effective
   - **Feature**: A new capability that aligns with the skill's stated intent but is currently missing
6. **Severity-tag each finding** — CRITICAL / HIGH / MEDIUM / LOW. Use this rubric:
   - **CRITICAL**: Skill produces wrong output, corrupts state, or violates a FOUNDATIONS principle. Must fix before the skill is used again.
   - **HIGH**: Missing guardrail or instruction that has already caused rework or wrong output in this session, or a plausibly near-term failure mode on the next use.
   - **MEDIUM**: Friction that cost non-trivial improvisation or required non-obvious judgment to work around. The skill still produced the right outcome, but the path was not smooth.
   - **LOW**: Wording refinement, coverage gap, or polish. Did not block progress and a competent operator could work past it without guidance.
7. **Cross-skill consistency scan and report** — Before constructing the report, evaluate the trigger list in the `Cross-skill consistency` Guardrail (below). If any finding affects a shared surface, scan the named sibling skills for inconsistencies; record any inconsistencies as additional Issues. Then output the structured report using the template below — the scan result MUST appear as a `Cross-skill consistency:` line in the report's Alignment Check section (either listing the scanned skills with "no inconsistencies" or naming the inconsistencies surfaced as Issues). When no triggers fire, write `Cross-skill consistency: not triggered (no findings affect shared surfaces).` Before final output, scan each Suggestion line for the literal ` or ` (with surrounding spaces); for each match, decide whether the "or" offers reader-facing alternative techniques (keep as-is) or names an implementer choice between distinct edit sites/rewrites (pick a primary and add an `Alternative:` sub-line per the "Single-change Suggestions" rule below). This catches rule violations at audit time rather than at implementation time.

## Report Template

Output this structure to the conversation (do not write to a file):

```markdown
# Skill Audit: <skill-name>

**Skill path**: <path>
**Session date**: YYYY-MM-DD
**Session summary**: <1-2 sentence description of the session work that exercised the target skill>

## Alignment Check

- **FOUNDATIONS.md**: <aligned / N violations found>
- **CLAUDE.md**: <aligned / N deviations found>
- **Cross-skill consistency**: <not triggered (no findings affect shared surfaces) / Scanned: skill-X, skill-Y — no inconsistencies / Scanned: skill-X — N inconsistencies surfaced as Issues>
[If violations: bullet list with specific foundation # or CLAUDE.md section + what conflicts]

## Issues

[If none: "No issues identified."]

1. **[I1] [SEVERITY]** <title>
   - **What happened**: <session evidence — what went wrong or was confusing>
   - **Skill gap**: <what the skill says or fails to say that caused this>
   - **Suggestion**: <how to fix the skill>

## Improvements

[If none: "No improvements identified."]

1. **[M1] [SEVERITY]** <title>
   - **Current behavior**: <what the skill currently says>
   - **Why improve**: <session evidence or reasoning>
   - **Suggestion**: <proposed change>

## Features

[If none: "No features identified."]

1. **[F1] [SEVERITY]** <title>
   - **What's missing**: <gap description>
   - **Why it fits**: <how this aligns with the skill's stated intent>
   - **Suggestion**: <proposed addition>

## Not Exercised This Session

[Optional section. Omit entirely when all skill steps and branches were exercised. Otherwise list one-line bullets naming skill steps or branches that were *not reached at all* during the session — this surfaces coverage gaps without speculating about them as issues. Branches that were reached and correctly resulted in a no-op (e.g., a conditional flag whose condition evaluated as false, a threshold checkpoint whose count landed below the firing threshold, a checklist item that was checked and passed) count as exercised — they verified the decision logic. Exclude them from this section. List up to ~10 major unexercised branches. Skip listing trivial conditionals (single flag checks within a sub-step) — focus on top-level phases, named sub-paths, and strategy choices.]

- <one-line description of skill step or branch not exercised>

## Summary

**Total**: N issues, N improvements, N features (N findings total) — N CRITICAL, N HIGH, N MEDIUM, N LOW
```

Double-check severity counts against findings before presenting. If a correction is needed after presenting, strike the incorrect line and restate.

**Single-change Suggestions (no "or" alternatives)**: Each Suggestion must propose exactly one concrete change. If the audit surfaces a genuine either/or tradeoff that cannot be resolved during the audit itself, choose one direction as the primary Suggestion and list the discarded direction as an `Alternative:` sub-line beneath it (e.g., `- **Alternative**: <one-line description of the other approach>`). Follow-up implementation applies the primary unless the user explicitly names the alternative. This prevents ambiguity during "implement all" runs, where the audit executor would otherwise have to make a unilateral choice with no user input. Scope: an "or" inside the proposed new prose offering complementary techniques to the *reader* is fine and does not require the `Alternative:` machinery; an "or" that leaves the *implementer* choosing between two distinct edit sites, two distinct rewrites, or two distinct sections to insert into is a violation. The violating case is the only one that needs the primary-plus-`Alternative:` treatment.

**Optional `recommended` tag**: Append ` — recommended` to the title line of any finding you want included when the user says "implement recommended." Untagged findings are treated as optional and excluded from "implement recommended" scope (but still included when the user says "implement all"). Example: `1. **[M2] [MEDIUM]** Tighten batching threshold — recommended`. If no findings are tagged, "recommended" falls back to "all" per the follow-up-implementation rule. The tag also functions as an author-curated priority signal for the audit reader, independent of the implementation pathway — use it to mark the subset you would prioritize if forced to choose. Worked example: an audit produces M1, M2 (tagged `— recommended`), M3. User invokes "implement recommended" → M2 only. User invokes "implement all" — or any ambiguous-plural invocation like "implement suggestions", "do them", "apply the findings" — → M1, M2, M3 (the tag is included in scope but does not narrow it).

If a finding's conclusion is that no change is needed (the current behavior is sufficient, or the gap is too minor to act on), append "— no change needed" to the Suggestion line. This marks the finding as informational and excludes it from "implement all/recommended" scope during follow-up implementation.

## Guardrails

- **Report only** — Never modify the target skill file. Output the report to the conversation only.
- **No false positives** — If a step in the skill wasn't exercised during the session, note "not exercised this session" rather than speculating about potential issues.
- **FOUNDATIONS alignment is mandatory** — Any suggestion that would violate a principle in `docs/FOUNDATIONS.md` must be flagged and rejected, even if it would otherwise be an improvement.
- **Scope discipline** — Do not propose expanding the skill's scope beyond its stated intent. The audit evaluates the skill as written, not what it could become.
- **Session evidence required** — Every Issue and Improvement must cite specific session evidence (what happened, what was expected). Findings based purely on hypothetical scenarios belong in Features, not Issues.
- **Follow-up implementation** — After the report is presented, the user may request implementation of specific suggestions. At that point, edit the target skill file directly — the "report only" guardrail applies only to the audit phase, not to user-directed follow-up. Task tracking is optional during the edit batch — appropriate for very large batches (>15 edits or multi-file refactors), skippable for tight single-file batches. Default to skipping unless the implementation will span multiple turns or qualifies as a "very large batch" per the preceding sentence (>15 edits or a multi-file refactor). A small multi-file batch (e.g., 5 edits across 2 files) is not by itself a multi-file refactor and defaults to skipping; ignore any auto-fired task-tracking reminders that contradict this rule when the batch is tight.

  **Before the first Edit call** — two pre-edit greps are required, in order:
  1. The **Pre-apply text-existence check** (anchor greps for each finding) — see sub-section below.
  2. If any finding triggers a **Cascade edit** (renumbering a list, prose-level concept rewrite, terminology rename, or a primary edit that would render related text inconsistent), also run the cascade **Pre-edit grep** — see the Cascade edits sub-section below.

  Both must succeed before any Edit call. Skipping either risks landing edits that look correct in isolation but leave the file inconsistent.

  **Re-evaluation**: If the codebase or the target skill file changed between the audit report and the follow-up request, re-evaluate each finding against the current state. Discard obsolete findings, adapt shifted ones, and renumber survivors before applying edits.

  **Pre-apply text-existence check**: Before the first Edit call, grep for one content-tied anchor per finding (a distinctive phrase from the finding's `old_string` target) and confirm it resolves to the expected location. These may be batched into a single grep call with `|` alternation when the anchors are textually distinct enough that overlap is not a risk — one grep per finding is the alternative when anchors share substrings or when batched output would be hard to interpret. If an anchor is missing or has moved, invoke Re-evaluation for that finding before proceeding. For purely additive edits (no existing text being replaced), a grep of the intended insertion context is sufficient. A same-session full Read of the target file may substitute for the per-finding grep when the Read occurred *within the current task phase* — defined as: no intervening context compression, no summarization, and no logical phase transition. The audit phase and the implementation phase count as separate task phases even when they are adjacent turns apart; a Read performed during the audit phase does NOT substitute for an implementation-phase grep. When the Read is from a prior task phase, do the explicit grep regardless of whether the text is still in-buffer. When in doubt, or if the file was only partially read, do the explicit grep.

  **Partial implementation**: If the user requests specific findings (e.g., "implement 1 and 3"), check whether skipped findings depend on implemented ones. If so, note the dependency and ask whether to include the dependent finding. If the user requests all findings be implemented (e.g., "implement all", "implement everything"), skip dependency checking and apply all edits in document order. If the user requests "recommended" (e.g., "implement recommended suggestions"), select only findings whose title line is tagged `— recommended`; if no findings are tagged, fall back to "all." For the "recommended" path, also run the same dependency check used for specific-findings requests — flag any untagged finding that an implemented tagged finding depends on, and ask whether to include the dependent finding. Findings that the audit report explicitly marks as "no change needed" or "no change — existing behavior is sufficient" are excluded from "all" and "recommended" scope.

  **Ambiguous plural verbs**: If the request uses a plural-but-ambiguous phrasing ("implement suggestions", "apply the recommendations", "do them", "implement the findings"), default to "all" rather than "recommended." The keyword "recommended" is the only trigger for the tagged-subset path — any other plural verb that doesn't explicitly name the tag convention resolves to "all." This prevents two defensibly-equivalent interpretations from diverging across invocations.

  **Post-implementation scope shifts**: When a request uses verbs like "fix", "apply", or "address" that point at items established by the assistant's own post-implementation summary (e.g., "out-of-scope inconsistencies surfaced", "cascade through layer files noted as next step") rather than at the audit's findings, treat the named items as the new scope. The verb-disambiguation defaults above only govern requests that target the audit's findings; once a follow-up wave has produced its own summary callouts, those callouts become the addressable surface for any further verb that points back at "what was just listed". Confirm the interpretation in one sentence before editing if ambiguity remains.

  **Edit ordering**: Apply edits in document order (top to bottom within each file) to minimize line-number shifts invalidating later edits. For multi-file batches, file-to-file order is unconstrained because different files don't share line numbers — order by finding number is acceptable. Within a single file, earlier-line edits always go first so that insertions above don't invalidate `old_string` anchors in later edits targeting the same file. If applying an earlier finding renders a later finding moot (e.g., the target text no longer exists), skip the moot finding and note it in the post-edit verification as "superseded by finding N." If an Edit call fails because a prior edit changed the target text, re-read the file to find the updated text and retry with the corrected `old_string`. If an edit inserts a new numbered step, renumber all subsequent steps and verify that the output summary or other sections referencing step numbers are updated accordingly. When an edit inserts or removes a numbered item, grep the entire target file for references to the affected step numbers (e.g., `Step 7`, `Step 8`, `Step 9` for a Step-7 insertion; this rule applies to any renumbered list, including items nested within a step — for those, grep for `item N`, `sub-step N`, `Step M item N`, and similar patterns appropriate to the file's reference convention) and queue a cascade edit for each stale reference before the first Edit call — not just the refs visible in preceding planned edits. Apply those cascade edits in the same batch or immediately after. When a single finding requires multiple `replace_all` calls whose old and new values overlap on a number range (e.g., Section 3→4, 4→5, 5→6 in a renumbering cascade), apply them in highest-to-lowest order so that no single replacement double-bumps a value that an earlier call just renamed.

  **Cascade edits**: When a primary edit creates inconsistency or dead code elsewhere in the file, queue a cascade edit alongside it. Apply the following sub-rules in order:

  - **What counts as a cascade trigger**: Related text uses the same terminology, references the same concept, would become inconsistent if only the finding's target text is changed, or — for reachability cascades — a parent guard, skip rule, or precondition excludes the case the primary edit is adding. Without the cascade, the primary edit's new text is dead code or contradicts an unchanged sibling. A recurring pattern worth naming explicitly: **duplicated list-in-summary** — if a numbered or bulleted list in the main workflow is also restated as a one-sentence summary in a Guardrails, Quick-reference, or Overview section, modifying the list requires cascading to the summary sentence.
  - **Pre-edit grep (REQUIRED before the first Edit call)**: For content-based cascades that share a target file with the primary edit (terminology rename, dump-section renumbering, identifier rename, prose-level concept rewrite), grep the file for the affected pattern to enumerate every site. Queue all primary and cascade edits in one batch. Do not rely on post-edit re-reads to discover missed sites — the post-edit verification's job is to confirm the batch landed coherently, not to find sites the pre-edit grep should have caught.
  - **Plural-variant grep**: When the target term has a plural variant ("Sections X and Y", "Sections 1, 2, and 3", "X-series and Y-series"), grep for both the singular and the plural patterns in the same pre-edit pass.
  - **Plural-form Edit mechanics**: Plural forms typically require manual single-Edit calls because `replace_all` is a literal-substring match and will not pick up digits that appear separated from the noun by other words.
  - **Summary tagging**: Note cascade edits in the post-implementation summary as "cascade from finding N."

  **Post-implementation summary**: After all edits, present a summary table showing the status of each finding: "implemented" (optionally with a brief anchor suffix, e.g., "implemented (line 149)" or "implemented (frontmatter)"), "implemented (variation: <one-line>)" (when the literal Suggestion proved unworkable during edit — e.g., broke markdown rendering, conflicted with existing structure, hit a syntax constraint — and the implementer applied a functionally-equivalent change preserving the finding's intent), "superseded by finding N", "cascade from finding N", "co-edit with finding N", or "skipped (reason)". This gives the user a clear per-finding status rather than requiring them to infer outcomes from the edit sequence. The table uses three canonical columns:

  1. `finding-id` — the audit's original key (e.g., `I1`, `M2`, `F1`), plus any sub-keys `N.a`/`N.b` (one finding, multiple primary edits) or `N.cascade` (consistency edit derived from finding N) per the sub-keying rules below. The prefix letter maps the finding's category: `I` for Issue, `M` for Improvement (i**M**provement), `F` for Feature. A report producing 2 Issues and 1 Improvement keys them `I1`, `I2`, `M1` regardless of severity. Always prefix by category — plain numeric keys (`1`, `2`, `3`) are not allowed, even when only one category has findings.
  2. `finding-title-short` — a 5–10 word restatement of the finding title, or the original title if already short.
  3. `status` — one of the enumerated status values above.

  List cascade edits as separate table rows keyed `N.cascade` (where N is the originating finding's number); the status column contains `cascade from finding N — <one-line reason>`. If a single finding requires multiple primary edits in different sections of the target skill (neither edit is a cascade of the other — both are co-equal owners of the suggestion), key them `N.a`, `N.b`, etc. Reserve `N.cascade` for edits that merely keep related text consistent with a primary change. Multiple consistency edits from a single primary may be rolled into one cascade row when they share a single purpose (e.g., a batch of step-number cross-reference updates); split into separate `N.cascade` rows only when the consistency concerns are distinct (e.g., one cascade for numbering updates, a separate cascade for terminology renames). When two or more separate findings are resolved by a single shared text edit (their Suggestions collapse onto the same passage), keep each finding as a distinct row in the summary table — key each by its original finding number and use the status `implemented (co-edit with finding M)` on each row to name the partners. Do NOT re-key them as sub-letters; `N.a`/`N.b` is reserved for one-finding-multiple-edits only. This preserves the finding-to-edit relationship so a reader of the status table can reconcile it against the audit's finding count. This keeps per-edit granularity consistent across audits.

  **Post-edit verification**: After all edits are applied, re-read all edited files (the main SKILL.md and any reference files that were modified) and verify as a single pass:
  1. **No overlap or contradiction** — edits don't conflict with each other
  2. **Cross-references valid**:
     - (a) **Numbering continuity** — step, phase, and section numbers are sequential with no gaps or duplicates. If the file has >150 lines OR >10 numbered references across multiple levels, prefer grep pattern search (e.g., grep for `Step [0-9]`, `### [0-9]`) to confirm; otherwise a visual scan suffices. Adapt grep patterns to the target skill's convention (numbered items, lettered sub-steps, or markdown headers).
     - (b) **File paths valid** — all referenced file paths still exist and point to correct targets.
     - (c) **New cross-references** — references introduced by new text point to content that actually exists. When the target skill uses nested numbering (sub-steps within steps), verify that cross-references disambiguate between levels (e.g., "Step 1, sub-step 5" vs. "Step 5").
     - (d) **Overview diagrams** — high-level overviews that become slightly inaccurate due to new branching logic are acceptable if the detailed step text handles the nuance. Note the discrepancy but do not force-update overview text that would become harder to scan.
     - (e) **Rendering integrity** — for edits inside markdown tables, fenced code blocks, or nested lists, verify the enclosing structure remains parseable. Block elements (footnote definitions, headers, blockquotes, blank lines) inserted between table rows split the table; new content inside a table cell must be inline (links, code spans, plain text, soft line breaks via `<br>`). Fenced code blocks must remain balanced (opening and closing fences match). Nested list indentation must be preserved or the sub-list collapses into a sibling.
  3. **Sequential flow coherent** — the skill reads coherently end-to-end after all edits
  4. **Contextual consistency** — numbering, terminology, and cross-references are consistent with adjacent unchanged text
  5. **Frontmatter integrity** — if any edit touched the YAML frontmatter, verify `---` delimiters are intact and the YAML parses correctly (name, description, and arguments are present and properly quoted)

  If any check fails, fix the offending edit(s), then re-run the full 5-check pass. Do not selectively re-check — a fix in one area can introduce issues in another.

  **Additive-only shortcut**: For edit batches that are purely additive — new text inserted into existing structure with no renumbering, no cross-reference rewrites, no frontmatter touches, and no modifications to existing sentences — a grep-for-anchors verification may substitute for the full 5-check re-read pass. Grep for one content-tied anchor per edit (a distinctive phrase from the new text) and confirm each appears exactly once. Additive insertions introduce unique text per edit, so the expected count is always 1; if any anchor returns more than one match, the chosen phrase wasn't distinctive enough — pick a longer phrase and re-grep. If any edit renumbers a list, rewrites or removes an existing cross-reference, alters frontmatter, or modifies existing sentences (not just appends), the full 5-check pass is required. Adding new cross-references inside the new additive prose is purely additive and does not disqualify the shortcut — only modifying or removing pre-existing cross-references does.
- **Cross-skill consistency** — If the target skill is part of a multi-skill workflow AND any finding affects interfaces shared with sibling skills, scan sibling skills for inconsistencies. Report cross-skill inconsistencies as Issues. When all findings are internal to the target skill (no triggers fire), no scan is needed — but Step 7 still requires the `Cross-skill consistency: not triggered` line in the report so that absence of scan is auditable.

  **Concrete shared-surface triggers**: A finding affects a shared surface when it changes any of the following. Each trigger is tagged with the kind of scan it warrants — `[file-convention]` (grep sibling skills for writes to the same file), `[schema]` (check producer/consumer schema contracts), `[downstream-format]` (read the consumer skill's input contract), `[archival]` (cross-reference archival workflow doc), `[numbering-convention]` (check the registry skill that owns the convention), `[shared-term]` (grep sibling skills for the term), or `[output-path]` (check overlapping write paths):
  - **[file-convention]** `specs/IMPLEMENTATION-ORDER.md` content conventions (wave naming, phase placement, derivation preamble format, dependency-graph notation)
  - **[schema]** `MEMORY.md` entry schema or frontmatter fields
  - **[downstream-format]** `docs/generated/*` output format (golden inventory, coverage matrix, scenario details)
  - **[archival]** `archive/*/` archival destination paths or naming conventions
  - **[numbering-convention]** Spec or ticket numbering conventions (S-series, ticket ID prefixes)
  - **[output-path]** Report-file output paths (`reports/*.md`, `docs/triage/*.md`)
  - **[shared-term]** Shared terminology that appears in multiple skills (e.g., "Adjunct Wave", "Phase Gate", "Damning Moment", "Pre-flight Warning")
  - **[downstream-format]** Output format consumed by a downstream skill in the same pipeline (e.g., observer dump consumed by `/scenario-analysis`, scenario report consumed by `/simulation-remediation`)

  When triggered, list the scanned sibling skills in the audit report alongside what (if anything) was adjusted. If no inconsistency was found, state "Scanned: <skill list> — no inconsistencies." This makes the scan auditable.
- **Repeated audit shortcut** — If the same skill has been audited *as the target* 2+ times in the current session and the most recent audit found 0 findings, note "Skill stable — no new session evidence since last audit" and skip the full checklist unless the skill was modified between audits. If the skill was modified since the last audit (including by follow-up implementation from a prior audit), treat the next audit as fresh — do not use the shortcut.
