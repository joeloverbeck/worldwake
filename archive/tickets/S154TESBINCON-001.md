# S154TESBINCON-001: Tooling rewrite — dual-layout `golden_inventory.py`

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: None
**Deps**: None

## Problem

`scripts/golden_inventory.py` and `scripts/test_golden_inventory.py` are tightly coupled to the per-file integration-test layout (`tests/golden_*.rs`). When ticket 002 moves the 54 golden source files into `tests/scenarios/`, the script breaks: its glob misses the new files, its per-file `cargo test --test <stem>` invocation pattern collapses (no per-file binary exists anymore), and its `_file_stem_to_detail_name()` derivation produces wrong detail-page names.

This ticket converts the inventory tooling from a layout-coupled implementation to a layout-agnostic one *before* ticket 002 lands, so the generator remains truthful through the transition and ticket 002 becomes a pure file-move diff.

## Assumption Reassessment (2026-05-19)

1. `scripts/golden_inventory.py` currently globs `tests/golden_*.rs` at three sites (lines 77, 103, 226 per the spec; verified during reassessment) and runs `cargo test --test <stem> -- --list` per file at line 237. `_file_stem_to_detail_name()` lives at line 360 and already does `removeprefix("golden_")` (a safe no-op for already-de-prefixed names), so handling `foo.rs` is essentially confirmation that existing semantics work for the post-move case.
2. `scripts/test_golden_inventory.py` has 8 existing tests covering current behavior: `test_parse_source_inventory_reads_per_file_golden_functions:19`, `test_parse_cargo_test_list_output_groups_tests_by_binary:39`, `test_parse_source_scenarios_reads_identifier_title_and_tests:66`, `test_parse_source_scenarios_accepts_letter_suffix_identifiers:112`, `test_validate_doc_test_references_flags_stale_names:136`, `test_validate_scenarios_flags_duplicates_empty_blocks_and_missing_compiled_tests:160`, `test_render_inventory_markdown_reports_summary_and_files:208`, `test_render_scenario_markdown_reports_primary_and_replay_tests:224`. All 8 embed the pre-T2 cargo-output format and per-file binary layout; the dual-layout work extends fixtures additively without breaking the existing 8.
3. Shared boundary under audit: the inventory script's *public* contract is the truthfulness and stability of `docs/generated/golden-e2e-inventory.md`, `docs/generated/golden-scenario-index.md`, `docs/generated/golden-scenario-details/*.md`, and `docs/generated/golden-coverage-matrix.md` when run via `python3 scripts/golden_inventory.py --write --check-docs`. The dual-layout work must keep the current pre-T2 layout authoritative while adding the post-T2 interpretation.
4. Implementation-time correction: the required `python3 scripts/golden_inventory.py --write --check-docs` proof exposed pre-existing generated-doc drift before any source-file moves. The live source already has `familiar_failed_attempt_accounting_excludes_search_only_sibling_failures` at `crates/worldwake-ai/tests/golden_survival_preferences.rs:382`, but `docs/generated/golden-e2e-inventory.md`, `docs/generated/golden-scenario-index.md`, and `docs/generated/golden-scenario-details/survival-preferences.md` did not list it and had stale Scenario 171 line numbers. This ticket includes that generated-doc freshness correction as required proof fallout; ticket 003 still owns post-T2 regeneration and fallback retirement.

## Architecture Check

1. Dual-layout glob is the cleanest transition path: ticket 002's massive file rename would otherwise have to coordinate with inventory-script regeneration in the same diff, conflating two unrelated concerns. By making the script accept both layouts first, ticket 002 becomes a pure file-move ticket and ticket 003 a pure regeneration-plus-cleanup ticket.
2. The dual-layout code is explicitly transitional and retired in ticket 003 (placeholder-replace pattern). No fossilized fallback survives the migration: ticket 003 drops both transitional branches (`tests/golden_*.rs` glob and per-file `cargo test --test <stem>` invocation) and verifies via grep that zero matches remain in glob/invocation code paths.

## Verified Layers

