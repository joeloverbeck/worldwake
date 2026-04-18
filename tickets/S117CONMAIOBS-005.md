# S117CONMAIOBS-005: `AcuteNeedSpike` detector

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: None
**Deps**: `archive/tickets/S117CONMAIOBS-001.md`, `specs/S117-convergence-maintenance-observer-smells.md`

## Problem

The survival-contested run showed Agent C at hunger=950 for 97 consecutive ticks and thirst=900 for 37 consecutive ticks — both below the 100-tick `SUSTAINED_CRITICAL_NEED` cutoff and therefore invisible today, yet both dangerously close to the agent's metabolism tolerance (`dehydration_tolerance_ticks=220`). This ticket adds a detector that flags maximal 30–99 tick runs where a need stays at or above the agent's critical threshold, filling the sub-threshold gap without double-flagging against `SUSTAINED_CRITICAL_NEED`.

## Assumption Reassessment (2026-04-18)

1. Per-tick needs samples are already collected in `AgentStats.needs_samples` (see 003 reassessment §1). The detector's scan is a linear walk — no new sampling required.
2. `DriveThresholds::critical(need)` exists at `crates/worldwake-core/src/drives.rs:92` as a public `const fn` taking `HomeostaticNeedId` and returning `Permille`. Unlike the medium threshold path in 003, this needs no per-need match — the helper already exists.
3. The existing `SustainedCriticalNeed` detector at `bin/observer.rs:887-927` fires on runs ≥ 100 ticks above `pm(750)`. That threshold is currently hardcoded at 750 permille rather than reading `DriveThresholds::critical(need)` — the new detector uses the per-agent critical threshold, so AcuteNeedSpike's lower-bound overlap with SustainedCriticalNeed may be non-zero only if the agent's critical threshold is below 750. The two detectors remain disjoint by their upper/lower run-length bounds (30–99 vs. ≥100), so no double-flagging occurs on the same run.
4. Shared abstraction boundary under audit: the `AgentStats.needs_samples` data structure + `detect_anomalies()` orchestrator. No new runtime systems involved.
5. Dedup key: `(AcuteNeedSpike, agent, need, run_start_tick)` — one anomaly per maximal run. Runs separated by a single-tick gap remain distinct (the reference implementation does not merge gaps per spec D5).

## Architecture Check

1. The detector reads per-agent critical thresholds via the existing `DriveThresholds::critical(need)` helper, respecting FND-14 (per-agent belief/parameter reasoning) and FND-22 (agent diversity). A hardcoded critical constant would contradict both principles and would miss spikes for agents with non-default drive profiles.
2. Maximal-run detection (find the longest continuous span at-or-above threshold, not overlapping sliding windows) is the correct shape: it emits one anomaly per genuinely-uninterrupted crisis, not one per tick.
3. No new ECS state, no cross-crate API, no backward-compatibility shim. Pure observer read-side.

## Verification Layers

1. Detector fires on a 40-tick thirst=900 run followed by relief → focused unit test with synthetic trajectory; assert one anomaly with `tick_range` matching 40 ticks and `description` containing "40 consecutive ticks".
2. Detector does NOT fire on a 29-tick run (below the 30-tick minimum) → focused unit test.
3. Detector does NOT fire on a 150-tick run (above the 99-tick maximum; SustainedCriticalNeed owns that territory) → focused unit test.
4. Detector does NOT double-emit for the same maximal run across overlapping scans → focused unit test with a 50-tick run asserts exactly one anomaly.
5. Single-layer ticket (observer read-side); no action-trace or event-log proof surface applies.

## What to Change

### 1. New detector function

Add `fn detect_acute_need_spike(stats_by_agent: &BTreeMap<EntityId, AgentStats>, thresholds_by_agent: &BTreeMap<EntityId, DriveThresholds>, names: &BTreeMap<EntityId, String>, anomalies: &mut Vec<Anomaly>)` below `detect_recipe_monoculture`.

Logic per agent per `HomeostaticNeedId`:

