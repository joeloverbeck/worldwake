---
name: skill-consolidate
description: "Consolidates a skill's SKILL.md by removing redundancies, regrouping fragmented topics, improving readability, and clarifying decision paths — while preserving every unique instruction."
user-invocable: true
arguments:
  - name: skill-path
    description: "Path to a skill directory (targets its SKILL.md) or a direct path to a markdown file under references/ (e.g., .codex/skills/implement-ticket or .codex/skills/implement-ticket/references/closeout.md)"
    required: true
---

# Skill Consolidate

Structural consolidation for skill files that have grown organically through iterative skill-audit improvement cycles. Removes redundancy, regroups fragmented topics, restructures for readability, and clarifies scattered decision paths — while preserving every unique instruction.

Complements skill-audit: skill-audit reports issues and suggests additions (growth). skill-consolidate removes structural entropy (pruning). Together they form a quality cycle.

## Invocation

```
/skill-consolidate <skill-path>
```

**Arguments** (required, positional):
- `<skill-path>` — path to either a skill directory (targets `<skill-path>/SKILL.md`, e.g., `.codex/skills/implement-ticket` or `.claude/skills/reassess-spec`) OR a direct path to a markdown file under `<skill-path>/references/` (e.g., `.codex/skills/implement-ticket/references/closeout.md`). Reference files are valid consolidation targets; the frontmatter-preservation guardrail is vacuous for them since reference files have no frontmatter.

If the argument is missing, ask the user to provide it before proceeding.

## Worktree Awareness

If working inside a worktree (e.g., `.claude/worktrees/<name>/`), ALL file paths — reads, writes, globs, greps — must use the worktree root as the base path.

## Process

Follow these 9 steps in order. Do not skip any step.

---

### Step 1: Read & Parse

Resolve the target file from `<skill-path>`:
- If `<skill-path>` ends in `.md`, treat it as a direct file target and read that file. The frontmatter-preservation guardrail is vacuous (reference files have no frontmatter); all other process steps apply normally.
- Otherwise, treat `<skill-path>` as a skill directory and read `<skill-path>/SKILL.md`.
- If the resolved file does not exist, stop and report the error.

Parse into logical blocks:
- **Frontmatter**: YAML metadata between `---` delimiters (SKILL.md targets only)
- **Sections**: Top-level headings (`##`) and their content
- **Subsections**: Nested headings (`###`, `####`)
- **Instruction lists**: Bullet points, numbered lists, inline directives
- **Guardrails**: Typically the final section with constraint bullets

Measure baseline with `wc -lc <target-file>` (or equivalent) and record the exact line and character counts. These numbers feed the Step 9 summary verbatim; do not estimate.

The "before" baseline for Step 9 metrics is the file content as read in this step, regardless of git state.

---

### Step 2: Redundancy Detection

Scan for instructions that appear in multiple locations with the same semantic content. Two instructions are redundant if removing either would leave the other sufficient — even if the wording differs.

For each redundancy cluster:
1. **Identify all instances**: Note the instruction text and its location (section + approximate position)
2. **Pick the canonical location**: Where the instruction is most contextually relevant (e.g., a "reassess before coding" instruction belongs in the reassessment section, not in guardrails)
3. **Mark non-canonical instances for removal**
4. **Record** the cluster for the diff summary

Common redundancy patterns to watch for:
- Principle restated in introduction, section body, AND guardrails
- The same corrective action described in multiple workflow phases
- "Do not X" warnings scattered across unrelated sections
- File/field lists repeated in multiple contexts (e.g., "update Files to Touch, Verification Layers, Test Plan" appearing 3+ times)

When the same technique or surface list appears at multiple workflow phases with genuinely different purposes (check vs. include in scope vs. implement), define the shared content once at its most natural location and replace other instances with cross-references. This is not pure redundancy removal — it is factoring out a shared reference.

---

### Step 3: Topic Regrouping

Identify instructions about the same topic that are scattered across multiple sections. A topic is fragmented when a reader must jump between multiple sections to get the full picture — typically 3+, but 2 sections suffice when one location is clearly the wrong home.

For each fragmented topic:
1. **Collect all instructions** related to that topic from all sections
2. **Choose the natural home**: The workflow phase where the topic is most relevant
3. **Create a dedicated subsection** (or extend an existing one) at that location
4. **Move instructions** from their scattered locations into the consolidated subsection
5. **Record** the regrouping for the diff summary

Do not change the overall section ordering (workflow phases stay in sequence). Only move content within or between sections.

---

### Step 4: Readability Restructuring

Identify wall-of-text patterns that hurt scannability:
- **Long flat lists**: 20+ consecutive bullets without sub-headings or grouping
- **Dense paragraphs**: Paragraphs with 5+ distinct instructions packed together
- **Missing hierarchy**: Sections that mix high-level principles with low-level details at the same indent level

