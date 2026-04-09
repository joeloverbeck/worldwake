---
name: skill-extract-references
description: Extract logically grouped content from a bloated SKILL.md into references/ docs, rewriting the skill as a thin entry point. Argument: path to the skill directory (e.g., .codex/skills/implement-ticket or .claude/skills/improve-loop).
---

# Skill Extract References

Refactor a skill by extracting large, logically grouped content blocks into `references/` docs and rewriting the SKILL.md as a thin orchestration entry point.

**Argument**: A skill directory path (e.g., `.claude/skills/implement-ticket`). The skill locates `SKILL.md` inside it automatically.

## Procedure

### 1. Read Inputs

- Read `<skill-dir>/SKILL.md` in full.
- List `<skill-dir>/references/` if it exists. Read every existing reference doc to understand what is already extracted.

### 2. Early Exit Check

If the SKILL.md is under 60 lines, output "Nothing to extract — SKILL.md is already thin (N lines)." and stop.

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

- **Conditional reference** — a block gated by conditional language in the original text. Look for these markers:
  - "if", "when", "only when", "for tickets that touch", "when the change involves"
  - Blocks nested under a conditional header or prefaced by conditional language.
  - The condition from the original text becomes the loading instruction in the thin SKILL.md.

**When ambiguous**: default to always-loaded. It is safer to load too much than to miss instructions that should have applied.

### 5. Group and Name

- Merge blocks that share a logical theme into a single reference doc. Do not create one reference per H3 — group by coherent topic.
- Use kebab-case descriptive filenames: `verification-and-closeout.md`, `ai-pipeline-checks.md`, `golden-reassessment.md`.
- If an existing reference doc in `references/` covers the same theme, merge the extracted content into it rather than creating a duplicate.

### 6. Write Reference Docs

- Create `<skill-dir>/references/` if it does not exist.
- Write each reference doc with:
  - An H1 title describing its purpose.
  - The extracted content, preserving its original structure (headers, lists, code blocks).
- Do not add frontmatter to reference docs — they are plain markdown loaded by the thin SKILL.md.

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

### 8. Output Summary

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
