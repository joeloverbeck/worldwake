# S154TESBINCON-001: Tooling rewrite — dual-layout `golden_inventory.py`

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: None
**Deps**: None

## Problem

`scripts/golden_inventory.py` and `scripts/test_golden_inventory.py` are tightly coupled to the per-file integration-test layout (`tests/golden_*.rs`). When ticket 002 moves the 54 golden source files into `tests/scenarios/`, the script breaks: its glob misses the new files, its per-file `cargo test --test <stem>` invocation pattern collapses (no per-file binary exists anymore), and its `_file_stem_to_detail_name()` derivation produces wrong detail-page names.

This ticket converts the inventory tooling from a layout-coupled implementation to a layout-agnostic one *before* ticket 002 lands, so `docs/generated/*` stays byte-identical through the transition and ticket 002 becomes a pure file-move diff.

## Assumption Reassessment (2026-05-19)

1. `scripts/golden_inventory.py` currently globs `tests/golden_*.rs` at three sites (lines 77, 103, 226 per the spec; verified during reassessment) and runs `cargo test --test <stem> -- --list` per file at line 237. `_file_stem_to_detail_name()` lives at line 360 and already does `removeprefix("golden_")` (a safe no-op for already-de-prefixed names), so handling `foo.rs` is essentially confirmation that existing semantics work for the post-move case.
2. `scripts/test_golden_inventory.py` has 8 existing tests covering current behavior: `test_parse_source_inventory_reads_per_file_golden_functions:19`, `test_parse_cargo_test_list_output_groups_tests_by_binary:39`, `test_parse_source_scenarios_reads_identifier_title_and_tests:66`, `test_parse_source_scenarios_accepts_letter_suffix_identifiers:112`, `test_validate_doc_test_references_flags_stale_names:136`, `test_validate_scenarios_flags_duplicates_empty_blocks_and_missing_compiled_tests:160`, `test_render_inventory_markdown_reports_summary_and_files:208`, `test_render_scenario_markdown_reports_primary_and_replay_tests:224`. All 8 embed the pre-T2 cargo-output format and per-file binary layout; the dual-layout work extends fixtures additively without breaking the existing 8.
3. Shared boundary under audit: the inventory script's *public* contract is the byte-identity of `docs/generated/golden-e2e-inventory.md`, `docs/generated/golden-scenario-index.md`, `docs/generated/golden-scenario-details/*.md`, and `docs/generated/golden-coverage-matrix.md` when run via `python3 scripts/golden_inventory.py --write --check-docs`. The dual-layout work must produce byte-identical output for the current (pre-T2) layout — only the *interpretation* of the discovered files changes.

## Architecture Check

1. Dual-layout glob is the cleanest transition path: ticket 002's massive file rename would otherwise have to coordinate with inventory-script regeneration in the same diff, conflating two unrelated concerns. By making the script accept both layouts first, ticket 002 becomes a pure file-move ticket and ticket 003 a pure regeneration-plus-cleanup ticket.
2. The dual-layout code is explicitly transitional and retired in ticket 003 (placeholder-replace pattern). No fossilized fallback survives the migration: ticket 003 drops both transitional branches (`tests/golden_*.rs` glob and per-file `cargo test --test <stem>` invocation) and verifies via grep that zero matches remain in glob/invocation code paths.

## Verification Layers

1. Byte-identical generated-doc output for pre-T2 layout → `python3 scripts/golden_inventory.py --write --check-docs` exits 0 with no diff against the pre-T1 committed state of `docs/generated/`, verified by `git diff --exit-code docs/generated/`.
2. Test-fixture coverage for both layouts → `python3 scripts/test_golden_inventory.py` passes with new fixtures covering the post-T2 layout in addition to the existing 8 pre-T2 tests.
3. Single-layer ticket: pure tooling-script change with no engine effects. Action-trace / event-log / decision-trace verification surfaces are not applicable.

## What to Change

### 1. Extend `golden_inventory.py` glob discovery

