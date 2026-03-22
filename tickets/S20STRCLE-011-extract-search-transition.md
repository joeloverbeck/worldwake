# S20STRCLE-011: Extract search/transition.rs

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: None
**Deps**: S20STRCLE-008

## Problem

The successor-building logic in `search/mod.rs` (~400 lines) handles hypothetical state transitions during plan search. Extracting it into a dedicated sub-module isolates the "what happens if the agent takes this action?" simulation concern.

## Assumption Reassessment (2026-03-22)

1. Functions to extract (verified via grep):
   - `build_successor()` (line 468, `#[cfg(test)]` gated variant)
   - `build_successor_detailed()` (line 489)
   - `terminal_kind()` (line 938)
2. `build_successor()` is `#[cfg(test)]`-only (line 467). `build_successor_detailed()` is the production entry point.
3–12. N/A — pure structural refactor.

## Architecture Check

1. Successor building is a self-contained transition-simulation concern: given a search node and a candidate action, produce the next hypothetical state. Clear boundary between "which actions to try" (candidates) and "what happens when we try one" (transition).
2. No backward-compatibility shims.

## Verification Layers

1. All tests pass → `cargo test -p worldwake-ai`.
2. Single-layer ticket: code motion only.

## What to Change

### 1. Create `search/transition.rs`

Move the listed functions. Import `SearchNode`, `SearchCandidate`, and other shared types from `super`.

### 2. Update `search/mod.rs`

- Add `mod transition;`
- Add `use transition::*;`.
- Remove moved items.

## Files to Touch

- `crates/worldwake-ai/src/search/transition.rs` (new)
- `crates/worldwake-ai/src/search/mod.rs` (modify)

## Out of Scope

- Extracting other search sub-modules
- Changing function signatures, visibility, or logic
- Modifying `agent_tick/`
- Any changes outside `worldwake-ai`
- Reorganizing tests (S20STRCLE-013)

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-ai` — all tests pass unchanged
2. `cargo clippy -p worldwake-ai` — no new warnings

### Invariants

1. Zero behavioral change
2. `lib.rs` re-exports continue to resolve
3. Function signatures and visibility unchanged

## Test Plan

### New/Modified Tests

1. None — code motion only; verification is command-based.

### Commands

1. `cargo test -p worldwake-ai`
2. `cargo clippy -p worldwake-ai`
