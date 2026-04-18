# S117CONMAIOBS-014: Survival baseline golden forensics must tolerate lawful in-transit state

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — golden harness / survival forensics proof surface in `worldwake-ai`
**Deps**: `archive/tickets/S117CONMAIOBS-011.md`, `docs/golden-e2e-testing.md`

## Problem

`S117CONMAIOBS-013` needs the ignored `all_agents_survive_1440_ticks` survival golden to become an honest behavior-level oracle again, but the live harness currently panics before it can prove anything about split-support survival behavior. In `crates/worldwake-ai/tests/golden_harness/survival_forensics_assertions.rs`, `observe_critical_windows()` calls `LocalSurvivalStateSummary::capture(&harness.world, agent).expect("golden survival agents should always have an effective place")`. Authoritative `World::effective_place()` in `crates/worldwake-core/src/world/placement.rs` returns `None` for lawful in-transit entities, and the authored baseline requires travel. That means the current ignored golden is asserting a false world invariant at the proof surface.

## Assumption Reassessment (2026-04-18)

1. The current failing command is real and reproducible: `cargo test -p worldwake-ai --test golden_survival_baseline all_agents_survive_1440_ticks -- --ignored --exact` fails immediately on `golden survival agents should always have an effective place`, before reaching the baseline survival-health assertions.
2. Shared abstraction boundary under audit: survival-golden forensic capture between `crates/worldwake-ai/tests/golden_harness/survival_forensics_assertions.rs`, `crates/worldwake-ai/src/survival_forensics.rs`, and authoritative `World::effective_place()` in `crates/worldwake-core/src/world/placement.rs`.
3. The failing assertion is in the proof surface, not the planner or observer logic. `observe_critical_windows()` samples forensic local-state every tick for every agent, including ticks where the agent is lawfully traveling and therefore has no authoritative effective place.
4. The intended invariant is narrower than the current harness claim. The baseline golden needs enough local-state forensic information to explain critical windows, but it does not require every agent to have a ground place on every sampled tick.
5. Live `LocalSurvivalStateSummary` in `crates/worldwake-ai/src/survival_forensics.rs` is a place-local snapshot with booleans for local food/water/wash/sleep support. Its current API assumes a place exists and therefore cannot represent lawful in-transit frames without panicking at the call site.
6. This ticket is not a planner-behavior ticket. It must not retune goal selection, observer anomaly logic, or scenario topology; it only restores the ignored survival golden as an honest oracle.
7. `docs/FOUNDATIONS.md` requires the proof surface to describe lawful world state rather than inventing a convenience invariant. Treating in-transit agents as if they always had a place would violate the causality-first model and degrade traceability.
8. Adjacent contradiction classification:
   - planner-side split-support survival behavior remains owned by `S117CONMAIOBS-013`
   - this ticket owns only the survival-golden harness/forensics mismatch that blocks `013`'s behavior-level proof

## Architecture Check

1. Fixing the golden harness separately is cleaner than broadening `S117CONMAIOBS-013` into mixed planner-plus-proof-surface work. It keeps planner behavior and proof infrastructure on distinct tickets.
2. The fix should preserve authoritative semantics: lawful in-transit state must remain representable as “no current place” rather than being collapsed into a fake place-local summary.

## Verification Layers

1. Lawful in-transit state no longer panics the survival-forensics capture path -> focused `worldwake-ai` unit/runtime coverage around `LocalSurvivalStateSummary` or `observe_critical_windows`
2. The ignored baseline golden becomes an honest behavior-level oracle again -> `cargo test -p worldwake-ai --test golden_survival_baseline all_agents_survive_1440_ticks -- --ignored --exact`
3. Forensic output still distinguishes place-local support from non-local travel frames -> focused proof on the serialized/reportable survival-forensics carrier
4. No planner or simulation semantics changed -> normal `worldwake-ai` crate verification only; no production AI behavior assertions belong to this ticket

## What to Change

### 1. Make the survival-forensics local-state carrier lawful for in-transit frames

