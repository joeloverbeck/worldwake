# S25-005: Workspace-wide verification

**Status**: ✅ COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: None — verification-only ticket
**Deps**: S25-001, S25-002, S25-003, S25-004

## Problem

Final verification that the complete S25 implementation compiles, passes all tests, and introduces no warnings across the entire 5-crate workspace.

## Assumption Reassessment (2026-03-25)

1. The workspace contains 5 crates: `worldwake-core`, `worldwake-sim`, `worldwake-systems`, `worldwake-ai`, `worldwake-cli`. S25 only modifies `worldwake-ai`, but downstream crates (`worldwake-cli`) depend on it and must still compile.
2. `FeasibilityHint` is re-exported from `worldwake-ai`'s `lib.rs`. If `worldwake-cli` uses `RankedGoal` (e.g., for display), it must be compatible with the new field.
3. `cargo clippy --workspace` must produce no new warnings — the `feasibility.rs` module must not trigger unused-variable, dead-code, or complexity warnings.
4. `cargo test --workspace` includes tests from all 5 crates, not just `worldwake-ai`.

## Architecture Check

1. Workspace verification is the standard final gate for all specs in this project. No architectural decisions to make.
2. No backward-compatibility shims.

## Verification Layers

1. Workspace compilation: `cargo build --workspace`
2. Workspace tests: `cargo test --workspace`
3. Workspace lint: `cargo clippy --workspace`
4. Single-layer ticket — verification is command-based

## What to Change

### 1. Run workspace build

```bash
cargo build --workspace
```

### 2. Run workspace tests

```bash
cargo test --workspace
```

### 3. Run workspace clippy

```bash
cargo clippy --workspace
```

### 4. Fix any issues

If any crate fails to compile due to the new `feasibility` field on `RankedGoal`, add the field initialization at the failing construction site. If clippy reports warnings in S25 code, fix them.

## Files to Touch

- None expected. If compilation issues surface in downstream crates, those files will be identified during verification.

## Out of Scope

- Writing new tests (covered by S25-001 through S25-004)
- Performance optimization
- Documentation updates beyond what's needed for compilation

## Acceptance Criteria

### Tests That Must Pass

1. `cargo build --workspace` — clean compilation
2. `cargo test --workspace` — all tests pass
3. `cargo clippy --workspace` — no new warnings

### Invariants

1. No crate in the workspace has compilation errors
2. No new clippy warnings introduced by S25 changes
3. All pre-existing tests continue to pass

## Test Plan

### New/Modified Tests

1. None — verification-only ticket; verification is command-based and existing runtime coverage is named in Assumption Reassessment.

### Commands

1. `cargo build --workspace`
2. `cargo test --workspace`
3. `cargo clippy --workspace`

## Outcome

- **Completion date**: 2026-03-25
- **What changed**: No code changes required. All three verification commands passed clean across the 5-crate workspace.
- **Deviations**: None.
- **Verification**: `cargo build --workspace` (clean), `cargo test --workspace` (all pass, 0 failures), `cargo clippy --workspace` (no warnings).
