# S20STRCLE-001: Scaffold agent_tick/ module directory

**Status**: ✅ COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: None
**Deps**: None

## Problem

`agent_tick.rs` is 6124 lines. Before extracting sub-modules, the file must be converted from a flat `.rs` file into a `agent_tick/mod.rs` directory module. This ticket creates the directory structure and moves the file with zero code changes.

## Assumption Reassessment (2026-03-22)

1. `agent_tick.rs` exists at `crates/worldwake-ai/src/agent_tick.rs` with 6124 lines — verified via `wc -l`.
2. `lib.rs` declares `pub mod agent_tick;` at line 6 and re-exports `AgentTickDriver`, `JourneyDebugSnapshot`, `JourneySwitchMarginSource` at line 29.
3. No external crates import `worldwake_ai::agent_tick::*` directly — all golden tests and CLI go through `lib.rs` re-exports.
4. Not an AI regression ticket — pure structural refactor.
5. N/A — no ordering dependency.
6. N/A — no heuristic changes.
7. N/A — not a start-failure ticket.
8. N/A — not a political ticket.
9. N/A — no ControlSource changes.
10. N/A — no golden scenario.
11. No mismatches found.
12. N/A — no arithmetic.

## Architecture Check

1. Converting `agent_tick.rs` → `agent_tick/mod.rs` is a prerequisite for all subsequent sub-module extractions. Doing it as a standalone ticket isolates risk: if the rename breaks anything, the diff is trivially reviewable.
2. No backward-compatibility shims — `pub mod agent_tick;` in `lib.rs` works identically for both file and directory modules in Rust.

## Verification Layers

1. All golden tests pass → `cargo test -p worldwake-ai` (full crate test suite proves no import breakage).
2. Single-layer ticket: structural rename only, no logic change — additional layer mapping is not applicable.

## What to Change

### 1. Create directory and move file

- `mkdir crates/worldwake-ai/src/agent_tick/`
- `mv crates/worldwake-ai/src/agent_tick.rs crates/worldwake-ai/src/agent_tick/mod.rs`

No code changes inside the file. No changes to `lib.rs`.

## Files to Touch

- `crates/worldwake-ai/src/agent_tick.rs` → `crates/worldwake-ai/src/agent_tick/mod.rs` (rename)

## Out of Scope

- Extracting any sub-modules from `agent_tick/mod.rs` (that is S20STRCLE-002 through S20STRCLE-007)
- Modifying `search.rs` (that is S20STRCLE-008+)
- Any behavioral or logic changes
- Any changes to `lib.rs` re-exports
- Any changes outside `worldwake-ai` crate

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-ai` — all unit + golden tests pass unchanged
2. `cargo clippy -p worldwake-ai` — no new warnings
3. `cargo build --workspace` — full workspace compiles

### Invariants

1. Zero behavioral change — this is a file rename only
2. All `pub use` re-exports in `lib.rs` continue to resolve
3. No new files created beyond the directory and moved `mod.rs`

## Test Plan

### New/Modified Tests

1. None — structural rename only; verification is command-based and existing runtime coverage is named in Assumption Reassessment.

### Commands

1. `cargo test -p worldwake-ai`
2. `cargo clippy -p worldwake-ai`
3. `cargo build --workspace`

## Outcome

- **Completion date**: 2026-03-23
- **What changed**: Moved `crates/worldwake-ai/src/agent_tick.rs` to `crates/worldwake-ai/src/agent_tick/mod.rs`. No code changes.
- **Deviations**: None.
- **Verification**: `cargo test -p worldwake-ai` — 21 passed, 0 failed. `cargo clippy -p worldwake-ai` — clean. `cargo build --workspace` — success.