- File-local constants (or module-level if already introduced by 002/003): `ACUTE_MIN_TICKS: usize = 30`, `ACUTE_MAX_TICKS: usize = 100` (exclusive upper; i.e., run_length < 100 qualifies).
- `let critical_permille = thresholds.critical(need).as_u16();`
- Single linear pass over `needs_samples` tracking the current run's start index. For each sample:
  - If `need_value(sample, need) >= critical_permille`, extend the current run.
  - Otherwise, if a current run exists, close it. If `ACUTE_MIN_TICKS <= run_length < ACUTE_MAX_TICKS`, emit an anomaly.
- After the loop, close any open run with the same emit condition.
- Anomaly shape:
  - `kind: AnomalyKind::AcuteNeedSpike`
  - `agent_name: names[&agent]`
  - `additional_agent_names: None`
  - `description: format!("{} above critical threshold ({} permille) for {} consecutive ticks (ticks {}–{}), peak {} permille. Below the 100-tick sustained-critical bar but within {:.0}% of starvation tolerance ({} ticks).", need_label, critical_permille, run_length, run_start, run_end, peak, percent_of_tolerance, tolerance_ticks)` — where `tolerance_ticks` is read from the agent's `MetabolismProfile.starvation_tolerance_ticks` (for hunger) or `dehydration_tolerance_ticks` (for thirst); for other needs, omit the tolerance clause.
  - `tick_range: Some((run_start, run_end))`

For fatigue/bladder/dirtiness needs, the tolerance clause is omitted from the description (those needs lack hard tolerance limits in the current model).

### 2. Wire into `detect_anomalies()`

Call from the orchestrator after `detect_recipe_monoculture`. Reuse the `thresholds_by_agent` BTreeMap already collected in 003.

### 3. MetabolismProfile lookup

Collect per-agent `MetabolismProfile` via `world.get_component_metabolism_profile(agent).copied().unwrap_or_default()` for the tolerance-clause formatting. The component is defined in `crates/worldwake-core/src/needs.rs` near line 131.

### 4. Focused unit tests

Add to the existing `#[cfg(test)] mod tests`:

- `test_acute_need_spike_fires_on_40_tick_run` — synthetic thirst trajectory: 40 consecutive ticks at 900 permille followed by 10 ticks at 100. Assert one anomaly with run_length 40.
- `test_acute_need_spike_does_not_fire_below_30_ticks` — 29-tick run. Assert zero anomalies.
- `test_acute_need_spike_does_not_fire_at_or_above_100_ticks` — 100-tick run. Assert zero anomalies (SustainedCriticalNeed's territory).
- `test_acute_need_spike_emits_once_per_maximal_run` — 50-tick run. Assert exactly one anomaly.
- `test_acute_need_spike_treats_gaps_as_distinct` — two 40-tick runs separated by a 1-tick dip below critical. Assert two anomalies.

## Files to Touch

- `crates/worldwake-cli/src/bin/observer.rs` (modify)

## Out of Scope

- Merging gapped runs (spec D5 explicitly does not merge across gaps).
- Changing the `SUSTAINED_CRITICAL_NEED` hardcoded 750-permille threshold — that is a separate cleanup if desired.
- Goldens against real scenario fixtures (007).
- Tolerance-clause support for non-hunger/thirst needs (model does not expose those tolerances today).

## Acceptance Criteria

### Tests That Must Pass

1. `test_acute_need_spike_fires_on_40_tick_run` passes.
2. `test_acute_need_spike_does_not_fire_below_30_ticks` passes.
3. `test_acute_need_spike_does_not_fire_at_or_above_100_ticks` passes.
4. `test_acute_need_spike_emits_once_per_maximal_run` passes.
5. `test_acute_need_spike_treats_gaps_as_distinct` passes.
6. Existing suite: `cargo test -p worldwake-cli`.

### Invariants

1. AcuteNeedSpike and SustainedCriticalNeed never double-flag the same maximal run (ranges 30–99 and ≥100 are disjoint).
2. The critical threshold in the description equals the agent's actual `DriveThresholds::critical(need).as_u16()` — not a hardcoded global constant.
3. Run detection uses `>=` against the critical threshold, not `>`. A sample exactly at critical counts as part of the run.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-cli/src/bin/observer.rs` — five new focused unit tests above.

### Commands

1. `cargo test -p worldwake-cli --bin observer acute_need_spike`
2. `cargo test -p worldwake-cli`
3. `cargo clippy --workspace --all-targets -- -D warnings`
