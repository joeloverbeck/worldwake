# S117CONMAIOBS-010: `MaintenanceStarvation` merged-window correctness and deficit threshold

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — observer anomaly classification in `worldwake-cli`
**Deps**: `archive/tickets/S117CONMAIOBS-001.md`, `archive/tickets/S117CONMAIOBS-003.md`, `archive/tickets/S117CONMAIOBS-006.md`, `S117CONMAIOBS-007`, `specs/S117-convergence-maintenance-observer-smells.md`

## Problem

The live `MAINTENANCE_STARVATION` detector currently emits descriptions that can contradict its own predicate. The root issue is architectural, not cosmetic: the detector finds qualifying 200-tick windows, merges them into a long span, then recomputes accumulation/relief/average over the merged span even when that merged span no longer satisfies the original condition. This makes the anomaly output untrustworthy and blocks honest archival of the S117 golden ticket. Reassessment also showed the original ticket scope was too broad: once the merged-span bug is fixed and the predicate is tightened, some baseline maintenance windows still look materially bad rather than obviously false.

## Assumption Reassessment (2026-04-18)

1. Live detector code in [`crates/worldwake-cli/src/bin/observer.rs`](../crates/worldwake-cli/src/bin/observer.rs) qualifies a window when `relief < accumulation && avg > medium_threshold`, stores only merged `(start, end)` spans, and later renders stats over the merged span (`detect_maintenance_starvation`, lines 1210-1268).
2. The baseline report proves the merged-span contradiction directly. Example outputs include:
   - Agent A bladder: `Average bladder in window: 483 permille (above medium threshold 550)`
   - Agent C thirst: `Average thirst in window: 310 permille (above medium threshold 360)`
   Those rendered averages cannot satisfy the detector's own `avg > medium_threshold` predicate.
3. Shared abstraction boundary under audit: the observer's rolling 200-tick maintenance detector over `AgentStats.needs_samples` and per-agent `DriveThresholds`. No simulation-side need accumulation logic is changed by this ticket.
4. The motivating invariant is stronger than "baseline should be quiet": every emitted starvation anomaly must correspond to a span that actually satisfies the detector predicate under the rendered numbers. Eliminating internally contradictory output is the owned correctness target for this ticket.
5. The current predicate is too sensitive for healthy drift because any tiny deficit (`relief < accumulation` by 1) plus an average above medium qualifies. Live reassessment also showed a second mismatch: after correcting the merged-span bug and tightening the gate to the agent's own `high` threshold plus majority-unrelieved accumulation, `survival-baseline.ron` still produces a small set of severe hunger/thirst windows that no longer look like obvious detector noise. Those remaining baseline windows are a separate disposition concern, not honest proof that the detector fix is wrong.
6. Existing positive coverage from archived `S117CONMAIOBS-003` proves the original rising-dirtiness forcing branch, balanced non-detection, below-medium non-detection, and overlap-merge behavior. Reassessment must update those tests to prove the landed corrected contract instead of preserving buggy merge behavior as the contract.
7. Existing read-model Section 2 `Maintenance rates` tables in `S117CONMAIOBS-006` are complementary but not authoritative for the anomaly predicate. This ticket must keep Section 2 whole-run summaries intact unless the reassessed detector contract requires a factual wording adjustment only.
8. Adjacent concerns remain separate:
   - `GEOGRAPHIC_CONVERGENCE` lawful baseline routing belongs to `S117CONMAIOBS-009`.
   - `ACUTE_NEED_SPIKE` may be a real baseline/planner issue and belongs to `S117CONMAIOBS-011`.
   - Remaining high-band baseline `MAINTENANCE_STARVATION` windows after the detector fix belong to `S117CONMAIOBS-012`.
9. Per `docs/FOUNDATIONS.md`, the fix must stay a derived observer computation over concrete need trajectories and authored thresholds. Hidden clamps, scenario-name suppressions, or arbitrary label-only filtering would be workaround behavior and are out of bounds.

## Architecture Check

1. The clean fix is to make the anomaly represent a real qualifying window again instead of a synthetic merged suffix. Reporting one canonical strongest qualifying 200-tick window per `(agent, need)` is cleaner than preserving contradictory merged spans.
2. Tightening the detector to the agent's own `high` threshold plus a majority-unrelieved accumulation gate is cleaner than globally weakening the detector after the fact. It keeps the detector tied to a mechanically interpretable cadence mismatch instead of "any slight positive drift above medium."

## Verification Layers

1. Every emitted starvation anomaly's rendered numbers satisfy the detector predicate -> focused observer unit/runtime coverage around detector/helper output
2. Original forcing starvation scenario still emits on the intended need under the tightened predicate -> existing `golden_observer_anomalies.rs` maintenance fixture plus focused observer tests
3. Baseline merged-span contradictions are removed even if some severe maintenance windows remain -> direct observer baseline run plus focused detector checks
4. Section 2 whole-run maintenance tables remain intact as summaries and are not used as a false proof of the anomaly predicate -> focused render test if wording changes
5. Single-layer ticket: no planner/action-trace proof surface is required because the change is observer-side anomaly computation only

