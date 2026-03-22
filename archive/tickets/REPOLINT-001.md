# REPOLINT-001: Make strict Clippy enforcement real for all targets

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: None
**Deps**: None

## Problem

The repository currently gives a false sense of lint cleanliness.

The repository still has a split verification contract.

The strict target-graph lint gate already exists in preview form, but it is not yet the canonical blocking verification path because `cargo clippy --workspace --all-targets -- -D warnings` still fails on pre-existing issues across tests and test-only docs. That means the repo still advertises a stricter standard than it can currently enforce as the single source of truth.

We need to close that backlog and then collapse verification onto one real strict gate.

## Assumption Reassessment (2026-03-18)

1. Workspace-level Clippy policy is already strict in [Cargo.toml](/home/joeloverbeck/projects/worldwake/Cargo.toml) via `all = deny` and `pedantic = deny` — confirmed.
2. Canonical local verification already exists in [scripts/verify.sh](/home/joeloverbeck/projects/worldwake/scripts/verify.sh), and [README.md](/home/joeloverbeck/projects/worldwake/README.md) already points to it — confirmed.
3. CI already exists in [.github/workflows/ci.yml](/home/joeloverbeck/projects/worldwake/.github/workflows/ci.yml): baseline verification is blocking, while strict lint runs only as a non-blocking preview — confirmed.
4. `cargo clippy --workspace --all-targets -- -D warnings` currently fails on pre-existing issues in `worldwake-ai` test targets and lib-test docs plus one `worldwake-systems` test helper — confirmed by local run on 2026-03-18.
5. Because docs, script, and CI surfaces already exist, the remaining implementation scope is not “add verification plumbing”; it is “clear the strict backlog and promote the existing plumbing to one canonical strict contract.”

## Architecture Check

1. The correct long-term architecture is one canonical strict verification contract for humans and CI, not a permanent split between baseline and preview modes.
2. The clean path is to make [scripts/verify.sh](/home/joeloverbeck/projects/worldwake/scripts/verify.sh) run the strict contract and have CI call that same path, rather than preserving parallel “baseline” and “strict preview” workflows indefinitely.
3. For test-only pedantic findings, narrowly scoped local `#[allow(...)]` is acceptable when refactoring would reduce scenario clarity without improving production architecture. No workspace-wide weakening is allowed.

## What to Change

### 1. Audit and fix the existing strict-lint backlog

- Group failures by crate and by lint family.
- Prefer small mechanical fixes where possible.
- Add narrowly-scoped `#[allow(...)]` only when the lint is genuinely counterproductive for the local code shape.
- Do not weaken workspace lint policy globally.

### 2. Collapse verification onto the strict gate

- Make `scripts/verify.sh` run the full strict verification contract by default.
- Promote strict all-target linting from preview-only to blocking CI verification.
- Remove the long-term baseline/preview split once the repo is green.

### 3. Keep verification surfaces aligned

- Ensure local developer guidance, tickets, and CI all refer to the same canonical verify command and behavior.

## Files to Touch

- `Cargo.toml` (modify only if targeted lint allowances are justified)
- `.github/workflows/ci.yml` (modify)
- `scripts/verify.sh` (modify)
- `README.md` (modify)
- affected crate files that currently fail strict Clippy

## Out of Scope

- weakening `pedantic` or `all` at workspace scope
- keeping a permanent preview-only strict job once the backlog is cleared
- unrelated refactors disguised as lint cleanup

## Acceptance Criteria

### Tests That Must Pass

1. `cargo clippy --workspace` passes
2. `cargo clippy --workspace --all-targets -- -D warnings` passes
3. `cargo test --workspace` still passes
4. `scripts/verify.sh` runs the same strict contract CI enforces

### Invariants

1. Strict lint enforcement applies to the full target graph, not just library/default targets.
2. CI and local verification reference the same canonical command and behavior.

## Test Plan

### New/Modified Tests

1. N/A expected unless a lint fix reveals missing behavioral coverage; this ticket is primarily verification-policy and lint-backlog work.

### Commands

1. `cargo clippy --workspace`
2. `cargo clippy --workspace --all-targets -- -D warnings`
3. `cargo test --workspace`
4. `scripts/verify.sh`

## Outcome

- Completion date: 2026-03-18
- What changed:
  - Cleared the current strict `clippy --all-targets` backlog with small mechanical fixes and narrowly scoped test-only `#[allow(...)]` annotations where pedantic refactors would have reduced scenario readability without improving production architecture.
  - Made `scripts/verify.sh` the canonical strict verification path.
  - Removed the non-blocking strict preview split from CI so `.github/workflows/ci.yml` now relies on the same canonical verification command.
  - Updated `README.md` to describe the single strict verification contract.
- Deviations from original plan:
  - No workspace lint policy weakening was needed.
  - No new behavioral tests were added; the work was verification-policy plus lint backlog cleanup, and existing coverage remained sufficient.
- Verification results:
  - `cargo clippy --workspace` passed.
  - `cargo clippy --workspace --all-targets -- -D warnings` passed.
  - `cargo test --workspace` passed.
  - `./scripts/verify.sh` passed.
