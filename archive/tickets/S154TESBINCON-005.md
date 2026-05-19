# S154TESBINCON-005: Hand-authored doc + skill sweep

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: None
**Deps**: archive/tickets/S154TESBINCON-002.md

## Problem

`tests/golden_*.rs` and `cargo test --test golden_*` patterns are referenced across hand-authored docs and `.claude/skills/*` agent instructions. After ticket 002's source moves, these references are stale: they point to file paths that no longer exist and invocation forms that no longer match the consolidated binary. Per FND-28 (no backward compatibility) and the spec's own commitment, the new invocation and path forms must be the only documented forms after this ticket lands.

## Assumption Reassessment (2026-05-19)

1. Reassessment confirmed the blast radius via grep and corrected the original target list. In addition to the initially named `docs/golden-e2e-testing.md`, `docs/debugging-traces.md`, five `.claude/skills/*` files, and `CLAUDE.md`, live active references also remain in `docs/scenario-roadmap.md`, `docs/cargo-artifact-hygiene.md`, `.claude/skills/fix-ci-failures/SKILL.md`, `.claude/skills/golden-gap-analysis/SKILL.md`, `.claude/skills/brainstorm/SKILL.md`, and `.claude/skills/handoff/references/examples.md`. Valid `golden_harness/` helper paths and `fn golden_*` test-name conventions are not stale source-file layout references.
2. `docs/plans/*` (5 plan docs reference the old convention) are historical/read-only per the spec's blast-radius table — no edits required, confirmed. `archive/specs/*` and `archive/tickets/*` are similarly read-only historical records.
3. Shared boundary under audit: the workflow muscle memory and AI-agent guidance pattern. After this ticket, the only documented form for invoking golden tests is `cargo test -p worldwake-ai --test golden_ai <scenario>` (substring filter against the module path), and the only documented file-location pattern for "golden test file" is `tests/scenarios/<name>.rs`.

## Architecture Check

1. The sweep is straightforward find-and-replace, but each surface needs context-sensitive judgment: a doc example invocation needs the full new form (`--test golden_ai <scenario>`); a skill's location-pattern reference needs the new path (`tests/scenarios/*.rs`); a workflow example needs both the path and the filter form updated. Mechanical `sed` is insufficient — each match needs case-by-case review to confirm the surrounding prose still makes sense post-replacement.
2. The asymmetric harness rename from ticket 002 (`golden_harness/` keeps its prefix; the two niche harnesses drop theirs) is documented in this ticket's affected doc files where the harness names appear. The shared-helper-location prose in `docs/golden-e2e-testing.md` (currently `crates/worldwake-ai/tests/golden_harness/survival_forensics_assertions.rs`) stays valid.
3. No backward-compat shim: the old `cargo test --test golden_<scenario>` form is replaced, not preserved as an alternate documented form.

## Verified Layers

1. Workspace grep returned only intentional historical references plus valid `golden_harness/` and `fn golden_*` conventions.
2. Deprecated per-file golden invocation examples were replaced in active docs and `.claude/skills/` guidance.
3. Single-layer ticket: pure documentation/instruction update with no code effect.

## Landed Changes

### 1. `docs/golden-e2e-testing.md`

Replaced prose references to the old `tests/golden_*.rs` source layout with `tests/scenarios/*.rs`. Updated workflow examples to the consolidated `golden_ai` binary plus per-scenario filter form. Added a brief note explaining the post-T2 layout. The shared-helper-location prose (`crates/worldwake-ai/tests/golden_harness/survival_forensics_assertions.rs`) stayed valid because `golden_harness/` retains its prefix.

### 2. `docs/debugging-traces.md`

Updated deprecated per-file golden example patterns to the consolidated filter form. The shared-helper reference at line 156 stayed valid (`golden_harness/` unchanged).

### 3. `.claude/skills/detect-architectural-debt/SKILL.md`

Updated the `description` frontmatter example path to the post-T2 `crates/worldwake-ai/tests/scenarios/trade_acquisition.rs` layout.

### 4. `.claude/skills/reassess-spec/SKILL.md`

Updated the pre-apply table example to use the new `crates/worldwake-ai/tests/scenarios/survival_*.rs` path pattern and checked other location-pattern references in the file.

### 5. `.claude/skills/simulation-remediation/SKILL.md`

Updated old location-pattern references and file-naming examples with `tests/scenarios/<name>.rs`. Updated the agent prompt language to glob `crates/worldwake-ai/tests/scenarios/*.rs`.

### 6. `.claude/skills/goap-architecture-report/SKILL.md`

Replaced the old inventory reference with `tests/scenarios/*.rs`.

### 7. `.claude/skills/implement-ticket/SKILL.md`

Replaced old all-golden-file scan guidance with `tests/scenarios/*.rs` scan guidance.

### 8. `CLAUDE.md`

Updated the narrow-check example to the consolidated `golden_ai` target and added a brief note explaining that the post-T2 form uses a substring filter against the module path.

### 9. Additional active docs and skill guidance found during reassessment

