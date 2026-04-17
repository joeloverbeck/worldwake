---
name: skill-rebalance-references
description: "Redistribute content across an already-split skill tree (SKILL.md + references/) by splitting overloaded references, moving content between references, and re-extracting accumulated bulk from SKILL.md. Use when an extract-references'd skill has re-bloated and needs structural rebalance (not prose tightening). Preserves every instruction."
user-invocable: true
arguments:
  - name: skill-path
    description: "Path to skill directory (e.g., .codex/skills/implement-ticket or .claude/skills/reassess-spec)"
    required: true
---

# Skill Rebalance References

Structural rebalance for skill trees that have re-bloated after a prior `skill-extract-references` pass. Redistributes content across `SKILL.md` + `references/*.md` through three operations: splitting overloaded references, moving content between references, and re-extracting accumulated bulk from `SKILL.md` — preserving every unique instruction.

Complements the sibling skills:
- `skill-extract-references` does the first-time monolith → thin + `references/` split.
- `skill-rebalance-references` (this skill) redistributes within an existing tree.
- `skill-consolidate` tightens prose within a single file (orthogonal concern).

Typical invocation flow when a skill has re-bloated: run this skill first (fix structure), then run `skill-consolidate` on each file whose content changed (tighten prose inside the newly-stable structure).

## Invocation

```
/skill-rebalance-references <skill-path>
```

**Arguments** (required, positional):
- `<skill-path>` — path to skill directory (e.g., `.codex/skills/implement-ticket` or `.claude/skills/reassess-spec`).

If the argument is missing, ask the user to provide it before proceeding.

## Worktree Awareness

If working inside a worktree (e.g., `.claude/worktrees/<name>/`), ALL file paths — reads, writes, globs, greps — must use the worktree root as the base path.

## Prerequisite

The target skill must already have a populated `references/` directory (produced by `skill-extract-references`). If `references/` is absent or empty, stop and direct the user to run `skill-extract-references` first. This skill does not handle first-time extraction.

## Process

Follow these 10 steps in order. Do not skip any step.

---

### Step 1: Read Inputs

- Resolve `<skill-path>` to an absolute path. Confirm `<skill-path>/SKILL.md` exists. If it does not, stop and report the error.
- Read `<skill-path>/SKILL.md` in full.
- List `<skill-path>/references/`. If the directory is missing or empty, stop and direct the user to run `skill-extract-references` first.
- Read every reference doc in full.
- Record line counts and character counts for `SKILL.md` and each reference. These are the "before" baseline for Step 10 metrics.

---

### Step 2: Eligibility Check (Early Exit)

Exit with "Nothing to rebalance — tree is already well-balanced" if ALL of the following hold:
- `SKILL.md` is ≤ 80 lines.
- Every reference is ≤ 150 lines (or ≤ 165 lines when the file covers one coherent topic and further splitting would create stubs or fracture tightly-related guidance — see Step 3 borderline tolerance).
- No reference covers ≥ 2 clearly distinct sub-topics (spot-check via H2/H3 heading diversity).

Otherwise, proceed.

---

### Step 3: Assess Structural Balance

Classify each file in the tree:

- **Overloaded reference**: > 150 lines of real content, OR covers 2+ distinct sub-topics identifiable by heading groups / workflow phases / orthogonal decision domains.
- **Underloaded reference**: < 20 lines of real content. Candidate for merge into another reference, not a split source.
- **SKILL.md re-bloat zones**: any workflow step containing > ~10 lines of dense directive content beyond framing + load instruction. Typical signs: embedded decision matrices, classification tables, long "when X, do Y" lists, accumulated audit-driven edge-case guidance.

**Borderline tolerance**: A file up to ~165 lines (≤ 10% over 150) that covers one coherent topic does NOT count as overloaded if further splitting would produce stubs or fracture tightly-related guidance. Classify it as acceptable and record the decision in Step 10 Observations so subsequent audits see it was a deliberate choice — this preserves idempotency when the next invocation reaches Step 2.

Record each flagged file/section with its classification. This list feeds Step 4.

---

### Step 4: Plan Operations

For each flagged item in Step 3, propose one or more of the three operations. Follow the priority order in Step 4d when multiple operations apply to the same file.

#### 4a. Split (for overloaded references)

