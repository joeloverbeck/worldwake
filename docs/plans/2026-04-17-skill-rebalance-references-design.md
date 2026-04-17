# Design: `skill-rebalance-references`

## Brainstorm Context

**Original request**: Analyze `.claude/skills/skill-consolidate/*` in the context of `.codex/skills/implement-ticket/*` having grown bloated again despite already having extracted references. Determine whether the three rebalance operations — (1) making references more granular, (2) moving information between references, (3) moving information from `SKILL.md` into references — should live in `skill-consolidate`, `skill-extract-references`, or a new skill.

**Reference material (read inline)**:
- `.claude/skills/skill-consolidate/SKILL.md` — single-file consolidation (redundancy, regrouping, tightening).
- `.claude/skills/skill-extract-references/SKILL.md` — initial monolith → thin `SKILL.md` + `references/` split.
- `.codex/skills/implement-ticket/SKILL.md` (256 lines) + 6 references (largest: `reassessment-checks.md` at 162 lines) — the concrete re-bloat case.

**Key interview insights**:
- `skill-extract-references` explicitly exits early when `SKILL.md` already has ≥ 3 load instructions — it punts on re-extraction by design.
- `skill-consolidate` only operates on a single file; never reads or writes `references/`.
- The three rebalance operations user identified form a coherent unit distinct from either existing skill. Different input shape (multi-file tree vs single file), different decision set (split/move/re-extract vs tighten, vs first-time classify).
- Extending either existing skill would break its single-responsibility invariant; a new skill preserves each existing skill's tight scope and lets the three compose as a pipeline.

**Final confidence**: 95%+ at end of section-by-section approval. No outstanding assumptions.

---

## 1. Overview & Identity

**Name**: `skill-rebalance-references`

**Location**: `.claude/skills/skill-rebalance-references/SKILL.md`

**Purpose**: Redistribute content across an already-split skill tree (`SKILL.md` + `references/*.md`) by splitting overloaded references, moving content between references, and re-extracting accumulated bulk from `SKILL.md` — preserving every instruction and leaving each reference coherent in scope.

**Prerequisite**: Target skill must already have `references/` populated by `skill-extract-references`. If `references/` is absent, the skill exits and directs the user to run `skill-extract-references` first.

**Invocation**: `/skill-rebalance-references <skill-path>` (positional argument, same shape as sibling skills).

**Complements, does not replace**:
- `skill-extract-references`: monolith → thin + `references/` (first-time split).
- `skill-rebalance-references`: redistribute within an existing tree (this new skill).
- `skill-consolidate`: tighten prose within a single file (orthogonal concern).

---

## 2. Procedure

Ten steps, following sibling-skill numbered-procedure convention.

### 1. Read inputs
- Resolve `<skill-path>` to absolute path. Confirm `<skill-path>/SKILL.md` exists.
- Read `SKILL.md` in full.
- List `<skill-path>/references/`. If missing or empty, stop and direct user to `skill-extract-references`.
- Read every reference doc. Record line counts for `SKILL.md` and each reference.

### 2. Eligibility check (early exit)
Exit with "nothing to rebalance" if ALL conditions hold:
- `SKILL.md` ≤ 80 lines.
- Every reference ≤ 150 lines.
- No reference covers ≥ 2 clearly distinct topics (spot-check via H2/H3 diversity).

Otherwise proceed.

### 3. Assess structural balance
Classify each file:
- **Overloaded reference**: > 150 lines OR covers 2+ distinct sub-topics identifiable by heading groups.
- **Underloaded reference**: < 20 lines of real content (candidate for merge, not split).
- **SKILL.md re-bloat zones**: any step containing > ~10 lines of dense directives beyond framing + load instruction.

### 4. Plan operations (three buckets)
For each flagged file, propose one or more of:
- **Split**: overloaded reference → 2+ new references (with proposed filenames).
- **Move**: content block currently in ref A → ref B (because it thematically belongs there).
- **Re-extract**: dense content inside a `SKILL.md` step → new or existing reference, replaced by a load instruction.

### 5. Present plan to user for approval
Print the plan in structured form (splits, moves, re-extracts) with line estimates. Do not write any files yet. Wait for approval.

### 6. Execute splits
- Write new reference files with extracted content.
- Remove split content from the source reference (leave a one-line cross-reference if the source still exists and still has content; delete the source if fully superseded).

### 7. Execute moves
- Append moved content to the destination reference at the topically correct location.
- Remove from source reference.

### 8. Execute re-extractions
- Write `SKILL.md` content to the target reference (new or existing).
- In `SKILL.md`, replace the dense block with the same load-instruction pattern that `skill-extract-references` uses ("Load `references/<name>.md`." with optional 1–2 sentence framing).

### 9. Update cross-references inside the skill
- In `SKILL.md`, every load instruction must point to an existing reference file.
- In references, any "see `references/foo.md`" link must still resolve. Rename fallout from splits gets fixed here.

### 10. Verify preservation and emit summary
- Spot-check 5 unique instructions from the original state (across `SKILL.md` + references) — each must still exist somewhere.
- Grep other skill files (`.claude/skills/*/SKILL.md`, `.codex/skills/*/SKILL.md`) for references to any renamed/split reference paths. Report external pointers that may need updating.
- Print summary: splits performed, moves performed, re-extractions performed, before/after line counts per file, external-reference warnings.

Do NOT commit.

---

## 3. Decision Rules

### Split a reference

