# S85OBSBEHENR-004: Affordance snapshots after travel

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: None
**Deps**: S85 (Observer Behavioral Enrichment)

## Problem

The observer captures affordances only from the first planning decision (effectively tick 0). Agents that travel to new locations have different affordances there, but these are never shown. This makes it impossible to diagnose whether an agent had access to the right actions at their destination.

## Assumption Reassessment (2026-04-10)

1. `AffordanceTrace` at `decision_trace.rs:218-221` with `available: Vec<AffordanceSummary>` and `place: Option<EntityId>`. `AffordanceSummary` at `decision_trace.rs:208-212` with `def_id`, `action_name`, `target_count`. Current observer captures initial affordances at `observer.rs:1260-1278` from the first planning decision with affordances. `AgentStats` at `observer.rs:55-77` does not currently store post-travel affordances.
2. S85 spec (Deliverable 4) describes this change. The observer already uses `AffordanceTrace` for initial display.
3. Single-layer ticket: observer-only data collection and formatting. No shared abstraction boundary.

## Architecture Check

1. Extends `AgentStats` with a `Vec<(Tick, AffordanceTrace)>` to accumulate post-travel affordance snapshots. Detection is based on observing committed `travel` actions in the action trace followed by a planning decision with affordances at a different place. This reuses existing trace data — no new simulation queries.
2. No backwards-compatibility aliasing or shims introduced.

## Verification Layers

1. Post-travel affordances captured and displayed → focused unit test with mock traces containing travel commit then planning with affordances
2. Final affordances displayed → focused unit test verifying last planning decision's affordances are emitted
3. No post-travel snapshot when no travel occurs → negative test
4. Single-layer observer-only ticket; no action/planning/event-log layer mapping applicable.

## What to Change

### 1. Add post-travel affordance tracking to AgentStats

Add a field to `AgentStats`:

```rust
post_travel_affordances: Vec<(Tick, AffordanceTrace)>,
```

Initialize as empty in `AgentStats::new`.

### 2. Collect post-travel affordances during trace analysis

In the Decision Trace Summary section where per-agent traces are iterated (around `observer.rs:1040+`), track committed `travel` actions from the action trace. When a planning decision occurs after a travel commit and has affordances at a different place than the last recorded affordance place, push `(tick, affordance_trace.clone())` to `post_travel_affordances`.

Also track the last planning decision with affordances to capture final affordances.

### 3. Emit post-travel affordance snapshots in per-agent summary

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

### 4. Add unit tests

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