1. Generated-doc output for pre-T2 layout → `python3 scripts/golden_inventory.py --write --check-docs` exits 0; implementation-time proof corrected pre-existing generated-doc drift for Scenario 171 rather than preserving stale byte identity.
2. Test-fixture coverage for both layouts → `python3 scripts/test_golden_inventory.py` passes with post-T2 fixtures covering the scenario-directory layout in addition to the existing pre-T2 tests.
3. Single-layer ticket: pure tooling-script change with no engine effects. Action-trace / event-log / decision-trace verification surfaces are not applicable.

## Landed Changes

### 1. Extend `golden_inventory.py` glob discovery

The three pre-existing glob sites in `scripts/golden_inventory.py` now discover scenarios from BOTH `crates/worldwake-ai/tests/scenarios/*.rs` and `crates/worldwake-ai/tests/golden_*.rs` through `_golden_source_paths()`. Results are deduplicated by source-file stem, and the scenario-directory path wins when both layouts contain the same de-prefixed stem.

This branch is placeholder, replaced by ticket 003 — mark with an in-code comment naming ticket 003 as the cleanup target so reviewers of this ticket understand the implementation is intentionally provisional.

### 2. Update cargo-test invocation logic

`run_cargo_test_list()` now checks for `crates/worldwake-ai/tests/golden_ai.rs`; when present, it invokes `cargo test -p worldwake-ai --test golden_ai -- --list` and buckets discovered tests by their module path (`scenarios::<name>::*`). Otherwise, it falls back to per-file `cargo test --test <stem> -- --list` invocations for the pre-T2 behavior.

Placeholder, replaced by ticket 003 — same in-code comment convention as section 1.

### 3. Confirm `_file_stem_to_detail_name()` handles both layouts

The existing `_file_stem_to_detail_name()` mapping remains valid for both `golden_foo.rs` → `foo.md` and `foo.rs` → `foo.md`; the post-T2 source parsing tests cover already-de-prefixed names.

### 4. Extend `test_golden_inventory.py` fixtures

The test file now includes cases for the post-T2 layout (`tests/scenarios/foo.rs` source files, `Running tests/golden_ai.rs` cargo-output format, module-path-bucketed test names `scenarios::foo::*`) while retaining the pre-T2 transitional coverage until ticket 003 removes both code paths.

## Landed Files

- `scripts/golden_inventory.py` (modify)
- `scripts/test_golden_inventory.py` (modify — additive fixtures)
- `docs/generated/golden-e2e-inventory.md` (regenerate — pre-existing Scenario 171 freshness correction)
- `docs/generated/golden-scenario-index.md` (regenerate — pre-existing Scenario 171 line/test correction)
- `docs/generated/golden-scenario-details/survival-preferences.md` (regenerate — pre-existing Scenario 171 line/test correction)
- `specs/S154-test-binary-consolidation.md` (truth-sync T1 verification wording)
- `archive/tickets/S154TESBINCON-002.md` (post-archive dependency/path repair)

## Out of Scope

- Any file moves under `crates/worldwake-ai/tests/` — ticket 002's scope
- Post-T2 regeneration of `docs/generated/*` — ticket 003's scope. This ticket includes only the pre-existing generated-doc freshness correction exposed by its required pre-T2 proof command.
- Retiring the dual-layout branches in `golden_inventory.py` — ticket 003's scope (the cleanup target referenced by this ticket's placeholder)
- Doc edits to `docs/golden-e2e-testing.md`, `docs/debugging-traces.md`, `.claude/skills/*.md`, or `CLAUDE.md` — ticket 005's scope
- `campaigns/golden-perf/harness.sh` — ticket 004's scope

## Acceptance Result

### Verification Commands

1. `python3 scripts/golden_inventory.py --write --check-docs` exits 0 against the current (unchanged, pre-T2) test layout
2. Generated artifacts in `docs/generated/golden-e2e-inventory.md`, `docs/generated/golden-scenario-index.md`, and `docs/generated/golden-scenario-details/survival-preferences.md` are refreshed for the pre-existing Scenario 171 drift exposed by `--write`; `docs/generated/golden-coverage-matrix.md` remains unchanged.
3. `python3 scripts/test_golden_inventory.py` passes — existing pre-T2 tests plus post-T2 dual-layout fixtures
4. Existing suite: `cargo test --workspace` passes — proves no Rust-side regression from the Python-only changes

