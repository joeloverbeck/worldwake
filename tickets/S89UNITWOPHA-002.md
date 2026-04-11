# S89UNITWOPHA-002: Decision trace tactical goal recording

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: Yes — diagnostic metadata (SearchTraceMetadata)
**Deps**: S89UNITWOPHA-001

## Problem

After S89UNITWOPHA-001 and S89UNITWOPHA-004, agents receive `TravelToGoal` tactical scoping for strategic `SatisfyGoal` stages across non-whitelisted remote-goal families, and supported no-evidence exploration fallback now uses a dedicated `Explore` progress-barrier contract. But the decision trace does not record which tactical goal was active during search. When debugging why an agent scoped its search to a particular location, the trace shows the strategic plan but not the tactical goal derived from it. This violates FND-29 (Debuggability).

## Assumption Reassessment (2026-04-11)

1. `SearchTraceMetadata` at `crates/worldwake-ai/src/search/mod.rs:47` currently has 3 fields: `strategic_plan`, `landmarks_extracted`, `landmark_orderings`. No `tactical_goal` field exists. Derives `Clone, Debug, Default`.
2. `trace_state` is constructed at line 235 and returned as part of `PlanSearchResult`. The `tactical_goal` local variable is constructed at lines 239-242 (after 001 changes: unconditional construction). Recording it into `trace_state` after construction is a single assignment.
3. Single shared boundary: `SearchTraceMetadata` struct, consumed by decision trace formatting in the AI crate's trace module.

## Architecture Check

1. Adding a `tactical_goal: Option<String>` field to `SearchTraceMetadata` is the minimal diagnostic extension. Using `String` (Debug-formatted) rather than storing the `TacticalGoal` value avoids exposing the `pub(super)` type outside the search module.
2. No backwards-compatibility aliasing/shims introduced.

## Verification Layers

1. Tactical goal recorded in trace → focused test: construct a search that produces a `TravelToGoal` tactical goal, verify `SearchTraceMetadata.tactical_goal` is `Some(...)` containing the variant name
2. Local goals produce `None` tactical goal in trace → focused test: Sleep goal produces `tactical_goal: None` in metadata
3. Single-layer ticket (diagnostic metadata only) — no cross-system mapping applicable

## What to Change

### 1. Add `tactical_goal` field to `SearchTraceMetadata`

In `crates/worldwake-ai/src/search/mod.rs`, add to the struct at line 47:

```rust
pub(crate) tactical_goal: Option<String>,
```

The `Default` derive will initialize this to `None`.

### 2. Record tactical goal after construction

After the tactical goal construction (post-001 change), add:

```rust
trace_state.tactical_goal = tactical_goal.as_ref().map(|tg| format!("{tg:?}"));
```

This records the Debug representation of the active tactical goal variant.

## Files to Touch

- `crates/worldwake-ai/src/search/mod.rs` (modify)

## Out of Scope

- Changing decision trace formatting or output format
- Adding tactical goal information to event log or world state
- Modifying the `TacticalGoal` enum's Debug derive

## Acceptance Criteria

### Tests That Must Pass

1. Existing suite: `cargo test -p worldwake-ai`
2. `search_trace_metadata_records_two_phase_strategic_and_landmark_details` — existing test may need update if it asserts exact `SearchTraceMetadata` field set (check during implementation)

### Invariants

1. `SearchTraceMetadata::tactical_goal` is diagnostic only — never used for search decisions, only for trace output
2. `Default` impl continues to work (all fields have defaults)

## Test Plan

### New/Modified Tests

None — the trace metadata recording is validated as part of S89UNITWOPHA-003 tests. This ticket ensures compilation and existing tests pass.

### Commands

1. `cargo test -p worldwake-ai` — all existing tests pass
2. `cargo clippy --workspace --all-targets -- -D warnings` — no new warnings
