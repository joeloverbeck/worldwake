# HARPREE14-009: Cache OmniscientBeliefView per agent tick

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: Yes -- refactor of agent_tick.rs internal flow
**Deps**: None (Wave 2, independent)
**Spec Reference**: HARDENING-PRE-E14.md, HARDEN-C02

## Problem

`process_agent()` in `agent_tick.rs` creates up to 9 separate `OmniscientBeliefView::new()` calls during a single agent's tick processing. While `OmniscientBeliefView::new()` is cheap (just a reference), the pattern obscures the data flow -- it's unclear which view reflects which world state. This makes reasoning about correctness harder.

## Assumption Reassessment (2026-03-11)

1. 9 occurrences of `OmniscientBeliefView::new()` in `agent_tick.rs` -- confirmed
2. `OmniscientBeliefView::new()` is cheap (just takes references) -- confirmed from code
3. World mutations happen at specific points (e.g., `persist_blocked_memory`) -- needs verification during implementation

## Architecture Check

1. Creating ONE view at the top and refreshing only after mutations makes the data flow explicit: each code section operates on a view of known freshness.
2. Comments documenting view generations clarify correctness for future readers.
3. No backwards-compatibility shims. Pure clarity refactor.

## What to Change

### 1. Create single `OmniscientBeliefView` at top of `process_agent()`

Replace the first `OmniscientBeliefView::new()` call with a let binding used throughout.

### 2. Identify mutation points

Find all points in `process_agent()` where world state is mutated (e.g., `persist_blocked_memory`, action completion). After each mutation, create a fresh view.

### 3. Replace scattered `OmniscientBeliefView::new()` calls

Each section uses the current view binding. After a mutation, rebind with a fresh view and add a comment explaining why.

### 4. Add doc comments

Document the view lifecycle: "View created here, valid until next mutation at line X."

## Files to Touch

- `crates/worldwake-ai/src/agent_tick.rs` (modify)

## Out of Scope

- Changing `OmniscientBeliefView` API or internals
- Modifying `process_agent()` logic or behavior
- Changes to `decision_runtime.rs` or other files
- Performance optimization (this is primarily a clarity improvement)

## Acceptance Criteria

### Tests That Must Pass

1. All existing agent_tick tests pass unchanged
2. Golden e2e hashes identical
3. `cargo test --workspace` passes
4. `cargo clippy --workspace` -- no new warnings

### Invariants

1. `process_agent()` behavior unchanged
2. Same belief views used at same logical points (just created fewer times)
3. Golden e2e state hashes identical
4. View is always fresh after any world mutation

## Test Plan

### New/Modified Tests

1. No new tests needed -- this is a pure clarity refactor. Existing tests validate identical behavior.

### Commands

1. `cargo test -p worldwake-ai agent_tick` (targeted)
2. `cargo test -p worldwake-ai --test golden_e2e` (determinism check)
3. `cargo test --workspace`
4. `cargo clippy --workspace`
