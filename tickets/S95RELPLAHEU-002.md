# S95RELPLAHEU-002: Add FF heuristic fields to SearchExpansionSummary

**Status**: PENDING
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — SearchExpansionSummary field additions in worldwake-ai, observer formatting in worldwake-cli
**Deps**: S95 spec

## Problem

The decision trace infrastructure lacks fields to record FF heuristic values and helpful-action counts during search expansions. Without these fields, the RPG heuristic is not observable in decision traces, violating P29 (Debuggability).

## Assumption Reassessment (2026-04-12)

1. `SearchExpansionSummary` exists at `crates/worldwake-ai/src/decision_trace.rs:814` with 17 fields. `ff_heuristic` and `helpful_action_count` do not yet exist. The struct derives `Clone, Debug, Serialize, Deserialize`.
2. 11 explicit struct literal construction sites across 4 files: `search/mod.rs` (2), `agent_tick/planning.rs` (2), `decision_trace.rs` (3), `observer.rs` (4). Each must be updated with default values (`ff_heuristic: None`, `helpful_action_count: 0`).
3. Existing parallel fields `landmark_heuristic: u32` and `preferred_candidates: u16` at lines 838-840 establish the pattern for the new fields.

## Architecture Check

1. Adding `Option<u32>` and `u16` fields to an existing trace struct follows the established pattern. `Option` for `ff_heuristic` correctly represents "FF not enabled or dead end detected." Zero for `helpful_action_count` is the natural default when FF is not active.
2. No backward-compatibility shims. All construction sites are updated in this ticket.

## Verification Layers

1. New fields exist with correct types → compilation success
2. Default values (None, 0) at all construction sites → existing tests pass unchanged
3. Observer formats ff_heuristic when present → observer test
4. Single-layer ticket — trace infrastructure only, no cross-system mapping needed.

## What to Change

### 1. Add fields to SearchExpansionSummary

In `crates/worldwake-ai/src/decision_trace.rs`, add after the `landmark_heuristic` field:

```rust
/// The FF relaxed-plan heuristic value at this expansion, or `None` if
/// FF is disabled, no operators were available, or the RPG detected a
/// dead end.
pub ff_heuristic: Option<u32>,
/// Number of helpful actions identified from the relaxed plan.
pub helpful_action_count: u16,
```

### 2. Update all construction sites

Add `ff_heuristic: None, helpful_action_count: 0` to each of the 11 construction sites:

- `crates/worldwake-ai/src/search/mod.rs` — 2 sites (lines ~554, ~678)
- `crates/worldwake-ai/src/agent_tick/planning.rs` — 2 sites (lines ~2311, ~2349)
- `crates/worldwake-ai/src/decision_trace.rs` — 2 test construction sites (lines ~3953, ~4113)
- `crates/worldwake-cli/src/bin/observer.rs` — 3 sites in `sample_summary` functions (lines ~2234, ~2261) and test helpers

### 3. Observer formatting

In `crates/worldwake-cli/src/bin/observer.rs`, add formatting for `ff_heuristic` alongside existing `landmark_heuristic` display. When `ff_heuristic` is `Some(h)`, display `h_ff={h}` and `helpful_actions={count}`.

## Files to Touch

- `crates/worldwake-ai/src/decision_trace.rs` (modify)
- `crates/worldwake-ai/src/search/mod.rs` (modify)
- `crates/worldwake-ai/src/agent_tick/planning.rs` (modify)
- `crates/worldwake-cli/src/bin/observer.rs` (modify)

## Out of Scope

- Populating `ff_heuristic` with actual RPG values (ticket 004)
- RPG algorithm (ticket 003)
- CognitiveProfile field (ticket 001)

## Acceptance Criteria

### Tests That Must Pass

1. All existing decision trace tests pass with new default field values
2. Observer test constructions compile with new fields
3. Existing suite: `cargo test --workspace`

### Invariants

1. `SearchExpansionSummary` remains `Clone + Debug + Serialize + Deserialize`
2. All construction sites initialize the new fields to inert defaults (None, 0)
3. No behavioral change — fields are populated with actual values in ticket 004

## Test Plan

### New/Modified Tests

1. None — fields are inert defaults until ticket 004 populates them. Compilation + existing tests verify structural correctness.

### Commands

1. `cargo test -p worldwake-ai`
2. `cargo test -p worldwake-cli`
3. `cargo test --workspace`
4. `cargo clippy --workspace --all-targets -- -D warnings`
