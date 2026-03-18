# TRACEVIEW-001: Add a Cross-Layer Timeline View for Emergent Scenario Debugging

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes — debug/reporting surface across existing trace sinks
**Deps**: `POLTRAC-001-political-system-trace-sink.md`, `archive/specs/2026-03-17-action-execution-trace.md`, existing decision trace system in `worldwake-ai`, existing action trace system in `worldwake-sim`

## Problem

Mixed emergent scenarios currently require jumping between multiple debug surfaces:

- decision trace for AI reasoning
- action trace for lifecycle execution
- raw event-log deltas for authoritative mutation
- future political trace for system-level office decisions

That is workable but slow. For cross-system debugging, the missing artifact is one compact timeline view that aligns these layers by tick.

## Assumption Reassessment (2026-03-18)

1. The repo already has decision traces in `worldwake-ai` and action traces in `worldwake-sim`, but no unified timeline that aligns them.
2. Cross-system tests such as `crates/worldwake-ai/tests/golden_emergent.rs` can currently prove behavior, but debugging them requires manual correlation across separate sinks.
3. A unified timeline is most valuable after a politics/system trace exists, because otherwise authoritative non-action decisions still remain opaque.
4. This is a debug/reporting surface problem, not an authoritative-model problem.

## Architecture Check

1. The cleanest approach is a read-only timeline builder over existing trace sinks and event-log data. It should not become a source of truth or a new cache that authoritative logic depends on.
2. This belongs in debugging/test support, not in the live simulation path.
3. The timeline should preserve layer boundaries while making temporal correlation easier, for example:
   - Tick 7: decision trace selected `EngageHostile`
   - Tick 8: action trace committed `attack`
   - Tick 8: authoritative mutation set `DeadAt`
   - Tick 8: political trace marked office vacant
   - Tick 13: political trace installed new holder

## What to Change

### 1. Define timeline model

Create a compact read-only timeline representation that can merge:

- decision trace entries
- action trace entries
- selected authoritative event-log mutations
- political trace entries

### 2. Add builder/helpers for tests

Expose helpers that let golden tests build and dump a cross-layer timeline for one agent, one office, or one scenario window.

### 3. Add at least one golden/debug usage

Use the timeline in one cross-system golden scenario or debug-oriented test so the output shape is proven useful.

## Files to Touch

- tracing/debug support files under the relevant crates (new or modify)
- `crates/worldwake-ai/tests/golden_harness/mod.rs` (modify if helper exposure is needed)
- `crates/worldwake-ai/tests/golden_emergent.rs` or another cross-system golden file (modify)
- `docs/golden-e2e-testing.md` (modify)

## Out of Scope

- New authoritative behavior
- Replacing existing decision/action/system trace sinks
- UI work outside textual/debug helper output

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-ai --test golden_emergent`
2. `cargo test -p worldwake-ai`

### Invariants

1. The cross-layer timeline must remain a derived debug view, never the source of truth.
2. Timeline output must preserve actual tick ordering and avoid inventing causal links not present in the underlying traces/event log.

## Test Plan

### New/Modified Tests

1. Add focused tests for timeline merge/order behavior using synthetic decision/action/system entries.
2. Add one real golden/debug assertion or dump-path usage for a cross-system scenario.

### Commands

1. `cargo test -p worldwake-ai --test golden_emergent`
2. `cargo test -p worldwake-ai`