For each pattern:
1. **Introduce sub-headings** that group related bullets by theme (e.g., "Type-change checks", "Serialization checks", "Test fixture checks")
2. **Add tiered structure**: High-level summary first, details nested beneath
3. **Break dense paragraphs** into focused bullets or numbered steps
4. **Record** the restructuring for the diff summary

---

### Step 5: Decision Path Clarification

Identify escalation or branching instructions that are mentioned repeatedly without unified guidance. Signs: the same "when you hit X" pattern appears in 3+ places, each with slightly different advice or no clear resolution path.

For each scattered decision path:
1. **Collect all mentions** of the decision/escalation pattern
2. **Unify** into a single explicit structure: a decision table, a "when X → do Y" list, or a flowchart-style description
3. **Place** the unified structure at the most relevant workflow phase
4. **Replace scattered mentions** with brief cross-references to the unified structure. Use descriptive section references (e.g., "see Section 3, Escalation decision tree") over bare section numbers, since numbers may shift in future consolidation passes.
5. **Record** the clarification for the diff summary

**Cross-reference hygiene**: Repairing broken or obsolete cross-references (e.g., a "see Section 1" pointer to a section that no longer exists in the current file, or references to content that has moved to another file) is in-scope consolidation hygiene, not scope expansion. Log each repair in the Step 9 summary under "Cross-reference Hygiene" so the change is visible in the diff surface.

---

### Step 6: Tighten Wording

For non-redundant instructions (those surviving Steps 2-5), tighten prose:
- Shorten sentences without losing meaning
- Remove filler words ("it is important to note that" → remove)
- Prefer active voice ("The ticket should be updated" → "Update the ticket")
- Eliminate hedging where the instruction is unconditional ("You should always check" → "Check")
- Compress repeated structural patterns ("When X, do Y. When X2, do Y2." → table or compact list)

**Critical constraint**: Never change the meaning of an instruction. If unsure whether tightening alters meaning, keep the original wording.

---

### Step 7: Rewrite

Before writing, briefly summarize planned changes in the conversation so the user sees what will change before the file is overwritten.

Write the consolidated SKILL.md in-place at `<skill-path>/SKILL.md`.

The rewritten file must:
1. **Preserve frontmatter exactly** — do not modify name, description, arguments, or any YAML field
2. **Maintain workflow phase ordering** — if the original has phases 1-7 in sequence, the consolidated version keeps the same logical sequence
3. **Contain every unique instruction** — deduplicated, regrouped, tightened, but present
4. **Use the same markdown conventions** — heading levels, list styles, code block formatting consistent with the original

---

### Step 8: Spot-Check Preservation

After writing, spot-check 3-5 unique instructions from the original (preferring instructions from different sections) to confirm each survives in the consolidated output. If any instruction was lost, restore it before presenting the diff summary.

---

### Step 9: Diff Summary

After writing, present a structured summary in the conversation:

```
## Consolidation Summary: <skill-name>

**Size**: <before chars> → <after chars> (<change>%)
**Lines**: <before> → <after> (may increase when dense paragraphs are broken into structured lists)

### Redundancies Merged (<count>)
- "<instruction summary>" — was in <N> locations, canonical: <section name>

### Topics Regrouped (<count>)
- "<topic>" — consolidated from <source sections> into <target section>

### Sections Restructured (<count>)
- "<section>" — <what changed> (e.g., "53 bullets → 5 themed sub-groups")

### Decision Paths Clarified (<count>)
- "<topic>" — unified from <N> mentions into <structure type>

### Wording Tightened
- <N> instructions shortened for conciseness (no semantic changes)
- Examples: "<before>" → "<after>" (include 2-3 representative samples)

### Cross-reference Hygiene (<count>)
- "<before>" → "<after>" — reason (e.g., "Section 1 no longer exists in this file; target now lives in SKILL.md Step 1")

### Observations (if any)
[Gaps noticed during consolidation that were not filled (per No Scope Expansion guardrail).]
```

Do NOT commit. Leave the file for user review via `git diff`.

---

## Guardrails

- **Semantic preservation**: Every unique instruction in the original must survive in the output. Redundant copies are removed; the canonical instance remains. When in doubt about whether two instructions are truly redundant, keep both.
- **Frontmatter untouched**: Never modify the YAML frontmatter (name, description, arguments, user-invocable, or any other field).
- **No scope expansion**: Do not add new instructions, features, guardrails, or edge-case handling. This is consolidation, not improvement. If you notice a gap, mention it in the diff summary under a "Observations" section but do not fill it.
- **No commit**: Write the file and stop. The user handles the file lifecycle.
- **Worktree discipline**: If working in a worktree, ALL file operations use the worktree root path.
- **Both skill locations**: Works on skills in `.claude/skills/`, `.codex/skills/`, or any other path the user provides.
- **Idempotency**: Running the skill twice on the same file should produce minimal or no further changes. If a skill is already well-consolidated, say so and make no edits.
