# S20STRCLE-002: Extract agent_tick/observation.rs

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: None
**Deps**: S20STRCLE-001

## Problem

The observation/read-phase logic in `agent_tick/mod.rs` (~400 lines) handles snapshot comparison, runtime refresh, and in-flight reconciliation. Extracting it into a dedicated sub-module improves navigability for all subsequent AI architecture work (S21–S28).

## Assumption Reassessment (2026-03-22)

1. Functions to extract (verified via grep at current line numbers in `agent_tick.rs`):
   - `refresh_runtime_for_read_phase()` (line 772)
   - `observation_snapshot_changed()` (line 1960)
   - `update_runtime_observation_snapshot()` (line 1991)
   - `facility_access_signature()` (line 2004)
   - `facility_queue_patience_exhausted()` (line 2025)
   - `commodity_signature()` (line 2052)
   - `filtered_commodity_signature()` (line 2065)
   - `unique_item_signature()` (line 2079)
   - `InFlightReconciliation` struct (line 157)
   - `reconcile_in_flight_state()` (line 1539)
   - `matching_start_failure()` (line 1600)
   - `reconcile_committed_facility_queue_intents()` (line 1609)
   - `ReadPhaseResult` struct (line 752)
   - `ReadPhaseContext` struct (line 148)
   - `handle_facility_queue_transitions()` (line 855)
2. All these are `pub(crate)` or private — no public API surface changes.
3. N/A — not planner/golden driven.
4. N/A — not an AI regression.
5. N/A — no ordering dependency.
6. N/A — no heuristic changes.
7. N/A — not a start-failure ticket.
8. N/A — not political.
9. N/A — no ControlSource changes.
10. N/A — no golden scenario.
11. No mismatches found.
12. N/A — no arithmetic.

## Architecture Check

1. These functions form a cohesive "observation and snapshot" phase — they read world state into the runtime's observation cache. Grouping them clarifies the pipeline boundary between "what the agent sees" and "what the agent decides."
2. No backward-compatibility shims — functions move, `mod.rs` re-imports via `use observation::*` or explicit items.

## Verification Layers

1. All golden tests pass → `cargo test -p worldwake-ai` proves no import/logic breakage.
2. Single-layer ticket: code motion only — additional layer mapping is not applicable.

## What to Change

### 1. Create `agent_tick/observation.rs`

Move the listed functions and structs into this new file. Add necessary `use` imports at the top of the new file for types referenced by these functions.

### 2. Update `agent_tick/mod.rs`

- Add `mod observation;`
- Add `use observation::*;` (or explicit items) so all call sites within `mod.rs` continue to compile without path changes.
- Remove the moved function bodies from `mod.rs`.

## Files to Touch

- `crates/worldwake-ai/src/agent_tick/observation.rs` (new)
- `crates/worldwake-ai/src/agent_tick/mod.rs` (modify — remove moved code, add `mod` + `use`)

## Out of Scope

- Extracting other pipeline stages (candidates, active_action, planning, execution, journey)
- Changing any function signatures, visibility, or logic
- Modifying `search.rs`
- Any changes outside `worldwake-ai` crate
- Renaming or reorganizing test functions (that is S20STRCLE-013)

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-ai` — all 59 inline tests + all golden tests pass unchanged
2. `cargo clippy -p worldwake-ai` — no new warnings

### Invariants

1. Zero behavioral change — pure code motion
2. All `pub use` re-exports in `lib.rs` continue to resolve (they reference `agent_tick::AgentTickDriver` etc., which stays in `mod.rs`)
3. Function signatures and visibility unchanged

## Test Plan

### New/Modified Tests

1. None — code motion only; verification is command-based and existing runtime coverage is named in Assumption Reassessment.

### Commands

1. `cargo test -p worldwake-ai`
2. `cargo clippy -p worldwake-ai`
