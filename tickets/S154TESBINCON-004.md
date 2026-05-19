# S154TESBINCON-004: `campaigns/golden-perf/harness.sh` rework

**Status**: PENDING
**Priority**: LOW
**Effort**: Small
**Engine Changes**: None
**Deps**: 002

## Problem

`campaigns/golden-perf/harness.sh` uses `find tests/ -name 'golden_*.rs'` to discover per-file golden binaries, then runs `cargo test --test <suite>` per file and ranks the 5 slowest test *binaries*. After ticket 002 lands, there are only 2 binaries (`golden_ai`, `integration_ai`), so the "5 slowest binaries" ranking collapses to a trivial result. The campaign concept (5 slowest *scenarios*) is still meaningful but the harness needs either repurposing or retirement.

Per the spec, the choice between repurpose and retire is deferred to ticket implementation based on current campaign activity.

## Assumption Reassessment (2026-05-19)

1. `campaigns/golden-perf/harness.sh` exists and currently uses the find-and-iterate pattern described in the spec (verified during reassessment). The directory also contains `checks.sh`, `program.md`, and `results.tsv`. Verify at implementation time whether any of those reference the harness directly or need parallel updates if `harness.sh` is retired.
2. Decision (repurpose vs retire) deferred to implementation. The implementer must check: (a) is the campaign actively used (recent commit activity on `harness.sh` / `results.tsv`); (b) does any CI workflow under `.github/workflows/*` invoke `harness.sh`; (c) does any doc reference the campaign as a live tool. If (a)/(b)/(c) all return "no recent activity / no references", retirement is the right path; otherwise repurpose.
3. Shared boundary under audit: the `golden-perf` campaign's user contract — ranking the 5 slowest scenarios so developers can attribute test runtime cost. If retired, the contract is dropped; if repurposed, the contract is preserved but the implementation shifts from per-binary timing to per-test timing (`cargo test -- --report-time` on the consolidated binary, or equivalent stable timing surface).

## Architecture Check

1. **Repurpose option**: rewrite the harness to drive `cargo test -p worldwake-ai --test golden_ai -- -Z unstable-options --report-time --test-threads=1` (nightly-only) or find an alternative stable timing surface. Parse per-test duration output, bucket by module path (`scenarios::<name>`), rank the 5 slowest modules. The nightly-only constraint is a known cost.
2. **Retire option**: delete `campaigns/golden-perf/` entirely. Any dangling references (in `docs/*`, `.github/workflows/*`, README files) are removed in the same diff. This is the FND-28-aligned default when the campaign is dormant — no dead campaign tooling left behind.
3. The choice depends on whether the per-scenario timing surface is worth the nightly-flag cost or an alternative timing surface exists. If recent activity on `results.tsv` is low and no CI invocation exists, retirement is correct.

## Verification Layers

1. **If repurposed**: `bash campaigns/golden-perf/harness.sh` exits 0 with intelligible output (the 5 slowest scenarios listed, parseable wall-clock values)
2. **If retired**: `grep -rln 'campaigns/golden-perf\|golden-perf/harness' docs/ .github/ scripts/ README.md 2>/dev/null` returns no matches; `ls campaigns/golden-perf/ 2>/dev/null` confirms the directory is gone
3. Single-layer ticket: tooling/CI surface only.

## What to Change

### 1. Investigate campaign activity

Check `git log --oneline -20 -- campaigns/golden-perf/` for recent activity; check `.github/workflows/*` for any reference to `golden-perf` or `harness.sh`; check `docs/*` and the workspace-root README for instructions invoking the campaign. Document the findings in the ticket's Architecture Check section during implementation before choosing the path.

### 2a. Repurpose path (if active)

Rewrite `campaigns/golden-perf/harness.sh`:

- Replace `find tests/ -name 'golden_*.rs'` with no discovery (the binary is fixed: `golden_ai`)
- Invoke `cargo test -p worldwake-ai --test golden_ai -- -Z unstable-options --report-time --test-threads=1` (or equivalent stable timing source if `-Z unstable-options` is unacceptable)
- Parse per-test duration output, bucket by module path (`scenarios::<name>`), rank the 5 slowest modules
- Output in the same format `results.tsv` expects (verify schema during implementation; reset `results.tsv` if the schema diverges)

### 2b. Retire path (if dormant)

Delete `campaigns/golden-perf/` entirely (`harness.sh`, `checks.sh`, `program.md`, `results.tsv`). Grep for and remove any references to this campaign in `docs/*`, `.github/workflows/*`, `scripts/*`, and the workspace-root README. Note in the spec's Outcome section that the campaign was retired.

## Files to Touch

Depends on the decision:

- **Repurpose**: `campaigns/golden-perf/harness.sh` (modify); possibly `campaigns/golden-perf/results.tsv` (schema check or reset)
- **Retire**: `campaigns/golden-perf/` (delete entire directory); any dangling references in `docs/*`, `.github/workflows/*`, README files (modify or remove)

Likely: `.github/workflows/` review (verify no CI workflow invokes the harness). To be confirmed during implementation: `grep -rln 'golden-perf' .github/ 2>/dev/null`.

## Out of Scope

- Any other campaigns under `campaigns/` — this ticket scopes to `golden-perf/` only
- Source-file moves, generated-doc regeneration, or script cleanup — tickets 002/003
- Doc/skill sweep for unrelated `tests/golden_*.rs` references — ticket 005

## Acceptance Criteria

### Tests That Must Pass

1. **If repurposed**: `bash campaigns/golden-perf/harness.sh` exits 0 with the 5-slowest-scenarios output (non-empty, parseable, sane wall-clock values)
2. **If retired**: `grep -rln 'campaigns/golden-perf' docs/ .github/ scripts/ README.md 2>/dev/null` returns no matches (or only the spec/archive itself if intentional)
3. Existing suite: `scripts/verify.sh` is unaffected by this ticket either way (the harness is not part of the gate)

### Invariants

1. No dangling references to `campaigns/golden-perf/` remain after this ticket lands (regardless of repurpose vs retire)
2. If repurposed, the harness uses the post-T2 binary name (`golden_ai`), not the pre-T2 per-scenario binary names

## Test Plan

### New/Modified Tests

1. None — this is operational tooling, not unit-tested. Verification is command-based (the harness exits 0 OR the directory is gone with no dangling references).

### Commands

1. **If repurposed**: `bash campaigns/golden-perf/harness.sh`
2. **If retired**: `grep -rln 'campaigns/golden-perf\|golden-perf/harness' docs/ .github/ scripts/ README.md 2>/dev/null` (expect no matches)
3. `scripts/verify.sh`
