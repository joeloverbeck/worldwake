# REPOLINT-001: Make strict Clippy enforcement real for all targets

**Status**: PENDING
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: None
**Deps**: None

## Problem

The repository currently gives a false sense of lint cleanliness.

`cargo clippy --workspace` passes, but `cargo clippy --workspace --all-targets -- -D warnings` still fails on pre-existing issues across tests and test-only docs. That means the repo cannot honestly enforce strict linting for the full build graph yet.

We need to close that gap and then make the stricter command the real required gate.

## Assumption Reassessment (2026-03-18)

1. Workspace-level Clippy policy is already strict in `Cargo.toml` via `all = deny` and `pedantic = deny` — confirmed.
2. The current practical verification baseline is only `cargo clippy --workspace` — confirmed by successful local run.
3. `cargo clippy --workspace --all-targets -- -D warnings` currently fails on pre-existing issues in test targets and test-only documentation — confirmed by local run during E16DPOLPLAN-013 follow-up.
4. The repo currently has no `.github/workflows/` CI enforcement at all — confirmed.
5. A new CI workflow can safely enforce the current baseline now, but strict all-targets enforcement must remain non-blocking until the backlog is cleared.

## Architecture Check

1. The correct long-term architecture is one canonical verification contract for humans and CI, not separate undocumented local habits.
2. Strict lint enforcement should become a real gate only after the repository is green under that contract. Pretending otherwise would create noise and brittle policy instead of discipline.

## What to Change

### 1. Audit and fix the existing strict-lint backlog

- Group failures by crate and by lint family.
- Prefer small mechanical fixes where possible.
- Add narrowly-scoped `#[allow(...)]` only when the lint is genuinely counterproductive for the local code shape.
- Do not weaken workspace lint policy globally.

### 2. Make strict all-target linting the required repository gate

- Promote `cargo clippy --workspace --all-targets -- -D warnings` from preview-only to blocking verification.
- Update any local verify docs/scripts so the strict command is the default.

### 3. Keep verification surfaces aligned

- Ensure local developer guidance, tickets, and CI all refer to the same canonical verify command.

## Files to Touch

- `Cargo.toml` (modify only if targeted lint allowances are justified)
- `.github/workflows/ci.yml` (modify)
- `scripts/verify.sh` (modify)
- affected crate files that currently fail strict Clippy

## Out of Scope

- weakening `pedantic` or `all` at workspace scope
- unrelated refactors disguised as lint cleanup

## Acceptance Criteria

### Tests That Must Pass

1. `cargo clippy --workspace` passes
2. `cargo clippy --workspace --all-targets -- -D warnings` passes
3. `cargo test --workspace` still passes

### Invariants

1. Strict lint enforcement applies to the full target graph, not just library/default targets.
2. CI and local verification reference the same canonical commands.

## Test Plan

### New/Modified Tests

1. N/A — verification ticket

### Commands

1. `cargo clippy --workspace`
2. `cargo clippy --workspace --all-targets -- -D warnings`
3. `cargo test --workspace`
4. `scripts/verify.sh --strict`
