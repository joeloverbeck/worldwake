# S143STABELVIE-005: CI grep lint forbidding `DebugWorldView` in `worldwake-ai/src/`

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: None (CI tooling; no Rust source changes)
**Deps**: S143STABELVIE-002

## Problem

Spec D7 mandates an automated check that no source file under `crates/worldwake-ai/src/**` imports `DebugWorldView`. Without it, the spec's enforcement of the FND-14A trait wall relies solely on the cfg-gate (which only blocks release builds; debug-mode builds could silently land `DebugWorldView` imports in planner code) and on code-review discipline. The grep-CI script closes that gap by failing CI when the import appears at any time, in any build mode.

## Assumption Reassessment (2026-05-13)

1. Established precedents: `scripts/check_active_goal_removed.sh` (21 grep patterns covering deprecated `ActiveGoal` accessors) and `scripts/check_no_artifact_state.sh` (`\bArtifactState\b`). Both are wired into `scripts/verify.sh` between the `cargo clippy` and `cargo test` stages. The new lint follows the same shape.
2. `ripgrep` (`rg`) is the established tool — both precedents use `rg -l '<pattern>'` to list matching files, with `set -euo pipefail` as the standard prologue. The new script adopts both conventions.
3. The lint scope is `crates/worldwake-ai/src/**` (production source) only — `crates/worldwake-ai/tests/**` is excluded because tests can legitimately import `DebugWorldView` under the cfg-gate for assertion purposes.

## Architecture Check

1. Grep-CI matches the established enforcement pattern in worldwake; no custom clippy lint plugin is introduced (the codebase has zero custom clippy lints; building one would add disproportionate tooling complexity for a single import-restriction check).
2. The script lives in `scripts/` alongside its precedents — co-located for easy discovery and uniform invocation from `verify.sh`.
3. Single-purpose script — does one thing (one grep, one decision); trivial to maintain.

## Verification Layers

Single-layer ticket (CI enforcement). The script's pass/fail is the proof surface. No simulation runtime is touched, no behavioral invariants beyond "no DebugWorldView import" are at stake.

## What to Change

### 1. New script `scripts/check_no_debug_view_in_ai.sh`

Verbatim from spec D7:

```bash
#!/usr/bin/env bash
set -euo pipefail
matches="$(rg -l 'DebugWorldView' crates/worldwake-ai/src 2>/dev/null || true)"
if [ -n "$matches" ]; then
  echo "DebugWorldView illegally imported in worldwake-ai:" >&2
  echo "$matches" >&2
  exit 1
fi
```

The script must be executable (`chmod +x scripts/check_no_debug_view_in_ai.sh`).

### 2. Wire into `scripts/verify.sh`

Add the invocation alongside the existing precedent scripts (between `cargo clippy` and `cargo test`, mirroring `check_active_goal_removed.sh` and `check_no_artifact_state.sh` placement):

```bash
echo "[verify] bash scripts/check_no_debug_view_in_ai.sh"
bash scripts/check_no_debug_view_in_ai.sh
```

## Files to Touch

- `scripts/check_no_debug_view_in_ai.sh` (new)
- `scripts/verify.sh` (modify — add 2-line invocation)

## Out of Scope

- Custom clippy lint plugin (deliberately excluded per spec D7's "No custom clippy lint is introduced" framing).
- Enforcement on test files (`crates/worldwake-ai/tests/**` may import `DebugWorldView` under the cfg-gate).
- Enforcement on other crates (`worldwake-sim`, `worldwake-systems`, `worldwake-cli`, `worldwake-visualizer`) — the spec scopes the prohibition to `worldwake-ai/src/**` (the planner library surface that must not reach release-mode authoritative reads).

## Acceptance Criteria

### Tests That Must Pass

1. `bash scripts/check_no_debug_view_in_ai.sh` exits 0 against the current codebase (no `worldwake-ai/src/**.rs` file imports `DebugWorldView` after tickets 002–004).
2. Synthetic test: introducing a `use worldwake_sim::DebugWorldView;` line into any file under `crates/worldwake-ai/src/` causes the script to exit 1 with the offending file path in stderr. Verified manually during ticket implementation; not committed.
3. `scripts/verify.sh` completes end-to-end after wiring (no spurious failures from the new step).

### Invariants

1. After this ticket lands, no `worldwake-ai/src/**.rs` file imports `DebugWorldView` (verified by the script's exit code in CI).
2. The script is reachable from `scripts/verify.sh` (the canonical pre-PR gate per CLAUDE.md's Pre-PR Verification section).

## Test Plan

### New/Modified Tests

None — documentation-only ticket; verification is command-based and existing runtime coverage is named in Assumption Reassessment.

### Commands

1. `bash scripts/check_no_debug_view_in_ai.sh` (must exit 0 against the current codebase)
2. `scripts/verify.sh` (must complete; the new step is wired in)