Propose a split when **both** hold:
- Reference file > 150 lines of real content.
- File covers ≥ 2 distinct sub-topics, identifiable by one of:
  - Multiple H2/H3 groups with non-overlapping subject matter.
  - Distinct workflow phases or decision domains within one file.
  - Internal "when X, ..." / "when Y, ..." blocks where X and Y are orthogonal concerns (not variants of the same concern).

Propose kebab-case filenames derived from each sub-topic's subject matter (same convention as `skill-extract-references`).

**Categorizing ambiguous sections**: When a section could plausibly sit in multiple destination files, assign it to the destination most likely to be loaded together with related content — i.e., the file a future caller would load first for that ticket/task type. Avoid duplicating across destinations unless the section is genuinely dual-purpose; when dual-purpose, prefer a cross-reference over duplication.

**Do NOT split when**:
- File is long but covers one coherent topic (length alone is insufficient).
- The only "sub-topics" are variants of the same decision (e.g., "shared-field addition" vs "shared-field removal" are both shared-type fallout).
- A proposed split would produce a file under 20 lines of real content.

#### 4b. Move (between references)

Propose a move when a content block's subject matter is the direct topic of a different reference. Specifically:
- Block in ref A describes behavior/rules primarily belonging to ref B's declared topic.
- Block appears in A only because it was colocated with a related instruction at extraction time, not because A owns it.

**Do NOT move when**:
- The block is genuinely dual-purpose. Use a cross-reference; duplicate only if the two contexts require semantically distinct phrasing.
- Moving would leave the source reference incoherent. In that case, consider a split instead.

#### 4c. Re-extract (from `SKILL.md` into references)

Propose a re-extraction when a `SKILL.md` step contains dense, self-contained guidance beyond thin-orchestration form. Specifically:
- Step body has > ~10 lines of directive content beyond framing + load instruction.
- Step contains an embedded decision matrix, classification table, or long "when X, do Y" list.
- Step has accumulated audit-driven edge-case guidance that no longer reads as orchestration.

**Destination**:
- If the content thematically matches an existing reference → append there (re-extraction + merge).
- If it represents a new coherent topic → create a new reference file with a kebab-case filename.

**Do NOT re-extract when**:
- The content is genuinely orchestration (the numbered sequence itself, short framing sentences, load directives).
- The step is already thin (framing + load + brief post-load note).

#### 4d. Priority order when multiple operations apply

When a single file qualifies for multiple operations, apply in this order:
1. **Re-extract first** — pull `SKILL.md` bulk into references; this may change which references are overloaded.
2. **Split next** — once references have absorbed re-extracted content, split any still-overloaded.
3. **Move last** — redistribute across the now-stable set of files.

This ordering prevents thrash (e.g., splitting a reference before re-extracting new content into it would immediately require another rebalance).

**Destination-aware re-extraction shortcut**: When a planned re-extraction target is itself slated for splitting, plan the split first (naming the final destination files during Step 4a) and route re-extracted content directly to those destinations during Step 6. The priority order above still holds conceptually — re-extraction decisions come before split decisions in the plan — but when split destinations can be named up front, skip the temporary append-then-redistribute intermediate write. This avoids churn and makes Step 10 spot-checks easier to follow. If destination assignment is ambiguous at plan time, fall back to the default append-then-split sequence.

---

### Step 5: Present Plan for Approval

Print the proposed plan in structured form before writing any files. Use this format:

```
## Rebalance Plan: <skill-name>

### Re-extractions (<count>)
- "<SKILL.md step summary>" (~N lines) → references/<target>.md (new | append)

### Splits (<count>)
- references/<source>.md (N lines) → references/<new-a>.md + references/<new-b>.md
  - <new-a>: <sub-topic A summary>
  - <new-b>: <sub-topic B summary>

### Moves (<count>)
- Block "<summary>" : references/<source>.md → references/<destination>.md
```

Wait for user approval before proceeding. If the user requests adjustments, revise the plan and re-present.

---

### Step 6: Execute Re-extractions

For each planned re-extraction (executed first per Step 4d priority):
1. Write the extracted content to the target reference file. If appending to an existing reference, insert at the topically correct location (typically end of file, unless a specific section is the natural home).
2. In `SKILL.md`, replace the dense block with a load instruction using the same pattern `skill-extract-references` uses:
   - Unconditional: "Load `references/<name>.md`."
   - Conditional: "If <condition>, load `references/<name>.md`."