Adjust the `LocalSurvivalStateSummary` capture/report path so it can represent ticks where an agent is in transit and has no authoritative `effective_place()`. The landed representation must stay concrete and deterministic; do not replace `None` with a synthetic place or hidden fallback.

### 2. Fix the ignored survival baseline golden to use the lawful carrier

Update the golden harness so `observe_critical_windows()` no longer assumes a place exists on every tick. The baseline golden should still capture enough forensic context to debug critical windows, but the harness must only assert invariants that are true in the authoritative world model.

### 3. Revalidate the blocked oracle for downstream planner work

Rerun the exact ignored baseline selector after the harness change. If it still fails, the next failure should be behavior- or contract-relevant rather than a proof-surface panic.

## Files to Touch

- `crates/worldwake-ai/src/survival_forensics.rs` (modify)
- `crates/worldwake-ai/tests/golden_harness/survival_forensics_assertions.rs` (modify)
- `crates/worldwake-ai/tests/golden_survival_baseline.rs` (modify only if the harnessed proof text/assertions need factual updates)
- `tickets/S117CONMAIOBS-013.md` (modify if closeout needs to remove the proof-surface blocker)

## Out of Scope

- Planner-side split-support survival preparation
- Observer anomaly detector tuning or suppression
- Scenario retuning for `survival-baseline.ron`
- Reinterpreting in-transit entities as having a synthetic current place

## Acceptance Criteria

### Tests That Must Pass

1. Focused `worldwake-ai` proof for lawful in-transit survival-forensics capture
2. `cargo test -p worldwake-ai --test golden_survival_baseline all_agents_survive_1440_ticks -- --ignored --exact`
3. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. The survival golden no longer asserts that all sampled agents always have an authoritative place while traveling.
2. Place-local forensic fields remain truthful for grounded frames and explicitly non-place-local for in-transit frames.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/survival_forensics.rs` or nearby focused harness coverage — prove the lawful in-transit carrier shape.
2. `crates/worldwake-ai/tests/golden_survival_baseline.rs` — rerun the exact ignored baseline selector as the restored behavior-level oracle.

### Commands

1. Focused `cargo test -p worldwake-ai <exact focused selector>`
2. `cargo test -p worldwake-ai --test golden_survival_baseline all_agents_survive_1440_ticks -- --ignored --exact`
3. `cargo test -p worldwake-ai`

## Outcome

Completed on 2026-04-18.

- Updated `LocalSurvivalStateSummary` in `crates/worldwake-ai/src/survival_forensics.rs` so lawful in-transit frames are represented explicitly as `place: None` with all place-local support flags `false`, instead of failing capture when `World::effective_place()` is absent.
- Updated `crates/worldwake-ai/tests/golden_harness/survival_forensics_assertions.rs` so `observe_critical_windows()` no longer asserts that every sampled agent always has an effective place.
- Added focused same-file proof in `crates/worldwake-ai/src/survival_forensics.rs` for in-transit capture behavior.
- Revalidated the blocked behavior-level oracle: `cargo test -p worldwake-ai --test golden_survival_baseline all_agents_survive_1440_ticks -- --ignored --exact` now passes and no longer fails on the old harness panic.

## Deviations

- The landed seam was narrower than the draft. No changes to `crates/worldwake-ai/tests/golden_survival_baseline.rs` were required; fixing the exported survival-forensics carrier plus the shared golden-harness helper was sufficient to restore the ignored selector as a lawful oracle.
- The focused verification command had to use the exact unit-test path `cargo test -p worldwake-ai --lib 'survival_forensics::tests::local_survival_state_summary_capture_marks_in_transit_agents_without_place' -- --exact` because a looser substring selector compiled the crate while executing zero tests.

## Verification Result

- Passed `cargo test -p worldwake-ai --lib 'survival_forensics::tests::local_survival_state_summary_capture_marks_in_transit_agents_without_place' -- --exact`
- Passed `cargo test -p worldwake-ai --test golden_survival_baseline all_agents_survive_1440_ticks -- --ignored --exact`
- Passed `cargo test -p worldwake-ai`
