# S117CONMAIOBS-009: `GeographicConvergence` lawful single-source dampener

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes — observer anomaly classification in `worldwake-cli`
**Deps**: `archive/tickets/S117CONMAIOBS-001.md`, `archive/tickets/S117CONMAIOBS-002.md`, `S117CONMAIOBS-007`, `specs/S117-convergence-maintenance-observer-smells.md`

## Problem

The new `GEOGRAPHIC_CONVERGENCE` detector currently fires on `scenarios/survival-baseline.ron`, even though the baseline topology lawfully concentrates all three agents at `Fertile Fields` because that place is the only authored food source. This is a detector false positive, not a healthy-scenario pathology: the observer is classifying single-source survival routing as convergence failure. If left unchanged, Section 3 will train downstream analysis to treat lawful concentration as suspicious and `S117CONMAIOBS-007` cannot close its baseline-regression guard honestly.

## Assumption Reassessment (2026-04-18)

1. Live detector logic in [`crates/worldwake-cli/src/bin/observer.rs`](../crates/worldwake-cli/src/bin/observer.rs) scans 200-tick windows and emits whenever 2+ agents each spend at least 60% of a window at the same place; it currently has no structural dampener for lawful split-support routing (`detect_geographic_convergence`, lines 973-1084).
2. The active S117 spec still states that the 60% threshold is safely above the healthy-baseline overlap floor, but the live baseline contradicts that claim: all three agents spend most of ticks `0–632` at `Fertile Fields` on a healthy run because the scenario has a single orchard-backed food source there (`scenarios/survival-baseline.ron`, lines 8-25 and 371-381).
3. Shared abstraction boundary under audit: the observer's read-side place-occupancy detector over `AgentStats.location_history` plus any same-crate static substrate reads needed to distinguish lawful single-source routing from suspicious convergence. No simulation system or planner logic is in scope.
4. The motivating regression invariant is not "baseline must be spatially uniform"; it is "baseline must not be flagged when agents lawfully share a place that only supplies one survival support family while complementary support remains elsewhere." A true convergence smell still needs to fire when the anchored place itself bundles the relevant survival support and agents still remain anchored there.
5. The current baseline report proves the false positive concretely: `GEOGRAPHIC_CONVERGENCE` fires for `(Agent A, Agent B, Agent C)` at `Fertile Fields` over ticks `0–632`, while the scenario itself only authors apples at `Fertile Fields` and water/wash at camp/forest sites.
6. Existing proof already covers the positive branch: `convergence_smell_fires_on_forced_hub_scenario` in `golden_observer_anomalies.rs` proves the detector's forcing fixture. Reassessment showed that fixture also uses a single viable place, so a blunt "sole provider" suppression would be wrong. What is missing is a lawful non-pathological negative case on the healthy baseline and a focused detector test proving the new split-support dampener.
7. Lawful competing branches intentionally excluded from the original convergence fixture remain excluded here; this ticket does not relax the forcing fixture or reinterpret the contested scenario. It only teaches the detector not to misclassify authored single-source baseline routing.
8. Adjacent contradictions exposed during reassessment split cleanly:
   - `MaintenanceStarvation` merged-span false positives are a separate observer bug and belong to `S117CONMAIOBS-010`.
   - `ACUTE_NEED_SPIKE` on baseline may reflect a real baseline/planner issue and belongs to `S117CONMAIOBS-011`.
9. The detector must remain a derived observer view per `docs/FOUNDATIONS.md` principles 3, 12, 27, and 29A: any dampener must be computed from existing authoritative/read-side substrate, not from a new scenario-only allowlist or hidden heuristic exception.

## Architecture Check

1. The clean fix is to add a read-side structural dampener that checks whether the converged place is merely one support node inside a split-support survival topology, rather than weakening the threshold globally or special-casing `survival-baseline.ron` by filename. That keeps the detector principled and reusable.
2. No backward-compatibility shim or per-scenario suppression list is introduced. The detector remains one mechanical path with better classification logic.

## Verification Layers

1. Lawful single-source baseline routing does not emit `GEOGRAPHIC_CONVERGENCE` -> observer golden E2E against `scenarios/survival-baseline.ron`
2. Forced anchored-hub scenario still emits exactly one convergence anomaly -> existing golden E2E in `golden_observer_anomalies.rs`
3. Dampener classification logic distinguishes sole-provider routing from suspicious anchored overlap -> focused `observer.rs` unit/runtime test at detector/helper level
4. Section 3 render contract stays unchanged for real convergence anomalies -> `golden_observer_anomalies.rs` string assertions plus existing observer render tests
5. Single-layer ticket: no additional planner/action-trace mapping is needed because the owned change is observer-side anomaly classification only

