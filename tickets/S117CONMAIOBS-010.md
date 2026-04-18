# S117CONMAIOBS-010: `MaintenanceStarvation` merged-window correctness and deficit threshold

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — observer anomaly classification in `worldwake-cli`
**Deps**: `archive/tickets/S117CONMAIOBS-001.md`, `archive/tickets/S117CONMAIOBS-003.md`, `archive/tickets/S117CONMAIOBS-006.md`, `S117CONMAIOBS-007`, `specs/S117-convergence-maintenance-observer-smells.md`

## Problem

The live `MAINTENANCE_STARVATION` detector currently emits false positives on `scenarios/survival-baseline.ron`, and the emitted descriptions can contradict the detector's own predicate. The root issue is architectural, not cosmetic: the detector finds qualifying 200-tick windows, merges them into a long span, then recomputes accumulation/relief/average over the merged span even when that merged span no longer satisfies the original condition. This makes the anomaly output untrustworthy and blocks honest archival of the S117 golden ticket.

## Assumption Reassessment (2026-04-18)

1. Live detector code in [`crates/worldwake-cli/src/bin/observer.rs`](../crates/worldwake-cli/src/bin/observer.rs) qualifies a window when `relief < accumulation && avg > medium_threshold`, stores only merged `(start, end)` spans, and later renders stats over the merged span (`detect_maintenance_starvation`, lines 1210-1268).
2. The baseline report proves the merged-span contradiction directly. Example outputs include:
   - Agent A bladder: `Average bladder in window: 483 permille (above medium threshold 550)`
   - Agent C thirst: `Average thirst in window: 310 permille (above medium threshold 360)`
   Those rendered averages cannot satisfy the detector's own `avg > medium_threshold` predicate.
3. Shared abstraction boundary under audit: the observer's rolling 200-tick maintenance detector over `AgentStats.needs_samples` and per-agent `DriveThresholds`. No simulation-side need accumulation logic is changed by this ticket.
4. The motivating invariant is stronger than "baseline should be quiet": every emitted starvation anomaly must correspond to a span that actually satisfies the detector predicate under the rendered numbers. False-positive suppression on baseline is a consequence of that correctness requirement, not the sole goal.
5. The current predicate is also too sensitive for healthy drift because any tiny deficit (`relief < accumulation` by 1) plus an average above medium qualifies. Baseline whole-run tables already show small positive net balances in healthy-ish runs, so a detector with no meaningful deficit floor is likely to overfire even after the merge bug is fixed.
6. Existing positive coverage from archived `S117CONMAIOBS-003` proves the original rising-dirtiness forcing branch, balanced non-detection, below-medium non-detection, and overlap-merge behavior. Reassessment must update those tests to prove whichever corrected span semantics land, instead of preserving buggy merge behavior as the contract.
7. Existing read-model Section 2 `Maintenance rates` tables in `S117CONMAIOBS-006` are complementary but not authoritative for the anomaly predicate. This ticket must keep Section 2 whole-run summaries intact unless the reassessed detector contract requires a factual wording adjustment only.
8. Adjacent concerns remain separate:
   - `GEOGRAPHIC_CONVERGENCE` lawful baseline routing belongs to `S117CONMAIOBS-009`.
   - `ACUTE_NEED_SPIKE` may be a real baseline/planner issue and belongs to `S117CONMAIOBS-011`.
9. Per `docs/FOUNDATIONS.md`, the fix must stay a derived observer computation over concrete need trajectories and authored thresholds. Hidden clamps, scenario-name suppressions, or arbitrary label-only filtering would be workaround behavior and are out of bounds.

## Architecture Check

1. The clean fix is to make the anomaly represent real qualifying windows again: either keep window-level anomalies, or merge only when the merged span still satisfies the same predicate, or render the strongest qualifying window instead of a synthetic merged suffix. Any of these is cleaner than keeping internally contradictory output.
2. Adding a meaningful deficit threshold or ratio is cleaner than globally weakening the detector after the fact. It keeps the detector tied to a mechanically interpretable cadence mismatch instead of "any slight positive drift above medium."

## Verification Layers

1. Every emitted starvation anomaly's rendered numbers satisfy the detector predicate -> focused observer unit/runtime coverage around detector/helper output
2. Healthy baseline does not emit starvation anomalies from tiny drift or merged-span artifacts -> observer golden E2E against `scenarios/survival-baseline.ron`
3. Original forcing starvation scenario still emits on the intended need -> existing `golden_observer_anomalies.rs` maintenance fixture plus focused observer tests
4. Section 2 whole-run maintenance tables remain intact as summaries and are not used as a false proof of the anomaly predicate -> focused render test if wording changes
5. Single-layer ticket: no planner/action-trace proof surface is required because the change is observer-side anomaly computation only

## What to Change

### 1. Correct the span semantics in `detect_maintenance_starvation()`

Rework the detector so that every emitted anomaly corresponds to a span that actually satisfies the live predicate. Valid end states include:

- reporting maximal qualifying windows individually
- merging only spans whose combined window still qualifies
- reporting a canonical strongest qualifying window per `(agent, need)`

The landed contract must avoid internally contradictory descriptions.

### 2. Add a material starvation threshold

Refine the detector so ordinary healthy drift does not count as starvation. The exact shape must be reassessed during implementation, but it must be mechanical and explainable, for example:

- minimum net deficit permille over the qualifying window
- minimum relief-to-accumulation shortfall ratio

### 3. Update focused and golden proof

Adjust the focused detector tests from `S117CONMAIOBS-003` and the baseline regression surface in `golden_observer_anomalies.rs` to match the corrected detector contract.

## Files to Touch

- `crates/worldwake-cli/src/bin/observer.rs` (modify)
- `crates/worldwake-cli/tests/golden_observer_anomalies.rs` (modify)
- `tickets/S117CONMAIOBS-007.md` (modify if the owning baseline-regression handoff needs a factual note)
- `specs/S117-convergence-maintenance-observer-smells.md` (modify if the landed detector contract no longer matches the draft wording)

## Out of Scope

- Re-authoring the baseline scenario
- Planner or simulation-side need cadence changes
- `GeographicConvergence` and `AcuteNeedSpike` behavior
- Removing Section 2 maintenance tables

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-cli --test golden_observer_anomalies`
2. `cargo test -p worldwake-cli --bin observer`
3. Existing suite: `cargo test -p worldwake-cli`

### Invariants

1. Every emitted `MAINTENANCE_STARVATION` anomaly corresponds to a span whose rendered numbers satisfy the live detector predicate.
2. The detector remains a derived observer computation over `needs_samples` plus authored `DriveThresholds`; no scenario-specific suppression path is introduced.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-cli/src/bin/observer.rs` — focused detector tests updated to prove corrected span semantics and the new material-deficit gate.
2. `crates/worldwake-cli/tests/golden_observer_anomalies.rs` — baseline regression assertion updated so `MAINTENANCE_STARVATION` stays absent on `survival-baseline.ron` once the detector is corrected.

### Commands

1. `cargo test -p worldwake-cli --test golden_observer_anomalies`
2. `cargo test -p worldwake-cli --bin observer`
3. `cargo test -p worldwake-cli`
4. `cargo clippy --workspace --all-targets -- -D warnings`

