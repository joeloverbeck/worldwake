# S143STABELVIE-005: CI grep lint forbidding `DebugWorldView` in `worldwake-ai/src/`

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: None (CI tooling; no Rust source changes)
**Deps**: archive/tickets/S143STABELVIE-002.md

## Problem

Spec D7 mandates an automated check that no source file under `crates/worldwake-ai/src/**` imports `DebugWorldView`. Without it, the spec's enforcement of the FND-14A trait wall relies solely on the cfg-gate (which only blocks release builds; debug-mode builds could silently land `DebugWorldView` imports in planner code) and on code-review discipline. The grep-CI script closes that gap by failing CI when the import appears at any time, in any build mode.

## Assumption Reassessment (2026-05-13)

1. Established precedents: `scripts/check_active_goal_removed.sh` (21 grep patterns covering deprecated `ActiveGoal` accessors) and `scripts/check_no_artifact_state.sh` (`\bArtifactState\b`). Both are wired into `scripts/verify.sh` after `cargo test --workspace` and before the clippy stages. The new lint follows the same shape.
2. `ripgrep` (`rg`) is the established tool — both precedents use `rg -l '<pattern>'` to list matching files, with `set -euo pipefail` as the standard prologue. The new script adopts both conventions.
3. The lint scope is `crates/worldwake-ai/src/**` (production source) only — `crates/worldwake-ai/tests/**` is excluded because tests can legitimately import `DebugWorldView` under the cfg-gate for assertion purposes.

## Architecture Check

1. Grep-CI matches the established enforcement pattern in worldwake; no custom clippy lint plugin is introduced (the codebase has zero custom clippy lints; building one would add disproportionate tooling complexity for a single import-restriction check).
2. The script lives in `scripts/` alongside its precedents — co-located for easy discovery and uniform invocation from `verify.sh`.
3. Single-purpose script — does one thing (one grep, one decision); trivial to maintain.

## Verified Layer

Single-layer ticket (CI enforcement). The script's pass/fail is the proof surface. No simulation runtime was touched, and no behavioral invariants beyond "no DebugWorldView import" are at stake.

## Landed Changes

### 1. Added `scripts/check_no_debug_view_in_ai.sh`

The added script follows the established `scripts/check_active_goal_removed.sh` and `scripts/check_no_artifact_state.sh` shape:

```bash
#!/usr/bin/env bash
set -euo pipefail

matches="$(
  rg -l 'DebugWorldView' crates/worldwake-ai/src 2>/dev/null || true
)"

if [ -n "$matches" ]; then
  echo "DebugWorldView illegally imported in worldwake-ai:" >&2
  echo "$matches" >&2
  exit 1
fi

echo "DebugWorldView import check verified: zero references in crates/worldwake-ai/src"
```

The script is executable.

### 2. Wired into `scripts/verify.sh`

The invocation was added beside the existing grep lints, after `check_no_artifact_state.sh` and before the clippy gates:

```bash
echo "[verify] bash scripts/check_no_debug_view_in_ai.sh"
bash scripts/check_no_debug_view_in_ai.sh
```

## Landed Files

- `scripts/check_no_debug_view_in_ai.sh` (added)
- `scripts/verify.sh` (modified)

## Out of Scope

- Custom clippy lint plugin (deliberately excluded per spec D7's "No custom clippy lint is introduced" framing).
- Enforcement on test files (`crates/worldwake-ai/tests/**` may import `DebugWorldView` under the cfg-gate).
- Enforcement on other crates (`worldwake-sim`, `worldwake-systems`, `worldwake-cli`, `worldwake-visualizer`) — the spec scopes the prohibition to `worldwake-ai/src/**` (the planner library surface that must not reach release-mode authoritative reads).

## Acceptance Result

1. `bash scripts/check_no_debug_view_in_ai.sh` exited 0 against the current codebase; no `worldwake-ai/src/**.rs` file imports `DebugWorldView`.
2. A temporary synthetic fixture under `/tmp/worldwake-s143-debug-view-probe/crates/worldwake-ai/src/probe.rs` containing `use worldwake_sim::DebugWorldView;` caused the script to exit 1 and print the offending file path.
3. `scripts/verify.sh` completed end-to-end after wiring. The live wrapper order is `cargo fmt --all -- --check`, `cargo test --workspace`, the three grep scripts, `cargo clippy --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, and `cargo run -p worldwake-cli --bin scenario-coverage -- --check`.

## Outcome

Completed on 2026-05-13.

- Added the `DebugWorldView` import lint for `crates/worldwake-ai/src`.
- Wired the lint into the canonical `scripts/verify.sh` gate beside the existing grep lints.
- Corrected the draft placement note to match the live wrapper order: the grep lints run after workspace tests and before clippy.

## Verification Result

- Passed `bash scripts/check_no_debug_view_in_ai.sh`.
- Passed synthetic negative probe in `/tmp/worldwake-s143-debug-view-probe` with `use worldwake_sim::DebugWorldView;`; the script exited 1 and reported `crates/worldwake-ai/src/probe.rs`.
- Passed `scripts/verify.sh`.
