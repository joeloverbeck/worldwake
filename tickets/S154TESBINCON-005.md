# S154TESBINCON-005: Hand-authored doc + skill sweep

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: None
**Deps**: archive/tickets/S154TESBINCON-002.md

## Problem

`tests/golden_*.rs` and `cargo test --test golden_*` patterns are referenced across hand-authored docs and `.claude/skills/*` agent instructions. After ticket 002's source moves, these references are stale: they point to file paths that no longer exist and invocation forms that no longer match the consolidated binary. Per FND-28 (no backward compatibility) and the spec's own commitment, the new invocation and path forms must be the only documented forms after this ticket lands.

## Assumption Reassessment (2026-05-19)

1. Reassessment confirmed the blast radius via grep: 9 files in `docs/` reference `tests/golden_` or `--test golden_`, 7 files across `.claude/skills/` reference the same patterns, plus `CLAUDE.md` has one stale reference at line 50. Specific files: `docs/golden-e2e-testing.md` (multiple prose references at lines 3, 103, 454+), `docs/debugging-traces.md` (line 156 helper reference, line 311+ command-pattern examples), `.claude/skills/detect-architectural-debt/SKILL.md` (line 7 description), `.claude/skills/reassess-spec/SKILL.md` (line 189 pre-apply table example + 1 reference), `.claude/skills/simulation-remediation/SKILL.md` (lines 43, 52, 120), `.claude/skills/goap-architecture-report/SKILL.md` (line 38), `.claude/skills/implement-ticket/SKILL.md` (line 57), `CLAUDE.md` (line 50 example invocation).
2. `docs/plans/*` (5 plan docs reference the old convention) are historical/read-only per the spec's blast-radius table — no edits required, confirmed. `archive/specs/*` and `archive/tickets/*` are similarly read-only historical records.
3. Shared boundary under audit: the workflow muscle memory and AI-agent guidance pattern. After this ticket, the only documented form for invoking golden tests is `cargo test -p worldwake-ai --test golden_ai <scenario>` (substring filter against the module path), and the only documented file-location pattern for "golden test file" is `tests/scenarios/<name>.rs`.

## Architecture Check

1. The sweep is straightforward find-and-replace, but each surface needs context-sensitive judgment: a doc example invocation needs the full new form (`--test golden_ai <scenario>`); a skill's location-pattern reference needs the new path (`tests/scenarios/*.rs`); a workflow example needs both the path and the filter form updated. Mechanical `sed` is insufficient — each match needs case-by-case review to confirm the surrounding prose still makes sense post-replacement.
2. The asymmetric harness rename from ticket 002 (`golden_harness/` keeps its prefix; the two niche harnesses drop theirs) is documented in this ticket's affected doc files where the harness names appear. The shared-helper-location prose in `docs/golden-e2e-testing.md` (currently `crates/worldwake-ai/tests/golden_harness/survival_forensics_assertions.rs`) stays valid.
3. No backward-compat shim: the old `cargo test --test golden_<scenario>` form is replaced, not preserved as an alternate documented form.

## Verification Layers

1. Workspace grep returns only intentional historical references → `grep -rn 'tests/golden_\*\.rs\|tests/golden_[a-z]' docs/golden-e2e-testing.md docs/debugging-traces.md .claude/skills/ CLAUDE.md` returns no matches in the edited files
2. Old invocation form replaced → `grep -rn 'cargo test.*--test golden_[^a]' docs/ .claude/skills/ CLAUDE.md` returns no matches (the `[^a]` excludes the new `golden_ai` form)
3. Single-layer ticket: pure documentation/instruction update with no code effect.

## What to Change

### 1. `docs/golden-e2e-testing.md`

Replace prose references to `tests/golden_*.rs` with `tests/scenarios/*.rs`. Update workflow examples that invoke `cargo test --test golden_<name>` to use `cargo test -p worldwake-ai --test golden_ai <name>`. Add a brief note explaining the post-T2 layout (one binary covers all goldens; per-scenario filter is a substring match against the module path). The shared-helper-location prose (`crates/worldwake-ai/tests/golden_harness/survival_forensics_assertions.rs`) stays valid because `golden_harness/` retains its prefix.

### 2. `docs/debugging-traces.md`

Update the `cargo test --test golden_*` example patterns to the new filter form. The shared-helper reference at line 156 stays valid (`golden_harness/` unchanged).

