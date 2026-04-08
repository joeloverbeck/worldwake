# WSPCLIPAI-001: Fix workspace clippy `worldwake-ai` test linkage to `worldwake_systems`

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: None — Cargo/test-linkage verification path only
**Deps**: None

## Problem

`cargo clippy --workspace --all-targets -- -D warnings` fails on the live branch even though `cargo clippy -p worldwake-ai --all-targets -- -D warnings` passes. The failure occurs when the workspace-wide clippy run builds the `worldwake-ai` lib test target and resolves unit-test imports of `worldwake_systems`.

## Assumption Reassessment (2026-04-08)

1. The first live failure is `E0463: can't find crate for worldwake_systems` from `crates/worldwake-ai/src/agent_tick/planning.rs:1177` during `cargo clippy --workspace --all-targets -- -D warnings`.
2. The failing symbol is the unit-test import `use worldwake_systems::build_full_action_registries;` inside the `#[cfg(test)]` module in `crates/worldwake-ai/src/agent_tick/planning.rs`.
3. The same source tree passes `cargo clippy -p worldwake-ai --all-targets -- -D warnings`, so the defect is not a normal package-level missing dependency. The divergence is specific to the workspace-wide clippy target graph.
4. A temporary dependency-shaping experiment showed `worldwake-ai` binaries such as `perf_diag` and `soak_seed_perf` include `tests/golden_harness/mod.rs`, which also imports `worldwake_systems`; that means the crate is lawfully needed by more than pure integration tests on this branch.
5. Shared boundary under audit: Cargo/clippy target resolution for `worldwake-ai` unit-test and binary test-related build paths, not planner logic, ControlBeliefView decomposition, or any authoritative simulation contract.
6. This issue was exposed while verifying `tickets/S75BELVDECOM-001-extract-control-belief-view.md`, but the failure boundary is adjacent build tooling, not the `RuntimeBeliefView`/`ControlBeliefView` trait split itself.
7. No golden, planner, or authoritative-world invariant is under audit here. The contract is strictly "workspace clippy must build the same lawful test-support imports that package-level clippy already accepts."

## Architecture Check

1. The clean fix is to repair the real Cargo/test-support boundary so workspace-wide clippy resolves `worldwake_systems` lawfully for the affected `worldwake-ai` targets. This is cleaner than weakening imports, duplicating registry builders, or adding workaround shims just to satisfy clippy.
2. No backward-compatibility aliases or code-path duplication should be introduced. If a helper currently lives in the wrong place for the workspace target graph, move or re-expose it cleanly at the owning boundary instead of papering over the linkage failure.

## Verification Layers

1. Workspace-wide test-support linkage -> `cargo clippy --workspace --all-targets -- -D warnings`
2. `worldwake-ai` package-local linkage remains intact -> `cargo clippy -p worldwake-ai --all-targets -- -D warnings`
3. No behavioral regression from any helper relocation or import-path cleanup -> `cargo test --workspace`
4. Strongest proof surface is build/lint target resolution itself; no action trace, event-log, or golden-layer assertions are applicable.

## What to Change

### 1. Identify the exact workspace target-graph mismatch

Trace why workspace-wide clippy resolves the `worldwake-ai` lib test target differently from the package-local clippy invocation even though both compile the same `#[cfg(test)]` module imports.

### 2. Repair the lawful helper/dependency boundary

Make the minimal clean change so the affected `worldwake-ai` unit-test and binary-support code can resolve `worldwake_systems` under the workspace-wide target graph without introducing duplicate helper implementations or weakening module boundaries.

### 3. Keep S75 scope separate

Do not reopen the ControlBeliefView refactor unless the root cause proves that ticket changed the failing target boundary. If the problem is independent, keep the fix isolated to the Cargo/import/support layout that actually owns it.

## Files to Touch

- `crates/worldwake-ai/Cargo.toml` (modify if the root cause is dependency scoping)
- `crates/worldwake-ai/src/agent_tick/planning.rs` (modify only if import structure is part of the root cause)
- `crates/worldwake-ai/src/bin/perf_diag.rs` (modify if binary test-support inclusion is part of the root cause)
- `crates/worldwake-ai/src/bin/soak_seed_perf.rs` (modify if binary test-support inclusion is part of the root cause)
- `crates/worldwake-ai/tests/golden_harness/mod.rs` (modify only if helper placement/import ownership is the real issue)

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

1. `worldwake-ai` keeps a single lawful way to access any shared registry/test-support helpers used by its unit tests or binary support code.
2. No duplicate helper implementations are introduced just to satisfy workspace clippy.
3. The fix does not widen or alter planner/runtime behavior.

## Test Plan

### New/Modified Tests

1. None expected — this is a verification-path repair unless reassessment proves a focused regression test is needed for helper placement.

### Commands

1. `cargo clippy --workspace --all-targets -- -D warnings`
2. `cargo clippy -p worldwake-ai --all-targets -- -D warnings`
3. `cargo test --workspace`
