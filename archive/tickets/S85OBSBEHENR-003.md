# S85OBSBEHENR-003: Need snapshots at behavioral transitions

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: None
**Deps**: S85 (Observer Behavioral Enrichment)

## Problem

The observer samples needs every tick and reports min/max/average in the per-agent summary. It also computes per-agent action type counts in 100-tick bins. But it does not correlate the two: when an agent's behavior narrows (fewer distinct action types), there is no snapshot of the agent's needs at that transition point. This makes it difficult to diagnose whether need pressure caused the behavioral narrowing.

## Assumption Reassessment (2026-04-10)

1. `NeedsSample` struct at `observer.rs:47-52` with fields `hunger`, `thirst`, `fatigue`, `bladder`, `dirtiness` (all `u16`). `AgentStats` at `observer.rs:55-77` holds `needs_samples: Vec<NeedsSample>`. The relevant action-type bins are currently computed in the global "Per-Agent Action Timeline" section from `action_trace` events, not in the per-agent decision summary. Section 2 — Per-Agent Summary already emits needs trajectory and then proceeds to later summary subsections without any existing behavioral-transition block.
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

### 1. Compute behavioral transitions from existing action timeline data

Factor the 100-tick action-type binning into a local observer helper so the same already-collected `action_trace` data can drive both the existing "Per-Agent Action Timeline" section and the new Section 2 per-agent summary transition detection.

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

In Section 2 — Per-Agent Summary, after the needs trajectory block, emit each detected transition:

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

## Outcome

Completed on 2026-04-10.

- Added observer-local helpers to reuse the existing per-agent 100-tick action-type bins for Section 2 summary analysis and to detect behavioral transitions from already-collected `action_trace` plus `needs_samples`.
- Section 2 — Per-Agent Summary now emits `**Behavioral transition**` lines after the needs trajectory block when an agent's distinct action types drop by 50% or more between consecutive bins, including a clamped needs snapshot at the transition tick.
- Added focused observer tests covering threshold-crossing detection, stable-action negative behavior, gradual-decline negative behavior, and transition formatting.

## Deviations

- During reassessment, the ticket was corrected to use the global per-agent action timeline bins as the live computation surface. The transition output still lands in Section 2 — Per-Agent Summary, but the owned binning logic is shared helper logic rather than a Decision Trace Summary-local calculation.

## Verification Result

- Passed `cargo test -p worldwake-cli behavioral_transition_detected_when_action_types_drop_by_half`
- Passed `cargo test -p worldwake-cli behavioral_transition_not_detected_when_action_types_are_stable`
- Passed `cargo test -p worldwake-cli behavioral_transition_only_fires_when_threshold_is_crossed`
- Passed `cargo test -p worldwake-cli`
- Passed `cargo clippy --workspace --all-targets -- -D warnings`
