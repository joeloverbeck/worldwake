---
name: skill-extract-references
description: Extract logically grouped content from a bloated SKILL.md into references/ docs, rewriting the skill as a thin entry point. Argument: path to the skill directory (e.g., .codex/skills/implement-ticket or .claude/skills/improve-loop).
---

# Skill Extract References

Refactor a skill by extracting large, logically grouped content blocks into `references/` docs and rewriting the SKILL.md as a thin orchestration entry point.

**Argument**: A skill directory path (e.g., `.claude/skills/implement-ticket`). The skill locates `SKILL.md` inside it automatically.

## Procedure

### 1. Read Inputs

- Resolve the argument to an absolute path. Confirm `<skill-dir>/SKILL.md` exists before proceeding. If the skill directory contains no SKILL.md, stop and report the error.
- Read `<skill-dir>/SKILL.md` in full. If the file exceeds Read tool limits, read in chunks using offset/limit. Ensure complete coverage before proceeding to Step 3.
- List `<skill-dir>/references/` if it exists. Read every existing reference doc to understand what is already extracted.

### 2. Early Exit Check

If the SKILL.md is under 60 lines, output "Nothing to extract — SKILL.md is already thin (N lines)." and stop. Also exit if the SKILL.md already contains 3+ load instructions pointing to `references/` — it is already in thin form.

### 3. Parse into Blocks

Split the SKILL.md into logical blocks using markdown structure:
- H2 (`##`) and H3 (`###`) headers define block boundaries.
- Numbered list groups and fenced code blocks within a header section belong to that block.
- The YAML frontmatter is always **core** — never extracted.
- The top-level title (H1) and any immediately following paragraph before the first H2 is **core**.

### 4. Classify Each Block

For each block, determine one of three categories:

- **Core** — stays inline in the thin SKILL.md. This includes:
  - The frontmatter and H1 title.
  - The top-level workflow/procedure steps (the numbered orchestration sequence).
  - Universal hard rules that are short and apply to every invocation.

- **Always-loaded reference** — a self-contained block that applies to every invocation but is large enough (roughly 20+ lines) to warrant extraction. Examples: verification checklists, guardrails sections, outcome definitions.

- **Conditional reference** — a block gated by a **section-level loading condition** in the original text. Look for headers or introductory sentences that gate an entire section's applicability:
  - "If the change touches X, ...", "Only when Y applies, ...", "For tickets that involve Z, ..."
  - Blocks nested under a conditional header or prefaced by a section-level conditional sentence.
  - The condition from the original text becomes the loading instruction in the thin SKILL.md.
  - Note: Individual bullets that start with "When..." inside an always-applicable checklist are domain-specific conditional logic, not loading-condition gates. Do not classify an entire section as conditional just because its bullets use "when" language.

**When ambiguous**: default to always-loaded. It is safer to load too much than to miss instructions that should have applied.

### 5. Group and Name

- Merge blocks that share a logical theme into a single reference doc. Do not create one reference per H3 — group by coherent topic. If two adjacent original steps share a reference doc and form a natural unit, they may be merged into a single thin step. Preserve the original step numbers in the heading (e.g., "Steps 5-6") for traceability.
- Use kebab-case descriptive filenames: `verification-and-closeout.md`, `ai-pipeline-checks.md`, `golden-reassessment.md`.
- If an existing reference doc in `references/` covers the same theme, merge the extracted content into it rather than creating a duplicate.

### 6. Write Reference Docs

- Create `<skill-dir>/references/` if it does not exist.
- Write each reference doc with:
  - An H1 title describing its purpose.
  - The extracted content, preserving its original structure (headers, lists, code blocks).
- Do not add frontmatter to reference docs — they are plain markdown loaded by the thin SKILL.md.
- When extracted content contains relative paths (links, image references), adjust them to account for the extra directory depth of `references/`.

### 7. Rewrite Thin SKILL.md

- **Preserve** the YAML frontmatter exactly as-is.
- **Preserve** the H1 title.
- Write the core workflow as a numbered list of steps. Each step is either:
  - An inline instruction (for core content that stayed), or
  - A load instruction pointing to a reference file:
    - Unconditional: "Load `references/verification-and-closeout.md`."
    - Conditional: "If the change touches AI pipelines, load `references/ai-pipeline-checks.md`."
- **Preserve** universal hard rules as a short section at the bottom.
- The thin SKILL.md should read as a clear, scannable orchestration sequence — not a wall of checklists.
- Each step may include a brief framing sentence (1-2 sentences) before or after the load instruction to preserve workflow context (e.g., what the step's purpose is, what to do with results). For steps where the load instruction is the primary content, integrate it naturally (e.g., "Load `references/codebase-validation.md`. Validate every reference from Step 2.") rather than making it a standalone directive.

### 8. Verify Content Preservation

After rewriting, verify that every H2/H3 section from the original SKILL.md appears either in the thin SKILL.md or in a reference doc. A quick scan of original section headers against the new files is sufficient.

### 9. Cross-Skill Reference Check

Grep other skill files (`.claude/skills/*/SKILL.md`, `.codex/skills/*/SKILL.md`) for references to the target skill's section headers or step numbers. If found, note them in the output summary as "references that may need updating" so the user can fix external pointers.

### 10. Output Summary

Print a brief summary:
```
Extracted N reference docs. SKILL.md: X lines → Y lines.

References:
- references/foo.md (always)
- references/bar.md (conditional: when X)
- references/baz.md (always)
```

## Hard Rules

- Never modify the YAML frontmatter (name, description).
- Never discard content — every instruction from the original SKILL.md must appear either in the thin SKILL.md or in a reference doc.
- Merge into existing reference docs when themes overlap; do not create duplicates.
- Keep nested conditionals together in one reference doc — do not split below the natural grouping level.
- Default ambiguous blocks to always-loaded.