Updated `docs/scenario-roadmap.md`, `docs/cargo-artifact-hygiene.md`, `.claude/skills/fix-ci-failures/SKILL.md`, `.claude/skills/golden-gap-analysis/SKILL.md`, `.claude/skills/brainstorm/SKILL.md`, and `.claude/skills/handoff/references/examples.md` where they still described old golden source paths or per-file test targets. Kept `docs/plans/*` historical and left valid `golden_harness/` helper paths alone.

## Landed Files

- `docs/golden-e2e-testing.md` (modify)
- `docs/debugging-traces.md` (modify)
- `.claude/skills/detect-architectural-debt/SKILL.md` (modify)
- `.claude/skills/reassess-spec/SKILL.md` (modify)
- `.claude/skills/simulation-remediation/SKILL.md` (modify)
- `.claude/skills/goap-architecture-report/SKILL.md` (modify)
- `.claude/skills/implement-ticket/SKILL.md` (modify)
- `.claude/skills/fix-ci-failures/SKILL.md` (modify)
- `.claude/skills/golden-gap-analysis/SKILL.md` (modify)
- `.claude/skills/brainstorm/SKILL.md` (modify)
- `.claude/skills/handoff/references/examples.md` (modify)
- `CLAUDE.md` (modify)
- `docs/scenario-roadmap.md` (modify)
- `docs/cargo-artifact-hygiene.md` (modify)

## Out of Scope

- `docs/plans/*` (5 plan docs that reference the old convention) — per spec, these describe past state at time of writing; no edit required
- `archive/specs/*` and `archive/tickets/*` — historical records, not edited
- Source-file moves, generated-doc regeneration, script cleanup, retired campaigns harness — archive/tickets/S154TESBINCON-002.md / archive/tickets/S154TESBINCON-003.md / archive/tickets/S154TESBINCON-004.md

## Acceptance Result

### Passed Verification

1. `rg -n 'tests/golden_|golden_[a-z0-9_]+\.rs|golden_\*\.rs|--test golden_[^a]' docs .claude/skills CLAUDE.md` returned no active stale matches outside `docs/plans/*`, valid `tests/golden_harness/` helper paths, and valid `fn golden_*` test-name references.
2. `rg -n 'cargo test.*--test golden_[^a]' docs .claude/skills CLAUDE.md` returned no active stale matches outside `docs/plans/*`.
3. `./scripts/verify.sh` passed.

### Invariants

1. Active docs now document the consolidated `golden_ai` target plus scenario-filter form (and the path-precise `scenarios::<scenario>` variant); no active doc surfaces the deprecated per-scenario test target form.
2. After this ticket, the only documented file-location pattern for a "golden test file" is `tests/scenarios/<name>.rs`; no doc surfaces the deprecated `tests/golden_<name>.rs` path

## Test Plan Result

### Modified Tests

1. None — documentation-only ticket; verification is command-based and existing runtime coverage is named in Assumption Reassessment.

### Commands Run

1. `rg -n 'tests/golden_|golden_[a-z0-9_]+\.rs|golden_\*\.rs|--test golden_[^a]' docs .claude/skills CLAUDE.md` (expect only historical `docs/plans/*`, valid `tests/golden_harness/`, and valid `fn golden_*` references)
2. `rg -n 'cargo test.*--test golden_[^a]' docs .claude/skills CLAUDE.md` (expect only historical `docs/plans/*` matches)
3. `scripts/verify.sh`

## Verification Result

1. Passed `git diff --check`.
2. Passed `rg -n 'tests/golden_|golden_[a-z0-9_]+\.rs|golden_\*\.rs|--test golden_[^a]' docs .claude/skills CLAUDE.md`: only historical `docs/plans/*` matches plus valid `tests/golden_harness/` helper paths remained.
3. Passed `rg -n 'cargo test.*--test golden_[^a]' docs .claude/skills CLAUDE.md`: only historical `docs/plans/*` matches remained.
4. Passed `./scripts/verify.sh`: ran `cargo fmt --all -- --check`, `cargo test --workspace`, active-goal/artifact/debug-view checks, `cargo clippy --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, and `cargo run -p worldwake-cli --bin scenario-coverage -- --check`.

## Outcome

Completed: 2026-05-19

What changed:

- Updated the hand-authored golden guidance in `docs/golden-e2e-testing.md`, `docs/debugging-traces.md`, `docs/scenario-roadmap.md`, and `docs/cargo-artifact-hygiene.md` so active docs point at `crates/worldwake-ai/tests/scenarios/*.rs` and `cargo test -p worldwake-ai --test golden_ai <scenario>`.
- Updated `CLAUDE.md` and active `.claude/skills/*` guidance so agent workflows use the consolidated `golden_ai` binary and scenario-source layout.
- Truthed the active S154 spec and this ticket after reassessment found additional active stale surfaces beyond the original T5 file list.

Deviations from original plan:

- Expanded the edited surface to include `docs/scenario-roadmap.md`, `docs/cargo-artifact-hygiene.md`, `.claude/skills/fix-ci-failures/SKILL.md`, `.claude/skills/golden-gap-analysis/SKILL.md`, `.claude/skills/brainstorm/SKILL.md`, and `.claude/skills/handoff/references/examples.md`.
- Left `docs/plans/*` unchanged as intentional historical planning records.
- Left `golden_harness/` helper paths and `fn golden_*` test names unchanged because they remain live conventions after S154.
