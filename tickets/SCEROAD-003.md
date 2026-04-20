# SCEROAD-003: Wire `scenario-coverage --check` into `scripts/verify.sh`

**Status**: PENDING
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: None — build/CI script edit only.
**Deps**: SCEROAD-001 (binary must exist and be invokable as `cargo run -p worldwake-cli --bin scenario-coverage -- --check`).

## Problem

Without an automated gate, the committed `docs/generated/scenario-coverage.md` can drift from the actual state of `scenarios/*.ron`, silently invalidating any "Landed" claim in `docs/scenario-roadmap.md` (SCEROAD-002). Design doc §9 specifies CI must diff freshly-generated vs committed content and fail on drift — matching the `scripts/profile_docs.py` and `scripts/golden_inventory.py` precedents already used in this project.
This gate only protects structural coverage drift. It does not certify that a scenario golden still proves the intended causal branch; that validity contract remains owned by `docs/golden-e2e-testing.md` and the roadmap entries authored in SCEROAD-002.

## Assumption Reassessment (2026-04-19)

1. `scripts/verify.sh` exists and currently runs `cargo test --workspace`, `cargo clippy --workspace`, and `cargo clippy --workspace --all-targets -- -D warnings` (confirmed by reading the file). `.github/workflows/ci.yml` invokes `./scripts/verify.sh` as its single verification step.
2. Design doc §9 says "New workflow step after `scripts/profile_docs.py` and `scripts/golden_inventory.py` checks". Verification against the current repo: `scripts/profile_docs.py` and `scripts/golden_inventory.py` are present but are **not** currently invoked from `scripts/verify.sh`. The spec's phrasing assumes an integration pattern that doesn't yet exist in verify.sh. Scope correction: the canonical integration point for the `--check` invocation is `scripts/verify.sh` itself; that is where all other workspace-wide verification lives today. Adding `profile_docs.py --check` / `golden_inventory.py --check-docs` invocations is out of scope — those are pre-existing gaps that predate this spec.
3. Shared abstraction boundary under audit: `scripts/verify.sh` as the single CI aggregator. This ticket adds one step and does not restructure the script.

## Architecture Check

1. **One aggregator, one entry point.** Reusing `scripts/verify.sh` keeps the CI surface unified — no new workflow YAML, no parallel entry point. This matches the project's current CI shape (`.github/workflows/ci.yml` calls `./scripts/verify.sh` and nothing else for workspace verification).
2. **Ordering for fast failure.** Place the `--check` step after `cargo test --workspace` and the two `cargo clippy` steps so compiler/test errors fail first — faster feedback, cheaper CI.
3. **Scope stays honest.** `scenario-coverage --check` confirms activation/doc drift only; it must not be described as a full scenario-validity gate.
4. **No backwards-compat shim.** No prior check to migrate from.

## Verification Layers

1. CI-gate correctness → manually induce drift (hand-edit `docs/generated/scenario-coverage.md`) and confirm `scripts/verify.sh` fails non-zero in the new step; revert and confirm it passes.
2. Step ordering → visual inspection of `scripts/verify.sh` confirms `--check` runs after the existing clippy/test steps.
3. Single-layer ticket: script edit only; no runtime/action/event-log layer applies.

## What to Change

### 1. Add a `--check` step to `scripts/verify.sh`

Append after the existing `cargo clippy --workspace --all-targets -- -D warnings` step:

```bash
echo "[verify] cargo run -p worldwake-cli --bin scenario-coverage -- --check"
cargo run -p worldwake-cli --bin scenario-coverage -- --check
```

Use the same `echo "[verify] ..."` pattern as the existing steps for consistency.

## Files to Touch

- `scripts/verify.sh` (modify)

## Out of Scope

- Adding `--check` invocations for `scripts/profile_docs.py` or `scripts/golden_inventory.py` — those are pre-existing gaps outside this spec's scope.
- Creating a new `.github/workflows/*.yml` — existing `ci.yml` already invokes `verify.sh`.
- Changes to the binary or its output format (owned by SCEROAD-001).
- Changes to `docs/scenario-roadmap.md` (owned by SCEROAD-002).

## Acceptance Criteria

### Tests That Must Pass

1. `./scripts/verify.sh` succeeds on a clean checkout with the committed `docs/generated/scenario-coverage.md` matching freshly-generated output.
2. `./scripts/verify.sh` fails with non-zero exit in the new step when `docs/generated/scenario-coverage.md` is hand-edited to introduce drift (manual verification).
3. CI (`.github/workflows/ci.yml`) surfaces the new step's output in its logs on pull requests.

### Invariants

1. The `--check` step runs **after** `cargo test --workspace` and the two `cargo clippy` steps — compiler/test errors fail first.
2. The step invocation matches the local authoring command exactly (`cargo run -p worldwake-cli --bin scenario-coverage -- --check`) — no divergent flag set between CI and local.
3. `scripts/verify.sh` continues to exit non-zero on any failing step (retains `set -euo pipefail`).
4. Ticket text and resulting script comments do not imply that `scenario-coverage --check` proves backing-golden causal validity.

## Test Plan

### New/Modified Tests

1. None — script edit; verification is command-based and manual.

### Commands

1. `./scripts/verify.sh` on a clean tree — must pass.
2. Induce drift: `echo "DRIFT" >> docs/generated/scenario-coverage.md && ./scripts/verify.sh` — must fail in the new step; then `git checkout -- docs/generated/scenario-coverage.md` to revert.
3. `cargo run -p worldwake-cli --bin scenario-coverage -- --check` — standalone sanity check.
