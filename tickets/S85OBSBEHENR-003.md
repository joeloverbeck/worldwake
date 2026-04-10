# S85OBSBEHENR-003: Need snapshots at behavioral transitions

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: None
**Deps**: S85 (Observer Behavioral Enrichment)

## Problem

The observer samples needs every tick and reports min/max/average in the per-agent summary. It also computes per-agent action type counts in 100-tick bins. But it does not correlate the two: when an agent's behavior narrows (fewer distinct action types), there is no snapshot of the agent's needs at that transition point. This makes it difficult to diagnose whether need pressure caused the behavioral narrowing.

## Assumption Reassessment (2026-04-10)

1. `NeedsSample` struct at `observer.rs:47-52` with fields `hunger`, `thirst`, `fatigue`, `bladder`, `dirtiness` (all `u16`). `AgentStats` at `observer.rs:55-77` holds `needs_samples: Vec<NeedsSample>`. Per-agent action timeline computed in 100-tick bins at `observer.rs:738-763` using `BTreeMap<u64, BTreeMap<&str, u32>>`. Per-agent summary emits needs trajectory and anomaly flags — behavioral transitions would go between these.
2. S85 spec (Deliverable 3) describes this change. The action timeline bins and needs samples are both existing infrastructure.
3. Single-layer ticket: observer-only analysis and formatting. No shared abstraction boundary.

## Architecture Check

1. Reuses existing `needs_samples` and action timeline bin data already computed by the observer. The transition detection is a simple comparison of consecutive bin action-type counts. No new data collection from the simulation — only post-hoc analysis of already-collected observer data.
2. No backwards-compatibility aliasing or shims introduced.

## Verification Layers

1. Behavioral transition detected when action types drop by 50%+ → focused unit test with synthetic bin data
2. Need snapshot at transition tick is correct → focused unit test correlating needs_samples with bin boundaries
3. No false transitions when action types are stable → negative test
4. Single-layer observer-only ticket; no action/planning/event-log layer mapping applicable.

## What to Change

### 1. Compute behavioral transitions in per-agent decision trace summary

In the Decision Trace Summary section (Section 3), after the per-agent action timeline bins are computed (around `observer.rs:760`), add transition detection logic:

For each agent, iterate consecutive 100-tick bins. Count the number of distinct action types in each bin. When the count drops by 50% or more from one bin to the next, record a `BehavioralTransition`:

```rust
struct BehavioralTransition {
    tick: u64,           // Start tick of the narrowing bin (bin_index * 100)
    types_before: usize, // Distinct action types in previous bin
    types_after: usize,  // Distinct action types in current bin
    needs: NeedsSample,  // Needs snapshot at the transition tick
}
```

To get the needs snapshot at the transition tick: find the `needs_samples` entry closest to `tick` (the observer samples needs every tick, so index `tick` into the samples vec, clamping to bounds).

### 2. Emit behavioral transitions in per-agent summary

In the per-agent summary section, after the needs trajectory and before the anomaly flags, emit each detected transition:

```
**Behavioral transition** at tick 500: action repertoire narrowed (5 types → 2 types)
  Needs: hunger=750, thirst=800, fatigue=200, bladder=100, dirtiness=500
```

### 3. Add unit tests

- Test with synthetic data where action types drop from 5 to 2 — verify transition is detected and needs snapshot is emitted
- Test with stable action types — verify no transition is emitted
- Test with gradual decline (e.g., 5→4→3) — verify transition is only emitted when the 50% threshold is crossed

## Files to Touch

- `crates/worldwake-cli/src/bin/observer.rs` (modify)

## Out of Scope

- Modifying simulation behavior or AI decision-making
- Changing how `NeedsSample` is collected
- Detecting behavioral transitions based on criteria other than action type count drops
- Interactive observer features or live dashboards

## Acceptance Criteria

### Tests That Must Pass

1. New test: behavioral transition detected when action types drop by 50%+, output contains `**Behavioral transition**` with correct tick, type counts, and needs values
2. New test: no transition emitted when action types are stable
3. New test: transition only fires at the 50% threshold, not for gradual declines below threshold
4. Existing suite: `cargo test -p worldwake-cli`

### Invariants

1. Observer remains read-only — no mutation of world state
2. Existing needs trajectory and anomaly flag sections are unchanged
3. Transitions are computed from already-collected observer data, not new simulation queries

## Test Plan

### New/Modified Tests

1. `crates/worldwake-cli/src/bin/observer.rs` (inline tests) — verifies behavioral transition detection and need snapshot formatting

### Commands

1. `cargo test -p worldwake-cli`
2. `cargo clippy --workspace --all-targets -- -D warnings`
