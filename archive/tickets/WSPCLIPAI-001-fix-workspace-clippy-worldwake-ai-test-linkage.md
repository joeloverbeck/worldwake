# WSPCLIPAI-001: Fix workspace clippy `worldwake-ai` test linkage to `worldwake_systems`

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: None — Cargo/test-linkage verification path only
**Deps**: None

## Problem

After the power outage, `cargo clippy --workspace --all-targets -- -D warnings` failed on the live branch with `E0463` import errors involving `worldwake_systems`. This ticket audited whether the failure was a real source-level dependency/linkage bug or stale workspace build state.

## Assumption Reassessment (2026-04-08)

1. Initial reproduction confirmed `cargo clippy --workspace --all-targets -- -D warnings` failed with `E0463` on `worldwake_systems` imports, first observed at `crates/worldwake-ai/src/agent_tick/planning.rs:1177`.
2. Broader reassessment falsified the original narrow root-cause hypothesis: after reproducing, the same clippy session also failed on `crates/worldwake-ai/tests/golden_harness/mod.rs:39` and `crates/worldwake-cli/src/handlers/actions.rs:7`, so the failure was not isolated to one `worldwake-ai` unit-test import or to the S75 control-view refactor boundary.
3. The direct dependents still built cleanly under ordinary compilation surfaces: `cargo build -p worldwake-cli` and `cargo check -p worldwake-cli` both passed while clippy failed. That proved the issue was clippy/build-artifact state, not a normal source-level dependency declaration error.
4. `crates/worldwake-ai/Cargo.toml` and `crates/worldwake-cli/Cargo.toml` already lawfully declare `worldwake-systems` as a normal dependency, and `crates/worldwake-systems/src/lib.rs` exports the cited symbols (`ActionRegistries`, `build_full_action_registries`, `dispatch_table`).
5. A temporary `dev-dependency` experiment on `worldwake-ai` did not fix the problem and was reverted, confirming that dependency scoping was not the actual fix path.
6. Running `cargo clean` for the full workspace cleared the stale artifact state; after that, `cargo clippy --workspace --all-targets -- -D warnings` passed with no source changes required.
7. Shared boundary under audit is therefore the workspace Cargo/clippy artifact state after the outage, not planner logic, S75 trait decomposition, or any production dependency boundary.

## Architecture Check

1. The clean fix is to restore a valid workspace artifact state rather than rewrite lawful imports or duplicate registry helpers. Since the direct dependency graph was already correct, changing code to "fix" the import failures would have violated the repo's no-workaround standard.
2. No backward-compatibility aliases, helper duplication, or boundary changes are warranted once the stale build state is removed.

## Verification Layers

1. Workspace-wide artifact/linkage state restored -> `cargo clippy --workspace --all-targets -- -D warnings`
2. Package-local clippy remains healthy after cleanup -> `cargo clippy -p worldwake-ai --all-targets -- -D warnings`
3. No broader regression from the cleanup action -> `cargo test --workspace`
4. Strongest proof surface is build/lint target resolution itself; no action trace, event-log, or golden-layer assertions are applicable.

## What to Change

### 1. Audit the failure boundary

Reproduce the failure, compare it against direct `cargo build`/`cargo check` surfaces, and confirm whether the problem is a real source-level dependency mismatch or stale workspace build state.

### 2. Restore a clean workspace artifact state

Clear the invalid Cargo/clippy artifacts that survived the outage so the exact workspace verification command can rebuild from a clean state.

### 3. Keep S75 scope separate

Do not reopen the ControlBeliefView refactor unless reassessment proves a real code-level dependency regression. If the problem is build-state only, resolve it without widening into production or test architecture.

## Files to Touch

- None expected in production or test code. This ticket closes through reassessment, artifact cleanup, and factual close-out only.

## Out of Scope

- Any additional RuntimeBeliefView domain decomposition
- Planner behavior changes
- Golden scenario behavior changes
- Trait-surface refactors unrelated to the workspace-clippy target graph

## Acceptance Criteria

### Tests That Must Pass

1. `cargo clippy --workspace --all-targets -- -D warnings`
2. `cargo clippy -p worldwake-ai --all-targets -- -D warnings`
3. `cargo test --workspace`

### Invariants

1. The existing lawful dependency/import paths remain unchanged.
2. No duplicate helper implementations or dependency-shape workarounds are introduced just to satisfy clippy.
3. Planner/runtime behavior and the S75 trait decomposition remain unchanged.

## Test Plan

### New/Modified Tests

1. None — this closed as a validation-only/build-state recovery ticket once reassessment proved the dependency graph was already correct.

### Commands

1. `cargo clippy --workspace --all-targets -- -D warnings`
2. `cargo clippy -p worldwake-ai --all-targets -- -D warnings`
3. `cargo test --workspace`

## Outcome

Completed on 2026-04-08.

- Reproduced the post-outage workspace clippy failure and verified it was not caused by `S75BELVDECOM-001` or by a real missing dependency declaration.
- Confirmed the same symptom appeared across multiple `worldwake-ai` and `worldwake-cli` clippy targets while ordinary `cargo build`/`cargo check` surfaces still passed, which narrowed the issue to stale Cargo/clippy artifact state.
- Restored the workspace to a clean artifact state with `cargo clean`.
- Verified that the exact CI-matching clippy command now passes with no source changes required.
- Reverted the temporary `worldwake-ai` `dev-dependency` experiment because it was not part of the real fix.

## Verification Result

- Passed `cargo clippy --workspace --all-targets -- -D warnings`
- Passed `cargo clippy -p worldwake-ai --all-targets -- -D warnings`
- Passed `cargo test --workspace`