## What to Change

### 1. Add a lawful single-source dampener to `detect_geographic_convergence()`

Teach the detector to suppress convergence anomalies when the shared place is simply one survival-support node inside a lawful split-support topology during the window. The exact live substrate and helper shape must be reassessed during implementation, but the end-state contract is:

- baseline-like "everyone must go to the only orchard, then leave for water/wash/sleep elsewhere" does not count as a convergence smell
- contested/hub scenarios where the anchored place itself bundles the relevant support still count
- the logic stays mechanical and derived from existing observer/static world substrate

### 2. Add focused negative coverage for the dampener

Add focused observer coverage proving the detector no longer fires on a lawful single-source routing case and still fires on the forced hub fixture.

### 3. Repair the baseline regression contract in the golden ticket surface

Update the baseline regression assertion path in `golden_observer_anomalies.rs` or the owning helper surface so the healthy baseline no longer reports convergence once the dampener lands.

## Files to Touch

- `crates/worldwake-cli/src/bin/observer.rs` (modify)
- `crates/worldwake-cli/tests/golden_observer_anomalies.rs` (modify)
- `tickets/S117CONMAIOBS-007.md` (modify if the owning baseline-regression handoff needs a factual note)

## Out of Scope

- Re-authoring `survival-baseline.ron`
- Changing planner or authoritative survival behavior
- Weakening the global 60% threshold without a structural justification
- `MaintenanceStarvation` or `AcuteNeedSpike` logic

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-cli --test golden_observer_anomalies`
2. `cargo test -p worldwake-cli --bin observer`
3. Existing suite: `cargo test -p worldwake-cli`

### Invariants

1. `GEOGRAPHIC_CONVERGENCE` remains a read-side observer classification over existing substrate; no scenario-name allowlist or authored override is introduced.
2. The forced convergence fixture still emits the anomaly, while healthy baseline lawful single-source routing does not.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-cli/src/bin/observer.rs` — focused detector/helper test proving lawful single-source routing is suppressed while suspicious anchored overlap still emits.
2. `crates/worldwake-cli/tests/golden_observer_anomalies.rs` — baseline regression assertion updated to prove `GEOGRAPHIC_CONVERGENCE` stays absent on `survival-baseline.ron`.

### Commands

1. `cargo test -p worldwake-cli --test golden_observer_anomalies`
2. `cargo test -p worldwake-cli --bin observer`
3. `cargo test -p worldwake-cli`
4. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

Completed on 2026-04-18.

- Added a private place-level survival-support summary helper in `crates/worldwake-cli/src/bin/observer.rs` and used it to suppress `GEOGRAPHIC_CONVERGENCE` when the converged place is only a single support node inside a split-support topology.
- Kept the detector mechanical and read-side only: the dampener reads existing authoritative world substrate (place tags, ground item lots, resource sources) and does not special-case `survival-baseline.ron` by name.
- Added focused observer coverage proving the dampener suppresses an orchard-only split-support node while still firing on a bundled support hub.
- Added a golden baseline regression assertion in `crates/worldwake-cli/tests/golden_observer_anomalies.rs` proving `GEOGRAPHIC_CONVERGENCE` stays absent on `scenarios/survival-baseline.ron` while the forcing hub fixture still emits.

## Deviations

- The drafted ticket described the fix as a “sole provider” dampener. Live reassessment showed that wording would wrongly suppress the existing positive hub fixture, which also has a single viable place. The landed rule is narrower and more honest: suppress only when the converged place exposes exactly one local survival-support family and complementary support clearly exists elsewhere.
- The focused observer proof used synthetic prototype-place support setup in `observer.rs` rather than expanding fixture scenarios. That kept the detector proof local while the golden file owned the real baseline regression and forcing-scenario end-to-end checks.
- `tickets/S117CONMAIOBS-007.md` did not need a factual follow-up edit in this ticket because the new baseline regression assertion now lives directly in `golden_observer_anomalies.rs`, which `007` already owns.

## Verification Result

- Passed `cargo test -p worldwake-cli --test golden_observer_anomalies`
- Passed `cargo test -p worldwake-cli --bin observer`
- Passed `cargo test -p worldwake-cli`
- Passed `cargo clippy --workspace --all-targets -- -D warnings`