Modify the three glob sites in `scripts/golden_inventory.py` (lines 77, 103, 226 in the current file — verify line numbers at implementation time) to discover scenarios from BOTH `crates/worldwake-ai/tests/scenarios/*.rs` and `crates/worldwake-ai/tests/golden_*.rs`. Merge results; deduplicate by source-file stem. Each entry in the merged inventory carries its source path so per-file detail-page rendering uses the original location.

This branch is placeholder, replaced by ticket 003 — mark with an in-code comment naming ticket 003 as the cleanup target so reviewers of this ticket understand the implementation is intentionally provisional.

### 2. Update cargo-test invocation logic

In `scripts/golden_inventory.py` (current site at line 237), introduce a runtime check: if the new `golden_ai` test binary exists (detect via successful `cargo test -p worldwake-ai -- --list` whose output names `golden_ai`), invoke a single `cargo test -p worldwake-ai --test golden_ai -- --list` and bucket the discovered tests by their module path (`scenarios::<name>::*`). Otherwise, fall back to per-file `cargo test --test <stem> -- --list` invocations (the existing pre-T2 behavior).

Placeholder, replaced by ticket 003 — same in-code comment convention as section 1.

### 3. Confirm `_file_stem_to_detail_name()` handles both layouts

Verify the existing function at line 360 maps both `golden_foo.rs` → `foo.md` and `foo.rs` → `foo.md` correctly (the existing `removeprefix("golden_")` is a no-op for already-de-prefixed names). Add a unit-test fixture covering both stems if not already present.

### 4. Extend `test_golden_inventory.py` fixtures

Add new test cases that cover the post-T2 layout (`tests/scenarios/foo.rs` source files, `Running tests/golden_ai.rs` cargo-output format, module-path-bucketed test names `scenarios::foo::*`). Keep the existing 8 tests intact — they cover the pre-T2 transitional path until ticket 003 removes both code paths.

## Files to Touch

- `scripts/golden_inventory.py` (modify)
- `scripts/test_golden_inventory.py` (modify — additive fixtures)

## Out of Scope

- Any file moves under `crates/worldwake-ai/tests/` — ticket 002's scope
- Any regeneration of `docs/generated/*` — ticket 003's scope
- Retiring the dual-layout branches in `golden_inventory.py` — ticket 003's scope (the cleanup target referenced by this ticket's placeholder)
- Doc edits to `docs/golden-e2e-testing.md`, `docs/debugging-traces.md`, `.claude/skills/*.md`, or `CLAUDE.md` — ticket 005's scope
- `campaigns/golden-perf/harness.sh` — ticket 004's scope

## Acceptance Criteria

### Tests That Must Pass

1. `python3 scripts/golden_inventory.py --write --check-docs` exits 0 against the current (unchanged, pre-T2) test layout
2. Generated artifacts in `docs/generated/golden-e2e-inventory.md`, `docs/generated/golden-scenario-index.md`, `docs/generated/golden-scenario-details/*.md`, `docs/generated/golden-coverage-matrix.md` are byte-identical to the pre-T1 state (`git diff --exit-code docs/generated/` after `--write` returns 0)
3. `python3 scripts/test_golden_inventory.py` passes — all 8 existing tests plus new dual-layout fixtures
4. Existing suite: `cargo test --workspace` passes — proves no Rust-side regression from the Python-only changes

### Invariants

1. Pre-T2 layout regeneration is byte-identical to the pre-T1 committed state — this contract allows ticket 002 to land as a pure file-move ticket
2. Dual-layout code paths carry transitional in-code comments naming ticket 003 as the cleanup target — no orphan fallback survives review

## Test Plan

### New/Modified Tests

1. `scripts/test_golden_inventory.py` — extend with fixtures for post-T2 layout (test paths matching `scenarios/<name>.rs`), post-T2 cargo-output format (`Running tests/golden_ai.rs`), and module-path-bucketed test names. Existing 8 tests cover pre-T2 behavior; new tests cover post-T2 behavior so both dual-layout branches have test coverage.

### Commands

1. `python3 scripts/golden_inventory.py --write --check-docs`
2. `python3 scripts/test_golden_inventory.py`
3. `git diff --exit-code docs/generated/` — confirms byte-identity
4. `scripts/verify.sh` — full workspace gate
