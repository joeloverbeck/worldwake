# S20STRCLE-006: Extract agent_tick/journey.rs

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: None
**Deps**: S20STRCLE-001

## Problem

The journey lifecycle logic in `agent_tick/mod.rs` (~400 lines) manages multi-step travel plans, recoverable blockages, and journey field updates. Extracting it into a dedicated sub-module isolates travel-plan management from the core decision loop.

## Assumption Reassessment (2026-03-22)

1. Functions to extract (verified via grep):
   - `update_journey_fields_for_adopted_plan()` (line 1760)
   - `handle_recoverable_travel_step_blockage()` (line 1790)
   - `blocked_leg_target()` (line 1841)
   - `JourneySwitchMarginSource` enum (line 43)
   - `JourneyDebugSnapshot` struct (line 49)
2. `JourneySwitchMarginSource` and `JourneyDebugSnapshot` are re-exported from `lib.rs` (line 29). The re-export path `agent_tick::JourneySwitchMarginSource` must continue to resolve — achieved by re-exporting from `agent_tick/mod.rs`.
3–12. N/A — pure structural refactor.

## Architecture Check

1. Journey management is a self-contained concern: it tracks multi-leg travel plans, handles edge blockages, and provides debug snapshots. Isolating it from the main decision loop reduces cognitive load.
2. No backward-compatibility shims — `mod.rs` re-exports the public types.

## Verification Layers

1. All tests pass → `cargo test -p worldwake-ai`.
2. Single-layer ticket: code motion only.

## What to Change

### 1. Create `agent_tick/journey.rs`

Move the listed functions, enum, and struct. Add necessary `use` imports.

### 2. Update `agent_tick/mod.rs`

- Add `mod journey;`
- Add `pub use journey::{JourneySwitchMarginSource, JourneyDebugSnapshot};` to preserve `lib.rs` re-export chain.
- Add `use journey::*;` for internal callers.
- Remove moved items from `mod.rs`.

## Files to Touch

- `crates/worldwake-ai/src/agent_tick/journey.rs` (new)
- `crates/worldwake-ai/src/agent_tick/mod.rs` (modify)

## Out of Scope

- Extracting other pipeline stages
- Changing function signatures, visibility, or logic
- Modifying `search.rs`
- Any changes outside `worldwake-ai`
- Reorganizing tests (S20STRCLE-013)

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-ai` — all tests pass unchanged
2. `cargo clippy -p worldwake-ai` — no new warnings

### Invariants

1. Zero behavioral change
2. `lib.rs` line 29 re-exports `JourneySwitchMarginSource`, `JourneyDebugSnapshot` — must continue resolving
3. Function signatures and visibility unchanged

## Test Plan

### New/Modified Tests

1. None — code motion only; verification is command-based.

### Commands

1. `cargo test -p worldwake-ai`
2. `cargo clippy -p worldwake-ai`
