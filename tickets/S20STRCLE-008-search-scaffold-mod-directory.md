# S20STRCLE-008: Scaffold search/ module directory

**Status**: PENDING
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: None
**Deps**: None (independent of S20STRCLE-001..007)

## Problem

`search.rs` is 5880 lines. Before extracting sub-modules, the file must be converted from a flat `.rs` file into a `search/mod.rs` directory module. This ticket creates the directory structure and moves the file with zero code changes.

## Assumption Reassessment (2026-03-22)

1. `search.rs` exists at `crates/worldwake-ai/src/search.rs` with 5880 lines — verified via `wc -l`.
2. `lib.rs` declares `pub mod search;` at line 27 and re-exports `search_plan`, `PlanSearchResult` at line 81.
3. No external crates import `worldwake_ai::search::*` directly — all consumers go through `lib.rs` re-exports.
4–12. N/A — pure structural refactor.

## Architecture Check

1. Same rationale as S20STRCLE-001: isolating the rename into its own ticket makes any breakage trivially diagnosable.
2. No backward-compatibility shims.

## Verification Layers

1. All tests pass → `cargo test -p worldwake-ai`.
2. Single-layer ticket: structural rename only.

## What to Change

### 1. Create directory and move file

- `mkdir crates/worldwake-ai/src/search/`
- `mv crates/worldwake-ai/src/search.rs crates/worldwake-ai/src/search/mod.rs`

No code changes inside the file. No changes to `lib.rs`.

## Files to Touch

- `crates/worldwake-ai/src/search.rs` → `crates/worldwake-ai/src/search/mod.rs` (rename)

## Out of Scope

- Extracting any sub-modules from `search/mod.rs` (that is S20STRCLE-009 through S20STRCLE-012)
- Modifying `agent_tick` (that is S20STRCLE-001..007)
- Any behavioral or logic changes
- Any changes to `lib.rs` re-exports
- Any changes outside `worldwake-ai` crate

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-ai` — all tests pass unchanged
2. `cargo clippy -p worldwake-ai` — no new warnings
3. `cargo build --workspace` — full workspace compiles

### Invariants

1. Zero behavioral change — file rename only
2. All `pub use` re-exports in `lib.rs` continue to resolve
3. No new files created beyond the directory and moved `mod.rs`

## Test Plan

### New/Modified Tests

1. None — structural rename only; verification is command-based.

### Commands

1. `cargo test -p worldwake-ai`
2. `cargo clippy -p worldwake-ai`
3. `cargo build --workspace`