Trigger a split when **both** hold:
- Reference file > 150 lines of real content.
- File covers ≥ 2 distinct sub-topics, identifiable by one of:
  - Multiple H2/H3 groups with non-overlapping subject matter.
  - Distinct workflow phases or decision domains within one file.
  - Internal "when X, ..." / "when Y, ..." blocks where X and Y are orthogonal concerns (not variants of the same concern).

**Naming the split**: derive kebab-case filenames from each sub-topic's subject matter (same convention as `skill-extract-references`).

**Do NOT split when**:
- File is long but covers one coherent topic (length alone is insufficient).
- The only "sub-topics" are variants of the same decision (e.g., "shared-field addition" vs "shared-field removal" — both are shared-type fallout).
- Split would produce a file under 20 lines.

### Move content between references

Trigger a move when a block's **subject matter** is the direct topic of a different reference. Specifically:
- Block in ref A describes behavior/rules primarily belonging to ref B's declared topic.
- Block appears in A only because it was colocated with a related instruction at extraction time, not because A owns it.

**Do NOT move when**:
- The block is genuinely dual-purpose and belongs in both (cross-reference instead; duplicate only if semantically distinct).
- Moving would leave the source reference incoherent (in that case, consider splitting instead).

### Re-extract from `SKILL.md`

Trigger a re-extraction when a `SKILL.md` step contains dense, self-contained guidance beyond thin-orchestration form. Specifically:
- Step body has > ~10 lines of directive content beyond framing + load instruction.
- Step contains an embedded decision matrix, classification table, or long "when X, do Y" list.
- Step has accumulated audit-driven edge-case guidance that no longer reads as orchestration.

**Destination choice**:
- If the content thematically matches an existing reference → append there (re-extraction + merge).
- If it represents a new coherent topic → create a new reference file.

**Do NOT re-extract when**:
- The content is genuinely orchestration (the numbered sequence itself, short framing sentences, load directives).
- The step is already thin (framing + load + brief post-load note).

### Priority order when multiple operations apply

When a single file qualifies for multiple operations, apply in this order:
1. **Re-extract first** — pull `SKILL.md` bulk into references; may change what needs splitting.
2. **Split next** — now that references have stabilized, split any still-overloaded.
3. **Move last** — redistribute across the now-stable set of files.

This ordering prevents thrash (e.g., splitting a reference before re-extracting new content into it would immediately require another rebalance).

---

## 4. Guardrails

- **Semantic preservation**: Every unique instruction in the original state (across `SKILL.md` + all references) must exist somewhere in the final tree. Verified in Step 10.
- **Frontmatter untouched**: Never modify `SKILL.md`'s YAML frontmatter. References have no frontmatter and stay that way.
- **No scope expansion**: Redistribute existing content only. Do not add instructions, fill gaps, tighten prose, or remove redundancy. Record observed redundancy in the summary under "Observations" — do not fix it (that is `skill-consolidate`'s job).
- **No deletion without relocation**: Content removed from a source file must land in a destination file. Exception: cross-reference stubs when a split fully supersedes the source.
- **Minimum file size**: Do not create a new reference file with < 20 lines of real content. If a split would produce a stub, merge into an existing reference or skip the split.
- **Prerequisite enforcement**: Exit with a clear message if `references/` does not exist. Direct the user to `skill-extract-references`.
- **No commit**: Write files and stop. User handles git.
- **Worktree discipline**: If working in a worktree, all paths use the worktree root.
- **Idempotency**: Running the skill twice on the same well-balanced tree should produce no changes. After rebalance, re-running should hit the Step 2 early exit.
- **Both skill locations**: Works on skills in `.claude/skills/`, `.codex/skills/`, or any user-provided path.

---

## 5. Composition with Sibling Skills

| Skill | Input state | Output state |
|-------|-------------|--------------|
| `skill-extract-references` | Monolithic bloated `SKILL.md`, no references | Thin `SKILL.md` + initial `references/` |
| `skill-rebalance-references` | Re-bloated tree (existing references + grown `SKILL.md`) | Redistributed tree with coherent reference scopes |
| `skill-consolidate` | Any single file with internal entropy | Same file, tightened |

**Typical sequences**:
- **First-time bloat**: run `skill-extract-references`. Done.
- **Re-bloat of an already-extracted skill**: run `skill-rebalance-references`, then run `skill-consolidate` on each file whose content changed.
- **Steady-state maintenance**: run `skill-consolidate` on `SKILL.md` periodically. If Step 2 early-exit fails because the file grew structurally rather than verbosely, escalate to `skill-rebalance-references`.

**What `skill-rebalance-references` explicitly does NOT do**:
- First-time extraction (use `skill-extract-references`).
- Prose tightening, redundancy removal, decision-path unification (use `skill-consolidate`).
- Audit for missing content or gaps (use `skill-audit`).

Each existing skill's invariants stay intact. No existing skill changes.

---

## 6. First Invocation Target

The motivating case is `.codex/skills/implement-ticket/`:
- `SKILL.md` at 256 lines — Step 0 contains a ~60-line classification matrix (fast path / shared-field / shared additive / full workflow) that reads as embedded decision logic rather than orchestration → **re-extract** candidate (likely new `references/ticket-classification.md`).
- `reassessment-checks.md` at 162 lines — likely **split** candidate along concern boundaries (shared-type, planner-boundary, golden/scenario). Final split decisions come from reading the file's actual internal structure.

This provides a concrete test of the skill on its first invocation. Results will inform whether decision-rule thresholds (150 / 80 / ~10 lines) need adjustment.