## What to Change

### 1. Correct the span semantics in `detect_maintenance_starvation()`

Rework the detector so that every emitted anomaly corresponds to one real qualifying 200-tick window rather than a merged suffix. The landed contract for this ticket is one canonical strongest qualifying window per `(agent, need)`, chosen by largest net deficit and then strongest average / earliest start for deterministic tie-breaking.

### 2. Add a material starvation threshold

Refine the detector so ordinary healthy drift does not count as starvation. The landed contract for this ticket must remain mechanical and explainable. Reassessment currently points to:

- average need above the agent's own `DriveThresholds.<need>.high()`
- relief strictly less than half of accumulation across the 200-tick window

### 3. Update focused and golden proof

Adjust the focused detector tests from `S117CONMAIOBS-003` and the maintenance golden text assertions in `golden_observer_anomalies.rs` to match the corrected detector contract. Do not force a baseline-silence assertion here if the remaining baseline windows still satisfy the tightened predicate honestly.

## Files to Touch

- `crates/worldwake-cli/src/bin/observer.rs` (modify)
- `crates/worldwake-cli/tests/golden_observer_anomalies.rs` (modify)
- `tickets/S117CONMAIOBS-007.md` (modify if the owning baseline-regression handoff needs a factual note)
- `tickets/S117CONMAIOBS-012.md` (new if the remaining baseline maintenance windows still need architectural disposition after the detector fix)
- `specs/S117-convergence-maintenance-observer-smells.md` (modify if the landed detector contract no longer matches the draft wording)

## Out of Scope

- Re-authoring the baseline scenario
- Planner or simulation-side need cadence changes
- `GeographicConvergence` and `AcuteNeedSpike` behavior
- Removing Section 2 maintenance tables
- Explaining or fixing any remaining severe baseline maintenance windows after the detector contract is corrected

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-cli --test golden_observer_anomalies`
2. `cargo test -p worldwake-cli --bin observer`
3. Existing suite: `cargo test -p worldwake-cli`

### Invariants

1. Every emitted `MAINTENANCE_STARVATION` anomaly corresponds to one 200-tick span whose rendered numbers satisfy the live detector predicate.
2. The detector remains a derived observer computation over `needs_samples` plus authored `DriveThresholds`; no scenario-specific suppression path is introduced.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-cli/src/bin/observer.rs` — focused detector tests updated to prove strongest-window selection and the tightened high-threshold / majority-unrelieved gate.
2. `crates/worldwake-cli/tests/golden_observer_anomalies.rs` — maintenance golden assertions updated to match the corrected wording/contract.

### Commands

1. `cargo test -p worldwake-cli --test golden_observer_anomalies`
2. `cargo test -p worldwake-cli --bin observer`
3. `cargo test -p worldwake-cli`
4. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

Completed on 2026-04-18.

- Reworked `detect_maintenance_starvation()` in `crates/worldwake-cli/src/bin/observer.rs` to emit one canonical strongest qualifying 200-tick window per `(agent, need)` instead of merging all qualifying windows into a synthetic suffix span.
- Tightened the detector predicate to the agent's own per-need `high` threshold plus a majority-unrelieved accumulation gate (`relief * 2 < accumulation`), and updated the rendered description to report the real qualifying window with an explicit net deficit.
- Updated the focused observer tests to prove the landed contract: strong-window positive detection, no fire when relief keeps up, no fire below the high band, and strongest-window selection.
- Updated the maintenance golden assertions in `crates/worldwake-cli/tests/golden_observer_anomalies.rs` to match the landed wording (`Net deficit`, `above high threshold`).
- Reassessment showed the original ticket scope was too broad: after the detector fix, `survival-baseline.ron` still emits three severe maintenance windows that are no longer merged-span contradictions. I created `tickets/S117CONMAIOBS-012.md` to own that remaining baseline-maintenance disposition explicitly instead of continuing to tune `010` toward silence.

## Deviations

- The drafted ticket assumed this slice would also make the healthy baseline quiet. Live reassessment after the detector fix showed that assumption was false: the remaining baseline maintenance windows still satisfy the tightened predicate honestly, so `010` was narrowed to detector correctness and handoff rather than detector correctness plus baseline silence.
- The landed detector contract is narrower and more specific than the draft's open-ended menu of options. Instead of preserving multiple possible end states (`maximal windows`, `merge-only-if-still-valid`, or `strongest window`), the implementation now commits to one deterministic strongest-window path.
- The active S117 roadmap changed during implementation: `tickets/S117CONMAIOBS-007.md` and `specs/S117-convergence-maintenance-observer-smells.md` were updated factually, and `tickets/S117CONMAIOBS-012.md` was created to own the remaining baseline-maintenance contradiction.

## Verification Result

- Passed `cargo test -p worldwake-cli --test golden_observer_anomalies`
- Passed `cargo test -p worldwake-cli --bin observer maintenance_starvation`
- Passed `cargo test -p worldwake-cli`
- Passed `cargo clippy --workspace --all-targets -- -D warnings`
