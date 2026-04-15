# S104SURBASREC-002: Triage mixed golden test files

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: None
**Deps**: archive/tickets/S104SURBASREC-001.md

## Problem

Six golden test files contain a mix of invariant-based tests and hash/sequence-dependent tests. They require per-test review to determine which tests to keep, which to remove, and whether any files should be split. After triage is complete, golden documentation must be regenerated to reflect the new test inventory.

## Assumption Reassessment (2026-04-15)

1. All 6 TRIAGE files exist in `crates/worldwake-ai/tests/` — confirmed during reassessment:
   - `golden_ai_decisions.rs` (19 tests, 2 hash calls)
   - `golden_experience_preferences.rs` (6 tests, 4 hash calls)
   - `golden_merchant_selling.rs` (20 tests, 10 hash calls)
   - `golden_offices.rs` (24 tests, 19 hash calls)
   - `golden_planner_pathology.rs` (4 tests, 2 hash calls)
   - `golden_simulation_gaps.rs` (10 tests, 11 hash calls)
2. `scripts/golden_inventory.py` exists and supports `--write --check-docs` flags — confirmed during reassessment.
3. Generated doc targets exist: `docs/generated/golden-scenario-index.md`, `docs/generated/golden-coverage-matrix.md`, `docs/generated/golden-e2e-inventory.md`, `docs/generated/golden-scenario-details/*.md`.
4. Factual follow-up from archived `S104SURBASREC-001`: Rust integration tests in `crates/worldwake-ai/tests/` are file-based and do not use a shared `mod golden_*;` entry point. If a TRIAGE file is deleted during this ticket, removing the file is sufficient; no central harness-module edit is expected.

## Architecture Check

1. Per-test review is more precise than whole-file deletion — preserves invariant-based tests that remain valid regardless of priority ordering changes, while removing only the hash/sequence-dependent tests that would break.
2. No backwards-compatibility shims introduced. Tests are either kept as-is, removed, or split into separate files.

## Verification Layers

1. Remaining tests pass → `cargo test -p worldwake-ai`
2. Golden docs regenerated → `python3 scripts/golden_inventory.py --write --check-docs` completes without error
3. Single-layer ticket — test infrastructure only, no runtime changes.

## What to Change

### 1. Review each TRIAGE file per-test

For each test function in the 6 TRIAGE files, classify as:
- **KEEP**: Tests structural invariants, no StateHash calls, no tick-specific action sequence assertions
- **REMOVE**: Uses StateHash or asserts specific goal/action sequences at specific ticks

Triage notes from the spec:
- `golden_ai_decisions.rs`: Low hash count (2), heavy invariants (64). Scenario 1 tests two hungry agents — directly relevant to survival. Most tests likely KEEP.
- `golden_experience_preferences.rs`: Small file, route preference learning may not depend on goal ordering.
- `golden_merchant_selling.rs`: Enterprise-weighted, may be insensitive to survival reordering.
- `golden_offices.rs`: High hash count (19), enterprise-weighted. Many tests likely REMOVE.
- `golden_planner_pathology.rs`: Low hash count (2), pathology-focused tests may be goal-ordering independent.
- `golden_simulation_gaps.rs`: Mixed invariants and hashes, gap-specific tests need individual review.

### 2. Apply triage decisions

- For files where ALL tests are KEEP: leave unchanged
- For files where ALL tests are REMOVE: delete the file
- For mixed files: remove individual REMOVE test functions and their associated setup code. If this leaves the file with only 1-2 tests, consider whether keeping the file is worthwhile or if remaining tests should be moved to an existing KEEP file.

### 3. Regenerate golden documentation

Run: `python3 scripts/golden_inventory.py --write --check-docs`

This regenerates:
- `docs/generated/golden-scenario-index.md`
- `docs/generated/golden-coverage-matrix.md`
- `docs/generated/golden-e2e-inventory.md`
- `docs/generated/golden-scenario-details/*.md`

## Files to Touch

- `crates/worldwake-ai/tests/golden_ai_decisions.rs` (modify or keep)
- `crates/worldwake-ai/tests/golden_experience_preferences.rs` (modify or delete)
- `crates/worldwake-ai/tests/golden_merchant_selling.rs` (modify or delete)
- `crates/worldwake-ai/tests/golden_offices.rs` (modify or delete)
- `crates/worldwake-ai/tests/golden_planner_pathology.rs` (modify or keep)
- `crates/worldwake-ai/tests/golden_simulation_gaps.rs` (modify or delete)
- `docs/generated/golden-scenario-index.md` (regenerate)
- `docs/generated/golden-coverage-matrix.md` (regenerate)
- `docs/generated/golden-e2e-inventory.md` (regenerate)
- `docs/generated/golden-scenario-details/*.md` (regenerate)

## Out of Scope

- KEEP files — not touched
- Already-deleted REMOVE files (handled by S104SURBASREC-001)
- Golden harness infrastructure
- Any behavioral or production code changes
- Creating new golden tests (handled by S104SURBASREC-005 and S104SURBASREC-006)

## Acceptance Criteria

### Tests That Must Pass

1. All remaining golden tests pass: `cargo test -p worldwake-ai`
2. Golden inventory script succeeds: `python3 scripts/golden_inventory.py --write --check-docs`
3. Existing suite: `cargo test --workspace`

### Invariants

1. No invariant-based test is removed — only hash/sequence-dependent tests
2. KEEP files remain unchanged
3. Golden documentation matches the current test inventory after regeneration

## Test Plan

### New/Modified Tests

1. None — triage-only ticket; tests are removed or retained, not created.

### Commands

1. `cargo test -p worldwake-ai` — remaining tests pass
2. `python3 scripts/golden_inventory.py --write --check-docs` — docs regenerated
3. `cargo clippy --workspace --all-targets -- -D warnings` — clean
