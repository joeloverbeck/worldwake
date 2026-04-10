# S85OBSBEHENR-004: Affordance snapshots after travel

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: None
**Deps**: S85 (Observer Behavioral Enrichment)

## Problem

The observer captures affordances only from the first planning decision (effectively tick 0). Agents that travel to new locations have different affordances there, but these are never shown. This makes it impossible to diagnose whether an agent had access to the right actions at their destination.

## Assumption Reassessment (2026-04-10)

1. `AffordanceTrace` at `decision_trace.rs:218-221` with `available: Vec<AffordanceSummary>` and `place: Option<EntityId>`. `AffordanceSummary` at `decision_trace.rs:208-212` with `def_id`, `action_name`, `target_count`. Current observer captures initial affordances at `observer.rs:1388-1406` from the first planning decision with affordances. The live observer summary already has access to both `DecisionTraceSink::traces_for(agent)` and `ActionTraceSink::events_for(agent)`, so post-travel snapshots can be derived during report rendering without extending `AgentStats`.
2. S85 spec (Deliverable 4) describes this change. The observer already uses `AffordanceTrace` for initial display.
3. Single-layer ticket: observer-only data collection and formatting. No shared abstraction boundary.

## Architecture Check

1. Derives post-travel affordance snapshots directly at report-render time by correlating committed `travel` events from `ActionTraceSink::events_for(agent)` with later planning traces that carry `AffordanceTrace`. The observer remains read-only and avoids duplicating trace-derived state into `AgentStats`.
2. No backwards-compatibility aliasing or shims introduced.

## Verification Layers

1. Post-travel affordances captured and displayed → focused unit test with mock traces containing travel commit then planning with affordances
2. Final affordances displayed → focused unit test verifying last planning decision's affordances are emitted
3. No post-travel snapshot when no travel occurs → negative test
4. Single-layer observer-only ticket; no action/planning/event-log layer mapping applicable.

## What to Change

### 1. Derive post-travel affordance snapshots during report rendering

In the per-agent Decision Trace Summary section, correlate committed `travel` actions from `action_trace.events_for(agent)` with later planning traces from `sink.traces_for(agent)`. When the first later planning decision with affordances occurs at a different place than the last recorded affordance place, emit that snapshot as a post-travel affordance view.

Also track the last planning decision with affordances to capture final affordances.

### 2. Emit post-travel affordance snapshots in per-agent summary

In the per-agent summary, after the initial affordances section, emit each post-travel snapshot:

```
**Affordances after travel** (tick 340, arrived at Thornwall Village):
  harvest(Water, Well), pick_up(Bread), sell(Ore), ...
```

Also emit end-of-simulation affordances from the last planning decision with affordances:

```
**Final affordances** (tick 1400):
  sleep, relieve, ...
```

Format each affordance as `action_name(target_count targets)` or just `action_name` when `target_count == 0`.

### 3. Add unit tests

- Test with a trace that includes a travel commit followed by planning with affordances at a new place — verify post-travel snapshot is emitted
- Test final affordances are emitted from the last planning decision
- Test that no post-travel snapshot is emitted when no travel occurs

## Files to Touch

- `crates/worldwake-cli/src/bin/observer.rs` (modify)

## Out of Scope

- Modifying simulation behavior or AI decision-making
- Changing how `AffordanceTrace` is collected during planning
- Affordance diff/comparison between locations (future work)
- Interactive observer features or live dashboards

## Acceptance Criteria

### Tests That Must Pass

1. New test: post-travel affordance snapshot emitted after travel commit with correct tick, place, and action list
2. New test: final affordances emitted from last planning decision with affordances
3. New test: no post-travel snapshot when no travel occurs
4. Existing suite: `cargo test -p worldwake-cli`

### Invariants

1. Observer remains read-only — no mutation of world or trace state
2. Initial affordance display is unchanged
3. Post-travel snapshots use existing `AffordanceTrace` data, not new simulation queries

## Test Plan

### New/Modified Tests

1. `crates/worldwake-cli/src/bin/observer.rs` (inline tests) — verifies post-travel and final affordance snapshot formatting

### Commands

1. `cargo test -p worldwake-cli`
2. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

Completed on 2026-04-10.

- Added observer-local affordance snapshot helpers that derive planning affordance snapshots, committed travel ticks, post-travel affordance snapshots, and final affordances directly from the existing decision and action traces.
- Section 2 now emits `**Affordances after travel**` entries when a committed `travel` is followed by a planning affordance snapshot at a new place, and it also emits `**Final affordances**` from the last planning tick with affordances.
- Added focused observer tests covering committed travel filtering, post-travel snapshot detection, final snapshot selection, no-travel negative behavior, and affordance formatting.

## Deviations

- During reassessment, the ticket was corrected away from extending `AgentStats`. The live observer already had the required data at report time through `DecisionTraceSink::traces_for(agent)` and `ActionTraceSink::events_for(agent)`, so the implementation kept the new logic as render-time derivation instead of duplicating trace-derived state.

## Verification Result

- Passed `cargo test -p worldwake-cli`
- Passed `cargo clippy --workspace --all-targets -- -D warnings`