### Invariants

1. Pre-T2 layout regeneration is stable and truthful for the live source tree; ticket 002 can still land as a pure file-move ticket because this ticket does not move test files.
2. Dual-layout code paths carry transitional in-code comments naming ticket 003 as the cleanup target — no orphan fallback survives review

## Test Plan Result

### Focused Tests

1. `scripts/test_golden_inventory.py` — added fixtures for post-T2 layout (test paths matching `scenarios/<name>.rs`), post-T2 cargo-output format (`Running tests/golden_ai.rs`), and module-path-bucketed test names. Existing tests continue to cover pre-T2 behavior, so both dual-layout branches have test coverage.

### Commands Run

1. `python3 scripts/golden_inventory.py --write --check-docs`
2. `python3 scripts/test_golden_inventory.py`
3. `git diff -- docs/generated/` — confirms the only generated-doc changes are the pre-existing Scenario 171 freshness correction
4. `scripts/verify.sh` — full workspace gate

## Outcome

Completed on 2026-05-19.

- `scripts/golden_inventory.py` now discovers both pre-T2 top-level `tests/golden_*.rs` files and post-T2 `tests/scenarios/*.rs` files through one source-path helper, deduplicating by de-prefixed scenario stem and preferring the scenario subdirectory when both layouts exist.
- `run_cargo_test_list()` now uses `golden_ai` when the consolidated entry binary exists and buckets `scenarios::<name>::*` list output back to `<name>.rs`; otherwise it keeps the pre-T2 per-file fallback. Both transitional branches name `S154TESBINCON-003` as the cleanup owner.
- `parse_cargo_test_list_output()` now understands both `Running tests/golden_<name>.rs` and `Running tests/golden_ai.rs` output and excludes nested harness-module tests from per-source inventories.
- `scripts/test_golden_inventory.py` now imports the dataclass module correctly under Python 3.10 and covers pre-T2 source parsing, post-T2 `scenarios/` parsing, duplicate-layout preference, pre-T2 cargo-list parsing, and post-T2 `golden_ai` module-path parsing.
- The required generator proof refreshed pre-existing Scenario 171 generated-doc drift: `docs/generated/golden-e2e-inventory.md`, `docs/generated/golden-scenario-index.md`, and `docs/generated/golden-scenario-details/survival-preferences.md` now list `familiar_failed_attempt_accounting_excludes_search_only_sibling_failures` and current source line references.
- `specs/S154-test-binary-consolidation.md` was truthed so T1 allows a pre-existing generated-doc freshness correction while T3 remains the owner for post-T2 regeneration and dual-layout fallback retirement.

## Deviations

- The draft expected `docs/generated/*` to stay byte-identical after the T1 proof command. Live reassessment disproved that: the source tree already contained a Scenario 171 test missing from generated docs. The landed boundary keeps the generated-doc refresh because it is the canonical output of the required generator command and leaves post-move regeneration to ticket 003.
- The drafted `golden_ai` existence check was implemented as an entry-file check for `crates/worldwake-ai/tests/golden_ai.rs`, followed by the real `cargo test -p worldwake-ai --test golden_ai -- --list` proof path. In the pre-T2 layout that file does not exist, so the existing per-file fallback remains authoritative.

## Verification Result

- Passed `python3 scripts/test_golden_inventory.py` (11 tests).
- Passed `python3 scripts/golden_inventory.py --write --check-docs` (`golden inventory ok: 54 files, 54 contributing files, 260 tests, 196 scenario blocks`).
- Passed `git diff --check`.
- Passed `cargo test --workspace` through `./scripts/verify.sh`.
- Passed `scripts/verify.sh` / `./scripts/verify.sh` (fmt check, workspace tests, active-goal/artifact/debug-view shell checks, workspace clippy, all-target clippy with `-D warnings`, and scenario coverage check).
