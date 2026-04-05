---
name: skill-consolidate
description: "Consolidates a skill's SKILL.md by removing redundancies, regrouping fragmented topics, improving readability, and clarifying decision paths — while preserving every unique instruction."
user-invocable: true
arguments:
  - name: skill-path
    description: "Path to skill directory (e.g., .codex/skills/implement-ticket or .claude/skills/reassess-spec)"
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
- `<skill-path>` — path to skill directory (e.g., `.codex/skills/implement-ticket` or `.claude/skills/reassess-spec`)

If the argument is missing, ask the user to provide it before proceeding.

## Worktree Awareness

If working inside a worktree (e.g., `.claude/worktrees/<name>/`), ALL file paths — reads, writes, globs, greps — must use the worktree root as the base path.

## Process

Follow these 8 steps in order. Do not skip any step.

---

### Step 1: Read & Parse

Read the target `SKILL.md` from `<skill-path>/SKILL.md`. If the file does not exist, stop and report the error.

Parse into logical blocks:
- **Frontmatter**: YAML metadata between `---` delimiters
- **Sections**: Top-level headings (`##`) and their content
- **Subsections**: Nested headings (`###`, `####`)
- **Instruction lists**: Bullet points, numbered lists, inline directives
- **Guardrails**: Typically the final section with constraint bullets

Count:
- Total lines (including blank lines)
- Unique instructions (distinct semantic directives, regardless of where they appear)

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

---

### Step 3: Topic Regrouping

Identify instructions about the same topic that are scattered across multiple sections. A topic is fragmented when a reader must jump between 3+ sections to get the full picture.

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
4. **Replace scattered mentions** with brief cross-references to the unified structure (e.g., "See Section N for contradiction handling")
5. **Record** the clarification for the diff summary

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

Write the consolidated SKILL.md in-place at `<skill-path>/SKILL.md`.

The rewritten file must:
1. **Preserve frontmatter exactly** — do not modify name, description, arguments, or any YAML field
2. **Maintain workflow phase ordering** — if the original has phases 1-7 in sequence, the consolidated version keeps the same logical sequence
3. **Contain every unique instruction** — deduplicated, regrouped, tightened, but present
4. **Use the same markdown conventions** — heading levels, list styles, code block formatting consistent with the original

---

### Step 8: Diff Summary

After writing, present a structured summary in the conversation:

```
## Consolidation Summary: <skill-name>

**Lines**: <before> → <after> (<reduction>%)

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
