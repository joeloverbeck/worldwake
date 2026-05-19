# S154TESBINCON-004: `campaigns/golden-perf/harness.sh` rework

**Status**: COMPLETED
**Priority**: LOW
**Effort**: Small
**Engine Changes**: None
**Deps**: archive/tickets/S154TESBINCON-002.md

## Problem

`campaigns/golden-perf/harness.sh` uses `find tests/ -name 'golden_*.rs'` to discover per-file golden binaries, then runs `cargo test --test <suite>` per file and ranks the 5 slowest test *binaries*. After ticket 002 lands, there are only 2 binaries (`golden_ai`, `integration_ai`), so the "5 slowest binaries" ranking collapses to a trivial result. The campaign concept (5 slowest *scenarios*) is still meaningful but the harness needs either repurposing or retirement.

Per the spec, this ticket chose between repurpose and retire based on live campaign activity.

## Assumption Reassessment (2026-05-19)

1. `campaigns/golden-perf/harness.sh` existed and used the stale find-and-iterate pattern described in the spec. The directory also contained `checks.sh`, `program.md`, and an untracked header-only `results.tsv`.
2. Live activity checks selected retirement: `git log --format='%h %ad %s' --date=short -- campaigns/golden-perf/` showed only March 2026 campaign commits, `results.tsv` had no recorded experiment rows, and `rg -n 'golden-perf|campaigns/golden-perf|golden-perf/harness' campaigns/golden-perf .github docs scripts README.md` found no CI/workflow or live doc/script references outside the campaign files, this spec, this ticket, and one historical `docs/plans/` design note.
3. Shared boundary audited: the `golden-perf` campaign's former user contract ranked the 5 slowest golden test binaries. After S154 consolidation that contract was stale, and no active consumer justified replacing it with a nightly-only per-scenario timing harness.

## Architecture Check

1. Retirement was cleaner than repurposing because the only plausible per-scenario replacement depended on unstable `--report-time` output or a new bespoke timing substrate, while the live campaign had no current consumer.
2. The entire stale campaign directory was removed. This follows the S154/FND-28 migration discipline by deleting the obsolete per-binary timing surface instead of preserving a fossilized fallback.
3. The active S154 spec records the T4 retirement outcome so the remaining T5 doc/skill sweep no longer treats `campaigns/golden-perf/` as a live tool.

## Verified Layers

1. Retired campaign directory absence -> direct path check (`test ! -e campaigns/golden-perf`)
2. No live dangling references -> grep over `docs/ .github/ scripts/ README.md`, excluding the historical `docs/plans/` design note
3. Single-layer ticket: tooling/CI surface only.

## Implementation Result

### 1. Campaign activity investigated

The campaign was dormant: only March 2026 campaign commits were present, no CI workflow invoked it, no live docs/scripts referenced it, and `results.tsv` contained only its header row.

### 2. Retired stale campaign

Deleted `campaigns/golden-perf/` (`harness.sh`, `checks.sh`, `program.md`, and `results.tsv`) and updated the active S154 spec's T4 section with the retirement outcome.

## Landed Files

- `campaigns/golden-perf/` (deleted entire directory)
- `specs/S154-test-binary-consolidation.md` (updated T4 retirement outcome)

## Out of Scope

- Any other campaigns under `campaigns/` — this ticket scopes to `golden-perf/` only
- Source-file moves, generated-doc regeneration, or script cleanup — tickets 002/003
- Doc/skill sweep for unrelated `tests/golden_*.rs` references — ticket 005

## Acceptance Result

### Verified Commands

1. `test ! -e campaigns/golden-perf`
2. `rg -n 'campaigns/golden-perf|golden-perf/harness' docs .github scripts README.md --glob '!docs/plans/**'` returns no matches
3. Existing suite: `scripts/verify.sh` is unaffected by this ticket; no Rust source or executable workspace gate changed, so the broad gate was waived for this iteration

### Invariants

1. No live dangling references to `campaigns/golden-perf/` remain after this ticket lands
2. No replacement harness documents or invokes the pre-T2 per-scenario binary names

## Test Plan Result

### Test Changes

1. None — this was operational tooling, not unit-tested. Verification was command-based: the directory is gone and no live references remain.

### Command Results

1. `test ! -e campaigns/golden-perf`
2. `rg -n 'campaigns/golden-perf|golden-perf/harness' docs .github scripts README.md --glob '!docs/plans/**'` (expect zero matches)
3. `git diff --check -- archive/tickets/S154TESBINCON-004.md specs/S154-test-binary-consolidation.md campaigns/golden-perf/checks.sh campaigns/golden-perf/harness.sh campaigns/golden-perf/program.md`

## Verification Result

1. Passed: `test ! -e campaigns/golden-perf`
2. Passed: `rg -n 'campaigns/golden-perf|golden-perf/harness' docs .github scripts README.md --glob '!docs/plans/**'` returned no matches
3. Passed: `git diff --check -- archive/tickets/S154TESBINCON-004.md specs/S154-test-binary-consolidation.md campaigns/golden-perf/checks.sh campaigns/golden-perf/harness.sh campaigns/golden-perf/program.md`
4. Waived: `scripts/verify.sh` broad gate is outside this deletion-only tooling-retirement proof; no Rust source, tests, generated docs, or executable workspace gate changed

## Outcome

Completed: 2026-05-19

The obsolete `golden-perf` campaign was retired rather than repurposed. The live activity check found no current CI, doc, or script consumer; no recorded experiment rows; and only March 2026 campaign history. The stale per-binary harness, checks wrapper, campaign program, and header-only results file were deleted. The active S154 spec now records T4's retirement outcome.

Deviations from original plan: selected the retire path, so no nightly-only or custom per-scenario timing harness was introduced.