### 3. `.claude/skills/detect-architectural-debt/SKILL.md`

Update the `description` frontmatter line 7 example path (`crates/worldwake-ai/tests/golden_trade_acquisition.rs`) to the post-T2 path (`crates/worldwake-ai/tests/scenarios/trade_acquisition.rs`).

### 4. `.claude/skills/reassess-spec/SKILL.md`

Update the pre-apply table example at line 189 (`grep -n "pm(750)" crates/worldwake-ai/tests/golden_survival_*.rs`) to use the new path pattern (`crates/worldwake-ai/tests/scenarios/survival_*.rs`). Check any other location-pattern references in the file.

### 5. `.claude/skills/simulation-remediation/SKILL.md`

Update at lines 43, 52, 120: replace `golden_*.rs` location-pattern references and `golden_[file].rs` file-naming examples with `tests/scenarios/<name>.rs`. Update the agent prompt language that instructs Explore agents to glob `crates/worldwake-ai/tests/golden_*.rs`.

### 6. `.claude/skills/goap-architecture-report/SKILL.md`

Update line 38: replace the `tests/golden_*.rs` inventory reference with `tests/scenarios/*.rs`.

### 7. `.claude/skills/implement-ticket/SKILL.md`

Update line 57: replace `scanning all golden_*.rs files` with `scanning all tests/scenarios/*.rs files`.

### 8. `CLAUDE.md`

Update line 50 example: replace `cargo test -p worldwake-ai --test golden_foo` with `cargo test -p worldwake-ai --test golden_ai foo`. Add a brief note (one line) explaining that the post-T2 form uses a substring filter against the module path.

## Files to Touch

- `docs/golden-e2e-testing.md` (modify)
- `docs/debugging-traces.md` (modify)
- `.claude/skills/detect-architectural-debt/SKILL.md` (modify)
- `.claude/skills/reassess-spec/SKILL.md` (modify)
- `.claude/skills/simulation-remediation/SKILL.md` (modify)
- `.claude/skills/goap-architecture-report/SKILL.md` (modify)
- `.claude/skills/implement-ticket/SKILL.md` (modify)
- `CLAUDE.md` (modify)

## Out of Scope

- `docs/plans/*` (5 plan docs that reference the old convention) — per spec, these describe past state at time of writing; no edit required
- `archive/specs/*` and `archive/tickets/*` — historical records, not edited
- Source-file moves, generated-doc regeneration, script cleanup, campaigns harness — tickets 002 / 003 / 004

## Acceptance Criteria

### Tests That Must Pass

1. `grep -rn 'tests/golden_\*\.rs\|tests/golden_[a-z]' docs/golden-e2e-testing.md docs/debugging-traces.md CLAUDE.md` returns no matches
2. `grep -rn 'tests/golden_\*\.rs\|tests/golden_[a-z]' .claude/skills/detect-architectural-debt/ .claude/skills/reassess-spec/ .claude/skills/simulation-remediation/ .claude/skills/goap-architecture-report/ .claude/skills/implement-ticket/` returns no matches
3. `grep -rn 'cargo test.*--test golden_[^a]' docs/ .claude/skills/ CLAUDE.md` returns no matches (the `[^a]` excludes the new `golden_ai` form)
4. Existing suite: `scripts/verify.sh` (the doc/skill edits do not affect the workspace gate, but run as a sanity check)

### Invariants

1. After this ticket, the only documented form for invoking a golden test is `cargo test -p worldwake-ai --test golden_ai <scenario>` (and the path-precise `scenarios::<scenario>` variant); no doc surfaces the deprecated `cargo test --test golden_<scenario>` form
2. After this ticket, the only documented file-location pattern for a "golden test file" is `tests/scenarios/<name>.rs`; no doc surfaces the deprecated `tests/golden_<name>.rs` path

## Test Plan

### New/Modified Tests

1. None — documentation-only ticket; verification is command-based and existing runtime coverage is named in Assumption Reassessment.

### Commands

1. `grep -rn 'tests/golden_\*\.rs\|tests/golden_[a-z]' docs/golden-e2e-testing.md docs/debugging-traces.md .claude/skills/ CLAUDE.md` (expect zero matches)
2. `grep -rn 'cargo test.*--test golden_[^a]' docs/ .claude/skills/ CLAUDE.md` (expect zero matches)
3. `scripts/verify.sh`