3. Retain a 1–2 sentence framing before or after the load instruction when workflow context requires it.

---

### Step 7: Execute Splits

For each planned split:
1. Write each new reference file with its sub-topic content. Preserve original structure (headers, lists, code blocks) within each split output.
2. Remove the split content from the source reference.
3. If the source reference still has coherent remaining content, leave a brief cross-reference line where the split content was (e.g., "See `references/<new-a>.md` and `references/<new-b>.md`.").
4. If the source reference is fully superseded by the splits, delete the source file and update every `SKILL.md` load instruction that pointed to it.

---

### Step 8: Execute Moves

For each planned move:
1. Append the moved block to the destination reference at the topically correct location.
2. Remove the block from the source reference.

---

### Step 9: Update Cross-References Inside the Skill

After Steps 6–8:
- In `SKILL.md`, verify every load instruction points to an existing reference file. Fix any paths broken by splits or deletions.
- In every reference, verify any "see `references/foo.md`" link still resolves. Update renamed/split targets.

---

### Step 10: Verify Preservation and Emit Summary

**Verify preservation**:
- Spot-check 5 unique instructions from the "before" state (sampled across `SKILL.md` + all original references). Confirm each still exists somewhere in the final tree. If any instruction was lost, restore it before emitting the summary.

**Cross-skill reference check**:
- Grep `.claude/skills/*/SKILL.md` and `.codex/skills/*/SKILL.md` (excluding the target skill itself) for references to any renamed, split, or deleted reference paths. Report external pointers that may need manual updating.

**Emit summary** in the conversation:

```
## Rebalance Summary: <skill-name>

### Size changes
- SKILL.md: <before> → <after> lines
- references/<name>.md: <before> → <after> lines
- references/<new-name>.md: NEW, <N> lines
- references/<deleted-name>.md: DELETED (fully split)

### Re-extractions performed (<count>)
- "<step summary>" → references/<target>.md

### Splits performed (<count>)
- references/<source>.md → references/<new-a>.md + references/<new-b>.md

### Moves performed (<count>)
- Block "<summary>": references/<source>.md → references/<destination>.md

### External reference warnings (<count>)
- <external-skill-path> references <renamed-or-deleted-path> — may need manual update

### Observations (if any)
[Redundancies or gaps noticed during rebalance that were NOT fixed per the No Scope Expansion guardrail. Flag for `skill-consolidate` or `skill-audit` follow-up.]
```

Do NOT commit. Leave files for user review via `git diff`.

---

## Guardrails

- **Semantic preservation**: Every unique instruction in the original state (across `SKILL.md` + all references) must survive somewhere in the final tree. When in doubt about whether two blocks are truly equivalent, keep both.
- **Frontmatter untouched**: Never modify `SKILL.md`'s YAML frontmatter (name, description, arguments, user-invocable, or any other field). References have no frontmatter and stay that way.
- **No scope expansion**: This skill redistributes existing content. It does not add new instructions, fill gaps, tighten prose, remove redundancy, or unify decision paths. If redundancy or gaps are noticed during rebalance, record them in the summary under "Observations" — do not fix them. Prose tightening is `skill-consolidate`'s job; gap filling is `skill-audit`'s job.
- **No deletion without relocation**: Content removed from a source file must land in a destination file. The only exception is cross-reference stubs when a split fully supersedes the source.
- **Minimum file size**: Do not create a new reference file with < 20 lines of real content. If a split would produce a stub, merge the stub into an existing reference or skip the split.
- **Prerequisite enforcement**: If `references/` does not exist or is empty, exit with a clear message directing the user to run `skill-extract-references` first. Do not attempt first-time extraction in this skill.
- **No commit**: Write files and stop. The user handles the file lifecycle.
- **Worktree discipline**: If working in a worktree, all file operations use the worktree root path.
- **Idempotency**: Running the skill twice on the same well-balanced tree should produce no changes. After a successful rebalance, re-running should hit the Step 2 early exit.
- **Both skill locations**: Works on skills in `.claude/skills/`, `.codex/skills/`, or any other path the user provides.

## Example Usage

```
/skill-rebalance-references .codex/skills/implement-ticket
/skill-rebalance-references .claude/skills/improve-loop
/skill-rebalance-references .claude/worktrees/my-feature/.codex/skills/implement-ticket
```
